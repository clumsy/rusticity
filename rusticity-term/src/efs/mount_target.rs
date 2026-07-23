use crate::common::{translate_column, ColumnId};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
use rusticity_core::efs::EfsMountTarget as MountTarget;
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for col in Column::all() {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    AvailabilityZone,
    MountTargetId,
    OwnerId,
    FileSystemId,
    SubnetId,
    VpcId,
    State,
    Ipv4Address,
    Ipv6Address,
    NetworkInterfaceId,
    SecurityGroups,
}

impl Column {
    const ID_AVAILABILITY_ZONE: &'static str = "column.efs.mt.availability_zone";
    const ID_MOUNT_TARGET_ID: &'static str = "column.efs.mt.mount_target_id";
    const ID_OWNER_ID: &'static str = "column.efs.mt.owner_id";
    const ID_FILE_SYSTEM_ID: &'static str = "column.efs.mt.file_system_id";
    const ID_SUBNET_ID: &'static str = "column.efs.mt.subnet_id";
    const ID_VPC_ID: &'static str = "column.efs.mt.vpc_id";
    const ID_STATE: &'static str = "column.efs.mt.state";
    const ID_IPV4_ADDRESS: &'static str = "column.efs.mt.ipv4_address";
    const ID_IPV6_ADDRESS: &'static str = "column.efs.mt.ipv6_address";
    const ID_NETWORK_INTERFACE_ID: &'static str = "column.efs.mt.network_interface_id";
    const ID_SECURITY_GROUPS: &'static str = "column.efs.mt.security_groups";

    pub const fn id(&self) -> &'static str {
        match self {
            Column::AvailabilityZone => Self::ID_AVAILABILITY_ZONE,
            Column::MountTargetId => Self::ID_MOUNT_TARGET_ID,
            Column::OwnerId => Self::ID_OWNER_ID,
            Column::FileSystemId => Self::ID_FILE_SYSTEM_ID,
            Column::SubnetId => Self::ID_SUBNET_ID,
            Column::VpcId => Self::ID_VPC_ID,
            Column::State => Self::ID_STATE,
            Column::Ipv4Address => Self::ID_IPV4_ADDRESS,
            Column::Ipv6Address => Self::ID_IPV6_ADDRESS,
            Column::NetworkInterfaceId => Self::ID_NETWORK_INTERFACE_ID,
            Column::SecurityGroups => Self::ID_SECURITY_GROUPS,
        }
    }

    pub const fn default_name(&self) -> &'static str {
        match self {
            Column::AvailabilityZone => "Availability zone (AZ-ID)",
            Column::MountTargetId => "Mount target ID",
            Column::OwnerId => "Owner ID",
            Column::FileSystemId => "File system ID",
            Column::SubnetId => "Subnet ID",
            Column::VpcId => "VPC ID",
            Column::State => "File system state",
            Column::Ipv4Address => "IPv4 address",
            Column::Ipv6Address => "IPv6 address",
            Column::NetworkInterfaceId => "Network interface ID",
            Column::SecurityGroups => "Security groups",
        }
    }

    pub const fn all() -> [Column; 11] {
        [
            Column::AvailabilityZone,
            Column::MountTargetId,
            Column::OwnerId,
            Column::FileSystemId,
            Column::SubnetId,
            Column::VpcId,
            Column::State,
            Column::Ipv4Address,
            Column::Ipv6Address,
            Column::NetworkInterfaceId,
            Column::SecurityGroups,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }

    /// All columns visible by default except Owner ID and File system ID.
    pub fn default_visible_ids() -> Vec<ColumnId> {
        Self::all()
            .iter()
            .filter(|c| !matches!(c, Column::OwnerId | Column::FileSystemId))
            .map(|c| c.id())
            .collect()
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            Self::ID_AVAILABILITY_ZONE => Some(Column::AvailabilityZone),
            Self::ID_MOUNT_TARGET_ID => Some(Column::MountTargetId),
            Self::ID_OWNER_ID => Some(Column::OwnerId),
            Self::ID_FILE_SYSTEM_ID => Some(Column::FileSystemId),
            Self::ID_SUBNET_ID => Some(Column::SubnetId),
            Self::ID_VPC_ID => Some(Column::VpcId),
            Self::ID_STATE => Some(Column::State),
            Self::ID_IPV4_ADDRESS => Some(Column::Ipv4Address),
            Self::ID_IPV6_ADDRESS => Some(Column::Ipv6Address),
            Self::ID_NETWORK_INTERFACE_ID => Some(Column::NetworkInterfaceId),
            Self::ID_SECURITY_GROUPS => Some(Column::SecurityGroups),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }
}

impl TableColumn<MountTarget> for Column {
    fn name(&self) -> &str {
        Box::leak(translate_column(self.id(), self.default_name()).into_boxed_str())
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
        translated.len().max(match self {
            Column::AvailabilityZone => 25,
            Column::MountTargetId => 22,
            Column::OwnerId => 14,
            Column::FileSystemId => 21,
            Column::SubnetId => 24,
            Column::VpcId => 22,
            Column::State => 18,
            Column::Ipv4Address => 15,
            Column::Ipv6Address => 20,
            Column::NetworkInterfaceId => 22,
            Column::SecurityGroups => 24,
        }) as u16
    }

    fn render(&self, item: &MountTarget) -> (String, Style) {
        match self {
            Column::AvailabilityZone => {
                let text = if item.availability_zone_id.is_empty() {
                    item.availability_zone_name.clone()
                } else {
                    format!(
                        "{} ({})",
                        item.availability_zone_name, item.availability_zone_id
                    )
                };
                (text, Style::default())
            }
            Column::MountTargetId => (item.mount_target_id.clone(), Style::default()),
            Column::OwnerId => (item.owner_id.clone(), Style::default()),
            Column::FileSystemId => (item.file_system_id.clone(), Style::default()),
            Column::SubnetId => (item.subnet_id.clone(), Style::default()),
            Column::VpcId => (item.vpc_id.clone(), Style::default()),
            Column::State => {
                let (text, color) = format_state(&item.life_cycle_state);
                (text.to_string(), Style::default().fg(color))
            }
            Column::Ipv4Address => (item.ip_address.clone(), Style::default()),
            Column::Ipv6Address => (item.ipv6_address.clone(), Style::default()),
            Column::NetworkInterfaceId => (item.network_interface_id.clone(), Style::default()),
            Column::SecurityGroups => (item.security_groups.clone(), Style::default()),
        }
    }
}

fn format_state(state: &str) -> (&'static str, Color) {
    match state {
        "available" | "Available" | "AVAILABLE" => ("✅ Available", Color::Green),
        "creating" | "Creating" | "CREATING" => ("⏳ Creating", Color::Yellow),
        "deleting" | "Deleting" | "DELETING" => ("🗑 Deleting", Color::Red),
        "deleted" | "Deleted" | "DELETED" => ("🗑 Deleted", Color::Red),
        "updating" | "Updating" | "UPDATING" => ("🔄 Updating", Color::Yellow),
        "error" | "Error" | "ERROR" => ("⚠ Error", Color::Red),
        _ => ("Unknown", Color::White),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mt_column_ids_have_prefix() {
        for col in Column::all() {
            assert!(col.id().starts_with("column.efs.mt."));
        }
    }

    #[test]
    fn test_mt_from_id_roundtrip() {
        for col in Column::all() {
            assert_eq!(Column::from_id(col.id()), Some(col));
        }
    }

    #[test]
    fn test_mt_owner_and_fs_id_hidden_by_default() {
        let visible = Column::default_visible_ids();
        assert!(!visible.contains(&Column::OwnerId.id()));
        assert!(!visible.contains(&Column::FileSystemId.id()));
        // All others visible.
        assert_eq!(visible.len(), 9);
        assert!(visible.contains(&Column::AvailabilityZone.id()));
        assert!(visible.contains(&Column::SecurityGroups.id()));
    }

    #[test]
    fn test_mt_all_eleven_columns() {
        assert_eq!(Column::all().len(), 11);
    }
}
