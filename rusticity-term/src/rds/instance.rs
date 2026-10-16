use crate::common::{format_iso_timestamp, translate_column, ColumnId};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
use rusticity_core::rds::{InventoryOptions, RdsInstance};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Identifier,
    Status,
    ClusterIdentifier,
    Role,
    Engine,
    EngineVersion,
    UpgradeRolloutOrder,
    ExtendedSupport,
    RegionAz,
    Class,
    Recommendations,
    Cpu,
    CurrentActivity,
    Maintenance,
    Vpc,
    MultiAz,
    StorageType,
    Storage,
    Iops,
    StorageThroughput,
    SecurityGroups,
    SubnetGroup,
    CertificateExpiry,
    PendingChanges,
    CharacterSet,
    OptionGroup,
    CreatedTime,
    Encrypted,
    PriorityTier,
    ReplicaLag,
    ReplicaState,
    ParameterGroup,
    CloneGroupId,
    Endpoint,
    Port,
    Arn,
}

impl Column {
    pub const ALL: [Self; 36] = [
        Self::Identifier,
        Self::Status,
        Self::ClusterIdentifier,
        Self::Role,
        Self::Engine,
        Self::EngineVersion,
        Self::UpgradeRolloutOrder,
        Self::ExtendedSupport,
        Self::RegionAz,
        Self::Class,
        Self::Recommendations,
        Self::Cpu,
        Self::CurrentActivity,
        Self::Maintenance,
        Self::Vpc,
        Self::MultiAz,
        Self::StorageType,
        Self::Storage,
        Self::Iops,
        Self::StorageThroughput,
        Self::SecurityGroups,
        Self::SubnetGroup,
        Self::CertificateExpiry,
        Self::PendingChanges,
        Self::CharacterSet,
        Self::OptionGroup,
        Self::CreatedTime,
        Self::Encrypted,
        Self::PriorityTier,
        Self::ReplicaLag,
        Self::ReplicaState,
        Self::ParameterGroup,
        Self::CloneGroupId,
        Self::Endpoint,
        Self::Port,
        Self::Arn,
    ];

    fn definition(self) -> (ColumnId, &'static str, usize) {
        match self {
            Self::Identifier => ("column.rds.instance.identifier", "DB identifier", 26),
            Self::Status => ("column.rds.instance.status", "Status", 18),
            Self::ClusterIdentifier => (
                "column.rds.instance.cluster_identifier",
                "DB cluster identifier",
                26,
            ),
            Self::Role => ("column.rds.instance.role", "Role", 14),
            Self::Engine => ("column.rds.instance.engine", "Engine", 18),
            Self::EngineVersion => ("column.rds.instance.engine_version", "Engine version", 14),
            Self::UpgradeRolloutOrder => (
                "column.rds.instance.upgrade_rollout_order",
                "Upgrade rollout order",
                21,
            ),
            Self::ExtendedSupport => (
                "column.rds.instance.extended_support",
                "RDS Extended Support",
                20,
            ),
            Self::RegionAz => ("column.rds.instance.region_az", "Region & AZ", 25),
            Self::Class => ("column.rds.instance.class", "Size", 18),
            Self::Recommendations => ("column.rds.instance.recommendations", "Recommendations", 16),
            Self::Cpu => ("column.rds.instance.cpu", "CPU", 10),
            Self::CurrentActivity => (
                "column.rds.instance.current_activity",
                "Current activity",
                18,
            ),
            Self::Maintenance => ("column.rds.instance.maintenance", "Maintenance", 22),
            Self::Vpc => ("column.rds.instance.vpc", "VPC", 24),
            Self::MultiAz => ("column.rds.instance.multi_az", "Multi-AZ", 10),
            Self::StorageType => ("column.rds.instance.storage_type", "Storage type", 14),
            Self::Storage => ("column.rds.instance.storage", "Storage", 14),
            Self::Iops => ("column.rds.instance.iops", "Provisioned IOPS", 16),
            Self::StorageThroughput => (
                "column.rds.instance.storage_throughput",
                "Storage throughput",
                20,
            ),
            Self::SecurityGroups => ("column.rds.instance.security_groups", "Security groups", 24),
            Self::SubnetGroup => (
                "column.rds.instance.subnet_group",
                "DB subnet group name",
                24,
            ),
            Self::CertificateExpiry => (
                "column.rds.instance.certificate_expiry",
                "DB Certificate Expiry",
                27,
            ),
            Self::PendingChanges => ("column.rds.instance.pending_changes", "Pending changes", 22),
            Self::CharacterSet => ("column.rds.instance.character_set", "Character set", 14),
            Self::OptionGroup => ("column.rds.instance.option_group", "Option group", 24),
            Self::CreatedTime => ("column.rds.instance.created_time", "Created time", 27),
            Self::Encrypted => ("column.rds.instance.encrypted", "Encrypted", 12),
            Self::PriorityTier => ("column.rds.instance.priority_tier", "Priority tier", 14),
            Self::ReplicaLag => ("column.rds.instance.replica_lag", "Replica lag", 14),
            Self::ReplicaState => ("column.rds.instance.replica_state", "Replica State", 18),
            Self::ParameterGroup => ("column.rds.instance.parameter_group", "Parameter group", 24),
            Self::CloneGroupId => ("column.rds.instance.clone_group_id", "Clone group ID", 26),
            Self::Endpoint => ("column.rds.instance.endpoint", "Endpoint", 48),
            Self::Port => ("column.rds.instance.port", "Port", 7),
            Self::Arn => ("column.rds.instance.arn", "ARN", 48),
        }
    }
    pub fn id(self) -> ColumnId {
        self.definition().0
    }
    pub fn default_name(self) -> &'static str {
        self.definition().1
    }
    pub fn name(self) -> String {
        translate_column(self.id(), self.default_name())
    }
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "column.rds.instance.region" | "column.rds.instance.availability_zone" => {
                Some(Self::RegionAz)
            }
            _ => Self::ALL.into_iter().find(|column| column.id() == id),
        }
    }
    pub fn ids() -> Vec<ColumnId> {
        Self::ALL.into_iter().map(Self::id).collect()
    }
    pub fn default_visible_ids() -> Vec<ColumnId> {
        [
            Self::Identifier,
            Self::Status,
            Self::Engine,
            Self::EngineVersion,
            Self::RegionAz,
            Self::Class,
            Self::Endpoint,
        ]
        .into_iter()
        .map(Self::id)
        .collect()
    }

    pub fn inventory_options(ids: &[ColumnId]) -> InventoryOptions {
        let has = |column: Self| ids.contains(&column.id());
        InventoryOptions {
            clusters: has(Self::Role) || has(Self::CloneGroupId) || has(Self::ReplicaLag),
            recommendations: has(Self::Recommendations),
            maintenance: has(Self::Maintenance),
            cpu: has(Self::Cpu),
            current_activity: has(Self::CurrentActivity),
            replica_lag: has(Self::ReplicaLag),
        }
    }

    pub fn render(self, instance: &RdsInstance) -> (String, Style) {
        if self == Self::Status {
            let (value, color) = format_status(&instance.status);
            return (value, Style::default().fg(color));
        }
        let value = match self {
            Self::Identifier => instance.identifier.clone(),
            Self::Status => instance.status.clone(),
            Self::ClusterIdentifier => instance.cluster_identifier.clone(),
            Self::Role => instance.role.clone(),
            Self::Engine => instance.engine.clone(),
            Self::EngineVersion => instance.engine_version.clone(),
            Self::UpgradeRolloutOrder => instance.upgrade_rollout_order.clone(),
            Self::ExtendedSupport => instance.extended_support.clone(),
            Self::RegionAz => [
                instance.region.as_str(),
                instance.availability_zone.as_str(),
            ]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" / "),
            Self::Class => instance.instance_class.clone(),
            Self::Recommendations => instance.recommendations.clone(),
            Self::Cpu => instance.cpu.clone(),
            Self::CurrentActivity => instance.current_activity.clone(),
            Self::Maintenance => instance.maintenance.clone(),
            Self::Vpc => instance.vpc.clone(),
            Self::MultiAz => yes_no(instance.multi_az),
            Self::StorageType => instance.storage_type.clone(),
            Self::Storage => instance
                .storage_gib
                .map(|v| format!("{v} GiB"))
                .unwrap_or_default(),
            Self::Iops => number(instance.iops),
            Self::StorageThroughput => instance
                .storage_throughput
                .map(|v| format!("{v} MiB/s"))
                .unwrap_or_default(),
            Self::SecurityGroups => instance.security_groups.clone(),
            Self::SubnetGroup => instance.subnet_group.clone(),
            Self::CertificateExpiry => format_iso_timestamp(&instance.certificate_expiry),
            Self::PendingChanges => instance.pending_changes.clone(),
            Self::CharacterSet => instance.character_set.clone(),
            Self::OptionGroup => instance.option_group.clone(),
            Self::CreatedTime => format_iso_timestamp(&instance.created_time),
            Self::Encrypted => yes_no(instance.encrypted),
            Self::PriorityTier => number(instance.priority_tier),
            Self::ReplicaLag => instance.replica_lag.clone(),
            Self::ReplicaState => instance.replica_state.clone(),
            Self::ParameterGroup => instance.parameter_group.clone(),
            Self::CloneGroupId => instance.clone_group_id.clone(),
            Self::Endpoint => instance.endpoint.clone(),
            Self::Port => number(instance.port),
            Self::Arn => instance.arn.clone(),
        };
        let color = if value.is_empty() || value == "-" {
            Color::DarkGray
        } else if value == "Unavailable" {
            Color::Yellow
        } else if matches!(
            self,
            Self::Encrypted | Self::MultiAz | Self::ExtendedSupport
        ) {
            match value.as_str() {
                "Yes" | "Enabled" => Color::Green,
                "No" | "Disabled" => Color::Yellow,
                _ => Color::White,
            }
        } else {
            Color::White
        };
        (
            if value.is_empty() { "-".into() } else { value },
            Style::default().fg(color),
        )
    }

    /// Own the translated label rather than leaking an allocation for name().
    pub fn into_table_column(self) -> Box<dyn TableColumn<RdsInstance>> {
        Box::new(InstanceColumn {
            column: self,
            label: self.name(),
        })
    }
}

fn number(value: Option<i32>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}
fn yes_no(value: Option<bool>) -> String {
    value
        .map(|v| if v { "Yes" } else { "No" }.into())
        .unwrap_or_default()
}

struct InstanceColumn {
    column: Column,
    label: String,
}
impl TableColumn<RdsInstance> for InstanceColumn {
    fn name(&self) -> std::borrow::Cow<'_, str> {
        self.label.as_str().into()
    }
    fn width(&self) -> u16 {
        self.label
            .chars()
            .count()
            .max(self.column.definition().2)
            .min(u16::MAX as usize) as u16
    }
    fn render(&self, item: &RdsInstance) -> (String, Style) {
        self.column.render(item)
    }
}

pub fn format_status(status: &str) -> (String, Color) {
    let (label, color) = match status.to_ascii_lowercase().as_str() {
        "available" => ("✅ Available", Color::Green),
        "creating" => ("⏳ Creating", Color::Yellow),
        "starting" => ("⏳ Starting", Color::Yellow),
        "stopping" => ("⏳ Stopping", Color::Yellow),
        "stopped" => ("Stopped", Color::DarkGray),
        "modifying" => ("🔄 Modifying", Color::Yellow),
        "rebooting" => ("🔄 Rebooting", Color::Yellow),
        "backing-up" => ("Backing up", Color::Yellow),
        "maintenance" => ("Maintenance", Color::Yellow),
        "upgrading" => ("🔄 Upgrading", Color::Yellow),
        "deleting" => ("🗑 Deleting", Color::Red),
        "failed" => ("⚠ Failed", Color::Red),
        "incompatible-restore"
        | "incompatible-network"
        | "inaccessible-encryption-credentials"
        | "storage-full" => return (status.into(), Color::Red),
        "" => ("-", Color::DarkGray),
        _ => return (status.into(), Color::White),
    };
    (label.into(), color)
}

pub fn status_color(status: &str) -> Color {
    format_status(status).1
}
