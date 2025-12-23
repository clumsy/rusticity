use crate::common::{translate_column, ColumnId};
use crate::ui::table::Column as TableColumn;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TagColumn {
    Key,
    Value,
}

impl TagColumn {
    const ID_KEY: &'static str = "column.cw.tag.key";
    const ID_VALUE: &'static str = "column.cw.tag.value";

    pub const fn id(&self) -> ColumnId {
        match self {
            TagColumn::Key => Self::ID_KEY,
            TagColumn::Value => Self::ID_VALUE,
        }
    }

    pub const fn default_name(&self) -> &'static str {
        match self {
            TagColumn::Key => "Key",
            TagColumn::Value => "Value",
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            Self::ID_KEY => Some(TagColumn::Key),
            Self::ID_VALUE => Some(TagColumn::Value),
            _ => None,
        }
    }

    pub const fn all() -> [TagColumn; 2] {
        [TagColumn::Key, TagColumn::Value]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }
}

impl TableColumn<(String, String)> for TagColumn {
    fn name(&self) -> &str {
        Box::leak(translate_column(self.id(), self.default_name()).into_boxed_str())
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
        translated.len().max(20) as u16
    }

    fn render(&self, item: &(String, String)) -> (String, ratatui::style::Style) {
        let text = match self {
            TagColumn::Key => item.0.clone(),
            TagColumn::Value => item.1.clone(),
        };
        (text, ratatui::style::Style::default())
    }
}
