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
    const ID_NAME: &'static str = "column.cw.alarm.name";
    const ID_STATE: &'static str = "column.cw.alarm.state";
    const ID_LAST_STATE_UPDATE: &'static str = "column.cw.alarm.last_state_update";
    const ID_DESCRIPTION: &'static str = "column.cw.alarm.description";
    const ID_CONDITIONS: &'static str = "column.cw.alarm.conditions";
    const ID_ACTIONS: &'static str = "column.cw.alarm.actions";
    const ID_STATE_DETAILS: &'static str = "column.cw.alarm.state_details";
    const ID_METRIC_NAME: &'static str = "column.cw.alarm.metric_name";
    const ID_NAMESPACE: &'static str = "column.cw.alarm.namespace";
    const ID_STATISTIC: &'static str = "column.cw.alarm.statistic";
    const ID_PERIOD: &'static str = "column.cw.alarm.period";
    const ID_RESOURCE: &'static str = "column.cw.alarm.resource";
    const ID_DIMENSIONS: &'static str = "column.cw.alarm.dimensions";
    const ID_EXPRESSION: &'static str = "column.cw.alarm.expression";
    const ID_TYPE: &'static str = "column.cw.alarm.type";
    const ID_CROSS_ACCOUNT: &'static str = "column.cw.alarm.cross_account";

    pub const fn id(&self) -> &'static str {
        match self {
            AlarmColumn::Name => Self::ID_NAME,
            AlarmColumn::State => Self::ID_STATE,
            AlarmColumn::LastStateUpdate => Self::ID_LAST_STATE_UPDATE,
            AlarmColumn::Description => Self::ID_DESCRIPTION,
            AlarmColumn::Conditions => Self::ID_CONDITIONS,
            AlarmColumn::Actions => Self::ID_ACTIONS,
            AlarmColumn::StateDetails => Self::ID_STATE_DETAILS,
            AlarmColumn::MetricName => Self::ID_METRIC_NAME,
            AlarmColumn::Namespace => Self::ID_NAMESPACE,
            AlarmColumn::Statistic => Self::ID_STATISTIC,
            AlarmColumn::Period => Self::ID_PERIOD,
            AlarmColumn::Resource => Self::ID_RESOURCE,
            AlarmColumn::Dimensions => Self::ID_DIMENSIONS,
            AlarmColumn::Expression => Self::ID_EXPRESSION,
            AlarmColumn::Type => Self::ID_TYPE,
            AlarmColumn::CrossAccount => Self::ID_CROSS_ACCOUNT,
        }
    }

    pub const fn default_name(&self) -> &'static str {
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
            Self::ID_NAME => Some(AlarmColumn::Name),
            Self::ID_STATE => Some(AlarmColumn::State),
            Self::ID_LAST_STATE_UPDATE => Some(AlarmColumn::LastStateUpdate),
            Self::ID_DESCRIPTION => Some(AlarmColumn::Description),
            Self::ID_CONDITIONS => Some(AlarmColumn::Conditions),
            Self::ID_ACTIONS => Some(AlarmColumn::Actions),
            Self::ID_STATE_DETAILS => Some(AlarmColumn::StateDetails),
            Self::ID_METRIC_NAME => Some(AlarmColumn::MetricName),
            Self::ID_NAMESPACE => Some(AlarmColumn::Namespace),
            Self::ID_STATISTIC => Some(AlarmColumn::Statistic),
            Self::ID_PERIOD => Some(AlarmColumn::Period),
            Self::ID_RESOURCE => Some(AlarmColumn::Resource),
            Self::ID_DIMENSIONS => Some(AlarmColumn::Dimensions),
            Self::ID_EXPRESSION => Some(AlarmColumn::Expression),
            Self::ID_TYPE => Some(AlarmColumn::Type),
            Self::ID_CROSS_ACCOUNT => Some(AlarmColumn::CrossAccount),
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
