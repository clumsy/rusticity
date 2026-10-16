//! Topic-scoped subscriptions. Keys never await AWS and selection/filter state
//! in the parent Topics table is retained when this view closes.
use super::subscription::Column;
use crate::{
    app::{App, Service},
    common::{filter_by_fields, render_pagination_text, ColumnId, InputFocus, SortDirection},
    inventory::{self, InventoryInput},
    keymap::{Action, Mode},
    table::{TableState, TableWindow},
    ui::{
        self,
        filter::{render_simple_filter, SimpleFilterConfig},
        table::{expanded_from_columns, render_table, TableConfig},
    },
};
use ratatui::{
    prelude::*,
    widgets::{Paragraph, Wrap},
};
use rusticity_core::{sns::SnsSubscription, SnsClient, SnsTopic};
use tokio::task::JoinHandle;

struct Running(JoinHandle<Result<Vec<SnsSubscription>, String>>);
impl Drop for Running {
    fn drop(&mut self) {
        self.0.abort();
    }
}
pub struct State {
    pub topic: Option<SnsTopic>,
    pub table: TableState<SnsSubscription>,
    pub input_focus: InputFocus,
    pub column_ids: Vec<ColumnId>,
    pub visible_column_ids: Vec<ColumnId>,
    pub error: Option<String>,
    loaded: bool,
    scope: String,
    running: Option<Running>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            topic: None,
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
    pub fn close(&mut self) {
        self.running = None;
        self.topic = None;
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
        && app.current_service == Service::SnsTopics
        && app.sns_state.subscriptions.topic.is_some()
}
pub fn filtered(app: &App) -> Vec<&SnsSubscription> {
    let state = &app.sns_state.subscriptions;
    let mut rows = filter_by_fields(&state.table.items, &state.table.filter, |s| {
        [
            &s.protocol,
            &s.endpoint,
            &s.subscription_arn,
            &s.status,
            &s.owner,
        ]
    });
    rows.sort_by(|a, b| {
        a.protocol
            .cmp(&b.protocol)
            .then_with(|| a.endpoint.cmp(&b.endpoint))
            .then_with(|| a.subscription_arn.cmp(&b.subscription_arn))
    });
    rows
}
pub fn selected(app: &App) -> Option<&SnsSubscription> {
    let rows = filtered(app);
    rows.get(app.sns_state.subscriptions.table.selected)
        .copied()
}
pub fn go_to_page(app: &mut App, page: usize) {
    let count = filtered(app).len();
    inventory::go_to_page(&mut app.sns_state.subscriptions.table, page, count);
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !app.service_selected || app.current_service != Service::SnsTopics {
        return false;
    }
    if !active(app) {
        if app.mode == Mode::Normal && *action == Action::Select {
            let rows = ui::sns::filtered_topics(app);
            let Some(topic) = app.sns_state.topics.get_selected(&rows).cloned() else {
                return true;
            };
            let scope = scope(app);
            let state = &mut app.sns_state.subscriptions;
            state.close();
            state.topic = Some(topic);
            state.scope = scope;
            state.table.loading = true;
            app.page_input.clear();
            app.update_current_tab_breadcrumb();
            return true;
        }
        return false;
    }
    if app.mode == Mode::Normal {
        match action {
            Action::GoBack => {
                app.sns_state.subscriptions.close();
                app.page_input.clear();
                app.update_current_tab_breadcrumb();
                return true;
            }
            Action::Refresh => {
                let state = &mut app.sns_state.subscriptions;
                state.running = None;
                state.table.loading = true;
                state.error = None;
                return true;
            }
            Action::Yank => {
                match selected(app).filter(|s| s.status == "Confirmed") {
                    Some(s) => app.clipboard.request_text(&s.subscription_arn),
                    None => app
                        .clipboard
                        .report_failure("No confirmed subscription ARN selected"),
                }
                return true;
            }
            _ => {}
        }
    }
    let filtered_count = filtered(app).len();
    let selected_arn = selected(app)
        .filter(|s| s.status == "Confirmed")
        .map(|s| s.subscription_arn.clone());
    let state = &mut app.sns_state.subscriptions;
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
    if app.sns_state.subscriptions.topic.is_none() {
        return false;
    }
    let current_scope = scope(app);
    if app.sns_state.subscriptions.scope != current_scope
        || !app.service_selected
        || !matches!(
            app.current_service,
            Service::SnsTopics | Service::CliOutput(_)
        )
    {
        app.sns_state.subscriptions.close();
        return true;
    }
    let state = &mut app.sns_state.subscriptions;
    let mut changed = false;
    if state.table.loading && state.running.is_none() {
        let topic = state.topic.as_ref().unwrap().arn.clone();
        let config = app.config.clone();
        state.running = Some(Running(tokio::spawn(async move {
            SnsClient::new(config)
                .list_subscriptions_by_topic(&topic)
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
            Err(e) => Err(format!("Subscription read failed: {e}")),
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
    let state = &app.sns_state.subscriptions;
    let Some(topic) = &state.topic else { return };
    let fields = vec![
        ui::labeled_field("Name", &topic.name),
        ui::labeled_field("Type", &topic.topic_type),
        ui::labeled_field("ARN", &topic.arn),
    ];
    let summary = ui::Summary::new("Details", fields);
    let sections = Layout::vertical([
        Constraint::Length(summary.height(area.width)),
        Constraint::Min(0),
    ])
    .split(area);
    summary.render_fill(frame, sections[0]);
    let mut body = sections[1];
    if let Some(error) = &state.error {
        let sections = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(body);
        let prefix = if state.loaded {
            "Refresh failed; showing previously loaded subscriptions"
        } else {
            "Subscriptions unavailable"
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
            placeholder: "Search by protocol, endpoint, ARN, status, or owner",
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
            sort_column: &Column::Protocol.name(),
            sort_direction: SortDirection::Asc,
            title: ui::format_title(&format!("Subscriptions ({})", view.total_items)),
            area: sections[1],
            get_expanded_content: Some(Box::new(|s| expanded_from_columns(&expanded, s))),
            is_active: app.mode == Mode::Normal,
        },
    );
    if empty && sections[1].height > 3 && sections[1].width > 2 {
        let message = if state.busy() {
            "Loading subscriptions… Esc cancels"
        } else if state.error.is_some() {
            "Read failed — Ctrl+R retries"
        } else if !state.loaded {
            "Not loaded"
        } else if !state.table.items.is_empty() {
            "No matching subscriptions"
        } else {
            "No subscriptions for this topic"
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
mod tests;
