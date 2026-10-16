use crate::common::{translate_column, ColumnId};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
use rusticity_core::SnsTopic;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Name,
    Type,
    Arn,
}
impl Column {
    pub const ALL: [Self; 3] = [Self::Name, Self::Type, Self::Arn];
    fn definition(self) -> (ColumnId, &'static str, u16) {
        match self {
            Self::Name => ("column.sns.topic.name", "Name", 40),
            Self::Type => ("column.sns.topic.type", "Type", 12),
            Self::Arn => ("column.sns.topic.arn", "ARN", 80),
        }
    }
    pub fn id(self) -> ColumnId {
        self.definition().0
    }
    pub fn default_name(self) -> &'static str {
        self.definition().1
    }
    pub fn name(self) -> String {
        translate_column(self.id(), self.default_name())
    }
    pub fn ids() -> Vec<ColumnId> {
        Self::ALL.into_iter().map(Self::id).collect()
    }
    pub fn default_visible_ids() -> Vec<ColumnId> {
        Self::ids()
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|col| col.id() == id)
    }
    pub fn render(self, topic: &SnsTopic) -> (String, Style) {
        let value = match self {
            Self::Name => &topic.name,
            Self::Type => &topic.topic_type,
            Self::Arn => &topic.arn,
        };
        (
            if value.is_empty() {
                "-".into()
            } else {
                value.clone()
            },
            Style::default(),
        )
    }
    pub fn into_table_column(self) -> Box<dyn TableColumn<SnsTopic>> {
        Box::new(crate::ui::table::OwnedColumn::new(
            self.id(),
            self.default_name(),
            self.definition().2,
            self != Self::Arn,
            move |topic: &SnsTopic| self.render(topic),
        ))
    }
}
