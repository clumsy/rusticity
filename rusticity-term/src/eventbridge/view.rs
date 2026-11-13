//! Rendering for the event bus and rule tables and the rule detail page.
use super::*;
use crate::ui::{self, Summary};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

pub(crate) fn clean(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_control() && c != '\n' { ' ' } else { c })
        .collect()
}

fn value(text: &str) -> String {
    if text.is_empty() {
        "-".into()
    } else {
        clean(text).replace('\n', " ")
    }
}

pub(crate) fn timestamp(text: &str) -> String {
    clean(&crate::common::format_iso_timestamp(text))
}

/// Console status: `ENABLED` -> `Enabled`.
pub(crate) fn state(raw: &str) -> String {
    match raw {
        "ENABLED" => "Enabled".into(),
        "DISABLED" => "Disabled".into(),
        "ENABLED_WITH_ALL_CLOUDTRAIL_MANAGEMENT_EVENTS" => "Enabled (all management events)".into(),
        other => other.into(),
    }
}

/// Discoverer state: `STARTED` -> `Started`; empty when the bus has none.
pub(crate) fn discovery(raw: &str) -> String {
    match raw {
        "STARTED" => "Started".into(),
        "STOPPED" => "Stopped".into(),
        other => other.into(),
    }
}

/// Console target type from the target ARN service.
pub(crate) fn target_type(arn: &str) -> String {
    let parts: Vec<&str> = arn.splitn(6, ':').collect();
    let resource = parts.get(5).copied().unwrap_or_default();
    match parts.get(2).copied().unwrap_or_default() {
        "lambda" => "Lambda function",
        "sqs" => "SQS queue",
        "sns" => "SNS topic",
        "states" => "Step Functions state machine",
        "events" if resource.starts_with("api-destination/") => "API destination",
        "events" => "Event bus",
        "kinesis" => "Kinesis stream",
        "firehose" => "Firehose stream",
        "logs" => "CloudWatch log group",
        "ecs" => "ECS task",
        "codebuild" => "CodeBuild project",
        "codepipeline" => "CodePipeline",
        "batch" => "Batch job queue",
        "ssm" => "Systems Manager",
        "sagemaker" => "SageMaker pipeline",
        "redshift" => "Redshift cluster",
        "execute-api" => "API Gateway",
        "appsync" => "AppSync API",
        "" => "",
        other => return other.to_string(),
    }
    .into()
}

fn state_style(text: &str) -> Style {
    if text.starts_with("Enabled") {
        Style::default().fg(Color::Green)
    } else if text.starts_with("Disabled") {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    }
}

#[allow(clippy::too_many_arguments)]
fn render_list<T: 'static>(
    frame: &mut Frame,
    app: &App,
    table: &Table<T>,
    title: &str,
    placeholder: &str,
    no_items: &str,
    status: ReadStatus,
    error: Option<&str>,
    area: Rect,
) {
    crate::spec_table::render(
        frame,
        app,
        table,
        crate::spec_table::Texts {
            title,
            placeholder,
            no_items,
        },
        status,
        error,
        state_style,
        area,
    );
}

pub(crate) fn summary_fields(
    source: &EventRule,
    detail: Option<&RuleDetail>,
    status: ReadStatus,
) -> Vec<(&'static str, String)> {
    let fresh = |field: fn(&RuleDetail) -> String| match (status, detail) {
        (ReadStatus::Ready, Some(d)) => field(d),
        (ReadStatus::Failed, _) => "unavailable".into(),
        _ => "…".into(),
    };
    vec![
        ("Rule name", value(&source.name)),
        ("Status", fresh(|d| value(&state(&d.rule.state)))),
        ("Type", fresh(|d| d.rule.rule_type().into())),
        ("Event bus name", fresh(|d| value(&d.rule.event_bus_name))),
        ("Description", fresh(|d| value(&d.rule.description))),
        ("Rule ARN", value(&source.arn)),
        ("Execution role", fresh(|d| value(&d.rule.role_arn))),
        ("Managed by", fresh(|d| value(&d.rule.managed_by))),
        ("Created by", fresh(|d| value(&d.created_by))),
    ]
}

fn field(label: &str, text: String) -> Line<'static> {
    let style = state_style(&text);
    match style.fg {
        Some(color) => ui::labeled_field_colored(label, text, color),
        None => ui::labeled_field(label, text),
    }
}

fn summary<'a>(title: &'a str, fields: Vec<(&'static str, String)>) -> Summary<'a> {
    Summary::new(
        title,
        fields
            .into_iter()
            .map(|(label, text)| field(label, text))
            .collect(),
    )
}

/// The pattern as the console shows it: pretty JSON, or the raw text when it
/// does not parse.
pub(crate) fn pattern_lines(pattern: &str) -> Vec<String> {
    let text = serde_json::from_str::<serde_json::Value>(pattern)
        .and_then(|v| serde_json::to_string_pretty(&v))
        .unwrap_or_else(|_| pattern.to_string());
    clean(&text).lines().map(str::to_string).collect()
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let state = &app.eventbridge_state;
    match &state.bus {
        None => render_list(
            frame,
            app,
            &state.buses,
            "Event buses",
            "Filter event buses",
            "No event buses in this Region.",
            state.status(),
            state.error(),
            area,
        ),
        Some(bus) => match &bus.rule {
            None => render_list(
                frame,
                app,
                &bus.rules,
                "Rules",
                "Filter rules",
                "No rules on this event bus.",
                bus.status(),
                bus.error(),
                area,
            ),
            Some(rule) => render_rule(frame, app, rule, area),
        },
    }
}

fn render_rule(frame: &mut Frame, app: &App, rule: &RuleView, area: Rect) {
    let status = rule.status();
    let header = summary(
        "Rule details",
        summary_fields(&rule.source, rule.detail(), status),
    );
    let scheduled = rule.scheduled();
    let names: Vec<&str> = Tab::ALL.iter().map(|t| t.name(scheduled)).collect();
    let tabs: Vec<(&str, Tab)> = names
        .iter()
        .copied()
        .zip(Tab::ALL.iter().copied())
        .collect();
    let chunks = Layout::vertical([
        Constraint::Length(header.height(area.width)),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(area);
    header.render_fill(frame, chunks[0]);
    ui::render_tabs(frame, chunks[1], &tabs, &rule.tab);
    let body = chunks[2];
    match rule.tab {
        Tab::Targets => {
            let (status, error) = section(status, rule.error(), rule.detail().map(|d| &d.targets));
            render_list(
                frame,
                app,
                &rule.targets,
                "Targets",
                "Filter targets",
                "No targets. The rule sends events nowhere.",
                status,
                error,
                body,
            );
        }
        Tab::Tags => {
            let (status, error) = section(status, rule.error(), rule.detail().map(|d| &d.tags));
            render_list(
                frame,
                app,
                &rule.tags,
                "Tags",
                "Filter tags",
                "No tags are associated with this rule.",
                status,
                error,
                body,
            );
        }
        Tab::Pattern => {
            if ui::loading::render(frame, body, status, "rule", rule.error()) {
                return;
            }
            let detail = rule.detail().expect("Ready implies detail");
            let (title, lines) = if scheduled {
                (
                    "Event schedule",
                    vec![format!(
                        "Schedule expression: {}",
                        value(&detail.rule.schedule_expression)
                    )],
                )
            } else {
                ("Event pattern", pattern_lines(&detail.rule.event_pattern))
            };
            let block = Block::default()
                .title(ui::format_title(title))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded);
            let visible: Vec<Line> = lines
                .into_iter()
                .skip(rule.pattern_scroll)
                .map(Line::from)
                .collect();
            frame.render_widget(
                Paragraph::new(visible)
                    .block(block)
                    .wrap(Wrap { trim: false }),
                body,
            );
        }
        Tab::Monitoring => {
            if ui::loading::render(frame, body, status, "rule", rule.error()) {
                return;
            }
            let detail = rule.detail().expect("Ready implies detail");
            let metrics = match &detail.metrics {
                Ok(metrics) => metrics,
                Err(error) => {
                    ui::loading::render(frame, body, ReadStatus::Failed, "Monitoring", Some(error));
                    return;
                }
            };
            let titles: Vec<String> = metrics
                .iter()
                .map(|m| format!("{} (Count)", m.name))
                .collect();
            let charts: Vec<_> = metrics
                .iter()
                .zip(&titles)
                .map(|(m, title)| crate::ui::monitoring::MetricChart {
                    title,
                    data: &m.points,
                    y_axis_label: "Count",
                    x_axis_label: None,
                    ..Default::default()
                })
                .collect();
            crate::ui::monitoring::render_monitoring_tab(
                frame,
                body,
                &charts,
                &[],
                &[],
                &[],
                rule.monitoring_scroll,
            );
        }
    }
}

/// A failed section fails only its own tab, after the rule read succeeded.
fn section<'a, T>(
    status: ReadStatus,
    error: Option<&'a str>,
    section: Option<&'a rusticity_core::eventbridge::Section<T>>,
) -> (ReadStatus, Option<&'a str>) {
    match section {
        Some(Err(message)) if status == ReadStatus::Ready => {
            (ReadStatus::Failed, Some(message.as_str()))
        }
        _ => (status, error),
    }
}
