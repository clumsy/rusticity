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
pub struct SnsSubscription {
    pub subscription_arn: String,
    pub topic_arn: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    SubscriptionArn,
    TopicArn,
}

impl Column {
    const ID_SUBSCRIPTION_ARN: &'static str = "column.sqs.subscription.subscription_arn";
    const ID_TOPIC_ARN: &'static str = "column.sqs.subscription.topic_arn";

    pub const fn id(&self) -> ColumnId {
        match self {
            Column::SubscriptionArn => Self::ID_SUBSCRIPTION_ARN,
            Column::TopicArn => Self::ID_TOPIC_ARN,
        }
    }

    pub const fn default_name(&self) -> &'static str {
        match self {
            Column::SubscriptionArn => "Subscription ARN",
            Column::TopicArn => "Topic ARN",
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            Self::ID_SUBSCRIPTION_ARN => Some(Column::SubscriptionArn),
            Self::ID_TOPIC_ARN => Some(Column::TopicArn),
            _ => None,
        }
    }

    pub const fn all() -> [Column; 2] {
        [Column::SubscriptionArn, Column::TopicArn]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }
}

impl TableColumn<SnsSubscription> for Column {
    fn name(&self) -> &str {
        Box::leak(translate_column(self.id(), self.default_name()).into_boxed_str())
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
        translated.len().max(60) as u16
    }

    fn render(&self, item: &SnsSubscription) -> (String, ratatui::style::Style) {
        let text = match self {
            Column::SubscriptionArn => item.subscription_arn.clone(),
            Column::TopicArn => item.topic_arn.clone(),
        };
        (text, ratatui::style::Style::default())
    }
}
