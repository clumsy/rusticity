//! SDK-replay tests for the read-only secret detail view. Secrets Manager speaks
//! AWS JSON 1.1: the operation is the `X-Amz-Target` header (`secretsmanager.<Op>`),
//! so each operation is answered, or failed, independently and every target seen
//! is recorded to prove no secret-value API is ever called.
use super::*;
use aws_smithy_http_client::test_util::infallible_client_fn;
use std::sync::{Arc, Mutex};

const REGION: &str = "us-east-1";
const ACCOUNT: &str = "123456789012";
const ARN: &str = "arn:aws:secretsmanager:us-east-1:123456789012:secret:prod/db-AbCdEf";
const CREATED: f64 = 1_576_540_098.0; // 2019-12-16T23:48:18Z
const LATER: f64 = 1_576_626_498.0; // 2019-12-17T23:48:18Z
const ACCESSED: f64 = 1_576_454_400.0; // 2019-12-16T00:00:00Z

type Responses = Arc<Mutex<std::collections::HashMap<&'static str, Vec<(u16, String)>>>>;

/// Each operation answers with a queue of responses (one per call, the last one
/// repeating), so pagination can be scripted.
#[derive(Clone)]
struct Mock {
    responses: Responses,
    seen: Arc<Mutex<Vec<(String, String)>>>,
}

impl Mock {
    fn new() -> Self {
        Self {
            responses: Arc::default(),
            seen: Arc::default(),
        }
    }
    fn on(self, op: &'static str, status: u16, body: impl Into<String>) -> Self {
        self.responses
            .lock()
            .unwrap()
            .entry(op)
            .or_default()
            .push((status, body.into()));
        self
    }
    fn ok(self, op: &'static str, body: impl Into<String>) -> Self {
        self.on(op, 200, body)
    }
    fn ops(&self) -> Vec<String> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|(op, _)| op.clone())
            .collect()
    }
    fn bodies(&self, op: &str) -> Vec<serde_json::Value> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .filter(|(seen, _)| seen == op)
            .map(|(_, body)| serde_json::from_str(body).unwrap())
            .collect()
    }
    fn config(&self) -> AwsConfig {
        let responses = self.responses.clone();
        let seen = self.seen.clone();
        let http = infallible_client_fn(move |request| {
            let op = request
                .headers()
                .get("x-amz-target")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .rsplit('.')
                .next()
                .unwrap_or_default()
                .to_string();
            let body = request
                .body()
                .bytes()
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .unwrap_or_default();
            seen.lock().unwrap().push((op.clone(), body));
            let mut responses = responses.lock().unwrap();
            let (status, body) = match responses.get_mut(op.as_str()) {
                Some(queue) if queue.len() > 1 => queue.remove(0),
                Some(queue) => queue[0].clone(),
                None => (400, format!("{{\"__type\":\"UnexpectedOperation:{op}\"}}")),
            };
            http::Response::builder()
                .status(status)
                .body(body.into_bytes())
                .unwrap()
        });
        let sdk_config = aws_config::SdkConfig::builder()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(REGION))
            .credentials_provider(
                aws_sdk_secretsmanager::config::SharedCredentialsProvider::new(
                    aws_sdk_secretsmanager::config::Credentials::new(
                        "test", "test", None, None, "test",
                    ),
                ),
            )
            .retry_config(aws_config::retry::RetryConfig::disabled())
            .http_client(http)
            .build();
        AwsConfig::from_sdk_config(sdk_config, REGION, ACCOUNT)
    }
}

fn describe_body() -> String {
    serde_json::json!({
        "ARN": ARN,
        "Name": "prod/db",
        "Description": "database credentials",
        "KmsKeyId": "alias/aws/secretsmanager",
        "RotationEnabled": true,
        "RotationLambdaARN": "arn:aws:lambda:us-east-1:123456789012:function:rotate",
        "RotationRules": {
            "AutomaticallyAfterDays": 30,
            "Duration": "2h",
            "ScheduleExpression": "rate(30 days)"
        },
        "LastRotatedDate": CREATED,
        "NextRotationDate": LATER,
        "LastChangedDate": LATER,
        "LastAccessedDate": ACCESSED,
        "CreatedDate": CREATED,
        "OwningService": "rds",
        "PrimaryRegion": REGION,
        "Tags": [
            {"Key": "team", "Value": "data"},
            {"Key": "env", "Value": "prod"}
        ],
        "ReplicationStatus": [
            {"Region": "us-west-2", "Status": "InSync", "KmsKeyId": "alias/aws/secretsmanager", "LastAccessedDate": ACCESSED},
            {"Region": "eu-west-1", "Status": "Failed", "StatusMessage": "AccessDenied on key"}
        ]
    })
    .to_string()
}

fn versions_body(entries: &[(&str, &[&str], f64)], next: Option<&str>) -> String {
    let list: Vec<_> = entries
        .iter()
        .map(|(id, stages, created)| {
            serde_json::json!({"VersionId": id, "VersionStages": stages, "CreatedDate": created})
        })
        .collect();
    let mut body = serde_json::json!({"ARN": ARN, "Name": "prod/db", "Versions": list});
    if let Some(next) = next {
        body["NextToken"] = serde_json::json!(next);
    }
    body.to_string()
}

#[tokio::test]
async fn detail_maps_metadata_rotation_replicas_tags_and_versions() {
    let mock = Mock::new().ok("DescribeSecret", describe_body()).ok(
        "ListSecretVersionIds",
        versions_body(
            &[
                ("v-old", &["AWSPREVIOUS"], CREATED),
                ("v-new", &["AWSCURRENT", "AWSPENDING"], LATER),
            ],
            None,
        ),
    );
    let detail = secret_details(&mock.config(), ARN).await.unwrap();
    let meta = &detail.metadata;
    assert_eq!(meta.name, "prod/db");
    assert_eq!(meta.arn, ARN);
    assert_eq!(meta.description, "database credentials");
    assert_eq!(meta.kms_key_id, "alias/aws/secretsmanager");
    assert_eq!(meta.owning_service, "rds");
    assert_eq!(meta.created_date, "2019-12-16T23:48:18Z");
    assert_eq!(meta.last_accessed_date, "2019-12-16");
    assert_eq!(meta.rotation.enabled, Some(true));
    assert_eq!(meta.rotation.automatically_after_days, Some(30));
    assert_eq!(meta.rotation.schedule_expression, "rate(30 days)");
    assert_eq!(meta.rotation.window, "2h");
    assert_eq!(meta.rotation.next_rotation_date, "2019-12-17T23:48:18Z");
    // Replicas and tags are sorted deterministically.
    let regions: Vec<_> = meta.replicas.iter().map(|r| r.region.as_str()).collect();
    assert_eq!(regions, ["eu-west-1", "us-west-2"]);
    assert_eq!(meta.replicas[0].status, "Failed");
    assert_eq!(meta.replicas[0].status_message, "AccessDenied on key");
    assert_eq!(meta.replicas[1].last_accessed_date, "2019-12-16");
    let keys: Vec<_> = meta.tags.iter().map(|t| t.key.as_str()).collect();
    assert_eq!(keys, ["env", "team"]);
    // Versions newest first with sorted staging labels.
    let versions = detail.versions.unwrap();
    assert_eq!(versions[0].version_id, "v-new");
    assert_eq!(versions[0].stages, ["AWSCURRENT", "AWSPENDING"]);
    assert_eq!(versions[1].version_id, "v-old");
}

#[tokio::test]
async fn only_metadata_operations_are_called_and_scoped_by_full_arn() {
    let mock = Mock::new()
        .ok("DescribeSecret", describe_body())
        .ok("ListSecretVersionIds", versions_body(&[], None));
    secret_details(&mock.config(), ARN).await.unwrap();
    assert_eq!(mock.ops(), ["DescribeSecret", "ListSecretVersionIds"]);
    for op in ["DescribeSecret", "ListSecretVersionIds"] {
        for body in mock.bodies(op) {
            assert_eq!(body["SecretId"], ARN);
        }
    }
    assert!(!mock
        .ops()
        .iter()
        .any(|op| op.contains("SecretValue") || op.starts_with("Put") || op.starts_with("Update")));
}

#[tokio::test]
async fn missing_rotation_flag_stays_unknown_not_disabled() {
    let body = serde_json::json!({"ARN": ARN, "Name": "prod/db"}).to_string();
    let mock = Mock::new()
        .ok("DescribeSecret", body)
        .ok("ListSecretVersionIds", versions_body(&[], None));
    let detail = secret_details(&mock.config(), ARN).await.unwrap();
    assert_eq!(detail.metadata.rotation.enabled, None);
    assert_eq!(detail.metadata.rotation.automatically_after_days, None);
    assert!(detail.metadata.created_date.is_empty());
    assert!(detail.metadata.replicas.is_empty());
    assert!(detail.versions.unwrap().is_empty());
}

#[tokio::test]
async fn versions_follow_pagination_tokens() {
    let mock = Mock::new()
        .ok("DescribeSecret", describe_body())
        .ok(
            "ListSecretVersionIds",
            versions_body(&[("v1", &["AWSPREVIOUS"], CREATED)], Some("page-2")),
        )
        .ok(
            "ListSecretVersionIds",
            versions_body(&[("v2", &["AWSCURRENT"], LATER)], None),
        );
    let versions = secret_details(&mock.config(), ARN)
        .await
        .unwrap()
        .versions
        .unwrap();
    assert_eq!(versions.len(), 2);
    let bodies = mock.bodies("ListSecretVersionIds");
    assert_eq!(bodies.len(), 2);
    assert_eq!(bodies[0]["MaxResults"], 100);
    assert!(bodies[0].get("NextToken").is_none());
    assert_eq!(bodies[1]["NextToken"], "page-2");
}

#[tokio::test]
async fn repeated_version_token_fails_the_section_not_the_detail() {
    let mock = Mock::new().ok("DescribeSecret", describe_body()).ok(
        "ListSecretVersionIds",
        versions_body(&[("v1", &["AWSCURRENT"], CREATED)], Some("same")),
    );
    let detail = secret_details(&mock.config(), ARN).await.unwrap();
    assert_eq!(detail.metadata.name, "prod/db");
    assert!(detail
        .versions
        .unwrap_err()
        .contains("repeated pagination token"));
}

#[tokio::test]
async fn denied_versions_surface_as_section_error() {
    let mock = Mock::new().ok("DescribeSecret", describe_body()).on(
        "ListSecretVersionIds",
        400,
        r#"{"__type":"AccessDeniedException","message":"not allowed"}"#,
    );
    let detail = secret_details(&mock.config(), ARN).await.unwrap();
    assert_eq!(detail.metadata.tags.len(), 2);
    let error = detail.versions.unwrap_err();
    assert!(error.contains("ListSecretVersionIds"), "{error}");
}

#[tokio::test]
async fn describe_failure_fails_the_whole_detail_without_listing_versions() {
    let mock = Mock::new().on(
        "DescribeSecret",
        400,
        r#"{"__type":"ResourceNotFoundException","message":"gone"}"#,
    );
    let error = secret_details(&mock.config(), ARN).await.unwrap_err();
    assert!(format!("{error:#}").contains("DescribeSecret"));
    assert_eq!(mock.ops(), ["DescribeSecret"]);
}

#[tokio::test]
async fn mismatched_identity_is_rejected() {
    let other = ARN.replace("prod/db-AbCdEf", "other-ZzZzZz");
    let body = serde_json::json!({"ARN": other, "Name": "other"}).to_string();
    let mock = Mock::new().ok("DescribeSecret", body);
    let error = secret_details(&mock.config(), ARN).await.unwrap_err();
    assert!(error.to_string().contains("mismatched identity"));
    assert_eq!(mock.ops(), ["DescribeSecret"]);
}

#[tokio::test]
async fn foreign_region_account_or_malformed_arn_is_rejected_before_any_request() {
    let mock = Mock::new();
    for arn in [
        "arn:aws:secretsmanager:eu-west-1:123456789012:secret:prod/db-AbCdEf",
        "arn:aws:secretsmanager:us-east-1:999999999999:secret:prod/db-AbCdEf",
        "arn:aws:kms:us-east-1:123456789012:key/abc",
        "prod/db",
    ] {
        assert!(secret_details(&mock.config(), arn).await.is_err(), "{arn}");
    }
    assert!(mock.ops().is_empty());
}
