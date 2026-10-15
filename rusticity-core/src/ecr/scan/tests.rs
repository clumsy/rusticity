use super::*;
use crate::AwsConfig;
use aws_smithy_http_client::test_util::infallible_client_fn;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

type Requests = Arc<Mutex<Vec<Value>>>;
fn client(pages: Vec<(u16, Value)>) -> (EcrClient, Requests) {
    let requests = Arc::new(Mutex::new(vec![]));
    let captured = requests.clone();
    let pages = Arc::new(Mutex::new(pages.into_iter()));
    let sdk = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-1"))
        .credentials_provider(aws_sdk_ecr::config::SharedCredentialsProvider::new(
            aws_sdk_ecr::config::Credentials::new("test", "test", None, None, "test"),
        ))
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(infallible_client_fn(move |request| {
            assert_eq!(
                request.headers()["x-amz-target"],
                "AmazonEC2ContainerRegistry_V20150921.DescribeImageScanFindings"
            );
            captured
                .lock()
                .unwrap()
                .push(serde_json::from_slice(request.body().bytes().unwrap()).unwrap());
            let (status, response) = pages.lock().unwrap().next().expect("unexpected request");
            http::Response::builder()
                .status(status)
                .header("content-type", "application/x-amz-json-1.1")
                .body(response.to_string().into_bytes())
                .unwrap()
        }))
        .build();
    (
        EcrClient::new(AwsConfig::from_sdk_config(sdk, "us-east-1", "123456789012")),
        requests,
    )
}
fn page(status: &str, findings: Value, token: Option<&str>) -> Value {
    json!({"imageScanStatus":{"status":status},"imageScanFindings":{"findings":findings},"nextToken":token})
}

#[tokio::test]
async fn scan_pages_basic_findings_by_digest_and_sorts_severity() {
    let first = json!({
        "imageScanStatus":{"status":"COMPLETE"},"nextToken":"next",
        "imageScanFindings":{
            "imageScanCompletedAt":1700000000,
            "vulnerabilitySourceUpdatedAt":1700000001,
            "findingSeverityCounts":{"LOW":1,"CRITICAL":1},
            "findings":[{"name":"CVE-low","severity":"LOW","attributes":[{"key":"package_name","value":"openssl"}]}]
        }
    });
    let (client, requests) = client(vec![
        (200, first),
        (
            200,
            page(
                "COMPLETE",
                json!([
                    {"name":"CVE-critical","severity":"CRITICAL","uri":"https://example.invalid/cve"}
                ]),
                None,
            ),
        ),
    ]);
    let report = client
        .image_scan_findings("nested/repo", "sha256:immutable")
        .await
        .unwrap();
    assert!(report.complete());
    assert_eq!(report.findings.len(), 2);
    assert_eq!(report.findings[0].name, "CVE-critical");
    assert!(report.findings[1]
        .details
        .contains(&("package_name".into(), "openssl".into())));
    assert_eq!(report.severity_counts["LOW"], 1);
    assert!(report.completed_at.starts_with("2023-11-14"));
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    for request in requests.iter() {
        assert_eq!(request["repositoryName"], "nested/repo");
        assert_eq!(request["imageId"]["imageDigest"], "sha256:immutable");
        assert!(request["imageId"].get("imageTag").is_none());
        assert_eq!(request["maxResults"], 100);
    }
    assert_eq!(requests[1]["nextToken"], "next");
}

#[tokio::test]
async fn scan_enhanced_preserves_all_packages_and_missing_fix_information() {
    let (client, _) = client(vec![(
        200,
        json!({
            "imageScanStatus":{"status":"ACTIVE"},
            "imageScanFindings":{"enhancedFindings":[{
                "severity":"HIGH","status":"ACTIVE","title":"title","fixAvailable":"PARTIAL",
                "packageVulnerabilityDetails":{"vulnerabilityId":"CVE-123","vulnerablePackages":[
                    {"name":"first","version":"1.0","fixedInVersion":"1.1","epoch":0,"release":"2","filePath":"/usr/lib/first","arch":"x86_64"},
                    {"name":"second","version":"2.0"}
                ]}
            }]}
        }),
    )]);
    let report = client.image_scan_findings("repo", "digest").await.unwrap();
    assert!(report.complete());
    let finding = &report.findings[0];
    assert_eq!(finding.name, "CVE-123");
    for pair in [
        ("Package", "first"),
        ("Package", "second"),
        ("Fixed in", "1.1"),
        ("Fixed in", "Not reported"),
        ("Epoch", "0"),
        ("Fix available", "PARTIAL"),
    ] {
        assert!(
            finding.details.contains(&(pair.0.into(), pair.1.into())),
            "missing {pair:?}"
        );
    }
}

#[tokio::test]
async fn scan_absent_pending_failed_and_unknown_are_not_complete() {
    for status in [
        "IN_PROGRESS",
        "PENDING",
        "FAILED",
        "UNSUPPORTED_IMAGE",
        "SCAN_ELIGIBILITY_EXPIRED",
        "FINDINGS_UNAVAILABLE",
        "FUTURE_STATUS",
    ] {
        let (client, _) = client(vec![(200, page(status, json!([]), None))]);
        let report = client.image_scan_findings("repo", "digest").await.unwrap();
        assert_eq!(report.status, status);
        assert!(!report.complete());
    }
    let (client, _) = client(vec![(
        200,
        json!({"imageScanStatus":{"status":"COMPLETE"}}),
    )]);
    let report = client.image_scan_findings("repo", "digest").await.unwrap();
    assert!(!report.complete());
    assert!(report.warning.unwrap().contains("payload unavailable"));
}

#[tokio::test]
async fn scan_unavailable_and_denied_reads_are_errors_not_empty_success() {
    for code in [
        "ScanNotFoundException",
        "AccessDeniedException",
        "ImageNotFoundException",
    ] {
        let (client, _) = client(vec![(
            400,
            json!({"__type":code,"message":"not available"}),
        )]);
        let error = client
            .image_scan_findings("repo", "digest")
            .await
            .unwrap_err();
        assert!(error.to_string().contains("unavailable"));
        assert!(format!("{error:#}").contains(code));
    }
}

#[tokio::test]
async fn scan_preserves_partial_results_on_later_page_failure() {
    let (client, _) = client(vec![
        (
            200,
            page(
                "COMPLETE",
                json!([{"name":"CVE-first","severity":"HIGH"}]),
                Some("next"),
            ),
        ),
        (
            400,
            json!({"__type":"AccessDeniedException","message":"denied"}),
        ),
    ]);
    let report = client.image_scan_findings("repo", "digest").await.unwrap();
    assert_eq!(report.findings.len(), 1);
    assert!(!report.complete());
    assert!(report.warning.unwrap().contains("Partial results"));
}

#[tokio::test]
async fn scan_repeated_token_and_empty_pages_are_bounded() {
    let (client, requests) = client(vec![
        (200, page("COMPLETE", json!([]), Some("same"))),
        (200, page("COMPLETE", json!([]), Some("same"))),
    ]);
    let report = client.image_scan_findings("repo", "digest").await.unwrap();
    assert!(!report.complete());
    assert!(report.warning.unwrap().contains("repeated"));
    assert_eq!(requests.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn scan_cap_distinguishes_exactly_full_from_truncated() {
    for (count, next, limited) in [
        (FINDING_LIMIT, None, false),
        (FINDING_LIMIT + 1, None, true),
        (FINDING_LIMIT, Some("more"), true),
    ] {
        let rows: Vec<_> = (0..count)
            .map(|i| json!({"name":format!("CVE-{i}"),"severity":"LOW"}))
            .collect();
        let (client, requests) = client(vec![(200, page("COMPLETE", json!(rows), next))]);
        let report = client.image_scan_findings("repo", "digest").await.unwrap();
        assert_eq!(report.findings.len(), FINDING_LIMIT);
        assert_eq!(report.warning.is_some(), limited);
        assert_eq!(requests.lock().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn scan_page_limit_handles_unique_tokens_without_findings() {
    let pages = (0..PAGE_LIMIT)
        .map(|i| (200, page("COMPLETE", json!([]), Some(&i.to_string()))))
        .collect();
    let (client, requests) = client(pages);
    let report = client.image_scan_findings("repo", "digest").await.unwrap();
    assert!(!report.complete());
    assert!(report.warning.unwrap().contains("100 pages"));
    assert_eq!(requests.lock().unwrap().len(), PAGE_LIMIT);
}

#[tokio::test]
async fn scan_missing_digest_never_makes_a_request() {
    let (client, requests) = client(vec![]);
    assert!(client.image_scan_findings("repo", "").await.is_err());
    assert!(client.image_scan_findings("", "digest").await.is_err());
    assert!(requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn scan_final_page_at_page_bound_is_complete() {
    let pages = (0..PAGE_LIMIT)
        .map(|i| {
            let token = (i + 1 < PAGE_LIMIT).then(|| format!("page-{i}"));
            (200, page("COMPLETE", json!([]), token.as_deref()))
        })
        .collect();
    let (client, requests) = client(pages);
    let report = client.image_scan_findings("repo", "digest").await.unwrap();
    assert_eq!(requests.lock().unwrap().len(), PAGE_LIMIT);
    assert!(report.complete());
    assert!(report.warning.is_none());
}

#[tokio::test]
async fn scan_longer_token_cycle_keeps_rows_with_original_warning() {
    let pages = ["a", "b", "a"]
        .into_iter()
        .map(|token| {
            (
                200,
                page(
                    "COMPLETE",
                    json!([{"name":"CVE-test","severity":"HIGH"}]),
                    Some(token),
                ),
            )
        })
        .collect();
    let (client, requests) = client(pages);
    let report = client.image_scan_findings("repo", "digest").await.unwrap();
    assert_eq!(requests.lock().unwrap().len(), 3);
    assert_eq!(report.findings.len(), 3);
    assert_eq!(
        report.warning.as_deref(),
        Some("Partial results: repeated pagination token")
    );
    assert!(!report.complete());
}
