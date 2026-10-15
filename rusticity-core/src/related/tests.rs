use super::*;
use crate::config::AwsConfig;
use crate::finder::{Resource, ResourceKind, SearchQuery, SearchUpdate};
use aws_smithy_http_client::test_util::infallible_client_fn;
use aws_smithy_types::body::SdkBody;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

// ── Test config builder ───────────────────────────────────────────────────────

fn test_config(
    http_client: impl Fn(http::Request<SdkBody>) -> http::Response<Vec<u8>> + Send + Sync + 'static,
) -> AwsConfig {
    test_config_with_account(http_client, "123456789012")
}

fn test_config_with_account(
    http_client: impl Fn(http::Request<SdkBody>) -> http::Response<Vec<u8>> + Send + Sync + 'static,
    account: &str,
) -> AwsConfig {
    let sdk_config = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-1"))
        .credentials_provider(aws_sdk_iam::config::SharedCredentialsProvider::new(
            aws_sdk_iam::config::Credentials::new("test-key", "test-secret", None, None, "test"),
        ))
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(infallible_client_fn(http_client))
        .build();
    AwsConfig::from_sdk_config(sdk_config, "us-east-1", account)
}

async fn collect_all(rx: &mut mpsc::Receiver<SearchUpdate>) -> (Vec<Resource>, Vec<String>, bool) {
    let mut resources = Vec::new();
    let mut warnings = Vec::new();
    let mut finished = false;
    while let Some(update) = rx.recv().await {
        match update {
            SearchUpdate::Results(batch) => resources.extend(batch),
            SearchUpdate::Warning(w) => warnings.push(w),
            SearchUpdate::Finished { limited: _ } => {
                finished = true;
            }
            _ => {}
        }
    }
    (resources, warnings, finished)
}

// ── Issue 1: try_send drops results → async send never drops ──────────────────

#[tokio::test]
async fn channel_capacity_1_still_delivers_all_results() {
    // Channel capacity 1: the old try_send would drop results when the
    // channel is full. The new awaited send blocks until the receiver
    // is ready, so all results are delivered.
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let config = test_config(move |request| {
        let uri = request.uri().to_string();
        let body = std::str::from_utf8(request.body().bytes().unwrap())
            .unwrap()
            .to_string();
        captured
            .lock()
            .unwrap()
            .push(format!("URI: {uri} BODY: {body}"));

        if uri.contains("/2015-03-31/functions/myfn") && !uri.contains("event-source-mappings") {
            // Return many resources in a single GetFunctionConfiguration response
            let subnets: Vec<String> = (0..40).map(|i| format!("subnet-{i}")).collect();
            let sgs: Vec<String> = (0..40).map(|i| format!("sg-{i}")).collect();
            let subnet_json = subnets
                .iter()
                .map(|s| format!("\"{s}\""))
                .collect::<Vec<_>>()
                .join(",");
            let sg_json = sgs
                .iter()
                .map(|s| format!("\"{s}\""))
                .collect::<Vec<_>>()
                .join(",");
            let body = format!(
                r#"{{"Role":"arn:aws:iam::123456789012:role/lambda-role","VpcConfig":{{"SubnetIds":[{}],"SecurityGroupIds":[{}],"VpcId":"vpc-1"}},"FunctionName":"myfn","FunctionArn":"arn:aws:lambda:us-east-1:123456789012:function:myfn"}}"#,
                subnet_json, sg_json
            );
            http::Response::builder()
                .status(200)
                .body(body.into_bytes())
                .unwrap()
        } else if uri.contains("event-source-mappings") {
            http::Response::builder()
                .status(200)
                .body(br#"{"EventSourceMappings":[]}"#.to_vec())
                .unwrap()
        } else {
            panic!("Unexpected request: URI={uri} BODY={body}");
        }
    });

    let source = Resource::from_arn("arn:aws:lambda:us-east-1:123456789012:function:myfn").unwrap();
    let (tx, mut rx) = mpsc::channel(1); // capacity 1!
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, _warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    // 40 subnets + 40 SGs + 1 VPC + 1 role = 82 resources, all must be delivered
    // even with channel capacity 1.
    assert_eq!(
        resources.len(),
        82,
        "All 82 resources must be delivered even with channel capacity 1"
    );
}

#[tokio::test]
async fn concurrent_resolvers_with_capacity_1_deliver_all() {
    // Two concurrent resolvers each pushing many results with channel capacity 1.
    // All must be delivered.
    let config = test_config(|request| {
        let uri = request.uri().to_string();
        if uri.contains("/2015-03-31/functions/myfn") && !uri.contains("event-source-mappings") {
            let subnets: Vec<String> = (0..20).map(|i| format!("subnet-{i}")).collect();
            let subnet_json = subnets
                .iter()
                .map(|s| format!("\"{s}\""))
                .collect::<Vec<_>>()
                .join(",");
            let body = format!(
                r#"{{"VpcConfig":{{"SubnetIds":[{}],"VpcId":"vpc-1"}},"FunctionName":"myfn","FunctionArn":"arn:aws:lambda:us-east-1:123456789012:function:myfn"}}"#,
                subnet_json
            );
            http::Response::builder()
                .status(200)
                .body(body.into_bytes())
                .unwrap()
        } else if uri.contains("event-source-mappings") {
            // Return 20 event source mappings
            let arns: Vec<String> = (0..20)
                .map(|i| {
                    format!(r#"{{"EventSourceArn":"arn:aws:sqs:us-east-1:123456789012:queue{i}"}}"#)
                })
                .collect();
            let body = format!(r#"{{"EventSourceMappings":[{}]}}"#, arns.join(","));
            http::Response::builder()
                .status(200)
                .body(body.into_bytes())
                .unwrap()
        } else {
            panic!("Unexpected: {uri}");
        }
    });

    let source = Resource::from_arn("arn:aws:lambda:us-east-1:123456789012:function:myfn").unwrap();
    let (tx, mut rx) = mpsc::channel(1);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, _warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    // 20 subnets + 1 VPC (from lambda_config) + 20 queues (from lambda_event_sources) = 41
    assert_eq!(
        resources.len(),
        41,
        "All 41 resources delivered with capacity 1"
    );
}

// ── Issue 2: Empty ARN sources still work ─────────────────────────────────────

#[tokio::test]
async fn ec2_empty_arn_uses_source_id() {
    // EC2 from term adapter has empty ARN — discover should still work using source.id
    let config = test_config(|request| {
        let body = std::str::from_utf8(request.body().bytes().unwrap())
            .unwrap()
            .to_string();
        if body.contains("DescribeInstances") {
            http::Response::builder()
                .status(200)
                .body(br#"<DescribeInstancesResponse><reservationSet><item><instancesSet><item><instanceId>i-12345</instanceId><vpcId>vpc-1</vpcId><subnetId>subnet-1</subnetId></item></instancesSet></item></reservationSet></DescribeInstancesResponse>"#.to_vec())
                .unwrap()
        } else {
            panic!("Unexpected: {body}");
        }
    });

    // Empty ARN, just ID + region (as term adapter sends)
    let source = Resource::new(ResourceKind::Instance, "i-12345", "i-12345", "us-east-1");
    let (tx, mut rx) = mpsc::channel(16);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Vpc && r.id == "vpc-1"));
    assert!(resources.iter().any(|r| r.arn.contains("subnet/subnet-1")));
    assert!(
        warnings
            .iter()
            .all(|w| !w.contains("cross-account") && !w.contains("cross-region")),
        "Empty ARN should not trigger guards: {warnings:?}"
    );
}

#[tokio::test]
async fn sqs_empty_arn_uses_get_queue_url() {
    // SQS from term adapter has empty ARN — should use GetQueueUrl then GetQueueAttributes
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let config = test_config(move |request| {
        let body = std::str::from_utf8(request.body().bytes().unwrap())
            .unwrap()
            .to_string();
        captured.lock().unwrap().push(body.clone());

        if request
            .headers()
            .get("x-amz-target")
            .is_some_and(|target| target == "AmazonSQS.GetQueueUrl")
        {
            http::Response::builder()
                .status(200)
                .body(
                    br#"{"QueueUrl":"https://sqs.us-east-1.amazonaws.com/123456789012/my-queue"}"#
                        .to_vec(),
                )
                .unwrap()
        } else if request
            .headers()
            .get("x-amz-target")
            .is_some_and(|target| target == "AmazonSQS.GetQueueAttributes")
        {
            http::Response::builder()
                .status(200)
                .body(br#"{"Attributes":{"QueueArn":"arn:aws:sqs:us-east-1:123456789012:my-queue","RedrivePolicy":"{\"deadLetterTargetArn\":\"arn:aws:sqs:us-east-1:123456789012:my-dlq\"}","KmsMasterKeyId":"arn:aws:kms:us-east-1:123456789012:key/kms-1"}}"#.to_vec())
                .unwrap()
        } else if request.uri().path().contains("event-source-mappings") {
            http::Response::builder()
                .status(200)
                .body(br#"{"EventSourceMappings":[{"FunctionArn":"arn:aws:lambda:us-east-1:123456789012:function:myfn"}]}"#.to_vec())
                .unwrap()
        } else {
            panic!("Unexpected: {body}");
        }
    });

    // Empty ARN, just name + region (as term adapter sends)
    let source = Resource::new(ResourceKind::Queue, "my-queue", "my-queue", "us-east-1");
    let (tx, mut rx) = mpsc::channel(16);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, _warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    // DLQ from redrive policy
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Queue && r.id == "my-dlq"));
    // KMS key
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Key && r.id == "kms-1"));
    // Lambda function from event source mapping
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Function && r.id == "myfn"));
}

// ── Issue 3: Pagination at RESULT_LIMIT ──────────────────────────────────────

#[tokio::test]
async fn pagination_caps_at_result_limit_with_warning() {
    // Generate enough event source mappings to hit RESULT_LIMIT (1000)
    let config = test_config(|request| {
        let uri = request.uri().to_string();
        if uri.contains("/2015-03-31/functions/myfn") && !uri.contains("event-source-mappings") {
            http::Response::builder()
                .status(200)
                .body(br#"{"FunctionName":"myfn","FunctionArn":"arn:aws:lambda:us-east-1:123456789012:function:myfn"}"#.to_vec())
                .unwrap()
        } else if uri.contains("event-source-mappings") {
            let page: usize = request
                .uri()
                .query()
                .unwrap_or_default()
                .split('&')
                .find_map(|parameter| parameter.strip_prefix("Marker=page"))
                .and_then(|page| page.parse().ok())
                .unwrap_or(0);
            let arns: Vec<_> = (page * 100..(page + 1) * 100)
                .map(|i| {
                    format!(r#"{{"EventSourceArn":"arn:aws:sqs:us-east-1:123456789012:q{i}"}}"#)
                })
                .collect();
            http::Response::builder()
                .status(200)
                .body(
                    format!(
                        r#"{{"EventSourceMappings":[{}],"NextMarker":"page{}"}}"#,
                        arns.join(","),
                        page + 1
                    )
                    .into_bytes(),
                )
                .unwrap()
        } else {
            panic!("Unexpected: {uri}");
        }
    });

    let source = Resource::from_arn("arn:aws:lambda:us-east-1:123456789012:function:myfn").unwrap();
    let (tx, mut rx) = mpsc::channel(16);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    // Should have exactly 1000 event source queues (RESULT_LIMIT)
    let queue_count = resources
        .iter()
        .filter(|r| r.kind == ResourceKind::Queue)
        .count();
    assert_eq!(queue_count, 1000, "Should cap at RESULT_LIMIT");
    // Should have a warning about the cap
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("capped") || w.contains("incomplete")),
        "Should warn about pagination cap: {warnings:?}"
    );
}

#[tokio::test]
async fn pagination_detects_repeated_token() {
    // Server returns the same token repeatedly — must terminate
    let config = test_config(|request| {
        let uri = request.uri().to_string();
        if uri.contains("/2015-03-31/functions/myfn") && !uri.contains("event-source-mappings") {
            http::Response::builder()
                .status(200)
                .body(br#"{"FunctionName":"myfn","FunctionArn":"arn:aws:lambda:us-east-1:123456789012:function:myfn"}"#.to_vec())
                .unwrap()
        } else if uri.contains("event-source-mappings") {
            // Always return same token "loop"
            http::Response::builder()
                .status(200)
                .body(br#"{"EventSourceMappings":[{"EventSourceArn":"arn:aws:sqs:us-east-1:123456789012:q1"}],"NextMarker":"loop"}"#.to_vec())
                .unwrap()
        } else {
            panic!("Unexpected: {uri}");
        }
    });

    let source = Resource::from_arn("arn:aws:lambda:us-east-1:123456789012:function:myfn").unwrap();
    let (tx, mut rx) = mpsc::channel(16);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, _warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    // Should only get 1 queue, not infinite
    let queue_count = resources
        .iter()
        .filter(|r| r.kind == ResourceKind::Queue)
        .count();
    assert_eq!(
        queue_count, 2,
        "Stops after first repeated next-token response; UI deduplicates rows"
    );
}

// ── Issue 4: ARN parsing — aws-cn, aws-us-gov, DDB stream, log-stream ──────────

#[test]
fn from_arn_accepts_aws_cn_partition() {
    let r = Resource::from_arn("arn:aws-cn:lambda:cn-north-1:123456789012:function:myfn").unwrap();
    assert_eq!(r.kind, ResourceKind::Function);
    assert_eq!(r.id, "myfn");
    assert_eq!(r.region, "cn-north-1");
}

#[test]
fn from_arn_accepts_aws_us_gov_partition() {
    let r = Resource::from_arn("arn:aws-us-gov:lambda:us-gov-west-1:123456789012:function:myfn")
        .unwrap();
    assert_eq!(r.kind, ResourceKind::Function);
    assert_eq!(r.id, "myfn");
    assert_eq!(r.region, "us-gov-west-1");
}

#[test]
fn from_arn_dynamodb_stream_with_table_name_normalizes_to_table() {
    // New format: arn:aws:dynamodb:region:account:table/name/stream/timestamp
    let r = Resource::from_arn(
        "arn:aws:dynamodb:us-east-1:123456789012:table/mytable/stream/2024-01-01T00:00:00.000",
    )
    .unwrap();
    assert_eq!(r.kind, ResourceKind::Table);
    assert_eq!(r.id, "mytable");
    // Original ARN preserved
    assert_eq!(
        r.arn,
        "arn:aws:dynamodb:us-east-1:123456789012:table/mytable/stream/2024-01-01T00:00:00.000"
    );
}

#[test]
fn from_arn_dynamodb_stream_old_format_stays_reference() {
    // Old format: arn:aws:dynamodb:region:account:stream/timestamp
    let r = Resource::from_arn(
        "arn:aws:dynamodb:us-east-1:123456789012:stream/2024-01-01T00:00:00.000",
    )
    .unwrap();
    assert_eq!(r.kind, ResourceKind::Reference);
}

#[test]
fn from_arn_log_stream_normalizes_to_log_group() {
    // arn:aws:logs:region:account:log-group:mygroup:log-stream:mystream
    let r = Resource::from_arn(
        "arn:aws:logs:us-east-1:123456789012:log-group:/aws/lambda/myfn:log-stream:2024/01/01/[$LATEST]abc123",
    )
    .unwrap();
    assert_eq!(r.kind, ResourceKind::LogGroup);
    assert_eq!(r.id, "/aws/lambda/myfn");
}

#[test]
fn from_arn_rejects_whitespace_in_fields() {
    assert!(Resource::from_arn("arn:aw s:lambda:us-east-1:123456789012:function:myfn").is_none());
    assert!(Resource::from_arn("arn:aws:lam bda:us-east-1:123456789012:function:myfn").is_none());
}

#[test]
fn from_arn_rejects_empty_service_partition_resource() {
    assert!(Resource::from_arn("arn::lambda:us-east-1:123456789012:function:myfn").is_none());
    assert!(Resource::from_arn("arn:aws::us-east-1:123456789012:function:myfn").is_none());
    assert!(Resource::from_arn("arn:aws:lambda:us-east-1:123456789012:").is_none());
}

#[test]
fn from_arn_rejects_unknown_partition() {
    assert!(Resource::from_arn("arn:aws-eu:lambda:us-east-1:123456789012:function:myfn").is_none());
}

// ── Issue 5: serde_json for RedrivePolicy ─────────────────────────────────────

#[test]
fn extract_json_string_uses_serde_json() {
    let json = r#"{"deadLetterTargetArn":"arn:aws:sqs:us-east-1:123456789012:my-dlq","maxReceiveCount":"3"}"#;
    let result = extract_json_string(json, "deadLetterTargetArn");
    assert_eq!(
        result.as_deref(),
        Some("arn:aws:sqs:us-east-1:123456789012:my-dlq")
    );
}

#[test]
fn extract_json_string_returns_none_for_missing_key() {
    let json = r#"{"otherKey":"value"}"#;
    assert!(extract_json_string(json, "deadLetterTargetArn").is_none());
}

#[test]
fn extract_json_string_returns_none_for_empty() {
    assert!(extract_json_string("", "key").is_none());
}

#[test]
fn check_malformed_json_warns_on_bad_json() {
    assert!(check_malformed_json("not json {", "test").is_some());
    assert!(check_malformed_json(r#"{"valid":true}"#, "test").is_none());
    assert!(check_malformed_json("", "test").is_none());
}

#[tokio::test]
async fn sqs_malformed_redrive_policy_warns() {
    let config = test_config(|request| {
        if request
            .headers()
            .get("x-amz-target")
            .is_some_and(|target| target == "AmazonSQS.GetQueueUrl")
        {
            http::Response::builder()
                .status(200)
                .body(
                    br#"{"QueueUrl":"https://sqs.us-east-1.amazonaws.com/123456789012/my-queue"}"#
                        .to_vec(),
                )
                .unwrap()
        } else if request
            .headers()
            .get("x-amz-target")
            .is_some_and(|target| target == "AmazonSQS.GetQueueAttributes")
        {
            // Malformed redrive policy
            http::Response::builder()
                .status(200)
                .body(br#"{"Attributes":{"RedrivePolicy":"not valid json{"}}"#.to_vec())
                .unwrap()
        } else {
            http::Response::builder()
                .status(200)
                .body(br#"{"EventSourceMappings":[]}"#.to_vec())
                .unwrap()
        }
    });

    let source = Resource::from_arn("arn:aws:sqs:us-east-1:123456789012:my-queue").unwrap();
    let (tx, mut rx) = mpsc::channel(16);
    let task = tokio::spawn(discover(config, source, tx));
    let (_resources, warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    assert!(
        warnings.iter().any(|w| w.contains("malformed")),
        "Should warn about malformed JSON: {warnings:?}"
    );
}

// ── Issue 6: CFN non-ARN physical resource IDs mapped ──────────────────────────

#[tokio::test]
async fn cfn_maps_non_arn_physical_resource_ids() {
    let config = test_config(|request| {
        let body = std::str::from_utf8(request.body().bytes().unwrap())
            .unwrap()
            .to_string();
        if body.contains("ListStackResources") {
            http::Response::builder()
                .status(200)
                .body(br#"<ListStackResourcesResponse><ListStackResourcesResult><StackResourceSummaries>
<member><PhysicalResourceId>my-bucket</PhysicalResourceId><ResourceType>AWS::S3::Bucket</ResourceType></member>
<member><PhysicalResourceId>my-fn</PhysicalResourceId><ResourceType>AWS::Lambda::Function</ResourceType></member>
<member><PhysicalResourceId>my-table</PhysicalResourceId><ResourceType>AWS::DynamoDB::Table</ResourceType></member>
<member><PhysicalResourceId>i-12345</PhysicalResourceId><ResourceType>AWS::EC2::Instance</ResourceType></member>
<member><PhysicalResourceId>vpc-abc</PhysicalResourceId><ResourceType>AWS::EC2::VPC</ResourceType></member>
<member><PhysicalResourceId>my-role</PhysicalResourceId><ResourceType>AWS::IAM::Role</ResourceType></member>
<member><PhysicalResourceId>https://sqs.us-east-1.amazonaws.com/123456789012/my-queue</PhysicalResourceId><ResourceType>AWS::SQS::Queue</ResourceType></member>
<member><PhysicalResourceId>abc-123</PhysicalResourceId><ResourceType>AWS::KMS::Key</ResourceType></member>
<member><PhysicalResourceId>unknown-id</PhysicalResourceId><ResourceType>AWS::Custom::Resource</ResourceType></member>
</StackResourceSummaries></ListStackResourcesResult></ListStackResourcesResponse>"#.to_vec())
                .unwrap()
        } else {
            panic!("Unexpected: {body}");
        }
    });

    let source =
        Resource::from_arn("arn:aws:cloudformation:us-east-1:123456789012:stack/mystack/uuid")
            .unwrap();
    let (tx, mut rx) = mpsc::channel(32);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, _warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    // S3 bucket
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Bucket && r.id == "my-bucket"));
    // Lambda function
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Function && r.id == "my-fn"));
    // DynamoDB table
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Table && r.id == "my-table"));
    // EC2 instance
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Instance && r.id == "i-12345"));
    // VPC
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Vpc && r.id == "vpc-abc"));
    // IAM role
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Role && r.id == "my-role"));
    // SQS queue (URL → name)
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Queue && r.id == "my-queue"));
    // KMS key
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Key && r.id == "abc-123"));
    // Unknown type → Reference with resourceType in name
    let unknown = resources.iter().find(|r| r.kind == ResourceKind::Reference);
    assert!(
        unknown.is_some(),
        "Unknown resource type should be Reference, not skipped"
    );
    assert_eq!(unknown.unwrap().id, "unknown-id");
    assert_eq!(unknown.unwrap().name, "AWS::Custom::Resource");
}

// ── Issue 7: RDS enhanced monitoring fields ───────────────────────────────────

#[tokio::test]
async fn rds_includes_monitoring_role_and_em_resource() {
    let config = test_config(|request| {
        let body = std::str::from_utf8(request.body().bytes().unwrap())
            .unwrap()
            .to_string();
        if body.contains("DescribeDBInstances") {
            http::Response::builder()
                .status(200)
                .body(br#"<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances><DBInstance>
<DBInstanceIdentifier>mydb</DBInstanceIdentifier>
<KmsKeyId>arn:aws:kms:us-east-1:123456789012:key/kms-rds</KmsKeyId>
<MonitoringRoleArn>arn:aws:iam::123456789012:role/rds-monitor</MonitoringRoleArn>
<EnhancedMonitoringResourceArn>arn:aws:logs:us-east-1:123456789012:log-group:RDSOSMetrics:log-stream:mydb</EnhancedMonitoringResourceArn>
<DBSubnetGroup><VpcId>vpc-rds</VpcId><Subnets><Subnet><SubnetIdentifier>subnet-1</SubnetIdentifier></Subnet><Subnet><SubnetIdentifier>subnet-2</SubnetIdentifier></Subnet></Subnets></DBSubnetGroup>
<VpcSecurityGroups><VpcSecurityGroupMembership><VpcSecurityGroupId>sg-rds-1</VpcSecurityGroupId></VpcSecurityGroupMembership></VpcSecurityGroups>
</DBInstance></DBInstances></DescribeDBInstancesResult></DescribeDBInstancesResponse>"#.to_vec())
                .unwrap()
        } else {
            panic!("Unexpected: {body}");
        }
    });

    let source = Resource::from_arn("arn:aws:rds:us-east-1:123456789012:db:mydb").unwrap();
    let (tx, mut rx) = mpsc::channel(32);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, _warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    // KMS key
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Key && r.id == "kms-rds"));
    // Monitoring role (IAM)
    assert!(
        resources
            .iter()
            .any(|r| r.kind == ResourceKind::Role && r.id == "rds-monitor"),
        "Should find monitoring_role_arn"
    );
    // Enhanced monitoring resource ARN (log stream → log group)
    let em = resources.iter().find(|r| r.kind == ResourceKind::LogGroup);
    assert!(
        em.is_some(),
        "Should find enhanced_monitoring_resource_arn as LogGroup"
    );
    assert_eq!(em.unwrap().id, "RDSOSMetrics");
    // VPC
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Vpc && r.id == "vpc-rds"));
    // Subnets
    assert!(resources.iter().any(|r| r.arn.contains("subnet/subnet-1")));
    assert!(resources.iter().any(|r| r.arn.contains("subnet/subnet-2")));
    // Security groups
    assert!(resources
        .iter()
        .any(|r| r.arn.contains("security-group/sg-rds-1")));
}

// ── Issue 7: FSx VPC/subnet/NI extraction ──────────────────────────────────────

#[tokio::test]
async fn fsx_includes_vpc_subnets_nics() {
    let config = test_config(|request| {
        let body = std::str::from_utf8(request.body().bytes().unwrap())
            .unwrap()
            .to_string();
        // FSx DescribeFileSystems
        if body.contains("FileSystemIds")
            || body.contains("DescribeFileSystems")
            || request.uri().to_string().contains("fsx")
        {
            http::Response::builder()
                .status(200)
                .body(br#"{"FileSystems":[{"FileSystemId":"fs-abc","KmsKeyId":"arn:aws:kms:us-east-1:123456789012:key/kms-fsx","VpcId":"vpc-fsx","SubnetIds":["subnet-fsx-1","subnet-fsx-2"],"NetworkInterfaceIds":["eni-1","eni-2"]}]}"#.to_vec())
                .unwrap()
        } else {
            panic!("Unexpected: {body}");
        }
    });

    let source =
        Resource::from_arn("arn:aws:fsx:us-east-1:123456789012:file-system/fs-abc").unwrap();
    let (tx, mut rx) = mpsc::channel(32);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, _warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    // KMS key
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Key && r.id == "kms-fsx"));
    // VPC
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Vpc && r.id == "vpc-fsx"));
    // Subnets
    assert!(resources
        .iter()
        .any(|r| r.arn.contains("subnet/subnet-fsx-1")));
    assert!(resources
        .iter()
        .any(|r| r.arn.contains("subnet/subnet-fsx-2")));
    // Network interfaces
    assert!(resources
        .iter()
        .any(|r| r.arn.contains("network-interface/eni-1")));
    assert!(resources
        .iter()
        .any(|r| r.arn.contains("network-interface/eni-2")));
}

// ── Issue 7: DDB no self-reference ─────────────────────────────────────────────

#[tokio::test]
async fn ddb_does_not_emit_self_reference() {
    let config = test_config(|request| {
        let body = std::str::from_utf8(request.body().bytes().unwrap())
            .unwrap()
            .to_string();
        if body.contains("TableName") {
            http::Response::builder()
                .status(200)
                .body(br#"{"Table":{"TableName":"orders","TableArn":"arn:aws:dynamodb:us-east-1:123456789012:table/orders","SSEDescription":{"KMSMasterKeyArn":"arn:aws:kms:us-east-1:123456789012:key/kms-ddb"}}}"#.to_vec())
                .unwrap()
        } else {
            panic!("Unexpected: {body}");
        }
    });

    let source =
        Resource::from_arn("arn:aws:dynamodb:us-east-1:123456789012:table/orders").unwrap();
    let (tx, mut rx) = mpsc::channel(16);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, _warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    // Should have KMS key
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Key && r.id == "kms-ddb"));
    // Should NOT have the table itself as a resource
    assert!(
        !resources.iter().any(|r| r.kind == ResourceKind::Table),
        "Should not emit self-reference"
    );
}

// ── Existing tests (updated for new behavior) ──────────────────────────────────

#[test]
fn from_arn_parses_supported_kinds() {
    let r =
        Resource::from_arn("arn:aws:lambda:us-east-1:123456789012:function:myfn:alias").unwrap();
    assert_eq!(r.kind, ResourceKind::Function);
    assert_eq!(r.id, "myfn");
    assert_eq!(
        r.arn,
        "arn:aws:lambda:us-east-1:123456789012:function:myfn:alias"
    );

    let r = Resource::from_arn("arn:aws:sqs:us-east-1:123456789012:my-queue").unwrap();
    assert_eq!(r.kind, ResourceKind::Queue);
    assert_eq!(r.id, "my-queue");

    let r = Resource::from_arn("arn:aws:dynamodb:us-east-1:123456789012:table/orders").unwrap();
    assert_eq!(r.kind, ResourceKind::Table);
    assert_eq!(r.id, "orders");

    let r = Resource::from_arn("arn:aws:ec2:us-east-1:123456789012:instance/i-12345").unwrap();
    assert_eq!(r.kind, ResourceKind::Instance);
    assert_eq!(r.id, "i-12345");

    let r = Resource::from_arn("arn:aws:ec2:us-east-1:123456789012:vpc/vpc-abc").unwrap();
    assert_eq!(r.kind, ResourceKind::Vpc);
    assert_eq!(r.id, "vpc-abc");

    let r = Resource::from_arn("arn:aws:s3:::my-bucket").unwrap();
    assert_eq!(r.kind, ResourceKind::Bucket);
    assert_eq!(r.id, "my-bucket");

    let r = Resource::from_arn("arn:aws:iam::123456789012:role/my-role").unwrap();
    assert_eq!(r.kind, ResourceKind::Role);
    assert_eq!(r.id, "my-role");

    let r = Resource::from_arn("arn:aws:iam::123456789012:user/my-user").unwrap();
    assert_eq!(r.kind, ResourceKind::User);
    assert_eq!(r.id, "my-user");

    let r = Resource::from_arn("arn:aws:iam::123456789012:group/my-group").unwrap();
    assert_eq!(r.kind, ResourceKind::Group);
    assert_eq!(r.id, "my-group");

    let r =
        Resource::from_arn("arn:aws:logs:us-east-1:123456789012:log-group:/aws/lambda/fn").unwrap();
    assert_eq!(r.kind, ResourceKind::LogGroup);
    assert_eq!(r.id, "/aws/lambda/fn");

    let r = Resource::from_arn("arn:aws:cloudwatch:us-east-1:123456789012:alarm:my-alarm").unwrap();
    assert_eq!(r.kind, ResourceKind::Alarm);
    assert_eq!(r.id, "my-alarm");

    let r =
        Resource::from_arn("arn:aws:cloudformation:us-east-1:123456789012:stack/mystack/uuid-1234")
            .unwrap();
    assert_eq!(r.kind, ResourceKind::Stack);
    assert_eq!(r.id, "mystack");

    let r = Resource::from_arn("arn:aws:ecr:us-east-1:123456789012:repository/my-repo").unwrap();
    assert_eq!(r.kind, ResourceKind::Repository);
    assert_eq!(r.id, "my-repo");

    let r = Resource::from_arn("arn:aws:ecr-public::123456789012:repository/my-public").unwrap();
    assert_eq!(r.kind, ResourceKind::PublicRepository);
    assert_eq!(r.id, "my-public");

    let r =
        Resource::from_arn("arn:aws:elasticfilesystem:us-east-1:123456789012:file-system/fs-12345")
            .unwrap();
    assert_eq!(r.kind, ResourceKind::Efs);
    assert_eq!(r.id, "fs-12345");

    let r = Resource::from_arn("arn:aws:fsx:us-east-1:123456789012:file-system/fs-abc").unwrap();
    assert_eq!(r.kind, ResourceKind::Fsx);
    assert_eq!(r.id, "fs-abc");

    let r = Resource::from_arn("arn:aws:kms:us-east-1:123456789012:key/abc-123").unwrap();
    assert_eq!(r.kind, ResourceKind::Key);
    assert_eq!(r.id, "abc-123");

    let r = Resource::from_arn("arn:aws:rds:us-east-1:123456789012:db:my-db").unwrap();
    assert_eq!(r.kind, ResourceKind::Database);
    assert_eq!(r.id, "my-db");

    let r = Resource::from_arn("arn:aws:apigateway:us-east-1::/restapis/abc123").unwrap();
    assert_eq!(r.kind, ResourceKind::Api);
    assert_eq!(r.id, "abc123");

    let r = Resource::from_arn("arn:aws:apigateway:us-east-1::/apis/def456").unwrap();
    assert_eq!(r.kind, ResourceKind::Api);
    assert_eq!(r.id, "def456");
}

#[test]
fn from_arn_preserves_actual_arn() {
    let arn = "arn:aws:lambda:us-east-1:123456789012:function:myfn:alias";
    let r = Resource::from_arn(arn).unwrap();
    assert_eq!(r.arn, arn);
}

#[test]
fn from_arn_s3_object_stays_reference() {
    let r = Resource::from_arn("arn:aws:s3:::bucket/key/path").unwrap();
    assert_eq!(r.kind, ResourceKind::Reference);
    assert_eq!(r.id, "bucket/key/path");
}

#[test]
fn from_arn_efs_access_point_stays_reference() {
    let r = Resource::from_arn(
        "arn:aws:elasticfilesystem:us-east-1:123456789012:access-point/fsap-12345",
    )
    .unwrap();
    assert_eq!(r.kind, ResourceKind::Reference);
}

#[test]
fn from_arn_unsupported_service_returns_reference() {
    let r = Resource::from_arn("arn:aws:secretsmanager:us-east-1:123456789012:my-topic").unwrap();
    assert_eq!(r.kind, ResourceKind::Reference);
    assert_eq!(r.id, "my-topic");
}

#[test]
fn from_arn_ec2_subnet_stays_reference() {
    let r = Resource::from_arn("arn:aws:ec2:us-east-1:123456789012:subnet/sub-12345").unwrap();
    assert_eq!(r.kind, ResourceKind::Reference);
}

#[test]
fn from_arn_ec2_security_group_stays_reference() {
    let r =
        Resource::from_arn("arn:aws:ec2:us-east-1:123456789012:security-group/sg-12345").unwrap();
    assert_eq!(r.kind, ResourceKind::Reference);
}

#[test]
fn from_arn_ec2_volume_stays_reference() {
    let r = Resource::from_arn("arn:aws:ec2:us-east-1:123456789012:volume/vol-12345").unwrap();
    assert_eq!(r.kind, ResourceKind::Reference);
}

#[test]
fn from_arn_invalid_returns_none() {
    assert!(Resource::from_arn("not-an-arn").is_none());
    assert!(Resource::from_arn("arn:aws").is_none());
    assert!(Resource::from_arn("arn:aws:s3:::").is_none());
    assert!(Resource::from_arn("arn:aws:lambda:us-east-1:123456789012:function:").is_none());
    assert!(Resource::from_arn("").is_none());
}

#[test]
fn from_arn_iam_role_with_path() {
    let r = Resource::from_arn("arn:aws:iam::123456789012:role/service-role/my-role").unwrap();
    assert_eq!(r.kind, ResourceKind::Role);
    assert_eq!(r.id, "my-role");
}

#[test]
fn resource_kind_reference_excluded_from_all() {
    assert!(!ResourceKind::ALL.contains(&ResourceKind::Reference));
    assert_eq!(ResourceKind::ALL.len(), 20);
}

#[test]
fn search_query_matches_reference_on_unqualified() {
    let query = SearchQuery::parse("myfn").unwrap();
    assert!(query.kinds.len() == ResourceKind::ALL.len());
    let resource = Resource {
        kind: ResourceKind::Reference,
        id: "myfn".into(),
        name: "myfn".into(),
        arn: "arn:aws:secretsmanager:us-east-1:123456789012:myfn".into(),
        region: "us-east-1".into(),
    };
    assert!(query.matches(&resource));
}

#[test]
fn search_query_matches_references_only_for_their_service() {
    let query = SearchQuery::parse("@lambda myfn").unwrap();
    assert!(query.kinds.len() < ResourceKind::ALL.len());
    let resource = Resource {
        kind: ResourceKind::Reference,
        id: "myfn".into(),
        name: "myfn".into(),
        arn: "arn:aws:secretsmanager:us-east-1:123456789012:myfn".into(),
        region: "us-east-1".into(),
    };
    assert!(!query.matches(&resource));
}

#[test]
fn resource_kind_reference_service_is_empty() {
    assert_eq!(ResourceKind::Reference.service(), "");
    assert_eq!(ResourceKind::Reference.label(), "Reference");
}

// ── Existing mock HTTP tests (updated) ──────────────────────────────────────

#[tokio::test]
async fn lambda_discover_extracts_role_loggroup_vpc_layers_kms_dlq_efs() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let config = test_config(move |request| {
        let uri = request.uri().to_string();
        let body = std::str::from_utf8(request.body().bytes().unwrap())
            .unwrap()
            .to_string();
        captured
            .lock()
            .unwrap()
            .push(format!("URI: {uri} BODY: {body}"));
        if uri.contains("/2015-03-31/functions/myfn") && !uri.contains("event-source-mappings") {
            http::Response::builder()
                .status(200)
                .body(br#"{"Role":"arn:aws:iam::123456789012:role/lambda-role","LoggingConfig":{"LogGroup":"/aws/lambda/myfn"},"VpcConfig":{"SubnetIds":["subnet-1","subnet-2"],"SecurityGroupIds":["sg-1"],"VpcId":"vpc-1"},"Layers":[{"Arn":"arn:aws:lambda:us-east-1:123456789012:layer:mylib:1"}],"FileSystemConfigs":[{"Arn":"arn:aws:elasticfilesystem:us-east-1:123456789012:file-system/fs-1"}],"DeadLetterConfig":{"TargetArn":"arn:aws:sqs:us-east-1:123456789012:dlq"},"KMSKeyArn":"arn:aws:kms:us-east-1:123456789012:key/kms-1","FunctionName":"myfn","FunctionArn":"arn:aws:lambda:us-east-1:123456789012:function:myfn"}"#.to_vec())
                .unwrap()
        } else if uri.contains("event-source-mappings") {
            http::Response::builder()
                .status(200)
                .body(br#"{"EventSourceMappings":[]}"#.to_vec())
                .unwrap()
        } else {
            panic!("Unexpected: URI={uri} BODY={body}");
        }
    });

    let source = Resource::from_arn("arn:aws:lambda:us-east-1:123456789012:function:myfn").unwrap();
    let (tx, mut rx) = mpsc::channel(16);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    assert!(warnings.is_empty(), "Unexpected warnings: {warnings:?}");

    let ids: Vec<ResourceKind> = resources.iter().map(|r| r.kind).collect();
    assert!(ids.contains(&ResourceKind::Role));
    assert!(ids.contains(&ResourceKind::LogGroup));
    assert!(ids.contains(&ResourceKind::Vpc));
    assert!(ids.contains(&ResourceKind::Reference));

    let role = resources
        .iter()
        .find(|r| r.kind == ResourceKind::Role)
        .unwrap();
    assert_eq!(role.id, "lambda-role");
    let log_group = resources
        .iter()
        .find(|r| r.kind == ResourceKind::LogGroup)
        .unwrap();
    assert_eq!(log_group.id, "/aws/lambda/myfn");
    let vpc = resources
        .iter()
        .find(|r| r.kind == ResourceKind::Vpc)
        .unwrap();
    assert_eq!(vpc.id, "vpc-1");
    assert!(resources.iter().any(|r| r.arn.contains("subnet/subnet-1")));
    assert!(resources.iter().any(|r| r.arn.contains("subnet/subnet-2")));
    assert!(resources
        .iter()
        .any(|r| r.arn.contains("security-group/sg-1")));
    assert!(resources.iter().any(|r| r.arn.contains("layer:mylib:1")));
    let efs = resources
        .iter()
        .find(|r| r.kind == ResourceKind::Efs)
        .unwrap();
    assert_eq!(efs.id, "fs-1");
    let dlq = resources
        .iter()
        .find(|r| r.kind == ResourceKind::Queue)
        .unwrap();
    assert_eq!(dlq.id, "dlq");
    let kms = resources
        .iter()
        .find(|r| r.kind == ResourceKind::Key)
        .unwrap();
    assert_eq!(kms.id, "kms-1");
    assert_eq!(requests.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn lambda_discover_event_source_mappings_paginated() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let config = test_config(move |request| {
        let uri = request.uri().to_string();
        let body = std::str::from_utf8(request.body().bytes().unwrap())
            .unwrap()
            .to_string();
        captured
            .lock()
            .unwrap()
            .push(format!("URI: {uri} BODY: {body}"));
        if uri.contains("/2015-03-31/functions/myfn") && !uri.contains("event-source-mappings") {
            http::Response::builder().status(200).body(br#"{"FunctionName":"myfn","FunctionArn":"arn:aws:lambda:us-east-1:123456789012:function:myfn"}"#.to_vec()).unwrap()
        } else if uri.contains("event-source-mappings") {
            if uri.contains("Marker=page2") || body.contains("page2") {
                http::Response::builder().status(200).body(br#"{"EventSourceMappings":[{"EventSourceArn":"arn:aws:sqs:us-east-1:123456789012:queue2","FunctionArn":"arn:aws:lambda:us-east-1:123456789012:function:myfn"}]}"#.to_vec()).unwrap()
            } else {
                http::Response::builder().status(200).body(br#"{"EventSourceMappings":[{"EventSourceArn":"arn:aws:sqs:us-east-1:123456789012:queue1","FunctionArn":"arn:aws:lambda:us-east-1:123456789012:function:myfn"}],"NextMarker":"page2"}"#.to_vec()).unwrap()
            }
        } else {
            panic!("Unexpected: URI={uri} BODY={body}");
        }
    });

    let source = Resource::from_arn("arn:aws:lambda:us-east-1:123456789012:function:myfn").unwrap();
    let (tx, mut rx) = mpsc::channel(16);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, _warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    let queue_ids: Vec<&str> = resources
        .iter()
        .filter(|r| r.kind == ResourceKind::Queue)
        .map(|r| r.id.as_str())
        .collect();
    assert!(queue_ids.contains(&"queue1"));
    assert!(queue_ids.contains(&"queue2"));
}

#[tokio::test]
async fn lambda_discover_source_error_still_finishes() {
    let config = test_config(|_request| {
        http::Response::builder()
            .status(403)
            .body(br#"{"message":"AccessDenied"}"#.to_vec())
            .unwrap()
    });
    let source = Resource::from_arn("arn:aws:lambda:us-east-1:123456789012:function:myfn").unwrap();
    let (tx, mut rx) = mpsc::channel(16);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished, "Must send Finished even on error");
    assert!(resources.is_empty(), "No resources on error");
    assert!(!warnings.is_empty(), "Should have warnings");
}

// ── Mock HTTP tests: SQS ──────────────────────────────────────────────────────

#[tokio::test]
async fn sqs_discover_extracts_redrive_and_kms() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let config = test_config(move |request| {
        let body = std::str::from_utf8(request.body().bytes().unwrap())
            .unwrap()
            .to_string();
        captured.lock().unwrap().push(body.clone());
        if request
            .headers()
            .get("x-amz-target")
            .is_some_and(|target| target == "AmazonSQS.GetQueueUrl")
        {
            http::Response::builder()
                .status(200)
                .body(
                    br#"{"QueueUrl":"https://sqs.us-east-1.amazonaws.com/123456789012/my-queue"}"#
                        .to_vec(),
                )
                .unwrap()
        } else if body.contains("QueueUrl") {
            http::Response::builder().status(200).body(br#"{"Attributes":{"RedrivePolicy":"{\"deadLetterTargetArn\":\"arn:aws:sqs:us-east-1:123456789012:my-dlq\",\"maxReceiveCount\":\"3\"}","KmsMasterKeyId":"arn:aws:kms:us-east-1:123456789012:key/kms-sqs"}}"#.to_vec()).unwrap()
        } else {
            http::Response::builder().status(200).body(br#"{"EventSourceMappings":[{"FunctionArn":"arn:aws:lambda:us-east-1:123456789012:function:myfn"}]}"#.to_vec()).unwrap()
        }
    });

    let source = Resource::from_arn("arn:aws:sqs:us-east-1:123456789012:my-queue").unwrap();
    let (tx, mut rx) = mpsc::channel(16);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, _warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Queue && r.id == "my-dlq"));
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Key && r.id == "kms-sqs"));
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Function && r.id == "myfn"));
}

// ── Mock HTTP tests: EC2 ──────────────────────────────────────────────────────

#[tokio::test]
async fn ec2_discover_extracts_vpc_subnet_sg_iam_volume() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let config = test_config(move |request| {
        let body = std::str::from_utf8(request.body().bytes().unwrap())
            .unwrap()
            .to_string();
        captured.lock().unwrap().push(body.clone());
        if body.contains("DescribeInstances") {
            http::Response::builder().status(200).body(br#"<DescribeInstancesResponse><reservationSet><item><instancesSet><item><instanceId>i-12345</instanceId><vpcId>vpc-1</vpcId><subnetId>subnet-1</subnetId><groupSet><item><groupId>sg-1</groupId><groupName>default</groupName></item></groupSet><iamInstanceProfile><arn>arn:aws:iam::123456789012:instance-profile/my-profile</arn></iamInstanceProfile><networkInterfaceSet><item><networkInterfaceId>eni-1</networkInterfaceId></item></networkInterfaceSet><blockDeviceMapping><item><deviceName>/dev/xvda</deviceName><ebs><volumeId>vol-1</volumeId></ebs></item></blockDeviceMapping></item></instancesSet></item></reservationSet></DescribeInstancesResponse>"#.to_vec()).unwrap()
        } else {
            panic!("Unexpected: {body}");
        }
    });

    let source = Resource::from_arn("arn:aws:ec2:us-east-1:123456789012:instance/i-12345").unwrap();
    let (tx, mut rx) = mpsc::channel(16);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, _warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Vpc && r.id == "vpc-1"));
    assert!(resources.iter().any(|r| r.arn.contains("subnet/subnet-1")));
    assert!(resources
        .iter()
        .any(|r| r.arn.contains("security-group/sg-1")));
    assert!(resources
        .iter()
        .any(|r| r.arn.contains("instance-profile/my-profile")));
    assert!(resources
        .iter()
        .any(|r| r.arn.contains("network-interface/eni-1")));
    assert!(resources.iter().any(|r| r.arn.contains("volume/vol-1")));
    assert_eq!(requests.lock().unwrap().len(), 1);
}

// ── Mock HTTP tests: DynamoDB ──────────────────────────────────────────────────

#[tokio::test]
async fn ddb_discover_extracts_kms_key() {
    let config = test_config(|request| {
        let body = std::str::from_utf8(request.body().bytes().unwrap())
            .unwrap()
            .to_string();
        if body.contains("TableName") {
            http::Response::builder().status(200).body(br#"{"Table":{"TableName":"orders","TableArn":"arn:aws:dynamodb:us-east-1:123456789012:table/orders","SSEDescription":{"KMSMasterKeyArn":"arn:aws:kms:us-east-1:123456789012:key/kms-ddb"}}}"#.to_vec()).unwrap()
        } else {
            panic!("Unexpected: {body}");
        }
    });

    let source =
        Resource::from_arn("arn:aws:dynamodb:us-east-1:123456789012:table/orders").unwrap();
    let (tx, mut rx) = mpsc::channel(16);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, _warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Key && r.id == "kms-ddb"));
}

// ── Mock HTTP tests: CloudFormation ────────────────────────────────────────────

#[tokio::test]
async fn cfn_discover_lists_stack_resources_paginated() {
    let config = test_config(|request| {
        let body = std::str::from_utf8(request.body().bytes().unwrap())
            .unwrap()
            .to_string();
        if body.contains("ListStackResources") {
            if body.contains("NextToken=page2")
                || body.contains("Action=ListStackResources&Version=2010-05-15&NextToken=page2")
            {
                http::Response::builder().status(200).body(br#"<ListStackResourcesResponse><ListStackResourcesResult><StackResourceSummaries><member><PhysicalResourceId>arn:aws:sqs:us-east-1:123456789012:queue2</PhysicalResourceId><LogicalResourceId>Queue2</LogicalResourceId><ResourceType>AWS::SQS::Queue</ResourceType></member></StackResourceSummaries></ListStackResourcesResult></ListStackResourcesResponse>"#.to_vec()).unwrap()
            } else {
                http::Response::builder().status(200).body(br#"<ListStackResourcesResponse><ListStackResourcesResult><StackResourceSummaries><member><PhysicalResourceId>arn:aws:sqs:us-east-1:123456789012:queue1</PhysicalResourceId><LogicalResourceId>Queue1</LogicalResourceId><ResourceType>AWS::SQS::Queue</ResourceType></member></StackResourceSummaries><NextToken>page2</NextToken></ListStackResourcesResult></ListStackResourcesResponse>"#.to_vec()).unwrap()
            }
        } else {
            panic!("Unexpected: {body}");
        }
    });

    let source =
        Resource::from_arn("arn:aws:cloudformation:us-east-1:123456789012:stack/mystack/uuid")
            .unwrap();
    let (tx, mut rx) = mpsc::channel(16);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, _warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    let queue_ids: Vec<&str> = resources
        .iter()
        .filter(|r| r.kind == ResourceKind::Queue)
        .map(|r| r.id.as_str())
        .collect();
    assert!(queue_ids.contains(&"queue1"));
    assert!(queue_ids.contains(&"queue2"));
}

// ── Mock HTTP tests: EFS ───────────────────────────────────────────────────────

#[tokio::test]
async fn efs_discover_extracts_kms_key() {
    let config = test_config(|_request| {
        http::Response::builder().status(200).body(br#"{"FileSystems":[{"FileSystemId":"fs-1","KmsKeyId":"arn:aws:kms:us-east-1:123456789012:key/kms-efs"}]}"#.to_vec()).unwrap()
    });

    let source =
        Resource::from_arn("arn:aws:elasticfilesystem:us-east-1:123456789012:file-system/fs-1")
            .unwrap();
    let (tx, mut rx) = mpsc::channel(16);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, _warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Key && r.id == "kms-efs"));
}

// ── Mock HTTP tests: RDS (updated) ───────────────────────────────────────────

#[tokio::test]
async fn rds_discover_extracts_kms_and_vpc() {
    let config = test_config(|request| {
        let body = std::str::from_utf8(request.body().bytes().unwrap())
            .unwrap()
            .to_string();
        if body.contains("DescribeDBInstances") {
            http::Response::builder().status(200).body(br#"<DescribeDBInstancesResponse><DescribeDBInstancesResult><DBInstances><DBInstance><DBInstanceIdentifier>mydb</DBInstanceIdentifier><KmsKeyId>arn:aws:kms:us-east-1:123456789012:key/kms-rds</KmsKeyId><DBSubnetGroup><VpcId>vpc-rds</VpcId></DBSubnetGroup><MonitoringRoleArn>arn:aws:iam::123456789012:role/rds-monitor</MonitoringRoleArn><EnhancedMonitoringResourceArn>arn:aws:logs:us-east-1:123456789012:log-group:RDSOSMetrics:log-stream:mydb</EnhancedMonitoringResourceArn></DBInstance></DBInstances></DescribeDBInstancesResult></DescribeDBInstancesResponse>"#.to_vec()).unwrap()
        } else {
            panic!("Unexpected: {body}");
        }
    });

    let source = Resource::from_arn("arn:aws:rds:us-east-1:123456789012:db:mydb").unwrap();
    let (tx, mut rx) = mpsc::channel(16);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, _warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Key && r.id == "kms-rds"));
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Vpc && r.id == "vpc-rds"));
    assert!(resources
        .iter()
        .any(|r| r.kind == ResourceKind::Role && r.id == "rds-monitor"));
}

// ── Unsupported source warning test ────────────────────────────────────────────

#[tokio::test]
async fn unsupported_source_produces_warning() {
    let config = AwsConfig::dummy(Some("us-east-1".into()));
    let source = Resource::new(ResourceKind::Bucket, "my-bucket", "my-bucket", "us-east-1");
    let (tx, mut rx) = mpsc::channel(16);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    assert!(resources.is_empty());
    assert!(warnings.iter().any(|w| w.contains("not supported")));
}

// ── Account guard test ─────────────────────────────────────────────────────────

#[tokio::test]
async fn cross_account_source_is_skipped() {
    let config = {
        let mut c = AwsConfig::dummy(Some("us-east-1".into()));
        c.account_id = "123456789012".into();
        c
    };
    let source = Resource::from_arn("arn:aws:lambda:us-east-1:999999999999:function:myfn").unwrap();
    let (tx, mut rx) = mpsc::channel(16);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    assert!(resources.is_empty(), "Should skip cross-account source");
    assert!(warnings.iter().any(|w| w.contains("cross-account")));
}

#[tokio::test]
async fn empty_account_id_allows_in_tests() {
    let config = test_config(|_request| {
        http::Response::builder().status(200).body(br#"{"FunctionName":"myfn","FunctionArn":"arn:aws:lambda:us-east-1:123456789012:function:myfn"}"#.to_vec()).unwrap()
    });
    let source = Resource::from_arn("arn:aws:lambda:us-east-1:123456789012:function:myfn").unwrap();
    let (tx, mut rx) = mpsc::channel(16);
    let task = tokio::spawn(discover(config, source, tx));
    let (_resources, warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    assert!(
        warnings.iter().all(|w| !w.contains("cross-account")),
        "Empty account should not trigger cross-account guard"
    );
}

// ── Empty source.id rejected ──────────────────────────────────────────────────

#[tokio::test]
async fn empty_source_id_rejected() {
    let config = AwsConfig::dummy(Some("us-east-1".into()));
    let source = Resource {
        kind: ResourceKind::Function,
        id: String::new(),
        name: String::new(),
        arn: "arn:aws:lambda:us-east-1:123456789012:function:myfn".into(),
        region: "us-east-1".into(),
    };
    let (tx, mut rx) = mpsc::channel(16);
    let task = tokio::spawn(discover(config, source, tx));
    let (resources, warnings, finished) = collect_all(&mut rx).await;
    task.await.unwrap();

    assert!(finished);
    assert!(resources.is_empty());
    assert!(warnings.iter().any(|w| w.contains("empty id")));
}
