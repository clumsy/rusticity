//! Extra reads are opt-in per visible column; failures never hide the inventory.
use super::{RdsClient, RdsInstance};
use anyhow::Result;
use aws_sdk_cloudwatch::types::{
    Dimension, Metric, MetricDataQuery, MetricStat, ScanBy, StatusCode,
};
use aws_sdk_rds::types::{DbCluster, DbRecommendation, ResourcePendingMaintenanceActions};
use aws_smithy_types::DateTime;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InventoryOptions {
    pub clusters: bool,
    pub recommendations: bool,
    pub maintenance: bool,
    pub cpu: bool,
    pub current_activity: bool,
    pub replica_lag: bool,
}

impl InventoryOptions {
    pub fn needs_metrics(self) -> bool {
        self.cpu || self.current_activity || self.replica_lag
    }
}

impl RdsClient {
    pub(super) async fn enrich(&self, instances: &mut [RdsInstance], options: InventoryOptions) {
        if instances.is_empty() {
            return;
        }
        if options.clusters && instances.iter().any(|i| !i.cluster_identifier.is_empty()) {
            match self.clusters().await {
                Ok(clusters) => apply_clusters(instances, &clusters),
                Err(error) => {
                    tracing::warn!("RDS cluster metadata unavailable: {error}");
                    for instance in instances
                        .iter_mut()
                        .filter(|i| !i.cluster_identifier.is_empty())
                    {
                        instance.role = "Unavailable".into();
                        instance.clone_group_id = "Unavailable".into();
                    }
                }
            }
        }
        if options.recommendations {
            match self.recommendations().await {
                Ok(rows) => apply_recommendations(instances, &rows),
                Err(error) => {
                    tracing::warn!("RDS recommendations unavailable: {error}");
                    for instance in instances.iter_mut() {
                        instance.recommendations = "Unavailable".into();
                    }
                }
            }
        }
        if options.maintenance {
            match self.maintenance().await {
                Ok(rows) => apply_maintenance(instances, &rows),
                Err(error) => {
                    tracing::warn!("RDS maintenance unavailable: {error}");
                    for instance in instances.iter_mut() {
                        instance.maintenance = "Unavailable".into();
                    }
                }
            }
        }
        if options.needs_metrics() {
            // At most 300 metrics per request (CloudWatch limit: 500).
            for chunk in instances.chunks_mut(100) {
                if let Err(error) = self.metrics(chunk, options).await {
                    tracing::warn!("RDS metrics unavailable: {error}");
                    for instance in chunk {
                        if options.cpu {
                            instance.cpu = "Unavailable".into();
                        }
                        if options.current_activity {
                            instance.current_activity = "Unavailable".into();
                        }
                        if options.replica_lag && is_replica_candidate(instance) {
                            instance.replica_lag = "Unavailable".into();
                        }
                    }
                }
            }
        }
    }

    async fn clusters(&self) -> Result<Vec<DbCluster>> {
        Ok(self
            .config
            .rds_client()
            .describe_db_clusters()
            .into_paginator()
            .items()
            .send()
            .collect::<Result<Vec<_>, _>>()
            .await?)
    }
    async fn recommendations(&self) -> Result<Vec<DbRecommendation>> {
        Ok(self
            .config
            .rds_client()
            .describe_db_recommendations()
            .into_paginator()
            .items()
            .send()
            .collect::<Result<Vec<_>, _>>()
            .await?)
    }
    async fn maintenance(&self) -> Result<Vec<ResourcePendingMaintenanceActions>> {
        Ok(self
            .config
            .rds_client()
            .describe_pending_maintenance_actions()
            .into_paginator()
            .items()
            .send()
            .collect::<Result<Vec<_>, _>>()
            .await?)
    }

    async fn metrics(
        &self,
        instances: &mut [RdsInstance],
        options: InventoryOptions,
    ) -> Result<()> {
        let mut queries = Vec::new();
        let mut targets = HashMap::new();
        for (index, instance) in instances.iter().enumerate() {
            let lag_name = if instance.engine.starts_with("aurora") {
                "AuroraReplicaLag"
            } else {
                "ReplicaLag"
            };
            for (enabled, metric, kind) in [
                (options.cpu, "CPUUtilization", 0),
                (options.current_activity, "DBLoad", 1),
                (
                    options.replica_lag && is_replica_candidate(instance),
                    lag_name,
                    2,
                ),
            ] {
                if !enabled {
                    continue;
                }
                let id = format!("m{index}_{kind}");
                targets.insert(id.clone(), (index, kind));
                let metric = Metric::builder()
                    .namespace("AWS/RDS")
                    .metric_name(metric)
                    .dimensions(
                        Dimension::builder()
                            .name("DBInstanceIdentifier")
                            .value(&instance.identifier)
                            .build(),
                    )
                    .build();
                queries.push(
                    MetricDataQuery::builder()
                        .id(id)
                        .return_data(true)
                        .metric_stat(
                            MetricStat::builder()
                                .metric(metric)
                                .period(60)
                                .stat("Average")
                                .build(),
                        )
                        .build(),
                );
            }
        }
        if queries.is_empty() {
            return Ok(());
        }
        let end = chrono::Utc::now().timestamp();
        let mut pages = self
            .config
            .cloudwatch_client()
            .get_metric_data()
            .set_metric_data_queries(Some(queries))
            .start_time(DateTime::from_secs(end - 900))
            .end_time(DateTime::from_secs(end))
            .scan_by(ScanBy::TimestampDescending)
            .into_paginator()
            .send();
        let mut latest: HashMap<String, (DateTime, f64)> = HashMap::new();
        let mut failed = Vec::new();
        while let Some(page) = pages.next().await {
            for result in page?.metric_data_results() {
                let id = result.id().unwrap_or_default();
                if matches!(
                    result.status_code(),
                    Some(StatusCode::Forbidden | StatusCode::InternalError)
                ) {
                    failed.push(id.to_string());
                }
                for (time, value) in result.timestamps().iter().zip(result.values()) {
                    if value.is_finite() && latest.get(id).is_none_or(|(old, _)| time > old) {
                        latest.insert(id.into(), (*time, *value));
                    }
                }
            }
        }
        for (id, (index, kind)) in targets {
            let instance = &mut instances[index];
            let value = if failed.contains(&id) {
                "Unavailable".into()
            } else if let Some((_, value)) = latest.get(&id) {
                match kind {
                    0 => format!("{value:.1}%"),
                    1 => format!("{value:.2} AAS"),
                    _ if *value < 0.0 => String::new(), // ReplicaLag -1 means unknown, not negative lag.
                    _ if instance.engine.starts_with("aurora") => format!("{value:.1} ms"),
                    _ => format!("{value:.1} s"),
                }
            } else {
                String::new()
            };
            match kind {
                0 => instance.cpu = value,
                1 => instance.current_activity = value,
                _ => instance.replica_lag = value,
            }
        }
        Ok(())
    }
}

fn is_replica_candidate(instance: &RdsInstance) -> bool {
    instance.is_replica || (!instance.cluster_identifier.is_empty() && instance.role != "Writer")
}

pub(super) fn apply_clusters(instances: &mut [RdsInstance], clusters: &[DbCluster]) {
    for instance in instances
        .iter_mut()
        .filter(|i| !i.cluster_identifier.is_empty())
    {
        if let Some(cluster) = clusters
            .iter()
            .find(|c| c.db_cluster_identifier() == Some(instance.cluster_identifier.as_str()))
        {
            instance.clone_group_id = cluster.clone_group_id().unwrap_or_default().into();
            if let Some(member) = cluster
                .db_cluster_members()
                .iter()
                .find(|m| m.db_instance_identifier() == Some(instance.identifier.as_str()))
            {
                instance.role = match member.is_cluster_writer() {
                    Some(true) => "Writer",
                    Some(false) => "Reader",
                    None => "",
                }
                .into();
                instance.is_replica = member.is_cluster_writer() == Some(false);
                instance.priority_tier = member.promotion_tier().or(instance.priority_tier);
            }
        }
    }
}

pub(super) fn apply_recommendations(instances: &mut [RdsInstance], rows: &[DbRecommendation]) {
    for instance in instances {
        // Without an ARN, we cannot attribute this resource's optional results.
        if instance.arn.is_empty() {
            continue;
        }
        instance.recommendations = rows
            .iter()
            .filter(|r| {
                r.resource_arn() == Some(instance.arn.as_str())
                    && r.status().is_some_and(|s| s.eq_ignore_ascii_case("active"))
            })
            .count()
            .to_string();
    }
}

pub(super) fn apply_maintenance(
    instances: &mut [RdsInstance],
    rows: &[ResourcePendingMaintenanceActions],
) {
    for instance in instances {
        // Without an ARN, we cannot attribute this resource's optional results.
        if instance.arn.is_empty() {
            continue;
        }
        instance.maintenance = rows
            .iter()
            .filter(|r| r.resource_identifier() == Some(instance.arn.as_str()))
            .flat_map(|r| r.pending_maintenance_action_details())
            .filter_map(|a| a.action())
            .collect::<Vec<_>>()
            .join(", ");
        if instance.maintenance.is_empty() {
            instance.maintenance = "None".into();
        }
    }
}
