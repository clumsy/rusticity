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
pub struct InstanceTag {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    Key,
    Value,
}

impl Column {
    pub fn id(&self) -> ColumnId {
        match self {
            Column::Key => "column.ec2.tag.key",
            Column::Value => "column.ec2.tag.value",
        }
    }

    pub fn default_name(&self) -> &'static str {
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
            "column.ec2.tag.key" => Some(Column::Key),
            "column.ec2.tag.value" => Some(Column::Value),
            _ => None,
        }
    }

    pub fn all() -> [Column; 2] {
        [Column::Key, Column::Value]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }
}

impl TableColumn<InstanceTag> for Column {
    fn name(&self) -> &str {
        Box::leak(translate_column(self.id(), self.default_name()).into_boxed_str())
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
        translated.len().max(20) as u16
    }

    fn render(&self, item: &InstanceTag) -> (String, ratatui::style::Style) {
        let text = match self {
            Column::Key => item.key.clone(),
            Column::Value => item.value.clone(),
        };
        (text, ratatui::style::Style::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_ids() {
        let ids = Column::ids();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], "column.ec2.tag.key");
        assert_eq!(ids[1], "column.ec2.tag.value");
    }

    #[test]
    fn test_column_from_id() {
        assert_eq!(Column::from_id("column.ec2.tag.key"), Some(Column::Key));
        assert_eq!(Column::from_id("column.ec2.tag.value"), Some(Column::Value));
        assert_eq!(Column::from_id("invalid"), None);
    }

    #[test]
    fn test_column_names() {
        assert_eq!(Column::Key.default_name(), "Key");
        assert_eq!(Column::Value.default_name(), "Value");
    }
}
