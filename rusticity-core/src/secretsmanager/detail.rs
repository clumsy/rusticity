//! Read-only detail view for a single secret. Metadata only.
//!
//! DescribeSecret is the authoritative, freshly-fetched source for the secret's
//! configuration, rotation, replication and tags. Its returned ARN is verified
//! against the requested ARN before anything else is trusted. Versions come from
//! ListSecretVersionIds and are gathered independently so a missing permission
//! there surfaces as that section's `Err(String)`, never as an empty success.
//!
//! API allowlist (read-only): DescribeSecret and ListSecretVersionIds. Neither
//! returns a secret value. GetSecretValue / BatchGetSecretValue and every
//! mutating call are never issued.
use super::{date_only, rfc3339, validate_secret_arn};
use crate::config::AwsConfig;
use crate::pagination::{Pagination, StopReason};
use anyhow::{anyhow, bail, Context, Result};
use std::time::Duration;

/// The whole detail operation is bounded; a versions error raised before the
/// deadline is preserved in the returned value.
const OP_TIMEOUT: Duration = Duration::from_secs(30);
/// Fail the versions section explicitly at this bound rather than show a partial list.
const MAX_PAGES: usize = 1000;
/// ListSecretVersionIds permits MaxResults 1..=100.
const PAGE_SIZE: i32 = 100;

/// One secret's detail. `versions` is a `Result` so its failure is displayed in
/// its own tab without discarding the authoritative `metadata`.
#[derive(Clone, Debug)]
pub struct SecretDetail {
    pub metadata: SecretDescription,
    pub versions: Result<Vec<SecretVersion>, String>,
}

/// Fresh DescribeSecret metadata. Absent SDK values stay `None`/empty rather
/// than invented: a missing `rotation_enabled` is `None`, not `false`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SecretDescription {
    pub name: String,
    pub arn: String,
    pub description: String,
    pub kms_key_id: String,
    pub secret_type: String,
    pub owning_service: String,
    pub primary_region: String,
    /// RFC3339 UTC, or empty when the SDK omitted it.
    pub created_date: String,
    pub last_changed_date: String,
    /// UTC `YYYY-MM-DD` only: AWS reports last-accessed at date precision.
    pub last_accessed_date: String,
    pub deleted_date: String,
    pub rotation: SecretRotation,
    pub replicas: Vec<SecretReplica>,
    pub tags: Vec<SecretTag>,
}

/// Rotation configuration. Dates are RFC3339 UTC, empty when absent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SecretRotation {
    pub enabled: Option<bool>,
    pub lambda_arn: String,
    pub automatically_after_days: Option<i64>,
    pub schedule_expression: String,
    pub window: String,
    pub last_rotated_date: String,
    pub next_rotation_date: String,
}

/// One replica Region of the secret.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SecretReplica {
    pub region: String,
    pub status: String,
    pub status_message: String,
    pub kms_key_id: String,
    /// UTC `YYYY-MM-DD`, empty when never accessed.
    pub last_accessed_date: String,
}

/// One version's metadata: identifier, staging labels and dates. Never a value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SecretVersion {
    pub version_id: String,
    pub stages: Vec<String>,
    pub created_date: String,
    /// UTC `YYYY-MM-DD`, empty when never accessed.
    pub last_accessed_date: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SecretTag {
    pub key: String,
    pub value: String,
}

/// Reject a malformed ARN, or one whose Region/account does not match the
/// active configuration, before any request is spent.
fn validate_against_config(config: &AwsConfig, arn: &str) -> Result<()> {
    validate_secret_arn(arn)?;
    let parts: Vec<_> = arn.splitn(6, ':').collect();
    let (region, account) = (parts[3], parts[4]);
    if !config.region.is_empty() && region != config.region {
        bail!(
            "Secret ARN Region {region:?} does not match the active Region {:?}",
            config.region
        );
    }
    if !config.account_id.is_empty() && account != config.account_id {
        bail!(
            "Secret ARN account {account:?} does not match the active account {:?}",
            config.account_id
        );
    }
    Ok(())
}

/// Entry point used by `SecretsManagerClient::secret_details`.
pub(crate) async fn secret_details(config: &AwsConfig, secret_arn: &str) -> Result<SecretDetail> {
    validate_against_config(config, secret_arn)?;
    secret_details_inner(config, secret_arn, MAX_PAGES).await
}

async fn secret_details_inner(
    config: &AwsConfig,
    secret_arn: &str,
    max_pages: usize,
) -> Result<SecretDetail> {
    let client = config.secretsmanager_client();
    let deadline = tokio::time::Instant::now() + OP_TIMEOUT;

    // Metadata must succeed before the versions section can be trusted.
    let out = tokio::time::timeout_at(
        deadline,
        client.describe_secret().secret_id(secret_arn).send(),
    )
    .await
    .map_err(|_| anyhow!("Secrets Manager DescribeSecret timed out after {OP_TIMEOUT:?}"))?
    .context("Secrets Manager DescribeSecret request failed")?;
    let returned_arn = out
        .arn()
        .filter(|arn| !arn.is_empty())
        .context("Secrets Manager DescribeSecret returned no ARN")?;
    if returned_arn != secret_arn {
        bail!(
            "Secrets Manager DescribeSecret returned secret {returned_arn:?} for requested ARN \
             {secret_arn:?}; refusing to present a mismatched identity"
        );
    }
    let name = out
        .name()
        .filter(|name| !name.is_empty())
        .context("Secrets Manager DescribeSecret returned no name")?;
    let rules = out.rotation_rules();
    let mut replicas: Vec<_> = out
        .replication_status()
        .iter()
        .map(|replica| SecretReplica {
            region: replica.region().unwrap_or_default().into(),
            status: replica
                .status()
                .map(|s| s.as_str().to_string())
                .unwrap_or_default(),
            status_message: replica.status_message().unwrap_or_default().into(),
            kms_key_id: replica.kms_key_id().unwrap_or_default().into(),
            last_accessed_date: date_only(replica.last_accessed_date()),
        })
        .collect();
    replicas.sort_by(|a, b| a.region.cmp(&b.region));
    let mut tags: Vec<_> = out
        .tags()
        .iter()
        .map(|tag| SecretTag {
            key: tag.key().unwrap_or_default().into(),
            value: tag.value().unwrap_or_default().into(),
        })
        .collect();
    tags.sort_by(|a, b| a.key.cmp(&b.key).then_with(|| a.value.cmp(&b.value)));
    let metadata = SecretDescription {
        name: name.into(),
        arn: returned_arn.into(),
        description: out.description().unwrap_or_default().into(),
        kms_key_id: out.kms_key_id().unwrap_or_default().into(),
        secret_type: out.r#type().unwrap_or_default().into(),
        owning_service: out.owning_service().unwrap_or_default().into(),
        primary_region: out.primary_region().unwrap_or_default().into(),
        created_date: rfc3339(out.created_date()),
        last_changed_date: rfc3339(out.last_changed_date()),
        last_accessed_date: date_only(out.last_accessed_date()),
        deleted_date: rfc3339(out.deleted_date()),
        rotation: SecretRotation {
            enabled: out.rotation_enabled(),
            lambda_arn: out.rotation_lambda_arn().unwrap_or_default().into(),
            automatically_after_days: rules.and_then(|r| r.automatically_after_days()),
            schedule_expression: rules
                .and_then(|r| r.schedule_expression())
                .unwrap_or_default()
                .into(),
            window: rules.and_then(|r| r.duration()).unwrap_or_default().into(),
            last_rotated_date: rfc3339(out.last_rotated_date()),
            next_rotation_date: rfc3339(out.next_rotation_date()),
        },
        replicas,
        tags,
    };

    let versions =
        tokio::time::timeout_at(deadline, fetch_versions(&client, secret_arn, max_pages))
            .await
            .map_err(|_| {
                "Secrets Manager versions timed out within the 30s detail-read budget".to_string()
            })
            .and_then(|read| read.map_err(|error| format!("{error:#}")));

    Ok(SecretDetail { metadata, versions })
}

async fn fetch_versions(
    client: &aws_sdk_secretsmanager::Client,
    secret_arn: &str,
    max_pages: usize,
) -> Result<Vec<SecretVersion>> {
    let mut versions = Vec::new();
    let mut token = None;
    let mut pagination = Pagination::new(max_pages);
    for _ in 0..max_pages {
        let page = client
            .list_secret_version_ids()
            .secret_id(secret_arn)
            .max_results(PAGE_SIZE)
            .set_next_token(token)
            .send()
            .await
            .context("Secrets Manager ListSecretVersionIds request failed")?;
        if page.arn().is_some_and(|arn| arn != secret_arn) {
            bail!("Secrets Manager ListSecretVersionIds returned versions for a different secret");
        }
        for entry in page.versions() {
            let mut stages: Vec<String> = entry.version_stages().to_vec();
            stages.sort();
            versions.push(SecretVersion {
                version_id: entry.version_id().unwrap_or_default().into(),
                stages,
                created_date: rfc3339(entry.created_date()),
                last_accessed_date: date_only(entry.last_accessed_date()),
            });
        }
        match pagination.advance(page.next_token) {
            Ok(Some(next)) => token = Some(next),
            Ok(None) => {
                // Newest first; ties broken by id for a stable order.
                versions.sort_by(|a, b| {
                    b.created_date
                        .cmp(&a.created_date)
                        .then_with(|| a.version_id.cmp(&b.version_id))
                });
                return Ok(versions);
            }
            Err(StopReason::RepeatedToken) => {
                bail!("Secrets Manager ListSecretVersionIds returned a repeated pagination token; aborting")
            }
            Err(StopReason::PageLimit | StopReason::ResultLimit) => break,
        }
    }
    bail!("Secrets Manager ListSecretVersionIds exceeded {max_pages} pages; version list is incomplete")
}

#[cfg(test)]
mod tests;
