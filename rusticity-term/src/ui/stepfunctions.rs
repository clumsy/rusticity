use crate::{
    app::App,
    common::{filter_by_fields, render_pagination_text, ColumnId, InputFocus, SortDirection},
    keymap::Mode,
    stepfunctions::columns::Column,
    table::{TableState, TableWindow},
    ui::{
        filter::{render_simple_filter, SimpleFilterConfig},
        format_title,
        table::{expanded_from_columns, render_table, TableConfig},
        titled_block,
    },
};
use ratatui::{prelude::*, widgets::Paragraph};
use rusticity_core::StateMachineMetadata;

pub struct State {
    pub machines: TableState<StateMachineMetadata>,
    pub executions: crate::stepfunctions::executions::State,
    pub input_focus: InputFocus,
    pub column_ids: Vec<ColumnId>,
    pub visible_column_ids: Vec<ColumnId>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            machines: TableState::new(),
            executions: crate::stepfunctions::executions::State::default(),
            input_focus: InputFocus::Filter,
            column_ids: Column::ids(),
            visible_column_ids: Column::default_visible_ids(),
        }
    }
}

pub fn filtered_machines(app: &App) -> Vec<&StateMachineMetadata> {
    let mut rows = filter_by_fields(
        &app.stepfunctions_state.machines.items,
        &app.stepfunctions_state.machines.filter,
        |machine| {
            [
                &machine.name,
                &machine.arn,
                &machine.machine_type,
                &machine.creation_date,
            ]
        },
    );
    rows.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.arn.cmp(&b.arn)));
    rows
}

pub fn render_state_machines(frame: &mut Frame, app: &App, area: Rect) {
    if app.stepfunctions_state.executions.machine.is_some() {
        crate::stepfunctions::executions::render(frame, app, area);
        return;
    }
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);
    let state = &app.stepfunctions_state;
    let view = state
        .machines
        .prepare_view(filtered_machines(app), TableWindow::SelectedPage);
    let pagination = render_pagination_text(view.current_page, view.total_pages);
    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &state.machines.filter,
            placeholder: "Search by name, ARN, type, or created",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: state.input_focus == InputFocus::Filter,
            is_pagination_focused: state.input_focus == InputFocus::Pagination,
        },
    );
    if state.machines.loading && state.machines.items.is_empty() {
        frame.render_widget(
            Paragraph::new("Loading state machines...")
                .block(titled_block("State machines"))
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
            title: format_title(&format!("State machines ({})", view.total_items)),
            area: chunks[1],
            get_expanded_content: Some(Box::new(|machine| {
                expanded_from_columns(&expanded_columns, machine)
            })),
            is_active: app.mode == Mode::Normal,
        },
    );
}

#[cfg(test)]
mod filter_tests {
    use super::*;
    use crate::app::Service;

    fn machine(name: &str, arn: &str, machine_type: &str, created: &str) -> StateMachineMetadata {
        StateMachineMetadata {
            name: name.to_string(),
            arn: arn.to_string(),
            machine_type: machine_type.to_string(),
            creation_date: created.to_string(),
        }
    }

    #[test]
    fn searches_all_fields_case_insensitively_and_sorts_by_name_then_arn() {
        let mut app = App::new_without_client("default".to_string(), None);
        app.current_service = Service::StepFunctionsStateMachines;
        app.stepfunctions_state.machines.items = vec![
            machine(
                "zeta",
                "arn:aws:states:us-east-1:1:stateMachine:zeta",
                "STANDARD",
                "2024-01-01T00:00:00Z",
            ),
            machine(
                "alpha",
                "arn:aws:states:us-east-1:1:stateMachine:alpha",
                "EXPRESS",
                "2024-02-02T00:00:00Z",
            ),
        ];

        // Type match (case-insensitive).
        app.stepfunctions_state.machines.filter = "express".to_string();
        let f = filtered_machines(&app);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].name, "alpha");

        // Shared ARN substring → both, sorted by name ascending.
        app.stepfunctions_state.machines.filter = "arn:aws:states".to_string();
        let f = filtered_machines(&app);
        assert_eq!(
            f.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(),
            ["alpha", "zeta"]
        );
    }

    #[test]
    fn empty_filter_returns_all_and_whitespace_is_significant() {
        let mut app = App::new_without_client("default".to_string(), None);
        app.stepfunctions_state.machines.items = vec![machine(
            "alpha",
            "arn:1",
            "STANDARD",
            "2024-01-01T00:00:00Z",
        )];

        app.stepfunctions_state.machines.filter = String::new();
        assert_eq!(filtered_machines(&app).len(), 1);

        app.stepfunctions_state.machines.filter = " alpha ".to_string();
        assert!(filtered_machines(&app).is_empty());
    }
}
