use crate::async_read::{AsyncRead, ReadStatus};
use crate::{
    app::App,
    common::{filter_by_fields, render_pagination_text, ColumnId, InputFocus, SortDirection},
    keymap::Mode,
    stepfunctions::columns::Column,
    table::{TableState, TableWindow},
    ui::{
        filter::{render_simple_filter, SimpleFilterConfig},
        format_title,
        loading::render_table,
        table::{expanded_from_columns, TableConfig},
    },
};
use ratatui::prelude::*;
use rusticity_core::StateMachineMetadata;

pub struct State {
    pub machines: TableState<StateMachineMetadata>,
    pub executions: crate::stepfunctions::executions::State,
    pub input_focus: InputFocus,
    pub column_ids: Vec<ColumnId>,
    pub visible_column_ids: Vec<ColumnId>,
    /// Owned background read backing the root State Machines inventory.
    pub(crate) inventory: AsyncRead<Vec<StateMachineMetadata>>,
    /// Inline error from the most recent inventory read; never an ErrorModal.
    pub(crate) inventory_error: Option<String>,
    /// Machine ARN/name a finder navigation queued to select once rows arrive.
    pub pending_focus: Option<String>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            machines: TableState::new(),
            executions: crate::stepfunctions::executions::State::default(),
            input_focus: InputFocus::Filter,
            column_ids: Column::ids(),
            visible_column_ids: Column::default_visible_ids(),
            inventory: AsyncRead::default(),
            inventory_error: None,
            pending_focus: None,
        }
    }
}
impl State {
    /// Root inventory read status, with populated-but-Idle treated as Ready so
    /// legacy loaders and seeded fixtures still render the table (see ui::sns).
    pub(crate) fn load_status(&self) -> ReadStatus {
        if self.executions.machine.is_some() {
            return self.executions.read_status();
        }
        match self.inventory.status() {
            ReadStatus::Idle if !self.machines.items.is_empty() => ReadStatus::Ready,
            status => status,
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
        state.load_status(),
        "State machines",
        state.inventory_error.as_deref(),
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
