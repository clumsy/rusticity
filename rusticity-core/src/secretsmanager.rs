//! Read-only Secrets Manager metadata inventory. ListSecrets supplies metadata
//! without per-secret DescribeSecret calls or secret-value retrieval. It excludes
//! secrets scheduled for deletion by default.
use crate::config::AwsConfig;
use crate::pagination::{Pagination, StopReason};
use anyhow::{bail, Context, Result};
use aws_smithy_types::date_time::Format;
use aws_smithy_types::DateTime;
use std::time::Duration;

const LIST_TIMEOUT: Duration = Duration::from_secs(30);
// Fail explicitly at the bound rather than presenting an incomplete inventory.
const MAX_PAGES: usize = 1000;
// ListSecrets permits MaxResults 1..=100; request the maximum page size.
const PAGE_SIZE: i32 = 100;

/// Metadata for one secret, sourced entirely from ListSecrets. No field here
/// is or derives from a secret value. Absent SDK values are preserved as
/// `None`/empty rather than invented: a missing `rotation_enabled` stays
/// `None` (not `false`) and an omitted timestamp stays an empty string.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SecretMetadata {
    pub name: String,
    pub arn: String,
    pub description: String,
    pub rotation_enabled: Option<bool>,
    /// RFC3339 UTC, or empty when the SDK omitted it.
    pub created_date: String,
    /// RFC3339 UTC, or empty when the SDK omitted it.
    pub last_changed_date: String,
    /// UTC `YYYY-MM-DD` only — AWS reports last-accessed at date precision.
    /// Empty when the secret has never been retrieved in the Region.
    pub last_accessed_date: String,
    /// RFC3339 UTC, or empty when the SDK omitted it.
    pub last_rotated_date: String,
    pub deleted_date: String,
    pub next_rotation_date: String,
    pub primary_region: String,
    pub owning_service: String,
    /// AWS's metadata Type identifier (for example, a managed external type).
    pub secret_type: String,
}

/// Format an optional timestamp as RFC3339 UTC, empty when absent.
fn rfc3339(ts: Option<&DateTime>) -> String {
    ts.and_then(|dt| dt.fmt(Format::DateTime).ok())
        .unwrap_or_default()
}

/// Format an optional timestamp as UTC `YYYY-MM-DD`, empty when absent.
/// AWS reports last-accessed at date precision, so only the date is kept.
fn date_only(ts: Option<&DateTime>) -> String {
    rfc3339(ts)
        .split_once('T')
        .map(|(date, _)| date.to_string())
        .unwrap_or_default()
}

/// A listed secret ARN must be a well-formed Secrets Manager secret ARN whose
/// name segment is non-empty. Validated before the row is accepted so a
/// malformed entry surfaces as an error rather than a silently empty success.
pub(crate) fn validate_secret_arn(arn: &str) -> Result<()> {
    let parts: Vec<_> = arn.splitn(6, ':').collect();
    if parts.first() != Some(&"arn") || parts.get(2) != Some(&"secretsmanager") {
        bail!("not a Secrets Manager secret ARN: {arn:?}");
    }
    if parts.len() != 6
        || parts[1].is_empty() // partition
        || parts[3].is_empty() // region
        || parts[4].len() != 12
        || !parts[4].bytes().all(|c| c.is_ascii_digit()) // account id
        || parts[5].strip_prefix("secret:").is_none_or(|name| name.is_empty())
        || arn.chars().any(char::is_control)
    {
        bail!("malformed Secrets Manager secret ARN: {arn:?}");
    }
    Ok(())
}

pub struct SecretsManagerClient {
    config: AwsConfig,
}

impl SecretsManagerClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    /// List all secrets' metadata, sorted deterministically by name then ARN.
    /// The whole operation is bounded by a 30s timeout; a repeated pagination
    /// token or the page cap is a hard error, never a partial success.
    pub async fn list_secrets(&self) -> Result<Vec<SecretMetadata>> {
        tokio::time::timeout(LIST_TIMEOUT, self.list_secrets_inner(MAX_PAGES))
            .await
            .map_err(|_| {
                anyhow::anyhow!("Secrets Manager ListSecrets timed out after {LIST_TIMEOUT:?}")
            })?
    }

    async fn list_secrets_inner(&self, max_pages: usize) -> Result<Vec<SecretMetadata>> {
        let client = self.config.secretsmanager_client();
        let mut secrets = Vec::new();
        let mut token = None;
        let mut pagination = Pagination::new(max_pages);
        for _ in 0..max_pages {
            let page = client
                .list_secrets()
                .max_results(PAGE_SIZE)
                .set_next_token(token)
                .send()
                .await
                .context("Secrets Manager ListSecrets request failed")?;
            for entry in page.secret_list() {
                let arn = entry
                    .arn()
                    .filter(|arn| !arn.is_empty())
                    .context("Secrets Manager ListSecrets returned a secret with no ARN")?;
                validate_secret_arn(arn)?;
                let name = entry
                    .name()
                    .filter(|name| !name.is_empty())
                    .context("Secrets Manager ListSecrets returned a secret with no name")?;
                secrets.push(SecretMetadata {
                    name: name.to_string(),
                    arn: arn.to_string(),
                    description: entry.description().unwrap_or_default().to_string(),
                    rotation_enabled: entry.rotation_enabled(),
                    created_date: rfc3339(entry.created_date()),
                    last_changed_date: rfc3339(entry.last_changed_date()),
                    last_accessed_date: date_only(entry.last_accessed_date()),
                    last_rotated_date: rfc3339(entry.last_rotated_date()),
                    deleted_date: rfc3339(entry.deleted_date()),
                    next_rotation_date: rfc3339(entry.next_rotation_date()),
                    primary_region: entry.primary_region().unwrap_or_default().into(),
                    owning_service: entry.owning_service().unwrap_or_default().into(),
                    secret_type: entry.r#type().unwrap_or_default().into(),
                });
            }
            match pagination.advance(page.next_token) {
                Ok(Some(next)) => token = Some(next),
                Ok(None) => {
                    secrets.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.arn.cmp(&b.arn)));
                    return Ok(secrets);
                }
                Err(StopReason::RepeatedToken) => {
                    bail!("Secrets Manager ListSecrets returned a repeated pagination token; aborting");
                }
                Err(StopReason::PageLimit | StopReason::ResultLimit) => break,
            }
        }
        bail!("Secrets Manager ListSecrets exceeded {max_pages} pages; inventory is incomplete")
    }
}

#[cfg(test)]
mod tests;
