//! SDK-replay tests for the read-only KMS key detail view.
//!
//! KMS speaks AWS JSON 1.1: operations are selected by the `X-Amz-Target`
//! header (`TrentService.<Op>`) and timestamps serialize as EpochSeconds
//! floats. The replay client matches on the target header so each of the five
//! allowlisted operations can be answered — or made to fail — independently,
//! which is exactly what the per-section error model requires.
use super::*;
use aws_smithy_http_client::test_util::infallible_client_fn;
use std::sync::{Arc, Mutex};

const REGION: &str = "us-east-1";
const ACCOUNT: &str = "123456789012";
const KEY_ID: &str = "1234abcd-12ab-34cd-56ef-1234567890ab";
fn key_arn() -> String {
    format!("arn:aws:kms:{REGION}:{ACCOUNT}:key/{KEY_ID}")
}

// 2019-12-16T23:48:18Z and neighbours, as EpochSeconds floats.
const CREATED: f64 = 1_576_540_098.0;
const UPDATED: f64 = 1_576_626_498.0;
const NEXT_ROT: f64 = 1_576_453_698.0;

/// A programmable KMS endpoint: maps each `TrentService.<Op>` target to a
/// (status, json-body) response, and records every target seen so a test can
/// assert the exact operation set and that forbidden operations never fire.
#[derive(Clone)]
struct Mock {
    responses: Arc<Mutex<std::collections::HashMap<&'static str, (u16, String)>>>,
    seen: Arc<Mutex<Vec<String>>>,
}

impl Mock {
    fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(std::collections::HashMap::new())),
            seen: Arc::new(Mutex::new(Vec::new())),
        }
    }
    fn on(self, op: &'static str, status: u16, body: impl Into<String>) -> Self {
        self.responses
            .lock()
            .unwrap()
            .insert(op, (status, body.into()));
        self
    }
    fn ok(self, op: &'static str, body: impl Into<String>) -> Self {
        self.on(op, 200, body)
    }
    fn targets(&self) -> Vec<String> {
        self.seen.lock().unwrap().clone()
    }
    fn client(&self) -> KmsClientForTest {
        let responses = self.responses.clone();
        let seen = self.seen.clone();
        let http = infallible_client_fn(move |request| {
            let target = request
                .headers()
                .get("x-amz-target")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default();
            let op = target.rsplit('.').next().unwrap_or_default().to_string();
            seen.lock().unwrap().push(op.clone());
            let (status, body) = responses
                .lock()
                .unwrap()
                .get(op.as_str())
                .cloned()
                .unwrap_or_else(|| (400, format!("{{\"__type\":\"UnexpectedOperation:{op}\"}}")));
            http::Response::builder()
                .status(status)
                .body(body.into_bytes())
                .unwrap()
        });
        let sdk_config = aws_config::SdkConfig::builder()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(REGION))
            .credentials_provider(aws_sdk_kms::config::SharedCredentialsProvider::new(
                aws_sdk_kms::config::Credentials::new("test", "test", None, None, "test"),
            ))
            .retry_config(aws_config::retry::RetryConfig::disabled())
            .http_client(http)
            .build();
        KmsClientForTest {
            config: AwsConfig::from_sdk_config(sdk_config, REGION, ACCOUNT),
        }
    }
}

/// Thin wrapper so tests can call the detail entry point with the mock config.
struct KmsClientForTest {
    config: AwsConfig,
}
impl KmsClientForTest {
    async fn key_details(&self, arn: &str) -> Result<KmsKeyDetail> {
        key_details(&self.config, arn).await
    }
}

// ── response builders (AWS JSON 1.1 bodies) ──────────────────────────────────

fn describe_key_body(arn: &str) -> String {
    serde_json::json!({
        "KeyMetadata": {
            "KeyId": KEY_ID,
            "Arn": arn,
            "Enabled": true,
            "KeyState": "Enabled",
            "KeyManager": "CUSTOMER",
            "KeyUsage": "ENCRYPT_DECRYPT",
            "Description": "detail test key",
            "CreationDate": CREATED,
        }
    })
    .to_string()
}

fn aliases_body(entries: &[(&str, &str)], truncated: bool, marker: Option<&str>) -> String {
    let list: Vec<_> = entries
        .iter()
        .map(|(name, id)| {
            serde_json::json!({
                "AliasName": name,
                "AliasArn": format!("arn:aws:kms:{REGION}:{ACCOUNT}:{name}"),
                "TargetKeyId": id,
                "CreationDate": CREATED,
                "LastUpdatedDate": UPDATED,
            })
        })
        .collect();
    let mut body = serde_json::json!({ "Aliases": list, "Truncated": truncated });
    if let Some(m) = marker {
        body["NextMarker"] = serde_json::json!(m);
    }
    body.to_string()
}

fn tags_body(entries: &[(&str, &str)], truncated: bool, marker: Option<&str>) -> String {
    let list: Vec<_> = entries
        .iter()
        .map(|(k, v)| serde_json::json!({ "TagKey": k, "TagValue": v }))
        .collect();
    let mut body = serde_json::json!({ "Tags": list, "Truncated": truncated });
    if let Some(m) = marker {
        body["NextMarker"] = serde_json::json!(m);
    }
    body.to_string()
}

fn rotation_body(enabled: bool, period: Option<i32>) -> String {
    let mut body = serde_json::json!({
        "KeyId": KEY_ID,
        "KeyRotationEnabled": enabled,
        "NextRotationDate": NEXT_ROT,
    });
    if let Some(p) = period {
        body["RotationPeriodInDays"] = serde_json::json!(p);
    }
    body.to_string()
}

fn policy_body(doc: &str) -> String {
    serde_json::json!({ "Policy": doc, "PolicyName": "default" }).to_string()
}

fn denied(action: &str) -> (u16, String) {
    (
        400,
        format!(
            "{{\"__type\":\"AccessDeniedException\",\"message\":\"not authorized to perform kms:{action}\"}}"
        ),
    )
}

/// A fully-populated, all-sections-succeed mock.
fn happy_mock() -> Mock {
    let arn = key_arn();
    Mock::new()
        .ok("DescribeKey", describe_key_body(&arn))
        .ok(
            "ListAliases",
            aliases_body(&[("alias/app", KEY_ID)], false, None),
        )
        .ok("GetKeyRotationStatus", rotation_body(true, Some(365)))
        .ok("GetKeyPolicy", policy_body("{\"Version\":\"2012-10-17\"}"))
        .ok(
            "ListResourceTags",
            tags_body(&[("env", "prod")], false, None),
        )
}

// ── ARN validation ───────────────────────────────────────────────────────────

#[test]
fn validate_key_arn_accepts_key_arns_and_rejects_everything_else() {
    validate_key_arn(&key_arn()).unwrap();
    validate_key_arn("arn:aws-us-gov:kms:us-gov-west-1:123456789012:key/abc").unwrap();
    for bad in [
        "",
        "not-an-arn",
        "arn:aws:sns:us-east-1:123456789012:topic", // wrong service
        "arn:aws:kms:us-east-1:123456789012:alias/app", // alias, not key
        "arn:aws:kms:us-east-1:123456789012:key/",  // empty key id
        "arn:aws:kms::123456789012:key/abc",        // empty region
        "arn:aws:kms:us-east-1:bad-account:key/abc", // bad account
        "arn::kms:us-east-1:123456789012:key/abc",  // empty partition
        "arn:aws:kms:us-east-1:123456789012:key/ab\nc", // control char
    ] {
        assert!(validate_key_arn(bad).is_err(), "accepted {bad:?}");
    }
}

#[tokio::test]
async fn rejects_region_and_account_mismatch_before_any_request() {
    let mock = happy_mock();
    let other_region = format!("arn:aws:kms:eu-west-1:{ACCOUNT}:key/{KEY_ID}");
    let err = mock.client().key_details(&other_region).await.unwrap_err();
    assert!(err.to_string().contains("does not match the active Region"));
    let other_account = format!("arn:aws:kms:{REGION}:999999999999:key/{KEY_ID}");
    let err = mock.client().key_details(&other_account).await.unwrap_err();
    assert!(err
        .to_string()
        .contains("does not match the active account"));
    // No request was spent on either mismatch.
    assert!(mock.targets().is_empty());
}

// ── happy path: identity, dates, rotation, allowlist ─────────────────────────

#[tokio::test]
async fn gathers_all_sections_on_a_valid_identity() {
    let mock = happy_mock();
    let detail = mock.client().key_details(&key_arn()).await.unwrap();

    assert_eq!(detail.metadata.key_id, KEY_ID);
    assert_eq!(detail.metadata.key_arn, key_arn());
    // Fresh metadata preserves the existing string-enum Debug labels.
    assert_eq!(detail.metadata.key_manager, "Customer");
    assert_eq!(detail.metadata.key_state, "Enabled");
    // Dates are RFC3339 UTC.
    assert_eq!(detail.metadata.creation_date, "2019-12-16T23:48:18Z");

    let aliases = detail.aliases.unwrap();
    assert_eq!(aliases.len(), 1);
    assert_eq!(aliases[0].name, "alias/app");
    assert_eq!(aliases[0].creation_date, "2019-12-16T23:48:18Z");
    assert_eq!(aliases[0].last_updated_date, "2019-12-17T23:48:18Z");

    let rotation = detail.rotation.unwrap();
    assert_eq!(rotation.enabled, Some(true));
    assert_eq!(rotation.rotation_period_days, Some(365));
    assert_eq!(rotation.next_rotation_date, "2019-12-15T23:48:18Z");

    assert_eq!(detail.policy.unwrap(), "{\"Version\":\"2012-10-17\"}");
    assert_eq!(
        detail.tags.unwrap(),
        vec![KmsTag {
            key: "env".into(),
            value: "prod".into()
        }]
    );
}

#[tokio::test]
async fn only_allowlisted_read_operations_are_ever_requested() {
    let mock = happy_mock();
    mock.client().key_details(&key_arn()).await.unwrap();
    let mut targets = mock.targets();
    targets.sort();
    targets.dedup();
    assert_eq!(
        targets,
        [
            "DescribeKey",
            "GetKeyPolicy",
            "GetKeyRotationStatus",
            "ListAliases",
            "ListResourceTags",
        ]
    );
    // Forbidden operations must never appear.
    for forbidden in [
        "Encrypt",
        "Decrypt",
        "GenerateDataKey",
        "ScheduleKeyDeletion",
        "PutKeyPolicy",
    ] {
        assert!(
            !mock.targets().iter().any(|t| t == forbidden),
            "forbidden operation {forbidden} was requested"
        );
    }
}

#[tokio::test]
async fn scopes_alias_and_tag_listing_to_the_requested_key() {
    // Capture request bodies to prove ListAliases/ListResourceTags are key-scoped.
    let bodies = Arc::new(Mutex::new(Vec::<(String, String)>::new()));
    let captured = bodies.clone();
    let arn = key_arn();
    let dk = describe_key_body(&arn);
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
        let body = String::from_utf8(request.body().bytes().unwrap().to_vec()).unwrap();
        captured.lock().unwrap().push((op.clone(), body));
        let resp = match op.as_str() {
            "DescribeKey" => dk.clone(),
            "ListAliases" => aliases_body(&[], false, None),
            "GetKeyRotationStatus" => rotation_body(false, None),
            "GetKeyPolicy" => policy_body("{}"),
            "ListResourceTags" => tags_body(&[], false, None),
            other => format!("{{\"__type\":\"Unexpected:{other}\"}}"),
        };
        http::Response::builder()
            .status(200)
            .body(resp.into_bytes())
            .unwrap()
    });
    let sdk_config = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(REGION))
        .credentials_provider(aws_sdk_kms::config::SharedCredentialsProvider::new(
            aws_sdk_kms::config::Credentials::new("test", "test", None, None, "test"),
        ))
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(http)
        .build();
    let config = AwsConfig::from_sdk_config(sdk_config, REGION, ACCOUNT);
    key_details(&config, &key_arn()).await.unwrap();

    let bodies = bodies.lock().unwrap();
    for (op, body) in bodies.iter() {
        let value: serde_json::Value = serde_json::from_str(body).unwrap();
        assert_eq!(value["KeyId"], key_arn(), "{op} must use the validated ARN");
        if op == "ListAliases" || op == "ListResourceTags" {
            assert_eq!(
                value["Limit"], 50,
                "{op} page limit must be valid for the SDK"
            );
        }
        if op == "GetKeyPolicy" {
            assert_eq!(value["PolicyName"], "default");
        }
    }
}

// ── identity verification ────────────────────────────────────────────────────

#[tokio::test]
async fn rejects_describe_key_returning_a_different_identity() {
    let other = format!("arn:aws:kms:{REGION}:{ACCOUNT}:key/ffffffff-ffff-ffff-ffff-ffffffffffff");
    let mock = happy_mock().ok("DescribeKey", describe_key_body(&other));
    let err = mock.client().key_details(&key_arn()).await.unwrap_err();
    assert!(
        err.to_string()
            .contains("refusing to present a mismatched identity"),
        "unexpected: {err}"
    );
}

#[tokio::test]
async fn describe_key_failure_fails_the_whole_operation() {
    let (status, body) = denied("DescribeKey");
    let mock = happy_mock().on("DescribeKey", status, body);
    let err = mock.client().key_details(&key_arn()).await.unwrap_err();
    let chain = format!("{err:#}");
    assert!(chain.contains("DescribeKey request failed"), "{chain}");
    assert!(
        chain.contains("not authorized") || chain.contains("AccessDenied"),
        "{chain}"
    );
}

// ── per-section error isolation (not pretended-success) ──────────────────────

#[tokio::test]
async fn a_denied_section_is_an_error_not_a_silent_empty_or_disabled() {
    let (ds, db) = denied("GetKeyRotationStatus");
    let (ps, pb) = denied("GetKeyPolicy");
    let (ts, tb) = denied("ListResourceTags");
    let (as_, ab) = denied("ListAliases");
    let mock = happy_mock()
        .on("GetKeyRotationStatus", ds, db)
        .on("GetKeyPolicy", ps, pb)
        .on("ListResourceTags", ts, tb)
        .on("ListAliases", as_, ab);
    let detail = mock.client().key_details(&key_arn()).await.unwrap();

    // DescribeKey still succeeded, so metadata is present and authoritative.
    assert_eq!(detail.metadata.key_id, KEY_ID);
    // Each failed section is an Err — crucially rotation is NOT Ok(enabled:false).
    let rot_err = detail.rotation.unwrap_err();
    assert!(
        rot_err.contains("not authorized") || rot_err.contains("AccessDenied"),
        "{rot_err}"
    );
    assert!(detail.policy.is_err());
    assert!(detail.tags.is_err());
    assert!(detail.aliases.is_err());
}

#[tokio::test]
async fn rotation_unsupported_state_surfaces_as_error_not_false() {
    // Asymmetric keys (and others) report rotation as unsupported via an API
    // error; that must become a section error, never Ok(enabled: false).
    let unsupported = (
        400u16,
        "{\"__type\":\"UnsupportedOperationException\",\"message\":\"rotation not supported for this KeyUsage\"}".to_string(),
    );
    let mock = happy_mock().on("GetKeyRotationStatus", unsupported.0, unsupported.1);
    let detail = mock.client().key_details(&key_arn()).await.unwrap();
    let err = detail.rotation.as_ref().unwrap_err();
    assert!(
        err.contains("not supported") || err.contains("Unsupported"),
        "{err}"
    );
    assert_ne!(
        detail.rotation,
        Ok(KmsRotation {
            enabled: Some(false),
            ..Default::default()
        })
    );
}

// ── pagination: multi-page, cap, repeated-token cycle, truncated protocol ────

#[tokio::test]
async fn aliases_and_tags_follow_the_truncated_marker_protocol_across_pages() {
    // Two-page aliases and tags, with Truncated+NextMarker then a final page.
    let page_a1 = aliases_body(&[("alias/a", KEY_ID)], true, Some("m1"));
    let page_a2 = aliases_body(&[("alias/b", KEY_ID)], false, None);
    let page_t1 = tags_body(&[("k1", "v1")], true, Some("tm1"));
    let page_t2 = tags_body(&[("k2", "v2")], false, None);
    let alias_calls = Arc::new(Mutex::new(0usize));
    let tag_calls = Arc::new(Mutex::new(0usize));
    let (ac, tc) = (alias_calls.clone(), tag_calls.clone());
    let arn = key_arn();
    let dk = describe_key_body(&arn);
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
        let resp = match op.as_str() {
            "DescribeKey" => dk.clone(),
            "GetKeyRotationStatus" => rotation_body(true, Some(90)),
            "GetKeyPolicy" => policy_body("{}"),
            "ListAliases" => {
                let mut n = ac.lock().unwrap();
                *n += 1;
                if *n == 1 {
                    page_a1.clone()
                } else {
                    page_a2.clone()
                }
            }
            "ListResourceTags" => {
                let mut n = tc.lock().unwrap();
                *n += 1;
                if *n == 1 {
                    page_t1.clone()
                } else {
                    page_t2.clone()
                }
            }
            other => format!("{{\"__type\":\"Unexpected:{other}\"}}"),
        };
        http::Response::builder()
            .status(200)
            .body(resp.into_bytes())
            .unwrap()
    });
    let sdk_config = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(REGION))
        .credentials_provider(aws_sdk_kms::config::SharedCredentialsProvider::new(
            aws_sdk_kms::config::Credentials::new("test", "test", None, None, "test"),
        ))
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(http)
        .build();
    let config = AwsConfig::from_sdk_config(sdk_config, REGION, ACCOUNT);
    let detail = key_details(&config, &key_arn()).await.unwrap();

    let aliases = detail.aliases.unwrap();
    assert_eq!(
        aliases.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
        ["alias/a", "alias/b"]
    );
    let tags = detail.tags.unwrap();
    assert_eq!(
        tags.iter().map(|t| t.key.as_str()).collect::<Vec<_>>(),
        ["k1", "k2"]
    );
    assert_eq!(*alias_calls.lock().unwrap(), 2);
    assert_eq!(*tag_calls.lock().unwrap(), 2);
}

#[tokio::test]
async fn non_truncated_page_completes_even_if_a_marker_is_echoed() {
    // Truncated=false with a stray NextMarker must NOT cause another request.
    let mock = happy_mock()
        .ok(
            "ListAliases",
            aliases_body(&[("alias/x", KEY_ID)], false, Some("ignored")),
        )
        .ok(
            "ListResourceTags",
            tags_body(&[("k", "v")], false, Some("ignored")),
        );
    let detail = mock.client().key_details(&key_arn()).await.unwrap();
    assert_eq!(detail.aliases.unwrap().len(), 1);
    assert_eq!(detail.tags.unwrap().len(), 1);
    // Exactly one page each.
    assert_eq!(
        mock.targets()
            .iter()
            .filter(|t| *t == "ListAliases")
            .count(),
        1
    );
    assert_eq!(
        mock.targets()
            .iter()
            .filter(|t| *t == "ListResourceTags")
            .count(),
        1
    );
}

#[tokio::test]
async fn repeated_alias_marker_is_a_section_error_not_an_infinite_loop() {
    let mock = happy_mock().ok(
        "ListAliases",
        aliases_body(&[("alias/a", KEY_ID)], true, Some("same")),
    );
    let detail = mock.client().key_details(&key_arn()).await.unwrap();
    let err = detail.aliases.unwrap_err();
    assert!(err.contains("repeated pagination token"), "{err}");
    // Other sections are unaffected.
    assert!(detail.tags.is_ok());
    assert!(detail.rotation.is_ok());
}

#[tokio::test]
async fn tag_marker_cycle_is_detected_as_a_section_error() {
    // a -> b -> a cycle on ListResourceTags.
    let page1 = tags_body(&[("k1", "v1")], true, Some("a"));
    let page2 = tags_body(&[("k2", "v2")], true, Some("b"));
    let page3 = tags_body(&[("k3", "v3")], true, Some("a"));
    let n = Arc::new(Mutex::new(0usize));
    let counter = n.clone();
    let arn = key_arn();
    let dk = describe_key_body(&arn);
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
        let resp = match op.as_str() {
            "DescribeKey" => dk.clone(),
            "GetKeyRotationStatus" => rotation_body(true, Some(90)),
            "GetKeyPolicy" => policy_body("{}"),
            "ListAliases" => aliases_body(&[], false, None),
            "ListResourceTags" => {
                let mut c = counter.lock().unwrap();
                *c += 1;
                match *c {
                    1 => page1.clone(),
                    2 => page2.clone(),
                    _ => page3.clone(),
                }
            }
            other => format!("{{\"__type\":\"Unexpected:{other}\"}}"),
        };
        http::Response::builder()
            .status(200)
            .body(resp.into_bytes())
            .unwrap()
    });
    let sdk_config = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(REGION))
        .credentials_provider(aws_sdk_kms::config::SharedCredentialsProvider::new(
            aws_sdk_kms::config::Credentials::new("test", "test", None, None, "test"),
        ))
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(http)
        .build();
    let config = AwsConfig::from_sdk_config(sdk_config, REGION, ACCOUNT);
    let detail = key_details(&config, &key_arn()).await.unwrap();
    assert!(detail
        .tags
        .unwrap_err()
        .contains("repeated pagination token"));
}

#[tokio::test]
async fn page_cap_makes_a_section_incomplete_rather_than_partial_success() {
    // An unending stream of distinct truncated markers must hit the page bound
    // and report incompleteness for that section only.
    let n = Arc::new(Mutex::new(0usize));
    let counter = n.clone();
    let arn = key_arn();
    let dk = describe_key_body(&arn);
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
        let resp = match op.as_str() {
            "DescribeKey" => dk.clone(),
            "GetKeyRotationStatus" => rotation_body(true, Some(90)),
            "GetKeyPolicy" => policy_body("{}"),
            "ListResourceTags" => tags_body(&[], false, None),
            "ListAliases" => {
                let mut c = counter.lock().unwrap();
                *c += 1;
                aliases_body(&[("alias/a", KEY_ID)], true, Some(&format!("m{c}")))
            }
            other => format!("{{\"__type\":\"Unexpected:{other}\"}}"),
        };
        http::Response::builder()
            .status(200)
            .body(resp.into_bytes())
            .unwrap()
    });
    let sdk_config = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(REGION))
        .credentials_provider(aws_sdk_kms::config::SharedCredentialsProvider::new(
            aws_sdk_kms::config::Credentials::new("test", "test", None, None, "test"),
        ))
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(http)
        .build();
    let config = AwsConfig::from_sdk_config(sdk_config, REGION, ACCOUNT);
    // A small bound keeps the test fast; the production bound is MAX_PAGES.
    let detail = key_details_inner(&config, &key_arn(), 3).await.unwrap();
    let err = detail.aliases.unwrap_err();
    assert!(
        err.contains("exceeded 3 pages") && err.contains("incomplete"),
        "{err}"
    );
}

// ── field fidelity: absent timestamps / tri-state rotation stay honest ───────

#[tokio::test]
async fn absent_rotation_fields_stay_none_and_empty_not_invented() {
    // rotation enabled=false with no period and no on-demand date.
    let mock = happy_mock().ok(
        "GetKeyRotationStatus",
        serde_json::json!({"KeyId": KEY_ID, "KeyRotationEnabled": false}).to_string(),
    );
    let detail = mock.client().key_details(&key_arn()).await.unwrap();
    let rotation = detail.rotation.unwrap();
    assert_eq!(rotation.enabled, Some(false));
    assert_eq!(rotation.rotation_period_days, None);
    assert_eq!(rotation.next_rotation_date, "");
    assert_eq!(rotation.on_demand_rotation_start_date, "");
}

#[tokio::test]
async fn empty_account_sections_are_ok_empty_not_errors() {
    let mock = happy_mock()
        .ok("ListAliases", aliases_body(&[], false, None))
        .ok("ListResourceTags", tags_body(&[], false, None));
    let detail = mock.client().key_details(&key_arn()).await.unwrap();
    assert_eq!(detail.aliases.unwrap(), vec![]);
    assert_eq!(detail.tags.unwrap(), vec![]);
}

#[tokio::test]
async fn truncated_without_marker_is_never_a_partial_success() {
    for marker in [None, Some("")] {
        let mock = happy_mock()
            .ok(
                "ListAliases",
                aliases_body(&[("alias/a", KEY_ID)], true, marker),
            )
            .ok("ListResourceTags", tags_body(&[("k", "v")], true, marker));
        let result = mock.client().key_details(&key_arn()).await.unwrap();
        assert!(result.aliases.unwrap_err().contains("incomplete"));
        assert!(result.tags.unwrap_err().contains("incomplete"));
        assert!(result.policy.is_ok() && result.rotation.is_ok());
    }
}

#[tokio::test]
async fn mismatched_key_id_is_rejected_before_any_section_reads() {
    let mut body: serde_json::Value = serde_json::from_str(&describe_key_body(&key_arn())).unwrap();
    body["KeyMetadata"]["KeyId"] = serde_json::json!("different-key");
    let mock = happy_mock().ok("DescribeKey", body.to_string());
    assert!(mock
        .client()
        .key_details(&key_arn())
        .await
        .unwrap_err()
        .to_string()
        .contains("KeyId"));
    assert_eq!(mock.targets(), vec!["DescribeKey"]);
}

#[tokio::test]
async fn slow_section_times_out_without_discarding_other_results() {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(10);
    let (slow, success, denied) = tokio::join!(
        section_read::<String>(deadline, std::future::pending()),
        section_read(deadline, async { Ok("kept") }),
        section_read::<String>(deadline, async { bail!("AccessDenied") }),
    );
    assert!(slow.unwrap_err().contains("timed out"));
    assert_eq!(success.unwrap(), "kept");
    assert!(denied.unwrap_err().contains("AccessDenied"));
}

#[tokio::test]
async fn alias_for_a_different_key_is_a_section_error() {
    let mock = happy_mock().ok(
        "ListAliases",
        aliases_body(&[("alias/other", "different")], false, None),
    );
    let detail = mock.client().key_details(&key_arn()).await.unwrap();
    assert!(detail.aliases.unwrap_err().contains("different key"));
    assert!(detail.policy.is_ok() && detail.tags.is_ok());
}
