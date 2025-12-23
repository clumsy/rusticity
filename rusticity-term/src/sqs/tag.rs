use crate::common::{translate_column, ColumnId};
use crate::ui::table::Column as TableColumn;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for col in Column::all() {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueTag {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    Key,
    Value,
}

impl Column {
    const ID_KEY: &'static str = "column.sqs.tag.key";
    const ID_VALUE: &'static str = "column.sqs.tag.value";

    pub const fn id(&self) -> ColumnId {
        match self {
            Column::Key => Self::ID_KEY,
            Column::Value => Self::ID_VALUE,
        }
    }

    pub const fn default_name(&self) -> &'static str {
        match self {
            Column::Key => "Key",
            Column::Value => "Value",
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            Self::ID_KEY => Some(Column::Key),
            Self::ID_VALUE => Some(Column::Value),
            _ => None,
        }
    }

    pub const fn all() -> [Column; 2] {
        [Column::Key, Column::Value]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }
}

impl TableColumn<QueueTag> for Column {
    fn name(&self) -> &str {
        Box::leak(translate_column(self.id(), self.default_name()).into_boxed_str())
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
        translated.len().max(20) as u16
    }

    fn render(&self, item: &QueueTag) -> (String, ratatui::style::Style) {
        let text = match self {
            Column::Key => item.key.clone(),
            Column::Value => item.value.clone(),
        };
        (text, ratatui::style::Style::default())
    }
}
