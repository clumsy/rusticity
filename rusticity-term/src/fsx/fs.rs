use crate::common::{format_iso_timestamp, translate_column, ColumnId, UTC_TIMESTAMP_WIDTH};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
use rusticity_core::fsx::FsxFileSystem as FileSystem;
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for col in Column::all() {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    Name,
    FileSystemId,
    FileSystemType,
    Status,
    DeploymentType,
    StorageClass,
    StorageCapacity,
    ThroughputCapacity,
    ProvisionedIops,
    CreationTime,
    LustreVersion,
    Vpc,
    KmsKeyId,
}

impl Column {
    const ID_NAME: &'static str = "column.fsx.fs.name";
    const ID_FILE_SYSTEM_ID: &'static str = "column.fsx.fs.file_system_id";
    const ID_FILE_SYSTEM_TYPE: &'static str = "column.fsx.fs.file_system_type";
    const ID_STATUS: &'static str = "column.fsx.fs.status";
    const ID_DEPLOYMENT_TYPE: &'static str = "column.fsx.fs.deployment_type";
    const ID_STORAGE_CLASS: &'static str = "column.fsx.fs.storage_class";
    const ID_STORAGE_CAPACITY: &'static str = "column.fsx.fs.storage_capacity";
    const ID_THROUGHPUT_CAPACITY: &'static str = "column.fsx.fs.throughput_capacity";
    const ID_PROVISIONED_IOPS: &'static str = "column.fsx.fs.provisioned_iops";
    const ID_CREATION_TIME: &'static str = "column.fsx.fs.creation_time";
    const ID_LUSTRE_VERSION: &'static str = "column.fsx.fs.lustre_version";
    const ID_VPC: &'static str = "column.fsx.fs.vpc";
    const ID_KMS_KEY_ID: &'static str = "column.fsx.fs.kms_key_id";

    pub const fn id(&self) -> &'static str {
        match self {
            Column::Name => Self::ID_NAME,
            Column::FileSystemId => Self::ID_FILE_SYSTEM_ID,
            Column::FileSystemType => Self::ID_FILE_SYSTEM_TYPE,
            Column::Status => Self::ID_STATUS,
            Column::DeploymentType => Self::ID_DEPLOYMENT_TYPE,
            Column::StorageClass => Self::ID_STORAGE_CLASS,
            Column::StorageCapacity => Self::ID_STORAGE_CAPACITY,
            Column::ThroughputCapacity => Self::ID_THROUGHPUT_CAPACITY,
            Column::ProvisionedIops => Self::ID_PROVISIONED_IOPS,
            Column::CreationTime => Self::ID_CREATION_TIME,
            Column::LustreVersion => Self::ID_LUSTRE_VERSION,
            Column::Vpc => Self::ID_VPC,
            Column::KmsKeyId => Self::ID_KMS_KEY_ID,
        }
    }

    pub const fn default_name(&self) -> &'static str {
        match self {
            Column::Name => "File system name",
            Column::FileSystemId => "File system ID",
            Column::FileSystemType => "File system type",
            Column::Status => "Status",
            Column::DeploymentType => "Deployment type",
            Column::StorageClass => "Storage class",
            Column::StorageCapacity => "Storage capacity",
            Column::ThroughputCapacity => "Throughput capacity",
            Column::ProvisionedIops => "Provisioned IOPS",
            Column::CreationTime => "Creation time",
            Column::LustreVersion => "File system Lustre version",
            Column::Vpc => "VPC",
            Column::KmsKeyId => "KMS key ID",
        }
    }

    pub const fn all() -> [Column; 13] {
        [
            Column::Name,
            Column::FileSystemId,
            Column::FileSystemType,
            Column::Status,
            Column::DeploymentType,
            Column::StorageClass,
            Column::StorageCapacity,
            Column::ThroughputCapacity,
            Column::ProvisionedIops,
            Column::CreationTime,
            Column::LustreVersion,
            Column::Vpc,
            Column::KmsKeyId,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }

    /// All columns visible by default except Provisioned IOPS, Lustre version,
    /// VPC, and KMS key ID.
    pub fn default_visible_ids() -> Vec<ColumnId> {
        Self::all()
            .iter()
            .filter(|c| {
                !matches!(
                    c,
                    Column::ProvisionedIops
                        | Column::LustreVersion
                        | Column::Vpc
                        | Column::KmsKeyId
                )
            })
            .map(|c| c.id())
            .collect()
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            Self::ID_NAME => Some(Column::Name),
            Self::ID_FILE_SYSTEM_ID => Some(Column::FileSystemId),
            Self::ID_FILE_SYSTEM_TYPE => Some(Column::FileSystemType),
            Self::ID_STATUS => Some(Column::Status),
            Self::ID_DEPLOYMENT_TYPE => Some(Column::DeploymentType),
            Self::ID_STORAGE_CLASS => Some(Column::StorageClass),
            Self::ID_STORAGE_CAPACITY => Some(Column::StorageCapacity),
            Self::ID_THROUGHPUT_CAPACITY => Some(Column::ThroughputCapacity),
            Self::ID_PROVISIONED_IOPS => Some(Column::ProvisionedIops),
            Self::ID_CREATION_TIME => Some(Column::CreationTime),
            Self::ID_LUSTRE_VERSION => Some(Column::LustreVersion),
            Self::ID_VPC => Some(Column::Vpc),
            Self::ID_KMS_KEY_ID => Some(Column::KmsKeyId),
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
            Column::Name => 24,
            Column::FileSystemId => 21,
            Column::FileSystemType => 16,
            Column::Status => 14,
            Column::DeploymentType => 18,
            Column::StorageClass => 13,
            Column::StorageCapacity => 16,
            Column::ThroughputCapacity => 19,
            Column::ProvisionedIops => 16,
            Column::CreationTime => UTC_TIMESTAMP_WIDTH as usize,
            Column::LustreVersion => 26,
            Column::Vpc => 21,
            Column::KmsKeyId => 30,
        }) as u16
    }

    fn render(&self, item: &FileSystem) -> (String, Style) {
        match self {
            Column::Name => (item.name.clone(), Style::default()),
            Column::FileSystemId => (item.file_system_id.clone(), Style::default()),
            Column::FileSystemType => (item.file_system_type.clone(), Style::default()),
            Column::Status => {
                let (text, color) = format_status(&item.status);
                (text.to_string(), Style::default().fg(color))
            }
            Column::DeploymentType => (
                crate::ui::fsx::format_deployment_type(&item.deployment_type),
                Style::default(),
            ),
            Column::StorageClass => (
                crate::ui::fsx::format_storage_class(&item.storage_class),
                Style::default(),
            ),
            Column::StorageCapacity => (item.storage_capacity.clone(), Style::default()),
            Column::ThroughputCapacity => (item.throughput_capacity.clone(), Style::default()),
            Column::ProvisionedIops => (item.provisioned_iops.clone(), Style::default()),
            Column::CreationTime => (format_iso_timestamp(&item.creation_time), Style::default()),
            Column::LustreVersion => (item.lustre_version.clone(), Style::default()),
            Column::Vpc => (item.vpc_id.clone(), Style::default()),
            Column::KmsKeyId => (item.kms_key_id.clone(), Style::default()),
        }
    }
}

fn format_status(status: &str) -> (&'static str, Color) {
    match status {
        "AVAILABLE" | "Available" | "available" => ("✅ Available", Color::Green),
        "CREATING" | "Creating" | "creating" => ("⏳ Creating", Color::Yellow),
        "UPDATING" | "Updating" | "updating" => ("🔄 Updating", Color::Yellow),
        "DELETING" | "Deleting" | "deleting" => ("🗑 Deleting", Color::Red),
        "FAILED" | "Failed" | "failed" => ("⚠ Failed", Color::Red),
        "MISCONFIGURED" | "Misconfigured" | "MISCONFIGURED_UNAVAILABLE" => {
            ("⚠ Misconfigured", Color::Red)
        }
        _ => ("Unknown", Color::White),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_ids_have_prefix() {
        for col in Column::all() {
            assert!(col.id().starts_with("column.fsx.fs."));
        }
    }

    #[test]
    fn test_from_id_roundtrip() {
        for col in Column::all() {
            assert_eq!(Column::from_id(col.id()), Some(col));
        }
    }

    #[test]
    fn test_all_thirteen_columns() {
        assert_eq!(Column::all().len(), 13);
    }

    #[test]
    fn test_default_visible_hides_iops_lustre_vpc_kms() {
        let visible = Column::default_visible_ids();
        assert!(!visible.contains(&Column::ProvisionedIops.id()));
        assert!(!visible.contains(&Column::LustreVersion.id()));
        assert!(!visible.contains(&Column::Vpc.id()));
        assert!(!visible.contains(&Column::KmsKeyId.id()));
        // The remaining nine are visible by default.
        assert_eq!(visible.len(), 9);
        assert!(visible.contains(&Column::Name.id()));
        assert!(visible.contains(&Column::Status.id()));
        assert!(visible.contains(&Column::StorageCapacity.id()));
        assert!(visible.contains(&Column::CreationTime.id()));
    }

    #[test]
    fn test_columns_reuse_summary_formatters() {
        // The list table must render like the Summary: Ssd -> SSD, Persistent2 -> Persistent 2.
        let fs = FileSystem {
            storage_class: "Ssd".to_string(),
            deployment_type: "Persistent2".to_string(),
            ..Default::default()
        };

        let (sc, _) = TableColumn::render(&Column::StorageClass, &fs);
        assert_eq!(sc, "SSD");
        let (dt, _) = TableColumn::render(&Column::DeploymentType, &fs);
        assert_eq!(dt, "Persistent 2");
    }
}
