use super::*;
use crate::AwsConfig;
use aws_smithy_http_client::test_util::infallible_client_fn;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

type Requests = Arc<Mutex<Vec<Value>>>;

/// Build an EcrClient whose HTTP transport replays the given (status, body)
/// pages in order, asserting the x-amz-target matches `target` and capturing
/// every request body for inspection.
fn client(target: &'static str, pages: Vec<(u16, Value)>) -> (EcrClient, Requests) {
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
                format!("AmazonEC2ContainerRegistry_V20150921.{target}")
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

// ── image_details (DescribeImages) ──────────────────────────────────────────

#[tokio::test]
async fn details_scope_the_immutable_digest_and_map_every_field() {
    let (client, requests) = client(
        "DescribeImages",
        vec![(
            200,
            json!({"imageDetails":[{
                "imageDigest":"sha256:immutable",
                "imageTags":["latest","v1.0.0"],
                "artifactMediaType":"application/vnd.oci.image.config.v1+json",
                "imageManifestMediaType":"application/vnd.docker.distribution.manifest.v2+json",
                "imagePushedAt":1700000000,
                "imageSizeInBytes":12345,
                "lastRecordedPullTime":1700000500,
                "imageStatus":"ACTIVE"
            }]}),
        )],
    );
    let details = client
        .image_details("nested/repo", "sha256:immutable")
        .await
        .unwrap();
    assert_eq!(details.digest, "sha256:immutable");
    assert_eq!(details.tags, ["latest", "v1.0.0"]);
    assert_eq!(
        details.artifact_type,
        "application/vnd.oci.image.config.v1+json"
    );
    assert_eq!(
        details.manifest_type,
        "application/vnd.docker.distribution.manifest.v2+json"
    );
    assert!(details.pushed_at.starts_with("2023-11-14"));
    assert_eq!(details.size_bytes, Some(12345));
    assert!(details.last_pull_time.starts_with("2023-11-14"));
    assert_eq!(details.image_status, "ACTIVE");
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0]["repositoryName"], "nested/repo");
    assert_eq!(
        requests[0]["imageIds"][0]["imageDigest"],
        "sha256:immutable"
    );
    assert!(requests[0]["imageIds"][0].get("imageTag").is_none());
}

#[tokio::test]
async fn details_fill_missing_optional_fields_with_dash_not_fabricated_values() {
    let (client, _) = client(
        "DescribeImages",
        vec![(200, json!({"imageDetails":[{"imageDigest":"sha256:bare"}]}))],
    );
    let details = client.image_details("repo", "sha256:bare").await.unwrap();
    assert!(details.tags.is_empty());
    assert_eq!(details.artifact_type, "-");
    assert_eq!(details.manifest_type, "-");
    assert_eq!(details.pushed_at, "-");
    assert_eq!(details.last_pull_time, "-");
    assert_eq!(details.image_status, "-");
    assert_eq!(details.size_bytes, None);
}

#[tokio::test]
async fn details_error_on_no_or_mismatched_image_rather_than_unrelated_data() {
    // Empty payload → no image found.
    let (empty, _) = client("DescribeImages", vec![(200, json!({"imageDetails":[]}))]);
    assert!(empty
        .image_details("repo", "sha256:absent")
        .await
        .unwrap_err()
        .to_string()
        .contains("No image found"));

    // A different digest in the payload must not be surfaced as the request's image.
    let (other, _) = client(
        "DescribeImages",
        vec![(
            200,
            json!({"imageDetails":[{"imageDigest":"sha256:other"}]}),
        )],
    );
    assert!(other
        .image_details("repo", "sha256:wanted")
        .await
        .unwrap_err()
        .to_string()
        .contains("No image found"));
}

#[tokio::test]
async fn details_surface_aws_error_code_and_never_request_without_scope() {
    let (failing, _) = client(
        "DescribeImages",
        vec![(
            400,
            json!({"__type":"ImageNotFoundException","message":"missing"}),
        )],
    );
    let error = failing.image_details("repo", "sha256:x").await.unwrap_err();
    assert!(error.to_string().contains("unavailable"));
    assert!(format!("{error:#}").contains("ImageNotFoundException"));

    let (scoped, requests) = client("DescribeImages", vec![]);
    assert!(scoped.image_details("repo", "").await.is_err());
    assert!(scoped.image_details("", "sha256:x").await.is_err());
    assert!(requests.lock().unwrap().is_empty());
}

// ── image_referrers (ListImageReferrers) ────────────────────────────────────

#[tokio::test]
async fn referrers_scope_subject_digest_paginate_and_map_fields() {
    let first = json!({
        "referrers":[{
            "digest":"sha256:sig",
            "mediaType":"application/vnd.oci.image.manifest.v1+json",
            "artifactType":"application/vnd.dev.cosign.simplesigning.v1+json",
            "size":512,
            "artifactStatus":"ACTIVE",
            "annotations":{"org.opencontainers.image.created":"2023"}
        }],
        "nextToken":"next"
    });
    let (client, requests) = client(
        "ListImageReferrers",
        vec![
            (200, first),
            (
                200,
                json!({"referrers":[{"digest":"sha256:sbom","mediaType":"application/spdx+json","size":1024}]}),
            ),
        ],
    );
    let report = client
        .image_referrers("nested/repo", "sha256:subject")
        .await
        .unwrap();
    assert!(report.warning.is_none());
    assert_eq!(report.entries.len(), 2);
    assert!(report.entries[0].contains(&("Referrer digest".into(), "sha256:sig".into())));
    // artifactType is preferred as the Type column when present.
    assert!(report.entries[0].contains(&(
        "Type".into(),
        "application/vnd.dev.cosign.simplesigning.v1+json".into()
    )));
    // Each entry carries exactly the two stable columns.
    assert_eq!(report.entries[0].len(), 2);
    // Falls back to mediaType when artifactType is absent.
    assert!(report.entries[1].contains(&("Referrer digest".into(), "sha256:sbom".into())));
    assert!(report.entries[1].contains(&("Type".into(), "application/spdx+json".into())));
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    for request in requests.iter() {
        assert_eq!(request["repositoryName"], "nested/repo");
        assert_eq!(request["subjectId"]["imageDigest"], "sha256:subject");
        assert!(request["subjectId"].get("imageTag").is_none());
        assert_eq!(request["maxResults"], 50);
    }
    assert_eq!(requests[1]["nextToken"], "next");
}

#[tokio::test]
async fn referrers_empty_payload_is_an_empty_report_not_an_error() {
    let (client, _) = client("ListImageReferrers", vec![(200, json!({"referrers":[]}))]);
    let report = client.image_referrers("repo", "sha256:x").await.unwrap();
    assert!(report.entries.is_empty());
    assert!(report.warning.is_none());
}

#[tokio::test]
async fn referrers_first_page_error_sets_unavailable_warning_with_code() {
    let (client, _) = client(
        "ListImageReferrers",
        vec![(
            400,
            json!({"__type":"AccessDeniedException","message":"denied"}),
        )],
    );
    let report = client.image_referrers("repo", "sha256:x").await.unwrap();
    assert!(report.entries.is_empty());
    let warning = report.warning.unwrap();
    assert!(warning.contains("Unavailable"));
    assert!(warning.contains("AccessDeniedException"));
}

#[tokio::test]
async fn referrers_preserve_partials_on_later_page_failure() {
    let (client, _) = client(
        "ListImageReferrers",
        vec![
            (
                200,
                json!({"referrers":[{"digest":"sha256:first","mediaType":"m","size":1}],"nextToken":"next"}),
            ),
            (
                400,
                json!({"__type":"AccessDeniedException","message":"denied"}),
            ),
        ],
    );
    let report = client.image_referrers("repo", "sha256:x").await.unwrap();
    assert_eq!(report.entries.len(), 1);
    assert!(report.warning.unwrap().contains("Partial results"));
}

#[tokio::test]
async fn referrers_repeated_token_is_bounded() {
    let page = json!({"referrers":[],"nextToken":"same"});
    let (client, requests) = client("ListImageReferrers", vec![(200, page.clone()), (200, page)]);
    let report = client.image_referrers("repo", "sha256:x").await.unwrap();
    assert!(report.warning.unwrap().contains("repeated"));
    assert_eq!(requests.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn referrers_cap_distinguishes_exactly_full_from_truncated() {
    for (count, next, limited) in [
        (ENTRY_LIMIT, None, false),
        (ENTRY_LIMIT + 1, None, true),
        (ENTRY_LIMIT, Some("more"), true),
    ] {
        let rows: Vec<_> = (0..count)
            .map(|i| json!({"digest":format!("sha256:{i}"),"mediaType":"m","size":1}))
            .collect();
        let page = match next {
            Some(token) => json!({"referrers":rows,"nextToken":token}),
            None => json!({"referrers":rows}),
        };
        let (client, requests) = client("ListImageReferrers", vec![(200, page)]);
        let report = client.image_referrers("repo", "sha256:x").await.unwrap();
        assert_eq!(report.entries.len(), ENTRY_LIMIT);
        assert_eq!(report.warning.is_some(), limited);
        assert_eq!(requests.lock().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn referrers_missing_scope_never_requests() {
    let (client, requests) = client("ListImageReferrers", vec![]);
    assert!(client.image_referrers("repo", "").await.is_err());
    assert!(client.image_referrers("", "sha256:x").await.is_err());
    assert!(requests.lock().unwrap().is_empty());
}

// ── image_replication (DescribeImageReplicationStatus) ──────────────────────

#[tokio::test]
async fn replication_by_digest_scopes_digest_and_maps_stable_columns() {
    let (client, requests) = client(
        "DescribeImageReplicationStatus",
        vec![(
            200,
            json!({"replicationStatuses":[
                {"region":"us-west-2","registryId":"999999999999","status":"COMPLETE"},
                {"region":"eu-west-1","status":"FAILED","failureCode":"InvalidTarget"}
            ]}),
        )],
    );
    let report = client
        .image_replication("nested/repo", "sha256:subject")
        .await
        .unwrap();
    assert!(report.warning.is_none());
    assert_eq!(report.entries.len(), 2);
    assert!(report.entries[0].contains(&("Target account".into(), "999999999999".into())));
    assert!(report.entries[0].contains(&("Region".into(), "us-west-2".into())));
    assert!(report.entries[0].contains(&("Status".into(), "COMPLETE".into())));
    assert!(report.entries[0].contains(&("Error".into(), "-".into())));
    assert!(report.entries[1].contains(&("Error".into(), "InvalidTarget".into())));
    assert!(report.entries[1].contains(&("Target account".into(), "-".into())));
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0]["repositoryName"], "nested/repo");
    assert_eq!(requests[0]["imageId"]["imageDigest"], "sha256:subject");
    // Only the chosen source field is sent — no tag leaks into a digest read.
    assert!(requests[0]["imageId"].get("imageTag").is_none());
}

#[tokio::test]
async fn replication_by_tag_sends_only_the_tag_and_refuses_blank() {
    let (tagged, requests) = client(
        "DescribeImageReplicationStatus",
        vec![(
            200,
            json!({"replicationStatuses":[{"region":"us-west-2","status":"IN_PROGRESS"}]}),
        )],
    );
    let report = tagged
        .image_replication_by_tag("nested/repo", "v1.0.0")
        .await
        .unwrap();
    assert_eq!(report.entries.len(), 1);
    assert!(report.entries[0].contains(&("Status".into(), "IN_PROGRESS".into())));
    {
        let captured = requests.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0]["imageId"]["imageTag"], "v1.0.0");
        // Only the chosen source field is sent — no digest leaks into a tag read.
        assert!(captured[0]["imageId"].get("imageDigest").is_none());
    }

    // A blank tag is refused before any request is made.
    let (blank, blank_requests) = client("DescribeImageReplicationStatus", vec![]);
    assert!(blank.image_replication_by_tag("repo", "").await.is_err());
    assert!(blank.image_replication_by_tag("", "v1").await.is_err());
    assert!(blank_requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn replication_missing_results_are_empty_and_errors_surface_code() {
    let (ok, _) = client("DescribeImageReplicationStatus", vec![(200, json!({}))]);
    let report = ok.image_replication("repo", "sha256:x").await.unwrap();
    assert!(report.entries.is_empty());
    assert!(report.warning.is_none());

    let (err, _) = client(
        "DescribeImageReplicationStatus",
        vec![(
            400,
            json!({"__type":"ImageNotFoundException","message":"missing"}),
        )],
    );
    let report = err.image_replication("repo", "sha256:x").await.unwrap();
    assert!(report.entries.is_empty());
    let warning = report.warning.unwrap();
    assert!(warning.contains("Unavailable"));
    assert!(warning.contains("ImageNotFoundException"));
}

#[tokio::test]
async fn replication_missing_scope_never_requests() {
    let (client, requests) = client("DescribeImageReplicationStatus", vec![]);
    assert!(client.image_replication("repo", "").await.is_err());
    assert!(client.image_replication("", "sha256:x").await.is_err());
    assert!(requests.lock().unwrap().is_empty());
}

// ── image_signing (DescribeImageSigningStatus) ──────────────────────────────

#[tokio::test]
async fn signing_scopes_digest_and_maps_profiles_and_failures() {
    let (client, requests) = client(
        "DescribeImageSigningStatus",
        vec![(
            200,
            json!({"signingStatuses":[
                {"status":"SIGNED","signingProfileArn":"arn:aws:signer:us-east-1:123456789012:/signing-profiles/p"},
                {"status":"FAILED","failureCode":"AccessDenied","failureReason":"no permission"}
            ]}),
        )],
    );
    let report = client
        .image_signing("nested/repo", "sha256:subject")
        .await
        .unwrap();
    assert!(report.warning.is_none());
    assert_eq!(report.entries.len(), 2);
    assert!(report.entries[0].contains(&("Status".into(), "SIGNED".into())));
    assert!(report.entries[0]
        .iter()
        .any(|(k, v)| k == "Signer profile" && v.ends_with("/signing-profiles/p")));
    assert!(report.entries[1].contains(&("Failure code".into(), "AccessDenied".into())));
    assert!(report.entries[1].contains(&("Failure reason".into(), "no permission".into())));
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0]["repositoryName"], "nested/repo");
    assert_eq!(requests[0]["imageId"]["imageDigest"], "sha256:subject");
    assert!(requests[0]["imageId"].get("imageTag").is_none());
}

#[tokio::test]
async fn signing_missing_results_are_empty_and_errors_surface_code() {
    let (ok, _) = client("DescribeImageSigningStatus", vec![(200, json!({}))]);
    let report = ok.image_signing("repo", "sha256:x").await.unwrap();
    assert!(report.entries.is_empty());
    assert!(report.warning.is_none());

    let (err, _) = client(
        "DescribeImageSigningStatus",
        vec![(400, json!({"__type":"ValidationException","message":"bad"}))],
    );
    let report = err.image_signing("repo", "sha256:x").await.unwrap();
    assert!(report.entries.is_empty());
    let warning = report.warning.unwrap();
    assert!(warning.contains("Unavailable"));
    assert!(warning.contains("ValidationException"));
}

#[tokio::test]
async fn signing_missing_scope_never_requests() {
    let (client, requests) = client("DescribeImageSigningStatus", vec![]);
    assert!(client.image_signing("repo", "").await.is_err());
    assert!(client.image_signing("", "sha256:x").await.is_err());
    assert!(requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn referrers_page_bound_keeps_partial_rows_but_allows_the_final_page() {
    for has_more in [false, true] {
        let pages = (0..PAGE_LIMIT).map(|i| {
            let token = if i + 1 < PAGE_LIMIT || has_more { Some(format!("page-{i}")) } else { None };
            (200, json!({"referrers":[{"digest":format!("sha256:{i}"),"mediaType":"test","size":1}],"nextToken":token}))
        }).collect();
        let (client, requests) = client("ListImageReferrers", pages);
        let report = client
            .image_referrers("repo", "sha256:subject")
            .await
            .unwrap();
        assert_eq!(requests.lock().unwrap().len(), PAGE_LIMIT);
        assert_eq!(report.entries.len(), PAGE_LIMIT);
        assert_eq!(
            report.warning,
            has_more.then(|| format!("Partial results: stopped after {PAGE_LIMIT} pages"))
        );
    }
}

#[tokio::test]
async fn referrers_longer_token_cycle_keeps_rows_with_original_warning() {
    let pages = ["a", "b", "a"].into_iter().map(|token| {
        (200, json!({"referrers":[{"digest":"sha256:row","mediaType":"test","size":1}],"nextToken":token}))
    }).collect();
    let (client, requests) = client("ListImageReferrers", pages);
    let report = client
        .image_referrers("repo", "sha256:subject")
        .await
        .unwrap();
    assert_eq!(report.entries.len(), 3);
    assert_eq!(requests.lock().unwrap().len(), 3);
    assert_eq!(
        report.warning.as_deref(),
        Some("Partial results: repeated pagination token")
    );
}
