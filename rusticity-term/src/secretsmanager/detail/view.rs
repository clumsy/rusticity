//! Rendering for the secret detail page, using the shared summary pane, tab
//! ribbon, filter bar and table renderer.
use super::*;
use crate::{
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
use ratatui::prelude::*;

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

fn timestamp(text: &str) -> String {
    clean(&crate::common::format_iso_timestamp(text))
}

/// The fixed "Secret details" pane above the tabs. Identity comes from the
/// entered row; description and key come only from the fresh DescribeSecret,
/// so they show a placeholder while loading and never the list snapshot.
pub(crate) fn secret_details_fields(
    source: &SecretMetadata,
    meta: Option<&SecretDescription>,
    status: ReadStatus,
) -> Vec<(&'static str, String)> {
    let fresh = |field: fn(&SecretDescription) -> String| match (status, meta) {
        (ReadStatus::Ready, Some(meta)) => field(meta),
        (ReadStatus::Failed, _) => "unavailable".into(),
        _ => "…".into(),
    };
    vec![
        ("Secret name", value(&source.name)),
        ("Secret ARN", value(&source.arn)),
        ("Description", fresh(|m| value(&m.description))),
        (
            "Encryption key",
            fresh(|m| {
                if m.kms_key_id.is_empty() {
                    "aws/secretsmanager".into()
                } else {
                    clean(&m.kms_key_id)
                }
            }),
        ),
    ]
}

pub(crate) fn overview_fields(meta: &SecretDescription) -> Vec<(&'static str, String)> {
    vec![
        ("Type", value(&meta.secret_type)),
        ("Managed by", value(&meta.owning_service)),
        ("Primary Region", value(&meta.primary_region)),
        ("Created on (UTC)", timestamp(&meta.created_date)),
        ("Last updated on (UTC)", timestamp(&meta.last_changed_date)),
        ("Last retrieved (UTC)", value(&meta.last_accessed_date)),
        ("Deleted on (UTC)", timestamp(&meta.deleted_date)),
    ]
}

pub(crate) fn rotation_fields(meta: &SecretDescription) -> Vec<(&'static str, String)> {
    let rotation = &meta.rotation;
    let schedule = if !rotation.schedule_expression.is_empty() {
        clean(&rotation.schedule_expression)
    } else if let Some(days) = rotation.automatically_after_days {
        format!("Every {days} days")
    } else {
        "-".into()
    };
    vec![
        (
            "Rotation status",
            match rotation.enabled {
                Some(true) => "Enabled".into(),
                // AWS omits the flag when rotation was never configured; the
                // console shows that as Disabled.
                Some(false) | None => "Disabled".into(),
            },
        ),
        ("Rotation schedule", schedule),
        ("Rotation window", value(&rotation.window)),
        ("Rotation function", value(&rotation.lambda_arn)),
        (
            "Last rotated date (UTC)",
            timestamp(&rotation.last_rotated_date),
        ),
        (
            "Next rotation date (UTC)",
            timestamp(&rotation.next_rotation_date),
        ),
    ]
}

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
    let state = &app.secretsmanager_state.detail;
    let (Some(source), Some(view)) = (state.source.as_ref(), state.view()) else {
        return false;
    };
    let loading = state.loading_status();
    // The same pane on every tab: it never changes as tabs are cycled.
    let header = secret_details_fields(source, view.metadata(), view.metadata_status(loading));
    let summary = Summary::new(
        "Secret details",
        header
            .iter()
            .map(|(label, text)| ui::labeled_field(label, text))
            .collect(),
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
        Tab::Overview | Tab::Rotation => {
            if ui::loading::render(
                frame,
                body,
                view.metadata_status(loading),
                "secret details",
                view.error(),
            ) {
                return true;
            }
            let meta = view.metadata().expect("Ready implies metadata");
            let fields = if view.tab == Tab::Overview {
                overview_fields(meta)
            } else {
                rotation_fields(meta)
            };
            let lines = fields
                .iter()
                .map(|(label, text)| ui::labeled_field(label, text))
                .collect();
            Summary::new(view.tab.name(), lines).render(frame, body);
        }
        Tab::Replication | Tab::Versions | Tab::Tags => {
            render_results(frame, app, view, loading, body)
        }
    }
    true
}

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
            is_input_focused: data.focus == InputFocus::Filter,
            controls: vec![FilterControl {
                text: pagination,
                is_focused: data.focus == InputFocus::Pagination,
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
                Tab::Replication => "This secret is not replicated to other Regions.",
                Tab::Versions => "No versions are listed for this secret.",
                Tab::Tags => "No tags are associated with this secret.",
                Tab::Overview | Tab::Rotation => unreachable!("summary panes render no table"),
            },
            has_unfiltered_items: !data.table.items.is_empty(),
        },
    );
}
