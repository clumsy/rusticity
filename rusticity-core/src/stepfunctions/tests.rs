use super::*;
use aws_smithy_http_client::test_util::{infallible_client_fn, ReplayEvent, StaticReplayClient};
use aws_smithy_types::body::SdkBody;
use std::sync::{Arc, Mutex};

const ENDPOINT: &str = "https://states.us-east-1.amazonaws.com/";
const ACCOUNT: &str = "123456789012";
const SM_ARN: &str = "arn:aws:states:us-east-1:123456789012:stateMachine:orders";

// ── test plumbing ────────────────────────────────────────────────────────────

fn event(status: u16, body: &str) -> ReplayEvent {
    ReplayEvent::new(
        http::Request::builder()
            .method("POST")
            .uri(ENDPOINT)
            .body(SdkBody::from(""))
            .unwrap(),
        http::Response::builder()
            .status(status)
            .body(SdkBody::from(body.to_string()))
            .unwrap(),
    )
}

fn ok(body: &str) -> ReplayEvent {
    event(200, body)
}

fn client(http_client: &StaticReplayClient) -> StepFunctionsClient {
    let sdk_config = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-1"))
        .credentials_provider(aws_sdk_sfn::config::SharedCredentialsProvider::new(
            aws_sdk_sfn::config::Credentials::new("test-key", "test-secret", None, None, "test"),
        ))
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(http_client.clone())
        .build();
    StepFunctionsClient::new(AwsConfig::from_sdk_config(sdk_config, "us-east-1", ACCOUNT))
}

fn standard(name: &str) -> StateMachineMetadata {
    StateMachineMetadata {
        name: name.into(),
        arn: format!("arn:aws:states:us-east-1:{ACCOUNT}:stateMachine:{name}"),
        machine_type: "STANDARD".into(),
        creation_date: String::new(),
    }
}

// ── StateMachineMetadata / ExecutionMetadata interface shape ───────────────────

#[test]
fn metadata_types_derive_the_agreed_traits_and_default_is_empty() {
    // Clone, Debug, Default, PartialEq, Eq for both structs.
    let sm = StateMachineMetadata::default();
    assert_eq!(sm, sm.clone());
    assert!(format!("{sm:?}").contains("StateMachineMetadata"));
    assert_eq!(
        sm,
        StateMachineMetadata {
            name: String::new(),
            arn: String::new(),
            machine_type: String::new(),
            creation_date: String::new(),
        }
    );

    let ex = ExecutionMetadata::default();
    assert_eq!(ex, ex.clone());
    assert!(format!("{ex:?}").contains("ExecutionMetadata"));
    assert_eq!(ex.duration_millis, None);
}

// ── duration math (only valid completed start/stop, never negative) ────────────

#[test]
fn duration_is_some_only_for_completed_well_ordered_pairs() {
    use aws_smithy_types::DateTime;
    let t0 = DateTime::from_secs(1_700_000_000);
    let t1 = DateTime::from_secs(1_700_000_012); // +12s
    assert_eq!(duration_millis(Some(&t0), Some(&t1)), Some(12_000));
    // equal start/stop -> 0ms (>= 0 is valid)
    assert_eq!(duration_millis(Some(&t0), Some(&t0)), Some(0));
    // running (no stop) -> None
    assert_eq!(duration_millis(Some(&t0), None), None);
    // no start -> None
    assert_eq!(duration_millis(None, Some(&t1)), None);
    // stop before start (clock skew) -> None, never a negative/underflowed value
    assert_eq!(duration_millis(Some(&t1), Some(&t0)), None);
}

// ── state-machine ARN validation ───────────────────────────────────────────────

#[test]
fn parse_state_machine_arn_accepts_full_unqualified_and_rejects_the_rest() {
    let (region, account, name) = parse_state_machine_arn(SM_ARN).unwrap();
    assert_eq!((region, account, name), ("us-east-1", ACCOUNT, "orders"));
    // Non-default partition is valid.
    assert!(parse_state_machine_arn(
        "arn:aws-us-gov:states:us-gov-west-1:123456789012:stateMachine:sm"
    )
    .is_ok());

    for bad in [
        "",
        "not-an-arn",
        "arn:aws:sqs:us-east-1:123456789012:queue", // wrong service
        "arn:aws:states:us-east-1:123456789012:stateMachine:", // empty name
        "arn:aws:states:us-east-1:123456789012:activity:a", // not a state machine
        "arn:aws:states::123456789012:stateMachine:sm", // empty region
        "arn:aws:states:us-east-1::stateMachine:sm", // empty account
        "arn:aws:states:us-east-1:not-account:stateMachine:sm", // non-numeric account
        "arn:aws:states:us-east-1:123456789012:stateMachine:sm:1", // qualified (version)
    ] {
        assert!(parse_state_machine_arn(bad).is_err(), "accepted {bad:?}");
    }
}

// ── reject EXPRESS / unknown BEFORE any request ───────────────────────────────

#[tokio::test]
async fn list_executions_rejects_express_before_any_http_call() {
    // Empty replay vec: any HTTP attempt would panic. Rejection must precede it.
    let http = StaticReplayClient::new(vec![]);
    let express = StateMachineMetadata {
        machine_type: "EXPRESS".into(),
        ..standard("fast")
    };
    let err = client(&http).list_executions(&express).await.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("EXPRESS"), "unexpected: {msg}");
    assert!(msg.contains("not supported"), "unexpected: {msg}");
}

#[tokio::test]
async fn list_executions_rejects_unknown_machine_type_before_any_http_call() {
    let http = StaticReplayClient::new(vec![]);
    let weird = StateMachineMetadata {
        machine_type: "SYNC_EXPRESS".into(),
        ..standard("weird")
    };
    let err = client(&http).list_executions(&weird).await.unwrap_err();
    assert!(
        err.to_string().contains("not supported") && err.to_string().contains("STANDARD"),
        "unexpected: {err}"
    );
}

// ── ListStateMachines: wire limits, metadata-only, pagination, sort ────────────

#[tokio::test]
async fn list_state_machines_sends_only_list_call_with_bounded_max_results() {
    let requests = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
    let seen_targets = Arc::new(Mutex::new(Vec::<String>::new()));
    let reqs = requests.clone();
    let tgts = seen_targets.clone();
    let sdk = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-1"))
        .credentials_provider(aws_sdk_sfn::config::SharedCredentialsProvider::new(
            aws_sdk_sfn::config::Credentials::new("t", "t", None, None, "t"),
        ))
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(infallible_client_fn(move |request| {
            let target = request.headers()["x-amz-target"].to_str().unwrap().to_string();
            tgts.lock().unwrap().push(target.clone());
            let body: serde_json::Value =
                serde_json::from_slice(request.body().bytes().unwrap()).unwrap();
            let n = reqs.lock().unwrap().len();
            reqs.lock().unwrap().push(body);
            let resp = if n == 0 {
                serde_json::json!({
                    "stateMachines": [{
                        "name": "orders", "stateMachineArn": SM_ARN,
                        "type": "STANDARD", "creationDate": 1_700_000_000
                    }],
                    "nextToken": "page-2"
                })
            } else {
                serde_json::json!({
                    "stateMachines": [{
                        "name": "alpha",
                        "stateMachineArn": format!("arn:aws:states:us-east-1:{ACCOUNT}:stateMachine:alpha"),
                        "type": "EXPRESS", "creationDate": 1_700_000_500
                    }]
                })
            };
            http::Response::builder().status(200).body(resp.to_string().into_bytes()).unwrap()
        }))
        .build();
    let machines = StepFunctionsClient::new(AwsConfig::from_sdk_config(sdk, "us-east-1", ACCOUNT))
        .list_state_machines()
        .await
        .unwrap();

    // Only ListStateMachines was called (no Describe*, no definition fetch).
    let targets = seen_targets.lock().unwrap();
    assert_eq!(targets.len(), 2);
    assert!(targets
        .iter()
        .all(|t| t == "AWSStepFunctions.ListStateMachines"));

    // maxResults is within the documented 0..=1000 range on every request,
    // and the continuation token is forwarded verbatim on the 2nd page.
    let reqs = requests.lock().unwrap();
    for body in reqs.iter() {
        assert_eq!(body["maxResults"], 1000);
    }
    assert!(reqs[0].get("nextToken").is_none());
    assert_eq!(reqs[1]["nextToken"], "page-2");

    // Sorted by name; both types captured verbatim; creationDate is RFC3339 UTC.
    let names: Vec<_> = machines.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["alpha", "orders"]);
    assert_eq!(machines[0].machine_type, "EXPRESS");
    assert_eq!(machines[1].machine_type, "STANDARD");
    assert_eq!(machines[1].creation_date, "2023-11-14T22:13:20Z");
}

#[tokio::test]
async fn list_state_machines_errors_on_repeated_token_instead_of_looping() {
    let page = |tok: &str| {
        format!(
            r#"{{"stateMachines":[{{"name":"orders","stateMachineArn":"{SM_ARN}","type":"STANDARD","creationDate":1700000000}}],"nextToken":"{tok}"}}"#
        )
    };
    let http = StaticReplayClient::new(vec![ok(&page("same")), ok(&page("same"))]);
    let err = client(&http).list_state_machines().await.unwrap_err();
    assert!(
        err.to_string().contains("repeated pagination token"),
        "unexpected: {err}"
    );
}

#[tokio::test]
async fn list_state_machines_page_bound_is_incomplete_error_not_partial_success() {
    let page = |tok: &str| {
        format!(
            r#"{{"stateMachines":[{{"name":"orders","stateMachineArn":"{SM_ARN}","type":"STANDARD","creationDate":1700000000}}],"nextToken":"{tok}"}}"#
        )
    };
    let http = StaticReplayClient::new(vec![ok(&page("a")), ok(&page("b"))]);
    let err = client(&http)
        .list_state_machines_inner(2)
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("inventory is incomplete"),
        "unexpected: {err}"
    );

    // Zero bound never issues a request.
    let http = StaticReplayClient::new(vec![]);
    let err = client(&http)
        .list_state_machines_inner(0)
        .await
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "Step Functions ListStateMachines exceeded 0 pages; inventory is incomplete"
    );
}

#[tokio::test]
async fn list_state_machines_handles_empty_account() {
    let http = StaticReplayClient::new(vec![ok(r#"{"stateMachines":[]}"#)]);
    assert!(client(&http)
        .list_state_machines()
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn list_state_machines_missing_required_field_errors_not_empty() {
    // ARN present but name empty -> explicit error, not a silent empty row.
    let body = format!(
        r#"{{"stateMachines":[{{"name":"","stateMachineArn":"{SM_ARN}","type":"STANDARD","creationDate":1700000000}}]}}"#
    );
    let http = StaticReplayClient::new(vec![ok(&body)]);
    let err = client(&http).list_state_machines().await.unwrap_err();
    assert!(err.to_string().contains("no name"), "unexpected: {err}");
}

#[tokio::test]
async fn list_state_machines_surfaces_denied_access_error() {
    let body = r#"{"__type":"AccessDeniedException","message":"not authorized to perform states:ListStateMachines"}"#;
    let http = StaticReplayClient::new(vec![event(400, body)]);
    let err = client(&http).list_state_machines().await.unwrap_err();
    assert!(
        format!("{err:#}").contains("ListStateMachines request failed"),
        "unexpected: {err:#}"
    );
}

// ── ListExecutions: scoping, allowlist, newest-first, duration, pages ──────────

#[tokio::test]
async fn list_executions_scopes_request_and_sorts_newest_first_with_duration() {
    let requests = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
    let targets = Arc::new(Mutex::new(Vec::<String>::new()));
    let reqs = requests.clone();
    let tgts = targets.clone();
    let sdk = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-1"))
        .credentials_provider(aws_sdk_sfn::config::SharedCredentialsProvider::new(
            aws_sdk_sfn::config::Credentials::new("t", "t", None, None, "t"),
        ))
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(infallible_client_fn(move |request| {
            tgts.lock().unwrap().push(
                request.headers()["x-amz-target"]
                    .to_str()
                    .unwrap()
                    .to_string(),
            );
            let body: serde_json::Value =
                serde_json::from_slice(request.body().bytes().unwrap()).unwrap();
            reqs.lock().unwrap().push(body);
            // Two executions: one completed (has stopDate), one running (no stop).
            let resp = serde_json::json!({
                "executions": [
                    {
                        "executionArn": SM_ARN.replace("stateMachine", "execution") + ":run-old",
                        "name": "run-old",
                        "stateMachineArn": SM_ARN,
                        "status": "SUCCEEDED",
                        "startDate": 1_700_000_000,
                        "stopDate": 1_700_000_030
                    },
                    {
                        "executionArn": SM_ARN.replace("stateMachine", "execution") + ":run-new",
                        "name": "run-new",
                        "stateMachineArn": SM_ARN,
                        "status": "RUNNING",
                        "startDate": 1_700_000_100
                    }
                ]
            });
            http::Response::builder()
                .status(200)
                .body(resp.to_string().into_bytes())
                .unwrap()
        }))
        .build();
    let sm = standard("orders");
    let execs = StepFunctionsClient::new(AwsConfig::from_sdk_config(sdk, "us-east-1", ACCOUNT))
        .list_executions(&sm)
        .await
        .unwrap();

    // Only ListExecutions; request scoped to this state machine; maxResults bounded.
    let targets = targets.lock().unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0], "AWSStepFunctions.ListExecutions");
    let reqs = requests.lock().unwrap();
    assert_eq!(reqs[0]["stateMachineArn"], SM_ARN);
    assert_eq!(reqs[0]["maxResults"], 1000);

    // Newest first by start date.
    let names: Vec<_> = execs.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["run-new", "run-old"]);
    // Completed execution carries duration; running one does not.
    assert_eq!(execs[0].status, "RUNNING");
    assert_eq!(execs[0].stop_date, "");
    assert_eq!(execs[0].duration_millis, None);
    assert_eq!(execs[1].status, "SUCCEEDED");
    assert_eq!(execs[1].start_date, "2023-11-14T22:13:20Z");
    assert_eq!(execs[1].stop_date, "2023-11-14T22:13:50Z");
    assert_eq!(execs[1].duration_millis, Some(30_000));
    assert!(execs.iter().all(|e| e.state_machine_arn == SM_ARN));
}

#[tokio::test]
async fn list_executions_rejects_out_of_scope_execution_arns() {
    let foreign_account = format!(
        r#"{{"executions":[{{"executionArn":"arn:aws:states:us-east-1:999999999999:execution:orders:r","name":"r","stateMachineArn":"{SM_ARN}","status":"SUCCEEDED","startDate":1700000000,"stopDate":1700000001}}]}}"#
    );
    let http = StaticReplayClient::new(vec![ok(&foreign_account)]);
    let err = client(&http)
        .list_executions(&standard("orders"))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("scope"), "account: {err}");

    // Execution ARN names a different state machine than the one requested.
    let foreign_machine = r#"{"executions":[{"executionArn":"arn:aws:states:us-east-1:123456789012:execution:other:r","name":"r","stateMachineArn":"arn:aws:states:us-east-1:123456789012:stateMachine:orders","status":"SUCCEEDED","startDate":1700000000,"stopDate":1700000001}]}"#;
    let http = StaticReplayClient::new(vec![ok(foreign_machine)]);
    let err = client(&http)
        .list_executions(&standard("orders"))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("not"), "machine: {err}");
}

#[tokio::test]
async fn list_executions_rejects_mismatched_echoed_state_machine_arn() {
    // Valid execution ARN for `orders`, but the echoed stateMachineArn disagrees.
    let body = r#"{"executions":[{"executionArn":"arn:aws:states:us-east-1:123456789012:execution:orders:r","name":"r","stateMachineArn":"arn:aws:states:us-east-1:123456789012:stateMachine:elsewhere","status":"SUCCEEDED","startDate":1700000000,"stopDate":1700000001}]}"#;
    let http = StaticReplayClient::new(vec![ok(body)]);
    let err = client(&http)
        .list_executions(&standard("orders"))
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("not the requested"),
        "unexpected: {err}"
    );
}

#[tokio::test]
async fn list_executions_accepts_response_that_omits_echoed_state_machine_arn() {
    // Scoped-response field may be absent; the row is still accepted and scoped
    // to the requested ARN (metadata-only; no injected payload is read).
    let body = r#"{"executions":[{"executionArn":"arn:aws:states:us-east-1:123456789012:execution:orders:r","name":"r","status":"SUCCEEDED","startDate":1700000000,"stopDate":1700000005,"input":"{\"ignored\":true}","output":"{\"ignored\":true}"}]}"#;
    let http = StaticReplayClient::new(vec![ok(body)]);
    let execs = client(&http)
        .list_executions(&standard("orders"))
        .await
        .unwrap();
    assert_eq!(execs.len(), 1);
    assert_eq!(execs[0].state_machine_arn, SM_ARN);
    assert_eq!(execs[0].duration_millis, Some(5_000));
    // Injected input/output payload is ignored — our struct has no such field.
    assert!(format!("{:?}", execs[0]).find("ignored").is_none());
}

#[tokio::test]
async fn list_executions_preserves_unknown_status_verbatim() {
    let body = r#"{"executions":[{"executionArn":"arn:aws:states:us-east-1:123456789012:execution:orders:r","name":"r","stateMachineArn":"arn:aws:states:us-east-1:123456789012:stateMachine:orders","status":"PENDING_REDRIVE","startDate":1700000000}]}"#;
    let http = StaticReplayClient::new(vec![ok(body)]);
    let execs = client(&http)
        .list_executions(&standard("orders"))
        .await
        .unwrap();
    assert_eq!(execs[0].status, "PENDING_REDRIVE");
    assert_eq!(execs[0].duration_millis, None);
}

#[tokio::test]
async fn list_executions_missing_required_field_errors_not_empty() {
    // Execution with no name -> explicit error.
    let body = r#"{"executions":[{"executionArn":"arn:aws:states:us-east-1:123456789012:execution:orders:r","name":"","stateMachineArn":"arn:aws:states:us-east-1:123456789012:stateMachine:orders","status":"SUCCEEDED","startDate":1700000000,"stopDate":1700000001}]}"#;
    let http = StaticReplayClient::new(vec![ok(body)]);
    let err = client(&http)
        .list_executions(&standard("orders"))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("no name"), "unexpected: {err}");
}

#[tokio::test]
async fn list_executions_errors_on_repeated_token_and_reports_page_bound() {
    let page = |tok: &str| {
        format!(
            r#"{{"executions":[{{"executionArn":"arn:aws:states:us-east-1:123456789012:execution:orders:r","name":"r","stateMachineArn":"{SM_ARN}","status":"SUCCEEDED","startDate":1700000000,"stopDate":1700000001}}],"nextToken":"{tok}"}}"#
        )
    };
    let http = StaticReplayClient::new(vec![ok(&page("same")), ok(&page("same"))]);
    let err = client(&http)
        .list_executions(&standard("orders"))
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("repeated pagination token"),
        "repeat: {err}"
    );

    let http = StaticReplayClient::new(vec![ok(&page("a")), ok(&page("b"))]);
    let err = client(&http)
        .list_executions_inner(SM_ARN, "us-east-1", ACCOUNT, "orders", 2)
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("inventory is incomplete"),
        "bound: {err}"
    );
}

#[tokio::test]
async fn list_executions_handles_empty_result() {
    let http = StaticReplayClient::new(vec![ok(r#"{"executions":[]}"#)]);
    assert!(client(&http)
        .list_executions(&standard("orders"))
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn requests_reject_wrong_active_context_before_http() {
    for arn in [
        "arn:aws:states:eu-west-1:123456789012:stateMachine:orders",
        "arn:aws:states:us-east-1:999999999999:stateMachine:orders",
    ] {
        let http = StaticReplayClient::new(vec![]);
        let machine = StateMachineMetadata {
            arn: arn.into(),
            ..standard("orders")
        };
        assert!(client(&http)
            .list_executions(&machine)
            .await
            .unwrap_err()
            .to_string()
            .contains("active"));
    }
}

#[tokio::test]
async fn inventory_rejects_malformed_and_mismatched_identities() {
    for (arn, name) in [
        ("arn:aws:sns:us-east-1:123456789012:orders", "orders"),
        (
            "arn:aws:states:us-east-1:999999999999:stateMachine:orders",
            "orders",
        ),
        (SM_ARN, "wrong-name"),
    ] {
        let body = serde_json::json!({"stateMachines":[{"name":name,"stateMachineArn":arn,"type":"STANDARD","creationDate":1700000000}]}).to_string();
        let http = StaticReplayClient::new(vec![ok(&body)]);
        assert!(client(&http).list_state_machines().await.is_err());
    }
}

#[test]
fn negative_submillisecond_duration_is_unknown_not_zero() {
    let start = DateTime::from_nanos(1_000_001).unwrap();
    let stop = DateTime::from_nanos(1_000_000).unwrap();
    assert_eq!(duration_millis(Some(&start), Some(&stop)), None);
}

#[tokio::test]
async fn executions_sort_fractional_timestamps_chronologically() {
    let body = serde_json::json!({"executions":[
        {"executionArn":"arn:aws:states:us-east-1:123456789012:execution:orders:whole", "name":"whole","stateMachineArn":SM_ARN,"status":"RUNNING","startDate":1700000000},
        {"executionArn":"arn:aws:states:us-east-1:123456789012:execution:orders:fraction", "name":"fraction","stateMachineArn":SM_ARN,"status":"RUNNING","startDate":1700000000.5}
    ]}).to_string();
    let http = StaticReplayClient::new(vec![ok(&body)]);
    let rows = client(&http)
        .list_executions(&standard("orders"))
        .await
        .unwrap();
    assert_eq!(
        rows.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
        ["fraction", "whole"]
    );
}

#[tokio::test]
async fn list_executions_maps_version_alias_and_redrive_metadata() {
    let version = format!("{SM_ARN}:42");
    let alias = format!("{SM_ARN}:PROD");
    let body = serde_json::json!({"executions":[{
        "executionArn":"arn:aws:states:us-east-1:123456789012:execution:orders:run",
        "stateMachineArn":SM_ARN,"name":"run","status":"SUCCEEDED",
        "startDate":1700000000,"stopDate":1700000005,
        "stateMachineVersionArn":version,"stateMachineAliasArn":alias,
        "redriveCount":2,"redriveDate":1700000003,
        "input":"must not be retained","output":"must not be retained"
    }]})
    .to_string();
    let http = StaticReplayClient::new(vec![ok(&body)]);
    let rows = client(&http)
        .list_executions(&standard("orders"))
        .await
        .unwrap();
    let row = &rows[0];
    assert_eq!(row.state_machine_version_arn, version);
    assert_eq!(row.state_machine_alias_arn, alias);
    assert_eq!(row.redrive_count, Some(2));
    assert_eq!(row.redrive_date, "2023-11-14T22:13:23Z");
    assert!(!format!("{row:?}").contains("must not be retained"));
}

#[tokio::test]
async fn missing_execution_qualifiers_and_redrive_count_stay_unknown_not_zero() {
    for count in [None, Some(0)] {
        let mut entry = serde_json::json!({
            "executionArn":"arn:aws:states:us-east-1:123456789012:execution:orders:run",
            "stateMachineArn":SM_ARN,"name":"run","status":"RUNNING","startDate":1700000000
        });
        if let Some(count) = count {
            entry["redriveCount"] = serde_json::json!(count);
        }
        let body = serde_json::json!({"executions":[entry]}).to_string();
        let http = StaticReplayClient::new(vec![ok(&body)]);
        let rows = client(&http)
            .list_executions(&standard("orders"))
            .await
            .unwrap();
        assert_eq!(rows[0].redrive_count, count);
        assert!(rows[0].state_machine_version_arn.is_empty());
        assert!(rows[0].state_machine_alias_arn.is_empty());
        assert!(rows[0].redrive_date.is_empty());
    }
}
