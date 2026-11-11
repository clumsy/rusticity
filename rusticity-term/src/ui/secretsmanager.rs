use crate::{
    app::App,
    common::{filter_by_fields, render_pagination_text, ColumnId, InputFocus, SortDirection},
    keymap::Mode,
    secretsmanager::columns::Column,
    table::{TableState, TableWindow},
    ui::{
        filter::{render_simple_filter, SimpleFilterConfig},
        format_title,
        loading::render_table,
        table::{expanded_from_columns, TableConfig},
    },
};
use ratatui::prelude::*;
use rusticity_core::SecretMetadata;

pub struct State {
    pub detail: crate::secretsmanager::detail::State,
    pub secrets: TableState<SecretMetadata>,
    pub input_focus: InputFocus,
    pub column_ids: Vec<ColumnId>,
    pub visible_column_ids: Vec<ColumnId>,
    pub(crate) inventory: crate::list_read::ListRead<SecretMetadata>,
    /// Secret ARN to enter once the list read completes.
    pub(crate) pending_focus: Option<String>,
}
impl State {
    /// Tab indicator status of the view on screen: the entered secret, else
    /// the list.
    pub(crate) fn load_status(&self) -> crate::async_read::ReadStatus {
        if self.detail.source.is_some() {
            self.detail.read_status()
        } else {
            self.inventory.status()
        }
    }
}
impl Default for State {
    fn default() -> Self {
        Self {
            detail: crate::secretsmanager::detail::State::default(),
            secrets: TableState::new(),
            input_focus: InputFocus::Filter,
            column_ids: Column::ids(),
            visible_column_ids: Column::default_visible_ids(),
            inventory: Default::default(),
            pending_focus: None,
        }
    }
}
pub fn filtered_secrets(app: &App) -> Vec<&SecretMetadata> {
    let mut rows = filter_by_fields(
        &app.secretsmanager_state.secrets.items,
        &app.secretsmanager_state.secrets.filter,
        |s| {
            [
                s.name.as_str(),
                s.arn.as_str(),
                s.description.as_str(),
                s.created_date.as_str(),
                s.last_changed_date.as_str(),
                s.last_accessed_date.as_str(),
                s.last_rotated_date.as_str(),
                s.deleted_date.as_str(),
                s.next_rotation_date.as_str(),
                s.owning_service.as_str(),
                s.secret_type.as_str(),
                crate::secretsmanager::columns::replication_type(s),
                match s.rotation_enabled {
                    Some(true) => "Enabled",
                    // Absent means never configured; the console shows Disabled.
                    Some(false) | None => "Disabled",
                },
            ]
        },
    );
    rows.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.arn.cmp(&b.arn)));
    rows
}
pub fn render_secrets(frame: &mut Frame, app: &App, area: Rect) {
    if crate::secretsmanager::detail::active(app) {
        crate::secretsmanager::detail::render(frame, app, area);
        return;
    }
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);
    let state = &app.secretsmanager_state;
    let view = state
        .secrets
        .prepare_view(filtered_secrets(app), TableWindow::SelectedPage);
    let pagination = render_pagination_text(view.current_page, view.total_pages);
    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &state.secrets.filter,
            placeholder: "Search metadata by name, description, ARN, rotation, or date",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: state.input_focus == InputFocus::Filter,
            is_pagination_focused: state.input_focus == InputFocus::Pagination,
        },
    );
    let columns: Vec<_> = state
        .visible_column_ids
        .iter()
        .filter_map(|id| Column::from_id(id))
        .map(Column::into_table_column)
        .collect();
    let expanded: Vec<_> = Column::ALL
        .into_iter()
        .map(Column::into_table_column)
        .collect();
    render_table(
        frame,
        TableConfig {
            items: view.items,
            selected_index: view.selected_index,
            expanded_index: view.expanded_index,
            columns: &columns,
            sort_column: &Column::Name.name(),
            sort_direction: SortDirection::Asc,
            title: format_title(&format!("Secrets ({})", view.total_items)),
            area: chunks[1],
            get_expanded_content: Some(Box::new(|s| expanded_from_columns(&expanded, s))),
            is_active: app.mode == Mode::Normal,
        },
        state.inventory.status(),
        "Secrets",
        state.inventory.error(),
        crate::ui::loading::EmptyState {
            no_items: "No secrets in this Region.",
            has_unfiltered_items: !state.secrets.items.is_empty(),
        },
    );
}
