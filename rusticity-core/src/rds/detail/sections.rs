//! Per-tab reads beyond DescribeDBInstances: Monitoring, Logs & events,
//! Zero-ETL integrations, Data migrations and Recommendations. They run in parallel after the
//! instance read (Data migrations reads AWS DMS), each bounded by its own timeout. A failed or slow read marks
//! only its own section unavailable and never fails the detail. Read-only.
use super::ENRICH_TIMEOUT;
use crate::config::AwsConfig;
use crate::metrics::{MetricQuery, MetricReader};
use anyhow::{anyhow, Result};
use aws_sdk_cloudwatch::types::{ComparisonOperator, MetricAlarm, Statistic};
use aws_sdk_rds::types::{DbRecommendation, Filter, SourceType};
use std::future::Future;

pub use super::connectivity::{RdsComputeResource, RdsIamRole, RdsReplica, RdsSecurityGroupRule};
pub use super::migrations::RdsDataMigration;

/// One section of a detail tab: its rows, or why they are unavailable.
pub type Section<T> = Result<Vec<T>, String>;

/// One CloudWatch chart of the Monitoring tab (last 3 hours, 5-minute period).
#[derive(Clone, Debug, PartialEq)]
pub struct RdsMetric {
    pub name: &'static str,
    pub unit: &'static str,
    pub points: Vec<(i64, f64)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RdsAlarm {
    pub name: String,
    pub state: String,
    pub condition: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RdsEvent {
    pub time: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RdsLogFile {
    pub name: String,
    /// ISO 8601; empty when absent.
    pub last_written: String,
    pub size: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RdsIntegration {
    pub name: String,
    pub created_time: String,
    pub arn: String,
    pub status: String,
    /// Source resource name: the last ARN segment.
    pub source: String,
    pub source_arn: String,
    pub destination_account: String,
    pub destination_type: String,
    pub destination_arn: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RdsRecommendation {
    pub severity: String,
    pub detection: String,
    pub recommendation: String,
    pub impact: String,
    pub category: String,
    pub status: String,
    pub created_time: String,
    pub updated_time: String,
    /// When the issue ended; empty for a continuing issue.
    pub end_time: String,
    /// Last segment of the resource ARN.
    pub resource_identifier: String,
}

/// Console Monitoring charts, in console order: (metric, unit).
pub const METRICS: &[(&str, &str)] = &[
    ("CPUUtilization", "Percent"),
    ("DatabaseConnections", "Count"),
    ("FreeStorageSpace", "Bytes"),
    ("FreeableMemory", "Bytes"),
    ("WriteIOPS", "Count/Second"),
    ("ReadIOPS", "Count/Second"),
    ("WriteLatency", "Seconds"),
    ("ReadLatency", "Seconds"),
    ("WriteThroughput", "Bytes/Second"),
    ("ReadThroughput", "Bytes/Second"),
    ("DiskQueueDepth", "Count"),
    ("NetworkReceiveThroughput", "Bytes/Second"),
    ("NetworkTransmitThroughput", "Bytes/Second"),
    ("SwapUsage", "Bytes"),
];

/// Recent events window, in minutes (7 days; the API keeps 14).
pub(crate) const EVENT_MINUTES: i32 = 7 * 24 * 60;

#[derive(Clone, Debug, PartialEq)]
pub struct Sections {
    pub compute: Section<RdsComputeResource>,
    pub security_group_rules: Section<RdsSecurityGroupRule>,
    pub replication: Section<RdsReplica>,
    pub monitoring: Section<RdsMetric>,
    pub alarms: Section<RdsAlarm>,
    pub events: Section<RdsEvent>,
    pub logs: Section<RdsLogFile>,
    pub integrations: Section<RdsIntegration>,
    pub migrations: Section<RdsDataMigration>,
    pub recommendations: Section<RdsRecommendation>,
}

impl Default for Sections {
    fn default() -> Self {
        Self {
            compute: Ok(Vec::new()),
            security_group_rules: Ok(Vec::new()),
            replication: Ok(Vec::new()),
            monitoring: Ok(Vec::new()),
            alarms: Ok(Vec::new()),
            events: Ok(Vec::new()),
            logs: Ok(Vec::new()),
            integrations: Ok(Vec::new()),
            migrations: Ok(Vec::new()),
            recommendations: Ok(Vec::new()),
        }
    }
}

pub(super) async fn bounded<T>(
    what: &str,
    read: impl Future<Output = Result<Vec<T>>>,
) -> Section<T> {
    match tokio::time::timeout(ENRICH_TIMEOUT, read).await {
        Ok(Ok(rows)) => Ok(rows),
        Ok(Err(error)) => {
            tracing::warn!("RDS {what} unavailable: {error:#}");
            Err(format!("{what} unavailable: {error:#}"))
        }
        Err(_) => Err(format!(
            "{what} unavailable: timed out after {ENRICH_TIMEOUT:?}"
        )),
    }
}

pub(super) async fn read(
    config: &AwsConfig,
    identifier: &str,
    arn: &str,
    resource_id: &str,
    endpoint: &str,
) -> Sections {
    let (monitoring, alarms, events, logs, integrations, migrations, recommendations) = tokio::join!(
        bounded("CloudWatch metrics", monitoring(config, identifier)),
        bounded("CloudWatch alarms", alarms(config, identifier)),
        bounded("Recent events", events(config, identifier)),
        bounded("Logs", logs(config, identifier)),
        bounded("Zero-ETL integrations", integrations(config, arn)),
        bounded("Data migrations", super::migrations::read(config, endpoint)),
        bounded("Recommendations", recommendations(config, arn, resource_id)),
    );
    Sections {
        compute: Ok(Vec::new()),
        security_group_rules: Ok(Vec::new()),
        replication: Ok(Vec::new()),
        monitoring,
        alarms,
        events,
        logs,
        integrations,
        migrations,
        recommendations,
    }
}

async fn monitoring(config: &AwsConfig, identifier: &str) -> Result<Vec<RdsMetric>> {
    let reader = MetricReader::new(config.cloudwatch_client());
    let reads = METRICS.iter().map(|(name, unit)| {
        let reader = &reader;
        async move {
            let query = MetricQuery::new("AWS/RDS", name, Statistic::Average)
                .dimension("DBInstanceIdentifier", identifier);
            Ok::<_, anyhow::Error>(RdsMetric {
                name,
                unit,
                points: reader.recent(query, 300).await?,
            })
        }
    });
    futures::future::try_join_all(reads).await
}

async fn alarms(config: &AwsConfig, identifier: &str) -> Result<Vec<RdsAlarm>> {
    let all = config
        .cloudwatch_client()
        .describe_alarms()
        .into_paginator()
        .send()
        .collect::<Result<Vec<_>, _>>()
        .await?;
    let mut rows: Vec<_> = all
        .iter()
        .flat_map(|page| page.metric_alarms())
        .filter(|a| watches(a, identifier))
        .map(alarm)
        .collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(rows)
}

/// An alarm on an AWS/RDS metric for exactly this instance.
pub(super) fn watches(alarm: &MetricAlarm, identifier: &str) -> bool {
    alarm.namespace() == Some("AWS/RDS")
        && alarm
            .dimensions()
            .iter()
            .any(|d| d.name() == Some("DBInstanceIdentifier") && d.value() == Some(identifier))
}

pub(super) fn alarm(a: &MetricAlarm) -> RdsAlarm {
    let op = match a.comparison_operator() {
        Some(ComparisonOperator::GreaterThanThreshold) => ">",
        Some(ComparisonOperator::GreaterThanOrEqualToThreshold) => ">=",
        Some(ComparisonOperator::LessThanThreshold) => "<",
        Some(ComparisonOperator::LessThanOrEqualToThreshold) => "<=",
        Some(other) => other.as_str(),
        None => "",
    };
    let condition = match (a.metric_name(), a.threshold()) {
        (Some(metric), Some(threshold)) => format!("{metric} {op} {threshold}"),
        (Some(metric), None) => metric.to_string(),
        _ => String::new(),
    };
    RdsAlarm {
        name: a.alarm_name().unwrap_or_default().into(),
        state: a
            .state_value()
            .map(|s| s.as_str().to_string())
            .unwrap_or_default(),
        condition,
    }
}

async fn events(config: &AwsConfig, identifier: &str) -> Result<Vec<RdsEvent>> {
    let events = config
        .rds_client()
        .describe_events()
        .source_type(SourceType::DbInstance)
        .source_identifier(identifier)
        .duration(EVENT_MINUTES)
        .into_paginator()
        .items()
        .send()
        .collect::<Result<Vec<_>, _>>()
        .await?;
    let mut rows: Vec<_> = events
        .iter()
        .filter(|e| e.source_identifier() == Some(identifier))
        .map(|e| RdsEvent {
            time: super::timestamp(e.date()),
            message: e.message().unwrap_or_default().into(),
        })
        .collect();
    // Newest first, as in the console.
    rows.sort_by(|a, b| b.time.cmp(&a.time));
    Ok(rows)
}

async fn logs(config: &AwsConfig, identifier: &str) -> Result<Vec<RdsLogFile>> {
    let files = config
        .rds_client()
        .describe_db_log_files()
        .db_instance_identifier(identifier)
        .into_paginator()
        .items()
        .send()
        .collect::<Result<Vec<_>, _>>()
        .await?;
    let mut rows: Vec<_> = files
        .iter()
        .map(|f| RdsLogFile {
            name: f.log_file_name().unwrap_or_default().into(),
            last_written: f
                .last_written()
                .map(|ms| super::timestamp(Some(&aws_smithy_types::DateTime::from_millis(ms))))
                .unwrap_or_default(),
            size: f.size(),
        })
        .collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(rows)
}

async fn integrations(config: &AwsConfig, arn: &str) -> Result<Vec<RdsIntegration>> {
    let all = config
        .rds_client()
        .describe_integrations()
        .into_paginator()
        .items()
        .send()
        .collect::<Result<Vec<_>, _>>()
        .await?;
    let mut rows: Vec<_> = all
        .iter()
        .filter(|i| i.source_arn() == Some(arn))
        .map(integration)
        .collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(rows)
}

pub(super) fn integration(i: &aws_sdk_rds::types::Integration) -> RdsIntegration {
    let source_arn = i.source_arn().unwrap_or_default().to_string();
    let destination_arn = i.target_arn().unwrap_or_default().to_string();
    let parts: Vec<&str> = destination_arn.splitn(6, ':').collect();
    let field = |n: usize| parts.get(n).copied().unwrap_or_default();
    RdsIntegration {
        name: i.integration_name().unwrap_or_default().into(),
        created_time: super::timestamp(i.create_time()),
        arn: i.integration_arn().unwrap_or_default().into(),
        status: i
            .status()
            .map(|s| s.as_str().to_string())
            .unwrap_or_default(),
        source: source_arn.rsplit(':').next().unwrap_or_default().into(),
        source_arn,
        destination_account: field(4).into(),
        destination_type: destination_type(field(2)),
        destination_arn,
    }
}

/// Console names for the destination service of an integration ARN.
pub(crate) fn destination_type(service: &str) -> String {
    match service {
        "redshift" => "Amazon Redshift",
        "redshift-serverless" => "Amazon Redshift Serverless",
        "glue" => "Amazon SageMaker lakehouse",
        other => other,
    }
    .into()
}

async fn recommendations(
    config: &AwsConfig,
    arn: &str,
    resource_id: &str,
) -> Result<Vec<RdsRecommendation>> {
    if resource_id.is_empty() {
        return Err(anyhow!("the instance has no resource ID"));
    }
    let all = config
        .rds_client()
        .describe_db_recommendations()
        .filters(
            Filter::builder()
                .name("dbi-resource-id")
                .values(resource_id)
                .build(),
        )
        .into_paginator()
        .items()
        .send()
        .collect::<Result<Vec<_>, _>>()
        .await?;
    Ok(own_recommendations(&all, arn))
}

/// Only this instance's rows: the filter is trusted, the ARN is verified.
pub(super) fn own_recommendations(rows: &[DbRecommendation], arn: &str) -> Vec<RdsRecommendation> {
    let text = |v: Option<&str>| v.unwrap_or_default().to_string();
    let mut rows: Vec<_> = rows
        .iter()
        .filter(|r| r.resource_arn() == Some(arn))
        .map(|r| RdsRecommendation {
            severity: text(r.severity()),
            detection: text(r.detection()),
            recommendation: text(r.recommendation()),
            impact: text(r.impact()),
            category: text(r.category()),
            status: text(r.status()),
            created_time: super::timestamp(r.created_time()),
            updated_time: super::timestamp(r.updated_time()),
            end_time: super::timestamp(
                r.issue_details()
                    .and_then(|i| i.performance_issue_details())
                    .and_then(|p| p.end_time()),
            ),
            resource_identifier: text(r.resource_arn())
                .rsplit(':')
                .next()
                .unwrap_or_default()
                .into(),
        })
        .collect();
    rows.sort_by(|a, b| b.updated_time.cmp(&a.updated_time));
    rows
}

/// The Summary count: active recommendations, or "Unavailable".
pub(super) fn active_count(section: &Section<RdsRecommendation>) -> String {
    match section {
        Ok(rows) => rows
            .iter()
            .filter(|r| r.status.eq_ignore_ascii_case("active"))
            .count()
            .to_string(),
        Err(_) => "Unavailable".into(),
    }
}
