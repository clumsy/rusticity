use crate::app::{App, ViewMode};
use crate::common::{CyclicEnum, InputFocus, PageSize};
use crate::ui::monitoring::MonitoringState;
use crate::ui::sqs::{
    filtered_eventbridge_pipes, filtered_lambda_triggers, filtered_queues, filtered_subscriptions,
    filtered_tags, QueueDetailTab as SqsQueueDetailTab, FILTER_CONTROLS,
    SUBSCRIPTION_FILTER_CONTROLS,
};

// ── Filter helpers ────────────────────────────────────────────────────────────

pub fn get_active_filter_mut(app: &mut App) -> Option<&mut String> {
    if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers
    {
        Some(&mut app.sqs_state.triggers.filter)
    } else if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::EventBridgePipes
    {
        Some(&mut app.sqs_state.pipes.filter)
    } else if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::Tagging
    {
        Some(&mut app.sqs_state.tags.filter)
    } else if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions
    {
        Some(&mut app.sqs_state.subscriptions.filter)
    } else {
        Some(&mut app.sqs_state.queues.filter)
    }
}

pub fn apply_filter_reset(app: &mut App) {
    app.sqs_state.queues.reset();
}

pub fn is_pagination_focused(app: &App) -> bool {
    app.sqs_state.input_focus == InputFocus::Pagination
}

pub fn next_filter_focus(app: &mut App) {
    if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions
    {
        app.sqs_state.input_focus = app.sqs_state.input_focus.next(SUBSCRIPTION_FILTER_CONTROLS);
    } else {
        app.sqs_state.input_focus = app.sqs_state.input_focus.next(FILTER_CONTROLS);
    }
}

pub fn prev_filter_focus(app: &mut App) {
    if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions
    {
        app.sqs_state.input_focus = app.sqs_state.input_focus.prev(SUBSCRIPTION_FILTER_CONTROLS);
    } else {
        app.sqs_state.input_focus = app.sqs_state.input_focus.prev(FILTER_CONTROLS);
    }
}

/// Cycle the subscription region dropdown in FilterInput mode.
pub fn next_item_filter_input(app: &mut App) {
    use crate::ui::sqs::SUBSCRIPTION_REGION;
    if app.sqs_state.input_focus == SUBSCRIPTION_REGION {
        let regions = crate::app::AwsRegion::all();
        app.sqs_state.subscription_region_selected =
            (app.sqs_state.subscription_region_selected + 1).min(regions.len() - 1);
        app.sqs_state.subscriptions.reset();
    }
}

pub fn prev_item_filter_input(app: &mut App) {
    use crate::ui::sqs::SUBSCRIPTION_REGION;
    if app.sqs_state.input_focus == SUBSCRIPTION_REGION {
        app.sqs_state.subscription_region_selected =
            app.sqs_state.subscription_region_selected.saturating_sub(1);
        app.sqs_state.subscriptions.reset();
    }
}

// ── Navigation ────────────────────────────────────────────────────────────────

pub fn next_item(app: &mut App) {
    if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers
    {
        let filtered = filtered_lambda_triggers(app);
        if !filtered.is_empty() {
            app.sqs_state.triggers.next_item(filtered.len());
        }
    } else if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::EventBridgePipes
    {
        let filtered = filtered_eventbridge_pipes(app);
        if !filtered.is_empty() {
            app.sqs_state.pipes.next_item(filtered.len());
        }
    } else if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::Tagging
    {
        let filtered = filtered_tags(app);
        if !filtered.is_empty() {
            app.sqs_state.tags.next_item(filtered.len());
        }
    } else if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions
    {
        let filtered = filtered_subscriptions(app);
        if !filtered.is_empty() {
            app.sqs_state.subscriptions.next_item(filtered.len());
        }
    } else {
        let filtered = filtered_queues(&app.sqs_state.queues.items, &app.sqs_state.queues.filter);
        if !filtered.is_empty() {
            app.sqs_state.queues.next_item(filtered.len());
        }
    }
}

pub fn prev_item(app: &mut App) {
    if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers
    {
        app.sqs_state.triggers.prev_item();
    } else if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::EventBridgePipes
    {
        app.sqs_state.pipes.prev_item();
    } else if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::Tagging
    {
        app.sqs_state.tags.prev_item();
    } else if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions
    {
        app.sqs_state.subscriptions.prev_item();
    } else {
        app.sqs_state.queues.prev_item();
    }
}

/// ScrollDown in detail view (policy scroll or monitoring).
pub fn scroll_down_detail(app: &mut App) {
    if app.sqs_state.current_queue.is_some() {
        if app.sqs_state.detail_tab == SqsQueueDetailTab::Monitoring {
            app.sqs_state
                .set_monitoring_scroll((app.sqs_state.monitoring_scroll() + 1).min(8));
        } else {
            let lines = app.sqs_state.policy_document.lines().count();
            let max_scroll = lines.saturating_sub(1);
            app.sqs_state.policy_scroll = (app.sqs_state.policy_scroll + 1).min(max_scroll);
        }
    }
}

/// ScrollDown fast (Ctrl+D) in detail view.
pub fn scroll_down_fast(app: &mut App) {
    if app.sqs_state.current_queue.is_some() {
        if app.sqs_state.detail_tab == SqsQueueDetailTab::Monitoring {
            app.sqs_state
                .set_monitoring_scroll((app.sqs_state.monitoring_scroll() + 1).min(8));
        } else {
            let lines = app.sqs_state.policy_document.lines().count();
            let max_scroll = lines.saturating_sub(1);
            app.sqs_state.policy_scroll = (app.sqs_state.policy_scroll + 10).min(max_scroll);
        }
    }
}

/// ScrollUp fast (Ctrl+U) in detail view.
pub fn scroll_up_fast(app: &mut App) {
    if app.sqs_state.current_queue.is_some() {
        if app.sqs_state.detail_tab == SqsQueueDetailTab::Monitoring {
            app.sqs_state
                .set_monitoring_scroll(app.sqs_state.monitoring_scroll().saturating_sub(1));
        } else {
            app.sqs_state.policy_scroll = app.sqs_state.policy_scroll.saturating_sub(10);
        }
    }
}

pub fn page_down_normal(app: &mut App) {
    let filtered = filtered_queues(&app.sqs_state.queues.items, &app.sqs_state.queues.filter);
    app.sqs_state.queues.page_down(filtered.len());
}

pub fn page_up_normal(app: &mut App) {
    app.sqs_state.queues.page_up();
}

pub fn go_to_page(app: &mut App, page: usize) {
    let filtered_count =
        filtered_queues(&app.sqs_state.queues.items, &app.sqs_state.queues.filter).len();
    app.sqs_state.queues.goto_page(page, filtered_count);
}

// ── Expand / collapse ─────────────────────────────────────────────────────────

pub fn expand_row(app: &mut App) {
    if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers
    {
        app.sqs_state.triggers.toggle_expand();
    } else if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::EventBridgePipes
    {
        app.sqs_state.pipes.toggle_expand();
    } else if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::Tagging
    {
        app.sqs_state.tags.toggle_expand();
    } else if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions
    {
        app.sqs_state.subscriptions.toggle_expand();
    } else {
        app.sqs_state.queues.expand();
    }
}

pub fn prev_pane(app: &mut App) {
    if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers
    {
        app.sqs_state.triggers.collapse();
    } else if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::EventBridgePipes
    {
        app.sqs_state.pipes.collapse();
    } else if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::Tagging
    {
        app.sqs_state.tags.collapse();
    } else if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions
    {
        app.sqs_state.subscriptions.collapse();
    } else {
        app.sqs_state.queues.collapse();
    }
}

pub fn collapse_row(app: &mut App) {
    app.sqs_state.queues.collapse();
}

// ── Detail tabs ───────────────────────────────────────────────────────────────

pub fn next_detail_tab(app: &mut App) {
    app.sqs_state.detail_tab = app.sqs_state.detail_tab.next();
    if app.sqs_state.detail_tab == SqsQueueDetailTab::Monitoring {
        app.sqs_state.set_metrics_loading(true);
        app.sqs_state.set_monitoring_scroll(0);
        app.sqs_state.clear_metrics();
    } else if app.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers {
        app.sqs_state.triggers.loading = true;
    } else if app.sqs_state.detail_tab == SqsQueueDetailTab::EventBridgePipes {
        app.sqs_state.pipes.loading = true;
    } else if app.sqs_state.detail_tab == SqsQueueDetailTab::Tagging {
        app.sqs_state.tags.loading = true;
    } else if app.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions {
        app.sqs_state.subscriptions.loading = true;
    }
}

pub fn prev_detail_tab(app: &mut App) {
    app.sqs_state.detail_tab = app.sqs_state.detail_tab.prev();
    if app.sqs_state.detail_tab == SqsQueueDetailTab::Monitoring {
        app.sqs_state.set_metrics_loading(true);
        app.sqs_state.set_monitoring_scroll(0);
        app.sqs_state.clear_metrics();
    }
}

// ── Select / go back ──────────────────────────────────────────────────────────

pub fn select_item(app: &mut App) {
    if app.sqs_state.current_queue.is_none() {
        let filtered = filtered_queues(&app.sqs_state.queues.items, &app.sqs_state.queues.filter);
        if let Some(queue) = app.sqs_state.queues.get_selected(&filtered) {
            app.sqs_state.current_queue = Some(queue.url.clone());
            if app.sqs_state.detail_tab == SqsQueueDetailTab::Monitoring {
                app.sqs_state.metrics_loading = true;
            } else if app.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers {
                app.sqs_state.triggers.loading = true;
            } else if app.sqs_state.detail_tab == SqsQueueDetailTab::EventBridgePipes {
                app.sqs_state.pipes.loading = true;
            } else if app.sqs_state.detail_tab == SqsQueueDetailTab::Tagging {
                app.sqs_state.tags.loading = true;
            } else if app.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions {
                app.sqs_state.subscriptions.loading = true;
            }
        }
    }
}

pub fn go_back(app: &mut App) {
    app.sqs_state.current_queue = None;
    app.view_mode = ViewMode::List;
    app.update_current_tab_breadcrumb();
}

// ── Column preferences ────────────────────────────────────────────────────────

pub fn column_selector_max(app: &App) -> usize {
    app.sqs_column_ids.len() - 1
}

pub fn column_count(app: &App) -> usize {
    app.sqs_column_ids.len()
}

pub fn toggle_column(app: &mut App) {
    let idx = app.column_selector_index;
    if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers
    {
        if idx > 0 && idx <= app.sqs_state.trigger_column_ids.len() {
            if let Some(col) = app.sqs_state.trigger_column_ids.get(idx - 1) {
                if let Some(pos) = app
                    .sqs_state
                    .trigger_visible_column_ids
                    .iter()
                    .position(|c| c == col)
                {
                    app.sqs_state.trigger_visible_column_ids.remove(pos);
                } else {
                    app.sqs_state.trigger_visible_column_ids.push(col.clone());
                }
            }
        } else if idx == app.sqs_state.trigger_column_ids.len() + 3 {
            app.sqs_state.triggers.page_size = PageSize::Ten;
        } else if idx == app.sqs_state.trigger_column_ids.len() + 4 {
            app.sqs_state.triggers.page_size = PageSize::TwentyFive;
        } else if idx == app.sqs_state.trigger_column_ids.len() + 5 {
            app.sqs_state.triggers.page_size = PageSize::Fifty;
        } else if idx == app.sqs_state.trigger_column_ids.len() + 6 {
            app.sqs_state.triggers.page_size = PageSize::OneHundred;
        }
    } else if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::EventBridgePipes
    {
        if idx > 0 && idx <= app.sqs_state.pipe_column_ids.len() {
            if let Some(col) = app.sqs_state.pipe_column_ids.get(idx - 1) {
                if let Some(pos) = app
                    .sqs_state
                    .pipe_visible_column_ids
                    .iter()
                    .position(|c| c == col)
                {
                    app.sqs_state.pipe_visible_column_ids.remove(pos);
                } else {
                    app.sqs_state.pipe_visible_column_ids.push(col.clone());
                }
            }
        } else if idx == app.sqs_state.pipe_column_ids.len() + 3 {
            app.sqs_state.pipes.page_size = PageSize::Ten;
        } else if idx == app.sqs_state.pipe_column_ids.len() + 4 {
            app.sqs_state.pipes.page_size = PageSize::TwentyFive;
        } else if idx == app.sqs_state.pipe_column_ids.len() + 5 {
            app.sqs_state.pipes.page_size = PageSize::Fifty;
        } else if idx == app.sqs_state.pipe_column_ids.len() + 6 {
            app.sqs_state.pipes.page_size = PageSize::OneHundred;
        }
    } else if app.sqs_state.current_queue.is_some()
        && app.sqs_state.detail_tab == SqsQueueDetailTab::Tagging
    {
        if idx > 0 && idx <= app.sqs_state.tag_column_ids.len() {
            if let Some(col) = app.sqs_state.tag_column_ids.get(idx - 1) {
                if let Some(pos) = app
                    .sqs_state
                    .tag_visible_column_ids
                    .iter()
                    .position(|c| c == col)
                {
                    app.sqs_state.tag_visible_column_ids.remove(pos);
                } else {
                    app.sqs_state.tag_visible_column_ids.push(col.clone());
                }
            }
        } else if idx == app.sqs_state.tag_column_ids.len() + 3 {
            app.sqs_state.tags.page_size = PageSize::Ten;
        } else if idx == app.sqs_state.tag_column_ids.len() + 4 {
            app.sqs_state.tags.page_size = PageSize::TwentyFive;
        } else if idx == app.sqs_state.tag_column_ids.len() + 5 {
            app.sqs_state.tags.page_size = PageSize::Fifty;
        } else if idx == app.sqs_state.tag_column_ids.len() + 6 {
            app.sqs_state.tags.page_size = PageSize::OneHundred;
        }
    } else {
        // Queue list view — idx maps directly to column index
        if let Some(col) = app.sqs_column_ids.get(idx) {
            if let Some(pos) = app.sqs_visible_column_ids.iter().position(|c| c == col) {
                app.sqs_visible_column_ids.remove(pos);
            } else {
                app.sqs_visible_column_ids.push(*col);
            }
        } else if idx == app.sqs_column_ids.len() + 2 {
            app.sqs_state.queues.page_size = PageSize::Ten;
        } else if idx == app.sqs_column_ids.len() + 3 {
            app.sqs_state.queues.page_size = PageSize::TwentyFive;
        } else if idx == app.sqs_column_ids.len() + 4 {
            app.sqs_state.queues.page_size = PageSize::Fifty;
        } else if idx == app.sqs_column_ids.len() + 5 {
            app.sqs_state.queues.page_size = PageSize::OneHundred;
        }
    }
}

// ── Breadcrumb / console URL ──────────────────────────────────────────────────

pub fn breadcrumb() -> Vec<String> {
    vec!["SQS".to_string(), "Queues".to_string()]
}

pub fn console_url(app: &App) -> String {
    use crate::sqs::{console_url_queue_detail, console_url_queues};
    if let Some(queue_url) = &app.sqs_state.current_queue {
        console_url_queue_detail(&app.config.region, queue_url)
    } else {
        console_url_queues(&app.config.region)
    }
}

pub fn yank(app: &App) {
    use crate::app::copy_to_clipboard;
    if let Some(queue_url) = &app.sqs_state.current_queue {
        copy_to_clipboard(queue_url);
    } else {
        let filtered = filtered_queues(&app.sqs_state.queues.items, &app.sqs_state.queues.filter);
        if let Some(q) = app.sqs_state.queues.get_selected(&filtered) {
            copy_to_clipboard(&q.url);
        }
    }
}
