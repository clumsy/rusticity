//! Read-only discovery of explicitly-linked AWS resources from a single source.
//!
//! Given one resource (e.g. a Lambda function), `discover` queries scoped SDK
//! describe/get responses to extract ARNs and IDs of directly-related resources
//! — IAM roles, log groups, VPC subnets, SQS dead-letter queues, etc. — and streams
//! them as [`SearchUpdate`] messages.
//!
//! Design constraints:
//! - No account-wide crawls, no recursion, no IAM-policy or tag-based guesses.
//! - Only explicit ARNs and IDs returned by scoped SDK calls are extracted.
//! - Independent sources run in parallel (bounded 2–4); each source has a 20 s
//!   timeout and capped pagination.
//! - `Finished` is always sent, even if every source errored.
//! - Account/region mismatch guards prevent cross-account or cross-region reads
//!   when the source ARN doesn't match the configured session.
//!
//! Unsupported source kinds produce a `Warning`, not a silent empty result.
use crate::config::AwsConfig;
use crate::finder::{Resource, ResourceKind, SearchUpdate, RESULT_LIMIT};
use crate::pagination::{Pagination, StopReason};
use anyhow::{anyhow, Result};
use aws_sdk_sqs::types::QueueAttributeName;
use futures::{stream, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio::sync::mpsc;

const TIMEOUT: Duration = Duration::from_secs(20);
const CONCURRENCY: usize = 4;

/// Entry point — discover related resources for the given source.
///
/// Streams `SearchUpdate::Results` (each resource wrapped as a `Reference` or
/// its native `ResourceKind`), `SearchUpdate::Warning` for per-source failures,
/// and a final `SearchUpdate::Finished`.
pub async fn discover(config: AwsConfig, source: Resource, tx: mpsc::Sender<SearchUpdate>) {
    let _ = tx.send(SearchUpdate::Provider("Related resources")).await;

    if !has_resolver(source.kind) {
        let _ = tx
            .send(SearchUpdate::Warning(format!(
                "Related discovery not supported for {} {}",
                source.kind.label(),
                source.id
            )))
            .await;
        let _ = tx.send(SearchUpdate::Finished { limited: false }).await;
        return;
    }

    // Reject empty source.id before any requests.
    if source.id.is_empty() {
        let _ = tx
            .send(SearchUpdate::Warning(
                "Source has empty id, skipping related discovery".into(),
            ))
            .await;
        let _ = tx.send(SearchUpdate::Finished { limited: false }).await;
        return;
    }

    // Centralized guard: only check actual nonempty ARN for account/region.
    // For global services (iam), region check is skipped.
    // For empty ARN (e.g. EC2/SQS from term adapter), skip account/region guard
    // but still proceed with source.id + region.
    if !guard_source(&config, &source, &tx).await {
        let _ = tx.send(SearchUpdate::Finished { limited: false }).await;
        return;
    }
    let mut config = config;
    if config.account_id.is_empty() {
        config.account_id = source.arn.split(':').nth(4).unwrap_or_default().into();
    }
    let parts = if source.kind == ResourceKind::Function {
        2
    } else {
        1
    };
    let mut work = stream::iter(0..parts)
        .map(|part| {
            let config = config.clone();
            let source = source.clone();
            let tx = tx.clone();
            async move {
                let request = async {
                    match source.kind {
                        ResourceKind::Function if part == 0 => {
                            lambda_config_inner(config, source, tx.clone()).await
                        }
                        ResourceKind::Function => {
                            lambda_event_sources_inner(config, source, tx.clone()).await
                        }
                        ResourceKind::Queue => {
                            sqs_attributes_inner(config, source, tx.clone()).await
                        }
                        ResourceKind::Instance => {
                            ec2_instance_inner(config, source, tx.clone()).await
                        }
                        ResourceKind::Table => ddb_table_inner(config, source, tx.clone()).await,
                        ResourceKind::Stack => {
                            cfn_stack_resources_inner(config, source, tx.clone()).await
                        }
                        ResourceKind::Efs => efs_filesystem_inner(config, source, tx.clone()).await,
                        ResourceKind::Fsx => fsx_filesystem_inner(config, source, tx.clone()).await,
                        ResourceKind::Database => {
                            rds_instance_inner(config, source, tx.clone()).await
                        }
                        _ => Ok(false),
                    }
                };
                match tokio::time::timeout(TIMEOUT, request).await {
                    Ok(Ok(limited)) => limited,
                    result => {
                        let error = match result {
                            Ok(Err(error)) => format!("{error:#}"),
                            _ => "request timed out".into(),
                        };
                        let _ = tx
                            .send(SearchUpdate::Warning(format!(
                                "Related resource lookup: {error}"
                            )))
                            .await;
                        true
                    }
                }
            }
        })
        .buffer_unordered(CONCURRENCY);
    let mut limited = false;
    while let Some(partial) = work.next().await {
        limited |= partial;
    }
    let _ = tx.send(SearchUpdate::Finished { limited }).await;
}

/// Returns false (and sends warnings) if the source ARN doesn't match
/// the configured account/region.
async fn guard_source(
    config: &AwsConfig,
    source: &Resource,
    tx: &mpsc::Sender<SearchUpdate>,
) -> bool {
    let service = source.arn.split(':').nth(2).unwrap_or("");
    if !source.arn.is_empty() && Resource::from_arn(&source.arn).is_none() {
        let _ = tx
            .send(SearchUpdate::Warning("Invalid source ARN".into()))
            .await;
        return false;
    }
    if !source.arn.is_empty() && !account_matches(config, &source.arn) {
        let _ = tx
            .send(SearchUpdate::Warning(format!(
                "Skipping cross-account resource: {}",
                source.arn
            )))
            .await;
        return false;
    }
    if (!source.arn.is_empty() && !region_matches(config, &source.arn, service))
        || (!source.region.is_empty()
            && source.region != "global"
            && source.region != config.region)
    {
        let _ = tx
            .send(SearchUpdate::Warning(format!(
                "Skipping cross-region resource: {}",
                source.arn
            )))
            .await;
        return false;
    }
    true
}

fn has_resolver(kind: ResourceKind) -> bool {
    matches!(
        kind,
        ResourceKind::Function
            | ResourceKind::Queue
            | ResourceKind::Instance
            | ResourceKind::Table
            | ResourceKind::Stack
            | ResourceKind::Efs
            | ResourceKind::Fsx
            | ResourceKind::Database
    )
}

// ── Async push helpers (never drop results) ──────────────────────────────────

/// Push a resource via awaited `send`. Never drops results due to channel pressure.
async fn push(tx: &mpsc::Sender<SearchUpdate>, mut resource: Resource) {
    if resource.arn.is_empty() && resource.id.is_empty() {
        return;
    }
    if resource.arn.split(':').nth(4) == Some("")
        && !matches!(resource.kind, ResourceKind::Bucket | ResourceKind::Api)
    {
        resource.arn.clear();
    }
    let _ = tx.send(SearchUpdate::Results(vec![resource])).await;
}

/// Build a Reference from an ARN string.
fn reference(arn: &str) -> Option<Resource> {
    if arn.is_empty() {
        return None;
    }
    Resource::from_arn(arn)
}

// ── Account/region guards ──────────────────────────────────────────────────────

fn account_matches(config: &AwsConfig, arn: &str) -> bool {
    if config.account_id.is_empty() {
        return true;
    }
    arn.split(':')
        .nth(4)
        .is_some_and(|account| account.is_empty() || account == config.account_id)
}

fn region_matches(config: &AwsConfig, arn: &str, service: &str) -> bool {
    if service == "iam" {
        return true;
    }
    let arn_region = arn.split(':').nth(3).unwrap_or("");
    arn_region.is_empty() || arn_region == config.region
}

// ── Partition-aware ARN construction ──────────────────────────────────────────

fn partition(config: &AwsConfig) -> &'static str {
    if !config.role_arn.is_empty() {
        let p = config.role_arn.split(':').nth(1).unwrap_or("");
        if p == "aws-cn" {
            return "aws-cn";
        } else if p == "aws-us-gov" {
            return "aws-us-gov";
        }
    }
    if config.region.starts_with("cn-") {
        "aws-cn"
    } else if config.region.starts_with("us-gov-") {
        "aws-us-gov"
    } else {
        "aws"
    }
}

fn arn_str(config: &AwsConfig, service: &str, region: &str, resource: &str) -> String {
    format!(
        "arn:{}:{}:{}:{}:{}",
        partition(config),
        service,
        region,
        config.account_id,
        resource
    )
}

fn log_group_arn(config: &AwsConfig, name: &str) -> String {
    arn_str(
        config,
        "logs",
        &config.region,
        &format!("log-group:{}", name),
    )
}

fn vpc_arn(config: &AwsConfig, id: &str) -> Option<Resource> {
    Resource::from_arn(&arn_str(
        config,
        "ec2",
        &config.region,
        &format!("vpc/{}", id),
    ))
}

fn subnet_arn(config: &AwsConfig, id: &str) -> Option<Resource> {
    Resource::from_arn(&arn_str(
        config,
        "ec2",
        &config.region,
        &format!("subnet/{}", id),
    ))
}

fn security_group_arn(config: &AwsConfig, id: &str) -> Option<Resource> {
    Resource::from_arn(&arn_str(
        config,
        "ec2",
        &config.region,
        &format!("security-group/{}", id),
    ))
}

fn volume_arn(config: &AwsConfig, id: &str) -> Option<Resource> {
    Resource::from_arn(&arn_str(
        config,
        "ec2",
        &config.region,
        &format!("volume/{}", id),
    ))
}

fn network_interface_arn(config: &AwsConfig, id: &str) -> Option<Resource> {
    Resource::from_arn(&arn_str(
        config,
        "ec2",
        &config.region,
        &format!("network-interface/{}", id),
    ))
}

// ── Pagination helpers ─────────────────────────────────────────────────────────

/// Report stream truncation without changing the already-emitted rows. All
/// paginated related resolvers share this adapter and the core continuation guard.
async fn next_related_page(
    pagination: &mut Pagination,
    next: Option<String>,
    count: usize,
    tx: &mpsc::Sender<SearchUpdate>,
) -> Result<Option<String>, ()> {
    match pagination.advance_streamed(next, count) {
        Ok(next) => Ok(next),
        Err(reason) => {
            let warning = match reason {
                StopReason::RepeatedToken => {
                    "Repeated pagination token; related results are incomplete".into()
                }
                StopReason::ResultLimit => {
                    format!("Pagination capped at {RESULT_LIMIT} results (may be incomplete)")
                }
                StopReason::PageLimit => {
                    "Pagination page limit reached; related results are incomplete".into()
                }
            };
            let _ = tx.send(SearchUpdate::Warning(warning)).await;
            Err(())
        }
    }
}

// ── JSON extraction using serde_json ──────────────────────────────────────────

fn extract_json_string(json: &str, key: &str) -> Option<String> {
    if json.is_empty() {
        return None;
    }
    serde_json::from_str::<Value>(json)
        .ok()
        .and_then(|v| v.get(key)?.as_str().map(|s| s.to_string()))
}

fn check_malformed_json(json: &str, label: &str) -> Option<String> {
    if json.is_empty() {
        return None;
    }
    if serde_json::from_str::<Value>(json).is_err() {
        Some(format!("{label}: malformed JSON in attribute"))
    } else {
        None
    }
}

// ── CFN physical resource ID mapping ──────────────────────────────────────────

fn cfn_resource(resource_type: &str, physical_id: &str, config: &AwsConfig) -> Option<Resource> {
    if physical_id.is_empty() {
        return None;
    }
    if physical_id.starts_with("arn:") {
        return reference(physical_id);
    }
    match resource_type {
        "AWS::S3::Bucket" => Some(Resource::new(
            ResourceKind::Bucket,
            physical_id,
            physical_id,
            &config.region,
        )),
        "AWS::Lambda::Function" => Some(Resource::new(
            ResourceKind::Function,
            physical_id,
            physical_id,
            &config.region,
        )),
        "AWS::DynamoDB::Table" => Some(Resource::new(
            ResourceKind::Table,
            physical_id,
            physical_id,
            &config.region,
        )),
        "AWS::EC2::Instance" => Some(Resource::new(
            ResourceKind::Instance,
            physical_id,
            physical_id,
            &config.region,
        )),
        "AWS::EC2::VPC" => Some(Resource::new(
            ResourceKind::Vpc,
            physical_id,
            physical_id,
            &config.region,
        )),
        "AWS::IAM::Role" => Some(Resource::new(
            ResourceKind::Role,
            physical_id,
            physical_id,
            "global",
        )),
        "AWS::IAM::User" => Some(Resource::new(
            ResourceKind::User,
            physical_id,
            physical_id,
            "global",
        )),
        "AWS::IAM::Group" => Some(Resource::new(
            ResourceKind::Group,
            physical_id,
            physical_id,
            "global",
        )),
        "AWS::KMS::Key" => Some(Resource::new(
            ResourceKind::Key,
            physical_id,
            physical_id,
            &config.region,
        )),
        "AWS::SQS::Queue" => {
            let name = physical_id.rsplit('/').next().unwrap_or(physical_id);
            Some(Resource::new(
                ResourceKind::Queue,
                name,
                name,
                &config.region,
            ))
        }
        "AWS::EFS::FileSystem" => Some(Resource::new(
            ResourceKind::Efs,
            physical_id,
            physical_id,
            &config.region,
        )),
        "AWS::FSx::FileSystem" => Some(Resource::new(
            ResourceKind::Fsx,
            physical_id,
            physical_id,
            &config.region,
        )),
        "AWS::RDS::DBInstance" => Some(Resource::new(
            ResourceKind::Database,
            physical_id,
            physical_id,
            &config.region,
        )),
        "AWS::Logs::LogGroup" => Some(Resource::new(
            ResourceKind::LogGroup,
            physical_id,
            physical_id,
            &config.region,
        )),
        _ => Some(Resource {
            kind: ResourceKind::Reference,
            id: physical_id.to_string(),
            name: resource_type.to_string(),
            arn: String::new(),
            region: config.region.clone(),
        }),
    }
}

// ── Lambda ──────────────────────────────────────────────────────────────────

async fn lambda_config_inner(
    config: AwsConfig,
    source: Resource,
    tx: mpsc::Sender<SearchUpdate>,
) -> Result<bool> {
    let client = config.lambda_client();
    let resp = client
        .get_function_configuration()
        .function_name(&source.id)
        .send()
        .await?;

    if let Some(role_arn) = resp.role() {
        if let Some(r) = reference(role_arn) {
            push(&tx, r).await;
        }
    }
    if let Some(lc) = resp.logging_config() {
        if let Some(lg) = lc.log_group() {
            if let Some(r) = reference(&log_group_arn(&config, lg)) {
                push(&tx, r).await;
            }
        }
    }
    if let Some(vpc) = resp.vpc_config() {
        for subnet_id in vpc.subnet_ids() {
            if let Some(r) = subnet_arn(&config, subnet_id) {
                push(&tx, r).await;
            }
        }
        for sg_id in vpc.security_group_ids() {
            if let Some(r) = security_group_arn(&config, sg_id) {
                push(&tx, r).await;
            }
        }
        if let Some(vpc_id) = vpc.vpc_id() {
            if let Some(r) = vpc_arn(&config, vpc_id) {
                push(&tx, r).await;
            }
        }
    }
    for layer in resp.layers() {
        if let Some(arn) = layer.arn() {
            if let Some(r) = reference(arn) {
                push(&tx, r).await;
            }
        }
    }
    for fsc in resp.file_system_configs() {
        if let Some(r) = reference(fsc.arn()) {
            push(&tx, r).await;
        }
    }
    if let Some(dlc) = resp.dead_letter_config() {
        if let Some(target) = dlc.target_arn() {
            if let Some(r) = reference(target) {
                push(&tx, r).await;
            }
        }
    }
    if let Some(kms_arn) = resp.kms_key_arn() {
        if let Some(r) = reference(kms_arn) {
            push(&tx, r).await;
        }
    }

    Ok(false)
}

async fn lambda_event_sources_inner(
    config: AwsConfig,
    source: Resource,
    tx: mpsc::Sender<SearchUpdate>,
) -> Result<bool> {
    let client = config.lambda_client();
    let mut marker: Option<String> = None;
    let mut pagination = Pagination::streaming(RESULT_LIMIT);
    let mut count = 0;
    let mut limited = false;
    loop {
        let mut req = client
            .list_event_source_mappings()
            .function_name(&source.id)
            .max_items(100);
        if let Some(m) = &marker {
            req = req.marker(m);
        }
        let resp = req.send().await?;
        for mapping in resp.event_source_mappings() {
            if let Some(arn) = mapping.event_source_arn() {
                if let Some(r) = reference(arn) {
                    push(&tx, r).await;
                }
            }
            count += 1;
        }
        let next = resp.next_marker().map(str::to_owned);
        match next_related_page(&mut pagination, next, count, &tx).await {
            Ok(Some(next)) => marker = Some(next),
            Ok(None) => break,
            Err(()) => {
                limited = true;
                break;
            }
        }
    }
    Ok(limited)
}

// ── SQS ──────────────────────────────────────────────────────────────────────

async fn sqs_attributes_inner(
    config: AwsConfig,
    source: Resource,
    tx: mpsc::Sender<SearchUpdate>,
) -> Result<bool> {
    let client = config.sqs_client();
    let queue_url = client
        .get_queue_url()
        .queue_name(&source.id)
        .send()
        .await?
        .queue_url()
        .filter(|url| !url.is_empty())
        .ok_or_else(|| anyhow!("GetQueueUrl returned no queue URL"))?
        .to_string();

    let attr_resp = client
        .get_queue_attributes()
        .queue_url(&queue_url)
        .attribute_names(QueueAttributeName::QueueArn)
        .attribute_names(QueueAttributeName::RedrivePolicy)
        .attribute_names(QueueAttributeName::KmsMasterKeyId)
        .send()
        .await;

    let mut partial = false;
    let attrs = match attr_resp {
        Ok(response) => response.attributes().cloned().unwrap_or_default(),
        Err(error) => {
            partial = true;
            let _ = tx
                .send(SearchUpdate::Warning(format!("SQS attributes: {error}")))
                .await;
            Default::default()
        }
    };
    if let Some(rp) = attrs.get(&QueueAttributeName::RedrivePolicy) {
        if let Some(warning) = check_malformed_json(rp, "sqs_attributes: RedrivePolicy") {
            partial = true;
            let _ = tx.send(SearchUpdate::Warning(warning)).await;
        }
        if let Some(dlq_arn) = extract_json_string(rp, "deadLetterTargetArn") {
            if let Some(r) = reference(&dlq_arn) {
                push(&tx, r).await;
            }
        }
    }
    if let Some(kms) = attrs.get(&QueueAttributeName::KmsMasterKeyId) {
        if let Some(r) = reference(kms) {
            push(&tx, r).await;
        }
    }

    let queue_arn = attrs
        .get(&QueueAttributeName::QueueArn)
        .cloned()
        .filter(|arn| !arn.is_empty())
        .or_else(|| (!source.arn.is_empty()).then(|| source.arn.clone()));
    let Some(queue_arn) = queue_arn else {
        let _ = tx
            .send(SearchUpdate::Warning(
                "SQS attributes did not provide QueueArn; consumer lookup unavailable".into(),
            ))
            .await;
        return Ok(true);
    };
    if Resource::from_arn(&queue_arn).is_none() {
        return Err(anyhow!("Invalid QueueArn returned by SQS"));
    }
    let client = config.lambda_client();
    let mut marker: Option<String> = None;
    let mut pagination = Pagination::streaming(RESULT_LIMIT);
    let mut count = 0;
    let mut limited = partial;
    loop {
        let mut req = client
            .list_event_source_mappings()
            .event_source_arn(&queue_arn)
            .max_items(100);
        if let Some(m) = &marker {
            req = req.marker(m);
        }
        let resp = req.send().await?;
        for mapping in resp.event_source_mappings() {
            if let Some(fa) = mapping.function_arn() {
                if let Some(r) = reference(fa) {
                    push(&tx, r).await;
                }
            }
            count += 1;
        }
        let next = resp.next_marker().map(str::to_owned);
        match next_related_page(&mut pagination, next, count, &tx).await {
            Ok(Some(next)) => marker = Some(next),
            Ok(None) => break,
            Err(()) => {
                limited = true;
                break;
            }
        }
    }
    Ok(limited)
}

// ── EC2 ──────────────────────────────────────────────────────────────────────

async fn ec2_instance_inner(
    config: AwsConfig,
    source: Resource,
    tx: mpsc::Sender<SearchUpdate>,
) -> Result<bool> {
    let client = config.ec2_client();
    let resp = client
        .describe_instances()
        .instance_ids(source.id.clone())
        .send()
        .await?;

    for reservation in resp.reservations() {
        for instance in reservation.instances() {
            if let Some(vpc_id) = instance.vpc_id() {
                if let Some(r) = vpc_arn(&config, vpc_id) {
                    push(&tx, r).await;
                }
            }
            if let Some(subnet_id) = instance.subnet_id() {
                if let Some(r) = subnet_arn(&config, subnet_id) {
                    push(&tx, r).await;
                }
            }
            for sg in instance.security_groups() {
                if let Some(sg_id) = sg.group_id() {
                    if let Some(r) = security_group_arn(&config, sg_id) {
                        push(&tx, r).await;
                    }
                }
            }
            if let Some(iip) = instance.iam_instance_profile() {
                if let Some(arn) = iip.arn() {
                    if let Some(r) = reference(arn) {
                        push(&tx, r).await;
                    }
                }
            }
            for nic in instance.network_interfaces() {
                if let Some(nic_id) = nic.network_interface_id() {
                    if let Some(r) = network_interface_arn(&config, nic_id) {
                        push(&tx, r).await;
                    }
                }
            }
            for bdm in instance.block_device_mappings() {
                if let Some(ebs) = bdm.ebs() {
                    if let Some(vol_id) = ebs.volume_id() {
                        if let Some(r) = volume_arn(&config, vol_id) {
                            push(&tx, r).await;
                        }
                    }
                }
            }
        }
    }
    Ok(false)
}

// ── DynamoDB ───────────────────────────────────────────────────────────────────

async fn ddb_table_inner(
    config: AwsConfig,
    source: Resource,
    tx: mpsc::Sender<SearchUpdate>,
) -> Result<bool> {
    let client = config.dynamodb_client();
    let resp = client
        .describe_table()
        .table_name(&source.id)
        .send()
        .await?;

    if let Some(table) = resp.table() {
        // KMS key for SSE
        if let Some(sse) = table.sse_description() {
            if let Some(kms_arn) = sse.kms_master_key_arn() {
                if let Some(r) = reference(kms_arn) {
                    push(&tx, r).await;
                }
            }
        }
        // Don't emit self (table ARN == source ARN)
        // Stream references: if table has stream specification enabled, the stream
        // ARN is tableArn/stream/timestamp — but from_arn normalizes table/name/stream/ts
        // to the parent table. We don't emit self, so skip table_arn if it equals source.
    }

    Ok(false)
}

// ── CloudFormation ───────────────────────────────────────────────────────────

async fn cfn_stack_resources_inner(
    config: AwsConfig,
    source: Resource,
    tx: mpsc::Sender<SearchUpdate>,
) -> Result<bool> {
    let client = config.cloudformation_client();
    let mut token: Option<String> = None;
    let mut pagination = Pagination::streaming(RESULT_LIMIT);
    let mut count = 0;
    let mut limited = false;
    loop {
        let mut req = client.list_stack_resources().stack_name(&source.id);
        if let Some(t) = &token {
            req = req.next_token(t);
        }
        let resp = req.send().await?;
        for res in resp.stack_resource_summaries() {
            if let Some(pid) = res.physical_resource_id() {
                if let Some(rtype) = res.resource_type() {
                    if let Some(r) = cfn_resource(rtype, pid, &config) {
                        push(&tx, r).await;
                    }
                } else if let Some(r) = reference(pid) {
                    push(&tx, r).await;
                }
            }
            count += 1;
        }
        let next = resp.next_token().map(str::to_owned);
        match next_related_page(&mut pagination, next, count, &tx).await {
            Ok(Some(next)) => token = Some(next),
            Ok(None) => break,
            Err(()) => {
                limited = true;
                break;
            }
        }
    }
    Ok(limited)
}

// ── EFS ──────────────────────────────────────────────────────────────────────

async fn efs_filesystem_inner(
    config: AwsConfig,
    source: Resource,
    tx: mpsc::Sender<SearchUpdate>,
) -> Result<bool> {
    let client = config.efs_client();
    let resp = client
        .describe_file_systems()
        .file_system_id(&source.id)
        .send()
        .await?;
    for fs in resp.file_systems() {
        if let Some(kms_id) = fs.kms_key_id() {
            if let Some(r) = reference(kms_id) {
                push(&tx, r).await;
            }
        }
    }
    Ok(false)
}

// ── FSx ──────────────────────────────────────────────────────────────────────

async fn fsx_filesystem_inner(
    config: AwsConfig,
    source: Resource,
    tx: mpsc::Sender<SearchUpdate>,
) -> Result<bool> {
    let client = config.fsx_client();
    let resp = client
        .describe_file_systems()
        .file_system_ids(source.id.clone())
        .send()
        .await?;
    for fs in resp.file_systems() {
        if let Some(kms_id) = fs.kms_key_id() {
            if let Some(r) = reference(kms_id) {
                push(&tx, r).await;
            }
        }
        if let Some(vpc_id) = fs.vpc_id() {
            if let Some(r) = vpc_arn(&config, vpc_id) {
                push(&tx, r).await;
            }
        }
        for subnet_id in fs.subnet_ids() {
            if let Some(r) = subnet_arn(&config, subnet_id) {
                push(&tx, r).await;
            }
        }
        for nic_id in fs.network_interface_ids() {
            if let Some(r) = network_interface_arn(&config, nic_id) {
                push(&tx, r).await;
            }
        }
    }
    Ok(false)
}

// ── RDS ───────────────────────────────────────────────────────────────────────

async fn rds_instance_inner(
    config: AwsConfig,
    source: Resource,
    tx: mpsc::Sender<SearchUpdate>,
) -> Result<bool> {
    let client = config.rds_client();
    let resp = client
        .describe_db_instances()
        .db_instance_identifier(&source.id)
        .send()
        .await?;
    for db in resp.db_instances() {
        if let Some(kms_id) = db.kms_key_id() {
            if let Some(r) = reference(kms_id) {
                push(&tx, r).await;
            }
        }
        // Enhanced monitoring role ARN (this is the IAM role)
        if let Some(role_arn) = db.monitoring_role_arn() {
            if let Some(r) = reference(role_arn) {
                push(&tx, r).await;
            }
        }
        // Enhanced monitoring resource ARN (this is a CloudWatch log stream ARN)
        if let Some(em_arn) = db.enhanced_monitoring_resource_arn() {
            if let Some(r) = reference(em_arn) {
                push(&tx, r).await;
            }
        }
        // DB subnet group → VPC + subnets
        if let Some(dsg) = db.db_subnet_group() {
            if let Some(vpc_id) = dsg.vpc_id() {
                if let Some(r) = vpc_arn(&config, vpc_id) {
                    push(&tx, r).await;
                }
            }
            for subnet in dsg.subnets() {
                if let Some(subnet_id) = subnet.subnet_identifier() {
                    if let Some(r) = subnet_arn(&config, subnet_id) {
                        push(&tx, r).await;
                    }
                }
            }
        }
        // VPC security groups
        for sg in db.vpc_security_groups() {
            if let Some(sg_id) = sg.vpc_security_group_id() {
                if let Some(r) = security_group_arn(&config, sg_id) {
                    push(&tx, r).await;
                }
            }
        }
    }
    Ok(false)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
