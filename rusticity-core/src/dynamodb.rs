use crate::config::AwsConfig;
use anyhow::Result;
use futures::stream::{self, StreamExt};

/// How many DescribeTable calls to run concurrently while listing tables.
const DESCRIBE_CONCURRENCY: usize = 10;

/// A DynamoDB table, flattened for display.
#[derive(Clone, Debug, Default)]
pub struct DdbTable {
    pub name: String,
    pub table_arn: String,
    pub table_id: String,
    /// Table status (ACTIVE, CREATING, ...).
    pub status: String,
    /// Partition (HASH) key, e.g. "id (S)".
    pub partition_key: String,
    /// Sort (RANGE) key, or empty when the table has none.
    pub sort_key: String,
    /// "On-demand" or "Provisioned".
    pub read_capacity_mode: String,
    pub write_capacity_mode: String,
    /// Provisioned units (0 for on-demand tables).
    pub provisioned_read_units: i64,
    pub provisioned_write_units: i64,
    /// "Standard" or "Standard-IA".
    pub table_class: String,
    /// "Enabled" / "Disabled".
    pub deletion_protection: String,
    pub total_size_bytes: i64,
    pub item_count: i64,
    pub creation_time: String,
    /// Server-side encryption type ("Owned by Amazon DynamoDB", "AWS managed key", ...).
    pub encryption_type: String,
    pub kms_key_arn: String,
    /// "Enabled" / "Disabled" for DynamoDB Streams.
    pub stream_enabled: String,
    pub stream_view_type: String,
    pub latest_stream_arn: String,
    pub global_secondary_indexes: Vec<DdbIndex>,
    pub local_secondary_indexes: Vec<DdbIndex>,
    pub replicas: Vec<DdbReplica>,
}

impl DdbTable {
    /// Total number of secondary indexes (global + local).
    pub fn index_count(&self) -> usize {
        self.global_secondary_indexes.len() + self.local_secondary_indexes.len()
    }

    /// Comma-separated global-table replica regions ("" when not a global table).
    pub fn replication_regions(&self) -> String {
        self.replicas
            .iter()
            .map(|r| r.region.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// A secondary index (global or local) on a table.
#[derive(Clone, Debug, Default)]
pub struct DdbIndex {
    pub name: String,
    /// "Global" or "Local".
    pub index_type: String,
    pub status: String,
    pub partition_key: String,
    pub sort_key: String,
    /// "ALL", "KEYS_ONLY" or "INCLUDE: a, b".
    pub projection: String,
    pub read_capacity_mode: String,
    pub write_capacity_mode: String,
    pub size_bytes: i64,
    pub item_count: i64,
    pub index_arn: String,
}

/// A global-table replica.
#[derive(Clone, Debug, Default)]
pub struct DdbReplica {
    pub region: String,
    pub status: String,
    pub status_description: String,
    pub table_class: String,
    pub kms_key_id: String,
    pub index_count: usize,
}

/// Point-in-time recovery / TTL settings loaded on demand (Additional settings tab).
#[derive(Clone, Debug, Default)]
pub struct DdbTableSettings {
    /// "Enabled" / "Disabled".
    pub ttl_status: String,
    pub ttl_attribute: String,
    /// "Enabled" / "Disabled".
    pub pitr_status: String,
    /// Backup recovery period in days (0 when unknown/disabled).
    pub recovery_period_days: i32,
    pub earliest_restorable_time: String,
    pub latest_restorable_time: String,
}

/// An on-demand backup of a table (Backups tab).
#[derive(Clone, Debug, Default)]
pub struct DdbBackup {
    pub name: String,
    pub backup_arn: String,
    pub status: String,
    /// USER / SYSTEM / AWS_BACKUP.
    pub backup_type: String,
    pub size_bytes: i64,
    pub creation_time: String,
    pub expiry_time: String,
}

pub struct DynamoDbClient {
    config: AwsConfig,
}

impl DynamoDbClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    /// List every table in the region, enriched with DescribeTable details.
    pub async fn list_tables(&self) -> Result<Vec<DdbTable>> {
        let client = self.config.dynamodb_client();

        // 1. Collect the table names (paginated).
        let mut names: Vec<String> = Vec::new();
        let mut start: Option<String> = None;
        loop {
            let mut req = client.list_tables().limit(100);
            if let Some(s) = start {
                req = req.exclusive_start_table_name(s);
            }
            let resp = req.send().await?;
            names.extend(resp.table_names().iter().cloned());
            start = resp.last_evaluated_table_name().map(|s| s.to_string());
            if start.is_none() {
                break;
            }
        }

        // 2. Describe them concurrently — DynamoDB has no bulk describe API.
        let mut tables: Vec<DdbTable> = stream::iter(names)
            .map(|name| {
                let client = client.clone();
                async move {
                    let resp = client
                        .describe_table()
                        .table_name(&name)
                        .send()
                        .await
                        .ok()?;
                    resp.table().map(table_from_description)
                }
            })
            .buffer_unordered(DESCRIBE_CONCURRENCY)
            .filter_map(|t| async move { t })
            .collect()
            .await;

        tables.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(tables)
    }

    /// TTL and point-in-time-recovery settings for a table.
    pub async fn describe_settings(&self, table_name: &str) -> Result<DdbTableSettings> {
        let client = self.config.dynamodb_client();
        let mut settings = DdbTableSettings::default();

        if let Ok(resp) = client
            .describe_time_to_live()
            .table_name(table_name)
            .send()
            .await
        {
            if let Some(ttl) = resp.time_to_live_description() {
                settings.ttl_status = match ttl.time_to_live_status() {
                    Some(s) => enabled_disabled(s.as_str()),
                    None => "Disabled".to_string(),
                };
                settings.ttl_attribute = ttl.attribute_name().unwrap_or("").to_string();
            }
        }

        if let Ok(resp) = client
            .describe_continuous_backups()
            .table_name(table_name)
            .send()
            .await
        {
            if let Some(cb) = resp.continuous_backups_description() {
                if let Some(pitr) = cb.point_in_time_recovery_description() {
                    settings.pitr_status = match pitr.point_in_time_recovery_status() {
                        Some(s) => enabled_disabled(s.as_str()),
                        None => "Disabled".to_string(),
                    };
                    settings.earliest_restorable_time =
                        fmt_time(pitr.earliest_restorable_date_time());
                    settings.latest_restorable_time = fmt_time(pitr.latest_restorable_date_time());
                    settings.recovery_period_days = pitr.recovery_period_in_days().unwrap_or(0);
                }
            }
        }

        Ok(settings)
    }

    /// Resource-based policy attached to a table or stream ARN, pretty-printed.
    /// Returns `None` when no policy exists.
    pub async fn get_resource_policy(&self, resource_arn: &str) -> Result<Option<String>> {
        let client = self.config.dynamodb_client();
        match client
            .get_resource_policy()
            .resource_arn(resource_arn)
            .send()
            .await
        {
            Ok(resp) => Ok(resp.policy().map(pretty_json)),
            // A missing policy is the common case, not an error.
            Err(_) => Ok(None),
        }
    }

    /// Tags on a table (Tags tab).
    pub async fn list_tags(&self, table_arn: &str) -> Result<Vec<(String, String)>> {
        let client = self.config.dynamodb_client();
        let mut out: Vec<(String, String)> = Vec::new();
        let mut next: Option<String> = None;
        loop {
            let mut req = client.list_tags_of_resource().resource_arn(table_arn);
            if let Some(t) = next {
                req = req.next_token(t);
            }
            let resp = req.send().await?;
            for tag in resp.tags() {
                out.push((tag.key().to_string(), tag.value().to_string()));
            }
            next = resp.next_token().map(|s| s.to_string());
            if next.is_none() {
                break;
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    /// On-demand backups of a table, newest first (Backups tab).
    pub async fn list_backups(&self, table_name: &str) -> Result<Vec<DdbBackup>> {
        let client = self.config.dynamodb_client();
        let mut out: Vec<DdbBackup> = Vec::new();
        let mut start_arn: Option<String> = None;

        loop {
            let mut req = client.list_backups().table_name(table_name);
            if let Some(a) = start_arn {
                req = req.exclusive_start_backup_arn(a);
            }
            let resp = req.send().await?;

            for b in resp.backup_summaries() {
                out.push(DdbBackup {
                    name: b.backup_name().unwrap_or("").to_string(),
                    backup_arn: b.backup_arn().unwrap_or("").to_string(),
                    status: b
                        .backup_status()
                        .map(|s| s.as_str().to_string())
                        .unwrap_or_default(),
                    backup_type: b
                        .backup_type()
                        .map(|t| t.as_str().to_string())
                        .unwrap_or_default(),
                    size_bytes: b.backup_size_bytes().unwrap_or(0),
                    creation_time: fmt_time(b.backup_creation_date_time()),
                    expiry_time: fmt_time(b.backup_expiry_date_time()),
                });
            }

            start_arn = resp.last_evaluated_backup_arn().map(|s| s.to_string());
            if start_arn.is_none() {
                break;
            }
        }

        out.sort_by(|a, b| b.creation_time.cmp(&a.creation_time));
        Ok(out)
    }

    /// Scan items from a table or one of its indexes (Items tab).
    pub async fn scan_items(
        &self,
        table_name: &str,
        index_name: Option<&str>,
        limit: i32,
        projected_attributes: Option<&[String]>,
    ) -> Result<Vec<DdbItem>> {
        let client = self.config.dynamodb_client();
        let mut req = client.scan().table_name(table_name).limit(limit);
        if let Some(index) = index_name {
            req = req.index_name(index);
        }
        if let Some(attrs) = projected_attributes {
            let (expression, names) = projection_expression(attrs);
            req = req.projection_expression(expression);
            for (placeholder, name) in names {
                req = req.expression_attribute_names(placeholder, name);
            }
        }
        let resp = req.send().await?;
        Ok(resp.items().iter().map(item_from_attributes).collect())
    }

    /// Query items by partition-key value from a table or one of its indexes.
    #[allow(clippy::too_many_arguments)]
    pub async fn query_items(
        &self,
        table_name: &str,
        index_name: Option<&str>,
        partition_key: &str,
        partition_value: &str,
        partition_type: &str,
        limit: i32,
        projected_attributes: Option<&[String]>,
    ) -> Result<Vec<DdbItem>> {
        use aws_sdk_dynamodb::types::AttributeValue;

        let client = self.config.dynamodb_client();
        let value = match partition_type {
            "N" => AttributeValue::N(partition_value.to_string()),
            "B" => AttributeValue::S(partition_value.to_string()),
            _ => AttributeValue::S(partition_value.to_string()),
        };

        let mut req = client
            .query()
            .table_name(table_name)
            .limit(limit)
            .key_condition_expression("#pk = :pk")
            .expression_attribute_names("#pk", partition_key)
            .expression_attribute_values(":pk", value);
        if let Some(index) = index_name {
            req = req.index_name(index);
        }
        if let Some(attrs) = projected_attributes {
            let (expression, names) = projection_expression(attrs);
            req = req.projection_expression(expression);
            for (placeholder, name) in names {
                req = req.expression_attribute_names(placeholder, name);
            }
        }
        let resp = req.send().await?;
        Ok(resp.items().iter().map(item_from_attributes).collect())
    }

    /// CloudWatch time series for the Monitor tab (last 3 hours, 1-minute period).
    pub async fn get_monitoring_metrics(&self, table_name: &str) -> Result<DdbMonitoringData> {
        let now = chrono::Utc::now();
        let start = now - chrono::Duration::hours(3);

        let mut data = DdbMonitoringData::default();

        // Consumed capacity is emitted as a Sum per period; divide by the period
        // to get the capacity units per second the console shows.
        {
            let queries = vec![
                ddb_expr_query("e1", "m1/PERIOD(m1)", true),
                ddb_expr_query("e2", "m2/PERIOD(m2)", true),
                ddb_metric_query(
                    "m1",
                    "ConsumedReadCapacityUnits",
                    "Sum",
                    table_name,
                    None,
                    false,
                ),
                ddb_metric_query(
                    "m2",
                    "ConsumedWriteCapacityUnits",
                    "Sum",
                    table_name,
                    None,
                    false,
                ),
            ];
            let mut res = self.run_get_metric_data(queries, start, now).await?;
            data.consumed_read_capacity = res.remove("e1").unwrap_or_default();
            data.consumed_write_capacity = res.remove("e2").unwrap_or_default();
        }

        // Throttling.
        {
            let queries = vec![
                ddb_metric_query("m1", "ReadThrottleEvents", "Sum", table_name, None, true),
                ddb_metric_query("m2", "WriteThrottleEvents", "Sum", table_name, None, true),
                ddb_metric_query("m3", "ThrottledRequests", "Sum", table_name, None, true),
            ];
            let mut res = self.run_get_metric_data(queries, start, now).await?;
            data.read_throttle_events = res.remove("m1").unwrap_or_default();
            data.write_throttle_events = res.remove("m2").unwrap_or_default();
            data.throttled_requests = res.remove("m3").unwrap_or_default();
        }

        // Latency per operation.
        {
            let queries = vec![
                ddb_metric_query(
                    "m1",
                    "SuccessfulRequestLatency",
                    "Average",
                    table_name,
                    Some(("Operation", "GetItem")),
                    true,
                ),
                ddb_metric_query(
                    "m2",
                    "SuccessfulRequestLatency",
                    "Average",
                    table_name,
                    Some(("Operation", "PutItem")),
                    true,
                ),
                ddb_metric_query(
                    "m3",
                    "SuccessfulRequestLatency",
                    "Average",
                    table_name,
                    Some(("Operation", "Query")),
                    true,
                ),
                ddb_metric_query(
                    "m4",
                    "SuccessfulRequestLatency",
                    "Average",
                    table_name,
                    Some(("Operation", "Scan")),
                    true,
                ),
            ];
            let mut res = self.run_get_metric_data(queries, start, now).await?;
            data.latency_get_item = res.remove("m1").unwrap_or_default();
            data.latency_put_item = res.remove("m2").unwrap_or_default();
            data.latency_query = res.remove("m3").unwrap_or_default();
            data.latency_scan = res.remove("m4").unwrap_or_default();
        }

        Ok(data)
    }

    async fn run_get_metric_data(
        &self,
        queries: Vec<aws_sdk_cloudwatch::types::MetricDataQuery>,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<std::collections::HashMap<String, Vec<(i64, f64)>>> {
        let client = self.config.cloudwatch_client();

        let mut req = client
            .get_metric_data()
            .start_time(aws_sdk_cloudwatch::primitives::DateTime::from_millis(
                start_time.timestamp_millis(),
            ))
            .end_time(aws_sdk_cloudwatch::primitives::DateTime::from_millis(
                end_time.timestamp_millis(),
            ));
        for q in queries {
            req = req.metric_data_queries(q);
        }

        let resp = req.send().await?;

        let mut out: std::collections::HashMap<String, Vec<(i64, f64)>> =
            std::collections::HashMap::new();
        for r in resp.metric_data_results() {
            let Some(id) = r.id() else { continue };
            let mut points: Vec<(i64, f64)> = r
                .timestamps()
                .iter()
                .zip(r.values().iter())
                .map(|(t, v)| (t.secs(), *v))
                .collect();
            points.sort_by_key(|(t, _)| *t);
            out.insert(id.to_string(), points);
        }
        Ok(out)
    }
}

/// Time series for each DynamoDB Monitor graph. Empty vectors render as "no data".
#[derive(Default, Clone, Debug)]
pub struct DdbMonitoringData {
    /// Consumed capacity units per second.
    pub consumed_read_capacity: Vec<(i64, f64)>,
    pub consumed_write_capacity: Vec<(i64, f64)>,
    /// Throttling counts.
    pub read_throttle_events: Vec<(i64, f64)>,
    pub write_throttle_events: Vec<(i64, f64)>,
    pub throttled_requests: Vec<(i64, f64)>,
    /// Average successful request latency per operation, in milliseconds.
    pub latency_get_item: Vec<(i64, f64)>,
    pub latency_put_item: Vec<(i64, f64)>,
    pub latency_query: Vec<(i64, f64)>,
    pub latency_scan: Vec<(i64, f64)>,
}

/// A single item returned by Scan or Query, with values rendered for display.
#[derive(Clone, Debug, Default)]
pub struct DdbItem {
    /// Attribute name -> displayable value, ordered by attribute name.
    pub attributes: std::collections::BTreeMap<String, String>,
}

impl DdbItem {
    /// Displayable value for an attribute, or an empty string when absent.
    pub fn get(&self, attribute: &str) -> &str {
        self.attributes
            .get(attribute)
            .map(|v| v.as_str())
            .unwrap_or("")
    }
}

fn item_from_attributes(
    item: &std::collections::HashMap<String, aws_sdk_dynamodb::types::AttributeValue>,
) -> DdbItem {
    DdbItem {
        attributes: item
            .iter()
            .map(|(k, v)| (k.clone(), attribute_to_string(v)))
            .collect(),
    }
}

/// Render an `AttributeValue` the way the console item explorer does.
pub fn attribute_to_string(value: &aws_sdk_dynamodb::types::AttributeValue) -> String {
    use aws_sdk_dynamodb::types::AttributeValue as Av;
    match value {
        Av::S(s) => s.clone(),
        Av::N(n) => n.clone(),
        Av::Bool(b) => b.to_string(),
        Av::Null(_) => "null".to_string(),
        Av::B(b) => format!("<binary {} bytes>", b.as_ref().len()),
        Av::Ss(items) => format!("[{}]", items.join(", ")),
        Av::Ns(items) => format!("[{}]", items.join(", ")),
        Av::Bs(items) => format!("[{} binary values]", items.len()),
        Av::L(items) => format!(
            "[{}]",
            items
                .iter()
                .map(attribute_to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Av::M(map) => {
            let mut pairs: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", k, attribute_to_string(v)))
                .collect();
            pairs.sort();
            format!("{{{}}}", pairs.join(", "))
        }
        _ => String::new(),
    }
}

/// Build a `ProjectionExpression` with placeholders so reserved words are safe.
fn projection_expression(attributes: &[String]) -> (String, Vec<(String, String)>) {
    let names: Vec<(String, String)> = attributes
        .iter()
        .enumerate()
        .map(|(i, a)| (format!("#p{}", i), a.clone()))
        .collect();
    let expression = names
        .iter()
        .map(|(placeholder, _)| placeholder.clone())
        .collect::<Vec<_>>()
        .join(", ");
    (expression, names)
}

/// Metric period used by every Monitor graph, in seconds.
const METRIC_PERIOD: i32 = 60;

/// Build a raw metric `MetricDataQuery` for the `AWS/DynamoDB` namespace.
fn ddb_metric_query(
    id: &str,
    metric_name: &str,
    stat: &str,
    table_name: &str,
    extra_dimension: Option<(&str, &str)>,
    return_data: bool,
) -> aws_sdk_cloudwatch::types::MetricDataQuery {
    use aws_sdk_cloudwatch::types::{Dimension, Metric, MetricDataQuery, MetricStat};

    let mut metric_b = Metric::builder()
        .namespace("AWS/DynamoDB")
        .metric_name(metric_name)
        .dimensions(
            Dimension::builder()
                .name("TableName")
                .value(table_name)
                .build(),
        );
    if let Some((k, v)) = extra_dimension {
        metric_b = metric_b.dimensions(Dimension::builder().name(k).value(v).build());
    }

    let stat = MetricStat::builder()
        .metric(metric_b.build())
        .period(METRIC_PERIOD)
        .stat(stat)
        .build();

    MetricDataQuery::builder()
        .id(id)
        .metric_stat(stat)
        .return_data(return_data)
        .build()
}

/// Build a metric-math expression `MetricDataQuery`.
fn ddb_expr_query(
    id: &str,
    expression: &str,
    return_data: bool,
) -> aws_sdk_cloudwatch::types::MetricDataQuery {
    aws_sdk_cloudwatch::types::MetricDataQuery::builder()
        .id(id)
        .expression(expression)
        .period(METRIC_PERIOD)
        .return_data(return_data)
        .build()
}

/// Pretty-print a JSON policy document; falls back to the raw string.
fn pretty_json(raw: &str) -> String {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|v| serde_json::to_string_pretty(&v).ok())
        .unwrap_or_else(|| raw.to_string())
}

fn fmt_time(t: Option<&aws_smithy_types::DateTime>) -> String {
    t.and_then(|t| t.fmt(aws_smithy_types::date_time::Format::DateTime).ok())
        .unwrap_or_default()
}

/// Normalize an SDK status debug string to "Enabled"/"Disabled" where possible.
fn enabled_disabled(raw: &str) -> String {
    match raw.to_uppercase().as_str() {
        "ENABLED" | "ENABLING" => "Enabled".to_string(),
        "DISABLED" | "DISABLING" => "Disabled".to_string(),
        other => other.to_string(),
    }
}

/// Render a key schema as "name (TYPE)" for the partition and sort keys.
fn key_names(
    key_schema: &[aws_sdk_dynamodb::types::KeySchemaElement],
    attribute_types: &std::collections::HashMap<String, String>,
) -> (String, String) {
    let mut partition = String::new();
    let mut sort = String::new();
    for k in key_schema {
        let name = k.attribute_name();
        let label = match attribute_types.get(name) {
            Some(t) => format!("{} ({})", name, t),
            None => name.to_string(),
        };
        match k.key_type() {
            aws_sdk_dynamodb::types::KeyType::Hash => partition = label,
            aws_sdk_dynamodb::types::KeyType::Range => sort = label,
            _ => {}
        }
    }
    (partition, sort)
}

/// "ALL", "KEYS_ONLY" or "INCLUDE: a, b".
fn projection_of(projection: Option<&aws_sdk_dynamodb::types::Projection>) -> String {
    let Some(p) = projection else {
        return String::new();
    };
    let kind = p
        .projection_type()
        .map(|t| t.as_str().to_string())
        .unwrap_or_default();
    if kind == "INCLUDE" && !p.non_key_attributes().is_empty() {
        format!("INCLUDE: {}", p.non_key_attributes().join(", "))
    } else {
        kind
    }
}

/// "On-demand" when no provisioned units are set, "Provisioned" otherwise.
fn capacity_mode(units: i64, billing_on_demand: bool) -> String {
    if billing_on_demand || units == 0 {
        "On-demand".to_string()
    } else {
        "Provisioned".to_string()
    }
}

fn table_from_description(t: &aws_sdk_dynamodb::types::TableDescription) -> DdbTable {
    use std::collections::HashMap;

    // Attribute name -> "S" / "N" / "B", used to label the key columns.
    let attribute_types: HashMap<String, String> = t
        .attribute_definitions()
        .iter()
        .map(|a| {
            (
                a.attribute_name().to_string(),
                a.attribute_type().as_str().to_string(),
            )
        })
        .collect();

    let (partition_key, sort_key) = key_names(t.key_schema(), &attribute_types);

    let billing_on_demand = t
        .billing_mode_summary()
        .and_then(|b| b.billing_mode())
        .map(|m| m.as_str() == "PAY_PER_REQUEST")
        .unwrap_or(false);

    let provisioned_read_units = t
        .provisioned_throughput()
        .and_then(|p| p.read_capacity_units())
        .unwrap_or(0);
    let provisioned_write_units = t
        .provisioned_throughput()
        .and_then(|p| p.write_capacity_units())
        .unwrap_or(0);

    let global_secondary_indexes: Vec<DdbIndex> = t
        .global_secondary_indexes()
        .iter()
        .map(|g| {
            let (pk, sk) = key_names(g.key_schema(), &attribute_types);
            let read = g
                .provisioned_throughput()
                .and_then(|p| p.read_capacity_units())
                .unwrap_or(0);
            let write = g
                .provisioned_throughput()
                .and_then(|p| p.write_capacity_units())
                .unwrap_or(0);
            DdbIndex {
                name: g.index_name().unwrap_or("").to_string(),
                index_type: "Global".to_string(),
                status: g
                    .index_status()
                    .map(|s| s.as_str().to_string())
                    .unwrap_or_default(),
                partition_key: pk,
                sort_key: sk,
                projection: projection_of(g.projection()),
                read_capacity_mode: capacity_mode(read, billing_on_demand),
                write_capacity_mode: capacity_mode(write, billing_on_demand),
                size_bytes: g.index_size_bytes().unwrap_or(0),
                item_count: g.item_count().unwrap_or(0),
                index_arn: g.index_arn().unwrap_or("").to_string(),
            }
        })
        .collect();

    let local_secondary_indexes: Vec<DdbIndex> = t
        .local_secondary_indexes()
        .iter()
        .map(|l| {
            let (pk, sk) = key_names(l.key_schema(), &attribute_types);
            DdbIndex {
                name: l.index_name().unwrap_or("").to_string(),
                index_type: "Local".to_string(),
                status: "ACTIVE".to_string(),
                partition_key: pk,
                sort_key: sk,
                projection: projection_of(l.projection()),
                read_capacity_mode: capacity_mode(provisioned_read_units, billing_on_demand),
                write_capacity_mode: capacity_mode(provisioned_write_units, billing_on_demand),
                size_bytes: l.index_size_bytes().unwrap_or(0),
                item_count: l.item_count().unwrap_or(0),
                index_arn: l.index_arn().unwrap_or("").to_string(),
            }
        })
        .collect();

    let replicas: Vec<DdbReplica> = t
        .replicas()
        .iter()
        .map(|r| DdbReplica {
            region: r.region_name().unwrap_or("").to_string(),
            status: r
                .replica_status()
                .map(|s| s.as_str().to_string())
                .unwrap_or_default(),
            status_description: r.replica_status_description().unwrap_or("").to_string(),
            table_class: r
                .replica_table_class_summary()
                .and_then(|c| c.table_class())
                .map(|c| format_table_class(c.as_str()))
                .unwrap_or_default(),
            kms_key_id: r.kms_master_key_id().unwrap_or("").to_string(),
            index_count: r.global_secondary_indexes().len(),
        })
        .collect();

    let (stream_enabled, stream_view_type) = match t.stream_specification() {
        Some(s) if s.stream_enabled() => (
            "Enabled".to_string(),
            s.stream_view_type()
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
        ),
        _ => ("Disabled".to_string(), String::new()),
    };

    let (encryption_type, kms_key_arn) = match t.sse_description() {
        Some(sse) => (
            match sse.sse_type().map(|t| t.as_str().to_string()) {
                Some(ref t) if t.eq_ignore_ascii_case("KMS") => "AWS managed key".to_string(),
                Some(other) => other,
                None => "Owned by Amazon DynamoDB".to_string(),
            },
            sse.kms_master_key_arn().unwrap_or("").to_string(),
        ),
        None => ("Owned by Amazon DynamoDB".to_string(), String::new()),
    };

    DdbTable {
        name: t.table_name().unwrap_or("").to_string(),
        table_arn: t.table_arn().unwrap_or("").to_string(),
        table_id: t.table_id().unwrap_or("").to_string(),
        status: t
            .table_status()
            .map(|s| s.as_str().to_string())
            .unwrap_or_default(),
        partition_key,
        sort_key,
        read_capacity_mode: capacity_mode(provisioned_read_units, billing_on_demand),
        write_capacity_mode: capacity_mode(provisioned_write_units, billing_on_demand),
        provisioned_read_units,
        provisioned_write_units,
        table_class: t
            .table_class_summary()
            .and_then(|c| c.table_class())
            .map(|c| format_table_class(c.as_str()))
            .unwrap_or_else(|| "Standard".to_string()),
        deletion_protection: match t.deletion_protection_enabled() {
            Some(true) => "Enabled".to_string(),
            _ => "Disabled".to_string(),
        },
        total_size_bytes: t.table_size_bytes().unwrap_or(0),
        item_count: t.item_count().unwrap_or(0),
        creation_time: fmt_time(t.creation_date_time()),
        encryption_type,
        kms_key_arn,
        stream_enabled,
        stream_view_type,
        latest_stream_arn: t.latest_stream_arn().unwrap_or("").to_string(),
        global_secondary_indexes,
        local_secondary_indexes,
        replicas,
    }
}

/// "StandardInfrequentAccess" -> "Standard-IA", "Standard" -> "Standard".
pub fn format_table_class(raw: &str) -> String {
    match raw.to_uppercase().replace(['_', '-'], "").as_str() {
        "STANDARDINFREQUENTACCESS" | "DYNAMODBSTANDARDINFREQUENTACCESS" => {
            "Standard-IA".to_string()
        }
        "STANDARD" | "DYNAMODBSTANDARD" => "Standard".to_string(),
        _ => raw.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_table_class() {
        assert_eq!(format_table_class("Standard"), "Standard");
        assert_eq!(format_table_class("DynamodbStandard"), "Standard");
        assert_eq!(
            format_table_class("StandardInfrequentAccess"),
            "Standard-IA"
        );
        assert_eq!(
            format_table_class("DYNAMODB_STANDARD_INFREQUENT_ACCESS"),
            "Standard-IA"
        );
    }

    #[test]
    fn test_capacity_mode() {
        assert_eq!(capacity_mode(0, true), "On-demand");
        assert_eq!(capacity_mode(5, true), "On-demand");
        assert_eq!(capacity_mode(0, false), "On-demand");
        assert_eq!(capacity_mode(5, false), "Provisioned");
    }

    #[test]
    fn test_enabled_disabled() {
        assert_eq!(enabled_disabled("ENABLED"), "Enabled");
        assert_eq!(enabled_disabled("Enabling"), "Enabled");
        assert_eq!(enabled_disabled("DISABLED"), "Disabled");
        assert_eq!(enabled_disabled("UNKNOWN"), "UNKNOWN");
    }

    #[test]
    fn test_index_count_sums_global_and_local() {
        let table = DdbTable {
            global_secondary_indexes: vec![DdbIndex::default(), DdbIndex::default()],
            local_secondary_indexes: vec![DdbIndex::default()],
            ..Default::default()
        };
        assert_eq!(table.index_count(), 3);
    }
}
