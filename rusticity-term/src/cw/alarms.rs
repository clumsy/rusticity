use crate::common::translate_column;
use crate::common::{ColumnId, UTC_TIMESTAMP_WIDTH};
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for col in [
        AlarmColumn::Name,
        AlarmColumn::State,
        AlarmColumn::LastStateUpdate,
        AlarmColumn::Description,
        AlarmColumn::Conditions,
        AlarmColumn::Actions,
        AlarmColumn::StateDetails,
        AlarmColumn::MetricName,
        AlarmColumn::Namespace,
        AlarmColumn::Statistic,
        AlarmColumn::Period,
        AlarmColumn::Resource,
        AlarmColumn::Dimensions,
        AlarmColumn::Expression,
        AlarmColumn::Type,
        AlarmColumn::CrossAccount,
    ] {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
}

#[derive(Debug, Clone)]
pub struct Alarm {
    pub name: String,
    pub state: String,
    pub state_updated_timestamp: String,
    pub description: String,
    pub metric_name: String,
    pub namespace: String,
    pub statistic: String,
    pub period: u32,
    pub comparison_operator: String,
    pub threshold: f64,
    pub actions_enabled: bool,
    pub state_reason: String,
    pub resource: String,
    pub dimensions: String,
    pub expression: String,
    pub alarm_type: String,
    pub cross_account: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlarmColumn {
    Name,
    State,
    LastStateUpdate,
    Description,
    Conditions,
    Actions,
    StateDetails,
    MetricName,
    Namespace,
    Statistic,
    Period,
    Resource,
    Dimensions,
    Expression,
    Type,
    CrossAccount,
}

impl AlarmColumn {
    pub fn id(&self) -> &'static str {
        match self {
            AlarmColumn::Name => "column.cw.alarm.name",
            AlarmColumn::State => "column.cw.alarm.state",
            AlarmColumn::LastStateUpdate => "column.cw.alarm.last_state_update",
            AlarmColumn::Description => "column.cw.alarm.description",
            AlarmColumn::Conditions => "column.cw.alarm.conditions",
            AlarmColumn::Actions => "column.cw.alarm.actions",
            AlarmColumn::StateDetails => "column.cw.alarm.state_details",
            AlarmColumn::MetricName => "column.cw.alarm.metric_name",
            AlarmColumn::Namespace => "column.cw.alarm.namespace",
            AlarmColumn::Statistic => "column.cw.alarm.statistic",
            AlarmColumn::Period => "column.cw.alarm.period",
            AlarmColumn::Resource => "column.cw.alarm.resource",
            AlarmColumn::Dimensions => "column.cw.alarm.dimensions",
            AlarmColumn::Expression => "column.cw.alarm.expression",
            AlarmColumn::Type => "column.cw.alarm.type",
            AlarmColumn::CrossAccount => "column.cw.alarm.cross_account",
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            AlarmColumn::Name => "Name",
            AlarmColumn::State => "State",
            AlarmColumn::LastStateUpdate => "Last state update",
            AlarmColumn::Description => "Description",
            AlarmColumn::Conditions => "Conditions",
            AlarmColumn::Actions => "Actions",
            AlarmColumn::StateDetails => "State details",
            AlarmColumn::MetricName => "Metric name",
            AlarmColumn::Namespace => "Namespace",
            AlarmColumn::Statistic => "Statistic",
            AlarmColumn::Period => "Period",
            AlarmColumn::Resource => "Resource",
            AlarmColumn::Dimensions => "Dimensions",
            AlarmColumn::Expression => "Expression",
            AlarmColumn::Type => "Type",
            AlarmColumn::CrossAccount => "Cross-account",
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }

    pub fn width(&self) -> u16 {
        match self {
            AlarmColumn::Name => 30,
            AlarmColumn::State => 15,
            AlarmColumn::LastStateUpdate => UTC_TIMESTAMP_WIDTH,
            AlarmColumn::Description => 40,
            AlarmColumn::Conditions => 30,
            AlarmColumn::Actions => 20,
            AlarmColumn::StateDetails => 30,
            AlarmColumn::MetricName => 25,
            AlarmColumn::Namespace => 20,
            AlarmColumn::Statistic => 15,
            AlarmColumn::Period => 10,
            AlarmColumn::Resource => 25,
            AlarmColumn::Dimensions => 25,
            AlarmColumn::Expression => 30,
            AlarmColumn::Type => 15,
            AlarmColumn::CrossAccount => 15,
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "column.cw.alarm.name" => Some(AlarmColumn::Name),
            "column.cw.alarm.state" => Some(AlarmColumn::State),
            "column.cw.alarm.last_state_update" => Some(AlarmColumn::LastStateUpdate),
            "column.cw.alarm.description" => Some(AlarmColumn::Description),
            "column.cw.alarm.conditions" => Some(AlarmColumn::Conditions),
            "column.cw.alarm.actions" => Some(AlarmColumn::Actions),
            "column.cw.alarm.state_details" => Some(AlarmColumn::StateDetails),
            "column.cw.alarm.metric_name" => Some(AlarmColumn::MetricName),
            "column.cw.alarm.namespace" => Some(AlarmColumn::Namespace),
            "column.cw.alarm.statistic" => Some(AlarmColumn::Statistic),
            "column.cw.alarm.period" => Some(AlarmColumn::Period),
            "column.cw.alarm.resource" => Some(AlarmColumn::Resource),
            "column.cw.alarm.dimensions" => Some(AlarmColumn::Dimensions),
            "column.cw.alarm.expression" => Some(AlarmColumn::Expression),
            "column.cw.alarm.type" => Some(AlarmColumn::Type),
            "column.cw.alarm.cross_account" => Some(AlarmColumn::CrossAccount),
            _ => None,
        }
    }

    pub fn all() -> [AlarmColumn; 16] {
        [
            AlarmColumn::Name,
            AlarmColumn::State,
            AlarmColumn::LastStateUpdate,
            AlarmColumn::Description,
            AlarmColumn::Conditions,
            AlarmColumn::Actions,
            AlarmColumn::StateDetails,
            AlarmColumn::MetricName,
            AlarmColumn::Namespace,
            AlarmColumn::Statistic,
            AlarmColumn::Period,
            AlarmColumn::Resource,
            AlarmColumn::Dimensions,
            AlarmColumn::Expression,
            AlarmColumn::Type,
            AlarmColumn::CrossAccount,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }
}

pub fn console_url(
    region: &str,
    view_mode: &str,
    page_size: usize,
    sort_col: &str,
    sort_dir: &str,
) -> String {
    format!(
        "https://{}.console.aws.amazon.com/cloudwatch/home?region={}#alarmsV2:alarms/{}/{}?~(sortingColumn~'{}~sortingDirection~'{})",
        region, region, view_mode, page_size, sort_col, sort_dir
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_ids_have_correct_prefix() {
        for col in AlarmColumn::all() {
            assert!(
                col.id().starts_with("column.cw.alarm."),
                "AlarmColumn ID '{}' should start with 'column.cw.alarm.'",
                col.id()
            );
        }
    }
}
