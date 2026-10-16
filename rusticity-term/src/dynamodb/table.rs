use crate::common::{format_bytes, translate_column, ColumnId};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
use rusticity_core::dynamodb::DdbTable as Table;
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
    Status,
    PartitionKey,
    SortKey,
    Indexes,
    ReplicationRegions,
    DeletionProtection,
    Favorite,
    ReadCapacityMode,
    WriteCapacityMode,
    TotalSize,
    TableClass,
}

impl Column {
    const ID_NAME: &'static str = "column.dynamodb.table.name";
    const ID_STATUS: &'static str = "column.dynamodb.table.status";
    const ID_PARTITION_KEY: &'static str = "column.dynamodb.table.partition_key";
    const ID_SORT_KEY: &'static str = "column.dynamodb.table.sort_key";
    const ID_INDEXES: &'static str = "column.dynamodb.table.indexes";
    const ID_REPLICATION_REGIONS: &'static str = "column.dynamodb.table.replication_regions";
    const ID_DELETION_PROTECTION: &'static str = "column.dynamodb.table.deletion_protection";
    const ID_FAVORITE: &'static str = "column.dynamodb.table.favorite";
    const ID_READ_CAPACITY_MODE: &'static str = "column.dynamodb.table.read_capacity_mode";
    const ID_WRITE_CAPACITY_MODE: &'static str = "column.dynamodb.table.write_capacity_mode";
    const ID_TOTAL_SIZE: &'static str = "column.dynamodb.table.total_size";
    const ID_TABLE_CLASS: &'static str = "column.dynamodb.table.table_class";

    pub const fn id(&self) -> &'static str {
        match self {
            Column::Name => Self::ID_NAME,
            Column::Status => Self::ID_STATUS,
            Column::PartitionKey => Self::ID_PARTITION_KEY,
            Column::SortKey => Self::ID_SORT_KEY,
            Column::Indexes => Self::ID_INDEXES,
            Column::ReplicationRegions => Self::ID_REPLICATION_REGIONS,
            Column::DeletionProtection => Self::ID_DELETION_PROTECTION,
            Column::Favorite => Self::ID_FAVORITE,
            Column::ReadCapacityMode => Self::ID_READ_CAPACITY_MODE,
            Column::WriteCapacityMode => Self::ID_WRITE_CAPACITY_MODE,
            Column::TotalSize => Self::ID_TOTAL_SIZE,
            Column::TableClass => Self::ID_TABLE_CLASS,
        }
    }

    pub const fn default_name(&self) -> &'static str {
        match self {
            Column::Name => "Name",
            Column::Status => "Status",
            Column::PartitionKey => "Partition key",
            Column::SortKey => "Sort key",
            Column::Indexes => "Indexes",
            Column::ReplicationRegions => "Replication Regions",
            Column::DeletionProtection => "Deletion protection",
            Column::Favorite => "Favorite",
            Column::ReadCapacityMode => "Read capacity mode",
            Column::WriteCapacityMode => "Write capacity mode",
            Column::TotalSize => "Total size",
            Column::TableClass => "Table class",
        }
    }

    pub const fn all() -> [Column; 12] {
        [
            Column::Name,
            Column::Status,
            Column::PartitionKey,
            Column::SortKey,
            Column::Indexes,
            Column::ReplicationRegions,
            Column::DeletionProtection,
            Column::Favorite,
            Column::ReadCapacityMode,
            Column::WriteCapacityMode,
            Column::TotalSize,
            Column::TableClass,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }

    /// Every column is visible by default, matching the DynamoDB console.
    pub fn default_visible_ids() -> Vec<ColumnId> {
        Self::ids()
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            Self::ID_NAME => Some(Column::Name),
            Self::ID_STATUS => Some(Column::Status),
            Self::ID_PARTITION_KEY => Some(Column::PartitionKey),
            Self::ID_SORT_KEY => Some(Column::SortKey),
            Self::ID_INDEXES => Some(Column::Indexes),
            Self::ID_REPLICATION_REGIONS => Some(Column::ReplicationRegions),
            Self::ID_DELETION_PROTECTION => Some(Column::DeletionProtection),
            Self::ID_FAVORITE => Some(Column::Favorite),
            Self::ID_READ_CAPACITY_MODE => Some(Column::ReadCapacityMode),
            Self::ID_WRITE_CAPACITY_MODE => Some(Column::WriteCapacityMode),
            Self::ID_TOTAL_SIZE => Some(Column::TotalSize),
            Self::ID_TABLE_CLASS => Some(Column::TableClass),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }
}

impl TableColumn<Table> for Column {
    fn name(&self) -> std::borrow::Cow<'_, str> {
        translate_column(self.id(), self.default_name()).into()
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
        translated.len().max(match self {
            Column::Name => 30,
            Column::Status => 12,
            Column::PartitionKey => 20,
            Column::SortKey => 20,
            Column::Indexes => 8,
            Column::ReplicationRegions => 20,
            Column::DeletionProtection => 19,
            Column::Favorite => 8,
            Column::ReadCapacityMode => 18,
            Column::WriteCapacityMode => 19,
            Column::TotalSize => 12,
            Column::TableClass => 12,
        }) as u16
    }

    fn render(&self, item: &Table) -> (String, Style) {
        match self {
            Column::Name => (item.name.clone(), Style::default()),
            Column::Status => {
                let (text, color) = format_status(&item.status);
                (text.to_string(), Style::default().fg(color))
            }
            Column::PartitionKey => (item.partition_key.clone(), Style::default()),
            Column::SortKey => (dash_if_empty(&item.sort_key), Style::default()),
            Column::Indexes => (item.index_count().to_string(), Style::default()),
            Column::ReplicationRegions => {
                (dash_if_empty(&item.replication_regions()), Style::default())
            }
            Column::DeletionProtection => (
                item.deletion_protection.clone(),
                Style::default().fg(crate::ui::dynamodb::enabled_disabled_color(
                    &item.deletion_protection,
                )),
            ),
            // Favorites are not persisted yet — always render the empty star.
            Column::Favorite => (
                "☆".to_string(),
                Style::default().fg(ratatui::style::Color::DarkGray),
            ),
            Column::ReadCapacityMode => (item.read_capacity_mode.clone(), Style::default()),
            Column::WriteCapacityMode => (item.write_capacity_mode.clone(), Style::default()),
            Column::TotalSize => (format_bytes(item.total_size_bytes), Style::default()),
            Column::TableClass => (item.table_class.clone(), Style::default()),
        }
    }
}

/// Show "-" instead of an empty cell (tables without a sort key).
fn dash_if_empty(value: &str) -> String {
    if value.is_empty() {
        "-".to_string()
    } else {
        value.to_string()
    }
}

pub fn format_status(status: &str) -> (&'static str, Color) {
    match status.to_uppercase().as_str() {
        "ACTIVE" => ("✅ Active", Color::Green),
        "CREATING" => ("⏳ Creating", Color::Yellow),
        "UPDATING" => ("🔄 Updating", Color::Yellow),
        "DELETING" => ("🗑 Deleting", Color::Red),
        "ARCHIVING" => ("📦 Archiving", Color::Yellow),
        "ARCHIVED" => ("📦 Archived", Color::DarkGray),
        "INACCESSIBLE_ENCRYPTION_CREDENTIALS" => ("⚠ Inaccessible", Color::Red),
        _ => ("Unknown", Color::White),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_ids_have_prefix() {
        for col in Column::all() {
            assert!(col.id().starts_with("column.dynamodb.table."));
        }
    }

    #[test]
    fn test_from_id_roundtrip() {
        for col in Column::all() {
            assert_eq!(Column::from_id(col.id()), Some(col));
        }
    }

    #[test]
    fn test_all_twelve_columns() {
        assert_eq!(Column::all().len(), 12);
    }

    #[test]
    fn test_column_order_matches_console() {
        assert_eq!(
            Column::all()
                .iter()
                .map(|c| c.default_name())
                .collect::<Vec<_>>(),
            vec![
                "Name",
                "Status",
                "Partition key",
                "Sort key",
                "Indexes",
                "Replication Regions",
                "Deletion protection",
                "Favorite",
                "Read capacity mode",
                "Write capacity mode",
                "Total size",
                "Table class",
            ]
        );
    }

    #[test]
    fn test_every_column_visible_by_default() {
        assert_eq!(Column::default_visible_ids(), Column::ids());
        assert_eq!(Column::default_visible_ids().len(), 12);
    }

    #[test]
    fn test_render_indexes_sort_key_and_replication() {
        use rusticity_core::dynamodb::{DdbIndex, DdbReplica};
        let table = Table {
            name: "orders".to_string(),
            status: "ACTIVE".to_string(),
            partition_key: "pk (S)".to_string(),
            sort_key: String::new(),
            global_secondary_indexes: vec![DdbIndex::default(), DdbIndex::default()],
            total_size_bytes: 2048,
            ..Default::default()
        };

        let (indexes, _) = TableColumn::render(&Column::Indexes, &table);
        assert_eq!(indexes, "2");
        // Missing sort keys and non-replicated tables render as "-".
        let (sort, _) = TableColumn::render(&Column::SortKey, &table);
        assert_eq!(sort, "-");
        let (regions, _) = TableColumn::render(&Column::ReplicationRegions, &table);
        assert_eq!(regions, "-");
        let (status, style) = TableColumn::render(&Column::Status, &table);
        assert_eq!(status, "✅ Active");
        assert_eq!(style.fg, Some(Color::Green));

        let replicated = Table {
            replicas: vec![
                DdbReplica {
                    region: "us-east-1".to_string(),
                    ..Default::default()
                },
                DdbReplica {
                    region: "eu-west-1".to_string(),
                    ..Default::default()
                },
            ],
            ..table
        };
        let (regions, _) = TableColumn::render(&Column::ReplicationRegions, &replicated);
        assert_eq!(regions, "us-east-1, eu-west-1");
    }

    #[test]
    fn test_format_status() {
        assert_eq!(format_status("ACTIVE").1, Color::Green);
        assert_eq!(format_status("CREATING").1, Color::Yellow);
        assert_eq!(format_status("DELETING").1, Color::Red);
        assert_eq!(format_status("whatever").0, "Unknown");
    }
}
