use super::*;

#[test]
fn finder_query_threshold_excludes_service_qualifiers_and_supports_unicode() {
    for input in ["", "@dynamodb", "@s3 ab", " a b "] {
        assert!(!SearchQuery::parse(input).unwrap().ready(), "{input}");
    }
    let query = SearchQuery::parse("@DDB abc").unwrap();
    assert!(query.ready());
    assert_eq!(query.kinds, [ResourceKind::Table]);
    assert!(SearchQuery::parse("@rds 数据库").unwrap().ready());
    assert!(SearchQuery::parse("@unknown abc").is_err());
    assert!(SearchQuery::parse("@ abc").is_err());
    let union = SearchQuery::parse("rusticity @s3 @dynamodb").unwrap();
    assert_eq!(union.text, "rusticity");
    assert_eq!(union.kinds.len(), 2);
    assert!(union.kinds.contains(&ResourceKind::Bucket));
}

#[test]
fn finder_index_query_quotes_input_and_limits_resource_types_and_region() {
    let query = SearchQuery::parse("@dynamodb orders service:iam")
        .unwrap()
        .index_query("us-east-2");
    assert_eq!(
        query,
        "orders* \"service:iam\" resourcetype:dynamodb:table region:us-east-2,global"
    );
    assert!(SearchQuery::parse("@apig test")
        .unwrap()
        .index_query("us-east-2")
        .contains("apigateway:restapis,apigateway:apis"));
}

#[test]
fn finder_normalizes_only_supported_openable_resource_types() {
    for (kind, typ, arn, id) in [
        (
            ResourceKind::Table,
            "dynamodb:table",
            "arn:aws:dynamodb:us-east-2:123456789012:table/orders",
            "orders",
        ),
        (
            ResourceKind::Bucket,
            "s3:bucket",
            "arn:aws:s3:::bucket",
            "bucket",
        ),
        (
            ResourceKind::Role,
            "iam:role",
            "arn:aws:iam::123456789012:role/path/role",
            "role",
        ),
        (
            ResourceKind::LogGroup,
            "logs:log-group",
            "arn:aws:logs:us-east-2:123456789012:log-group:/aws/lambda/fn:*",
            "/aws/lambda/fn",
        ),
        (
            ResourceKind::Database,
            "rds:db",
            "arn:aws:rds:us-east-2:123456789012:db:orders",
            "orders",
        ),
        (
            ResourceKind::Api,
            "apigateway:restapis",
            "arn:aws:apigateway:us-east-2::/restapis/abc",
            "abc",
        ),
        (
            ResourceKind::Stack,
            "cloudformation:stack",
            "arn:aws:cloudformation:us-east-2:123456789012:stack/orders/uuid",
            "orders",
        ),
        (
            ResourceKind::Repository,
            "ecr:repository",
            "arn:aws:ecr:us-east-2:123456789012:repository/path/repo",
            "path/repo",
        ),
    ] {
        let resource = Resource::from_index(typ, arn, "us-east-2").unwrap();
        assert_eq!(resource.kind, kind);
        assert_eq!(resource.id, id);
        assert_eq!(resource.arn, arn);
    }
    assert!(Resource::from_index(
        "rds:cluster",
        "arn:aws:rds:us-east-2:123456789012:cluster:c",
        "us-east-2"
    )
    .is_none());
    assert!(Resource::from_index(
        "lambda:function",
        "arn:aws:lambda:us-east-2:123456789012:function:fn:version",
        "us-east-2"
    )
    .is_none());
    assert!(Resource::from_index("s3:bucket", "bad-arn", "global").is_none());
}

#[tokio::test]
async fn finder_cached_fallback_streams_matching_sources_without_aws_calls() {
    let cache = Arc::new(Mutex::new(SearchCache::default()));
    {
        let mut cache = cache.lock().await;
        cache.index_unavailable = Some(Instant::now());
        cache.inventory.insert(
            ResourceKind::Table,
            (
                Instant::now(),
                vec![
                    Resource::new(
                        ResourceKind::Table,
                        "rusticity-orders",
                        "Orders",
                        "us-east-2",
                    ),
                    Resource::new(ResourceKind::Table, "unrelated", "Other", "us-east-2"),
                ],
                false,
            ),
        );
        cache.inventory.insert(
            ResourceKind::Bucket,
            (
                Instant::now(),
                vec![Resource::new(
                    ResourceKind::Bucket,
                    "rusticity-bucket",
                    "Bucket",
                    "global",
                )],
                true,
            ),
        );
    }
    // Dummy config has no credentials; cached inventories must not access AWS.
    let (tx, mut rx) = mpsc::channel(8);
    search(
        AwsConfig::dummy(Some("us-east-2".into())),
        SearchQuery::parse("@s3 @ddb rusticity").unwrap(),
        cache,
        tx,
    )
    .await;
    let mut rows = Vec::new();
    let mut finished = false;
    while let Some(update) = rx.recv().await {
        match update {
            SearchUpdate::Results(batch) => rows.extend(batch),
            SearchUpdate::Finished { limited } => {
                finished = true;
                assert!(limited);
            }
            SearchUpdate::Warning(warning) => panic!("Unexpected AWS read: {warning}"),
            _ => {}
        }
    }
    assert!(finished);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.id.starts_with("rusticity")));
}

#[tokio::test]
async fn finder_short_queries_do_not_send_requests() {
    let (tx, mut rx) = mpsc::channel(1);
    search(
        AwsConfig::dummy(None),
        SearchQuery::parse("@s3 ab").unwrap(),
        Arc::new(Mutex::new(SearchCache::default())),
        tx,
    )
    .await;
    assert!(rx.recv().await.is_none());
}

#[test]
fn finder_vpc_qualifier_and_ec2_resource_type_are_supported() {
    let query = SearchQuery::parse("@vpc network").unwrap();
    assert_eq!(query.kinds, [ResourceKind::Vpc]);
    assert!(query
        .index_query("us-east-2")
        .contains("resourcetype:ec2:vpc"));
    let ec2 = SearchQuery::parse("@ec2 vpc").unwrap();
    assert!(ec2.kinds.contains(&ResourceKind::Vpc));
    assert!(ec2.kinds.contains(&ResourceKind::Instance));
    let resource = Resource::from_index(
        "ec2:vpc",
        "arn:aws:ec2:us-east-2:123456789012:vpc/vpc-test",
        "us-east-2",
    )
    .unwrap();
    assert_eq!(resource.kind, ResourceKind::Vpc);
    assert_eq!(resource.kind.service(), "vpc");
    assert_eq!(resource.id, "vpc-test");
}

#[test]
fn finder_sns_qualifier_and_topic_resource_type_are_supported() {
    let query = SearchQuery::parse("@sns alerts").unwrap();
    assert_eq!(query.kinds, [ResourceKind::Topic]);
    assert!(query
        .index_query("us-east-2")
        .contains("resourcetype:sns:topic"));
    let resource = Resource::from_index(
        "sns:topic",
        "arn:aws:sns:us-east-2:123456789012:my-topic",
        "us-east-2",
    )
    .unwrap();
    assert_eq!(resource.kind, ResourceKind::Topic);
    assert_eq!(resource.kind.service(), "sns");
    assert_eq!(resource.id, "my-topic");
    assert_eq!(resource.arn, "arn:aws:sns:us-east-2:123456789012:my-topic");
}

#[test]
fn finder_prefix_terms_use_resource_explorer_wildcards_without_quotes() {
    for (input, prefix) in [
        ("@lambda Fsx", "fsx*"),
        (
            "@lambda BETA-SagemakerHyperpodEks-FsxDraKmsGrant",
            "beta-sagemakerhyperpodeks-fsxdrakmsgrant*",
        ),
        ("@lambda my_function.v2", "my_function.v2*"),
        ("@lambda fsx grant", "fsx* grant*"),
    ] {
        assert_eq!(
            SearchQuery::parse(input).unwrap().index_query("us-east-1"),
            format!("{prefix} resourcetype:lambda:function region:us-east-1,global")
        );
    }
}

#[test]
fn finder_syntax_bearing_terms_stay_literal_without_standalone_wildcards() {
    for (input, literal) in [
        ("service:iam", "\"service:iam\""),
        ("region:us-west-2", "\"region:us-west-2\""),
        ("-orders", "\"-orders\""),
        ("fsx*", "\"fsx*\""),
        ("name,other", "\"name,other\""),
        ("name\"quoted", "\"name\\\"quoted\""),
        ("name\\path", "\"name\\\\path\""),
        ("(orders)", "\"(orders)\""),
    ] {
        let query = SearchQuery::parse(&format!("@lambda {input}")).unwrap();
        assert_eq!(
            query.index_query("us-east-1"),
            format!("{literal} resourcetype:lambda:function region:us-east-1,global")
        );
    }
}

#[test]
fn finder_sns_topic_arns_are_openable_but_subscription_arns_remain_references() {
    let arn = "arn:aws:sns:us-east-1:123456789012:orders.fifo";
    let topic = Resource::from_arn(arn).unwrap();
    assert_eq!(topic.kind, ResourceKind::Topic);
    assert_eq!(topic.id, "orders.fifo");
    assert_eq!(topic.arn, arn);
    let subscription = Resource::from_arn(&format!("{arn}:subscription-id")).unwrap();
    assert_eq!(subscription.kind, ResourceKind::Reference);
}

#[test]
fn finder_secret_qualifiers_and_resource_type_are_supported() {
    for q in ["@secretsmanager", "@secrets", "@sm"] {
        let query = SearchQuery::parse(&format!("{q} db")).unwrap();
        assert_eq!(query.kinds, [ResourceKind::Secret], "{q}");
        assert!(query
            .index_query("us-east-2")
            .contains("resourcetype:secretsmanager:secret"));
    }
}

#[test]
fn finder_secret_index_id_is_full_arn_and_name_strips_only_secret_prefix() {
    // Same-named secrets differ only by the random suffix, so the id must be
    // the full ARN; the display name keeps the suffix (never a last-6 strip).
    let arn = "arn:aws:secretsmanager:us-east-2:123456789012:secret:prod/db-AbCdEf";
    let resource = Resource::from_index("secretsmanager:secret", arn, "us-east-2").unwrap();
    assert_eq!(resource.kind, ResourceKind::Secret);
    assert_eq!(resource.kind.service(), "secretsmanager");
    assert_eq!(resource.id, arn);
    assert_eq!(resource.name, "prod/db-AbCdEf");
    assert_eq!(resource.arn, arn);
}

#[test]
fn finder_secret_arns_are_openable_with_arn_as_id() {
    let arn = "arn:aws:secretsmanager:us-east-1:123456789012:secret:prod/db-AbCdEf";
    let secret = Resource::from_arn(arn).unwrap();
    assert_eq!(secret.kind, ResourceKind::Secret);
    assert_eq!(secret.id, arn);
    assert_eq!(secret.name, "prod/db-AbCdEf");
    assert_eq!(secret.arn, arn);
}

#[tokio::test]
async fn secret_finder_stops_at_inventory_cap_and_never_fetches_values() {
    use aws_smithy_http_client::test_util::infallible_client_fn;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    let count = Arc::new(AtomicUsize::new(0));
    let requests = count.clone();
    let sdk=aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-1"))
        .credentials_provider(aws_sdk_secretsmanager::config::SharedCredentialsProvider::new(aws_sdk_secretsmanager::config::Credentials::new("test","test",None,None,"test")))
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(infallible_client_fn(move|request|{
            assert!(request.headers()["x-amz-target"].to_str().unwrap().ends_with(".ListSecrets"));
            let body:serde_json::Value=serde_json::from_slice(request.body().bytes().unwrap()).unwrap();
            assert_eq!(body["MaxResults"],100);
            let page=requests.fetch_add(1,Ordering::SeqCst);
            if page>0{assert_eq!(body["NextToken"],format!("page-{page}"));}
            let rows:Vec<_>=(0..100).map(|i|serde_json::json!({"Name":format!("secret-{page}-{i}"),"ARN":format!("arn:aws:secretsmanager:us-east-1:123456789012:secret:secret-{page}-{i}-AbCdEf")})).collect();
            http::Response::builder().status(200).body(serde_json::json!({"SecretList":rows,"NextToken":format!("page-{}",page+1)}).to_string().into_bytes()).unwrap()
        })).build();
    let config = AwsConfig::from_sdk_config(sdk, "us-east-1", "123456789012");
    let (rows, limited) = inventory::fetch(&config, ResourceKind::Secret)
        .await
        .unwrap();
    assert!(limited);
    assert_eq!(rows.len(), 2000);
    assert_eq!(count.load(Ordering::SeqCst), 20);
    assert_eq!(rows[0].id, rows[0].arn);
    assert_eq!(rows[0].name, "secret-0-0");
}
