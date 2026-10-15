use super::*;
use aws_smithy_http_client::test_util::{ReplayEvent, StaticReplayClient};
use aws_smithy_types::body::SdkBody;

const ENDPOINT: &str = "https://sns.us-east-1.amazonaws.com/";

fn page(members: &[&str], next_token: Option<&str>) -> String {
    let members: String = members
        .iter()
        .map(|arn| format!("<member><TopicArn>{arn}</TopicArn></member>"))
        .collect();
    let token = next_token
        .map(|t| format!("<NextToken>{t}</NextToken>"))
        .unwrap_or_default();
    format!(
        "<ListTopicsResponse xmlns=\"http://sns.amazonaws.com/doc/2010-03-31/\">\
<ListTopicsResult><Topics>{members}</Topics>{token}</ListTopicsResult>\
<ResponseMetadata><RequestId>req-id</RequestId></ResponseMetadata></ListTopicsResponse>"
    )
}

fn error_response(code: &str, message: &str) -> String {
    format!(
        "<ErrorResponse xmlns=\"http://sns.amazonaws.com/doc/2010-03-31/\">\
<Error><Type>Sender</Type><Code>{code}</Code><Message>{message}</Message></Error>\
<RequestId>req-id</RequestId></ErrorResponse>"
    )
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

fn client(http_client: &StaticReplayClient) -> SnsClient {
    let sdk_config = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-1"))
        .credentials_provider(aws_sdk_sns::config::SharedCredentialsProvider::new(
            aws_sdk_sns::config::Credentials::new("test-key", "test-secret", None, None, "test"),
        ))
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(http_client.clone())
        .build();
    SnsClient::new(AwsConfig::from_sdk_config(
        sdk_config,
        "us-east-1",
        "123456789012",
    ))
}

// ── ARN parsing ────────────────────────────────────────────────────────────

#[test]
fn from_arn_derives_name_and_standard_vs_fifo() {
    let std = SnsTopic::from_arn("arn:aws:sns:us-east-1:123456789012:orders").unwrap();
    assert_eq!(std.name, "orders");
    assert_eq!(std.topic_type, "Standard");
    assert_eq!(std.arn, "arn:aws:sns:us-east-1:123456789012:orders");

    let fifo = SnsTopic::from_arn("arn:aws:sns:us-east-1:123456789012:orders.fifo").unwrap();
    assert_eq!(fifo.name, "orders.fifo");
    assert_eq!(fifo.topic_type, "FIFO");

    // Non-default partition is still a valid SNS ARN.
    let gov = SnsTopic::from_arn("arn:aws-us-gov:sns:us-gov-west-1:123456789012:topic").unwrap();
    assert_eq!(gov.name, "topic");
    assert_eq!(gov.topic_type, "Standard");
}

#[test]
fn from_arn_rejects_malformed_and_non_sns() {
    for bad in [
        "",
        "not-an-arn",
        "arn:aws:sqs:us-east-1:123456789012:queue", // wrong service
        "arn:aws:sns:us-east-1:123456789012:",      // empty name
        "arn:aws:sns:us-east-1:123456789012",       // missing name segment
    ] {
        assert!(
            SnsTopic::from_arn(bad).is_err(),
            "{bad:?} should be rejected"
        );
    }
}

// ── list_topics pagination ───────────────────────────────────────────────────

#[tokio::test]
async fn list_topics_follows_tokens_and_sorts_until_empty_token() {
    let http = StaticReplayClient::new(vec![
        ok(page(
            &[
                "arn:aws:sns:us-east-1:123456789012:zeta",
                "arn:aws:sns:us-east-1:123456789012:alpha.fifo",
            ],
            Some("tok-1"),
        )),
        ok(page(
            &["arn:aws:sns:us-east-1:123456789012:mid"],
            // Final page signalled by an EMPTY token (must still terminate).
            Some(""),
        )),
    ]);
    let topics = client(&http).list_topics().await.unwrap();
    let names: Vec<_> = topics.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["alpha.fifo", "mid", "zeta"]); // sorted
    assert_eq!(topics[0].topic_type, "FIFO");
    assert_eq!(topics[1].topic_type, "Standard");
}

#[tokio::test]
async fn list_topics_terminates_on_absent_token() {
    let http = StaticReplayClient::new(vec![ok(page(
        &["arn:aws:sns:us-east-1:123456789012:only"],
        None,
    ))]);
    let topics = client(&http).list_topics().await.unwrap();
    assert_eq!(topics.len(), 1);
    assert_eq!(topics[0].name, "only");
}

#[tokio::test]
async fn list_topics_handles_empty_account() {
    let http = StaticReplayClient::new(vec![ok(page(&[], None))]);
    let topics = client(&http).list_topics().await.unwrap();
    assert!(topics.is_empty());
}

#[tokio::test]
async fn list_topics_errors_on_repeated_token_instead_of_looping() {
    let http = StaticReplayClient::new(vec![
        ok(page(
            &["arn:aws:sns:us-east-1:123456789012:a"],
            Some("same"),
        )),
        // Service (incorrectly) returns the SAME token: must abort, not loop.
        ok(page(
            &["arn:aws:sns:us-east-1:123456789012:b"],
            Some("same"),
        )),
    ]);
    let err = client(&http).list_topics().await.unwrap_err();
    assert!(
        err.to_string().contains("repeated pagination token"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn list_topics_surfaces_malformed_arn_as_error_not_empty_success() {
    let http = StaticReplayClient::new(vec![ok(page(
        &["arn:aws:sns:us-east-1:123456789012:good", "garbage-arn"],
        None,
    ))]);
    let err = client(&http).list_topics().await.unwrap_err();
    assert!(
        err.to_string().contains("not an SNS topic ARN"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn list_topics_surfaces_denied_access_error() {
    let http = StaticReplayClient::new(vec![event(
        403,
        error_response(
            "AuthorizationError",
            "User is not authorized to perform sns:ListTopics",
        ),
    )]);
    let err = client(&http).list_topics().await.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("ListTopics request failed"),
        "unexpected error context: {msg}"
    );
    // The underlying SDK error (authorization) is preserved in the chain.
    let chain = format!("{err:#}");
    assert!(
        chain.contains("AuthorizationError") || chain.contains("not authorized"),
        "authorization cause missing: {chain}"
    );
}

#[test]
fn rejects_incomplete_arns_and_subscription_identifiers() {
    for arn in [
        "arn::sns:us-east-1:123456789012:topic",
        "arn:aws:sns::123456789012:topic",
        "arn:aws:sns:us-east-1::topic",
        "arn:aws:sns:us-east-1:not-account:topic",
        "arn:aws:sns:us-east-1:123456789012:topic:subscription-id",
        "arn:aws:sns:us-east-1:123456789012:.fifo",
        "arn:aws:sns:us-east-1:123456789012:bad name",
    ] {
        assert!(SnsTopic::from_arn(arn).is_err(), "accepted {arn}");
    }
}

#[tokio::test]
async fn detects_longer_token_cycles_and_reports_page_bound() {
    let http = StaticReplayClient::new(vec![
        ok(page(&[], Some("a"))),
        ok(page(&[], Some("b"))),
        ok(page(&[], Some("a"))),
    ]);
    assert!(client(&http)
        .list_topics()
        .await
        .unwrap_err()
        .to_string()
        .contains("repeated pagination token"));
    let http = StaticReplayClient::new(vec![ok(page(&[], Some("a"))), ok(page(&[], Some("b")))]);
    assert!(client(&http)
        .list_topics_inner(2)
        .await
        .unwrap_err()
        .to_string()
        .contains("inventory is incomplete"));
}

#[tokio::test]
async fn requests_only_list_topics_and_sends_returned_continuation_token() {
    use aws_smithy_http_client::test_util::infallible_client_fn;
    use std::sync::{Arc, Mutex};
    let captured = Arc::new(Mutex::new(Vec::<String>::new()));
    let requests = captured.clone();
    let config = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-1"))
        .credentials_provider(aws_sdk_sns::config::SharedCredentialsProvider::new(
            aws_sdk_sns::config::Credentials::new("test", "test", None, None, "test"),
        ))
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(infallible_client_fn(move |request| {
            let body = String::from_utf8(request.body().bytes().unwrap().to_vec()).unwrap();
            assert!(body.contains("Action=ListTopics"));
            assert!(!body.contains("MaxResults"));
            let mut requests = requests.lock().unwrap();
            requests.push(body);
            let response = if requests.len() == 1 {
                page(
                    &["arn:aws:sns:us-east-1:123456789012:first"],
                    Some("next-page"),
                )
            } else {
                assert_eq!(requests.len(), 2);
                page(&["arn:aws:sns:us-east-1:123456789012:second.fifo"], None)
            };
            http::Response::builder()
                .status(200)
                .body(response.into_bytes())
                .unwrap()
        }))
        .build();
    let topics = SnsClient::new(AwsConfig::from_sdk_config(
        config,
        "us-east-1",
        "123456789012",
    ))
    .list_topics()
    .await
    .unwrap();
    assert_eq!(topics.len(), 2);
    let requests = captured.lock().unwrap();
    assert!(!requests[0].contains("NextToken"));
    assert!(requests[1].contains("NextToken=next-page"));
}
