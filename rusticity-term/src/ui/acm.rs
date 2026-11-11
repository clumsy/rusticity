use crate::async_read::{AsyncRead, ReadStatus};
use crate::{
    acm::columns::Column,
    app::App,
    common::{filter_by_fields, render_pagination_text, ColumnId, InputFocus, SortDirection},
    keymap::Mode,
    table::{TableState, TableWindow},
    ui::{
        filter::{render_simple_filter, SimpleFilterConfig},
        format_title,
        loading::{render_table, EmptyState},
        table::{expanded_from_columns, TableConfig},
    },
};
use ratatui::prelude::*;
use rusticity_core::Certificate;

pub struct State {
    pub certificates: TableState<Certificate>,
    pub input_focus: InputFocus,
    pub column_ids: Vec<ColumnId>,
    pub visible_column_ids: Vec<ColumnId>,
    pub(crate) inventory: AsyncRead<Vec<Certificate>>,
    /// Inline error from the latest read; never an ErrorModal focus steal.
    pub(crate) error: Option<String>,
    /// Chosen in Preferences; see `crate::sort`.
    pub sort: crate::sort::SortOrder,
}

/// The table default order.
pub fn default_sort() -> crate::sort::SortOrder {
    crate::sort::SortOrder::new(Column::DomainName.id(), SortDirection::Asc)
}

impl Default for State {
    fn default() -> Self {
        Self {
            certificates: TableState::new(),
            input_focus: InputFocus::Filter,
            column_ids: Column::ids(),
            visible_column_ids: Column::default_visible_ids(),
            inventory: AsyncRead::default(),
            error: None,
            sort: default_sort(),
        }
    }
}

impl State {
    /// Only a fresh successful read is Ready; nothing loaded yet stays Idle.
    pub(crate) fn load_status(&self) -> ReadStatus {
        match self.inventory.status() {
            ReadStatus::Ready if self.error.is_some() => ReadStatus::Failed,
            ReadStatus::Idle if self.error.is_some() => ReadStatus::Failed,
            status => status,
        }
    }
}

/// Filter on exactly the text each column shows, sorted by domain then ARN.
pub fn filtered_certificates(app: &App) -> Vec<&Certificate> {
    let mut rows = filter_by_fields(
        &app.acm_state.certificates.items,
        &app.acm_state.certificates.filter,
        |cert| {
            Column::ALL
                .into_iter()
                .map(|column| column.text(cert))
                .chain(std::iter::once(cert.arn.clone()))
                .collect::<Vec<_>>()
        },
    );
    rows.sort_by(|a, b| {
        a.domain_name
            .cmp(&b.domain_name)
            .then_with(|| a.arn.cmp(&b.arn))
    });
    let state = &app.acm_state;
    let order = state
        .sort
        .effective(default_sort(), &state.visible_column_ids);
    crate::sort::apply(&mut rows, order, |cert, id| {
        Column::from_id(id)
            .map(|c| c.text(cert))
            .unwrap_or_default()
    });
    rows
}

pub fn render_certificates(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);
    let state = &app.acm_state;
    let sort_order = state
        .sort
        .effective(default_sort(), &state.visible_column_ids);
    let sort_name = Column::from_id(sort_order.column)
        .map(|c| c.name())
        .unwrap_or_default();
    let view = state
        .certificates
        .prepare_view(filtered_certificates(app), TableWindow::SelectedPage);
    let pagination = render_pagination_text(view.current_page, view.total_pages);
    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &state.certificates.filter,
            placeholder: "Search by domain, certificate ID, status, type, or date",
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
            sort_column: &sort_name,
            sort_direction: sort_order.direction,
            title: format_title(&format!("Certificates ({})", view.total_items)),
            area: chunks[1],
            get_expanded_content: Some(Box::new(|cert| expanded_from_columns(&expanded, cert))),
            is_active: app.mode == Mode::Normal,
        },
        state.load_status(),
        "Certificates",
        state.error.as_deref(),
        EmptyState {
            no_items: "No certificates in this Region.",
            has_unfiltered_items: !state.certificates.items.is_empty(),
        },
    );
}
