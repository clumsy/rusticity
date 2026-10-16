use crate::app::{App, Service};
use crate::common::{CyclicEnum, InputFocus};
use crate::preferences::bindings;
use crate::ui::dynamodb::{
    current_table, filtered_dynamodb_tables, filtered_items, DetailTab, ItemOperation,
    ItemProjection, FILTER_CONTROLS, ITEMS_FILTER_CONTROLS, ITEM_INDEX, ITEM_OPERATION,
    ITEM_PROJECTION,
};

fn current_index_count(app: &App) -> usize {
    current_table(app).map(|t| t.index_count()).unwrap_or(0)
}

fn current_replica_count(app: &App) -> usize {
    current_table(app).map(|t| t.replicas.len()).unwrap_or(0)
}

// ── Navigation ────────────────────────────────────────────────────────────────

pub fn next_item(app: &mut App) {
    if app.dynamodb_state.current_table.is_some() {
        match app.dynamodb_state.detail_tab {
            DetailTab::Items => {
                let n = filtered_items(app).len();
                if n > 0 {
                    app.dynamodb_state.items.next_item(n);
                }
            }
            DetailTab::Indexes => {
                let n = current_index_count(app);
                if n > 0 {
                    app.dynamodb_state.indexes_table.next_item(n);
                }
            }
            DetailTab::GlobalTables => {
                let n = current_replica_count(app);
                if n > 0 {
                    app.dynamodb_state.replicas_table.next_item(n);
                }
            }
            DetailTab::Backups => {
                let n = app.dynamodb_state.backups.items.len();
                if n > 0 {
                    app.dynamodb_state.backups.next_item(n);
                }
            }
            DetailTab::Tags => {
                let n = app.dynamodb_state.tags_table.items.len();
                if n > 0 {
                    app.dynamodb_state.tags_table.next_item(n);
                }
            }
            _ => {}
        }
        return;
    }
    let filtered = filtered_dynamodb_tables(app);
    if !filtered.is_empty() {
        app.dynamodb_state.tables.next_item(filtered.len());
    }
}

pub fn prev_item(app: &mut App) {
    if app.dynamodb_state.current_table.is_some() {
        match app.dynamodb_state.detail_tab {
            DetailTab::Items => app.dynamodb_state.items.prev_item(),
            DetailTab::Indexes => app.dynamodb_state.indexes_table.prev_item(),
            DetailTab::GlobalTables => app.dynamodb_state.replicas_table.prev_item(),
            DetailTab::Backups => app.dynamodb_state.backups.prev_item(),
            DetailTab::Tags => app.dynamodb_state.tags_table.prev_item(),
            _ => {}
        }
        return;
    }
    app.dynamodb_state.tables.prev_item();
}

pub fn page_down_filter_input(app: &mut App) {
    if is_items_tab(app) {
        if app.dynamodb_state.input_focus == InputFocus::Filter {
            page_down_normal(app);
        } else {
            let count = filtered_items(app).len();
            let state = &mut app.dynamodb_state;
            state.input_focus.handle_page_down(
                &mut state.items.selected,
                &mut state.items.scroll_offset,
                state.items.page_size.value(),
                count,
            );
        }
        return;
    }
    if app.dynamodb_state.input_focus == InputFocus::Filter {
        let filtered = filtered_dynamodb_tables(app);
        app.dynamodb_state.tables.page_down(filtered.len());
    } else {
        let page_size = app.dynamodb_state.tables.page_size.value();
        let filtered_count = filtered_dynamodb_tables(app).len();
        app.dynamodb_state.input_focus.handle_page_down(
            &mut app.dynamodb_state.tables.selected,
            &mut app.dynamodb_state.tables.scroll_offset,
            page_size,
            filtered_count,
        );
    }
}

pub fn page_down_normal(app: &mut App) {
    if is_items_tab(app) {
        let n = filtered_items(app).len();
        app.dynamodb_state.items.page_down(n);
        return;
    }
    let filtered = filtered_dynamodb_tables(app);
    app.dynamodb_state.tables.page_down(filtered.len());
}

pub fn page_up_filter_input(app: &mut App) {
    if is_items_tab(app) {
        if app.dynamodb_state.input_focus == InputFocus::Filter {
            page_up_normal(app);
        } else {
            let state = &mut app.dynamodb_state;
            state.input_focus.handle_page_up(
                &mut state.items.selected,
                &mut state.items.scroll_offset,
                state.items.page_size.value(),
            );
        }
        return;
    }
    if app.dynamodb_state.input_focus == InputFocus::Filter {
        app.dynamodb_state.tables.page_up();
    } else {
        let page_size = app.dynamodb_state.tables.page_size.value();
        app.dynamodb_state.input_focus.handle_page_up(
            &mut app.dynamodb_state.tables.selected,
            &mut app.dynamodb_state.tables.scroll_offset,
            page_size,
        );
    }
}

pub fn page_up_normal(app: &mut App) {
    if is_items_tab(app) {
        app.dynamodb_state.items.page_up();
        return;
    }
    app.dynamodb_state.tables.page_up();
}

/// True when the Items explorer is the active view.
pub fn is_items_tab(app: &App) -> bool {
    app.dynamodb_state.current_table.is_some() && app.dynamodb_state.detail_tab == DetailTab::Items
}

pub fn expand_row(app: &mut App) {
    if app.dynamodb_state.current_table.is_some() {
        match app.dynamodb_state.detail_tab {
            DetailTab::Items => app.dynamodb_state.items.toggle_expand(),
            DetailTab::Indexes => app.dynamodb_state.indexes_table.toggle_expand(),
            DetailTab::GlobalTables => app.dynamodb_state.replicas_table.toggle_expand(),
            DetailTab::Backups => app.dynamodb_state.backups.toggle_expand(),
            DetailTab::Tags => app.dynamodb_state.tags_table.toggle_expand(),
            _ => {}
        }
    } else {
        app.dynamodb_state.tables.toggle_expand();
    }
}

pub fn prev_pane(app: &mut App) {
    collapse_row(app);
}

pub fn collapse_row(app: &mut App) {
    if app.dynamodb_state.current_table.is_some() {
        match app.dynamodb_state.detail_tab {
            DetailTab::Items => app.dynamodb_state.items.collapse(),
            DetailTab::Indexes => app.dynamodb_state.indexes_table.collapse(),
            DetailTab::GlobalTables => app.dynamodb_state.replicas_table.collapse(),
            DetailTab::Backups => app.dynamodb_state.backups.collapse(),
            DetailTab::Tags => app.dynamodb_state.tags_table.collapse(),
            _ => {}
        }
    } else {
        app.dynamodb_state.tables.collapse();
    }
}

pub fn go_to_page(app: &mut App, page: usize) {
    if is_items_tab(app) {
        let count = filtered_items(app).len();
        app.dynamodb_state.items.goto_page(page.max(1), count);
        return;
    }
    let filtered_count = filtered_dynamodb_tables(app).len();
    app.dynamodb_state.tables.goto_page(page, filtered_count);
}

// ── Filter ────────────────────────────────────────────────────────────────────

pub fn get_active_filter_mut(app: &mut App) -> Option<&mut String> {
    if is_items_tab(app) {
        return Some(&mut app.dynamodb_state.item_filter);
    }
    Some(&mut app.dynamodb_state.tables.filter)
}

pub fn apply_filter_reset(app: &mut App) {
    if is_items_tab(app) {
        app.dynamodb_state.item_filter.clear();
        reload_items(app);
        return;
    }
    app.dynamodb_state.tables.reset();
}

pub fn start_filter(app: &mut App) {
    app.dynamodb_state.input_focus = InputFocus::Filter;
}

pub fn next_filter_focus(app: &mut App) {
    let controls = filter_controls(app);
    app.dynamodb_state.input_focus = app.dynamodb_state.input_focus.next(controls);
}

pub fn prev_filter_focus(app: &mut App) {
    let controls = filter_controls(app);
    app.dynamodb_state.input_focus = app.dynamodb_state.input_focus.prev(controls);
}

/// The Items tab adds the operation, index and projection dropdowns.
fn filter_controls(app: &App) -> &'static [InputFocus] {
    if is_items_tab(app) {
        &ITEMS_FILTER_CONTROLS
    } else {
        &FILTER_CONTROLS
    }
}

// ── Items tab dropdowns ───────────────────────────────────────────────────────

/// Cycle the focused Items dropdown forward (j / Down in filter input).
pub fn next_dropdown_value(app: &mut App) {
    cycle_dropdown(app, true);
}

/// Cycle the focused Items dropdown backward (k / Up in filter input).
pub fn prev_dropdown_value(app: &mut App) {
    cycle_dropdown(app, false);
}

fn cycle_dropdown(app: &mut App, forward: bool) {
    if !is_items_tab(app) {
        return;
    }
    let index_count = current_table(app).map(|t| t.index_count() + 1).unwrap_or(1);
    let focus = app.dynamodb_state.input_focus;
    if focus == ITEM_OPERATION {
        app.dynamodb_state.item_operation = if forward {
            app.dynamodb_state.item_operation.next()
        } else {
            app.dynamodb_state.item_operation.prev()
        };
    } else if focus == ITEM_INDEX {
        let current = app.dynamodb_state.item_index;
        app.dynamodb_state.item_index = if forward {
            (current + 1) % index_count
        } else {
            (current + index_count - 1) % index_count
        };
    } else if focus == ITEM_PROJECTION {
        app.dynamodb_state.item_projection = if forward {
            app.dynamodb_state.item_projection.next()
        } else {
            app.dynamodb_state.item_projection.prev()
        };
    } else {
        return;
    }
    reload_items(app);
}

/// Drop the current result set and schedule a fresh Scan or Query.
pub fn reload_items(app: &mut App) {
    app.dynamodb_state.clear_items_result();
    // A Query with no partition-key value yet has nothing to run.
    app.dynamodb_state.items_loading = app.dynamodb_state.item_operation == ItemOperation::Scan
        || !app.dynamodb_state.item_filter.is_empty();
}

pub fn is_pagination_focused(app: &App) -> bool {
    app.dynamodb_state.input_focus == InputFocus::Pagination
}

// ── Yank / console URL / breadcrumb ──────────────────────────────────────────

pub fn yank(app: &App) {
    if app.dynamodb_state.current_table.is_some() {
        // Detail view — copy the ARN (fall back to the table name).
        if let Some(t) = current_table(app) {
            if t.table_arn.is_empty() {
                app.clipboard.request_text(&t.name);
            } else {
                app.clipboard.request_text(&t.table_arn);
            }
        }
    } else {
        let filtered = filtered_dynamodb_tables(app);
        if let Some(t) = app.dynamodb_state.tables.get_selected(&filtered) {
            app.clipboard.request_text(&t.name);
        }
    }
}

pub fn breadcrumb(app: &App) -> Vec<String> {
    let mut parts = vec!["DynamoDB".to_string()];
    if let Some(name) = &app.dynamodb_state.current_table {
        parts.push(name.clone());
    } else {
        parts.push("Tables".to_string());
    }
    parts
}

pub fn console_url(app: &App) -> String {
    use crate::dynamodb;
    if let Some(name) = &app.dynamodb_state.current_table {
        return dynamodb::console_url_table(
            &app.config.region,
            name,
            app.dynamodb_state.detail_tab.console_tab(),
        );
    }
    dynamodb::console_url_tables(&app.config.region)
}

// ── Select / go back / detail tabs ───────────────────────────────────────────

pub fn select_item(app: &mut App) {
    if app.dynamodb_state.current_table.is_some() {
        return;
    }
    let filtered = filtered_dynamodb_tables(app);
    let Some(name) = app
        .dynamodb_state
        .tables
        .get_selected(&filtered)
        .map(|t| t.name.clone())
    else {
        return;
    };
    app.dynamodb_state.current_table = Some(name);
    app.dynamodb_state.detail_tab = DetailTab::Items;
    reset_detail_tables(app);
    app.dynamodb_state.item_operation = ItemOperation::default();
    app.dynamodb_state.item_index = 0;
    app.dynamodb_state.item_projection = ItemProjection::default();
    app.dynamodb_state.item_filter.clear();
    // Attribute columns are table specific, so start with all of them visible.
    app.dynamodb_state.item_hidden_columns.clear();
    // Items is the landing tab, so schedule its scan straight away.
    reload_items(app);
}

pub fn go_back(app: &mut App) {
    app.dynamodb_state.current_table = None;
    app.dynamodb_state.detail_tab = DetailTab::Items;
}

pub fn next_detail_tab(app: &mut App) {
    app.dynamodb_state.detail_tab = app.dynamodb_state.detail_tab.next();
    reset_detail_tables(app);
    maybe_load_tab_data(app);
}

pub fn prev_detail_tab(app: &mut App) {
    app.dynamodb_state.detail_tab = app.dynamodb_state.detail_tab.prev();
    reset_detail_tables(app);
    maybe_load_tab_data(app);
}

fn reset_detail_tables(app: &mut App) {
    app.dynamodb_state.items.reset();
    app.dynamodb_state.indexes_table.reset();
    app.dynamodb_state.replicas_table.reset();
    app.dynamodb_state.backups.reset();
    app.dynamodb_state.tags_table.reset();
}

/// Mark on-demand data as needing a load when the owning tab becomes active.
fn maybe_load_tab_data(app: &mut App) {
    match app.dynamodb_state.detail_tab {
        // TTL shows on Settings and PITR on Backups — one call feeds both.
        DetailTab::Settings => {
            app.dynamodb_state.settings_loading = true;
        }
        DetailTab::Backups => {
            app.dynamodb_state.backups.items.clear();
            app.dynamodb_state.backups_loading = true;
            app.dynamodb_state.settings_loading = true;
        }
        DetailTab::AdditionalSettings => {
            app.dynamodb_state.table_policy = None;
            app.dynamodb_state.stream_policy = None;
            app.dynamodb_state.permissions_loading = true;
        }
        DetailTab::Items => {
            reload_items(app);
        }
        DetailTab::Tags => {
            app.dynamodb_state.tags_table.items.clear();
            app.dynamodb_state.tags_loading = true;
        }
        DetailTab::Monitor => {
            use crate::ui::monitoring::MonitoringState;
            app.dynamodb_state.clear_metrics();
            app.dynamodb_state.monitoring_scroll = 0;
            app.dynamodb_state.metrics_loading = true;
        }
        _ => {}
    }
}

// ── Monitor scrolling ─────────────────────────────────────────────────────────

/// Scroll the Monitor charts down (Ctrl+D / ScrollDown).
pub fn scroll_down_detail(app: &mut App) {
    use crate::ui::monitoring::MonitoringState;
    if app.dynamodb_state.detail_tab == DetailTab::Monitor {
        let max = crate::ui::dynamodb::MONITORING_CHARTS.saturating_sub(1);
        app.dynamodb_state
            .set_monitoring_scroll((app.dynamodb_state.monitoring_scroll() + 1).min(max));
    }
}

/// Scroll the Monitor charts up (Ctrl+U / ScrollUp).
pub fn scroll_up_detail(app: &mut App) {
    use crate::ui::monitoring::MonitoringState;
    if app.dynamodb_state.detail_tab == DetailTab::Monitor {
        app.dynamodb_state
            .set_monitoring_scroll(app.dynamodb_state.monitoring_scroll().saturating_sub(1));
    }
}

// ── Column preferences ────────────────────────────────────────────────────────

pub fn toggle_column(app: &mut App) {
    bindings::activate_for(app, Service::DynamoDbTables);
}

pub fn next_preferences(app: &mut App) {
    bindings::move_section_for(app, Service::DynamoDbTables, true);
}

pub fn prev_preferences(app: &mut App) {
    bindings::move_section_for(app, Service::DynamoDbTables, false);
}

pub fn column_selector_max(app: &App) -> usize {
    bindings::model_for(app, Service::DynamoDbTables).max_index()
}

pub fn column_count(app: &App) -> usize {
    bindings::model_for(app, Service::DynamoDbTables).column_count()
}
