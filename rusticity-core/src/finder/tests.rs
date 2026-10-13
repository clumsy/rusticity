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
        "\"orders\"* \"service:iam\"* resourcetype:dynamodb:table region:us-east-2,global"
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
