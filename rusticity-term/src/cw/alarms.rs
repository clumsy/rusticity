use crate::common::{ColumnTrait, UTC_TIMESTAMP_WIDTH};

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
    pub fn name(&self) -> &'static str {
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

    pub fn all() -> Vec<AlarmColumn> {
        vec![
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
}

impl ColumnTrait for AlarmColumn {
    fn name(&self) -> &'static str {
        self.name()
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
