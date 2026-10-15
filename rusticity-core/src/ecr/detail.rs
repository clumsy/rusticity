//! Read-only ECR image detail reads by immutable image digest. Never starts a
//! scan and never mutates registry state. Backs the image page's general
//! metadata, referrers, replication status and managed signing status tabs.
use super::EcrClient;
use anyhow::{anyhow, ensure, Result};
use aws_sdk_ecr::error::ProvideErrorMetadata;
use aws_sdk_ecr::types::ImageIdentifier;
use tokio::time::{timeout_at, Duration, Instant};

const ENTRY_LIMIT: usize = 1_000;
const PAGE_LIMIT: usize = 100;
const TIMEOUT: Duration = Duration::from_secs(30);
const MISSING: &str = "-";

/// General metadata for a single image, resolved by immutable digest.
#[derive(Debug)]
pub struct ImageDetails {
    pub tags: Vec<String>,
    pub digest: String,
    pub artifact_type: String,
    pub manifest_type: String,
    pub pushed_at: String,
    pub size_bytes: Option<i64>,
    pub last_pull_time: String,
    pub image_status: String,
}

/// A key/value report rendered as one block per entry (referrer, replication
/// destination, signing status). `warning` carries partial-result context,
/// including retained AWS error codes, without failing the whole read.
#[derive(Debug)]
pub struct DetailReport {
    pub entries: Vec<Vec<(String, String)>>,
    pub warning: Option<String>,
}

impl EcrClient {
    /// General image metadata via DescribeImages, filtered to the immutable
    /// digest. Validates the returned digest and errors on a missing or
    /// mismatched image rather than returning unrelated data.
    pub async fn image_details(&self, repository: &str, digest: &str) -> Result<ImageDetails> {
        ensure!(!repository.is_empty(), "Missing ECR repository name");
        ensure!(
            !digest.is_empty(),
            "Missing image digest; refresh the image list"
        );
        let client = self.config.ecr_client();
        let deadline = Instant::now() + TIMEOUT;
        let result = timeout_at(
            deadline,
            client
                .describe_images()
                .repository_name(repository)
                .image_ids(ImageIdentifier::builder().image_digest(digest).build())
                .send(),
        )
        .await;
        let response = match result {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => {
                return Err(anyhow!("Image metadata unavailable: {}", service(error)))
            }
            Err(_) => {
                return Err(anyhow!(
                    "Image metadata unavailable: request timed out after 30 seconds"
                ))
            }
        };
        let image = response
            .image_details
            .unwrap_or_default()
            .into_iter()
            .find(|image| image.image_digest.as_deref() == Some(digest))
            .ok_or_else(|| anyhow!("No image found for digest {digest} in {repository}"))?;
        // DescribeImages echoes the digest; guard against a shape that somehow
        // returns a different one so the page never shows an unrelated image.
        let returned = image.image_digest.unwrap_or_default();
        ensure!(
            returned == digest,
            "ECR returned digest {returned} for requested {digest}"
        );
        Ok(ImageDetails {
            tags: image.image_tags.unwrap_or_default(),
            digest: returned,
            artifact_type: image.artifact_media_type.unwrap_or_else(|| MISSING.into()),
            manifest_type: image
                .image_manifest_media_type
                .unwrap_or_else(|| MISSING.into()),
            pushed_at: image
                .image_pushed_at
                .map(timestamp)
                .unwrap_or_else(|| MISSING.into()),
            size_bytes: image.image_size_in_bytes,
            last_pull_time: image
                .last_recorded_pull_time
                .map(timestamp)
                .unwrap_or_else(|| MISSING.into()),
            image_status: image
                .image_status
                .map(|s| s.as_str().to_string())
                .unwrap_or_else(|| MISSING.into()),
        })
    }

    /// Referrers (e.g. signatures, SBOMs, attestations) via ListImageReferrers,
    /// scoped by the immutable subject digest. Paginated and bounded.
    pub async fn image_referrers(&self, repository: &str, digest: &str) -> Result<DetailReport> {
        let (repository, digest) = validate(repository, digest)?;
        let client = self.config.ecr_client();
        let deadline = Instant::now() + TIMEOUT;
        let mut report = DetailReport {
            entries: Vec::new(),
            warning: None,
        };
        let mut token: Option<String> = None;
        let mut seen = std::collections::HashSet::new();
        for page in 0..PAGE_LIMIT {
            let result = timeout_at(
                deadline,
                client
                    .list_image_referrers()
                    .repository_name(repository)
                    .subject_id(
                        aws_sdk_ecr::types::SubjectIdentifier::builder()
                            .image_digest(digest)
                            .build()
                            .map_err(|e| anyhow!("Invalid subject digest: {e}"))?,
                    )
                    .max_results(50)
                    .set_next_token(token.clone())
                    .send(),
            )
            .await;
            let response = match page_result(result, page, &mut report) {
                Some(response) => response,
                None => break,
            };
            for referrer in response.referrers.unwrap_or_default() {
                // Stable columns the image page renders: digest and artifact type.
                let kind = referrer
                    .artifact_type
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| nonempty(referrer.media_type));
                report.entries.push(vec![
                    ("Referrer digest".into(), nonempty(referrer.digest)),
                    ("Type".into(), kind),
                ]);
            }
            if cap_reached(&mut report, response.next_token.as_deref()) {
                break;
            }
            match next_token(response.next_token, &mut seen, &mut report, page) {
                Some(next) => token = Some(next),
                None => break,
            }
        }
        Ok(report)
    }

    /// Cross-region replication status via DescribeImageReplicationStatus,
    /// scoped by immutable digest. One entry per destination region.
    pub async fn image_replication(&self, repository: &str, digest: &str) -> Result<DetailReport> {
        let (repository, digest) = validate(repository, digest)?;
        self.replication_for(
            repository,
            ImageIdentifier::builder().image_digest(digest).build(),
        )
        .await
    }

    /// Cross-region replication status scoped by a selected tag instead of a
    /// digest. The parent's source selector uses this when the user picks a tag
    /// as the replication source. Refuses a blank tag.
    pub async fn image_replication_by_tag(
        &self,
        repository: &str,
        tag: &str,
    ) -> Result<DetailReport> {
        ensure!(!repository.is_empty(), "Missing ECR repository name");
        ensure!(!tag.is_empty(), "Missing image tag");
        self.replication_for(
            repository,
            ImageIdentifier::builder().image_tag(tag).build(),
        )
        .await
    }

    async fn replication_for(
        &self,
        repository: &str,
        image_id: ImageIdentifier,
    ) -> Result<DetailReport> {
        let client = self.config.ecr_client();
        let deadline = Instant::now() + TIMEOUT;
        let mut report = DetailReport {
            entries: Vec::new(),
            warning: None,
        };
        let result = timeout_at(
            deadline,
            client
                .describe_image_replication_status()
                .repository_name(repository)
                .image_id(image_id)
                .send(),
        )
        .await;
        // Not paginated: a single page carries all replication statuses.
        let response = match page_result(result, 0, &mut report) {
            Some(response) => response,
            None => return Ok(report),
        };
        for status in response.replication_statuses.unwrap_or_default() {
            report.entries.push(vec![
                (
                    "Target account".into(),
                    status.registry_id.map(nonempty).unwrap_or_else(dash),
                ),
                (
                    "Region".into(),
                    status.region.map(nonempty).unwrap_or_else(dash),
                ),
                (
                    "Status".into(),
                    status
                        .status
                        .map(|s| s.as_str().to_string())
                        .unwrap_or_else(dash),
                ),
                (
                    "Error".into(),
                    status.failure_code.map(nonempty).unwrap_or_else(dash),
                ),
            ]);
        }
        Ok(report)
    }

    /// Managed signing status via DescribeImageSigningStatus, scoped by
    /// immutable digest. One entry per signing profile.
    pub async fn image_signing(&self, repository: &str, digest: &str) -> Result<DetailReport> {
        let (repository, digest) = validate(repository, digest)?;
        let client = self.config.ecr_client();
        let deadline = Instant::now() + TIMEOUT;
        let mut report = DetailReport {
            entries: Vec::new(),
            warning: None,
        };
        let result = timeout_at(
            deadline,
            client
                .describe_image_signing_status()
                .repository_name(repository)
                .image_id(ImageIdentifier::builder().image_digest(digest).build())
                .send(),
        )
        .await;
        // Not paginated: a single page carries all signing statuses.
        let response = match page_result(result, 0, &mut report) {
            Some(response) => response,
            None => return Ok(report),
        };
        for status in response.signing_statuses.unwrap_or_default() {
            report.entries.push(vec![
                (
                    "Signer profile".into(),
                    status
                        .signing_profile_arn
                        .map(nonempty)
                        .unwrap_or_else(dash),
                ),
                (
                    "Status".into(),
                    status
                        .status
                        .map(|s| s.as_str().to_string())
                        .unwrap_or_else(dash),
                ),
                (
                    "Failure reason".into(),
                    status.failure_reason.map(nonempty).unwrap_or_else(dash),
                ),
                (
                    "Failure code".into(),
                    status.failure_code.map(nonempty).unwrap_or_else(dash),
                ),
            ]);
        }
        Ok(report)
    }
}

fn validate<'a>(repository: &'a str, digest: &'a str) -> Result<(&'a str, &'a str)> {
    ensure!(!repository.is_empty(), "Missing ECR repository name");
    ensure!(
        !digest.is_empty(),
        "Missing image digest; refresh the image list"
    );
    Ok((repository, digest))
}

/// Render a page-fetch result into a response, recording a warning and
/// signalling a stop on error/timeout. Errors on page 0 are returned to the
/// caller via the DetailReport warning so partials from later pages survive.
fn page_result<T, E>(
    result: Result<Result<T, E>, tokio::time::error::Elapsed>,
    page: usize,
    report: &mut DetailReport,
) -> Option<T>
where
    E: ProvideErrorMetadata + std::error::Error + Send + Sync + 'static,
{
    match result {
        Ok(Ok(response)) => Some(response),
        Ok(Err(error)) => {
            let detail = service(error);
            report.warning = Some(if page == 0 {
                format!("Unavailable: {detail}")
            } else {
                format!("Partial results: {detail}")
            });
            None
        }
        Err(_) => {
            report.warning = Some(if page == 0 {
                "Unavailable: request timed out after 30 seconds".into()
            } else {
                "Partial results: request timed out after 30 seconds".into()
            });
            None
        }
    }
}

/// True when the entry cap is hit; truncates and records a warning. Keeping one
/// extra entry (via the strict `>`) and the live token lets exactly-at-limit be
/// told apart from truncated.
fn cap_reached(report: &mut DetailReport, next_token: Option<&str>) -> bool {
    let has_more = next_token.map(|t| !t.is_empty()).unwrap_or(false);
    if report.entries.len() > ENTRY_LIMIT || (report.entries.len() == ENTRY_LIMIT && has_more) {
        report.entries.truncate(ENTRY_LIMIT);
        report.warning = Some(format!("Partial results: limited to {ENTRY_LIMIT} entries"));
        return true;
    }
    false
}

/// Resolve the next pagination token, stopping (returns None) on an empty
/// token, a repeated token, or the final allowed page.
fn next_token(
    token: Option<String>,
    seen: &mut std::collections::HashSet<String>,
    report: &mut DetailReport,
    page: usize,
) -> Option<String> {
    let next = token.filter(|t| !t.is_empty())?;
    if !seen.insert(next.clone()) {
        report.warning = Some("Partial results: repeated pagination token".into());
        return None;
    }
    if page + 1 == PAGE_LIMIT {
        report.warning = Some(format!("Partial results: stopped after {PAGE_LIMIT} pages"));
        return None;
    }
    Some(next)
}

fn service<E>(error: E) -> String
where
    E: ProvideErrorMetadata + std::error::Error + Send + Sync + 'static,
{
    if let Some(code) = error.code() {
        format!(
            "{}: {}",
            code,
            error.message().unwrap_or("No details reported")
        )
    } else {
        format!("{:#}", anyhow::Error::new(error))
    }
}

fn timestamp(date: aws_smithy_types::DateTime) -> String {
    date.fmt(aws_smithy_types::date_time::Format::DateTime)
        .unwrap_or_default()
}

fn nonempty(value: String) -> String {
    if value.is_empty() {
        MISSING.into()
    } else {
        value
    }
}

fn dash() -> String {
    MISSING.into()
}

#[cfg(test)]
mod tests;
