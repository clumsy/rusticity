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
    pub subscriptions: crate::sns::subscriptions::State,
    pub input_focus: InputFocus,
    pub column_ids: Vec<ColumnId>,
    pub visible_column_ids: Vec<ColumnId>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            topics: TableState::new(),
            subscriptions: crate::sns::subscriptions::State::default(),
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
        |topic| [&topic.name, &topic.arn, &topic.topic_type],
    );
    rows.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.arn.cmp(&b.arn)));
    rows
}
pub fn render_topics(frame: &mut Frame, app: &App, area: Rect) {
    if app.sns_state.subscriptions.topic.is_some() {
        crate::sns::subscriptions::render(frame, app, area);
        return;
    }
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

#[cfg(test)]
mod filter_tests {
    use super::*;

    fn topic(name: &str, arn: &str, topic_type: &str) -> SnsTopic {
        SnsTopic {
            name: name.to_string(),
            arn: arn.to_string(),
            topic_type: topic_type.to_string(),
        }
    }

    #[test]
    fn searches_name_arn_type_case_insensitively_and_sorts_by_name_then_arn() {
        let mut app = App::new_without_client("default".to_string(), None);
        app.sns_state.topics.items = vec![
            topic("zeta", "arn:aws:sns:us-east-1:1:zeta", "Standard"),
            topic("alpha", "arn:aws:sns:us-east-1:1:alpha", "FIFO"),
        ];

        // Type match (case-insensitive).
        app.sns_state.topics.filter = "fifo".to_string();
        let f = filtered_topics(&app);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].name, "alpha");

        // Shared ARN substring → both, sorted by name ascending.
        app.sns_state.topics.filter = "arn:aws:sns".to_string();
        let f = filtered_topics(&app);
        assert_eq!(
            f.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
            ["alpha", "zeta"]
        );
    }

    #[test]
    fn empty_filter_returns_all_and_whitespace_is_significant() {
        let mut app = App::new_without_client("default".to_string(), None);
        app.sns_state.topics.items = vec![topic("alpha", "arn:1", "Standard")];

        app.sns_state.topics.filter = String::new();
        assert_eq!(filtered_topics(&app).len(), 1);

        app.sns_state.topics.filter = " alpha ".to_string();
        assert!(filtered_topics(&app).is_empty());
    }
}
