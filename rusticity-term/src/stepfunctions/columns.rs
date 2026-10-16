//! State machine inventory columns. ListStateMachines supplies every column
//! directly; no per-machine DescribeStateMachine requests are needed.
use crate::common::{translate_column, ColumnId};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
use rusticity_core::StateMachineMetadata;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Name,
    Type,
    Created,
    Arn,
}
impl Column {
    pub const ALL: [Self; 4] = [Self::Name, Self::Type, Self::Created, Self::Arn];
    fn definition(self) -> (ColumnId, &'static str, u16) {
        match self {
            Self::Name => ("column.stepfunctions.statemachine.name", "Name", 40),
            Self::Type => ("column.stepfunctions.statemachine.type", "Type", 12),
            Self::Created => (
                "column.stepfunctions.statemachine.created",
                "Created (UTC)",
                26,
            ),
            Self::Arn => ("column.stepfunctions.statemachine.arn", "ARN", 80),
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
    pub fn render(self, machine: &StateMachineMetadata) -> (String, Style) {
        let value = match self {
            Self::Name => &machine.name,
            Self::Type => &machine.machine_type,
            Self::Created => &machine.creation_date,
            Self::Arn => &machine.arn,
        };
        (
            if value.is_empty() {
                "-".into()
            } else {
                value
                    .chars()
                    .map(|c| if c.is_control() { ' ' } else { c })
                    .collect()
            },
            Style::default(),
        )
    }
    pub fn into_table_column(self) -> Box<dyn TableColumn<StateMachineMetadata>> {
        Box::new(crate::ui::table::OwnedColumn::new(
            self.id(),
            self.default_name(),
            self.definition().2,
            self != Self::Arn,
            move |machine: &StateMachineMetadata| self.render(machine),
        ))
    }
}
