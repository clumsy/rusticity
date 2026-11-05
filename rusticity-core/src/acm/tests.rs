//! SDK-replay tests for the ACM inventory. ACM speaks AWS JSON 1.1; the
//! operation is the `X-Amz-Target` header (`CertificateManager.<Op>`), so every
//! call is recorded to prove only ListCertificates is ever issued.
use super::*;
use aws_smithy_http_client::test_util::infallible_client_fn;
use std::sync::{Arc, Mutex};

const REGION: &str = "us-east-1";
const ACCOUNT: &str = "123456789012";
const ISSUED: f64 = 1_576_540_098.0; // 2019-12-16T23:48:18Z
const EXPIRES: f64 = 1_608_162_498.0; // 2020-12-16T23:48:18Z

fn arn(id: &str) -> String {
    format!("arn:aws:acm:{REGION}:{ACCOUNT}:certificate/{id}")
}

#[derive(Clone, Default)]
struct Mock {
    pages: Arc<Mutex<Vec<(u16, String)>>>,
    seen: Arc<Mutex<Vec<(String, serde_json::Value)>>>,
}

impl Mock {
    fn page(self, status: u16, body: impl Into<String>) -> Self {
        self.pages.lock().unwrap().push((status, body.into()));
        self
    }
    fn ops(&self) -> Vec<String> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|(op, _)| op.clone())
            .collect()
    }
    fn bodies(&self) -> Vec<serde_json::Value> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|(_, b)| b.clone())
            .collect()
    }
    fn client(&self) -> AcmClient {
        let pages = self.pages.clone();
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
                .and_then(|b| serde_json::from_slice(b).ok())
                .unwrap_or(serde_json::Value::Null);
            seen.lock().unwrap().push((op.clone(), body));
            let mut pages = pages.lock().unwrap();
            let (status, body) = if op != "ListCertificates" {
                (400, format!("{{\"__type\":\"UnexpectedOperation:{op}\"}}"))
            } else if pages.len() > 1 {
                pages.remove(0)
            } else {
                pages[0].clone()
            };
            http::Response::builder()
                .status(status)
                .body(body.into_bytes())
                .unwrap()
        });
        let sdk_config = aws_config::SdkConfig::builder()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(REGION))
            .credentials_provider(aws_sdk_acm::config::SharedCredentialsProvider::new(
                aws_sdk_acm::config::Credentials::new("test", "test", None, None, "test"),
            ))
            .retry_config(aws_config::retry::RetryConfig::disabled())
            .http_client(http)
            .build();
        AcmClient::new(AwsConfig::from_sdk_config(sdk_config, REGION, ACCOUNT))
    }
}

fn summary(id: &str, domain: &str) -> serde_json::Value {
    serde_json::json!({"CertificateArn": arn(id), "DomainName": domain})
}

fn body(entries: Vec<serde_json::Value>, next: Option<&str>) -> String {
    let mut body = serde_json::json!({ "CertificateSummaryList": entries });
    if let Some(next) = next {
        body["NextToken"] = serde_json::json!(next);
    }
    body.to_string()
}

#[tokio::test]
async fn maps_every_summary_field_without_inventing_absent_values() {
    let full = serde_json::json!({
        "CertificateArn": arn("full-1"),
        "DomainName": "example.com",
        "SubjectAlternativeNameSummaries": ["example.com", "www.example.com"],
        "HasAdditionalSubjectAlternativeNames": true,
        "Status": "ISSUED",
        "Type": "AMAZON_ISSUED",
        "KeyAlgorithm": "EC_prime256v1",
        "InUse": true,
        "Exported": false,
        "RenewalEligibility": "ELIGIBLE",
        "NotBefore": ISSUED,
        "NotAfter": EXPIRES,
        "CreatedAt": ISSUED,
        "IssuedAt": ISSUED,
        "ManagedBy": "CLOUDFRONT"
    });
    let mock = Mock::default().page(
        200,
        body(vec![full, summary("bare-2", "bare.example")], None),
    );
    let certs = mock.client().list_certificates().await.unwrap();
    let bare = &certs[0];
    assert_eq!(bare.domain_name, "bare.example");
    assert_eq!(bare.id, "bare-2");
    assert_eq!(bare.in_use, None);
    assert!(bare.status.is_empty() && bare.not_after.is_empty());
    let full = &certs[1];
    assert_eq!(full.arn, arn("full-1"));
    assert_eq!(full.id, "full-1");
    assert_eq!(full.additional_names, ["example.com", "www.example.com"]);
    assert!(full.more_names);
    assert_eq!(full.status, "ISSUED");
    assert_eq!(full.cert_type, "AMAZON_ISSUED");
    assert_eq!(full.key_algorithm, "EC_prime256v1");
    assert_eq!(full.in_use, Some(true));
    assert_eq!(full.exported, Some(false));
    assert_eq!(full.renewal_eligibility, "ELIGIBLE");
    assert_eq!(full.managed_by, "CLOUDFRONT");
    assert_eq!(full.not_before, "2019-12-16T23:48:18Z");
    assert_eq!(full.not_after, "2020-12-16T23:48:18Z");
    assert!(full.imported_at.is_empty() && full.revoked_at.is_empty());
}

#[tokio::test]
async fn requests_every_key_type_max_page_size_and_only_list_certificates() {
    let mock = Mock::default().page(200, body(vec![], None));
    assert!(mock.client().list_certificates().await.unwrap().is_empty());
    assert_eq!(mock.ops(), ["ListCertificates"]);
    let request = &mock.bodies()[0];
    assert_eq!(request["MaxItems"], 1000);
    let key_types: Vec<_> = request["Includes"]["keyTypes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    for wanted in ["RSA_2048", "RSA_4096", "EC_prime256v1", "EC_secp384r1"] {
        assert!(
            key_types.contains(&wanted),
            "{wanted} missing: {key_types:?}"
        );
    }
    // No status filter: issued, pending, expired and failed are all listed.
    assert!(request.get("CertificateStatuses").is_none());
}

#[tokio::test]
async fn follows_tokens_and_sorts_by_domain_then_arn() {
    let mock = Mock::default()
        .page(
            200,
            body(
                vec![summary("b", "same.example"), summary("z", "zeta.example")],
                Some("t2"),
            ),
        )
        .page(
            200,
            body(
                vec![summary("a", "same.example"), summary("m", "alpha.example")],
                None,
            ),
        );
    let certs = mock.client().list_certificates().await.unwrap();
    let ids: Vec<_> = certs.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["m", "a", "b", "z"]);
    let bodies = mock.bodies();
    assert!(bodies[0].get("NextToken").is_none());
    assert_eq!(bodies[1]["NextToken"], "t2");
}

#[tokio::test]
async fn repeated_token_is_an_error_not_a_partial_inventory() {
    let mock = Mock::default().page(200, body(vec![summary("a", "a.example")], Some("same")));
    let error = mock.client().list_certificates().await.unwrap_err();
    assert!(error.to_string().contains("repeated pagination token"));
}

#[tokio::test]
async fn denied_and_malformed_responses_surface_as_errors() {
    let denied = Mock::default().page(
        400,
        r#"{"__type":"AccessDeniedException","message":"not allowed"}"#,
    );
    let error = denied.client().list_certificates().await.unwrap_err();
    assert!(format!("{error:#}").contains("ListCertificates"));

    for entry in [
        serde_json::json!({"DomainName": "no-arn.example"}),
        serde_json::json!({"CertificateArn": "arn:aws:kms:us-east-1:123456789012:key/x"}),
    ] {
        let mock = Mock::default().page(200, body(vec![entry], None));
        assert!(mock.client().list_certificates().await.is_err());
    }
}

#[test]
fn certificate_id_accepts_only_acm_certificate_arns() {
    assert_eq!(certificate_id(&arn("1234-abcd")).unwrap(), "1234-abcd");
    assert_eq!(
        certificate_id("arn:aws-cn:acm:cn-north-1:123456789012:certificate/x").unwrap(),
        "x"
    );
    for bad in [
        "arn:aws:acm:us-east-1:123456789012:certificate/",
        "arn:aws:acm:us-east-1:123456789012:other/x",
        "arn:aws:acm::123456789012:certificate/x",
        "arn:aws:acm:us-east-1:12345:certificate/x",
        "arn:aws:acm:us-east-1:123456789012:certificate/a/b",
        "arn:aws:sns:us-east-1:123456789012:certificate/x",
        "certificate/x",
    ] {
        assert!(certificate_id(bad).is_err(), "{bad}");
    }
}
