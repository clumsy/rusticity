//! Read-only detail view for a single KMS key.
//!
//! DescribeKey is the authoritative, freshly-fetched source for key metadata;
//! its returned identity is verified against the requested ARN before any
//! other call is made. Aliases, rotation status, policy and tags are each
//! gathered independently so that a per-section failure — a missing
//! permission, or a rotation state the API reports as unsupported — surfaces
//! as that section's `Err(String)` rather than failing the whole view or, worse,
//! being silently presented as an empty/"disabled" success.
//!
//! API allowlist (read-only, never write): DescribeKey, ListAliases (scoped to
//! the key via `key_id`), GetKeyRotationStatus, GetKeyPolicy (`policy_name`
//! defaulting to the only supported value), and ListResourceTags. No
//! Encrypt/Decrypt/GenerateDataKey/export/mutating call is ever issued.
use crate::config::AwsConfig;
use crate::pagination::{Pagination, StopReason};
use anyhow::{anyhow, bail, Context, Result};
use aws_smithy_types::date_time::Format;
use aws_smithy_types::DateTime;
use std::time::Duration;

/// The whole detail operation is bounded; section errors raised before the
/// deadline are preserved in the returned value.
const OP_TIMEOUT: Duration = Duration::from_secs(30);
/// Fail a section explicitly at this bound rather than present a partial list.
const MAX_PAGES: usize = 1000;
/// ListResourceTags permits at most 50; this page size is also valid for ListAliases.
const PAGE_LIMIT: i32 = 50;

/// A single key's detail, as surfaced to the UI. Each independently-gathered
/// section is a `Result` so a failure in one is displayed as an error there
/// without discarding the others or the authoritative `metadata`.
#[derive(Clone, Debug)]
pub struct KmsKeyDetail {
    /// Authoritative metadata, freshly fetched via DescribeKey.
    pub metadata: super::KmsKey,
    pub aliases: Result<Vec<KmsAlias>, String>,
    pub rotation: Result<KmsRotation, String>,
    pub policy: Result<String, String>,
    pub tags: Result<Vec<KmsTag>, String>,
}

/// One alias pointing at the key. Timestamps are RFC3339 UTC, empty when the
/// SDK omitted them (never invented).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KmsAlias {
    pub name: String,
    pub arn: String,
    pub creation_date: String,
    pub last_updated_date: String,
}

/// One resource tag on the key.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KmsTag {
    pub key: String,
    pub value: String,
}

/// Rotation status. `enabled` is tri-state: `None` means the API did not report
/// it (not "disabled"). Dates are RFC3339 UTC, empty when absent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KmsRotation {
    pub enabled: Option<bool>,
    pub rotation_period_days: Option<i32>,
    pub next_rotation_date: String,
    pub on_demand_rotation_start_date: String,
}

/// Format an optional timestamp as RFC3339 UTC, empty when absent.
fn rfc3339(ts: Option<&DateTime>) -> String {
    ts.and_then(|dt| dt.fmt(Format::DateTime).ok())
        .unwrap_or_default()
}

/// Validate a full KMS *key* ARN (not an alias ARN). The request entry point
/// separately checks its region/account against the active configuration.
///
/// A KMS key ARN is:
///   arn:<partition>:kms:<region>:<account>:key/<key-id>
pub(crate) fn validate_key_arn(arn: &str) -> Result<()> {
    let parts: Vec<_> = arn.splitn(6, ':').collect();
    if parts.first() != Some(&"arn") || parts.get(2) != Some(&"kms") {
        bail!("not a KMS key ARN: {arn:?}");
    }
    let resource = parts.get(5).copied().unwrap_or_default();
    let key_id = resource.strip_prefix("key/");
    if parts.len() != 6
        || parts[1].is_empty() // partition
        || parts[3].is_empty() // region
        || parts[4].len() != 12
        || !parts[4].bytes().all(|c| c.is_ascii_digit()) // account id
        || key_id.is_none_or(str::is_empty) // must be key/<non-empty>, not alias/*
        || arn.chars().any(char::is_control)
    {
        bail!("malformed KMS key ARN: {arn:?}");
    }
    Ok(())
}

/// Reject a region/account that does not match the active config, before any
/// request is spent.
fn validate_against_config(config: &AwsConfig, arn: &str) -> Result<()> {
    validate_key_arn(arn)?;
    let parts: Vec<_> = arn.splitn(6, ':').collect();
    let (region, account) = (parts[3], parts[4]);
    if !config.region.is_empty() && region != config.region {
        bail!(
            "KMS key ARN Region {region:?} does not match the active Region {:?}",
            config.region
        );
    }
    if !config.account_id.is_empty() && account != config.account_id {
        bail!(
            "KMS key ARN account {account:?} does not match the active account {:?}",
            config.account_id
        );
    }
    Ok(())
}

/// Entry point used by `KmsClient::key_details`. Bounds the whole operation by
/// a single timeout; the returned value preserves each section's own error.
pub(crate) async fn key_details(config: &AwsConfig, key_arn: &str) -> Result<KmsKeyDetail> {
    validate_against_config(config, key_arn)?;
    key_details_inner(config, key_arn, MAX_PAGES).await
}

async fn key_details_inner(
    config: &AwsConfig,
    key_arn: &str,
    max_pages: usize,
) -> Result<KmsKeyDetail> {
    let client = config.kms_client();
    let deadline = tokio::time::Instant::now() + OP_TIMEOUT;

    // Metadata must succeed before the key's other sections can be trusted.
    let describe = tokio::time::timeout_at(deadline, client.describe_key().key_id(key_arn).send())
        .await
        .map_err(|_| anyhow!("KMS DescribeKey timed out after {OP_TIMEOUT:?}"))?
        .context("KMS DescribeKey request failed")?;
    let meta = describe
        .key_metadata
        .context("KMS DescribeKey returned no key metadata")?;

    // Verify the authoritative identity matches what was requested, so a
    // response for a different key (e.g. via an alias indirection) is rejected
    // rather than silently detailed.
    let returned_arn = meta
        .arn()
        .filter(|a| !a.is_empty())
        .context("KMS DescribeKey returned metadata with no ARN")?;
    if returned_arn != key_arn {
        bail!(
            "KMS DescribeKey returned key {returned_arn:?} for requested ARN {key_arn:?}; \
             refusing to present a mismatched identity"
        );
    }

    if meta.key_id() != key_arn.rsplit('/').next().unwrap_or_default() {
        bail!("KMS DescribeKey returned a KeyId that does not match the requested ARN");
    }
    let metadata = describe_to_kms_key(&meta);
    // Keep every sub-request bound to the validated full identity.
    let key_id = key_arn.to_string();

    // Independent sections share the original deadline. A slow or denied
    // section must not discard fresh metadata or another successful section.
    let (aliases, rotation, policy, tags) = tokio::join!(
        section_read(deadline, fetch_aliases(&client, &key_id, max_pages)),
        section_read(deadline, fetch_rotation(&client, &key_id)),
        section_read(deadline, fetch_policy(&client, &key_id)),
        section_read(deadline, fetch_tags(&client, &key_id, max_pages)),
    );

    Ok(KmsKeyDetail {
        metadata,
        aliases,
        rotation,
        policy,
        tags,
    })
}

async fn section_read<T>(
    deadline: tokio::time::Instant,
    read: impl std::future::Future<Output = Result<T>>,
) -> Result<T, String> {
    tokio::time::timeout_at(deadline, read)
        .await
        .map_err(|_| "KMS section timed out within the 30s detail-read budget".to_string())?
        .map_err(|error| format!("{error:#}"))
}

/// Map fresh DescribeKey metadata onto the existing `KmsKey` struct, preserving
/// its string-enum Debug labels (Aws/Customer etc.) exactly as `list_keys` does.
fn describe_to_kms_key(meta: &aws_sdk_kms::types::KeyMetadata) -> super::KmsKey {
    let dbg = |v: Option<String>| v.unwrap_or_default();
    super::KmsKey {
        key_id: meta.key_id().to_string(),
        key_arn: meta.arn().unwrap_or_default().to_string(),
        description: meta.description().unwrap_or_default().to_string(),
        key_state: dbg(meta.key_state().map(|s| format!("{s:?}"))),
        key_usage: dbg(meta.key_usage().map(|u| format!("{u:?}"))),
        key_spec: dbg(meta.key_spec().map(|s| format!("{s:?}"))),
        key_manager: dbg(meta.key_manager().map(|m| format!("{m:?}"))),
        creation_date: rfc3339(meta.creation_date()),
        expiration_date: rfc3339(meta.valid_to()),
        deletion_date: rfc3339(meta.deletion_date()),
        custom_key_store_id: meta.custom_key_store_id().unwrap_or_default().to_string(),
        origin: dbg(meta.origin().map(|o| format!("{o:?}"))),
        enabled: meta.enabled(),
        alias: String::new(), // aliases are surfaced in their own section
        multi_region: meta.multi_region().unwrap_or(false),
    }
}

async fn fetch_aliases(
    client: &aws_sdk_kms::Client,
    key_id: &str,
    max_pages: usize,
) -> Result<Vec<KmsAlias>> {
    let mut aliases = Vec::new();
    let mut marker = None;
    let mut pagination = Pagination::new(max_pages);
    for _ in 0..max_pages {
        let page = client
            .list_aliases()
            .key_id(key_id) // scope to this key only
            .limit(PAGE_LIMIT)
            .set_marker(marker)
            .send()
            .await
            .context("KMS ListAliases request failed")?;
        for entry in page.aliases() {
            if entry
                .target_key_id()
                .is_some_and(|target| target != key_id.rsplit('/').next().unwrap_or_default())
            {
                bail!("KMS ListAliases returned an alias for a different key");
            }
            aliases.push(KmsAlias {
                name: entry.alias_name().unwrap_or_default().to_string(),
                arn: entry.alias_arn().unwrap_or_default().to_string(),
                creation_date: rfc3339(entry.creation_date()),
                last_updated_date: rfc3339(entry.last_updated_date()),
            });
        }
        match pagination.advance(next_marker(page.truncated(), page.next_marker)?) {
            Ok(Some(next)) => marker = Some(next),
            Ok(None) => {
                aliases.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.arn.cmp(&b.arn)));
                return Ok(aliases);
            }
            Err(StopReason::RepeatedToken) => {
                bail!("KMS ListAliases returned a repeated pagination token; aborting")
            }
            Err(StopReason::PageLimit | StopReason::ResultLimit) => break,
        }
    }
    bail!("KMS ListAliases exceeded {max_pages} pages; alias list is incomplete")
}

async fn fetch_rotation(client: &aws_sdk_kms::Client, key_id: &str) -> Result<KmsRotation> {
    let status = client
        .get_key_rotation_status()
        .key_id(key_id)
        .send()
        .await
        .context("KMS GetKeyRotationStatus request failed")?;
    Ok(KmsRotation {
        enabled: Some(status.key_rotation_enabled()),
        rotation_period_days: status.rotation_period_in_days(),
        next_rotation_date: rfc3339(status.next_rotation_date()),
        on_demand_rotation_start_date: rfc3339(status.on_demand_rotation_start_date()),
    })
}

async fn fetch_policy(client: &aws_sdk_kms::Client, key_id: &str) -> Result<String> {
    let out = client
        .get_key_policy()
        .key_id(key_id)
        .policy_name("default") // the only policy name KMS supports
        .send()
        .await
        .context("KMS GetKeyPolicy request failed")?;
    out.policy()
        .map(str::to_string)
        .context("KMS GetKeyPolicy returned no policy document")
}

async fn fetch_tags(
    client: &aws_sdk_kms::Client,
    key_id: &str,
    max_pages: usize,
) -> Result<Vec<KmsTag>> {
    let mut tags = Vec::new();
    let mut marker = None;
    let mut pagination = Pagination::new(max_pages);
    for _ in 0..max_pages {
        let page = client
            .list_resource_tags()
            .key_id(key_id)
            .limit(PAGE_LIMIT)
            .set_marker(marker)
            .send()
            .await
            .context("KMS ListResourceTags request failed")?;
        for tag in page.tags() {
            tags.push(KmsTag {
                key: tag.tag_key().to_string(),
                value: tag.tag_value().to_string(),
            });
        }
        match pagination.advance(next_marker(page.truncated(), page.next_marker)?) {
            Ok(Some(next)) => marker = Some(next),
            Ok(None) => {
                tags.sort_by(|a, b| a.key.cmp(&b.key).then_with(|| a.value.cmp(&b.value)));
                return Ok(tags);
            }
            Err(StopReason::RepeatedToken) => {
                bail!("KMS ListResourceTags returned a repeated pagination token; aborting")
            }
            Err(StopReason::PageLimit | StopReason::ResultLimit) => break,
        }
    }
    bail!("KMS ListResourceTags exceeded {max_pages} pages; tag list is incomplete")
}

/// A truncated page without a usable marker is incomplete, not a successful
/// end-of-stream. Never silently return a partial alias/tag inventory.
fn next_marker(truncated: bool, marker: Option<String>) -> Result<Option<String>> {
    if !truncated {
        return Ok(None);
    }
    let marker = marker
        .filter(|marker| !marker.is_empty())
        .context("KMS returned Truncated without NextMarker; list is incomplete")?;
    Ok(Some(marker))
}

#[cfg(test)]
mod tests;
