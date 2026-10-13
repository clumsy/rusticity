use crate::config::AwsConfig;
use anyhow::Result;
use aws_sdk_rds::types::{DbInstance, PendingModifiedValues};
use aws_smithy_types::{date_time::Format, DateTime};

mod enrichment;
pub use enrichment::InventoryOptions;

/// Read-only database inventory. Rows are DB instances; cluster metadata enriches
/// member instances without inventing separate cluster rows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RdsInstance {
    pub identifier: String,
    pub arn: String,
    pub status: String,
    pub engine: String,
    pub engine_version: String,
    pub instance_class: String,
    pub region: String,
    pub availability_zone: String,
    pub endpoint: String,
    pub port: Option<i32>,
    pub cluster_identifier: String,
    pub role: String,
    pub upgrade_rollout_order: String,
    pub extended_support: String,
    pub recommendations: String,
    pub cpu: String,
    pub current_activity: String,
    pub maintenance: String,
    pub vpc: String,
    pub multi_az: Option<bool>,
    pub storage_type: String,
    pub storage_gib: Option<i32>,
    pub iops: Option<i32>,
    pub storage_throughput: Option<i32>,
    pub security_groups: String,
    pub subnet_group: String,
    pub certificate_expiry: String,
    pub pending_changes: String,
    pub character_set: String,
    pub option_group: String,
    pub created_time: String,
    pub encrypted: Option<bool>,
    pub priority_tier: Option<i32>,
    pub replica_lag: String,
    pub replica_state: String,
    pub parameter_group: String,
    pub clone_group_id: String,
    pub is_replica: bool,
}

pub struct RdsClient {
    config: AwsConfig,
}

impl RdsClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    pub async fn list_instances(&self) -> Result<Vec<RdsInstance>> {
        let mut stream = self
            .config
            .rds_client()
            .describe_db_instances()
            .into_paginator()
            .items()
            .send();
        let mut instances = Vec::new();
        while let Some(instance) = stream.next().await {
            instances.push(RdsInstance::from_sdk(&instance?, &self.config.region));
        }
        instances.sort_by(|a, b| a.identifier.cmp(&b.identifier));
        Ok(instances)
    }

    pub async fn list_instances_with_options(
        &self,
        options: InventoryOptions,
    ) -> Result<Vec<RdsInstance>> {
        let mut instances = self.list_instances().await?;
        self.enrich(&mut instances, options).await;
        Ok(instances)
    }
}

fn timestamp(value: Option<&DateTime>) -> String {
    value
        .and_then(|time| time.fmt(Format::DateTime).ok())
        .unwrap_or_default()
}

fn pending_changes(values: Option<&PendingModifiedValues>) -> String {
    let Some(v) = values else {
        return String::new();
    };
    // Summarize changed fields, never Debug-print a structure that can contain
    // credentials (PendingModifiedValues includes a password field).
    let changes = [
        (v.db_instance_class().is_some(), "Size"),
        (v.allocated_storage().is_some(), "Storage"),
        (v.master_user_password().is_some(), "Password"),
        (v.port().is_some(), "Port"),
        (v.backup_retention_period().is_some(), "Backup retention"),
        (v.multi_az().is_some(), "Multi-AZ"),
        (v.engine_version().is_some(), "Engine version"),
        (v.license_model().is_some(), "License"),
        (v.iops().is_some(), "IOPS"),
        (v.storage_throughput().is_some(), "Storage throughput"),
        (v.db_instance_identifier().is_some(), "DB identifier"),
        (v.storage_type().is_some(), "Storage type"),
        (v.ca_certificate_identifier().is_some(), "Certificate"),
        (v.db_subnet_group_name().is_some(), "Subnet group"),
        (v.pending_cloudwatch_logs_exports().is_some(), "Log exports"),
        (!v.processor_features().is_empty(), "Processor features"),
        (v.automation_mode().is_some(), "Automation mode"),
        (
            v.resume_full_automation_mode_time().is_some(),
            "Resume automation",
        ),
        (v.multi_tenant().is_some(), "Multi-tenant"),
        (
            v.iam_database_authentication_enabled().is_some(),
            "IAM authentication",
        ),
        (v.dedicated_log_volume().is_some(), "Dedicated log volume"),
        (v.engine().is_some(), "Engine"),
        (
            !v.additional_storage_volumes().is_empty(),
            "Additional storage volumes",
        ),
    ];
    changes
        .into_iter()
        .filter_map(|(pending, label)| pending.then_some(label))
        .collect::<Vec<_>>()
        .join(", ")
}

impl RdsInstance {
    fn from_sdk(instance: &DbInstance, region: &str) -> Self {
        let is_replica = instance
            .read_replica_source_db_instance_identifier()
            .is_some()
            || instance
                .read_replica_source_db_cluster_identifier()
                .is_some();
        let cluster_identifier = instance
            .db_cluster_identifier()
            .unwrap_or_default()
            .to_string();
        let role = if !cluster_identifier.is_empty() {
            String::new()
        } else if is_replica {
            "Read replica".into()
        } else if instance.db_instance_identifier().is_some() {
            "Instance".into()
        } else {
            String::new()
        };
        Self {
            identifier: instance.db_instance_identifier().unwrap_or_default().into(),
            arn: instance.db_instance_arn().unwrap_or_default().into(),
            status: instance.db_instance_status().unwrap_or_default().into(),
            engine: instance.engine().unwrap_or_default().into(),
            engine_version: instance.engine_version().unwrap_or_default().into(),
            instance_class: instance.db_instance_class().unwrap_or_default().into(),
            region: region.into(),
            availability_zone: instance.availability_zone().unwrap_or_default().into(),
            endpoint: instance
                .endpoint()
                .and_then(|e| e.address())
                .unwrap_or_default()
                .into(),
            port: instance.endpoint().and_then(|e| e.port()),
            cluster_identifier,
            role,
            is_replica,
            upgrade_rollout_order: instance
                .upgrade_rollout_order()
                .map(|v| v.as_str().into())
                .unwrap_or_default(),
            extended_support: match instance.engine_lifecycle_support() {
                Some("open-source-rds-extended-support") => "Enabled".into(),
                Some("open-source-rds-extended-support-disabled") => "Disabled".into(),
                value => value.unwrap_or_default().into(),
            },
            vpc: instance
                .db_subnet_group()
                .and_then(|g| g.vpc_id())
                .unwrap_or_default()
                .into(),
            multi_az: instance.multi_az(),
            storage_type: instance.storage_type().unwrap_or_default().into(),
            storage_gib: instance.allocated_storage(),
            iops: instance.iops(),
            storage_throughput: instance.storage_throughput(),
            security_groups: instance
                .vpc_security_groups()
                .iter()
                .filter_map(|g| g.vpc_security_group_id())
                .chain(
                    instance
                        .db_security_groups()
                        .iter()
                        .filter_map(|g| g.db_security_group_name()),
                )
                .collect::<Vec<_>>()
                .join(", "),
            subnet_group: instance
                .db_subnet_group()
                .and_then(|g| g.db_subnet_group_name())
                .unwrap_or_default()
                .into(),
            certificate_expiry: timestamp(
                instance.certificate_details().and_then(|c| c.valid_till()),
            ),
            pending_changes: pending_changes(instance.pending_modified_values()),
            character_set: [
                instance.character_set_name(),
                instance.nchar_character_set_name(),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(", "),
            option_group: instance
                .option_group_memberships()
                .iter()
                .filter_map(|g| g.option_group_name())
                .collect::<Vec<_>>()
                .join(", "),
            created_time: timestamp(instance.instance_create_time()),
            encrypted: instance.storage_encrypted(),
            priority_tier: instance.promotion_tier(),
            replica_state: instance
                .status_infos()
                .iter()
                .filter(|s| s.status_type() == Some("read replication"))
                .filter_map(|s| s.status())
                .collect::<Vec<_>>()
                .join(", "),
            parameter_group: instance
                .db_parameter_groups()
                .iter()
                .filter_map(|g| g.db_parameter_group_name())
                .collect::<Vec<_>>()
                .join(", "),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests;
