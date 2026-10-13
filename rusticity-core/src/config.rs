use anyhow::Result;

/// ECR Public is a global service only available in us-east-1.
pub const ECR_PUBLIC_REGION: &str = "us-east-1";

#[derive(Clone, Debug)]
pub struct AwsConfig {
    pub region: String,
    pub account_id: String,
    pub role_arn: String,
    pub region_auto_detected: bool,
    /// Resolved SDK config — reused by all client factories to avoid re-running credential chain.
    sdk_config: aws_config::SdkConfig,
}

impl AwsConfig {
    pub async fn new(region: Option<String>) -> Result<Self> {
        Self::new_with_timeout(region, std::time::Duration::from_secs(10)).await
    }

    pub async fn new_with_timeout(
        region: Option<String>,
        timeout: std::time::Duration,
    ) -> Result<Self> {
        // Check for region early to avoid IMDS timeout
        if region.is_none()
            && std::env::var("AWS_REGION").is_err()
            && std::env::var("AWS_DEFAULT_REGION").is_err()
        {
            return Err(anyhow::anyhow!("Missing Region"));
        }

        let mut config_loader = aws_config::defaults(aws_config::BehaviorVersion::latest());

        // Use profile from AWS_PROFILE env var if set
        let profile_to_use = std::env::var("AWS_PROFILE").ok();
        if let Some(ref profile) = profile_to_use {
            if !profile.is_empty() {
                tracing::info!("Using AWS profile: {}", profile);
                config_loader = config_loader.profile_name(profile);
            }
        }

        if let Some(r) = &region {
            config_loader = config_loader.region(aws_config::Region::new(r.clone()));
        }

        // Load config once — stored and reused for all client factories.
        let sdk_config = tokio::time::timeout(timeout, config_loader.load())
            .await
            .map_err(|_| anyhow::anyhow!("Timeout loading AWS config"))?;

        // Double-check region is set
        if sdk_config.region().is_none() {
            return Err(anyhow::anyhow!("Missing Region"));
        }

        // Try to get identity with timeout
        let (account_id, role_arn) =
            match tokio::time::timeout(timeout, Self::try_get_identity(&sdk_config)).await {
                Ok(Ok((acc, role))) => {
                    tracing::info!("Loaded identity: account={}, role={}", acc, role);
                    (acc, role)
                }
                Ok(Err(e)) => {
                    tracing::error!("Failed to get identity: {}", e);
                    return Err(e);
                }
                Err(_) => return Err(anyhow::anyhow!("Timeout getting AWS identity")),
            };

        let (region_str, auto_detected) = match sdk_config.region() {
            Some(r) => (r.as_ref().to_string(), false),
            None => {
                let fastest = Self::find_fastest_region().await?;
                (fastest, true)
            }
        };

        Ok(Self {
            region: region_str,
            account_id,
            role_arn,
            region_auto_detected: auto_detected,
            sdk_config,
        })
    }

    async fn try_get_identity(config: &aws_config::SdkConfig) -> Result<(String, String)> {
        let sts_client = aws_sdk_sts::Client::new(config);
        let identity = sts_client.get_caller_identity().send().await?;
        let account_id = identity.account().unwrap_or("").to_string();
        let role_arn = identity.arn().unwrap_or("").to_string();
        Ok((account_id, role_arn))
    }

    async fn find_fastest_region() -> Result<String> {
        use std::time::Instant;

        let regions = [
            "us-east-1",
            "us-east-2",
            "us-west-1",
            "us-west-2",
            "af-south-1",
            "ap-east-1",
            "ap-south-1",
            "ap-south-2",
            "ap-northeast-1",
            "ap-northeast-2",
            "ap-northeast-3",
            "ap-southeast-1",
            "ap-southeast-2",
            "ap-southeast-3",
            "ap-southeast-4",
            "ca-central-1",
            "ca-west-1",
            "eu-central-1",
            "eu-central-2",
            "eu-west-1",
            "eu-west-2",
            "eu-west-3",
            "eu-north-1",
            "eu-south-1",
            "eu-south-2",
            "il-central-1",
            "me-central-1",
            "me-south-1",
            "sa-east-1",
        ];

        let mut tasks = Vec::new();

        for &region in &regions {
            let region_name = region.to_string();
            tasks.push(tokio::spawn(async move {
                let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                    .region(aws_config::Region::new(region_name.clone()))
                    .load()
                    .await;
                let s3 = aws_sdk_s3::Client::new(&config);
                let start = Instant::now();
                match tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    s3.list_buckets().send(),
                )
                .await
                {
                    Ok(Ok(_)) => Some((region_name, start.elapsed())),
                    _ => Some((region_name, std::time::Duration::from_secs(9999))),
                }
            }));
        }

        let results = futures::future::join_all(tasks).await;
        let mut latencies: Vec<(String, std::time::Duration)> = results
            .into_iter()
            .filter_map(|r| r.ok().flatten())
            .collect();

        latencies.sort_by_key(|(_, d)| *d);

        latencies
            .first()
            .map(|(r, _)| r.clone())
            .ok_or_else(|| anyhow::anyhow!("Could not determine fastest region"))
    }

    pub fn dummy(region: Option<String>) -> Self {
        // Build a minimal SdkConfig with just the region — no credential resolution.
        let region_str = region.unwrap_or_default();
        let sdk_config = aws_config::SdkConfig::builder()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region_str.clone()))
            .build();
        Self {
            region: region_str,
            account_id: "".to_string(),
            role_arn: "".to_string(),
            region_auto_detected: false,
            sdk_config,
        }
    }

    pub async fn get_account_for_profile(profile: &str, region: &str) -> Result<String> {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .profile_name(profile)
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        let sts_client = aws_sdk_sts::Client::new(&config);
        let identity = sts_client.get_caller_identity().send().await?;
        Ok(identity.account().unwrap_or("").to_string())
    }

    // ── Client factories ──────────────────────────────────────────────────────
    // All factories reuse self.sdk_config — credentials are already resolved,
    // so these are instantaneous (no network calls).

    pub fn s3_client(&self) -> aws_sdk_s3::Client {
        aws_sdk_s3::Client::new(&self.sdk_config)
    }

    pub async fn s3_client_with_region(&self, region: &str) -> aws_sdk_s3::Client {
        // Needs a different region — build a new config with the same credentials.
        let config = aws_config::SdkConfig::builder()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .credentials_provider(
                self.sdk_config
                    .credentials_provider()
                    .expect("credentials must be set")
                    .clone(),
            )
            .build();
        aws_sdk_s3::Client::new(&config)
    }

    pub fn cloudformation_client(&self) -> aws_sdk_cloudformation::Client {
        aws_sdk_cloudformation::Client::new(&self.sdk_config)
    }

    pub fn cloudtrail_client(&self) -> aws_sdk_cloudtrail::Client {
        aws_sdk_cloudtrail::Client::new(&self.sdk_config)
    }

    pub fn lambda_client(&self) -> aws_sdk_lambda::Client {
        aws_sdk_lambda::Client::new(&self.sdk_config)
    }

    pub fn iam_client(&self) -> aws_sdk_iam::Client {
        aws_sdk_iam::Client::new(&self.sdk_config)
    }

    pub fn ecr_client(&self) -> aws_sdk_ecr::Client {
        aws_sdk_ecr::Client::new(&self.sdk_config)
    }

    pub fn kms_client(&self) -> aws_sdk_kms::Client {
        aws_sdk_kms::Client::new(&self.sdk_config)
    }

    pub fn efs_client(&self) -> aws_sdk_efs::Client {
        aws_sdk_efs::Client::new(&self.sdk_config)
    }

    pub fn dynamodb_client(&self) -> aws_sdk_dynamodb::Client {
        aws_sdk_dynamodb::Client::new(&self.sdk_config)
    }

    pub fn rds_client(&self) -> aws_sdk_rds::Client {
        aws_sdk_rds::Client::new(&self.sdk_config)
    }

    pub fn fsx_client(&self) -> aws_sdk_fsx::Client {
        aws_sdk_fsx::Client::new(&self.sdk_config)
    }

    pub fn ecr_public_client(&self) -> aws_sdk_ecrpublic::Client {
        // ECR Public is a global service only available in us-east-1.
        // Reuse stored credentials but override region.
        let config = aws_config::SdkConfig::builder()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(ECR_PUBLIC_REGION))
            .credentials_provider(
                self.sdk_config
                    .credentials_provider()
                    .expect("credentials must be set")
                    .clone(),
            )
            .build();
        aws_sdk_ecrpublic::Client::new(&config)
    }

    pub fn cloudwatch_client(&self) -> aws_sdk_cloudwatch::Client {
        aws_sdk_cloudwatch::Client::new(&self.sdk_config)
    }

    pub fn sqs_client(&self) -> aws_sdk_sqs::Client {
        aws_sdk_sqs::Client::new(&self.sdk_config)
    }

    pub fn cloudwatch_logs_client(&self) -> aws_sdk_cloudwatchlogs::Client {
        aws_sdk_cloudwatchlogs::Client::new(&self.sdk_config)
    }

    pub fn pipes_client(&self) -> aws_sdk_pipes::Client {
        aws_sdk_pipes::Client::new(&self.sdk_config)
    }

    pub fn ec2_client(&self) -> aws_sdk_ec2::Client {
        aws_sdk_ec2::Client::new(&self.sdk_config)
    }

    pub fn apigateway_client(&self) -> aws_sdk_apigateway::Client {
        aws_sdk_apigateway::Client::new(&self.sdk_config)
    }

    pub fn apigatewayv2_client(&self) -> aws_sdk_apigatewayv2::Client {
        aws_sdk_apigatewayv2::Client::new(&self.sdk_config)
    }

    pub fn alarms_client(&self) -> aws_sdk_cloudwatch::Client {
        aws_sdk_cloudwatch::Client::new(&self.sdk_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dummy_config_with_region() {
        let region = "us-west-2";
        let config = AwsConfig::dummy(Some(region.to_string()));
        assert_eq!(config.region, region);
        assert_eq!(config.account_id, "");
        assert!(!config.region_auto_detected);
    }

    #[test]
    fn test_dummy_config_without_region() {
        let config = AwsConfig::dummy(None);
        assert_eq!(config.region, "");
        assert_eq!(config.account_id, "");
        assert!(!config.region_auto_detected);
    }

    #[tokio::test]
    async fn test_new_fails_without_credentials() {
        // Clear AWS env vars to simulate no credentials
        std::env::remove_var("AWS_ACCESS_KEY_ID");
        std::env::remove_var("AWS_SECRET_ACCESS_KEY");
        std::env::remove_var("AWS_SESSION_TOKEN");
        std::env::set_var("AWS_PROFILE", "nonexistent-profile-test");

        let result = AwsConfig::new(Some("us-east-1".to_string())).await;

        // Should fail with credentials error before attempting region detection
        assert!(result.is_err());
        let err_str = format!("{}", result.unwrap_err());
        // Can be credentials error or dispatch failure (both indicate auth issues)
        assert!(
            err_str.contains("credentials")
                || err_str.contains("profile")
                || err_str.contains("dispatch"),
            "Expected auth error, got: {}",
            err_str
        );
    }

    #[tokio::test]
    async fn test_timeout_is_configurable() {
        std::env::remove_var("AWS_ACCESS_KEY_ID");
        std::env::remove_var("AWS_SECRET_ACCESS_KEY");
        std::env::set_var("AWS_PROFILE", "nonexistent-profile-test");

        // Test with very short timeout
        let result = AwsConfig::new_with_timeout(
            Some("us-east-1".to_string()),
            std::time::Duration::from_millis(100),
        )
        .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_dummy_config_preserves_values() {
        let region = "eu-west-1";
        let config = AwsConfig::dummy(Some(region.to_string()));
        assert_eq!(config.region, region);
        assert_eq!(config.account_id, "");
        assert_eq!(config.role_arn, "");
        assert!(!config.region_auto_detected);
    }

    #[test]
    fn test_ecr_public_region_is_us_east_1() {
        // ECR Public is a global service — must always use us-east-1 regardless of user region.
        assert_eq!(ECR_PUBLIC_REGION, "us-east-1");
    }

    #[test]
    fn test_client_factories_are_sync() {
        // All client factories (except s3_client_with_region) must be sync —
        // they should not do any async work after credentials are resolved.
        let config = AwsConfig::dummy(Some("us-east-1".to_string()));
        let _ = config.s3_client();
        let _ = config.ecr_client();
        let _ = config.cloudwatch_client();
        let _ = config.lambda_client();
        let _ = config.iam_client();
        let _ = config.sqs_client();
        let _ = config.ec2_client();
    }
}

#[cfg(test)]
mod pagination_tests {
    use super::*;
    use crate::dynamodb::DynamoDbClient;
    use crate::rds::{InventoryOptions, RdsClient};
    use aws_smithy_http_client::test_util::{ReplayEvent, StaticReplayClient};
    use aws_smithy_types::body::SdkBody;

    fn replay(uri: &str, request: String, status: u16, response: String) -> ReplayEvent {
        ReplayEvent::new(
            http::Request::builder()
                .method("POST")
                .uri(uri)
                .body(SdkBody::from(request))
                .unwrap(),
            http::Response::builder()
                .status(status)
                .body(SdkBody::from(response))
                .unwrap(),
        )
    }

    fn config(http_client: &StaticReplayClient) -> AwsConfig {
        let sdk_config = aws_config::SdkConfig::builder()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_sdk_iam::config::SharedCredentialsProvider::new(
                aws_sdk_iam::config::Credentials::new(
                    "test-key",
                    "test-secret",
                    None,
                    None,
                    "test",
                ),
            ))
            .retry_config(aws_config::retry::RetryConfig::disabled())
            .http_client(http_client.clone())
            .build();
        AwsConfig {
            sdk_config,
            ..AwsConfig::dummy(Some("us-east-1".to_string()))
        }
    }

    #[tokio::test]
    async fn rds_optional_reads_are_gated_and_fail_without_hiding_inventory() {
        let uri = "https://rds.us-east-1.amazonaws.com/";
        let inventory = "<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances><DBInstance>\
            <DBInstanceIdentifier>orders</DBInstanceIdentifier><DBInstanceStatus>available</DBInstanceStatus>\
            <DBInstanceArn>arn:aws:rds:us-east-1:123456789012:db:orders</DBInstanceArn>\
            </DBInstance></DBInstances></DescribeDBInstancesResult></DescribeDBInstancesResponse>";
        let denied = "<ErrorResponse><Error><Type>Sender</Type><Code>AccessDenied</Code><Message>denied</Message></Error></ErrorResponse>";
        for enabled in [false, true] {
            let mut events = vec![replay(
                uri,
                "Action=DescribeDBInstances&Version=2014-10-31".into(),
                200,
                inventory.into(),
            )];
            if enabled {
                events.push(replay(
                    uri,
                    "Action=DescribeDBRecommendations&Version=2014-10-31".into(),
                    403,
                    denied.into(),
                ));
                events.push(replay(
                    uri,
                    "Action=DescribePendingMaintenanceActions&Version=2014-10-31".into(),
                    403,
                    denied.into(),
                ));
                // CloudWatch uses a different wire protocol; an HTTP failure is enough
                // to verify the optional metric error path, not its serialization.
                events.push(replay(
                    "https://monitoring.us-east-1.amazonaws.com/",
                    String::new(),
                    403,
                    String::new(),
                ));
            }
            let http = StaticReplayClient::new(events);
            let rows = RdsClient::new(config(&http))
                .list_instances_with_options(InventoryOptions {
                    recommendations: enabled,
                    maintenance: enabled,
                    cpu: enabled,
                    current_activity: enabled,
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].identifier, "orders");
            assert_eq!(rows[0].status, "available");
            let expected = if enabled { "Unavailable" } else { "" };
            assert_eq!(rows[0].recommendations, expected);
            assert_eq!(rows[0].maintenance, expected);
            assert_eq!(rows[0].cpu, expected);
            assert_eq!(rows[0].current_activity, expected);
            assert_eq!(http.actual_requests().count(), if enabled { 4 } else { 1 });
            if !enabled {
                http.assert_requests_match(&[]);
            }
        }
    }

    #[tokio::test]
    async fn rds_inventory_follows_markers_maps_fields_and_sorts_identifiers() {
        let uri = "https://rds.us-east-1.amazonaws.com/";
        let body = "Action=DescribeDBInstances&Version=2014-10-31";
        let http = StaticReplayClient::new(vec![
            replay(uri, body.into(), 200,
                "<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances><DBInstance>\
                <DBInstanceIdentifier>zeta</DBInstanceIdentifier><DBInstanceStatus>available</DBInstanceStatus>\
                <Engine>postgres</Engine><EngineVersion>17.6</EngineVersion><DBInstanceClass>db.t4g.micro</DBInstanceClass>\
                <AvailabilityZone>us-east-1a</AvailabilityZone><DBInstanceArn>arn:aws:rds:us-east-1:123456789012:db:zeta</DBInstanceArn>\
                <Endpoint><Address>zeta.example.invalid</Address><Port>5432</Port></Endpoint>\
                </DBInstance></DBInstances><Marker>page2</Marker></DescribeDBInstancesResult></DescribeDBInstancesResponse>".into()),
            replay(uri, format!("{body}&Marker=page2"), 200,
                "<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances><DBInstance>\
                <DBInstanceIdentifier>alpha</DBInstanceIdentifier><DBInstanceStatus>creating</DBInstanceStatus>\
                </DBInstance></DBInstances></DescribeDBInstancesResult></DescribeDBInstancesResponse>".into()),
        ]);
        let rows = RdsClient::new(config(&http))
            .list_instances()
            .await
            .unwrap();
        assert_eq!(
            rows.iter()
                .map(|i| i.identifier.as_str())
                .collect::<Vec<_>>(),
            ["alpha", "zeta"]
        );
        assert!(rows[0].endpoint.is_empty());
        assert_eq!(rows[1].endpoint, "zeta.example.invalid");
        assert_eq!(rows[1].port, Some(5432));
        assert_eq!(rows[1].engine_version, "17.6");
        assert_eq!(rows[1].region, "us-east-1");
        http.assert_requests_match(&[]);
    }

    #[tokio::test]
    async fn rds_inventory_empty_and_repeated_markers_terminate() {
        let uri = "https://rds.us-east-1.amazonaws.com/";
        let body = "Action=DescribeDBInstances&Version=2014-10-31";
        for repeat in [false, true] {
            let response = format!(
                "<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances/>{}\
                </DescribeDBInstancesResult></DescribeDBInstancesResponse>",
                if repeat { "<Marker>same</Marker>" } else { "" }
            );
            let mut events = vec![replay(uri, body.into(), 200, response.clone())];
            if repeat {
                events.push(replay(uri, format!("{body}&Marker=same"), 200, response));
            }
            let http = StaticReplayClient::new(events);
            let rows = RdsClient::new(config(&http))
                .list_instances()
                .await
                .unwrap();
            assert!(rows.is_empty());
            http.assert_requests_match(&[]);
        }
    }

    #[tokio::test]
    async fn rds_inventory_propagates_later_page_failure_without_partial_success() {
        let uri = "https://rds.us-east-1.amazonaws.com/";
        let body = "Action=DescribeDBInstances&Version=2014-10-31";
        let http = StaticReplayClient::new(vec![
            replay(uri, body.into(), 200,
                "<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances><DBInstance>\
                <DBInstanceIdentifier>first</DBInstanceIdentifier></DBInstance></DBInstances><Marker>page2</Marker>\
                </DescribeDBInstancesResult></DescribeDBInstancesResponse>".into()),
            replay(uri, format!("{body}&Marker=page2"), 403,
                "<ErrorResponse><Error><Type>Sender</Type><Code>AccessDenied</Code><Message>denied</Message></Error></ErrorResponse>".into()),
        ]);
        assert!(RdsClient::new(config(&http))
            .list_instances()
            .await
            .is_err());
        http.assert_requests_match(&[]);
    }

    #[tokio::test]
    async fn dynamodb_scan_preserves_typed_cursor_and_requests_one_page_at_a_time() {
        let uri = "https://dynamodb.us-east-1.amazonaws.com/";
        let key = serde_json::json!({
            "pk": {"S": "customer"},
            "sk": {"N": "123456789012345678901234567890"},
            "binary": {"B": "AAEC/w=="},
        });
        let request = serde_json::json!({
            "TableName": "orders", "IndexName": "by-customer", "Limit": 100,
            "ProjectionExpression": "#p0, #p1",
            "ExpressionAttributeNames": {"#p0": "pk", "#p1": "sk"},
        });
        let mut next_request = request.clone();
        next_request["ExclusiveStartKey"] = key.clone();
        let http = StaticReplayClient::new(vec![
            replay(
                uri,
                request.to_string(),
                200,
                serde_json::json!({
                    "Items": [{"pk": {"S": "first"}}], "LastEvaluatedKey": key,
                })
                .to_string(),
            ),
            replay(
                uri,
                next_request.to_string(),
                200,
                r#"{"Items": [], "LastEvaluatedKey": {}}"#.into(),
            ),
        ]);
        let client = DynamoDbClient::new(config(&http));
        let projection = vec!["pk".into(), "sk".into()];
        let page = client
            .scan_items("orders", Some("by-customer"), 100, Some(&projection), None)
            .await
            .unwrap();
        assert_eq!(http.actual_requests().count(), 1);
        assert_eq!(page.items[0].get("pk"), "first");
        let cursor = page.next_cursor.unwrap();
        assert_eq!(
            cursor["sk"].as_n().unwrap(),
            "123456789012345678901234567890"
        );
        assert_eq!(cursor["binary"].as_b().unwrap().as_ref(), &[0, 1, 2, 255]);
        let page = client
            .scan_items(
                "orders",
                Some("by-customer"),
                100,
                Some(&projection),
                Some(cursor),
            )
            .await
            .unwrap();
        assert!(page.items.is_empty());
        assert!(page.next_cursor.is_none());
        http.assert_requests_match(&[]);
    }

    #[tokio::test]
    async fn dynamodb_query_continues_after_an_empty_page_without_changing_conditions() {
        let uri = "https://dynamodb.us-east-1.amazonaws.com/";
        let key = serde_json::json!({"customer_id": {"N": "42"}, "sk": {"S": "last"}});
        let request = serde_json::json!({
            "TableName": "orders", "IndexName": "by-customer", "Limit": 100,
            "KeyConditionExpression": "#pk = :pk",
            "ExpressionAttributeNames": {"#pk": "customer_id", "#p0": "customer_id", "#p1": "sk"},
            "ExpressionAttributeValues": {":pk": {"N": "42"}},
            "ProjectionExpression": "#p0, #p1",
        });
        let mut next_request = request.clone();
        next_request["ExclusiveStartKey"] = key.clone();
        let http = StaticReplayClient::new(vec![
            replay(
                uri,
                request.to_string(),
                200,
                serde_json::json!({"Items": [], "LastEvaluatedKey": key}).to_string(),
            ),
            replay(
                uri,
                next_request.to_string(),
                200,
                r#"{"Items": [{"customer_id": {"N": "42"}, "sk": {"S": "next"}}]}"#.into(),
            ),
        ]);
        let client = DynamoDbClient::new(config(&http));
        let projection = vec!["customer_id".into(), "sk".into()];
        let page = client
            .query_items(
                "orders",
                Some("by-customer"),
                "customer_id",
                "42",
                "N",
                100,
                Some(&projection),
                None,
            )
            .await
            .unwrap();
        assert!(page.items.is_empty());
        assert!(page.next_cursor.is_some());
        assert_eq!(http.actual_requests().count(), 1);
        let page = client
            .query_items(
                "orders",
                Some("by-customer"),
                "customer_id",
                "42",
                "N",
                100,
                Some(&projection),
                page.next_cursor,
            )
            .await
            .unwrap();
        assert_eq!(page.items[0].get("sk"), "next");
        assert!(page.next_cursor.is_none());
        http.assert_requests_match(&[]);
    }

    #[tokio::test]
    async fn iam_paginators_follow_markers_and_collect_items() {
        for (operation, collection, name, id) in [
            ("ListUsers", "Users", "UserName", "UserId"),
            ("ListRoles", "Roles", "RoleName", "RoleId"),
            ("ListGroups", "Groups", "GroupName", "GroupId"),
        ] {
            let page = |item: &str, more: &str| {
                format!(
                    "<{operation}Response><{operation}Result><{collection}>\
                 <member><Path>/</Path><{name}>{item}</{name}><{id}>{item}</{id}>\
                 <Arn>arn:aws:iam::123456789012:test/{item}</Arn>\
                 <CreateDate>2026-01-01T00:00:00Z</CreateDate></member>\
                 </{collection}>{more}</{operation}Result></{operation}Response>"
                )
            };
            let body = format!("Action={operation}&Version=2010-05-08");
            let http = StaticReplayClient::new(vec![
                replay(
                    "https://iam.amazonaws.com/",
                    body.clone(),
                    200,
                    page(
                        "first",
                        "<IsTruncated>true</IsTruncated><Marker>page2</Marker>",
                    ),
                ),
                replay(
                    "https://iam.amazonaws.com/",
                    format!("{body}&Marker=page2"),
                    200,
                    page("second", "<IsTruncated>false</IsTruncated>"),
                ),
            ]);
            let client = crate::iam::IamClient::new(config(&http));
            let names: Vec<String> = match operation {
                "ListUsers" => client
                    .list_users()
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|u| u.user_name)
                    .collect(),
                "ListRoles" => client
                    .list_roles()
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|r| r.role_name)
                    .collect(),
                _ => client
                    .list_groups()
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|g| g.group_name)
                    .collect(),
            };
            assert_eq!(names, ["first", "second"]);
            http.assert_requests_match(&[]);
        }
    }

    #[tokio::test]
    async fn iam_paginator_stops_on_duplicate_marker_even_for_empty_pages() {
        let response = "<ListUsersResponse><ListUsersResult><Users/>\
            <IsTruncated>true</IsTruncated><Marker>same</Marker>\
            </ListUsersResult></ListUsersResponse>";
        let body = "Action=ListUsers&Version=2010-05-08";
        let http = StaticReplayClient::new(vec![
            replay(
                "https://iam.amazonaws.com/",
                body.into(),
                200,
                response.into(),
            ),
            replay(
                "https://iam.amazonaws.com/",
                format!("{body}&Marker=same"),
                200,
                response.into(),
            ),
        ]);
        let client = crate::iam::IamClient::new(config(&http));
        assert!(client.list_users().await.unwrap().is_empty());
        http.assert_requests_match(&[]);
    }

    #[tokio::test]
    async fn iam_paginator_propagates_errors() {
        let http = StaticReplayClient::new(vec![replay(
            "https://iam.amazonaws.com/",
            "Action=ListUsers&Version=2010-05-08".into(),
            403,
            "<ErrorResponse><Error><Type>Sender</Type><Code>AccessDenied</Code>\
             <Message>denied</Message></Error></ErrorResponse>"
                .into(),
        )]);
        let client = crate::iam::IamClient::new(config(&http));
        assert!(client
            .list_users()
            .await
            .unwrap_err()
            .starts_with("Failed to list users:"));
        http.assert_requests_match(&[]);
    }

    #[tokio::test]
    async fn log_stream_paginator_keeps_order_and_stops_at_existing_limit() {
        let page = |start: usize, token: &str| {
            serde_json::json!({
                "logStreams": (start..start + 50).map(|i| serde_json::json!({
                    "logStreamName": format!("stream-{i}"), "creationTime": 0,
                    "lastEventTimestamp": 1000
                })).collect::<Vec<_>>(),
                "nextToken": token,
            })
            .to_string()
        };
        let uri = "https://logs.us-east-1.amazonaws.com/";
        let http = StaticReplayClient::new(vec![
            replay(uri, r#"{"logGroupName":"group","orderBy":"LastEventTime","descending":true}"#.into(), 200, page(0, "page2")),
            replay(uri, r#"{"logGroupName":"group","orderBy":"LastEventTime","descending":true,"nextToken":"page2"}"#.into(), 200, page(50, "page3")),
        ]);
        let client = crate::cw::CloudWatchClient::new(config(&http))
            .await
            .unwrap();
        let streams = client.list_log_streams("group").await.unwrap();
        assert_eq!(streams.len(), 100);
        assert_eq!(streams[0].name, "stream-0");
        assert_eq!(streams[99].name, "stream-99");
        assert_eq!(streams[0].last_event_time.unwrap().timestamp(), 1);
        // A third request would fail the replay, even though page2 has a token.
        http.assert_requests_match(&[]);
    }
}
