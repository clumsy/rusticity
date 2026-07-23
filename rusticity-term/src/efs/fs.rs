use crate::common::{format_iso_timestamp, translate_column, ColumnId, UTC_TIMESTAMP_WIDTH};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for col in Column::all() {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
}

#[derive(Debug, Clone)]
pub struct FileSystem {
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
    pub dns_name: String,
    pub tags: Vec<(String, String)>,
    pub total_size_bytes: i64,
    pub size_in_standard_bytes: i64,
    pub size_in_ia_bytes: i64,
    pub size_in_archive_bytes: i64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    Name,
    FileSystemId,
    CreationToken,
    Encrypted,
    KmsKeyId,
    TotalSize,
    SizeInStandard,
    SizeInIa,
    SizeInArchive,
    ProvisionedThroughput,
    ThroughputMode,
    LifeCycleState,
    NumberOfMountTargets,
    OwnerId,
    CreationTime,
    PerformanceMode,
    AvailabilityZone,
    ReplicationOverwriteProtection,
}

impl Column {
    const ID_NAME: &'static str = "column.efs.fs.name";
    const ID_FILE_SYSTEM_ID: &'static str = "column.efs.fs.file_system_id";
    const ID_CREATION_TOKEN: &'static str = "column.efs.fs.creation_token";
    const ID_ENCRYPTED: &'static str = "column.efs.fs.encrypted";
    const ID_KMS_KEY_ID: &'static str = "column.efs.fs.kms_key_id";
    const ID_TOTAL_SIZE: &'static str = "column.efs.fs.total_size";
    const ID_SIZE_IN_STANDARD: &'static str = "column.efs.fs.size_in_standard";
    const ID_SIZE_IN_IA: &'static str = "column.efs.fs.size_in_ia";
    const ID_SIZE_IN_ARCHIVE: &'static str = "column.efs.fs.size_in_archive";
    const ID_PROVISIONED_THROUGHPUT: &'static str = "column.efs.fs.provisioned_throughput";
    const ID_THROUGHPUT_MODE: &'static str = "column.efs.fs.throughput_mode";
    const ID_LIFE_CYCLE_STATE: &'static str = "column.efs.fs.life_cycle_state";
    const ID_NUMBER_OF_MOUNT_TARGETS: &'static str = "column.efs.fs.number_of_mount_targets";
    const ID_OWNER_ID: &'static str = "column.efs.fs.owner_id";
    const ID_CREATION_TIME: &'static str = "column.efs.fs.creation_time";
    const ID_PERFORMANCE_MODE: &'static str = "column.efs.fs.performance_mode";
    const ID_AVAILABILITY_ZONE: &'static str = "column.efs.fs.availability_zone";
    const ID_REPLICATION_OVERWRITE_PROTECTION: &'static str =
        "column.efs.fs.replication_overwrite_protection";

    pub const fn id(&self) -> &'static str {
        match self {
            Column::Name => Self::ID_NAME,
            Column::FileSystemId => Self::ID_FILE_SYSTEM_ID,
            Column::CreationToken => Self::ID_CREATION_TOKEN,
            Column::Encrypted => Self::ID_ENCRYPTED,
            Column::KmsKeyId => Self::ID_KMS_KEY_ID,
            Column::TotalSize => Self::ID_TOTAL_SIZE,
            Column::SizeInStandard => Self::ID_SIZE_IN_STANDARD,
            Column::SizeInIa => Self::ID_SIZE_IN_IA,
            Column::SizeInArchive => Self::ID_SIZE_IN_ARCHIVE,
            Column::ProvisionedThroughput => Self::ID_PROVISIONED_THROUGHPUT,
            Column::ThroughputMode => Self::ID_THROUGHPUT_MODE,
            Column::LifeCycleState => Self::ID_LIFE_CYCLE_STATE,
            Column::NumberOfMountTargets => Self::ID_NUMBER_OF_MOUNT_TARGETS,
            Column::OwnerId => Self::ID_OWNER_ID,
            Column::CreationTime => Self::ID_CREATION_TIME,
            Column::PerformanceMode => Self::ID_PERFORMANCE_MODE,
            Column::AvailabilityZone => Self::ID_AVAILABILITY_ZONE,
            Column::ReplicationOverwriteProtection => Self::ID_REPLICATION_OVERWRITE_PROTECTION,
        }
    }

    pub const fn default_name(&self) -> &'static str {
        match self {
            Column::Name => "Name",
            Column::FileSystemId => "File system ID",
            Column::CreationToken => "Creation token",
            Column::Encrypted => "Encrypted",
            Column::KmsKeyId => "KMS Key ID",
            Column::TotalSize => "Total size",
            Column::SizeInStandard => "Size in Standard",
            Column::SizeInIa => "Size in IA",
            Column::SizeInArchive => "Size in Archive",
            Column::ProvisionedThroughput => "Provisioned Throughput (MiB/s)",
            Column::ThroughputMode => "Throughput mode",
            Column::LifeCycleState => "File system state",
            Column::NumberOfMountTargets => "Number of mount targets",
            Column::OwnerId => "Owner ID",
            Column::CreationTime => "Creation time",
            Column::PerformanceMode => "Performance mode",
            Column::AvailabilityZone => "Availability Zone",
            Column::ReplicationOverwriteProtection => "Replication overwrite protection",
        }
    }

    pub const fn all() -> [Column; 18] {
        [
            Column::Name,
            Column::FileSystemId,
            Column::CreationToken,
            Column::Encrypted,
            Column::KmsKeyId,
            Column::TotalSize,
            Column::SizeInStandard,
            Column::SizeInIa,
            Column::SizeInArchive,
            Column::ProvisionedThroughput,
            Column::ThroughputMode,
            Column::LifeCycleState,
            Column::NumberOfMountTargets,
            Column::OwnerId,
            Column::CreationTime,
            Column::PerformanceMode,
            Column::AvailabilityZone,
            Column::ReplicationOverwriteProtection,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }

    /// Default visible columns — excludes: ThroughputMode, PerformanceMode,
    /// NumberOfMountTargets, OwnerId, CreationToken, KmsKeyId
    pub fn default_visible_ids() -> Vec<ColumnId> {
        vec![
            Self::ID_NAME,
            Self::ID_FILE_SYSTEM_ID,
            Self::ID_ENCRYPTED,
            Self::ID_TOTAL_SIZE,
            Self::ID_SIZE_IN_STANDARD,
            Self::ID_SIZE_IN_IA,
            Self::ID_SIZE_IN_ARCHIVE,
            Self::ID_PROVISIONED_THROUGHPUT,
            Self::ID_LIFE_CYCLE_STATE,
            Self::ID_CREATION_TIME,
            Self::ID_AVAILABILITY_ZONE,
            Self::ID_REPLICATION_OVERWRITE_PROTECTION,
        ]
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            Self::ID_NAME => Some(Column::Name),
            Self::ID_FILE_SYSTEM_ID => Some(Column::FileSystemId),
            Self::ID_CREATION_TOKEN => Some(Column::CreationToken),
            Self::ID_ENCRYPTED => Some(Column::Encrypted),
            Self::ID_KMS_KEY_ID => Some(Column::KmsKeyId),
            Self::ID_TOTAL_SIZE => Some(Column::TotalSize),
            Self::ID_SIZE_IN_STANDARD => Some(Column::SizeInStandard),
            Self::ID_SIZE_IN_IA => Some(Column::SizeInIa),
            Self::ID_SIZE_IN_ARCHIVE => Some(Column::SizeInArchive),
            Self::ID_PROVISIONED_THROUGHPUT => Some(Column::ProvisionedThroughput),
            Self::ID_THROUGHPUT_MODE => Some(Column::ThroughputMode),
            Self::ID_LIFE_CYCLE_STATE => Some(Column::LifeCycleState),
            Self::ID_NUMBER_OF_MOUNT_TARGETS => Some(Column::NumberOfMountTargets),
            Self::ID_OWNER_ID => Some(Column::OwnerId),
            Self::ID_CREATION_TIME => Some(Column::CreationTime),
            Self::ID_PERFORMANCE_MODE => Some(Column::PerformanceMode),
            Self::ID_AVAILABILITY_ZONE => Some(Column::AvailabilityZone),
            Self::ID_REPLICATION_OVERWRITE_PROTECTION => {
                Some(Column::ReplicationOverwriteProtection)
            }
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }
}

impl TableColumn<FileSystem> for Column {
    fn name(&self) -> &str {
        Box::leak(translate_column(self.id(), self.default_name()).into_boxed_str())
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
        translated.len().max(match self {
            Column::Name => 30,
            Column::FileSystemId => 21,
            Column::CreationToken => 20,
            Column::Encrypted => 10,
            Column::KmsKeyId => 30,
            Column::TotalSize => 12,
            Column::SizeInStandard => 16,
            Column::SizeInIa => 10,
            Column::SizeInArchive => 14,
            Column::ProvisionedThroughput => 30,
            Column::ThroughputMode => 16,
            Column::LifeCycleState => 16,
            Column::NumberOfMountTargets => 23,
            Column::OwnerId => 14,
            Column::CreationTime => UTC_TIMESTAMP_WIDTH as usize,
            Column::PerformanceMode => 16,
            Column::AvailabilityZone => 20,
            Column::ReplicationOverwriteProtection => 34,
        }) as u16
    }

    fn render(&self, item: &FileSystem) -> (String, Style) {
        let (text, style) = match self {
            Column::Name => (item.name.clone(), Style::default()),
            Column::FileSystemId => (item.file_system_id.clone(), Style::default()),
            Column::CreationToken => (item.creation_token.clone(), Style::default()),
            Column::Encrypted => {
                let color = match item.encrypted.as_str() {
                    "Yes" => Color::Green,
                    "No" => Color::Yellow,
                    _ => Color::White,
                };
                (item.encrypted.clone(), Style::default().fg(color))
            }
            Column::KmsKeyId => (item.kms_key_id.clone(), Style::default()),
            Column::TotalSize => (item.total_size.clone(), Style::default()),
            Column::SizeInStandard => (item.size_in_standard.clone(), Style::default()),
            Column::SizeInIa => (item.size_in_ia.clone(), Style::default()),
            Column::SizeInArchive => (item.size_in_archive.clone(), Style::default()),
            Column::ProvisionedThroughput => {
                (item.provisioned_throughput.clone(), Style::default())
            }
            Column::ThroughputMode => (
                format_throughput_mode(&item.throughput_mode),
                Style::default(),
            ),
            Column::LifeCycleState => {
                let (text, color) = format_life_cycle_state(&item.life_cycle_state);
                (text.to_string(), Style::default().fg(color))
            }
            Column::NumberOfMountTargets => {
                (item.number_of_mount_targets.clone(), Style::default())
            }
            Column::OwnerId => (item.owner_id.clone(), Style::default()),
            Column::CreationTime => (format_iso_timestamp(&item.creation_time), Style::default()),
            Column::PerformanceMode => (
                format_performance_mode(&item.performance_mode),
                Style::default(),
            ),
            Column::AvailabilityZone => (item.availability_zone.clone(), Style::default()),
            Column::ReplicationOverwriteProtection => {
                let text = format_replication_protection(&item.replication_overwrite_protection);
                let color = match item.replication_overwrite_protection.as_str() {
                    "Enabled" | "ENABLED" => Color::Green,
                    "Disabled" | "DISABLED" => Color::Yellow,
                    "Replicating" | "REPLICATING" => Color::Cyan,
                    _ => Color::White,
                };
                (text, Style::default().fg(color))
            }
        };
        (text, style)
    }
}

fn format_life_cycle_state(state: &str) -> (&'static str, Color) {
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

fn format_throughput_mode(mode: &str) -> String {
    match mode {
        "Bursting" => "Bursting".to_string(),
        "Provisioned" => "Provisioned".to_string(),
        "Elastic" => "Elastic".to_string(),
        s => s.to_string(),
    }
}

fn format_performance_mode(mode: &str) -> String {
    match mode {
        "GeneralPurpose" => "General Purpose".to_string(),
        "MaxIo" => "Max I/O".to_string(),
        s => s.to_string(),
    }
}

fn format_replication_protection(val: &str) -> String {
    match val {
        "Enabled" => "Enabled".to_string(),
        "Disabled" => "Disabled".to_string(),
        "Replicating" => "Replicating".to_string(),
        s => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_ids_have_correct_prefix() {
        for col in Column::all() {
            assert!(
                col.id().starts_with("column.efs.fs."),
                "Column ID '{}' should start with 'column.efs.fs.'",
                col.id()
            );
        }
    }

    #[test]
    fn test_from_id_roundtrip() {
        for col in Column::all() {
            assert_eq!(Column::from_id(col.id()), Some(col));
        }
    }

    #[test]
    fn test_default_visible_excludes_specified_columns() {
        let ids = Column::default_visible_ids();
        assert!(
            !ids.contains(&Column::ThroughputMode.id()),
            "ThroughputMode not default"
        );
        assert!(
            !ids.contains(&Column::PerformanceMode.id()),
            "PerformanceMode not default"
        );
        assert!(
            !ids.contains(&Column::NumberOfMountTargets.id()),
            "NumberOfMountTargets not default"
        );
        assert!(!ids.contains(&Column::OwnerId.id()), "OwnerId not default");
        assert!(
            !ids.contains(&Column::CreationToken.id()),
            "CreationToken not default"
        );
        assert!(
            !ids.contains(&Column::KmsKeyId.id()),
            "KmsKeyId not default"
        );
    }

    #[test]
    fn test_default_visible_includes_core_columns() {
        let ids = Column::default_visible_ids();
        assert!(ids.contains(&Column::Name.id()));
        assert!(ids.contains(&Column::FileSystemId.id()));
        assert!(ids.contains(&Column::LifeCycleState.id()));
        assert!(ids.contains(&Column::CreationTime.id()));
        assert!(ids.contains(&Column::TotalSize.id()));
    }

    #[test]
    fn test_all_18_columns() {
        assert_eq!(Column::all().len(), 18);
    }
}
