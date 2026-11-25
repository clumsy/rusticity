use super::*;
use aws_smithy_http_client::test_util::infallible_client_fn;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

fn config(http: aws_sdk_resourceexplorer2::config::SharedHttpClient) -> AwsConfig {
    let sdk_config = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-1"))
        .credentials_provider(aws_sdk_iam::config::SharedCredentialsProvider::new(
            aws_sdk_iam::config::Credentials::new("test-key", "test-secret", None, None, "test"),
        ))
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(http)
        .build();
    AwsConfig::from_sdk_config(sdk_config, "us-east-1", "123456789012")
}

fn reply(status: u16, body: Value) -> http::Response<String> {
    http::Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(body.to_string())
        .unwrap()
}

fn instance(id: &str, tags: Option<Value>) -> Value {
    let mut resource = json!({
        "Arn": format!("arn:aws:ec2:us-east-1:123456789012:instance/{id}"),
        "OwningAccountId": "123456789012",
        "Region": "us-east-1",
        "ResourceType": "ec2:instance",
        "Service": "ec2",
        "LastReportedAt": "2026-11-20T10:00:00Z",
    });
    if let Some(tags) = tags {
        resource["Properties"] =
            json!([{"Name": "tags", "LastReportedAt": "2026-11-20T10:00:00Z", "Data": tags}]);
    }
    resource
}

#[test]
fn identifiers_come_from_the_arn_resource_part() {
    for (arn, id) in [
        ("arn:aws:ec2:us-east-1:1:instance/i-0abc", "i-0abc"),
        ("arn:aws:lambda:us-east-1:1:function:orders", "orders"),
        (
            "arn:aws:logs:us-east-1:1:log-group:/aws/lambda/x",
            "/aws/lambda/x",
        ),
        (
            "arn:aws:elasticloadbalancing:us-east-1:1:loadbalancer/app/web/50dc",
            "app/web/50dc",
        ),
        ("arn:aws:apigateway:us-east-1::/restapis/abc123", "abc123"),
        ("arn:aws:s3:::my-bucket", "my-bucket"),
        ("arn:aws:sqs:us-east-1:1:jobs", "jobs"),
        ("arn:aws:iam::1:role/admin", "admin"),
    ] {
        assert_eq!(identifier_from_arn(arn), id, "{arn}");
    }
}

#[test]
fn a_resource_maps_its_console_fields_and_application_name() {
    let sdk: SdkResource = {
        SdkResource::builder()
            .arn("arn:aws:ec2:us-east-1:123456789012:instance/i-0abc")
            .owning_account_id("123456789012")
            .region("us-east-1")
            .resource_type("ec2:instance")
            .service("ec2")
            .properties(
                aws_sdk_resourceexplorer2::types::ResourceProperty::builder()
                    .name("tags")
                    .data(Document::Array(vec![
                        Document::Object(
                            [
                                ("Key".to_string(), Document::String("Stage".into())),
                                ("Value".to_string(), Document::String("prod".into())),
                            ]
                            .into(),
                        ),
                        Document::Object(
                            [
                                ("Key".to_string(), Document::String("awsApplication".into())),
                                (
                                    "Value".to_string(),
                                    Document::String(
                                        "arn:aws:resource-groups:us-east-1:123456789012:group/Billing/0a1b2c".into(),
                                    ),
                                ),
                            ]
                            .into(),
                        ),
                    ]))
                    .build(),
            )
            .build()
    };
    let r = ExplorerResource::from_sdk(&sdk).unwrap();
    assert_eq!(r.identifier, "i-0abc");
    assert_eq!(r.service, "ec2");
    assert_eq!(r.resource_type, "ec2:instance");
    assert_eq!(r.region, "us-east-1");
    assert_eq!(r.owner_account, "123456789012");
    assert_eq!(r.application, "Billing");
    let tags = r.tags.unwrap();
    assert_eq!(tags[0].0, "Stage", "sorted by key, uppercase first");
    assert_eq!(tags.len(), 2);
    assert_eq!(application_name("not-an-arn"), "not-an-arn");
}

#[test]
fn a_view_without_tags_is_not_shown_as_untagged() {
    let sdk = SdkResource::builder()
        .arn("arn:aws:s3:::logs")
        .service("s3")
        .resource_type("s3:bucket")
        .region("us-east-1")
        .build();
    let r = ExplorerResource::from_sdk(&sdk).unwrap();
    assert_eq!(r.tags, None);
    assert!(r.application.is_empty());
    assert!(ExplorerResource::from_sdk(&SdkResource::builder().build()).is_none());
}

#[tokio::test]
async fn search_sends_the_query_as_typed_and_follows_pages() {
    let bodies = Arc::new(Mutex::new(Vec::new()));
    let seen = bodies.clone();
    let http = infallible_client_fn(move |request| {
        assert_eq!(request.uri().path(), "/Search");
        let body: Value = serde_json::from_slice(request.body().bytes().unwrap()).unwrap();
        seen.lock().unwrap().push(body.clone());
        if body["NextToken"] == "page2" {
            reply(
                200,
                json!({
                    "Resources": [instance("i-0b", Some(json!([])))],
                    "ViewArn": "arn:aws:resource-explorer-2:us-east-1:123456789012:view/default/1",
                    "Count": {"TotalResources": 2, "Complete": true},
                }),
            )
        } else {
            reply(
                200,
                json!({
                    "Resources": [instance("i-0a", Some(json!([{"Key": "Name", "Value": "web"}])))],
                    "NextToken": "page2",
                    "Count": {"TotalResources": 2, "Complete": true},
                }),
            )
        }
    });
    let found = search(&config(http), "-tag:none service:ec2")
        .await
        .unwrap();
    let ids: Vec<_> = found
        .resources
        .iter()
        .map(|r| r.identifier.as_str())
        .collect();
    assert_eq!(ids, ["i-0a", "i-0b"]);
    assert!(found.complete);
    assert!(found.view_arn.ends_with("view/default/1"));
    assert_eq!(
        found.resources[1].tags,
        Some(vec![]),
        "tagged view, no tags"
    );
    let bodies = bodies.lock().unwrap();
    assert_eq!(bodies.len(), 2);
    assert_eq!(bodies[0]["QueryString"], "-tag:none service:ec2");
    assert_eq!(bodies[0]["MaxResults"], 1000);
}

#[tokio::test]
async fn an_incomplete_count_is_reported() {
    let http = infallible_client_fn(|_| {
        reply(
            200,
            json!({
                "Resources": [instance("i-0a", None)],
                "Count": {"TotalResources": 1000, "Complete": false},
            }),
        )
    });
    let found = search(&config(http), "").await.unwrap();
    assert!(!found.complete);
    assert_eq!(found.resources.len(), 1);
}

#[tokio::test]
async fn results_stop_at_the_limit_and_say_so() {
    let http = infallible_client_fn(|request| {
        let body: Value = serde_json::from_slice(request.body().bytes().unwrap()).unwrap();
        let page: usize = body["NextToken"]
            .as_str()
            .and_then(|t| t.strip_prefix('p'))
            .map_or(0, |n| n.parse().unwrap());
        let rows: Vec<Value> = (0..600)
            .map(|i| instance(&format!("i-{page}-{i}"), None))
            .collect();
        reply(
            200,
            json!({"Resources": rows, "NextToken": format!("p{}", page + 1)}),
        )
    });
    let found = search(&config(http), "").await.unwrap();
    assert_eq!(found.resources.len(), RESULT_LIMIT);
    assert!(!found.complete);
}

#[tokio::test]
async fn a_failed_search_is_an_error() {
    let http = infallible_client_fn(|_| {
        http::Response::builder()
            .status(401)
            .header("x-amzn-errortype", "UnauthorizedException")
            .body(json!({"message": "No default view in this Region"}).to_string())
            .unwrap()
    });
    let error = search(&config(http), "").await.unwrap_err();
    assert!(
        format!("{error:#}").contains("UnauthorizedException")
            || format!("{error:#}").contains("No default view"),
        "{error:#}"
    );
}
