use super::{Resource, ResourceKind};
use crate::config::AwsConfig;
use anyhow::Result;

// Per-source bound limits cache size and control-plane reads. UI reports partial results.
const INVENTORY_LIMIT: usize = 2000;

pub(super) async fn fetch(config: &AwsConfig, kind: ResourceKind) -> Result<(Vec<Resource>, bool)> {
    let mut rows = Vec::new();
    let region = config.region.as_str();
    // SDK item streams handle page tokens, including duplicate-token protection.
    macro_rules! collect {
        ($request:expr, $item:ident, $row:expr) => {{
            let mut stream = $request.into_paginator().items().send();
            while let Some(item) = stream.next().await {
                let $item = item?;
                rows.push($row);
                if rows.len() >= INVENTORY_LIMIT {
                    return Ok((rows, true));
                }
            }
        }};
    }
    match kind {
        ResourceKind::Vpc => collect!(
            config.ec2_client().describe_vpcs(),
            vpc,
            Resource::new(
                kind,
                vpc.vpc_id().unwrap_or_default(),
                vpc.tags()
                    .iter()
                    .find(|tag| tag.key() == Some("Name"))
                    .and_then(|tag| tag.value())
                    .unwrap_or_default(),
                region
            )
        ),
        ResourceKind::Table => collect!(
            config.dynamodb_client().list_tables(),
            table,
            Resource::new(kind, table.clone(), table, region)
        ),
        ResourceKind::Bucket => collect!(
            config.s3_client().list_buckets(),
            bucket,
            Resource::new(
                kind,
                bucket.name().unwrap_or_default(),
                bucket.name().unwrap_or_default(),
                bucket.bucket_region().unwrap_or("global")
            )
        ),
        ResourceKind::Queue => collect!(
            config.sqs_client().list_queues().max_results(1000),
            url,
            Resource::new(
                kind,
                url.rsplit('/').next().unwrap_or_default(),
                &url,
                region
            )
        ),
        ResourceKind::Database => collect!(
            config.rds_client().describe_db_instances(),
            db,
            Resource::new(
                kind,
                db.db_instance_identifier().unwrap_or_default(),
                db.db_instance_identifier().unwrap_or_default(),
                region
            )
        ),
        ResourceKind::Function => collect!(
            config.lambda_client().list_functions(),
            function,
            Resource::new(
                kind,
                function.function_name().unwrap_or_default(),
                function.function_name().unwrap_or_default(),
                region
            )
        ),
        ResourceKind::Stack => collect!(
            config.cloudformation_client().describe_stacks(),
            stack,
            Resource::new(
                kind,
                stack.stack_name().unwrap_or_default(),
                stack.stack_name().unwrap_or_default(),
                region
            )
        ),
        ResourceKind::LogGroup => collect!(
            config.cloudwatch_logs_client().describe_log_groups(),
            group,
            Resource::new(
                kind,
                group.log_group_name().unwrap_or_default(),
                group.log_group_name().unwrap_or_default(),
                region
            )
        ),
        ResourceKind::User => collect!(
            config.iam_client().list_users(),
            user,
            Resource::new(kind, user.user_name(), user.user_name(), "global")
        ),
        ResourceKind::Role => collect!(
            config.iam_client().list_roles(),
            role,
            Resource::new(kind, role.role_name(), role.role_name(), "global")
        ),
        ResourceKind::Group => collect!(
            config.iam_client().list_groups(),
            group,
            Resource::new(kind, group.group_name(), group.group_name(), "global")
        ),
        ResourceKind::Repository => collect!(
            config.ecr_client().describe_repositories(),
            repo,
            Resource::new(
                kind,
                repo.repository_name().unwrap_or_default(),
                repo.repository_name().unwrap_or_default(),
                region
            )
        ),
        ResourceKind::PublicRepository => collect!(
            config.ecr_public_client().describe_repositories(),
            repo,
            Resource::new(
                kind,
                repo.repository_name().unwrap_or_default(),
                repo.repository_name().unwrap_or_default(),
                "us-east-1"
            )
        ),
        ResourceKind::Efs => collect!(
            config.efs_client().describe_file_systems(),
            fs,
            Resource::new(
                kind,
                fs.file_system_id(),
                fs.name().unwrap_or(fs.file_system_id()),
                region
            )
        ),
        ResourceKind::Fsx => {
            let mut pages = config
                .fsx_client()
                .describe_file_systems()
                .into_paginator()
                .send();
            while let Some(page) = pages.next().await {
                for fs in page?.file_systems() {
                    rows.push(Resource::new(
                        kind,
                        fs.file_system_id().unwrap_or_default(),
                        fs.tags()
                            .iter()
                            .find(|t| t.key() == Some("Name"))
                            .and_then(|t| t.value())
                            .unwrap_or_default(),
                        region,
                    ));
                    if rows.len() >= INVENTORY_LIMIT {
                        return Ok((rows, true));
                    }
                }
            }
        }
        ResourceKind::Key => {
            collect!(
                config.kms_client().list_keys(),
                key,
                Resource::new(
                    kind,
                    key.key_id().unwrap_or_default(),
                    key.key_id().unwrap_or_default(),
                    region
                )
            );
            let mut aliases = config
                .kms_client()
                .list_aliases()
                .into_paginator()
                .items()
                .send();
            let mut count = 0;
            while let Some(alias) = aliases.next().await {
                let alias = alias?;
                if let Some(row) = rows
                    .iter_mut()
                    .find(|r| Some(r.id.as_str()) == alias.target_key_id())
                {
                    row.name.push(' ');
                    row.name.push_str(alias.alias_name().unwrap_or_default());
                }
                count += 1;
                if count >= INVENTORY_LIMIT {
                    return Ok((rows, true));
                }
            }
        }
        ResourceKind::Instance => {
            let mut pages = config
                .ec2_client()
                .describe_instances()
                .into_paginator()
                .send();
            while let Some(page) = pages.next().await {
                for instance in page?.reservations().iter().flat_map(|r| r.instances()) {
                    let name = instance
                        .tags()
                        .iter()
                        .find(|t| t.key() == Some("Name"))
                        .and_then(|t| t.value())
                        .unwrap_or_default();
                    rows.push(Resource::new(
                        kind,
                        instance.instance_id().unwrap_or_default(),
                        name,
                        region,
                    ));
                    if rows.len() >= INVENTORY_LIMIT {
                        return Ok((rows, true));
                    }
                }
            }
        }
        ResourceKind::Alarm => {
            use aws_sdk_cloudwatch::types::AlarmType;
            let mut pages = config
                .cloudwatch_client()
                .describe_alarms()
                .alarm_types(AlarmType::MetricAlarm)
                .alarm_types(AlarmType::CompositeAlarm)
                .into_paginator()
                .send();
            while let Some(page) = pages.next().await {
                let page = page?;
                for name in page
                    .metric_alarms()
                    .iter()
                    .filter_map(|a| a.alarm_name())
                    .chain(
                        page.composite_alarms()
                            .iter()
                            .filter_map(|a| a.alarm_name()),
                    )
                {
                    rows.push(Resource::new(kind, name, name, region));
                    if rows.len() >= INVENTORY_LIMIT {
                        return Ok((rows, true));
                    }
                }
            }
        }
        ResourceKind::Api => {
            collect!(
                config.apigateway_client().get_rest_apis(),
                api,
                Resource::new(
                    kind,
                    api.id().unwrap_or_default(),
                    api.name().unwrap_or_default(),
                    region
                )
            );
            // API Gateway v2 has no generated paginator.
            let mut token = None;
            loop {
                let page = config
                    .apigatewayv2_client()
                    .get_apis()
                    .set_next_token(token.clone())
                    .send()
                    .await?;
                for api in page.items() {
                    rows.push(Resource::new(
                        kind,
                        api.api_id().unwrap_or_default(),
                        api.name().unwrap_or_default(),
                        region,
                    ));
                    if rows.len() >= INVENTORY_LIMIT {
                        return Ok((rows, true));
                    }
                }
                let next = page.next_token().map(str::to_owned);
                if next.is_none() {
                    break;
                }
                if next == token {
                    return Ok((rows, true));
                }
                token = next;
            }
        }
    }
    rows.retain(|r| !r.id.is_empty());
    Ok((rows, false))
}
