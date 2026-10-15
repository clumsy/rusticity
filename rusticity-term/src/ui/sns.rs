use crate::{
    app::App,
    common::{filter_by_fields, render_pagination_text, ColumnId, InputFocus, SortDirection},
    keymap::Mode,
    sns::columns::Column,
    table::{TableState, TableWindow},
    ui::{
        filter::{render_simple_filter, SimpleFilterConfig},
        format_title,
        table::{expanded_from_columns, render_table, TableConfig},
        titled_block,
    },
};
use ratatui::{prelude::*, widgets::Paragraph};
use rusticity_core::SnsTopic;

pub struct State {
    pub topics: TableState<SnsTopic>,
    pub input_focus: InputFocus,
    pub column_ids: Vec<ColumnId>,
    pub visible_column_ids: Vec<ColumnId>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            topics: TableState::new(),
            input_focus: InputFocus::Filter,
            column_ids: Column::ids(),
            visible_column_ids: Column::default_visible_ids(),
        }
    }
}
pub fn filtered_topics(app: &App) -> Vec<&SnsTopic> {
    let mut rows = filter_by_fields(
        &app.sns_state.topics.items,
        &app.sns_state.topics.filter,
        |topic| vec![&topic.name, &topic.arn, &topic.topic_type],
    );
    rows.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.arn.cmp(&b.arn)));
    rows
}
pub fn render_topics(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);
    let state = &app.sns_state;
    let view = state
        .topics
        .prepare_view(filtered_topics(app), TableWindow::SelectedPage);
    let pagination = render_pagination_text(view.current_page, view.total_pages);
    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &state.topics.filter,
            placeholder: "Search by name, ARN, or type",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: state.input_focus == InputFocus::Filter,
            is_pagination_focused: state.input_focus == InputFocus::Pagination,
        },
    );
    if state.topics.loading && state.topics.items.is_empty() {
        frame.render_widget(
            Paragraph::new("Loading SNS topics...")
                .block(titled_block("Topics"))
                .alignment(Alignment::Center),
            chunks[1],
        );
        return;
    }
    let columns: Vec<_> = state
        .visible_column_ids
        .iter()
        .filter_map(|id| Column::from_id(id))
        .map(Column::into_table_column)
        .collect();
    let expanded_columns: Vec<_> = Column::ALL
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
            title: format_title(&format!("Topics ({})", view.total_items)),
            area: chunks[1],
            get_expanded_content: Some(Box::new(|topic| {
                expanded_from_columns(&expanded_columns, topic)
            })),
            is_active: app.mode == Mode::Normal,
        },
    );
}
