use crate::config::AwsConfig;
use anyhow::Result;

fn fmt_bytes(bytes: i64) -> String {
    const GIB: i64 = 1 << 30;
    const MIB: i64 = 1 << 20;
    const KIB: i64 = 1 << 10;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[derive(Clone, Debug)]
pub struct EfsFileSystem {
    pub file_system_id: String,
    pub file_system_arn: String,
    pub name: String,
    pub creation_token: String,
    pub encrypted: String,
    pub kms_key_id: String,
    pub total_size: String,
    pub size_in_standard: String,
    pub size_in_ia: String,
    pub size_in_archive: String,
    pub provisioned_throughput: String,
    pub throughput_mode: String,
    pub life_cycle_state: String,
    pub number_of_mount_targets: String,
    pub owner_id: String,
    pub creation_time: String,
    pub performance_mode: String,
    pub availability_zone: String,
    pub replication_overwrite_protection: String,
    /// Raw byte sizes for percentage calculations
    pub total_size_bytes: i64,
    pub size_in_standard_bytes: i64,
    pub size_in_ia_bytes: i64,
    pub size_in_archive_bytes: i64,
    /// DNS name: <fs-id>.efs.<region>.amazonaws.com
    pub dns_name: String,
    /// Tags as (key, value) pairs
    pub tags: Vec<(String, String)>,
}

pub struct EfsClient {
    config: AwsConfig,
}

impl EfsClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    pub async fn list_file_systems(&self) -> Result<Vec<EfsFileSystem>> {
        let client = self.config.efs_client();

        let mut file_systems: Vec<EfsFileSystem> = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut request = client.describe_file_systems();
            if let Some(m) = marker {
                request = request.marker(m);
            }

            let response = request.send().await?;

            for fs in response.file_systems.unwrap_or_default() {
                let name = fs.name().map(|s| s.to_string()).unwrap_or_default();

                let sizes = fs.size_in_bytes();
                let total_size = sizes.map(|s| fmt_bytes(s.value)).unwrap_or_default();
                let size_in_standard = sizes
                    .and_then(|s| s.value_in_standard)
                    .map(fmt_bytes)
                    .unwrap_or_default();
                let size_in_ia = sizes
                    .and_then(|s| s.value_in_ia)
                    .map(fmt_bytes)
                    .unwrap_or_default();
                let size_in_archive = sizes
                    .and_then(|s| s.value_in_archive)
                    .map(fmt_bytes)
                    .unwrap_or_default();

                let provisioned_throughput = fs
                    .provisioned_throughput_in_mibps()
                    .map(|v| format!("{:.1}", v))
                    .unwrap_or_default();

                let throughput_mode = fs
                    .throughput_mode()
                    .map(|m| format!("{:?}", m))
                    .unwrap_or_default();

                let life_cycle_state = format!("{:?}", fs.life_cycle_state());

                let performance_mode = format!("{:?}", fs.performance_mode());

                let availability_zone = fs.availability_zone_name().unwrap_or("").to_string();

                let encrypted = match fs.encrypted() {
                    Some(true) => "Yes".to_string(),
                    Some(false) => "No".to_string(),
                    None => String::new(),
                };

                let kms_key_id = fs.kms_key_id().unwrap_or("").to_string();

                let replication_overwrite_protection = fs
                    .file_system_protection()
                    .and_then(|p| p.replication_overwrite_protection())
                    .map(|r| format!("{:?}", r))
                    .unwrap_or_default();

                let creation_time = fs
                    .creation_time()
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default();

                file_systems.push(EfsFileSystem {
                    file_system_id: fs.file_system_id.clone(),
                    file_system_arn: fs.file_system_arn().unwrap_or("").to_string(),
                    name,
                    creation_token: fs.creation_token.clone(),
                    encrypted,
                    kms_key_id,
                    total_size,
                    size_in_standard,
                    size_in_ia,
                    size_in_archive,
                    provisioned_throughput,
                    throughput_mode,
                    life_cycle_state,
                    number_of_mount_targets: fs.number_of_mount_targets.to_string(),
                    owner_id: fs.owner_id.clone(),
                    creation_time,
                    performance_mode,
                    availability_zone,
                    replication_overwrite_protection,
                    total_size_bytes: sizes.map(|s| s.value).unwrap_or(0),
                    size_in_standard_bytes: sizes.and_then(|s| s.value_in_standard).unwrap_or(0),
                    size_in_ia_bytes: sizes.and_then(|s| s.value_in_ia).unwrap_or(0),
                    size_in_archive_bytes: sizes.and_then(|s| s.value_in_archive).unwrap_or(0),
                    dns_name: format!(
                        "{}.efs.{}.amazonaws.com",
                        fs.file_system_id.as_str(),
                        // extract region from ARN or use "unknown"
                        fs.file_system_arn()
                            .and_then(|a| a.split(':').nth(3))
                            .unwrap_or("unknown")
                    ),
                    tags: fs
                        .tags
                        .iter()
                        .map(|t| (t.key.clone(), t.value.clone()))
                        .collect(),
                });
            }

            marker = response.next_marker;
            if marker.is_none() {
                break;
            }
        }

        // Sort by creation_time descending
        file_systems.sort_by(|a, b| b.creation_time.cmp(&a.creation_time));

        Ok(file_systems)
    }

    /// List the access points for a file system.
    pub async fn describe_access_points(
        &self,
        file_system_id: &str,
    ) -> Result<Vec<EfsAccessPoint>> {
        let client = self.config.efs_client();
        let mut out: Vec<EfsAccessPoint> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = client
                .describe_access_points()
                .file_system_id(file_system_id);
            if let Some(t) = next_token {
                req = req.next_token(t);
            }
            let resp = req.send().await?;

            for ap in resp.access_points() {
                let posix_user = ap
                    .posix_user()
                    .map(|p| format!("{}:{}", p.uid(), p.gid()))
                    .unwrap_or_default();

                let (path, creation_info) = match ap.root_directory() {
                    Some(rd) => {
                        let path = rd.path().unwrap_or("/").to_string();
                        let ci = rd
                            .creation_info()
                            .map(|c| {
                                format!("{}:{} {}", c.owner_uid(), c.owner_gid(), c.permissions())
                            })
                            .unwrap_or_default();
                        (path, ci)
                    }
                    None => (String::new(), String::new()),
                };

                out.push(EfsAccessPoint {
                    access_point_id: ap.access_point_id().unwrap_or("").to_string(),
                    name: ap.name().unwrap_or("").to_string(),
                    path,
                    posix_user,
                    creation_info,
                    life_cycle_state: ap
                        .life_cycle_state()
                        .map(|s| format!("{:?}", s))
                        .unwrap_or_default(),
                });
            }

            next_token = resp.next_token().map(|s| s.to_string());
            if next_token.is_none() {
                break;
            }
        }
        Ok(out)
    }

    /// List the mount targets (network interfaces) for a file system, including
    /// their security groups. Sorted by availability zone name ascending.
    pub async fn describe_mount_targets(
        &self,
        file_system_id: &str,
    ) -> Result<Vec<EfsMountTarget>> {
        let client = self.config.efs_client();
        let mut out: Vec<EfsMountTarget> = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut req = client
                .describe_mount_targets()
                .file_system_id(file_system_id);
            if let Some(m) = marker {
                req = req.marker(m);
            }
            let resp = req.send().await?;

            for mt in resp.mount_targets() {
                let mount_target_id = mt.mount_target_id().to_string();

                // Security groups require a separate call and only succeed when
                // the mount target is available; ignore errors (leave empty).
                let security_groups = match client
                    .describe_mount_target_security_groups()
                    .mount_target_id(&mount_target_id)
                    .send()
                    .await
                {
                    Ok(sg_resp) => sg_resp.security_groups().join(", "),
                    Err(_) => String::new(),
                };

                out.push(EfsMountTarget {
                    mount_target_id,
                    availability_zone_name: mt.availability_zone_name().unwrap_or("").to_string(),
                    availability_zone_id: mt.availability_zone_id().unwrap_or("").to_string(),
                    owner_id: mt.owner_id().unwrap_or("").to_string(),
                    file_system_id: mt.file_system_id().to_string(),
                    subnet_id: mt.subnet_id().to_string(),
                    vpc_id: mt.vpc_id().unwrap_or("").to_string(),
                    life_cycle_state: format!("{:?}", mt.life_cycle_state()),
                    ip_address: mt.ip_address().unwrap_or("").to_string(),
                    ipv6_address: mt.ipv6_address().unwrap_or("").to_string(),
                    network_interface_id: mt.network_interface_id().unwrap_or("").to_string(),
                    security_groups,
                });
            }

            marker = resp.next_marker().map(|s| s.to_string());
            if marker.is_none() {
                break;
            }
        }

        out.sort_by(|a, b| a.availability_zone_name.cmp(&b.availability_zone_name));
        Ok(out)
    }

    /// Fetch the replication configuration for a file system. Returns
    /// `Ok(None)` when the file system is not being replicated
    /// (`ReplicationNotFound`).
    pub async fn get_replication(&self, file_system_id: &str) -> Result<Option<String>> {
        let client = self.config.efs_client();
        let resp = client
            .describe_replication_configurations()
            .file_system_id(file_system_id)
            .send()
            .await;

        match resp {
            Ok(out) => {
                let configs = out.replications();
                if configs.is_empty() {
                    return Ok(None);
                }
                Ok(Some(format_replication(configs)))
            }
            Err(err) => {
                if err
                    .as_service_error()
                    .map(|e| e.is_replication_not_found())
                    .unwrap_or(false)
                {
                    Ok(None)
                } else {
                    Err(err.into())
                }
            }
        }
    }

    /// Fetch and pretty-print the file system's resource policy, if one exists.
    /// Returns `Ok(None)` when no policy is attached (`PolicyNotFound`).
    pub async fn get_file_system_policy(&self, file_system_id: &str) -> Result<Option<String>> {
        let client = self.config.efs_client();
        let resp = client
            .describe_file_system_policy()
            .file_system_id(file_system_id)
            .send()
            .await;

        match resp {
            Ok(out) => {
                let Some(policy) = out.policy() else {
                    return Ok(None);
                };
                // Pretty-print; fall back to the raw string if it isn't valid JSON.
                let pretty = serde_json::from_str::<serde_json::Value>(policy)
                    .ok()
                    .and_then(|v| serde_json::to_string_pretty(&v).ok())
                    .unwrap_or_else(|| policy.to_string());
                Ok(Some(pretty))
            }
            Err(err) => {
                if err
                    .as_service_error()
                    .map(|e| e.is_policy_not_found())
                    .unwrap_or(false)
                {
                    Ok(None)
                } else {
                    Err(err.into())
                }
            }
        }
    }

    /// Fetch the six EFS monitoring graphs for a file system via CloudWatch
    /// GetMetricData (metric math). Mirrors the AWS console's EFS monitoring tab.
    pub async fn get_monitoring_metrics(&self, file_system_id: &str) -> Result<EfsMonitoringData> {
        let now = chrono::Utc::now();
        let short_start = now - chrono::Duration::hours(3);
        // StorageBytes is emitted with coarse (daily) granularity, so widen its window.
        let long_start = now - chrono::Duration::hours(24);

        let mut data = EfsMonitoringData::default();

        // Graph 1 — Throughput utilization (%).
        // e1 = (MeteredIOBytes/1MiB)/PERIOD, e2 = PermittedThroughput/1MiB, e4 = e1*100/e2.
        {
            let queries = vec![
                efs_expr_query("e1", "(m1/1048576)/PERIOD(m1)", 60, false),
                efs_expr_query("e2", "m2/1048576", 60, false),
                efs_expr_query("e4", "((e1)*100)/(e2)", 60, true),
                efs_metric_query(
                    "m1",
                    "MeteredIOBytes",
                    "Sum",
                    60,
                    file_system_id,
                    None,
                    false,
                ),
                efs_metric_query(
                    "m2",
                    "PermittedThroughput",
                    "Average",
                    60,
                    file_system_id,
                    None,
                    false,
                ),
            ];
            let mut res = self.run_get_metric_data(queries, short_start, now).await?;
            data.throughput_utilization = res.remove("e4").unwrap_or_default();
        }

        // Graph 2 — IOPS by type (stat SampleCount). % of TotalIOBytes samples.
        {
            let queries = iops_or_throughput_queries("SampleCount", file_system_id);
            let mut res = self.run_get_metric_data(queries, short_start, now).await?;
            data.iops_data_write = res.remove("e2").unwrap_or_default();
            data.iops_data_read = res.remove("e3").unwrap_or_default();
            data.iops_metadata = res.remove("e4").unwrap_or_default();
        }

        // Graph 3 — Throughput by type (stat Sum). % of TotalIOBytes.
        {
            let queries = iops_or_throughput_queries("Sum", file_system_id);
            let mut res = self.run_get_metric_data(queries, short_start, now).await?;
            data.throughput_data_write = res.remove("e2").unwrap_or_default();
            data.throughput_data_read = res.remove("e3").unwrap_or_default();
            data.throughput_metadata = res.remove("e4").unwrap_or_default();
        }

        // Graph 4 — IOPS utilization (%). PercentIOLimit, Average.
        {
            let queries = vec![efs_metric_query(
                "m1",
                "PercentIOLimit",
                "Average",
                60,
                file_system_id,
                None,
                true,
            )];
            let mut res = self.run_get_metric_data(queries, short_start, now).await?;
            data.iops_utilization = res.remove("m1").unwrap_or_default();
        }

        // Graph 5 — Client connections (Sum).
        {
            let queries = vec![efs_metric_query(
                "m1",
                "ClientConnections",
                "Sum",
                60,
                file_system_id,
                None,
                true,
            )];
            let mut res = self.run_get_metric_data(queries, short_start, now).await?;
            data.client_connections = res.remove("m1").unwrap_or_default();
        }

        // Graph 6 — Storage bytes by storage class (Average). Standard / IA / Archive.
        {
            let queries = vec![
                efs_metric_query(
                    "mStandard",
                    "StorageBytes",
                    "Average",
                    3600,
                    file_system_id,
                    Some(("StorageClass", "Standard")),
                    true,
                ),
                efs_metric_query(
                    "mIA",
                    "StorageBytes",
                    "Average",
                    3600,
                    file_system_id,
                    Some(("StorageClass", "IA")),
                    true,
                ),
                efs_metric_query(
                    "mArchive",
                    "StorageBytes",
                    "Average",
                    3600,
                    file_system_id,
                    Some(("StorageClass", "Archive")),
                    true,
                ),
            ];
            let mut res = self.run_get_metric_data(queries, long_start, now).await?;
            data.storage_standard = res.remove("mStandard").unwrap_or_default();
            data.storage_ia = res.remove("mIA").unwrap_or_default();
            data.storage_archive = res.remove("mArchive").unwrap_or_default();
        }

        Ok(data)
    }

    /// Run a GetMetricData request and collect every returned series by its query id.
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
            let mut series: Vec<(i64, f64)> = r
                .timestamps()
                .iter()
                .zip(r.values().iter())
                .map(|(ts, val)| (ts.as_secs_f64() as i64, *val))
                .collect();
            series.sort_by_key(|(ts, _)| *ts);
            out.insert(id.to_string(), series);
        }
        Ok(out)
    }
}

/// An EFS access point.
#[derive(Clone, Debug, Default)]
pub struct EfsAccessPoint {
    pub access_point_id: String,
    pub name: String,
    pub path: String,
    /// POSIX user as `uid:gid`.
    pub posix_user: String,
    /// Root directory creation info as `owner_uid:owner_gid permissions`.
    pub creation_info: String,
    pub life_cycle_state: String,
}

/// An EFS mount target (network interface in a subnet).
#[derive(Clone, Debug, Default)]
pub struct EfsMountTarget {
    pub mount_target_id: String,
    pub availability_zone_name: String,
    pub availability_zone_id: String,
    pub owner_id: String,
    pub file_system_id: String,
    pub subnet_id: String,
    pub vpc_id: String,
    pub life_cycle_state: String,
    pub ip_address: String,
    pub ipv6_address: String,
    pub network_interface_id: String,
    /// Comma-separated security group IDs.
    pub security_groups: String,
}

/// Format a replication configuration into a human-readable multi-line summary.
fn format_replication(
    configs: &[aws_sdk_efs::types::ReplicationConfigurationDescription],
) -> String {
    let mut lines: Vec<String> = Vec::new();
    for cfg in configs {
        lines.push(format!(
            "Source file system: {}",
            cfg.source_file_system_id()
        ));
        lines.push(format!(
            "Source region: {}",
            cfg.source_file_system_region()
        ));
        if let Ok(ts) = cfg
            .creation_time()
            .fmt(aws_smithy_types::date_time::Format::DateTime)
        {
            lines.push(format!("Created: {}", ts));
        }
        lines.push(String::new());
        lines.push("Destinations:".to_string());
        for d in cfg.destinations() {
            lines.push(format!(
                "  • {} ({}) — {:?}",
                d.file_system_id(),
                d.region(),
                d.status()
            ));
            if let Some(ts) = d.last_replicated_timestamp() {
                if let Ok(s) = ts.fmt(aws_smithy_types::date_time::Format::DateTime) {
                    lines.push(format!("    last replicated: {}", s));
                }
            }
        }
    }
    lines.join("\n")
}

/// Time series for each EFS monitoring graph. Empty vectors render as "no data".
#[derive(Default, Clone, Debug)]
pub struct EfsMonitoringData {
    /// Graph 1 — Throughput utilization (%).
    pub throughput_utilization: Vec<(i64, f64)>,
    /// Graph 2 — IOPS by type (% of total IO operations).
    pub iops_data_write: Vec<(i64, f64)>,
    pub iops_data_read: Vec<(i64, f64)>,
    pub iops_metadata: Vec<(i64, f64)>,
    /// Graph 3 — Throughput by type (% of total IO bytes).
    pub throughput_data_write: Vec<(i64, f64)>,
    pub throughput_data_read: Vec<(i64, f64)>,
    pub throughput_metadata: Vec<(i64, f64)>,
    /// Graph 4 — IOPS utilization (%).
    pub iops_utilization: Vec<(i64, f64)>,
    /// Graph 5 — Client connections.
    pub client_connections: Vec<(i64, f64)>,
    /// Graph 6 — Storage bytes by storage class.
    pub storage_standard: Vec<(i64, f64)>,
    pub storage_ia: Vec<(i64, f64)>,
    pub storage_archive: Vec<(i64, f64)>,
}

/// Build a raw metric `MetricDataQuery` for the `AWS/EFS` namespace.
fn efs_metric_query(
    id: &str,
    metric_name: &str,
    stat: &str,
    period: i32,
    file_system_id: &str,
    extra_dimension: Option<(&str, &str)>,
    return_data: bool,
) -> aws_sdk_cloudwatch::types::MetricDataQuery {
    use aws_sdk_cloudwatch::types::{Dimension, Metric, MetricDataQuery, MetricStat};

    let mut metric_b = Metric::builder()
        .namespace("AWS/EFS")
        .metric_name(metric_name);
    // StorageBytes uses StorageClass as the leading dimension in the console.
    if let Some((k, v)) = extra_dimension {
        metric_b = metric_b.dimensions(Dimension::builder().name(k).value(v).build());
    }
    metric_b = metric_b.dimensions(
        Dimension::builder()
            .name("FileSystemId")
            .value(file_system_id)
            .build(),
    );

    let stat = MetricStat::builder()
        .metric(metric_b.build())
        .period(period)
        .stat(stat)
        .build();

    MetricDataQuery::builder()
        .id(id)
        .metric_stat(stat)
        .return_data(return_data)
        .build()
}

/// Build a metric-math expression `MetricDataQuery`.
fn efs_expr_query(
    id: &str,
    expression: &str,
    period: i32,
    return_data: bool,
) -> aws_sdk_cloudwatch::types::MetricDataQuery {
    aws_sdk_cloudwatch::types::MetricDataQuery::builder()
        .id(id)
        .expression(expression)
        .period(period)
        .return_data(return_data)
        .build()
}

/// Shared query set for graphs 2 (IOPS by type) and 3 (Throughput by type):
/// Data write / read / metadata as a percentage of TotalIOBytes, differing only
/// by the aggregation `stat` (SampleCount vs Sum).
fn iops_or_throughput_queries(
    stat: &str,
    file_system_id: &str,
) -> Vec<aws_sdk_cloudwatch::types::MetricDataQuery> {
    vec![
        efs_expr_query("e2", "(m2*100)/m1", 60, true),
        efs_expr_query("e3", "(m3*100)/m1", 60, true),
        efs_expr_query("e4", "(m4*100)/m1", 60, true),
        efs_metric_query("m1", "TotalIOBytes", stat, 60, file_system_id, None, false),
        efs_metric_query(
            "m2",
            "DataWriteIOBytes",
            stat,
            60,
            file_system_id,
            None,
            false,
        ),
        efs_metric_query(
            "m3",
            "DataReadIOBytes",
            stat,
            60,
            file_system_id,
            None,
            false,
        ),
        efs_metric_query(
            "m4",
            "MetadataIOBytes",
            stat,
            60,
            file_system_id,
            None,
            false,
        ),
    ]
}
