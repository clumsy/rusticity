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
    const ID_UUID: &'static str = "column.sqs.trigger.uuid";
    const ID_ARN: &'static str = "column.sqs.trigger.arn";
    const ID_STATUS: &'static str = "column.sqs.trigger.status";
    const ID_LAST_MODIFIED: &'static str = "column.sqs.trigger.last_modified";

    pub const fn id(&self) -> ColumnId {
        match self {
            Column::Uuid => Self::ID_UUID,
            Column::Arn => Self::ID_ARN,
            Column::Status => Self::ID_STATUS,
            Column::LastModified => Self::ID_LAST_MODIFIED,
        }
    }

    pub const fn default_name(&self) -> &'static str {
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
            Self::ID_UUID => Some(Column::Uuid),
            Self::ID_ARN => Some(Column::Arn),
            Self::ID_STATUS => Some(Column::Status),
            Self::ID_LAST_MODIFIED => Some(Column::LastModified),
            _ => None,
        }
    }

    pub const fn all() -> [Column; 4] {
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
