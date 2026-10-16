//! State-machine-scoped executions. Keys never await AWS input and the parent
//! State Machines table keeps its selection/filter when this view closes.
//!
//! EXPRESS state machines do not support the ListExecutions metadata API. The
//! summary view still opens for an Express machine, but it shows an explicit
//! "unsupported" message (never an empty success) and `poll` performs NO
//! request for it. STANDARD machines poll asynchronously, scoped to the active
//! profile/region/account/role: the task is aborted on context change or when
//! the service is left (except CLI output), and a cancelled or late-completing
//! read keeps previously loaded rows (stale, never silently emptied).
use crate::{
    app::{App, Service},
    common::{
        filter_by_fields, render_pagination_text, translate_column, ColumnId, InputFocus,
        SortDirection,
    },
    inventory::{self, InventoryInput},
    keymap::{Action, Mode},
    table::{TableState, TableWindow},
    ui::{
        self,
        filter::{render_simple_filter, SimpleFilterConfig},
        table::{expanded_from_columns, render_table, Column as TableColumn, TableConfig},
    },
};
use ratatui::{
    prelude::*,
    widgets::{Paragraph, Wrap},
};
use rusticity_core::{ExecutionMetadata, StateMachineMetadata, StepFunctionsClient};
use tokio::task::JoinHandle;

/// Machine `machine_type` literal whose executions are not listable via the
/// metadata ListExecutions API. Matched case-insensitively.
const EXPRESS_TYPE: &str = "EXPRESS";

fn is_express(machine: &StateMachineMetadata) -> bool {
    machine.machine_type.eq_ignore_ascii_case(EXPRESS_TYPE)
}

/// Human-readable execution duration. `None` (no stop date yet, or the service
/// omitted it) renders as "-" — never a fabricated "elapsed since now" value.
fn format_duration(duration_millis: Option<u64>) -> String {
    let Some(ms) = duration_millis else {
        return "-".into();
    };
    if ms < 1000 {
        return format!("{ms}ms");
    }
    let secs = ms / 1000;
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 {
        parts.push(format!("{minutes}m"));
    }
    if seconds > 0 {
        parts.push(format!("{seconds}s"));
    }
    if parts.is_empty() {
        // A whole number of seconds with no higher unit (e.g. exactly 1000ms).
        parts.push(format!("{seconds}s"));
    }
    parts.join(" ")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Name,
    Status,
    Started,
    Ended,
    Duration,
    Version,
    Alias,
    RedriveCount,
    LastRedriveTime,
}
impl Column {
    pub const ALL: [Self; 9] = [
        Self::Name,
        Self::Status,
        Self::Started,
        Self::Ended,
        Self::Duration,
        Self::Version,
        Self::Alias,
        Self::RedriveCount,
        Self::LastRedriveTime,
    ];
    fn definition(self) -> (ColumnId, &'static str, u16) {
        match self {
            Self::Name => ("column.stepfunctions.execution.name", "Name", 40),
            Self::Status => ("column.stepfunctions.execution.status", "Status", 14),
            Self::Started => ("column.stepfunctions.execution.started", "Start Time", 26),
            Self::Ended => ("column.stepfunctions.execution.ended", "End Time", 26),
            Self::Duration => ("column.stepfunctions.execution.duration", "Duration", 14),
            Self::Version => ("column.stepfunctions.execution.version", "Version", 10),
            Self::Alias => ("column.stepfunctions.execution.alias", "Alias", 24),
            Self::RedriveCount => (
                "column.stepfunctions.execution.redrive_count",
                "Redrive count",
                15,
            ),
            Self::LastRedriveTime => (
                "column.stepfunctions.execution.last_redrive_time",
                "Last redrive time",
                26,
            ),
        }
    }
    pub fn id(self) -> ColumnId {
        self.definition().0
    }
    pub fn default_name(self) -> &'static str {
        self.definition().1
    }
    pub fn name(self) -> String {
        translate_column(self.id(), self.default_name())
    }
    pub fn ids() -> Vec<ColumnId> {
        Self::ALL.into_iter().map(Self::id).collect()
    }
    pub fn default_visible_ids() -> Vec<ColumnId> {
        [
            Self::Name,
            Self::Status,
            Self::Started,
            Self::Ended,
            Self::Duration,
        ]
        .into_iter()
        .map(Self::id)
        .collect()
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.id() == id)
    }
    pub fn render(self, execution: &ExecutionMetadata) -> (String, Style) {
        let value = match self {
            Self::Name => execution.name.clone(),
            Self::Status => execution.status.clone(),
            Self::Started => execution.start_date.clone(),
            Self::Ended => execution.stop_date.clone(),
            Self::Duration => {
                return (
                    format_duration(execution.duration_millis),
                    status_style(execution),
                )
            }
            Self::Version => qualifier(
                &execution.state_machine_version_arn,
                &execution.state_machine_arn,
            )
            .into(),
            Self::Alias => qualifier(
                &execution.state_machine_alias_arn,
                &execution.state_machine_arn,
            )
            .into(),
            Self::RedriveCount => execution
                .redrive_count
                .map(|count| count.to_string())
                .unwrap_or_default(),
            Self::LastRedriveTime => execution.redrive_date.clone(),
        };
        let text = if value.is_empty() {
            "-".into()
        } else {
            value
                .chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect()
        };
        let style = if self == Self::Status {
            status_style(execution)
        } else {
            Style::default()
        };
        (text, style)
    }
    pub fn into_table_column(self) -> Box<dyn TableColumn<ExecutionMetadata>> {
        Box::new(crate::ui::table::OwnedColumn::new(
            self.id(),
            self.default_name(),
            self.definition().2,
            true,
            move |item: &ExecutionMetadata| self.render(item),
        ))
    }
}

/// Show the version number or alias name, not the whole qualified ARN.
/// Only strip a qualifier from this execution's exact parent identity.
fn qualifier<'a>(arn: &'a str, parent: &str) -> &'a str {
    if parent.is_empty() {
        return "";
    }
    arn.strip_prefix(parent)
        .and_then(|rest| rest.strip_prefix(':'))
        .filter(|value| !value.is_empty() && !value.contains(':'))
        .unwrap_or("")
}

fn status_style(execution: &ExecutionMetadata) -> Style {
    let color = match execution.status.as_str() {
        "SUCCEEDED" => Color::Green,
        "RUNNING" | "PENDING_REDRIVE" => Color::Cyan,
        "FAILED" | "TIMED_OUT" | "ABORTED" => Color::Red,
        _ => Color::White,
    };
    Style::default().fg(color)
}

struct Running(JoinHandle<Result<Vec<ExecutionMetadata>, String>>);
impl Drop for Running {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub struct State {
    pub machine: Option<StateMachineMetadata>,
    pub table: TableState<ExecutionMetadata>,
    pub input_focus: InputFocus,
    pub column_ids: Vec<ColumnId>,
    pub visible_column_ids: Vec<ColumnId>,
    pub error: Option<String>,
    pub(super) loaded: bool,
    scope: String,
    running: Option<Running>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            machine: None,
            table: TableState::new(),
            input_focus: InputFocus::Filter,
            column_ids: Column::ids(),
            visible_column_ids: Column::default_visible_ids(),
            error: None,
            loaded: false,
            scope: String::new(),
            running: None,
        }
    }
}
impl State {
    pub fn busy(&self) -> bool {
        self.table.loading || self.running.is_some()
    }
    /// True when the entered machine is Express, whose executions the metadata
    /// API cannot list. Rendering shows an explicit message and `poll` never
    /// issues a request in this case.
    pub fn unsupported(&self) -> bool {
        self.machine.as_ref().is_some_and(is_express)
    }
    pub fn close(&mut self) {
        self.running = None;
        self.machine = None;
        self.table.items.clear();
        self.table.filter.clear();
        self.table.reset();
        self.table.loading = false;
        self.error = None;
        self.loaded = false;
        self.scope.clear();
        self.input_focus = InputFocus::Filter;
    }
}

fn scope(app: &App) -> String {
    format!(
        "{:?}",
        (
            &app.profile,
            &app.config.region,
            &app.config.account_id,
            &app.config.role_arn
        )
    )
}

pub fn active(app: &App) -> bool {
    app.service_selected
        && app.current_service == Service::StepFunctionsStateMachines
        && app.stepfunctions_state.executions.machine.is_some()
}

pub fn filtered(app: &App) -> Vec<&ExecutionMetadata> {
    let state = &app.stepfunctions_state.executions;
    let mut rows = filter_by_fields(&state.table.items, &state.table.filter, |e| {
        let count = e
            .redrive_count
            .map(|count| count.to_string())
            .unwrap_or_default();
        [
            e.name.as_str(),
            e.status.as_str(),
            e.arn.as_str(),
            e.start_date.as_str(),
            e.stop_date.as_str(),
            e.state_machine_version_arn.as_str(),
            e.state_machine_alias_arn.as_str(),
            e.redrive_date.as_str(),
        ]
        .into_iter()
        .map(std::borrow::Cow::Borrowed)
        .chain(std::iter::once(std::borrow::Cow::Owned(count)))
    });
    rows.sort_by(|a, b| {
        // Most recent starts first; ties broken by name then ARN for stability.
        chrono::DateTime::parse_from_rfc3339(&b.start_date)
            .ok()
            .cmp(&chrono::DateTime::parse_from_rfc3339(&a.start_date).ok())
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.arn.cmp(&b.arn))
    });
    rows
}

pub fn selected(app: &App) -> Option<&ExecutionMetadata> {
    let rows = filtered(app);
    rows.get(app.stepfunctions_state.executions.table.selected)
        .copied()
}

pub fn go_to_page(app: &mut App, page: usize) {
    let count = filtered(app).len();
    inventory::go_to_page(&mut app.stepfunctions_state.executions.table, page, count);
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !app.service_selected || app.current_service != Service::StepFunctionsStateMachines {
        return false;
    }
    if !active(app) {
        if app.mode == Mode::Normal && *action == Action::Select {
            let rows = ui::stepfunctions::filtered_machines(app);
            let Some(machine) = app
                .stepfunctions_state
                .machines
                .get_selected(&rows)
                .cloned()
            else {
                return true;
            };
            let scope = scope(app);
            let express = is_express(&machine);
            let state = &mut app.stepfunctions_state.executions;
            state.close();
            state.machine = Some(machine);
            state.scope = scope;
            // Express machines are not listable: open the summary but do not
            // enter a loading state, so poll never issues a request for them.
            state.table.loading = !express;
            app.page_input.clear();
            app.update_current_tab_breadcrumb();
            return true;
        }
        return false;
    }
    if app.mode == Mode::Normal {
        match action {
            Action::GoBack => {
                app.stepfunctions_state.executions.close();
                app.page_input.clear();
                app.update_current_tab_breadcrumb();
                return true;
            }
            Action::Refresh => {
                // A refresh on an Express machine is a no-op: nothing to list.
                if !app.stepfunctions_state.executions.unsupported() {
                    let state = &mut app.stepfunctions_state.executions;
                    state.running = None;
                    state.table.loading = true;
                    state.error = None;
                }
                return true;
            }
            Action::Yank => {
                match selected(app) {
                    Some(e) if !e.arn.is_empty() => app.clipboard.request_text(&e.arn),
                    _ => app.clipboard.report_failure("No execution ARN selected"),
                }
                return true;
            }
            _ => {}
        }
    }
    let filtered_count = filtered(app).len();
    let selected_arn = selected(app)
        .filter(|e| !e.arn.is_empty())
        .map(|e| e.arn.clone());
    let state = &mut app.stepfunctions_state.executions;
    inventory::handle_action(
        InventoryInput {
            table: &mut state.table,
            focus: &mut state.input_focus,
            mode: &mut app.mode,
            page_input: &mut app.page_input,
            error_message: &mut app.error_message,
            error_scroll: &mut app.error_scroll,
            filtered_count,
            selected_arn,
            clipboard: &app.clipboard,
        },
        action,
    )
}

pub async fn poll(app: &mut App) -> bool {
    if app.stepfunctions_state.executions.machine.is_none() {
        return false;
    }
    let current_scope = scope(app);
    if app.stepfunctions_state.executions.scope != current_scope
        || !app.service_selected
        || !matches!(
            app.current_service,
            Service::StepFunctionsStateMachines | Service::CliOutput(_)
        )
    {
        app.stepfunctions_state.executions.close();
        return true;
    }
    // Express machines are never listed: no request is ever issued for them.
    if app.stepfunctions_state.executions.unsupported() {
        if app.stepfunctions_state.executions.table.loading {
            app.stepfunctions_state.executions.table.loading = false;
            return true;
        }
        return false;
    }
    let state = &mut app.stepfunctions_state.executions;
    let mut changed = false;
    if state.table.loading && state.running.is_none() {
        let machine = state.machine.as_ref().unwrap().clone();
        let config = app.config.clone();
        state.running = Some(Running(tokio::spawn(async move {
            StepFunctionsClient::new(config)
                .list_executions(&machine)
                .await
                .map_err(|e| format!("{e:#}"))
        })));
        changed = true;
    }
    if state
        .running
        .as_ref()
        .is_some_and(|task| task.0.is_finished())
    {
        let mut task = state.running.take().unwrap();
        let result = match (&mut task.0).await {
            Ok(result) => result,
            Err(e) => Err(format!("Execution read failed: {e}")),
        };
        state.table.loading = false;
        match result {
            Ok(rows) => {
                state.table.items = rows;
                state.loaded = true;
                state.error = None;
                state.table.reset();
            }
            Err(error) => {
                state.error = Some(
                    error
                        .chars()
                        .map(|c| if c.is_control() { ' ' } else { c })
                        .collect(),
                )
            }
        }
        changed = true;
    }
    changed
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let state = &app.stepfunctions_state.executions;
    let Some(machine) = &state.machine else {
        return;
    };
    let fields = vec![
        ui::labeled_field("Name", &machine.name),
        ui::labeled_field("ARN", &machine.arn),
        ui::labeled_field("Type", &machine.machine_type),
        ui::labeled_field("Created", &machine.creation_date),
    ];
    let summary = ui::Summary::new("Details", fields);
    let sections = Layout::vertical([
        Constraint::Length(summary.height(area.width)),
        Constraint::Min(0),
    ])
    .split(area);
    summary.render_fill(frame, sections[0]);
    let mut body = sections[1];

    // Express: explicit unsupported notice; the executions table is not shown
    // and no request was ever issued.
    if state.unsupported() {
        frame.render_widget(
            Paragraph::new(
                "Execution history is not available for EXPRESS state machines via ListExecutions. \
                 Use CloudWatch Logs if execution logging is configured. Ctrl+O opens the state machine console.",
            )
            .style(Style::default().fg(Color::Yellow))
            .wrap(Wrap { trim: false }),
            body,
        );
        return;
    }

    if let Some(error) = &state.error {
        let sections = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(body);
        let prefix = if state.loaded {
            "Refresh failed; showing previously loaded executions"
        } else {
            "Executions unavailable"
        };
        frame.render_widget(
            Paragraph::new(format!("{prefix}: {error} — Ctrl+R retries"))
                .style(Style::default().fg(Color::Yellow))
                .wrap(Wrap { trim: false }),
            sections[0],
        );
        body = sections[1];
    }
    let sections = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(body);
    let view = state
        .table
        .prepare_view(filtered(app), TableWindow::SelectedPage);
    let pagination = render_pagination_text(view.current_page, view.total_pages);
    render_simple_filter(
        frame,
        sections[0],
        SimpleFilterConfig {
            filter_text: &state.table.filter,
            placeholder: "Search by name, status, ARN, or date",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: state.input_focus == InputFocus::Filter,
            is_pagination_focused: state.input_focus == InputFocus::Pagination,
        },
    );
    let empty = view.items.is_empty();
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
            sort_column: &Column::Started.name(),
            sort_direction: SortDirection::Desc,
            title: ui::format_title(&format!("Executions ({})", view.total_items)),
            area: sections[1],
            get_expanded_content: Some(Box::new(|e| expanded_from_columns(&expanded, e))),
            is_active: app.mode == Mode::Normal,
        },
    );
    if empty && sections[1].height > 3 && sections[1].width > 2 {
        let message = if state.busy() {
            "Loading executions… Esc cancels"
        } else if state.error.is_some() {
            "Read failed — Ctrl+R retries"
        } else if !state.loaded {
            "Not loaded"
        } else if !state.table.items.is_empty() {
            "No matching executions"
        } else {
            "No executions for this state machine"
        };
        frame.render_widget(
            Paragraph::new(message),
            Rect {
                x: sections[1].x + 1,
                y: sections[1].y + 2,
                width: sections[1].width - 2,
                height: sections[1].height - 3,
            },
        );
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new_without_client("test".into(), Some("us-east-1".into()));
        app.config.account_id = "123456789012".into();
        app.current_service = Service::StepFunctionsStateMachines;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.stepfunctions_state.machines.items = vec![StateMachineMetadata {
            name: "orders".into(),
            arn: "arn:aws:states:us-east-1:123456789012:stateMachine:orders".into(),
            machine_type: "STANDARD".into(),
            creation_date: String::new(),
        }];
        app.handle_action(Action::Select);
        app
    }
    async fn finish(app: &mut App, result: Result<Vec<ExecutionMetadata>, String>) {
        app.stepfunctions_state.executions.running = Some(Running(tokio::spawn(async { result })));
        tokio::task::yield_now().await;
        assert!(poll(app).await);
    }
    #[tokio::test]
    async fn actual_refresh_retains_rows_on_failure_and_recovers_on_retry() {
        let mut app = app();
        finish(
            &mut app,
            Ok(vec![ExecutionMetadata {
                name: "kept".into(),
                arn: "arn:aws:states:us-east-1:123456789012:execution:orders:kept".into(),
                status: "SUCCEEDED".into(),
                ..Default::default()
            }]),
        )
        .await;
        app.handle_action(Action::Refresh);
        assert!(app.stepfunctions_state.executions.table.loading);
        assert_eq!(app.stepfunctions_state.executions.table.items.len(), 1);
        finish(&mut app, Err("AccessDenied".into())).await;
        assert_eq!(
            app.stepfunctions_state.executions.table.items[0].name,
            "kept"
        );
        assert!(app
            .stepfunctions_state
            .executions
            .error
            .as_deref()
            .unwrap()
            .contains("AccessDenied"));
        app.handle_action(Action::Refresh);
        finish(&mut app, Ok(vec![])).await;
        assert!(app.stepfunctions_state.executions.loaded);
        assert!(app.stepfunctions_state.executions.table.items.is_empty());
        assert!(app.stepfunctions_state.executions.error.is_none());
    }
    #[tokio::test]
    async fn closing_or_changing_scope_aborts_running_read() {
        for change in 0..5 {
            let mut app = app();
            let task =
                tokio::spawn(std::future::pending::<Result<Vec<ExecutionMetadata>, String>>());
            let abort = task.abort_handle();
            app.stepfunctions_state.executions.running = Some(Running(task));
            match change {
                0 => app.handle_action(Action::GoBack),
                1 => app.profile = "other".into(),
                2 => app.config.region = "eu-west-1".into(),
                3 => app.config.account_id = "999999999999".into(),
                _ => app.config.role_arn = "arn:aws:iam::123456789012:role/other".into(),
            }
            if change != 0 {
                assert!(poll(&mut app).await);
            }
            tokio::task::yield_now().await;
            assert!(abort.is_finished());
            assert!(app.stepfunctions_state.executions.machine.is_none());
            assert!(!app.stepfunctions_state.executions.busy());
        }
    }
}
