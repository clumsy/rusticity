//! Rendering for the KMS key detail page. Shares the same summary pane, tab
//! ribbon, filter bar, table and JSON renderer as the other detail services.
use super::*;
use crate::{
    common::{render_pagination_text, SortDirection},
    kms::key,
    table::TableWindow,
    ui::{
        self,
        filter::{render_filter_bar, FilterConfig, FilterControl},
        render_json_highlighted,
        table::{render_table, Column, TableConfig},
        Summary,
    },
};
use ratatui::{
    prelude::*,
    widgets::{Clear, Paragraph, Wrap},
};

/// Strip control characters so a malformed AWS string cannot corrupt the TUI.
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

fn key_state_color(state: &str) -> Color {
    match state {
        "Enabled" => Color::Green,
        "Disabled" | "PendingImport" => Color::Yellow,
        "PendingDeletion" | "PendingReplicaDeletion" | "Unavailable" => Color::Red,
        _ => Color::White,
    }
}

/// General configuration fields. Prefer the fresh DescribeKey metadata once
/// loaded; fall back to the cloned list snapshot so the pane is populated on
/// entry before the fetch returns.
fn general_fields(
    source: &key::Key,
    meta: Option<&rusticity_core::kms::KmsKey>,
) -> Vec<(&'static str, String)> {
    macro_rules! field {
        ($f:ident) => {
            meta.map(|m| m.$f.as_str()).unwrap_or(source.$f.as_str())
        };
    }
    let multi_region = meta.map(|m| m.multi_region).unwrap_or(source.multi_region);
    vec![
        ("Key ID", value(field!(key_id))),
        ("ARN", value(field!(key_arn))),
        ("Description", value(field!(description))),
        ("Status", value(field!(key_state))),
        ("Key manager", value(field!(key_manager))),
        ("Key spec", value(field!(key_spec))),
        ("Key usage", value(field!(key_usage))),
        ("Origin", value(field!(origin))),
        (
            "Regionality",
            if multi_region {
                "Multi-Region".into()
            } else {
                "Single-Region".into()
            },
        ),
        ("Custom key store ID", value(field!(custom_key_store_id))),
        (
            "Creation date",
            crate::common::format_iso_timestamp(field!(creation_date)),
        ),
        (
            "Expiration date",
            crate::common::format_iso_timestamp(field!(expiration_date)),
        ),
        (
            "Scheduled deletion",
            crate::common::format_iso_timestamp(field!(deletion_date)),
        ),
    ]
}

fn rotation_fields(rotation: Option<&KmsRotation>) -> Vec<(&'static str, String)> {
    match rotation {
        Some(rotation) => vec![
            (
                "Automatic rotation",
                match rotation.enabled {
                    Some(true) => "Enabled".into(),
                    Some(false) => "Disabled".into(),
                    None => "Unknown".into(),
                },
            ),
            (
                "Rotation period (days)",
                rotation
                    .rotation_period_days
                    .map(|d| d.to_string())
                    .unwrap_or_else(|| "-".into()),
            ),
            (
                "Next rotation date",
                crate::common::format_iso_timestamp(&rotation.next_rotation_date),
            ),
            (
                "On-demand rotation started",
                crate::common::format_iso_timestamp(&rotation.on_demand_rotation_start_date),
            ),
        ],
        None => vec![("Automatic rotation", "-".into())],
    }
}

/// Wrap the tab names across as many rows as the width needs, like ECR.
fn ribbon_rows(width: u16) -> Vec<Vec<(&'static str, Tab)>> {
    let mut rows = vec![];
    let mut row: Vec<(&'static str, Tab)> = vec![];
    let mut used = 0usize;
    for tab in Tab::ALL {
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
    let state = &app.kms_state.detail;
    let (Some(source), Some(view)) = (state.source.as_ref(), state.view()) else {
        return false;
    };
    // Shared content-sized, max-three-column Details summary.
    let live_state = view
        .metadata()
        .map(|meta| meta.key_state.as_str())
        .unwrap_or(&source.key_state);
    let summary = Summary::new(
        "Details",
        vec![
            ui::labeled_field("Alias", value(&source.alias)),
            ui::labeled_field("Key ID", value(&source.key_id)),
            ui::labeled_field_colored("Status", value(live_state), key_state_color(live_state)),
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
    let body = chunks[2];
    let live_state = view
        .metadata()
        .map(|m| m.key_state.as_str())
        .unwrap_or(source.key_state.as_str())
        .to_string();
    match view.tab {
        Tab::General => render_summary_pane(
            frame,
            app,
            body,
            "General configuration",
            general_fields(source, view.metadata()),
            view.data().error.as_deref(),
            Some(("Status", key_state_color(&live_state))),
        ),
        Tab::Rotation => render_summary_pane(
            frame,
            app,
            body,
            "Key rotation",
            rotation_fields(view.rotation()),
            view.data().error.as_deref(),
            None,
        ),
        Tab::Policy => render_policy(frame, app, view, body),
        Tab::Aliases | Tab::Tags => render_results(frame, app, view, body),
    }
    true
}

/// A labeled summary pane (General/Rotation) plus an inline freshness/error
/// notice, mirroring ECR's General tab.
fn render_summary_pane(
    frame: &mut Frame,
    app: &App,
    mut body: Rect,
    title: &str,
    fields: Vec<(&'static str, String)>,
    error: Option<&str>,
    colored: Option<(&str, Color)>,
) {
    let lines = fields
        .iter()
        .map(|(label, text)| match colored {
            Some((name, color)) if *label == name => ui::labeled_field_colored(label, text, color),
            _ => ui::labeled_field(label, text),
        })
        .collect();
    if let Some(notice) = error.map(clean).or_else(|| {
        app.kms_state
            .detail
            .busy()
            .then(|| "Loading… (showing last known values)".into())
    }) {
        let parts = Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).split(body);
        frame.render_widget(
            Paragraph::new(notice)
                .style(Style::default().fg(Color::Yellow))
                .wrap(Wrap { trim: false }),
            parts[0],
        );
        body = parts[1];
    }
    Summary::new(title, lines).render(frame, body);
}

/// Read-only key policy: shared JSON renderer, plain fallback when the document
/// is not JSON-shaped, scrollable, no editor and no writes.
fn render_policy(frame: &mut Frame, app: &App, view: &View, mut area: Rect) {
    let data = view.data();
    if let Some(policy) = view.policy() {
        if let Some(notice) = data.error.as_deref().map(clean).or_else(|| {
            app.kms_state
                .detail
                .busy()
                .then(|| "Refreshing… showing last known policy".into())
        }) {
            let parts = Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).split(area);
            frame.render_widget(
                Paragraph::new(notice)
                    .style(Style::default().fg(Color::Yellow))
                    .wrap(Wrap { trim: false }),
                parts[0],
            );
            area = parts[1];
        }
        render_json_highlighted(frame, area, policy, view.policy_scroll, "Key policy", true);
        return;
    }
    // No policy loaded — show the per-section error, or a loading/empty notice.
    let message = if let Some(error) = &data.error {
        format!("Key policy unavailable: {}", clean(error))
    } else if app.kms_state.detail.busy() || !data.loaded {
        "Loading key policy…".into()
    } else {
        "No key policy returned by AWS.".into()
    };
    frame.render_widget(Paragraph::new(message).wrap(Wrap { trim: false }), area);
}

/// Shared column adapter for a detail-table tab, driven by the Tab schema and
/// the per-tab visible-column preferences.
fn columns(view: &View) -> Vec<Box<dyn Column<Row>>> {
    view.tab
        .columns()
        .iter()
        .filter(|(id, _)| view.data().visible_column_ids.contains(id))
        .map(|(name, width)| {
            let name = *name;
            Box::new(ui::table::OwnedColumn::new(
                name,
                name,
                *width,
                false,
                move |row: &Row| (value(row.field(name)), Style::default()),
            )) as Box<dyn Column<Row>>
        })
        .collect()
}

fn empty_message(view: &View, busy: bool) -> &'static str {
    if busy {
        return "Loading…";
    }
    let data = view.data();
    if data.error.is_some() {
        return "Results unavailable — see warning above; Ctrl+R retries.";
    }
    if !data.loaded {
        return "Not loaded";
    }
    if !data.table.filter.is_empty() && !data.table.items.is_empty() {
        return "No matching results";
    }
    match view.tab {
        Tab::Aliases => "No aliases are associated with this key.",
        Tab::Tags => "No tags are associated with this key.",
        _ => "",
    }
}

fn render_results(frame: &mut Frame, app: &App, view: &View, mut area: Rect) {
    let data = view.data();
    // A per-section error shows as a visible warning above the table; the last
    // successful rows (if any) stay rendered below it, visibly stale.
    if let Some(error) = data.error.as_deref().map(str::to_string).or_else(|| {
        app.kms_state
            .detail
            .busy()
            .then(|| "Refreshing… showing last known rows".into())
    }) {
        let parts = Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).split(area);
        frame.render_widget(
            Paragraph::new(format!("Warning: {}", clean(&error)))
                .style(Style::default().fg(Color::Yellow))
                .wrap(Wrap { trim: false }),
            parts[0],
        );
        area = parts[1];
    }
    let parts = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);
    let prepared = data
        .table
        .prepare_view(data.filtered(), TableWindow::SelectedPage);
    let pagination = render_pagination_text(prepared.current_page, prepared.total_pages);
    render_filter_bar(
        frame,
        FilterConfig {
            filter_text: &data.table.filter,
            placeholder: "Filter results",
            mode: app.mode,
            is_input_focused: data.focus == crate::common::InputFocus::Filter,
            controls: vec![FilterControl {
                text: pagination.clone(),
                is_focused: data.focus == crate::common::InputFocus::Pagination,
            }],
            area: parts[0],
        },
    );
    let empty = prepared.items.is_empty();
    let table_columns = columns(view);
    render_table(
        frame,
        TableConfig {
            items: prepared.items,
            selected_index: prepared.selected_index,
            expanded_index: prepared.expanded_index,
            columns: &table_columns,
            sort_column: "",
            sort_direction: SortDirection::Asc,
            title: ui::format_title(&format!("{} ({})", view.tab.name(), prepared.total_items)),
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
            Paragraph::new(empty_message(view, app.kms_state.detail.busy()))
                .wrap(Wrap { trim: false }),
            inner,
        );
    }
}
