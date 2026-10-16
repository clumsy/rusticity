use super::*;
use crate::config::AwsConfig;
use aws_smithy_http_client::test_util::{infallible_client_fn, ReplayEvent, StaticReplayClient};
use aws_smithy_types::body::SdkBody;
use std::sync::{Arc, Mutex};

const ENDPOINT: &str = "https://sns.us-east-1.amazonaws.com/";
const TOPIC: &str = "arn:aws:sns:us-east-1:123456789012:orders";

/// One <member> for the ListSubscriptionsByTopic XML result. `arn` goes into
/// SubscriptionArn verbatim so we can exercise the ARN-literal status mapping.
fn member(
    subscription_arn: &str,
    topic_arn: &str,
    protocol: &str,
    endpoint: &str,
    owner: &str,
) -> String {
    format!(
        "<member>\
<SubscriptionArn>{subscription_arn}</SubscriptionArn>\
<Owner>{owner}</Owner>\
<Protocol>{protocol}</Protocol>\
<Endpoint>{endpoint}</Endpoint>\
<TopicArn>{topic_arn}</TopicArn>\
</member>"
    )
}

/// A page whose <member> elements are supplied pre-rendered, plus an optional
/// NextToken. `topic_arn` is emitted only when `Some` so we can test the
/// response-omits-TopicArn fallback path.
fn page(members: &[String], next_token: Option<&str>) -> String {
    let members: String = members.concat();
    let token = next_token
        .map(|t| format!("<NextToken>{t}</NextToken>"))
        .unwrap_or_default();
    format!(
        "<ListSubscriptionsByTopicResponse xmlns=\"http://sns.amazonaws.com/doc/2010-03-31/\">\
<ListSubscriptionsByTopicResult><Subscriptions>{members}</Subscriptions>{token}</ListSubscriptionsByTopicResult>\
<ResponseMetadata><RequestId>req-id</RequestId></ResponseMetadata></ListSubscriptionsByTopicResponse>"
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

fn client_in(http_client: &StaticReplayClient, region: &str, account: &str) -> SnsClient {
    let sdk_config = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()))
        .credentials_provider(aws_sdk_sns::config::SharedCredentialsProvider::new(
            aws_sdk_sns::config::Credentials::new("test-key", "test-secret", None, None, "test"),
        ))
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(http_client.clone())
        .build();
    SnsClient::new(AwsConfig::from_sdk_config(sdk_config, region, account))
}

fn client(http_client: &StaticReplayClient) -> SnsClient {
    client_in(http_client, "us-east-1", "123456789012")
}

// ── status classification ───────────────────────────────────────────────────

#[test]
fn status_maps_from_arn_literal_scoped_to_topic() {
    let confirmed = format!("{TOPIC}:11111111-1111-1111-1111-111111111111");
    for (arn, expected) in [
        ("PendingConfirmation", "Pending confirmation"),
        ("Deleted", "Deleted"),
        (confirmed.as_str(), "Confirmed"),
        ("", "Unknown"),
        ("something-else", "Unknown"),
        // A real-looking ARN for a DIFFERENT topic must NOT be asserted confirmed.
        (
            "arn:aws:sns:us-east-1:123456789012:other:22222222-2222-2222-2222-222222222222",
            "Unknown",
        ),
        // Topic prefix without the `:suffix` is not a subscription ARN.
        (TOPIC, "Unknown"),
    ] {
        assert_eq!(
            SnsSubscription::classify_status(arn, TOPIC),
            expected,
            "arn {arn:?}"
        );
    }
}

// ── happy path: pending / confirmed / deleted / unknown + sort ───────────────

#[tokio::test]
async fn maps_statuses_and_sorts_by_protocol_endpoint_arn() {
    let confirmed_arn = format!("{TOPIC}:aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa");
    let members = vec![
        member(
            "PendingConfirmation",
            TOPIC,
            "sqs",
            "arn:z-queue",
            "123456789012",
        ),
        member(
            &confirmed_arn,
            TOPIC,
            "email",
            "ops@example.com",
            "123456789012",
        ),
        member(
            "Deleted",
            TOPIC,
            "https",
            "https://hook.example",
            "123456789012",
        ),
        member("not-an-arn", TOPIC, "lambda", "arn:fn", ""),
    ];
    let http = StaticReplayClient::new(vec![ok(page(&members, None))]);
    let subs = client(&http)
        .list_subscriptions_by_topic(TOPIC)
        .await
        .unwrap();

    // Sorted by protocol, then endpoint, then subscription ARN.
    let protocols: Vec<_> = subs.iter().map(|s| s.protocol.as_str()).collect();
    assert_eq!(protocols, ["email", "https", "lambda", "sqs"]);

    let by_proto = |p: &str| subs.iter().find(|s| s.protocol == p).unwrap();
    assert_eq!(by_proto("email").status, "Confirmed");
    assert_eq!(by_proto("email").subscription_arn, confirmed_arn);
    assert_eq!(by_proto("https").status, "Deleted");
    assert_eq!(by_proto("sqs").status, "Pending confirmation");
    assert_eq!(by_proto("lambda").status, "Unknown");
    // Owner/endpoint/protocol are passed through raw; no invented PII.
    assert_eq!(by_proto("lambda").owner, "");
    assert_eq!(by_proto("email").owner, "123456789012");
    // topic_arn is populated from the response.
    assert!(subs.iter().all(|s| s.topic_arn == TOPIC));
}

#[tokio::test]
async fn retains_duplicate_pending_subscriptions_with_same_endpoint() {
    let members = vec![
        member(
            "PendingConfirmation",
            TOPIC,
            "email",
            "dup@example.com",
            "123456789012",
        ),
        member(
            "PendingConfirmation",
            TOPIC,
            "email",
            "dup@example.com",
            "123456789012",
        ),
    ];
    let http = StaticReplayClient::new(vec![ok(page(&members, None))]);
    let subs = client(&http)
        .list_subscriptions_by_topic(TOPIC)
        .await
        .unwrap();
    assert_eq!(subs.len(), 2, "duplicate pending must not be dropped");
    assert!(subs.iter().all(|s| s.status == "Pending confirmation"));
}

#[tokio::test]
async fn empty_list_is_empty_success() {
    let http = StaticReplayClient::new(vec![ok(page(&[], None))]);
    let subs = client(&http)
        .list_subscriptions_by_topic(TOPIC)
        .await
        .unwrap();
    assert!(subs.is_empty());
}

// ── pagination ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn follows_tokens_until_empty_token() {
    let a = format!("{TOPIC}:aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa");
    let b = format!("{TOPIC}:bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb");
    let http = StaticReplayClient::new(vec![
        ok(page(
            &[member(&a, TOPIC, "sqs", "arn:q1", "123456789012")],
            Some("tok-1"),
        )),
        // Final page signalled by an EMPTY token (must still terminate).
        ok(page(
            &[member(&b, TOPIC, "sqs", "arn:q2", "123456789012")],
            Some(""),
        )),
    ]);
    let subs = client(&http)
        .list_subscriptions_by_topic(TOPIC)
        .await
        .unwrap();
    assert_eq!(subs.len(), 2);
    assert!(subs.iter().all(|s| s.status == "Confirmed"));
}

#[tokio::test]
async fn sends_topic_arn_url_encoded_and_exact_next_token_without_max_results() {
    let captured = Arc::new(Mutex::new(Vec::<String>::new()));
    let requests = captured.clone();
    let a = format!("{TOPIC}:aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa");
    let config = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-1"))
        .credentials_provider(aws_sdk_sns::config::SharedCredentialsProvider::new(
            aws_sdk_sns::config::Credentials::new("test", "test", None, None, "test"),
        ))
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(infallible_client_fn(move |request| {
            let body = String::from_utf8(request.body().bytes().unwrap().to_vec()).unwrap();
            assert!(body.contains("Action=ListSubscriptionsByTopic"));
            assert!(!body.contains("MaxResults"));
            // TopicArn is form-URL-encoded in the request body.
            assert!(body.contains("TopicArn=arn%3Aaws%3Asns%3Aus-east-1%3A123456789012%3Aorders"));
            let mut requests = requests.lock().unwrap();
            requests.push(body);
            let response = if requests.len() == 1 {
                page(
                    &[member(&a, TOPIC, "sqs", "arn:q", "123456789012")],
                    Some("next-page"),
                )
            } else {
                assert_eq!(requests.len(), 2);
                page(&[], None)
            };
            http::Response::builder()
                .status(200)
                .body(response.into_bytes())
                .unwrap()
        }))
        .build();
    let subs = SnsClient::new(AwsConfig::from_sdk_config(
        config,
        "us-east-1",
        "123456789012",
    ))
    .list_subscriptions_by_topic(TOPIC)
    .await
    .unwrap();
    assert_eq!(subs.len(), 1);
    let requests = captured.lock().unwrap();
    assert!(!requests[0].contains("NextToken"));
    assert!(requests[1].contains("NextToken=next-page"));
}

#[tokio::test]
async fn repeated_token_aborts_instead_of_looping() {
    let http = StaticReplayClient::new(vec![
        ok(page(&[], Some("same"))),
        ok(page(&[], Some("same"))),
    ]);
    let err = client(&http)
        .list_subscriptions_by_topic(TOPIC)
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("repeated pagination token"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn detects_longer_token_cycles() {
    let http = StaticReplayClient::new(vec![
        ok(page(&[], Some("a"))),
        ok(page(&[], Some("b"))),
        ok(page(&[], Some("a"))),
    ]);
    assert!(client(&http)
        .list_subscriptions_by_topic(TOPIC)
        .await
        .unwrap_err()
        .to_string()
        .contains("repeated pagination token"));
}

#[tokio::test]
async fn final_page_at_bound_is_complete_but_incomplete_bound_errors() {
    // A final page landing exactly on the bound is complete.
    let http = StaticReplayClient::new(vec![
        ok(page(&[], Some("next"))),
        ok(page(
            &[member(
                "PendingConfirmation",
                TOPIC,
                "sqs",
                "arn:q",
                "123456789012",
            )],
            None,
        )),
    ]);
    let subs = client(&http)
        .list_subscriptions_by_topic_inner(TOPIC, 2)
        .await
        .unwrap();
    assert_eq!(subs.len(), 1);

    // One more page needed than the bound allows => explicit incomplete error.
    let http = StaticReplayClient::new(vec![ok(page(&[], Some("a"))), ok(page(&[], Some("b")))]);
    let err = client(&http)
        .list_subscriptions_by_topic_inner(TOPIC, 2)
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("subscription list is incomplete"),
        "unexpected error: {err}"
    );

    // Zero bound never sends a request and reports incomplete.
    let http = StaticReplayClient::new(vec![]);
    let err = client(&http)
        .list_subscriptions_by_topic_inner(TOPIC, 0)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("exceeded 0 pages"));
}

// ── validation: input rejected before any request ────────────────────────────

#[tokio::test]
async fn rejects_missing_and_malformed_arn_without_network() {
    // No replay events => any network call panics the StaticReplayClient.
    for bad in [
        "",
        "not-an-arn",
        "arn:aws:sqs:us-east-1:123456789012:queue",
        "arn:aws:sns:us-east-1:123456789012:",
        "arn:aws:sns:us-east-1:not-account:topic",
    ] {
        let http = StaticReplayClient::new(vec![]);
        assert!(
            client(&http)
                .list_subscriptions_by_topic(bad)
                .await
                .is_err(),
            "{bad:?} should be rejected"
        );
    }
}

#[tokio::test]
async fn rejects_account_and_region_mismatch_without_network() {
    // Active config is a different account.
    let http = StaticReplayClient::new(vec![]);
    let err = client_in(&http, "us-east-1", "999999999999")
        .list_subscriptions_by_topic(TOPIC)
        .await
        .unwrap_err();
    assert!(err
        .to_string()
        .contains("does not match the active account"));

    // Active config is a different Region.
    let http = StaticReplayClient::new(vec![]);
    let err = client_in(&http, "eu-west-1", "123456789012")
        .list_subscriptions_by_topic(TOPIC)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("does not match the active Region"));
}

// ── response validation / error surfacing ─────────────────────────────────────

#[tokio::test]
async fn mismatched_response_topic_arn_is_an_error() {
    let foreign = "arn:aws:sns:us-east-1:123456789012:other:cccccccc-cccc-cccc-cccc-cccccccccccc";
    let members = vec![member(
        foreign,
        "arn:aws:sns:us-east-1:123456789012:other",
        "sqs",
        "arn:q",
        "123456789012",
    )];
    let http = StaticReplayClient::new(vec![ok(page(&members, None))]);
    let err = client(&http)
        .list_subscriptions_by_topic(TOPIC)
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("expected"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn missing_response_topic_arn_is_scoped_to_requested_arn() {
    // Emit members without a TopicArn element; result must carry requested ARN.
    let member_no_topic = format!(
        "<member><SubscriptionArn>{TOPIC}:dddddddd-dddd-dddd-dddd-dddddddddddd</SubscriptionArn>\
<Owner>123456789012</Owner><Protocol>sqs</Protocol><Endpoint>arn:q</Endpoint></member>"
    );
    let http = StaticReplayClient::new(vec![ok(page(&[member_no_topic], None))]);
    let subs = client(&http)
        .list_subscriptions_by_topic(TOPIC)
        .await
        .unwrap();
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0].topic_arn, TOPIC);
    assert_eq!(subs[0].status, "Confirmed");
}

#[tokio::test]
async fn denied_access_surfaces_error_not_empty_success() {
    let http = StaticReplayClient::new(vec![event(
        403,
        error_response(
            "AuthorizationError",
            "User is not authorized to perform sns:ListSubscriptionsByTopic",
        ),
    )]);
    let err = client(&http)
        .list_subscriptions_by_topic(TOPIC)
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("ListSubscriptionsByTopic request failed"),
        "unexpected context: {err}"
    );
    // Underlying AWS metadata/error chain is preserved.
    let chain = format!("{err:#}");
    assert!(
        chain.contains("AuthorizationError") || chain.contains("not authorized"),
        "authorization cause missing: {chain}"
    );
}
