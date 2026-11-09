//! Read-only detail for one DB instance: one DescribeDBInstances call scoped to
//! the instance ARN fills the Summary and the summary tabs, tags included. The
//! returned identity is verified against the requested ARN. The other tabs come
//! from bounded parallel reads in `sections`. Never a password, a connection
//! or a mutating call.
use super::timestamp;
use crate::config::AwsConfig;
use anyhow::{anyhow, bail, Context, Result};
use aws_sdk_rds::types::DbInstance;
use std::time::Duration;

mod connectivity;
mod migrations;
pub mod sections;
pub use sections::Sections;

const TIMEOUT: Duration = Duration::from_secs(30);
/// Bound on each optional read after the instance read succeeded.
const ENRICH_TIMEOUT: Duration = Duration::from_secs(15);

/// Fresh metadata. Absent values stay `None`/empty rather than invented.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RdsInstanceDetail {
    pub identifier: String,
    pub arn: String,
    pub resource_id: String,
    pub status: String,
    pub role: String,
    pub engine: String,
    pub engine_version: String,
    pub instance_class: String,
    pub region: String,
    pub availability_zone: String,
    // Connectivity & security
    pub endpoint: String,
    pub port: Option<i32>,
    pub vpc: String,
    pub subnet_group: String,
    pub subnets: Vec<String>,
    pub network_type: String,
    /// `sg-123 (active)` per VPC security group.
    pub security_groups: Vec<String>,
    pub publicly_accessible: Option<bool>,
    pub certificate_authority: String,
    pub certificate_expiry: String,
    // Configuration
    pub db_name: String,
    /// Raw `EngineLifecycleSupport`.
    pub engine_lifecycle_support: String,
    pub license_model: String,
    /// (name, status) per option group.
    pub option_groups: Vec<(String, String)>,
    /// (name, parameter apply status) per parameter group.
    pub parameter_groups: Vec<(String, String)>,
    pub created_time: String,
    pub deletion_protection: Option<bool>,
    pub multi_tenant: Option<bool>,
    pub capacity_reservation: String,
    /// From EC2 DescribeInstanceTypes: "Unavailable" when that read fails,
    /// empty when the class has no EC2 equivalent.
    pub vcpu: String,
    pub memory: String,
    pub secondary_zone: String,
    pub master_secret_arn: String,
    pub master_secret_status: String,
    /// Alias name (`aws/secretsmanager`) when KMS resolves one, else the key ID.
    pub master_secret_kms_key: String,
    pub storage_upgrade_available: Option<bool>,
    /// Raw `DatabaseInsightsMode`.
    pub database_insights_mode: String,
    pub monitoring_interval: Option<i32>,
    pub storage_type: String,
    pub storage_gib: Option<i32>,
    pub max_storage_gib: Option<i32>,
    pub iops: Option<i32>,
    pub storage_throughput: Option<i32>,
    pub encrypted: Option<bool>,
    pub kms_key_id: String,
    pub multi_az: Option<bool>,
    pub master_username: String,
    pub iam_auth: Option<bool>,
    pub performance_insights: Option<bool>,
    // Maintenance & backups
    pub auto_minor_version_upgrade: Option<bool>,
    pub maintenance_window: String,
    pub pending_changes: String,
    pub backup_retention_days: Option<i32>,
    pub backup_window: String,
    pub copy_tags_to_snapshot: Option<bool>,
    pub latest_restorable_time: String,
    pub backup_target: String,
    pub tags: Vec<(String, String)>,
    /// Manage IAM roles.
    pub iam_roles: Vec<sections::RdsIamRole>,
    // Summary metrics, from the same enrichment as the list columns.
    // "Unavailable" when that read fails or times out; empty when no data.
    pub cpu: String,
    pub current_activity: String,
    /// Active recommendations, counted from `sections.recommendations`.
    pub recommendations: String,
    /// Monitoring, Logs & events, Zero-ETL integrations and Recommendations.
    pub sections: Sections,
}

/// `arn:<partition>:rds:<region>:<account>:db:<identifier>`; returns the identifier.
pub(crate) fn validate_instance_arn(arn: &str) -> Result<&str> {
    let parts: Vec<_> = arn.splitn(7, ':').collect();
    if parts.len() != 7
        || parts[0] != "arn"
        || parts[1].is_empty()
        || parts[2] != "rds"
        || parts[3].is_empty()
        || parts[4].len() != 12
        || !parts[4].bytes().all(|c| c.is_ascii_digit())
        || parts[5] != "db"
        || parts[6].is_empty()
        || arn.chars().any(char::is_control)
    {
        bail!("not an RDS DB instance ARN: {arn:?}");
    }
    Ok(parts[6])
}

pub(crate) async fn instance_details(config: &AwsConfig, arn: &str) -> Result<RdsInstanceDetail> {
    validate_instance_arn(arn)?;
    let parts: Vec<_> = arn.splitn(7, ':').collect();
    if !config.region.is_empty() && parts[3] != config.region {
        bail!(
            "DB instance ARN Region {:?} does not match the active Region {:?}",
            parts[3],
            config.region
        );
    }
    if !config.account_id.is_empty() && parts[4] != config.account_id {
        bail!(
            "DB instance ARN account {:?} does not match the active account {:?}",
            parts[4],
            config.account_id
        );
    }
    let out = tokio::time::timeout(
        TIMEOUT,
        config
            .rds_client()
            .describe_db_instances()
            .db_instance_identifier(arn)
            .send(),
    )
    .await
    .map_err(|_| anyhow!("RDS DescribeDBInstances timed out after {TIMEOUT:?}"))?
    .context("RDS DescribeDBInstances request failed")?;
    let instances = out.db_instances();
    let [instance] = instances else {
        bail!(
            "RDS DescribeDBInstances returned {} instances for {arn:?}; expected exactly one",
            instances.len()
        );
    };
    if instance.db_instance_arn() != Some(arn) {
        bail!(
            "RDS DescribeDBInstances returned {:?} for requested ARN {arn:?}; refusing to \
             present a mismatched identity",
            instance.db_instance_arn().unwrap_or_default()
        );
    }
    let mut detail = map(instance, &config.region);
    let groups = detail.security_groups.clone();
    let (compute, security_group_rules, replication) = (
        sections::bounded(
            "Connected compute resources",
            connectivity::compute_resources(config, &groups),
        ),
        sections::bounded(
            "Security group rules",
            connectivity::security_group_rules(config, &groups),
        ),
        sections::bounded("Replication", connectivity::replication(config, instance)),
    );
    let ((cpu, current_activity), mut sections, (vcpu, memory), kms_key, compute, rules, replicas) = tokio::join!(
        enrich_summary(config, instance),
        sections::read(
            config,
            &detail.identifier,
            arn,
            &detail.resource_id,
            &detail.endpoint,
        ),
        class_spec(config, &detail.instance_class),
        kms_alias(config, &detail.master_secret_kms_key),
        compute,
        security_group_rules,
        replication,
    );
    sections.compute = compute;
    sections.security_group_rules = rules;
    sections.replication = replicas;
    detail.cpu = cpu;
    detail.current_activity = current_activity;
    detail.vcpu = vcpu;
    detail.memory = memory;
    detail.master_secret_kms_key = kms_key;
    detail.recommendations = sections::active_count(&sections.recommendations);
    detail.sections = sections;
    Ok(detail)
}

/// CPU and current activity (DBLoad) for the Summary. Reuses the list
/// enrichment (CloudWatch GetMetricData). A slow or failed read marks only its
/// fields "Unavailable"; it never fails or blocks the detail.
async fn enrich_summary(config: &AwsConfig, instance: &DbInstance) -> (String, String) {
    let mut rows = [super::RdsInstance::from_sdk(instance, &config.region)];
    let options = super::InventoryOptions {
        cpu: true,
        current_activity: true,
        ..Default::default()
    };
    let client = super::RdsClient::new(config.clone());
    if tokio::time::timeout(ENRICH_TIMEOUT, client.enrich(&mut rows, options))
        .await
        .is_err()
    {
        return ("Unavailable".into(), "Unavailable".into());
    }
    let [row] = rows;
    (row.cpu, row.current_activity)
}

/// vCPU and memory of the EC2 type behind a `db.` class, as the console shows
/// them: `2` and `1 GB`.
async fn class_spec(config: &AwsConfig, class: &str) -> (String, String) {
    let Some(ec2_type) = class.strip_prefix("db.").filter(|t| t.contains('.')) else {
        return (String::new(), String::new());
    };
    let read = config
        .ec2_client()
        .describe_instance_types()
        .instance_types(aws_sdk_ec2::types::InstanceType::from(ec2_type))
        .send();
    match tokio::time::timeout(ENRICH_TIMEOUT, read).await {
        Ok(Ok(out)) => match out.instance_types().first() {
            Some(t) => (
                t.v_cpu_info()
                    .and_then(|v| v.default_v_cpus())
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                t.memory_info()
                    .and_then(|m| m.size_in_mib())
                    .map(gigabytes)
                    .unwrap_or_default(),
            ),
            None => (String::new(), String::new()),
        },
        Ok(Err(error)) => {
            // An RDS-only class is not an EC2 type: absent, not unavailable.
            let text = format!("{error:#}");
            if text.contains("InvalidInstanceType") {
                return (String::new(), String::new());
            }
            tracing::warn!("EC2 DescribeInstanceTypes for {ec2_type} failed: {text}");
            ("Unavailable".into(), "Unavailable".into())
        }
        Err(_) => ("Unavailable".into(), "Unavailable".into()),
    }
}

/// `1024` MiB -> `1 GB`, `512` -> `0.5 GB`.
pub(crate) fn gigabytes(mib: i64) -> String {
    let gb = mib as f64 / 1024.0;
    let text = format!("{gb:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    format!("{text} GB")
}

/// The alias name of a KMS key, or the key ID itself when it has none or the
/// lookup fails. Read-only ListAliases scoped by the key.
async fn kms_alias(config: &AwsConfig, key: &str) -> String {
    if key.is_empty() {
        return String::new();
    }
    let read = config.kms_client().list_aliases().key_id(key).send();
    match tokio::time::timeout(ENRICH_TIMEOUT, read).await {
        Ok(Ok(out)) => out
            .aliases()
            .iter()
            .filter_map(|a| a.alias_name())
            .next()
            .map(|name| name.trim_start_matches("alias/").to_string())
            .unwrap_or_else(|| key.to_string()),
        _ => key.to_string(),
    }
}

fn names<'a>(values: impl Iterator<Item = Option<&'a str>>) -> Vec<String> {
    values.flatten().map(str::to_string).collect()
}

fn map(i: &DbInstance, region: &str) -> RdsInstanceDetail {
    let row = super::RdsInstance::from_sdk(i, region);
    let text = |v: Option<&str>| v.unwrap_or_default().to_string();
    let group = i.db_subnet_group();
    let mut tags: Vec<(String, String)> = i
        .tag_list()
        .iter()
        .map(|t| (text(t.key()), text(t.value())))
        .collect();
    tags.sort();
    RdsInstanceDetail {
        identifier: row.identifier,
        arn: row.arn,
        resource_id: text(i.dbi_resource_id()),
        status: row.status,
        role: if row.cluster_identifier.is_empty() {
            row.role
        } else {
            format!("Cluster member ({})", row.cluster_identifier)
        },
        engine: row.engine,
        engine_version: row.engine_version,
        instance_class: row.instance_class,
        region: row.region,
        availability_zone: row.availability_zone,
        endpoint: row.endpoint,
        port: row.port,
        vpc: row.vpc,
        subnet_group: row.subnet_group,
        subnets: names(
            group
                .map(|g| g.subnets())
                .unwrap_or_default()
                .iter()
                .map(|s| s.subnet_identifier()),
        ),
        network_type: text(i.network_type()),
        security_groups: i
            .vpc_security_groups()
            .iter()
            .filter_map(|g| {
                let id = g.vpc_security_group_id()?;
                Some(match g.status() {
                    Some(status) => format!("{id} ({status})"),
                    None => id.to_string(),
                })
            })
            .collect(),
        publicly_accessible: i.publicly_accessible(),
        certificate_authority: text(i.ca_certificate_identifier()),
        certificate_expiry: row.certificate_expiry,
        db_name: text(i.db_name()),
        engine_lifecycle_support: text(i.engine_lifecycle_support()),
        license_model: text(i.license_model()),
        option_groups: i
            .option_group_memberships()
            .iter()
            .filter_map(|g| Some((g.option_group_name()?.into(), text(g.status()))))
            .collect(),
        parameter_groups: i
            .db_parameter_groups()
            .iter()
            .filter_map(|g| {
                Some((
                    g.db_parameter_group_name()?.into(),
                    text(g.parameter_apply_status()),
                ))
            })
            .collect(),
        multi_tenant: i.multi_tenant(),
        capacity_reservation: text(i.db_capacity_reservation_identifier()),
        vcpu: String::new(),
        memory: String::new(),
        secondary_zone: text(i.secondary_availability_zone()),
        master_secret_arn: text(i.master_user_secret().and_then(|s| s.secret_arn())),
        master_secret_status: text(i.master_user_secret().and_then(|s| s.secret_status())),
        master_secret_kms_key: text(i.master_user_secret().and_then(|s| s.kms_key_id())),
        storage_upgrade_available: i.is_storage_config_upgrade_available(),
        database_insights_mode: i
            .database_insights_mode()
            .map(|m| m.as_str().to_string())
            .unwrap_or_default(),
        monitoring_interval: i.monitoring_interval(),
        created_time: row.created_time,
        deletion_protection: i.deletion_protection(),
        storage_type: row.storage_type,
        storage_gib: row.storage_gib,
        max_storage_gib: i.max_allocated_storage(),
        iops: row.iops,
        storage_throughput: row.storage_throughput,
        encrypted: row.encrypted,
        kms_key_id: text(i.kms_key_id()),
        multi_az: row.multi_az,
        master_username: text(i.master_username()),
        iam_auth: i.iam_database_authentication_enabled(),
        performance_insights: i.performance_insights_enabled(),
        auto_minor_version_upgrade: i.auto_minor_version_upgrade(),
        maintenance_window: text(i.preferred_maintenance_window()),
        pending_changes: row.pending_changes,
        backup_retention_days: i.backup_retention_period(),
        backup_window: text(i.preferred_backup_window()),
        copy_tags_to_snapshot: i.copy_tags_to_snapshot(),
        latest_restorable_time: timestamp(i.latest_restorable_time()),
        backup_target: text(i.backup_target()),
        tags,
        iam_roles: connectivity::iam_roles(i),
        cpu: String::new(),
        current_activity: String::new(),
        recommendations: String::new(),
        sections: Sections::default(),
    }
}

#[cfg(test)]
mod tests;
