use super::*;
use crate::{
    common::{format_iso_timestamp, render_dropdown, render_pagination_text, SortDirection},
    table::TableWindow,
    ui::{
        filter::{render_filter_bar, FilterConfig, FilterControl},
        table::{render_table, Column, TableConfig},
        Summary,
    },
};
use ratatui::{
    prelude::*,
    widgets::{Clear, Paragraph, Wrap},
};

pub(super) fn clean(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}
fn value(text: &str) -> String {
    if text.is_empty() {
        "-".into()
    } else {
        clean(text)
    }
}
fn display_status(status: &str) -> String {
    match status {
        "ACTIVE" => "Active".into(),
        "COMPLETE" => "Complete".into(),
        "IN_PROGRESS" => "In progress".into(),
        "FAILED" => "Failed".into(),
        "ARCHIVED" => "Archived".into(),
        "SIGNED" => "Signed".into(),
        _ => value(status),
    }
}
fn status_color(status: &str) -> Color {
    match status {
        "ACTIVE" | "COMPLETE" | "SIGNED" => Color::Green,
        "FAILED" | "UNSUPPORTED_IMAGE" | "FINDINGS_UNAVAILABLE" => Color::Red,
        _ => Color::Yellow,
    }
}
fn artifact_type(artifact: &str, manifest: &str) -> String {
    match artifact {
        "application/vnd.docker.container.image.v1+json"
        | "application/vnd.oci.image.config.v1+json" => "Image".into(),
        "" | "-" => match manifest {
            "application/vnd.docker.distribution.manifest.v2+json"
            | "application/vnd.oci.image.manifest.v1+json" => "Image".into(),
            "application/vnd.docker.distribution.manifest.list.v2+json"
            | "application/vnd.oci.image.index.v1+json" => "Image index".into(),
            _ => "-".into(),
        },
        _ => clean(artifact),
    }
}
fn tags(view: &View) -> String {
    match &view.metadata {
        Some(details) if details.tags.is_empty() => "Untagged".into(),
        Some(details) => details
            .tags
            .iter()
            .map(|s| clean(s))
            .collect::<Vec<_>>()
            .join(", "),
        None => format!(
            "{} (snapshot; full tag list not loaded)",
            value(&view.image.tag)
        ),
    }
}
fn uri(app: &App, view: &View) -> String {
    let base = app
        .ecr_state
        .current_repository_uri
        .as_deref()
        .unwrap_or_default();
    if !base.is_empty() {
        let tag = match &view.metadata {
            Some(details) => details.tags.first().map(String::as_str),
            None => {
                Some(view.image.tag.as_str()).filter(|tag| !tag.is_empty() && *tag != "<untagged>")
            }
        };
        return clean(&match tag {
            Some(tag) => format!("{base}:{tag}"),
            None => format!("{base}@{}", view.digest),
        });
    }
    value(&view.image.uri)
}
fn general_fields(app: &App, view: &View) -> Vec<(&'static str, String)> {
    let details = view.metadata.as_ref();
    let image = &view.image;
    vec![
        (
            "Artifact type",
            artifact_type(
                details.map_or(image.artifact_type.as_str(), |d| &d.artifact_type),
                details.map_or("", |d| &d.manifest_type),
            ),
        ),
        (
            "Repository",
            value(
                app.ecr_state
                    .current_repository
                    .as_deref()
                    .unwrap_or_default(),
            ),
        ),
        (
            "Pushed at",
            format_iso_timestamp(details.map_or(image.pushed_at.as_str(), |d| &d.pushed_at)),
        ),
        (
            "Last recorded pull time",
            format_iso_timestamp(
                details.map_or(image.last_pull_time.as_str(), |d| &d.last_pull_time),
            ),
        ),
        (
            "Size (MB)",
            details
                .map_or(Some(image.size_bytes), |d| d.size_bytes)
                .map(|size| format!("{:.2}", size as f64 / 1_000_000.0))
                .unwrap_or_else(|| "-".into()),
        ),
        (
            "Image status",
            display_status(details.map_or("-", |d| &d.image_status)),
        ),
    ]
}
fn scan_fields(view: &View) -> Vec<(&'static str, String)> {
    let data = &view.tabs[DetailTab::Scanning as usize];
    let Some(scan) = &view.scan else {
        let status = match data.error.as_deref() {
            Some(error) if error.contains("ScanNotFoundException") => "No scan",
            Some(error) if error.contains("AccessDenied") => {
                "Access denied — scan availability unknown"
            }
            Some(_) => "Unavailable",
            None if data.loaded => "Unavailable",
            None => "Not checked / loading",
        };
        return vec![("Status", status.into())];
    };
    let mut fields = vec![
        ("Status", display_status(&scan.status)),
        ("Status detail", value(&scan.description)),
        (
            "Scan completed at",
            format_iso_timestamp(&scan.completed_at),
        ),
        (
            "Vulnerability source updated at",
            format_iso_timestamp(&scan.source_updated_at),
        ),
    ];
    for (label, severity) in [
        ("Critical", "CRITICAL"),
        ("High", "HIGH"),
        ("Medium", "MEDIUM"),
        ("Low", "LOW"),
        ("Info", "INFORMATIONAL"),
    ] {
        let count = scan
            .severity_counts
            .get(severity)
            .map(|n| n.to_string())
            .unwrap_or_else(|| {
                if scan.complete() && (!scan.severity_counts.is_empty() || scan.findings.is_empty())
                {
                    "0".into()
                } else {
                    "-".into()
                }
            });
        fields.push((label, count));
    }
    fields
}
fn ribbon_rows(width: u16) -> Vec<Vec<(&'static str, DetailTab)>> {
    let mut rows = vec![];
    let mut row = vec![];
    let mut used = 0;
    for tab in DetailTab::ALL {
        let len = tab.name().len() + if row.is_empty() { 0 } else { 3 };
        if !row.is_empty() && used + len > width as usize {
            rows.push(std::mem::take(&mut row));
            used = 0;
        }
        used += tab.name().len() + if row.is_empty() { 0 } else { 3 };
        row.push((tab.name(), *tab));
    }
    rows.push(row);
    rows
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) -> bool {
    let Some(view) = &app.ecr_state.scan.view else {
        return false;
    };
    // Use the same content-sized, max-three-column labeled pane as other services.
    let summary = Summary::new(
        "Details",
        vec![
            ui::labeled_field("Image tags", tags(view)),
            ui::labeled_field("URI", uri(app, view)),
            ui::labeled_field("Digest", value(&view.digest)),
        ],
    );
    let ribbons = ribbon_rows(area.width);
    let chunks = Layout::vertical([
        Constraint::Length(summary.height(area.width)),
        Constraint::Length(ribbons.len() as u16),
        Constraint::Min(0),
    ])
    .split(area);
    summary.render_fill(frame, chunks[0]);
    for (i, row) in ribbons.iter().enumerate() {
        if i < chunks[1].height as usize {
            ui::render_tabs(
                frame,
                Rect {
                    y: chunks[1].y + i as u16,
                    height: 1,
                    ..chunks[1]
                },
                row,
                &view.tab,
            );
        }
    }
    let mut body = chunks[2];
    if view.tab == DetailTab::General {
        let lines = general_fields(app, view)
            .iter()
            .map(|(label, value)| {
                if *label == "Image status" {
                    ui::labeled_field_colored(
                        label,
                        value,
                        status_color(view.metadata.as_ref().map_or("", |d| &d.image_status)),
                    )
                } else {
                    ui::labeled_field(label, value)
                }
            })
            .collect();
        let used = Summary::new("General information", lines).render(frame, body);
        body.y += used.height;
        body.height = body.height.saturating_sub(used.height);
        let notice = if let Some(error) = &view.data().error {
            Some(format!("Metadata unavailable: {}", clean(error)))
        } else if app.ecr_state.scan.busy() {
            Some("Loading image metadata… (showing last known values)".into())
        } else {
            None
        };
        if let Some(notice) = notice {
            frame.render_widget(Paragraph::new(notice).wrap(Wrap { trim: false }), body);
        }
    } else {
        if view.tab == DetailTab::Scanning {
            let lines = scan_fields(view)
                .iter()
                .map(|(label, text)| {
                    if *label == "Status" {
                        ui::labeled_field_colored(
                            label,
                            text,
                            status_color(view.scan.as_ref().map_or("", |s| &s.status)),
                        )
                    } else {
                        ui::labeled_field(label, text)
                    }
                })
                .collect();
            let used = Summary::new("Scanning and vulnerabilities", lines).render(frame, body);
            body.y += used.height;
            body.height = body.height.saturating_sub(used.height);
        }
        render_results(frame, app, view, body);
    }
    true
}
struct FieldColumn(&'static str, u16);
impl Column<Row> for FieldColumn {
    fn name(&self) -> &str {
        self.0
    }
    fn width(&self) -> u16 {
        self.1
    }
    fn render(&self, row: &Row) -> (String, Style) {
        let text = row.field(self.0);
        (
            value(text),
            if self.0 == "Status" {
                Style::default().fg(status_color(text))
            } else {
                Style::default()
            },
        )
    }
}
fn columns(view: &View) -> Vec<Box<dyn Column<Row>>> {
    view.tab
        .columns()
        .iter()
        .filter(|(id, _)| view.data().visible_column_ids.contains(id))
        .map(|(name, width)| Box::new(FieldColumn(name, *width)) as Box<dyn Column<Row>>)
        .collect()
}
fn empty_message(view: &View, busy: bool) -> &'static str {
    if busy {
        return "Loading…";
    }
    let data = view.data();
    if data.error.is_some() {
        return "Results unavailable or incomplete — see warning above; Ctrl+R retries.";
    }
    if !data.loaded {
        return "Not loaded";
    }
    if !data.table.filter.is_empty() && !data.table.items.is_empty() {
        return "No matching results";
    }
    match view.tab {
        DetailTab::Referrers => "No referrers — this artifact does not have any referrers.",
        DetailTab::Replication => "No replication statuses reported by AWS for this source.",
        DetailTab::Signing => "No signing statuses reported by AWS.",
        DetailTab::Scanning => {
            if !view.scan.as_ref().is_some_and(|s| s.complete()) {
                "Scan results unavailable or incomplete; an empty list does not mean zero vulnerabilities."
            } else if view
                .scan
                .as_ref()
                .is_some_and(|s| s.severity_counts.values().any(|n| *n > 0))
            {
                "AWS reports findings but returned no details; Ctrl+R retries."
            } else {
                "No findings reported by this scan. This is not a guarantee the image is vulnerability-free."
            }
        }
        DetailTab::General => "",
    }
}
fn source_label(view: &View) -> String {
    view.source_tag
        .as_ref()
        .map(|tag| format!("tag: {} (mutable)", clean(tag)))
        .unwrap_or_else(|| "digest".into())
}
fn short_label(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        text.into()
    } else {
        format!(
            "{}…",
            text.chars()
                .take(width.saturating_sub(1))
                .collect::<String>()
        )
    }
}
fn render_results(frame: &mut Frame, app: &App, view: &View, mut area: Rect) {
    let data = view.data();
    if let Some(error) = &data.error {
        let parts = Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).split(area);
        frame.render_widget(
            Paragraph::new(format!("Warning: {}", clean(error)))
                .style(Style::default().fg(Color::Yellow))
                .wrap(Wrap { trim: false }),
            parts[0],
        );
        area = parts[1];
    }
    let parts = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);
    let prepared = data
        .table
        .prepare_view(data.filtered(view.tab), TableWindow::SelectedPage);
    let pagination = render_pagination_text(prepared.current_page, prepared.total_pages);
    let mut controls = vec![];
    if view.tab == DetailTab::Replication {
        controls.push(FilterControl {
            text: short_label(
                &format!("Select status source: {} ▾", source_label(view)),
                area.width.saturating_sub(24) as usize,
            ),
            is_focused: view.selecting_source(),
        });
    }
    controls.push(FilterControl {
        text: pagination.clone(),
        is_focused: data.focus == crate::common::InputFocus::Pagination,
    });
    render_filter_bar(
        frame,
        FilterConfig {
            filter_text: &data.table.filter,
            placeholder: if view.tab.status_filter() {
                "Filter by status"
            } else {
                "Filter results"
            },
            mode: app.mode,
            is_input_focused: data.focus == crate::common::InputFocus::Filter,
            controls,
            area: parts[0],
        },
    );
    let empty = prepared.items.is_empty();
    let table_columns = columns(view);
    let name = if view.tab == DetailTab::Scanning {
        "Vulnerabilities"
    } else {
        view.tab.name()
    };
    render_table(
        frame,
        TableConfig {
            items: prepared.items,
            selected_index: prepared.selected_index,
            expanded_index: prepared.expanded_index,
            columns: &table_columns,
            sort_column: "",
            sort_direction: SortDirection::Asc,
            title: ui::format_title(&format!("{name} ({})", prepared.total_items)),
            area: parts[1],
            get_expanded_content: Some(Box::new(|row: &Row| {
                row.0
                    .iter()
                    .map(|(label, text)| {
                        (
                            format!("{}: {}", clean(label), value(text)),
                            Style::default(),
                        )
                    })
                    .collect()
            })),
            is_active: app.mode == Mode::Normal,
        },
    );
    if empty && parts[1].height > 3 && parts[1].width > 2 {
        let inner = Rect {
            x: parts[1].x + 1,
            y: parts[1].y + 2,
            width: parts[1].width - 2,
            height: parts[1].height - 3,
        };
        frame.render_widget(Clear, inner);
        frame.render_widget(
            Paragraph::new(empty_message(view, app.ecr_state.scan.busy()))
                .wrap(Wrap { trim: false }),
            inner,
        );
    }
    // Same inline control/dropdown renderer as DynamoDB, IAM, EC2 and SQS.
    if let Some(selected) = view.source_picker {
        let sources = view.sources();
        let visible = frame
            .area()
            .bottom()
            .saturating_sub(parts[0].bottom())
            .saturating_sub(2)
            .max(1) as usize;
        let start = selected.saturating_sub(visible.saturating_sub(1));
        let names: Vec<String> = sources
            .iter()
            .skip(start)
            .take(visible)
            .map(|source| {
                short_label(
                    &source
                        .as_ref()
                        .map(|tag| format!("tag: {}", clean(tag)))
                        .unwrap_or_else(|| "digest (immutable)".into()),
                    area.width.saturating_sub(pagination.len() as u16 + 7) as usize,
                )
            })
            .collect();
        render_dropdown(
            frame,
            &names,
            selected.saturating_sub(start),
            parts[0],
            pagination.len() as u16 + 3,
        );
    }
}
pub(super) fn copy_uri(app: &App) -> String {
    let view = app.ecr_state.scan.view.as_ref().unwrap();
    uri(app, view)
}
