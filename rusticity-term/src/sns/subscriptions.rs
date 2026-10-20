//! Topic-scoped subscriptions. Keys never await AWS and selection/filter state
//! in the parent Topics table is retained when this view closes.
use super::subscription::Column;
use crate::{
    app::{App, Service},
    async_read::{AsyncRead, ReadScope, ReadStatus},
    common::{filter_by_fields, render_pagination_text, ColumnId, InputFocus, SortDirection},
    inventory::{self, InventoryInput},
    keymap::{Action, Mode},
    table::{TableState, TableWindow},
    ui::{
        self,
        filter::{render_simple_filter, SimpleFilterConfig},
        loading::render_table,
        table::{expanded_from_columns, TableConfig},
    },
};
use ratatui::prelude::*;
use rusticity_core::{sns::SnsSubscription, SnsClient, SnsTopic};

pub struct State {
    pub topic: Option<SnsTopic>,
    pub table: TableState<SnsSubscription>,
    pub input_focus: InputFocus,
    pub column_ids: Vec<ColumnId>,
    pub visible_column_ids: Vec<ColumnId>,
    pub error: Option<String>,
    read: AsyncRead<Vec<SnsSubscription>>,
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
            read: AsyncRead::default(),
        }
    }
}
impl State {
    pub fn busy(&self) -> bool {
        self.read.is_busy()
    }
    /// View-facing read state, never inferred from row count. Loading from the
    /// shared read while busy; Failed when the last read failed; Ready once a
    /// read completed; Idle when no key is entered.
    pub(crate) fn read_status(&self) -> ReadStatus {
        if self.read.is_busy() {
            self.read.status()
        } else if self.error.is_some() {
            ReadStatus::Failed
        } else {
            self.read.status()
        }
    }
    pub fn close(&mut self) {
        *self = State::default();
    }
    /// Clear results immediately for a refresh of the SAME topic: drop the old
    /// rows, selection and error so nothing stale is shown while the fresh read
    /// runs. Only resource identity, filter and column preferences survive.
    fn clear_for_refresh(&mut self) {
        self.table.items.clear();
        self.table.reset();
        self.error = None;
    }
}
fn scope(app: &App, topic: &SnsTopic) -> ReadScope {
    ReadScope::from_app(app, topic.arn.clone())
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
            let scope = scope(app, &topic);
            let state = &mut app.sns_state.subscriptions;
            state.close();
            state.read.request(scope);
            state.topic = Some(topic);
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
                let scope = app
                    .sns_state
                    .subscriptions
                    .topic
                    .as_ref()
                    .map(|topic| scope(app, topic));
                let state = &mut app.sns_state.subscriptions;
                if let Some(scope) = scope {
                    if !state.read.matches_scope(&scope) {
                        state.close();
                        app.update_current_tab_breadcrumb();
                        return true;
                    }
                    // Drop the displayed rows the instant a refresh begins so the
                    // old result is never shown during or after the fresh read.
                    state.clear_for_refresh();
                    state.read.request(scope);
                }
                state.table.loading = true;
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
    let topic = app.sns_state.subscriptions.topic.as_ref().unwrap().clone();
    let current_scope = scope(app, &topic);
    if !app
        .sns_state
        .subscriptions
        .read
        .matches_scope(&current_scope)
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
    if state.table.loading && !state.read.is_busy() {
        // Shared InventoryInput can schedule a retry through table.loading.
        state.read.request(current_scope.clone());
    }
    if state.table.loading && state.read.is_pending() {
        let config = app.config.clone();
        let topic_arn = topic.arn.clone();
        if state.read.start(async move {
            SnsClient::new(config)
                .list_subscriptions_by_topic(&topic_arn)
                .await
                .map_err(|e| format!("{e:#}"))
        }) {
            changed = true;
        }
    }
    if let Some(result) = state.read.take_ready(&current_scope).await {
        state.table.loading = false;
        match result {
            Ok(rows) => {
                state.table.items = rows;
                state.error = None;
                state.table.reset();
            }
            Err(error) => {
                // Never leave stale rows behind a failed read.
                state.table.items.clear();
                state.table.reset();
                state.error = Some(
                    error
                        .into_message("Subscription read failed")
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
    let body = sections[1];
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
        state.read_status(),
        "Subscriptions",
        state.error.as_deref(),
        ui::loading::EmptyState {
            no_items: "No subscriptions for this topic",
            has_unfiltered_items: !state.table.items.is_empty(),
        },
    );
}
#[cfg(test)]
mod tests;
