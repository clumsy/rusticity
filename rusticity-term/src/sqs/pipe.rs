use crate::common::{format_unix_timestamp, translate_column, ColumnId, UTC_TIMESTAMP_WIDTH};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for col in Column::all() {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBridgePipe {
    pub name: String,
    pub status: String,
    pub target: String,
    pub last_modified: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    Name,
    Status,
    Target,
    LastModified,
}

impl Column {
    pub fn id(&self) -> ColumnId {
        match self {
            Column::Name => "column.sqs.pipe.name",
            Column::Status => "column.sqs.pipe.status",
            Column::Target => "column.sqs.pipe.target",
            Column::LastModified => "column.sqs.pipe.last_modified",
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            Column::Name => "Pipe name",
            Column::Status => "Status",
            Column::Target => "Target",
            Column::LastModified => "Last modified",
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "column.sqs.pipe.name" => Some(Column::Name),
            "column.sqs.pipe.status" => Some(Column::Status),
            "column.sqs.pipe.target" => Some(Column::Target),
            "column.sqs.pipe.last_modified" => Some(Column::LastModified),
            _ => None,
        }
    }

    pub fn all() -> [Column; 4] {
        [
            Column::Name,
            Column::Status,
            Column::Target,
            Column::LastModified,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }
}

impl TableColumn<EventBridgePipe> for Column {
    fn name(&self) -> &str {
        Box::leak(translate_column(self.id(), self.default_name()).into_boxed_str())
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
        translated.len().max(match self {
            Column::Name => 40,
            Column::Status => 15,
            Column::Target => 60,
            Column::LastModified => UTC_TIMESTAMP_WIDTH as usize,
        }) as u16
    }

    fn render(&self, item: &EventBridgePipe) -> (String, Style) {
        let text = match self {
            Column::Name => item.name.clone(),
            Column::Status => {
                if item.status == "RUNNING" {
                    format!("✅ {}", item.status)
                } else {
                    item.status.clone()
                }
            }
            Column::Target => item.target.clone(),
            Column::LastModified => format_unix_timestamp(&item.last_modified),
        };
        let style = match self {
            Column::Status if item.status == "RUNNING" => Style::default().fg(Color::Green),
            _ => Style::default(),
        };
        (text, style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_ids_have_correct_prefix() {
        for col in Column::all() {
            assert!(
                col.id().starts_with("column.sqs.pipe."),
                "Column ID '{}' should start with 'column.sqs.pipe.'",
                col.id()
            );
        }
    }
}
