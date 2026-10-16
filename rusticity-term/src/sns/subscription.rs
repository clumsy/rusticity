use crate::{
    common::{translate_column, ColumnId},
    ui::table::Column as TableColumn,
};
use ratatui::prelude::*;
use rusticity_core::sns::SnsSubscription;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Protocol,
    Endpoint,
    Arn,
    Status,
    Owner,
}
impl Column {
    pub const ALL: [Self; 5] = [
        Self::Protocol,
        Self::Endpoint,
        Self::Arn,
        Self::Status,
        Self::Owner,
    ];
    fn definition(self) -> (ColumnId, &'static str, u16) {
        match self {
            Self::Protocol => ("column.sns.subscription.protocol", "Protocol", 14),
            Self::Endpoint => ("column.sns.subscription.endpoint", "Endpoint", 60),
            Self::Arn => ("column.sns.subscription.arn", "Subscription ARN", 80),
            Self::Status => ("column.sns.subscription.status", "Status", 26),
            Self::Owner => ("column.sns.subscription.owner", "Owner", 16),
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
        [Self::Protocol, Self::Endpoint, Self::Arn, Self::Status]
            .into_iter()
            .map(Self::id)
            .collect()
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.id() == id)
    }
    pub fn render(self, subscription: &SnsSubscription) -> (String, Style) {
        let value = match self {
            Self::Protocol => &subscription.protocol,
            Self::Endpoint => &subscription.endpoint,
            Self::Arn => &subscription.subscription_arn,
            Self::Status => &subscription.status,
            Self::Owner => &subscription.owner,
        };
        let text = if value.is_empty() {
            "-".into()
        } else {
            value
                .chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect()
        };
        let color = if self == Self::Status {
            match subscription.status.as_str() {
                "Confirmed" => Color::Green,
                "Pending confirmation" | "Unknown" => Color::Yellow,
                "Deleted" => Color::Red,
                _ => Color::White,
            }
        } else {
            Color::White
        };
        (text, Style::default().fg(color))
    }
    pub fn into_table_column(self) -> Box<dyn TableColumn<SnsSubscription>> {
        Box::new(crate::ui::table::OwnedColumn::new(
            self.id(),
            self.default_name(),
            self.definition().2,
            !matches!(self, Self::Endpoint | Self::Arn),
            move |item: &SnsSubscription| self.render(item),
        ))
    }
}
