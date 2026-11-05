//! Rendering for the KMS key detail page. Shares the same summary pane, tab
//! ribbon, filter bar, table and JSON renderer as the other detail services.
use super::*;
use crate::{
    async_read::ReadStatus,
    common::{render_pagination_text, SortDirection},
    table::TableWindow,
    ui::{
        self,
        filter::{render_filter_bar, FilterConfig, FilterControl},
        loading::render_table,
        table::{Column, TableConfig},
        Summary,
    },
};
use ratatui::{
    prelude::*,
    widgets::{Paragraph, Wrap},
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
/// General configuration fields from the fresh DescribeKey metadata ONLY. This
/// is called exclusively when the General section is Ready, so mutable fields
/// never fall back to the stale cloned list snapshot.
fn general_fields(meta: &rusticity_core::kms::KmsKey) -> Vec<(&'static str, String)> {
    macro_rules! field {
        ($f:ident) => {
            meta.$f.as_str()
        };
    }
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
            if meta.multi_region {
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
    let loading = state.loading_status();
    // Header shows the entered key's IDENTITY only (alias + key id), which the
    // initial snapshot owns. Status comes strictly from the fresh DescribeKey:
    // before it arrives (or if General failed) it is shown as unknown, never the
    // stale list-snapshot state.
    let status_field = match view.general_status(loading) {
        ReadStatus::Ready => {
            let s = view
                .metadata()
                .map(|m| m.key_state.as_str())
                .unwrap_or("-")
                .to_string();
            ui::labeled_field_colored("Status", value(&s), key_state_color(&s))
        }
        ReadStatus::Failed => ui::labeled_field("Status", "unavailable".to_string()),
        _ => ui::labeled_field("Status", "…".to_string()),
    };
    let summary = Summary::new(
        "Details",
        vec![
            ui::labeled_field("Alias", value(&source.alias)),
            ui::labeled_field("Key ID", value(&source.key_id)),
            status_field,
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
    match view.tab {
        Tab::General => {
            // Each section gates on its OWN status: a loading/failed/idle General
            // consumes the pane and shows no stale fields; only Ready renders the
            // fresh metadata.
            if ui::loading::render(
                frame,
                body,
                view.general_status(loading),
                "key details",
                view.general_error(),
            ) {
                return true;
            }
            let meta = view.metadata().expect("General Ready implies metadata");
            render_summary_pane(
                frame,
                body,
                "General configuration",
                general_fields(meta),
                Some(("Status", key_state_color(&meta.key_state))),
            );
        }
        Tab::Rotation => {
            if ui::loading::render(
                frame,
                body,
                view.rotation_status(loading),
                "key rotation",
                view.rotation_error(),
            ) {
                return true;
            }
            render_summary_pane(
                frame,
                body,
                "Key rotation",
                rotation_fields(view.rotation()),
                None,
            );
        }
        Tab::Policy => render_policy(frame, app, view, loading, body),
        Tab::Aliases | Tab::Tags => render_results(frame, app, view, loading, body),
    }
    true
}

/// A labeled summary pane (General/Rotation). Rendered only when the section is
/// Ready, so it never carries a stale-value notice.
fn render_summary_pane(
    frame: &mut Frame,
    body: Rect,
    title: &str,
    fields: Vec<(&'static str, String)>,
    colored: Option<(&str, Color)>,
) {
    let lines = fields
        .iter()
        .map(|(label, text)| match colored {
            Some((name, color)) if *label == name => ui::labeled_field_colored(label, text, color),
            _ => ui::labeled_field(label, text),
        })
        .collect();
    Summary::new(title, lines).render(frame, body);
}

/// Read-only key policy: shared JSON renderer, plain fallback when the document
/// is not JSON-shaped, scrollable, no editor and no writes.
fn render_policy(
    frame: &mut Frame,
    app: &App,
    view: &View,
    loading: Option<ReadStatus>,
    area: Rect,
) {
    // Loading/failure/idle consume the pane; the previous policy is never shown
    // during a reload or after a failed one. Only Ready renders the JSON.
    if ui::loading::render(
        frame,
        area,
        view.policy_status(loading),
        "key policy",
        view.policy_error(),
    ) {
        return;
    }
    match view.policy() {
        Some(policy) => crate::text_viewer::render(
            frame,
            app,
            area,
            policy,
            view.policy_scroll,
            "Key policy",
            true,
            ui::Highlight::Json,
        ),
        // Ready with no policy document: AWS returned none for this key.
        None => frame.render_widget(
            Paragraph::new("No key policy returned by AWS.")
                .wrap(Wrap { trim: false })
                .block(ui::titled_block("Key policy")),
            area,
        ),
    }
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

fn render_results(
    frame: &mut Frame,
    app: &App,
    view: &View,
    loading: Option<ReadStatus>,
    area: Rect,
) {
    let data = view.data();
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
        data.section_status(loading),
        view.tab.name(),
        data.error.as_deref(),
        ui::loading::EmptyState {
            no_items: match view.tab {
                Tab::Aliases => "No aliases are associated with this key.",
                Tab::Tags => "No tags are associated with this key.",
                _ => unreachable!("only alias and tag tabs render results"),
            },
            has_unfiltered_items: !data.table.items.is_empty(),
        },
    );
}
