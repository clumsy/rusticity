//! Read existing private ECR scan results by immutable image digest. Never starts a scan.
use super::EcrClient;
use anyhow::{anyhow, ensure, Result};
use aws_sdk_ecr::types::{EnhancedImageScanFinding, ImageIdentifier, ImageScanFinding};
use std::collections::{BTreeMap, HashSet};
use tokio::time::{timeout_at, Duration, Instant};

pub const FINDING_LIMIT: usize = 1_000;
const PAGE_LIMIT: usize = 100;
const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Default)]
pub struct ScanReport {
    pub status: String,
    pub description: String,
    pub completed_at: String,
    pub source_updated_at: String,
    /// AWS's summary, not a count derived from a potentially truncated list.
    pub severity_counts: BTreeMap<String, i32>,
    pub findings: Vec<Finding>,
    pub has_findings_payload: bool,
    pub warning: Option<String>,
}

#[derive(Debug)]
pub struct Finding {
    pub severity: String,
    pub name: String,
    pub description: String,
    pub details: Vec<(String, String)>,
}

impl ScanReport {
    pub fn complete(&self) -> bool {
        self.has_findings_payload
            && matches!(self.status.as_str(), "COMPLETE" | "ACTIVE")
            && self.warning.is_none()
    }
}

impl EcrClient {
    pub async fn image_scan_findings(&self, repository: &str, digest: &str) -> Result<ScanReport> {
        ensure!(!repository.is_empty(), "Missing ECR repository name");
        ensure!(
            !digest.is_empty(),
            "Missing image digest; refresh the image list"
        );
        let client = self.config.ecr_client();
        let mut report = ScanReport::default();
        let mut token = None;
        let mut seen = HashSet::new();
        let deadline = Instant::now() + TIMEOUT;
        for page in 0..PAGE_LIMIT {
            let result = timeout_at(
                deadline,
                client
                    .describe_image_scan_findings()
                    .repository_name(repository)
                    .image_id(ImageIdentifier::builder().image_digest(digest).build())
                    .max_results(100)
                    .set_next_token(token)
                    .send(),
            )
            .await;
            let response = match result {
                Ok(Ok(response)) => response,
                result => {
                    let error = match result {
                        Ok(Err(error)) => {
                            use aws_sdk_ecr::error::ProvideErrorMetadata;
                            if let Some(service) = error.as_service_error() {
                                format!(
                                    "{}: {}",
                                    service.code().unwrap_or("ECR error"),
                                    service.message().unwrap_or("No details reported")
                                )
                            } else {
                                format!("{:#}", anyhow::Error::new(error))
                            }
                        }
                        _ => "Scan findings request timed out after 30 seconds".into(),
                    };
                    if page == 0 {
                        return Err(anyhow!("Scan findings unavailable: {error}"));
                    }
                    report.warning = Some(format!("Partial results: {error}"));
                    break;
                }
            };
            if let Some(status) = response.image_scan_status {
                report.status = status.status.map(|s| s.as_str().into()).unwrap_or_default();
                report.description = status.description.unwrap_or_default();
            }
            if let Some(findings) = response.image_scan_findings {
                report.has_findings_payload = true;
                if let Some(date) = findings.image_scan_completed_at {
                    report.completed_at = timestamp(date);
                }
                if let Some(date) = findings.vulnerability_source_updated_at {
                    report.source_updated_at = timestamp(date);
                }
                if let Some(counts) = findings.finding_severity_counts {
                    report.severity_counts = counts
                        .into_iter()
                        .map(|(severity, count)| (severity.as_str().into(), count))
                        .collect();
                }
                let rows = findings
                    .findings
                    .unwrap_or_default()
                    .into_iter()
                    .map(basic)
                    .chain(
                        findings
                            .enhanced_findings
                            .unwrap_or_default()
                            .into_iter()
                            .map(enhanced),
                    );
                // Keep one extra to distinguish exactly-at-limit from truncated results.
                let remaining = FINDING_LIMIT + 1 - report.findings.len();
                report.findings.extend(rows.take(remaining));
            } else {
                report.warning = Some("Scan findings payload unavailable; refresh to retry".into());
                break;
            }
            token = response.next_token.filter(|token| !token.is_empty());
            if report.findings.len() > FINDING_LIMIT
                || (report.findings.len() == FINDING_LIMIT && token.is_some())
            {
                report.findings.truncate(FINDING_LIMIT);
                report.warning = Some(format!(
                    "Partial results: limited to {FINDING_LIMIT} findings"
                ));
                break;
            }
            let Some(next) = token.as_ref() else { break };
            if !seen.insert(next.clone()) {
                report.warning = Some("Partial results: repeated pagination token".into());
                break;
            }
            if page + 1 == PAGE_LIMIT {
                report.warning = Some(format!("Partial results: stopped after {PAGE_LIMIT} pages"));
            }
        }
        report.findings.sort_by(|a, b| {
            severity_rank(&a.severity)
                .cmp(&severity_rank(&b.severity))
                .then_with(|| a.name.cmp(&b.name))
        });
        Ok(report)
    }
}

fn timestamp(date: aws_smithy_types::DateTime) -> String {
    date.fmt(aws_smithy_types::date_time::Format::DateTime)
        .unwrap_or_default()
}

fn severity_rank(severity: &str) -> usize {
    [
        "CRITICAL",
        "HIGH",
        "MEDIUM",
        "LOW",
        "INFORMATIONAL",
        "UNDEFINED",
    ]
    .iter()
    .position(|s| *s == severity)
    .unwrap_or(6)
}

fn basic(finding: ImageScanFinding) -> Finding {
    let mut details = vec![("Scanner".into(), "Basic".into())];
    if let Some(uri) = finding.uri {
        details.push(("Reference".into(), uri));
    }
    for attribute in finding.attributes.unwrap_or_default() {
        details.push((attribute.key, attribute.value.unwrap_or_else(|| "-".into())));
    }
    Finding {
        severity: finding
            .severity
            .map(|s| s.as_str().into())
            .unwrap_or_else(|| "UNKNOWN".into()),
        name: finding.name.unwrap_or_else(|| "Unnamed finding".into()),
        description: finding.description.unwrap_or_default(),
        details,
    }
}

fn enhanced(finding: EnhancedImageScanFinding) -> Finding {
    let mut details = vec![("Scanner".into(), "Enhanced (Inspector)".into())];
    for (label, value) in [
        ("Status", finding.status),
        ("Fix available", finding.fix_available),
        ("Exploit available", finding.exploit_available),
        ("Finding ARN", finding.finding_arn),
    ] {
        if let Some(value) = value {
            details.push((label.into(), value));
        }
    }
    let mut name = finding.title.unwrap_or_else(|| "Unnamed finding".into());
    if let Some(vulnerability) = finding.package_vulnerability_details {
        if let Some(id) = vulnerability.vulnerability_id {
            name = id;
        }
        if let Some(url) = vulnerability.source_url {
            details.push(("Reference".into(), url));
        }
        for package in vulnerability.vulnerable_packages.unwrap_or_default() {
            details.push(("Package".into(), package.name.unwrap_or_else(|| "-".into())));
            for (label, value) in [
                ("Installed version", package.version),
                ("Release", package.release),
                ("Architecture", package.arch),
                ("Package manager", package.package_manager),
                ("Path", package.file_path),
            ] {
                if let Some(value) = value {
                    details.push((label.into(), value));
                }
            }
            if let Some(epoch) = package.epoch {
                details.push(("Epoch".into(), epoch.to_string()));
            }
            details.push((
                "Fixed in".into(),
                package
                    .fixed_in_version
                    .unwrap_or_else(|| "Not reported".into()),
            ));
        }
    }
    Finding {
        severity: finding.severity.unwrap_or_else(|| "UNKNOWN".into()),
        name,
        description: finding.description.unwrap_or_default(),
        details,
    }
}

#[cfg(test)]
mod tests;
