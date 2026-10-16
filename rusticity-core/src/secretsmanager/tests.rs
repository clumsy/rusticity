use super::*;
use aws_smithy_http_client::test_util::{ReplayEvent, StaticReplayClient};
use aws_smithy_types::body::SdkBody;

const ENDPOINT: &str = "https://secretsmanager.us-east-1.amazonaws.com/";

// Known epochs (seconds). ListSecrets uses the AWS JSON 1.1 protocol and
// serializes timestamps as EpochSeconds floats.
const CREATED: f64 = 1_576_540_098.0; // 2019-12-16T23:48:18Z
const CHANGED: f64 = 1_576_626_498.0; // 2019-12-17T23:48:18Z
const ROTATED: f64 = 1_576_453_698.0; // 2019-12-15T23:48:18Z
const ACCESSED_MIDNIGHT: f64 = 1_576_454_400.0; // 2019-12-16T00:00:00Z -> date 2019-12-16

/// One SecretList entry. Fields set to `None` are omitted entirely so the
/// tests can assert absent-vs-false and empty-vs-invented behaviour.
#[derive(Default, Clone)]
struct Entry {
    arn: &'static str,
    name: &'static str,
    description: Option<&'static str>,
    rotation_enabled: Option<bool>,
    created: Option<f64>,
    changed: Option<f64>,
    accessed: Option<f64>,
    rotated: Option<f64>,
}

fn entry_json(e: &Entry) -> String {
    let mut fields = Vec::new();
    fields.push(format!("\"ARN\":\"{}\"", e.arn));
    fields.push(format!("\"Name\":\"{}\"", e.name));
    if let Some(d) = e.description {
        fields.push(format!("\"Description\":\"{d}\""));
    }
    if let Some(r) = e.rotation_enabled {
        fields.push(format!("\"RotationEnabled\":{r}"));
    }
    if let Some(t) = e.created {
        fields.push(format!("\"CreatedDate\":{t}"));
    }
    if let Some(t) = e.changed {
        fields.push(format!("\"LastChangedDate\":{t}"));
    }
    if let Some(t) = e.accessed {
        fields.push(format!("\"LastAccessedDate\":{t}"));
    }
    if let Some(t) = e.rotated {
        fields.push(format!("\"LastRotatedDate\":{t}"));
    }
    format!("{{{}}}", fields.join(","))
}

fn page(entries: &[Entry], next_token: Option<&str>) -> String {
    let list: Vec<_> = entries.iter().map(entry_json).collect();
    let token = next_token
        .map(|t| format!(",\"NextToken\":\"{t}\""))
        .unwrap_or_default();
    format!("{{\"SecretList\":[{}]{}}}", list.join(","), token)
}

fn ok(response: String) -> ReplayEvent {
    event(200, response)
}

fn event(status: u16, response: String) -> ReplayEvent {
    ReplayEvent::new(
        http::Request::builder()
            .method("POST")
            .uri(ENDPOINT)
            .body(SdkBody::from(""))
            .unwrap(),
        http::Response::builder()
            .status(status)
            .body(SdkBody::from(response))
            .unwrap(),
    )
}

fn client(http_client: &StaticReplayClient) -> SecretsManagerClient {
    let sdk_config = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-1"))
        .credentials_provider(
            aws_sdk_secretsmanager::config::SharedCredentialsProvider::new(
                aws_sdk_secretsmanager::config::Credentials::new(
                    "test-key",
                    "test-secret",
                    None,
                    None,
                    "test",
                ),
            ),
        )
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(http_client.clone())
        .build();
    SecretsManagerClient::new(AwsConfig::from_sdk_config(
        sdk_config,
        "us-east-1",
        "123456789012",
    ))
}

fn secret(name: &'static str, arn: &'static str) -> Entry {
    Entry {
        arn,
        name,
        ..Default::default()
    }
}

// ── ARN validation ───────────────────────────────────────────────────────────

#[test]
fn validate_secret_arn_accepts_wellformed_and_rejects_others() {
    validate_secret_arn("arn:aws:secretsmanager:us-east-1:123456789012:secret:prod/db-AbCdEf")
        .unwrap();
    // Non-default partition is still valid.
    validate_secret_arn("arn:aws-us-gov:secretsmanager:us-gov-west-1:123456789012:secret:s-A1b2C3")
        .unwrap();
    for bad in [
        "",
        "not-an-arn",
        "arn:aws:sns:us-east-1:123456789012:topic", // wrong service
        "arn:aws:secretsmanager:us-east-1:123456789012:", // empty name segment
        "arn:aws:secretsmanager::123456789012:secret:x", // empty region
        "arn:aws:secretsmanager:us-east-1:not-account:secret:x", // bad account
        "arn::secretsmanager:us-east-1:123456789012:secret:x", // empty partition
    ] {
        assert!(
            validate_secret_arn(bad).is_err(),
            "{bad:?} should be rejected"
        );
    }
}

// ── pagination ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_secrets_follows_tokens_and_sorts_until_empty_token() {
    let http = StaticReplayClient::new(vec![
        ok(page(
            &[
                secret(
                    "zeta",
                    "arn:aws:secretsmanager:us-east-1:123456789012:secret:zeta-ZZ1111",
                ),
                secret(
                    "alpha",
                    "arn:aws:secretsmanager:us-east-1:123456789012:secret:alpha-AA1111",
                ),
            ],
            Some("tok-1"),
        )),
        ok(page(
            &[secret(
                "mid",
                "arn:aws:secretsmanager:us-east-1:123456789012:secret:mid-MM1111",
            )],
            // Final page signalled by an EMPTY token (must still terminate).
            Some(""),
        )),
    ]);
    let secrets = client(&http).list_secrets().await.unwrap();
    let names: Vec<_> = secrets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["alpha", "mid", "zeta"]); // sorted by name then ARN
}

#[tokio::test]
async fn list_secrets_terminates_on_absent_token() {
    let http = StaticReplayClient::new(vec![ok(page(
        &[secret(
            "only",
            "arn:aws:secretsmanager:us-east-1:123456789012:secret:only-OO1111",
        )],
        None,
    ))]);
    let secrets = client(&http).list_secrets().await.unwrap();
    assert_eq!(secrets.len(), 1);
    assert_eq!(secrets[0].name, "only");
    assert_eq!(
        secrets[0].arn,
        "arn:aws:secretsmanager:us-east-1:123456789012:secret:only-OO1111"
    );
}

#[tokio::test]
async fn list_secrets_handles_empty_account() {
    let http = StaticReplayClient::new(vec![ok(page(&[], None))]);
    let secrets = client(&http).list_secrets().await.unwrap();
    assert!(secrets.is_empty());
}

#[tokio::test]
async fn list_secrets_errors_on_repeated_token_instead_of_looping() {
    let http = StaticReplayClient::new(vec![
        ok(page(
            &[secret(
                "a",
                "arn:aws:secretsmanager:us-east-1:123456789012:secret:a-AA1111",
            )],
            Some("same"),
        )),
        ok(page(
            &[secret(
                "b",
                "arn:aws:secretsmanager:us-east-1:123456789012:secret:b-BB1111",
            )],
            Some("same"),
        )),
    ]);
    let err = client(&http).list_secrets().await.unwrap_err();
    assert!(
        err.to_string().contains("repeated pagination token"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn detects_longer_token_cycles_and_reports_page_bound() {
    let http = StaticReplayClient::new(vec![
        ok(page(&[], Some("a"))),
        ok(page(&[], Some("b"))),
        ok(page(&[], Some("a"))),
    ]);
    assert!(client(&http)
        .list_secrets()
        .await
        .unwrap_err()
        .to_string()
        .contains("repeated pagination token"));
    let http = StaticReplayClient::new(vec![ok(page(&[], Some("a"))), ok(page(&[], Some("b")))]);
    assert!(client(&http)
        .list_secrets_inner(2)
        .await
        .unwrap_err()
        .to_string()
        .contains("inventory is incomplete"));
}

#[tokio::test]
async fn final_page_at_bound_is_complete_and_zero_bound_never_requests() {
    let http = StaticReplayClient::new(vec![
        ok(page(&[], Some("next"))),
        ok(page(
            &[secret(
                "s",
                "arn:aws:secretsmanager:us-east-1:123456789012:secret:s-SS1111",
            )],
            None,
        )),
    ]);
    let secrets = client(&http).list_secrets_inner(2).await.unwrap();
    assert_eq!(secrets.len(), 1);
    let http = StaticReplayClient::new(vec![]);
    let err = client(&http).list_secrets_inner(0).await.unwrap_err();
    assert_eq!(
        err.to_string(),
        "Secrets Manager ListSecrets exceeded 0 pages; inventory is incomplete"
    );
}

// ── error / malformed handling ───────────────────────────────────────────────

#[tokio::test]
async fn list_secrets_surfaces_malformed_arn_as_error_not_empty_success() {
    let http = StaticReplayClient::new(vec![ok(page(
        &[
            secret(
                "good",
                "arn:aws:secretsmanager:us-east-1:123456789012:secret:good-GG1111",
            ),
            secret("bad", "garbage-arn"),
        ],
        None,
    ))]);
    let err = client(&http).list_secrets().await.unwrap_err();
    assert!(
        err.to_string().contains("not a Secrets Manager secret ARN"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn list_secrets_errors_when_name_missing() {
    // An entry with an ARN but no Name must be an explicit error, not dropped.
    let http = StaticReplayClient::new(vec![ok(format!(
        "{{\"SecretList\":[{{\"ARN\":\"{}\"}}]}}",
        "arn:aws:secretsmanager:us-east-1:123456789012:secret:noname-NN1111"
    ))]);
    let err = client(&http).list_secrets().await.unwrap_err();
    assert!(
        err.to_string().contains("no name"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn list_secrets_surfaces_denied_access_error_without_leaking() {
    let http = StaticReplayClient::new(vec![event(
        400,
        "{\"__type\":\"AccessDeniedException\",\"message\":\"User is not authorized to perform secretsmanager:ListSecrets\"}".into(),
    )]);
    let err = client(&http).list_secrets().await.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("ListSecrets request failed"),
        "unexpected error context: {msg}"
    );
    let chain = format!("{err:#}");
    assert!(
        chain.contains("AccessDenied") || chain.contains("not authorized"),
        "authorization cause missing: {chain}"
    );
}

// ── field mapping: rotation tri-state, timestamp precision, no values ────────

#[tokio::test]
async fn rotation_enabled_preserves_true_false_and_absent() {
    let http = StaticReplayClient::new(vec![ok(page(
        &[
            Entry {
                name: "rot-on",
                arn: "arn:aws:secretsmanager:us-east-1:123456789012:secret:rot-on-R1",
                rotation_enabled: Some(true),
                ..Default::default()
            },
            Entry {
                name: "rot-off",
                arn: "arn:aws:secretsmanager:us-east-1:123456789012:secret:rot-off-R2",
                rotation_enabled: Some(false),
                ..Default::default()
            },
            Entry {
                name: "rot-none",
                arn: "arn:aws:secretsmanager:us-east-1:123456789012:secret:rot-none-R3",
                rotation_enabled: None,
                ..Default::default()
            },
        ],
        None,
    ))]);
    let secrets = client(&http).list_secrets().await.unwrap();
    // Sorted by name: rot-none, rot-off, rot-on.
    assert_eq!(secrets[0].rotation_enabled, None); // absent stays None, not false
    assert_eq!(secrets[1].rotation_enabled, Some(false));
    assert_eq!(secrets[2].rotation_enabled, Some(true));
}

#[tokio::test]
async fn timestamp_precision_rfc3339_except_last_accessed_date_only() {
    let http = StaticReplayClient::new(vec![ok(page(
        &[Entry {
            name: "dated",
            arn: "arn:aws:secretsmanager:us-east-1:123456789012:secret:dated-D1",
            description: Some("prod database credentials"),
            created: Some(CREATED),
            changed: Some(CHANGED),
            rotated: Some(ROTATED),
            accessed: Some(ACCESSED_MIDNIGHT),
            rotation_enabled: Some(true),
        }],
        None,
    ))]);
    let s = &client(&http).list_secrets().await.unwrap()[0];
    assert_eq!(s.description, "prod database credentials");
    assert_eq!(s.created_date, "2019-12-16T23:48:18Z");
    assert_eq!(s.last_changed_date, "2019-12-17T23:48:18Z");
    assert_eq!(s.last_rotated_date, "2019-12-15T23:48:18Z");
    // Date-precision only for last_accessed_date.
    assert_eq!(s.last_accessed_date, "2019-12-16");
}

#[tokio::test]
async fn omitted_timestamps_and_description_stay_empty_not_invented() {
    let http = StaticReplayClient::new(vec![ok(page(
        &[secret(
            "sparse",
            "arn:aws:secretsmanager:us-east-1:123456789012:secret:sparse-S1",
        )],
        None,
    ))]);
    let s = &client(&http).list_secrets().await.unwrap()[0];
    assert_eq!(s.description, "");
    assert_eq!(s.created_date, "");
    assert_eq!(s.last_changed_date, "");
    assert_eq!(s.last_accessed_date, "");
    assert_eq!(s.last_rotated_date, "");
    assert_eq!(s.rotation_enabled, None);
}

// ── request shape: only ListSecrets, MaxResults<=100, token forwarded ────────

#[tokio::test]
async fn requests_only_list_secrets_with_max_results_100_and_forwards_token() {
    use aws_smithy_http_client::test_util::infallible_client_fn;
    use std::sync::{Arc, Mutex};
    let captured = Arc::new(Mutex::new(Vec::<(String, String)>::new()));
    let requests = captured.clone();
    let config = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-1"))
        .credentials_provider(
            aws_sdk_secretsmanager::config::SharedCredentialsProvider::new(
                aws_sdk_secretsmanager::config::Credentials::new(
                    "test", "test", None, None, "test",
                ),
            ),
        )
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(infallible_client_fn(move |request| {
            let target = request
                .headers()
                .get("x-amz-target")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_string();
            let body = String::from_utf8(request.body().bytes().unwrap().to_vec()).unwrap();
            let mut requests = requests.lock().unwrap();
            requests.push((target, body));
            let response = if requests.len() == 1 {
                page(
                    &[secret(
                        "first",
                        "arn:aws:secretsmanager:us-east-1:123456789012:secret:first-F1",
                    )],
                    Some("next-page"),
                )
            } else {
                assert_eq!(requests.len(), 2);
                page(
                    &[secret(
                        "second",
                        "arn:aws:secretsmanager:us-east-1:123456789012:secret:second-S2",
                    )],
                    None,
                )
            };
            http::Response::builder()
                .status(200)
                .body(response.into_bytes())
                .unwrap()
        }))
        .build();
    let secrets = SecretsManagerClient::new(AwsConfig::from_sdk_config(
        config,
        "us-east-1",
        "123456789012",
    ))
    .list_secrets()
    .await
    .unwrap();
    assert_eq!(secrets.len(), 2);
    let requests = captured.lock().unwrap();
    // Only ListSecrets is ever called — never GetSecretValue/BatchGetSecretValue/DescribeSecret.
    for (target, _) in requests.iter() {
        assert!(
            target.ends_with(".ListSecrets"),
            "unexpected operation target: {target}"
        );
    }
    // MaxResults is within the permitted 1..=100 range.
    assert!(requests[0].1.contains("\"MaxResults\":100"));
    assert!(!requests[0].1.contains("NextToken"));
    assert!(requests[1].1.contains("\"NextToken\":\"next-page\""));
}

#[tokio::test]
async fn model_never_carries_secret_values() {
    // Even if the service erroneously echoed a value field, SecretMetadata has
    // no field to hold it: assert the struct's whole surface is metadata only.
    let http = StaticReplayClient::new(vec![ok(format!(
        "{{\"SecretList\":[{{\"ARN\":\"{}\",\"Name\":\"leaky\",\"SecretString\":\"hunter2\"}}]}}",
        "arn:aws:secretsmanager:us-east-1:123456789012:secret:leaky-L1"
    ))]);
    let s = &client(&http).list_secrets().await.unwrap()[0];
    assert_eq!(s.name, "leaky");
    // Serialized debug of the whole row must not contain the value.
    assert!(
        !format!("{s:?}").contains("hunter2"),
        "secret value leaked into metadata model"
    );
}

#[test]
fn secret_arns_must_have_nonempty_secret_resource_component() {
    for arn in [
        "arn:aws:secretsmanager:us-east-1:123456789012:other:value",
        "arn:aws:secretsmanager:us-east-1:123456789012:secret:",
        "arn:aws:secretsmanager:us-east-1:123456789012:secret:line\nbreak",
    ] {
        assert!(validate_secret_arn(arn).is_err(), "accepted {arn:?}");
    }
}

#[tokio::test]
async fn list_metadata_includes_console_dates_owning_service_type_and_primary_region() {
    let http = StaticReplayClient::new(vec![ok(serde_json::json!({"SecretList":[{
        "ARN":"arn:aws:secretsmanager:us-east-1:123456789012:secret:console-AbCdEf",
        "Name":"console","DeletedDate":CREATED,"NextRotationDate":CHANGED,
        "PrimaryRegion":"us-east-1","OwningService":"rds","Type":"ConfluentCloudApiKey"
    }]})
    .to_string())]);
    let rows = client(&http).list_secrets().await.unwrap();
    let row = &rows[0];
    assert_eq!(row.deleted_date, "2019-12-16T23:48:18Z");
    assert_eq!(row.next_rotation_date, "2019-12-17T23:48:18Z");
    assert_eq!(row.primary_region, "us-east-1");
    assert_eq!(row.owning_service, "rds");
    assert_eq!(row.secret_type, "ConfluentCloudApiKey");
}
