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
pub struct LambdaTrigger {
    pub uuid: String,
    pub arn: String,
    pub status: String,
    pub last_modified: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    Uuid,
    Arn,
    Status,
    LastModified,
}

impl Column {
    pub fn id(&self) -> ColumnId {
        match self {
            Column::Uuid => "column.sqs.trigger.uuid",
            Column::Arn => "column.sqs.trigger.arn",
            Column::Status => "column.sqs.trigger.status",
            Column::LastModified => "column.sqs.trigger.last_modified",
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            Column::Uuid => "UUID",
            Column::Arn => "ARN",
            Column::Status => "Status",
            Column::LastModified => "Last modified",
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "column.sqs.trigger.uuid" => Some(Column::Uuid),
            "column.sqs.trigger.arn" => Some(Column::Arn),
            "column.sqs.trigger.status" => Some(Column::Status),
            "column.sqs.trigger.last_modified" => Some(Column::LastModified),
            _ => None,
        }
    }

    pub fn all() -> [Column; 4] {
        [
            Column::Uuid,
            Column::Arn,
            Column::Status,
            Column::LastModified,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }
}

impl TableColumn<LambdaTrigger> for Column {
    fn name(&self) -> &str {
        Box::leak(translate_column(self.id(), self.default_name()).into_boxed_str())
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
        translated.len().max(match self {
            Column::Uuid => 40,
            Column::Arn => 60,
            Column::Status => 15,
            Column::LastModified => UTC_TIMESTAMP_WIDTH as usize,
        }) as u16
    }

    fn render(&self, item: &LambdaTrigger) -> (String, Style) {
        let text = match self {
            Column::Uuid => item.uuid.clone(),
            Column::Arn => item.arn.clone(),
            Column::Status => {
                if item.status == "Enabled" {
                    format!("✅ {}", item.status)
                } else {
                    item.status.clone()
                }
            }
            Column::LastModified => format_unix_timestamp(&item.last_modified),
        };
        let style = match self {
            Column::Status if item.status == "Enabled" => Style::default().fg(Color::Green),
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
                col.id().starts_with("column.sqs.trigger."),
                "Column ID '{}' should start with 'column.sqs.trigger.'",
                col.id()
            );
        }
    }
}
