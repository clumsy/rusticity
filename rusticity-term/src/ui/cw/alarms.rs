use crate::app::App;
use crate::common::{ColumnId, CyclicEnum, InputFocus};
use crate::cw::{Alarm, AlarmColumn};
use crate::keymap::Mode;
use crate::ui::table::{render_table, Column, TableConfig};
use crate::ui::{
    calculate_dynamic_height, format_title, labeled_field, render_fields_with_dynamic_columns,
    render_json_highlighted, render_tabs, titled_block, vertical,
};
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

impl CyclicEnum for AlarmTab {
    const ALL: &'static [Self] = &[Self::AllAlarms, Self::InAlarm];
}

impl AlarmTab {
    pub fn name(&self) -> &'static str {
        match self {
            AlarmTab::AllAlarms => "All alarms",
            AlarmTab::InAlarm => "In alarm",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlarmViewMode {
    Table,
    Detail,
    Cards,
}

/// Detail tabs shown in the alarm detail view (ribbon below chart).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AlarmDetailTab {
    #[default]
    Details,
    Tags,
    MuteRules,
    Actions,
    History,
    ParentAlarms,
}

impl CyclicEnum for AlarmDetailTab {
    const ALL: &'static [Self] = &[
        Self::Details,
        Self::Tags,
        Self::MuteRules,
        Self::Actions,
        Self::History,
        Self::ParentAlarms,
    ];
}

impl AlarmDetailTab {
    pub fn name(&self) -> &'static str {
        match self {
            AlarmDetailTab::Details => "Details",
            AlarmDetailTab::Tags => "Tags",
            AlarmDetailTab::MuteRules => "Mute rules",
            AlarmDetailTab::Actions => "Actions",
            AlarmDetailTab::History => "History",
            AlarmDetailTab::ParentAlarms => "Parent alarms",
        }
    }
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

fn format_period(period_secs: u32) -> String {
    match period_secs {
        60 => "1 minute".to_string(),
        300 => "5 minutes".to_string(),
        900 => "15 minutes".to_string(),
        3600 => "1 hour".to_string(),
        86400 => "1 day".to_string(),
        s if s < 60 => format!("{} seconds", s),
        s if s % 3600 == 0 => format!("{} hours", s / 3600),
        s if s % 60 == 0 => format!("{} minutes", s / 60),
        s => format!("{} seconds", s),
    }
}

fn format_treat_missing(raw: &str) -> &str {
    match raw {
        "notBreaching" => "Treat missing data as good (not breaching threshold)",
        "breaching" => "Treat missing data as bad (breaching threshold)",
        "ignore" => "Do not evaluate (maintain the alarm state)",
        "missing" => "Treat missing data as missing",
        other => other,
    }
}

fn format_comparison(
    op: &str,
    threshold: f64,
    statistic: &str,
    eval_periods: u32,
    datapoints: u32,
    period: u32,
) -> String {
    let op_str = match op {
        "GreaterThanOrEqualToThreshold" => ">=",
        "GreaterThanThreshold" => ">",
        "LessThanThreshold" => "<",
        "LessThanOrEqualToThreshold" => "<=",
        "LessThanLowerOrGreaterThanUpperThreshold" => "outside band",
        other => other,
    };
    // window = eval_periods * period (e.g. 3 eval periods × 60s = 3 minutes)
    let window_label = format_period(eval_periods * period);
    format!(
        "{} {} {} for {} datapoints within {}",
        statistic, op_str, threshold, datapoints, window_label
    )
}

fn render_alarm_detail(frame: &mut Frame, app: &App, area: Rect) {
    use crate::app::AlarmDetailTab;

    let alarm_name = app.alarms_state.current_alarm.as_ref().unwrap();
    let Some(alarm) = app
        .alarms_state
        .table
        .items
        .iter()
        .find(|a| &a.name == alarm_name)
    else {
        return;
    };

    // ── Layout ────────────────────────────────────────────────────────────────
    // Row 0: metric name (1 line)
    // Row 1: state tabs + threshold condition (1 line)
    // Row 2: chart
    // Row 3: detail tab ribbon (1 line)
    // Row 4: detail content
    let total_height = area.height as usize;
    // render_monitoring_tab renders each chart at max 20 lines.
    // Cap chart_height to 20 to avoid blank space below the chart.
    let chart_height = total_height.saturating_sub(20).clamp(14, 20) as u16;
    let detail_height = total_height
        .saturating_sub(3 + chart_height as usize)
        .max(8) as u16;

    let chunks = vertical(
        [
            Constraint::Length(1),             // header: metric name
            Constraint::Length(1),             // state tabs + threshold
            Constraint::Length(chart_height),  // chart
            Constraint::Length(1),             // detail tab ribbon
            Constraint::Length(detail_height), // detail content
        ],
        area,
    );

    // ── Header: metric name + threshold condition ─────────────────────────────
    // For metric math alarms use the visible expression's label (e.g. "Unschedulable nodes %")
    // or fall back to the alarm name. For standard alarms use metric_name.
    let header_text = if !alarm.metric_name.is_empty() {
        alarm.metric_name.clone()
    } else {
        // Find the visible expression's label
        let visible_label = alarm
            .sub_metrics
            .iter()
            .find(|sm| sm.return_data && sm.expression.is_some() && !sm.label.is_empty())
            .map(|sm| sm.label.as_str())
            .unwrap_or("");
        if !visible_label.is_empty() {
            visible_label.to_string()
        } else {
            alarm.name.clone()
        }
    };

    // For metric math alarms alarm.period = 0 (period lives on each sub-metric).
    // Use the period from the first raw sub-metric as the effective period.
    let effective_period = if alarm.period > 0 {
        alarm.period
    } else {
        alarm
            .sub_metrics
            .iter()
            .filter(|sm| sm.expression.is_none() && sm.period > 0)
            .map(|sm| sm.period as u32)
            .next()
            .unwrap_or(60)
    };

    // Header: metric name (bold, single line)
    let header = Paragraph::new(Line::from(Span::styled(
        &header_text,
        Style::default().add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(header, chunks[0]);

    // ── State indicator tabs (no threshold — it's shown as red line in chart) ─
    let state_upper = alarm.state.to_uppercase();
    let state_tabs = [
        ("In alarm", "ALARM"),
        ("OK", "OK"),
        ("Insufficient data", "INSUFFICIENT_DATA"),
        ("Disabled/Muted", "DISABLED"),
    ];
    let state_spans: Vec<Span> = state_tabs
        .iter()
        .enumerate()
        .flat_map(|(i, (label, key))| {
            let mut spans = Vec::new();
            if i > 0 {
                spans.push(Span::raw(" ⋮ "));
            }
            let is_active = state_upper == *key;
            let style = if is_active {
                let color = match *key {
                    "ALARM" => Color::Red,
                    "OK" => Color::Green,
                    "INSUFFICIENT_DATA" => COLOR_ORANGE,
                    _ => Color::DarkGray,
                };
                Style::default()
                    .fg(color)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            spans.push(Span::styled(*label, style));
            spans
        })
        .collect();
    frame.render_widget(Paragraph::new(Line::from(state_spans)), chunks[1]);

    // ── Chart ─────────────────────────────────────────────────────────────────
    // For metric math alarms metric_data is empty (we skip GetMetricStatistics);
    // show the chart area regardless — the component handles empty data gracefully.
    let chart_title = if alarm.metric_name.is_empty() {
        alarm.name.as_str()
    } else {
        alarm.metric_name.as_str()
    };
    if app.alarms_state.metrics_loading {
        frame.render_widget(
            Paragraph::new("Loading metric data…")
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center),
            chunks[2],
        );
    } else {
        let chart_threshold = if alarm.comparison_operator.is_empty() {
            None
        } else {
            Some(alarm.threshold)
        };
        let chart = crate::ui::monitoring::MetricChart {
            title: chart_title,
            data: &app.alarms_state.metric_data,
            y_axis_label: "",
            x_axis_label: None,
            threshold: chart_threshold,
        };
        crate::ui::monitoring::render_monitoring_tab(frame, chunks[2], &[chart], &[], &[], &[], 0);
    }

    // ── Detail tab ribbon ─────────────────────────────────────────────────────
    let detail_tabs: Vec<(&str, AlarmDetailTab)> =
        AlarmDetailTab::ALL.iter().map(|t| (t.name(), *t)).collect();
    // Indent by 1 to avoid being clipped by adjacent block borders
    let ribbon_area = Rect {
        x: chunks[3].x + 1,
        width: chunks[3].width.saturating_sub(1),
        ..chunks[3]
    };
    render_tabs(
        frame,
        ribbon_area,
        &detail_tabs,
        &app.alarms_state.detail_tab,
    );

    // ── Detail content ────────────────────────────────────────────────────────
    match app.alarms_state.detail_tab {
        AlarmDetailTab::Details => {
            render_alarm_details_tab(frame, alarm, effective_period, chunks[4])
        }
        _ => {
            let paragraph = Paragraph::new(format!(
                "{} — coming soon",
                app.alarms_state.detail_tab.name()
            ))
            .block(titled_block(app.alarms_state.detail_tab.name()))
            .alignment(Alignment::Center);
            frame.render_widget(paragraph, chunks[4]);
        }
    }
}

fn render_alarm_details_tab(frame: &mut Frame, alarm: &Alarm, effective_period: u32, area: Rect) {
    use ratatui::layout::{Constraint, Direction, Layout};

    // ── Details fields ────────────────────────────────────────────────────────
    let detail_fields = vec![
        labeled_field("Name", &alarm.name),
        labeled_field("Type", format!("Metric alarm ({})", alarm.alarm_type)),
        labeled_field(
            "Description",
            if alarm.description.is_empty() {
                "-"
            } else {
                &alarm.description
            },
        ),
        labeled_field("State", &alarm.state),
        labeled_field(
            "Threshold",
            if alarm.statistic.is_empty() {
                // Metric math alarm — use expression label as "statistic"
                let label = alarm
                    .sub_metrics
                    .iter()
                    .find(|sm| sm.return_data && sm.expression.is_some() && !sm.label.is_empty())
                    .map(|sm| sm.label.as_str())
                    .unwrap_or(&alarm.name);
                let window_label = format_period(alarm.evaluation_periods * effective_period);
                let op_str = match alarm.comparison_operator.as_str() {
                    "GreaterThanOrEqualToThreshold" => ">=",
                    "GreaterThanThreshold" => ">",
                    "LessThanThreshold" => "<",
                    "LessThanOrEqualToThreshold" => "<=",
                    other => other,
                };
                format!(
                    "{} {} {} for {} datapoints within {}",
                    label, op_str, alarm.threshold, alarm.datapoints_to_alarm, window_label
                )
            } else {
                format_comparison(
                    &alarm.comparison_operator,
                    alarm.threshold,
                    &alarm.statistic,
                    alarm.evaluation_periods,
                    alarm.datapoints_to_alarm,
                    effective_period,
                )
            },
        ),
        labeled_field(
            "Actions",
            if alarm.actions_enabled {
                "Actions enabled"
            } else {
                "Actions disabled"
            },
        ),
        labeled_field("Last state update", &alarm.state_updated_timestamp),
        labeled_field(
            "Namespace",
            if alarm.namespace.is_empty() {
                "-"
            } else {
                &alarm.namespace
            },
        ),
        labeled_field(
            "Metric name",
            if alarm.metric_name.is_empty() {
                "-"
            } else {
                &alarm.metric_name
            },
        ),
        labeled_field(
            "Dimensions",
            if alarm.dimensions.is_empty() {
                "-"
            } else {
                &alarm.dimensions
            },
        ),
        labeled_field(
            "Statistic",
            if alarm.statistic.is_empty() {
                "-"
            } else {
                &alarm.statistic
            },
        ),
        labeled_field("Period", format_period(effective_period)),
        labeled_field(
            "Datapoints to alarm",
            format!(
                "{} out of {}",
                alarm.datapoints_to_alarm, alarm.evaluation_periods
            ),
        ),
        labeled_field(
            "Missing data treatment",
            format_treat_missing(&alarm.treat_missing_data),
        ),
        labeled_field(
            "Percentiles with low samples",
            if alarm.evaluate_low_sample_percentile.is_empty() {
                "-"
            } else {
                &alarm.evaluate_low_sample_percentile
            },
        ),
        labeled_field(
            "ARN",
            if alarm.alarm_arn.is_empty() {
                "-"
            } else {
                &alarm.alarm_arn
            },
        ),
    ];

    let detail_height = calculate_dynamic_height(&detail_fields, area.width.saturating_sub(4)) + 2;

    // ── EventBridge section ───────────────────────────────────────────────────
    let eventbridge_json = if alarm.alarm_arn.is_empty() {
        "{}".to_string()
    } else {
        format!(
            "{{\n  \"source\": [\"aws.cloudwatch\"],\n  \"detail-type\": [\"CloudWatch Alarm State Change\"],\n  \"resources\": [\"{}\"]  \n}}",
            alarm.alarm_arn
        )
    };
    let eventbridge_lines = eventbridge_json.lines().count() as u16 + 4; // +2 header +2 borders

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(detail_height),
            Constraint::Length(eventbridge_lines),
            Constraint::Min(0),
        ])
        .split(area);

    // Render details pane
    let details_block = titled_block("Details");
    let details_inner = details_block.inner(sections[0]);
    frame.render_widget(details_block, sections[0]);
    render_fields_with_dynamic_columns(frame, details_inner, detail_fields);

    // Render EventBridge rule pane
    render_json_highlighted(
        frame,
        sections[1],
        &eventbridge_json,
        0,
        " View EventBridge rule ",
        false,
    );
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    // If an alarm is selected, render detail view
    if app.alarms_state.current_alarm.is_some() {
        render_alarm_detail(frame, app, area);
        return;
    }

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
    let tabs: Vec<(&str, AlarmTab)> = AlarmTab::ALL.iter().map(|tab| (tab.name(), *tab)).collect();
    crate::ui::render_tabs(frame, chunks[1], &tabs, &app.alarms_state.alarm_tab);

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
    let title = format_title(&format!("Alarms ({})", count));

    // Slice to current page
    let start_idx = current_page * page_size;
    let end_idx = (start_idx + page_size).min(filtered_alarms.len());
    let page_alarms: Vec<&Alarm> = if start_idx < filtered_alarms.len() {
        filtered_alarms[start_idx..end_idx].to_vec()
    } else {
        Vec::new()
    };

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
        items: page_alarms,
        selected_index: app.alarms_state.table.selected % page_size,
        expanded_index: app.alarms_state.table.expanded_item.and_then(|idx| {
            if idx >= start_idx && idx < end_idx {
                Some(idx - start_idx)
            } else {
                None
            }
        }),
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
    const PAGINATION: &str = "[1] 2 3";
    const PAGINATION_LEN: usize = PAGINATION.len();

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
            ..Default::default()
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

    #[test]
    fn test_alarm_detail_tab_cycles_with_tab_key() {
        use crate::common::CyclicEnum;
        let mut tab = AlarmDetailTab::Details;
        tab = tab.next();
        assert_eq!(tab, AlarmDetailTab::Tags);
        tab = tab.next();
        assert_eq!(tab, AlarmDetailTab::MuteRules);
        tab = tab.prev();
        assert_eq!(tab, AlarmDetailTab::Tags);
        tab = tab.prev();
        assert_eq!(tab, AlarmDetailTab::Details);
    }

    #[test]
    fn test_alarm_detail_tab_wraps_around() {
        use crate::common::CyclicEnum;
        let last = AlarmDetailTab::ParentAlarms;
        assert_eq!(
            last.next(),
            AlarmDetailTab::Details,
            "Last tab wraps to first"
        );
        let first = AlarmDetailTab::Details;
        assert_eq!(
            first.prev(),
            AlarmDetailTab::ParentAlarms,
            "First tab wraps to last"
        );
    }

    #[test]
    fn test_format_period_produces_human_readable() {
        assert_eq!(format_period(60), "1 minute");
        assert_eq!(format_period(300), "5 minutes");
        assert_eq!(format_period(3600), "1 hour");
        assert_eq!(format_period(86400), "1 day");
        assert_eq!(format_period(120), "2 minutes");
    }

    #[test]
    fn test_format_treat_missing_known_values() {
        assert!(format_treat_missing("notBreaching").contains("good"));
        assert!(format_treat_missing("breaching").contains("bad"));
        assert!(format_treat_missing("ignore").contains("maintain"));
        assert!(format_treat_missing("missing").contains("missing"));
    }

    #[test]
    fn test_format_comparison_builds_readable_threshold() {
        let result = format_comparison("GreaterThanOrEqualToThreshold", 90.0, "Average", 3, 3, 60);
        assert!(result.contains(">="), "must contain >= operator");
        assert!(result.contains("90"), "must contain threshold");
        assert!(result.contains("Average"), "must contain statistic");
        assert!(
            result.contains("3 datapoints"),
            "must contain datapoints count"
        );
    }

    #[test]
    fn test_alarms_next_detail_tab_cycles_detail_tabs_in_detail_view() {
        use crate::app::{App, Service};
        let mut app = App::new_without_client("default".to_string(), None);
        app.current_service = Service::CloudWatchAlarms;
        app.service_selected = true;
        app.alarms_state.current_alarm = Some("my-alarm".to_string());
        app.alarms_state.detail_tab = AlarmDetailTab::Details;

        crate::cw::actions::alarms_next_detail_tab(&mut app);
        assert_eq!(
            app.alarms_state.detail_tab,
            AlarmDetailTab::Tags,
            "Tab in detail view must cycle detail tabs, not list tabs"
        );
    }

    #[test]
    fn test_alarms_next_detail_tab_cycles_list_tabs_in_list_view() {
        use crate::app::AlarmTab;
        use crate::app::{App, Service};
        let mut app = App::new_without_client("default".to_string(), None);
        app.current_service = Service::CloudWatchAlarms;
        app.service_selected = true;
        // No alarm selected — list view
        app.alarms_state.alarm_tab = AlarmTab::AllAlarms;

        crate::cw::actions::alarms_next_detail_tab(&mut app);
        assert_eq!(
            app.alarms_state.alarm_tab,
            AlarmTab::InAlarm,
            "Tab in list view must cycle list tabs"
        );
        assert_eq!(
            app.alarms_state.detail_tab,
            AlarmDetailTab::Details,
            "detail_tab must be unaffected when in list view"
        );
    }
}
