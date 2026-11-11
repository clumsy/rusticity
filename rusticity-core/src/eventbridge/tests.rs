use super::*;
use aws_smithy_http_client::test_util::infallible_client_fn;
use std::sync::{Arc, Mutex};

const ARN: &str = "arn:aws:events:us-east-2:123456789012:rule/orders/nightly";

/// Answers each request from `route(target)` (the X-Amz-Target operation, or
/// the form Action for CloudWatch) and records `<target> <body>`.
fn routed(
    route: impl Fn(&str) -> (u16, String) + Send + Sync + 'static,
) -> (AwsConfig, Arc<Mutex<Vec<String>>>) {
    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let record = seen.clone();
    let http = infallible_client_fn(move |request| {
        let body = request
            .body()
            .bytes()
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        let target = request
            .headers()
            .get("x-amz-target")
            .and_then(|t| t.to_str().ok())
            .map(|t| t.rsplit('.').next().unwrap_or_default().to_string())
            .unwrap_or_default();
        record.lock().unwrap().push(format!("{target} {body}"));
        let (status, out) = route(&target);
        http::Response::builder()
            .status(status)
            .body(out.into_bytes())
            .unwrap()
    });
    let sdk_config = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-2"))
        .credentials_provider(aws_sdk_eventbridge::config::SharedCredentialsProvider::new(
            aws_sdk_eventbridge::config::Credentials::new("test", "test", None, None, "test"),
        ))
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(http)
        .build();
    (
        AwsConfig::from_sdk_config(sdk_config, "us-east-2", "123456789012"),
        seen,
    )
}

const DENIED: &str = r#"{"__type":"AccessDeniedException","message":"no"}"#;

#[test]
fn rule_arns_name_the_bus_and_default_has_none() {
    assert_eq!(
        rule_identity(ARN).unwrap(),
        ("orders".to_string(), "nightly".to_string())
    );
    assert_eq!(
        rule_identity("arn:aws:events:us-east-2:123456789012:rule/nightly").unwrap(),
        ("default".to_string(), "nightly".to_string())
    );
    for bad in [
        "arn:aws:events:us-east-2:123456789012:event-bus/orders",
        "arn:aws:events:us-east-2:123:rule/x",
        "arn:aws:events:us-east-2:123456789012:rule/",
        "arn:aws:sqs:us-east-2:123456789012:rule/x",
        "nightly",
    ] {
        assert!(rule_identity(bad).is_err(), "{bad}");
    }
}

#[tokio::test]
async fn buses_list_default_first() {
    let (config, _) = routed(|_| {
        (
            200,
            r#"{"EventBuses":[{"Name":"zeta","Arn":"arn:z"},{"Name":"default","Arn":"arn:d"},{"Name":"alpha","Arn":"arn:a","CreationTime":1767225600}]}"#.into(),
        )
    });
    let buses = EventBridgeClient::new(config)
        .list_event_buses()
        .await
        .unwrap();
    let names: Vec<_> = buses.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, ["default", "alpha", "zeta"]);
    assert_eq!(buses[1].created, "2026-01-01T00:00:00Z");
}

#[tokio::test]
async fn rules_are_scoped_to_the_bus_and_a_foreign_rule_fails() {
    let (config, seen) = routed(|_| {
        (
            200,
            r#"{"Rules":[{"Name":"b","EventBusName":"orders","State":"ENABLED","ScheduleExpression":"rate(1 hour)"},{"Name":"a","EventBusName":"orders","State":"DISABLED","EventPattern":"{}"}]}"#.into(),
        )
    });
    let client = EventBridgeClient::new(config);
    let rules = client.list_rules("orders").await.unwrap();
    assert_eq!(rules[0].name, "a");
    assert_eq!(rules[0].rule_type(), "Standard");
    assert_eq!(rules[1].rule_type(), "Scheduled");
    assert!(seen.lock().unwrap()[0].contains(r#""EventBusName":"orders""#));
    assert!(
        client.list_rules("other").await.is_err(),
        "foreign bus rows"
    );
}

#[tokio::test]
async fn rule_details_verify_identity_and_sections_fail_alone() {
    let (config, seen) = routed(|target| {
        match target {
        "DescribeRule" => (
            200,
            format!(r#"{{"Name":"nightly","Arn":"{ARN}","EventBusName":"orders","State":"ENABLED","CreatedBy":"123456789012"}}"#),
        ),
        "ListTargetsByRule" => (
            200,
            r#"{"Targets":[{"Id":"2","Arn":"arn:aws:sqs:us-east-2:123456789012:q","InputPath":"$.detail"},{"Id":"1","Arn":"arn:aws:lambda:us-east-2:123456789012:function:f"}]}"#.into(),
        ),
        "ListTagsForResource" => (200, r#"{"Tags":[{"Key":"team","Value":"data"}]}"#.into()),
        _ => (403, DENIED.into()),
    }
    });
    let detail = EventBridgeClient::new(config)
        .rule_details(ARN)
        .await
        .unwrap();
    assert_eq!(detail.rule.name, "nightly");
    assert_eq!(detail.created_by, "123456789012");
    let targets = detail.targets.as_ref().unwrap();
    assert_eq!(targets[0].id, "1");
    assert_eq!(targets[1].input, "$.detail");
    assert_eq!(detail.tags, Ok(vec![("team".into(), "data".into())]));
    assert!(detail.metrics.unwrap_err().contains("CloudWatch metrics"));
    let seen = seen.lock().unwrap();
    let describe = seen.iter().find(|r| r.starts_with("DescribeRule")).unwrap();
    assert!(
        describe.contains(r#""Name":"nightly""#) && describe.contains(r#""EventBusName":"orders""#)
    );
    for verb in ["Put", "Delete", "Enable", "Disable", "Remove"] {
        assert!(
            !seen.iter().any(|r| r.starts_with(verb)),
            "read-only: {verb}"
        );
    }
}

#[tokio::test]
async fn mismatched_or_foreign_rules_are_rejected() {
    let (config, seen) = routed(|_| {
        (
            200,
            r#"{"Name":"nightly","Arn":"arn:aws:events:us-east-2:123456789012:rule/orders/other"}"#
                .into(),
        )
    });
    let client = EventBridgeClient::new(config);
    let error = client.rule_details(ARN).await.unwrap_err();
    assert!(error.to_string().contains("mismatched identity"));
    seen.lock().unwrap().clear();
    for arn in [
        "arn:aws:events:eu-west-1:123456789012:rule/orders/nightly",
        "arn:aws:events:us-east-2:999999999999:rule/orders/nightly",
    ] {
        assert!(client.rule_details(arn).await.is_err(), "{arn}");
    }
    assert!(
        seen.lock().unwrap().is_empty(),
        "no request for a foreign ARN"
    );
}

#[test]
fn bus_type_owner_and_schema_discovery() {
    let bus = |name: &str, arn: &str| EventBus {
        name: name.into(),
        arn: arn.into(),
        ..Default::default()
    };
    let mut buses = vec![
        bus(
            "default",
            "arn:aws:events:us-east-2:123456789012:event-bus/default",
        ),
        bus(
            "orders",
            "arn:aws:events:us-east-2:123456789012:event-bus/orders",
        ),
        bus(
            "aws.partner/x/y",
            "arn:aws:events:us-east-2:210987654321:event-bus/aws.partner/x/y",
        ),
    ];
    let types: Vec<_> = buses.iter().map(EventBus::bus_type).collect();
    assert_eq!(types, ["Default", "Custom Event Bus - Classic", "Partner"]);
    assert_eq!(buses[2].owner_account(), "210987654321");
    let rows = vec![(buses[1].arn.clone(), "STARTED".to_string())];
    apply_discoverers(&mut buses, Some(&rows));
    assert_eq!(buses[1].schema_discovery, "STARTED");
    assert!(buses[0].schema_discovery.is_empty(), "no discoverer");
    apply_discoverers(&mut buses, None);
    assert_eq!(buses[0].schema_discovery, "Unavailable");
}
