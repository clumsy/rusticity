//! Read-only ACM certificate inventory from ListCertificates alone: no
//! per-certificate DescribeCertificate, GetCertificate or ExportCertificate.
//!
//! ListCertificates returns only RSA_2048 certificates unless key types are
//! requested explicitly, so every key algorithm the SDK knows is included.
//! Every certificate status is listed (none is filtered out).
use crate::config::AwsConfig;
use crate::pagination::{Pagination, StopReason};
use anyhow::{bail, Context, Result};
use aws_sdk_acm::types::{Filters, KeyAlgorithm};
use aws_smithy_types::date_time::Format;
use aws_smithy_types::DateTime;
use std::time::Duration;

const LIST_TIMEOUT: Duration = Duration::from_secs(30);
/// Fail explicitly at the bound rather than present an incomplete inventory.
const MAX_PAGES: usize = 1000;
/// ListCertificates permits MaxItems 1..=1000.
const PAGE_SIZE: i32 = 1000;

/// Metadata for one certificate, entirely from ListCertificates. Never the
/// certificate body, chain or private key. Enum values keep their AWS wire
/// form (e.g. `AMAZON_ISSUED`); absent values stay `None`/empty.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Certificate {
    pub arn: String,
    /// The final ARN segment after `certificate/`.
    pub id: String,
    pub domain_name: String,
    pub additional_names: Vec<String>,
    /// AWS truncated `additional_names` in the summary.
    pub more_names: bool,
    pub cert_type: String,
    pub status: String,
    pub in_use: Option<bool>,
    pub exported: Option<bool>,
    pub renewal_eligibility: String,
    pub key_algorithm: String,
    pub managed_by: String,
    /// RFC3339 UTC, or empty when the SDK omitted it.
    pub not_before: String,
    pub not_after: String,
    pub created_at: String,
    pub issued_at: String,
    pub imported_at: String,
    pub revoked_at: String,
}

fn rfc3339(ts: Option<&DateTime>) -> String {
    ts.and_then(|dt| dt.fmt(Format::DateTime).ok())
        .unwrap_or_default()
}

/// `arn:<partition>:acm:<region>:<account>:certificate/<id>`; returns the id.
pub fn certificate_id(arn: &str) -> Result<&str> {
    let parts: Vec<_> = arn.splitn(6, ':').collect();
    if parts.first() != Some(&"arn") || parts.get(2) != Some(&"acm") {
        bail!("not an ACM certificate ARN: {arn:?}");
    }
    let id = parts
        .get(5)
        .and_then(|resource| resource.strip_prefix("certificate/"));
    match id {
        Some(id)
            if !parts[1].is_empty()
                && !parts[3].is_empty()
                && parts[4].len() == 12
                && parts[4].bytes().all(|c| c.is_ascii_digit())
                && !id.is_empty()
                && !id.contains('/')
                && !arn.chars().any(char::is_control) =>
        {
            Ok(id)
        }
        _ => bail!("malformed ACM certificate ARN: {arn:?}"),
    }
}

pub struct AcmClient {
    config: AwsConfig,
}

impl AcmClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    /// All certificates in the Region, sorted by domain name then ARN. The
    /// whole read is bounded by 30s; a repeated token or the page cap is an
    /// error, never a partial success.
    pub async fn list_certificates(&self) -> Result<Vec<Certificate>> {
        tokio::time::timeout(LIST_TIMEOUT, self.list_inner(MAX_PAGES))
            .await
            .map_err(|_| anyhow::anyhow!("ACM ListCertificates timed out after {LIST_TIMEOUT:?}"))?
    }

    async fn list_inner(&self, max_pages: usize) -> Result<Vec<Certificate>> {
        let client = self.config.acm_client();
        let includes = Filters::builder()
            .set_key_types(Some(
                KeyAlgorithm::values()
                    .iter()
                    .map(|v| KeyAlgorithm::from(*v))
                    .collect(),
            ))
            .build();
        let mut certificates = Vec::new();
        let mut token = None;
        let mut pagination = Pagination::new(max_pages);
        for _ in 0..max_pages {
            let page = client
                .list_certificates()
                .includes(includes.clone())
                .max_items(PAGE_SIZE)
                .set_next_token(token)
                .send()
                .await
                .context("ACM ListCertificates request failed")?;
            for entry in page.certificate_summary_list() {
                let arn = entry
                    .certificate_arn()
                    .filter(|arn| !arn.is_empty())
                    .context("ACM ListCertificates returned a certificate with no ARN")?;
                let id = certificate_id(arn)?;
                let text = |value: Option<&str>| value.unwrap_or_default().to_string();
                certificates.push(Certificate {
                    arn: arn.into(),
                    id: id.into(),
                    domain_name: text(entry.domain_name()),
                    additional_names: entry.subject_alternative_name_summaries().to_vec(),
                    more_names: entry
                        .has_additional_subject_alternative_names()
                        .unwrap_or(false),
                    cert_type: text(entry.r#type().map(|v| v.as_str())),
                    status: text(entry.status().map(|v| v.as_str())),
                    in_use: entry.in_use(),
                    exported: entry.exported(),
                    renewal_eligibility: text(entry.renewal_eligibility().map(|v| v.as_str())),
                    key_algorithm: text(entry.key_algorithm().map(|v| v.as_str())),
                    managed_by: text(entry.managed_by().map(|v| v.as_str())),
                    not_before: rfc3339(entry.not_before()),
                    not_after: rfc3339(entry.not_after()),
                    created_at: rfc3339(entry.created_at()),
                    issued_at: rfc3339(entry.issued_at()),
                    imported_at: rfc3339(entry.imported_at()),
                    revoked_at: rfc3339(entry.revoked_at()),
                });
            }
            match pagination.advance(page.next_token) {
                Ok(Some(next)) => token = Some(next),
                Ok(None) => {
                    certificates.sort_by(|a, b| {
                        a.domain_name
                            .cmp(&b.domain_name)
                            .then_with(|| a.arn.cmp(&b.arn))
                    });
                    return Ok(certificates);
                }
                Err(StopReason::RepeatedToken) => {
                    bail!("ACM ListCertificates returned a repeated pagination token; aborting")
                }
                Err(StopReason::PageLimit | StopReason::ResultLimit) => break,
            }
        }
        bail!("ACM ListCertificates exceeded {max_pages} pages; inventory is incomplete")
    }
}

#[cfg(test)]
mod tests;
