use crate::app::App;
use crate::common::{ColumnId, InputFocus};
use crate::cw::{Alarm, AlarmColumn};
use crate::keymap::Mode;
use crate::ui::table::{render_table, Column, TableConfig};
use crate::ui::vertical;
use ratatui::{prelude::*, widgets::*};

pub const FILTER_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

pub struct State {
    pub alarms: Vec<Alarm>,
    pub selected: usize,
    pub loading: bool,
    pub alarm_filter: String,
    pub alarm_tab: AlarmTab,
    pub sort_column: String,
    pub sort_direction: crate::common::SortDirection,
    pub visible_columns: Vec<ColumnId>,
    pub all_columns: Vec<ColumnId>,
    pub expanded_alarm: Option<usize>,
    pub input_focus: InputFocus,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            alarms: Vec::new(),
            selected: 0,
            loading: false,
            alarm_filter: String::new(),
            alarm_tab: AlarmTab::AllAlarms,
            sort_column: "Name".to_string(),
            sort_direction: crate::common::SortDirection::Asc,
            visible_columns: [
                AlarmColumn::Name,
                AlarmColumn::State,
                AlarmColumn::LastStateUpdate,
                AlarmColumn::Conditions,
            ]
            .iter()
            .map(|c| c.id())
            .collect(),
            all_columns: AlarmColumn::ids(),
            expanded_alarm: None,
            input_focus: InputFocus::Filter,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlarmTab {
    AllAlarms,
    InAlarm,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlarmViewMode {
    Table,
    Detail,
    Cards,
}

const COLOR_ORANGE: Color = Color::Rgb(255, 165, 0);

struct AlarmTableColumn {
    column_type: AlarmColumn,
}

impl Column<Alarm> for AlarmTableColumn {
    fn name(&self) -> &str {
        Box::leak(self.column_type.name().into_boxed_str())
    }

    fn width(&self) -> u16 {
        self.column_type.width()
    }

    fn render(&self, alarm: &Alarm) -> (String, Style) {
        match self.column_type {
            AlarmColumn::Name => (alarm.name.clone(), Style::default()),
            AlarmColumn::State => {
                let state_upper = alarm.state.to_uppercase();
                let (icon, color, text) = if state_upper == "OK" {
                    ("✅", Color::Green, "OK")
                } else if state_upper == "ALARM" {
                    ("⚠️", COLOR_ORANGE, "In alarm")
                } else if state_upper == "INSUFFICIENT_DATA" {
                    ("❔", Color::Gray, "Insufficient data")
                } else {
                    ("", Color::White, alarm.state.as_str())
                };
                (format!("{} {}", icon, text), Style::default().fg(color))
            }
            AlarmColumn::LastStateUpdate => (
                format!("{} (UTC)", alarm.state_updated_timestamp),
                Style::default(),
            ),
            AlarmColumn::Description => (alarm.description.clone(), Style::default()),
            AlarmColumn::Conditions => (
                format!(
                    "{} {} {}",
                    alarm.metric_name, alarm.comparison_operator, alarm.threshold
                ),
                Style::default(),
            ),
            AlarmColumn::Actions => {
                if alarm.actions_enabled {
                    (
                        "✅ Actions enabled".to_string(),
                        Style::default().fg(Color::Green),
                    )
                } else {
                    ("No actions".to_string(), Style::default())
                }
            }
            AlarmColumn::StateDetails => (alarm.state_reason.clone(), Style::default()),
            AlarmColumn::MetricName => (alarm.metric_name.clone(), Style::default()),
            AlarmColumn::Namespace => (alarm.namespace.clone(), Style::default()),
            AlarmColumn::Statistic => (alarm.statistic.clone(), Style::default()),
            AlarmColumn::Period => (alarm.period.to_string(), Style::default()),
            AlarmColumn::Resource => (alarm.resource.clone(), Style::default()),
            AlarmColumn::Dimensions => (alarm.dimensions.clone(), Style::default()),
            AlarmColumn::Expression => (alarm.expression.clone(), Style::default()),
            AlarmColumn::Type => (alarm.alarm_type.clone(), Style::default()),
            AlarmColumn::CrossAccount => (alarm.cross_account.clone(), Style::default()),
        }
    }
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    let chunks = vertical(
        [
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(0),
        ],
        area,
    );

    // Filter pane with pagination on right
    let page_size = app.alarms_state.table.page_size.value();
    let filtered_count = match app.alarms_state.alarm_tab {
        crate::app::AlarmTab::AllAlarms => app.alarms_state.table.items.len(),
        crate::app::AlarmTab::InAlarm => app
            .alarms_state
            .table
            .items
            .iter()
            .filter(|a| a.state.to_uppercase() == "ALARM")
            .count(),
    };
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = app.alarms_state.table.selected / page_size;

    let pagination = crate::ui::render_pagination_text(current_page, total_pages);

    crate::ui::filter::render_simple_filter(
        frame,
        chunks[0],
        crate::ui::filter::SimpleFilterConfig {
            filter_text: &app.alarms_state.table.filter,
            placeholder: "Search",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.alarms_state.input_focus == InputFocus::Filter,
            is_pagination_focused: app.alarms_state.input_focus == InputFocus::Pagination,
        },
    );

    // Tabs between panes (left-justified)
    let tabs_data = [
        (
            "All alarms",
            app.alarms_state.alarm_tab == crate::app::AlarmTab::AllAlarms,
        ),
        (
            "In alarm",
            app.alarms_state.alarm_tab == crate::app::AlarmTab::InAlarm,
        ),
    ];
    let tab_spans = crate::ui::render_inner_tab_spans(&tabs_data);
    let tabs = Paragraph::new(Line::from(tab_spans));
    frame.render_widget(tabs, chunks[1]);

    // Filter alarms
    let mut filtered_alarms: Vec<&Alarm> = match app.alarms_state.alarm_tab {
        crate::app::AlarmTab::AllAlarms => app.alarms_state.table.items.iter().collect(),
        crate::app::AlarmTab::InAlarm => app
            .alarms_state
            .table
            .items
            .iter()
            .filter(|a| a.state.to_uppercase() == "ALARM")
            .collect(),
    };

    if !app.alarms_state.table.filter.is_empty() {
        let filter_lower = app.alarms_state.table.filter.to_lowercase();
        filtered_alarms.retain(|a| a.name.to_lowercase().contains(&filter_lower));
    }

    let count = filtered_alarms.len();
    let title = format!(" Alarms ({}) ", count);

    let columns: Vec<Box<dyn Column<Alarm>>> = app
        .cw_alarm_visible_column_ids
        .iter()
        .filter_map(|col_id| {
            AlarmColumn::from_id(col_id).map(|col| {
                Box::new(AlarmTableColumn { column_type: col }) as Box<dyn Column<Alarm>>
            })
        })
        .collect();

    let config = TableConfig {
        items: filtered_alarms,
        selected_index: app.alarms_state.table.selected,
        expanded_index: app.alarms_state.table.expanded_item,
        columns: &columns,
        sort_column: &app.alarms_state.sort_column,
        sort_direction: app.alarms_state.sort_direction,
        title,
        area: chunks[2],
        is_active: !matches!(
            app.mode,
            Mode::SpaceMenu
                | Mode::ServicePicker
                | Mode::ColumnSelector
                | Mode::ErrorModal
                | Mode::HelpModal
                | Mode::RegionPicker
                | Mode::CalendarPicker
                | Mode::TabPicker
                | Mode::FilterInput
        ),
        get_expanded_content: Some(Box::new(|alarm: &Alarm| {
            crate::ui::table::expanded_from_columns(&columns, alarm)
        })),
    };

    render_table(frame, config);
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDTH: usize = 100;
    #[allow(dead_code)]
    const PAGINATION: &str = "[1] 2 3";
    #[allow(dead_code)]
    const PAGINATION_LEN: usize = 7;

    fn test_spacing(content_len: usize) -> usize {
        WIDTH - content_len - PAGINATION_LEN
    }

    #[test]
    fn test_pagination_spacing() {
        assert_eq!(test_spacing(6), 87);
        assert_eq!(test_spacing(1), 92);
        assert_eq!(test_spacing(4), 89);
        assert_eq!(test_spacing(5), 88);
        assert_eq!(6 + 87 + PAGINATION_LEN, WIDTH);
        assert_eq!(1 + 92 + PAGINATION_LEN, WIDTH);
    }

    fn test_alarm() -> Alarm {
        Alarm {
            name: "TestAlarm".to_string(),
            state: "OK".to_string(),
            state_updated_timestamp: "2025-07-22 17:13:07".to_string(),
            description: "Test description".to_string(),
            metric_name: "CPUUtilization".to_string(),
            namespace: "AWS/EC2".to_string(),
            statistic: "Average".to_string(),
            period: 300,
            comparison_operator: "GreaterThanThreshold".to_string(),
            threshold: 80.0,
            actions_enabled: true,
            state_reason: "Threshold Crossed".to_string(),
            resource: "i-1234567890abcdef0".to_string(),
            dimensions: "InstanceId=i-1234567890abcdef0".to_string(),
            expression: String::new(),
            alarm_type: "MetricAlarm".to_string(),
            cross_account: String::new(),
        }
    }

    fn format_alarm_line(col: &AlarmColumn, alarm: &Alarm) -> String {
        let value = match col {
            AlarmColumn::Name => alarm.name.clone(),
            AlarmColumn::State => "✅ OK".to_string(),
            AlarmColumn::LastStateUpdate => format!("{} (UTC)", alarm.state_updated_timestamp),
            AlarmColumn::Conditions => format!(
                "{} {} {}",
                alarm.metric_name, alarm.comparison_operator, alarm.threshold
            ),
            AlarmColumn::StateDetails => alarm.state_reason.clone(),
            _ => String::new(),
        };
        format!("{}: {}", col.name(), value)
    }

    #[test]
    fn test_expanded_content_full_values() {
        let alarm = test_alarm();
        let cols = [
            AlarmColumn::Name,
            AlarmColumn::State,
            AlarmColumn::LastStateUpdate,
            AlarmColumn::Conditions,
        ];
        let lines: Vec<_> = cols.iter().map(|c| format_alarm_line(c, &alarm)).collect();
        let content = lines.join("\n");

        let ts = lines
            .iter()
            .find(|l| l.starts_with("Last state update:"))
            .unwrap();
        assert_eq!(ts, "Last state update: 2025-07-22 17:13:07 (UTC)");
        assert!(ts.contains("(UTC)") && !ts.contains("( UTC"));
        assert!(content.contains("Name: TestAlarm"));
        assert!(content.contains("State: ✅ OK"));
        assert!(content.contains("Conditions: CPUUtilization GreaterThanThreshold 80"));
    }

    #[test]
    fn test_expanded_content_not_affected_by_divisors() {
        let mut alarm = test_alarm();
        alarm.state = "ALARM".to_string();
        alarm.state_updated_timestamp = "2025-10-20 10:59:06".to_string();
        alarm.state_reason = "Threshold Crossed: 1 datapoint [500.0 (20/10/25 10:59:00)] was less than the threshold (1000.0).".to_string();

        let cols = [
            AlarmColumn::Name,
            AlarmColumn::LastStateUpdate,
            AlarmColumn::StateDetails,
        ];
        let lines: Vec<_> = cols.iter().map(|c| format_alarm_line(c, &alarm)).collect();

        let ts = lines
            .iter()
            .find(|l| l.starts_with("Last state update:"))
            .unwrap();
        assert!(ts.ends_with("(UTC)") && !ts.contains("( UTC") && !ts.contains("( TC)"));

        let state = lines
            .iter()
            .find(|l| l.starts_with("State details:"))
            .unwrap();
        assert!(state.contains("Threshold Crossed") && state.contains("datapoint"));
        assert!(state.contains("(20/10/25") && state.contains("(1000.0)"));
    }

    #[test]
    fn test_last_state_update_column_width_fits_timestamp() {
        let ts = "2025-07-22 17:13:07 (UTC)";
        let width = AlarmColumn::LastStateUpdate.width() as usize;
        assert!(width >= ts.len());
        assert!(width >= "Last state update ↑".to_string().len());
    }
}
