use crate::app::{App, ViewMode};
use crate::common::InputFocus;
use crate::ui::cw::alarms::AlarmTab;
use crate::ui::cw::alarms::FILTER_CONTROLS;

// ── CloudWatch Alarms ─────────────────────────────────────────────────────────

pub fn alarms_get_filtered_count(app: &App) -> usize {
    match app.alarms_state.alarm_tab {
        AlarmTab::AllAlarms => app.alarms_state.table.items.len(),
        AlarmTab::InAlarm => app
            .alarms_state
            .table
            .items
            .iter()
            .filter(|a| a.state.to_uppercase() == "ALARM")
            .count(),
    }
}

pub fn alarms_apply_filter_reset(app: &mut App) {
    app.alarms_state.table.reset();
}

pub fn alarms_is_pagination_focused(app: &App) -> bool {
    app.alarms_state.input_focus == InputFocus::Pagination
}

pub fn alarms_next_filter_focus(app: &mut App) {
    app.alarms_state.input_focus = app.alarms_state.input_focus.next(&FILTER_CONTROLS);
}

pub fn alarms_prev_filter_focus(app: &mut App) {
    app.alarms_state.input_focus = app.alarms_state.input_focus.prev(&FILTER_CONTROLS);
}

pub fn alarms_start_filter(app: &mut App) {
    // Alarms only needs mode switch — input_focus defaults to Filter
    let _ = app; // caller sets mode
}

pub fn alarms_next_detail_tab(app: &mut App) {
    if app.alarms_state.current_alarm.is_some() {
        // In alarm detail view — cycle detail tabs
        use crate::common::CyclicEnum;
        app.alarms_state.detail_tab = app.alarms_state.detail_tab.next();
    } else {
        // In alarm list view — cycle alarm list tabs (AllAlarms / InAlarm)
        use crate::app::AlarmTab;
        app.alarms_state.alarm_tab = match app.alarms_state.alarm_tab {
            AlarmTab::AllAlarms => AlarmTab::InAlarm,
            AlarmTab::InAlarm => AlarmTab::AllAlarms,
        };
        app.alarms_state.table.reset();
    }
}

pub fn alarms_prev_detail_tab(app: &mut App) {
    if app.alarms_state.current_alarm.is_some() {
        use crate::common::CyclicEnum;
        app.alarms_state.detail_tab = app.alarms_state.detail_tab.prev();
    } else {
        use crate::app::AlarmTab;
        app.alarms_state.alarm_tab = match app.alarms_state.alarm_tab {
            AlarmTab::AllAlarms => AlarmTab::InAlarm,
            AlarmTab::InAlarm => AlarmTab::AllAlarms,
        };
    }
}

pub fn alarms_next_item(app: &mut App) {
    let filtered = alarms_get_filtered_count(app);
    if filtered > 0 {
        app.alarms_state.table.next_item(filtered);
    }
}

pub fn alarms_prev_item(app: &mut App) {
    app.alarms_state.table.prev_item();
}

pub fn alarms_page_down_filter_input(app: &mut App) {
    let page_size = app.alarms_state.table.page_size.value();
    let filtered_count = app.alarms_state.table.items.len();
    app.alarms_state.input_focus.handle_page_down(
        &mut app.alarms_state.table.selected,
        &mut app.alarms_state.table.scroll_offset,
        page_size,
        filtered_count,
    );
}

pub fn alarms_page_down_normal(app: &mut App) {
    let filtered = alarms_get_filtered_count(app);
    if filtered > 0 {
        app.alarms_state.table.page_down(filtered);
    }
}

pub fn alarms_page_up_filter_input(app: &mut App) {
    let page_size = app.alarms_state.table.page_size.value();
    app.alarms_state.input_focus.handle_page_up(
        &mut app.alarms_state.table.selected,
        &mut app.alarms_state.table.scroll_offset,
        page_size,
    );
}

pub fn alarms_page_up_normal(app: &mut App) {
    app.alarms_state.table.page_up();
}

pub fn alarms_go_to_page(app: &mut App, page: usize) {
    let page_size = app.alarms_state.table.page_size.value();
    let target = (page - 1) * page_size;
    let filtered_count = alarms_get_filtered_count(app);
    // last page start = (total_pages - 1) * page_size
    let total_pages = filtered_count.div_ceil(page_size).max(1);
    let max_offset = (total_pages - 1) * page_size;
    app.alarms_state.table.scroll_offset = target.min(max_offset);
    app.alarms_state.table.selected = app
        .alarms_state
        .table
        .scroll_offset
        .min(filtered_count.saturating_sub(1));
}

/// Right arrow: expand alarm row (only if not already expanded).
pub fn alarms_expand_row(app: &mut App) {
    if !app.alarms_state.table.is_expanded() {
        app.alarms_state.table.toggle_expand();
    }
}

/// Enter in list view: toggle expand.
pub fn alarms_select_expand(app: &mut App) {
    app.alarms_state.table.toggle_expand();
}

/// Enter in list view when no alarm is open: open alarm detail.
pub fn alarms_select_item(app: &mut App) {
    if app.alarms_state.current_alarm.is_none() {
        let filtered_alarms: Vec<_> = app.alarms_state.table.items.iter().collect();
        if let Some(alarm) = app.alarms_state.table.get_selected(&filtered_alarms) {
            app.alarms_state.current_alarm = Some(alarm.name.clone());
            app.alarms_state.metrics_loading = true;
            app.alarms_state.detail_tab = crate::app::AlarmDetailTab::Details;
            app.view_mode = ViewMode::Detail;
            app.update_current_tab_breadcrumb();
        }
    }
}

/// Left arrow: collapse expanded row.
pub fn alarms_prev_pane(app: &mut App) {
    app.alarms_state.table.collapse();
}

/// Left arrow (collapse_row): same as prev_pane.
pub fn alarms_collapse_row(app: &mut App) {
    app.alarms_state.table.collapse();
}

/// GoBack: leave alarm detail, return to list.
pub fn alarms_go_back(app: &mut App) {
    app.alarms_state.current_alarm = None;
    app.alarms_state.metric_data.clear();
    app.view_mode = ViewMode::List;
    app.update_current_tab_breadcrumb();
}

/// GoBack when row is expanded (no alarm open): collapse.
pub fn alarms_go_back_list(app: &mut App) {
    if app.alarms_state.table.has_expanded_item() {
        app.alarms_state.table.collapse();
    }
}

/// Refresh: mark metrics as loading to trigger a reload.
pub fn alarms_refresh(app: &mut App) {
    app.alarms_state.metrics_loading = true;
}

pub fn alarms_column_selector_max(_app: &App) -> usize {
    29
}

pub fn alarms_column_count(_app: &App) -> usize {
    14
}

pub fn alarms_next_preferences(app: &mut App) {
    if app.column_selector_index < 18 {
        app.column_selector_index = 18;
    } else if app.column_selector_index < 22 {
        app.column_selector_index = 22;
    } else if app.column_selector_index < 28 {
        app.column_selector_index = 28;
    } else {
        app.column_selector_index = 0;
    }
}

pub fn alarms_prev_preferences(app: &mut App) {
    if app.column_selector_index >= 28 {
        app.column_selector_index = 22;
    } else if app.column_selector_index >= 22 {
        app.column_selector_index = 18;
    } else if app.column_selector_index >= 18 {
        app.column_selector_index = 0;
    } else {
        app.column_selector_index = 28;
    }
}

pub fn alarms_breadcrumb() -> Vec<String> {
    vec!["CloudWatch".to_string(), "Alarms".to_string()]
}

// ── CloudWatch Log Groups ─────────────────────────────────────────────────────

pub fn logs_is_pagination_focused(app: &App) -> bool {
    app.log_groups_state.input_focus == InputFocus::Pagination
}

pub fn logs_apply_filter_reset(app: &mut App) {
    app.log_groups_state.log_groups.reset();
}

pub fn logs_next_filter_focus(app: &mut App) {
    use crate::app::ViewMode;
    use crate::ui::cw::logs::{FILTER_CONTROLS, LIST_FILTER_CONTROLS};
    if app.view_mode == ViewMode::List {
        app.log_groups_state.input_focus =
            app.log_groups_state.input_focus.next(&LIST_FILTER_CONTROLS);
    } else {
        app.log_groups_state.input_focus = app.log_groups_state.input_focus.next(&FILTER_CONTROLS);
    }
}

pub fn logs_next_event_filter_focus(app: &mut App) {
    app.log_groups_state.event_input_focus = app.log_groups_state.event_input_focus.next();
}

pub fn logs_prev_filter_focus(app: &mut App) {
    use crate::app::ViewMode;
    use crate::ui::cw::logs::{FILTER_CONTROLS, LIST_FILTER_CONTROLS};
    if app.view_mode == ViewMode::List {
        app.log_groups_state.input_focus =
            app.log_groups_state.input_focus.prev(&LIST_FILTER_CONTROLS);
    } else {
        app.log_groups_state.input_focus = app.log_groups_state.input_focus.prev(&FILTER_CONTROLS);
    }
}

pub fn logs_prev_event_filter_focus(app: &mut App) {
    app.log_groups_state.event_input_focus = app.log_groups_state.event_input_focus.prev();
}

pub fn logs_next_item(app: &mut App) {
    use crate::app::filtered_log_groups;
    use crate::app::filtered_log_streams;
    if app.view_mode == ViewMode::List {
        let filtered = filtered_log_groups(app);
        app.log_groups_state.log_groups.next_item(filtered.len());
    } else if app.view_mode == ViewMode::Detail {
        let filtered = filtered_log_streams(app);
        if !filtered.is_empty() {
            let max = filtered.len() - 1;
            if app.log_groups_state.selected_stream < max {
                app.log_groups_state.selected_stream =
                    (app.log_groups_state.selected_stream + 1).min(max);
                app.log_groups_state.expanded_stream = None;
            }
        }
    }
    // ViewMode::Events handled in the caller (event_scroll_offset)
}

pub fn logs_prev_item(app: &mut App) {
    use crate::app::filtered_log_streams;
    if app.view_mode == ViewMode::List {
        app.log_groups_state.log_groups.prev_item();
    } else if app.view_mode == ViewMode::Detail && app.log_groups_state.selected_stream > 0 {
        app.log_groups_state.selected_stream =
            app.log_groups_state.selected_stream.saturating_sub(1);
        app.log_groups_state.expanded_stream = None;
    }
    let _ = filtered_log_streams(app); // suppress unused warning
}

pub fn logs_page_down_filter_input(app: &mut App) {
    use crate::app::{filtered_log_groups, filtered_log_streams};
    if app.view_mode == ViewMode::List {
        let filtered = filtered_log_groups(app);
        let page_size = app.log_groups_state.log_groups.page_size.value();
        let filtered_count = filtered.len();
        app.log_groups_state.input_focus.handle_page_down(
            &mut app.log_groups_state.log_groups.selected,
            &mut app.log_groups_state.log_groups.scroll_offset,
            page_size,
            filtered_count,
        );
    } else {
        let filtered = filtered_log_streams(app);
        let page_size = app.log_groups_state.stream_page_size;
        let filtered_count = filtered.len();
        if app.log_groups_state.input_focus == InputFocus::Pagination && filtered_count > 0 {
            let total_pages = filtered_count.div_ceil(page_size).max(1);
            let current_page = app.log_groups_state.stream_current_page;
            if current_page + 1 < total_pages {
                app.log_groups_state.stream_current_page = current_page + 1;
                app.log_groups_state.selected_stream = (current_page + 1) * page_size;
            }
        }
        app.log_groups_state.expanded_stream = None;
    }
}

pub fn logs_page_down_normal_list(app: &mut App) {
    use crate::app::filtered_log_groups;
    let filtered = filtered_log_groups(app);
    app.log_groups_state.log_groups.page_down(filtered.len());
}

pub fn logs_page_down_normal_detail(app: &mut App) {
    use crate::app::filtered_log_streams;
    use crate::app::nav_page_down;
    let len = filtered_log_streams(app).len();
    nav_page_down(&mut app.log_groups_state.selected_stream, len, 10);
}

pub fn logs_page_down_events(app: &mut App) {
    use crate::app::nav_page_down;
    let max = app.log_groups_state.log_events.len();
    nav_page_down(&mut app.log_groups_state.event_scroll_offset, max, 10);
}

pub fn logs_page_up_filter_input(app: &mut App) {
    use crate::app::filtered_log_streams;
    if app.view_mode == ViewMode::List {
        let page_size = app.log_groups_state.log_groups.page_size.value();
        app.log_groups_state.input_focus.handle_page_up(
            &mut app.log_groups_state.log_groups.selected,
            &mut app.log_groups_state.log_groups.scroll_offset,
            page_size,
        );
    } else {
        let page_size = app.log_groups_state.stream_page_size;
        if app.log_groups_state.input_focus == InputFocus::Pagination {
            let current_page = app.log_groups_state.stream_current_page;
            if current_page > 0 {
                app.log_groups_state.stream_current_page = current_page - 1;
                app.log_groups_state.selected_stream = (current_page - 1) * page_size;
            }
        }
        app.log_groups_state.expanded_stream = None;
    }
    let _ = filtered_log_streams(app);
}

pub fn logs_page_up_normal_list(app: &mut App) {
    app.log_groups_state.log_groups.page_up();
}

pub fn logs_page_up_normal_detail(app: &mut App) {
    app.log_groups_state.selected_stream = app.log_groups_state.selected_stream.saturating_sub(10);
}

pub fn logs_page_up_events(app: &mut App) {
    if app.log_groups_state.event_scroll_offset < 10 && app.log_groups_state.has_older_events {
        app.log_groups_state.loading = true;
    }
    app.log_groups_state.event_scroll_offset =
        app.log_groups_state.event_scroll_offset.saturating_sub(10);
}

/// Right arrow: expand the selected item in current view mode.
pub fn logs_expand_row(app: &mut App) {
    if app.view_mode == ViewMode::List {
        if app.log_groups_state.log_groups.expanded_item
            != Some(app.log_groups_state.log_groups.selected)
        {
            app.log_groups_state.log_groups.expanded_item =
                Some(app.log_groups_state.log_groups.selected);
        }
    } else if app.view_mode == ViewMode::Detail
        && app.log_groups_state.expanded_stream != Some(app.log_groups_state.selected_stream)
    {
        app.log_groups_state.expanded_stream = Some(app.log_groups_state.selected_stream);
    } else if app.view_mode == ViewMode::Events
        && app.log_groups_state.expanded_event != Some(app.log_groups_state.event_scroll_offset)
    {
        app.log_groups_state.expanded_event = Some(app.log_groups_state.event_scroll_offset);
    }
}

/// Left arrow: collapse the selected item in current view mode.
pub fn logs_prev_pane(app: &mut App) {
    if app.view_mode == ViewMode::List && app.log_groups_state.log_groups.has_expanded_item() {
        app.log_groups_state.log_groups.collapse();
    } else if app.view_mode == ViewMode::Detail && app.log_groups_state.expanded_stream.is_some() {
        app.log_groups_state.expanded_stream = None;
    } else if app.view_mode == ViewMode::Events && app.log_groups_state.expanded_event.is_some() {
        app.log_groups_state.expanded_event = None;
    }
}

/// Left arrow (collapse_row match arm).
pub fn logs_collapse_row(app: &mut App) {
    if app.view_mode == ViewMode::Events {
        if let Some(idx) = app.log_groups_state.expanded_event {
            app.log_groups_state.expanded_event = None;
            app.log_groups_state.selected_event = idx;
        }
    } else if app.view_mode == ViewMode::Detail {
        if let Some(idx) = app.log_groups_state.expanded_stream {
            app.log_groups_state.expanded_stream = None;
            app.log_groups_state.selected_stream = idx;
        }
    } else {
        app.log_groups_state.log_groups.collapse();
    }
}

pub fn logs_go_to_page(app: &mut App, page: usize) {
    match app.view_mode {
        ViewMode::Events => {
            let page_size = 20usize;
            let target = (page - 1) * page_size;
            let max = app.log_groups_state.log_events.len().saturating_sub(1);
            app.log_groups_state.event_scroll_offset = target.min(max);
        }
        ViewMode::Detail => {
            let page_size = app.log_groups_state.stream_page_size;
            app.log_groups_state.stream_current_page = (page - 1).min(
                app.log_groups_state
                    .log_streams
                    .len()
                    .div_ceil(page_size)
                    .saturating_sub(1),
            );
            app.log_groups_state.selected_stream = 0;
        }
        ViewMode::List => {
            let total = app.log_groups_state.log_groups.items.len();
            app.log_groups_state.log_groups.goto_page(page, total);
        }
        _ => {}
    }
}

/// Enter: select log group, stream, or toggle event expansion.
pub fn logs_select_item(app: &mut App) {
    use crate::app::{filtered_log_groups, filtered_log_streams};
    if app.view_mode == ViewMode::List {
        let filtered = filtered_log_groups(app);
        if let Some(selected_group) = filtered.get(app.log_groups_state.log_groups.selected) {
            if let Some(actual_idx) = app
                .log_groups_state
                .log_groups
                .items
                .iter()
                .position(|g| g.name == selected_group.name)
            {
                app.log_groups_state.log_groups.selected = actual_idx;
            }
        }
        app.view_mode = ViewMode::Detail;
        app.log_groups_state.log_streams.clear();
        app.log_groups_state.tags.items.clear();
        app.log_groups_state.tags.reset();
        app.log_groups_state.selected_stream = 0;
        app.log_groups_state.loading = true;
        app.column_selector_index = 0;
        app.update_current_tab_breadcrumb();
    } else if app.view_mode == ViewMode::Detail {
        let filtered = filtered_log_streams(app);
        if let Some(selected_stream) = filtered.get(app.log_groups_state.selected_stream) {
            if let Some(actual_idx) = app
                .log_groups_state
                .log_streams
                .iter()
                .position(|s| s.name == selected_stream.name)
            {
                app.log_groups_state.selected_stream = actual_idx;
            }
        }
        app.view_mode = ViewMode::Events;
        app.update_current_tab_breadcrumb();
        app.log_groups_state.log_events.clear();
        app.log_groups_state.event_scroll_offset = 0;
        app.log_groups_state.next_backward_token = None;
        app.log_groups_state.loading = true;
    } else if app.view_mode == ViewMode::Events {
        if app.log_groups_state.expanded_event == Some(app.log_groups_state.event_scroll_offset) {
            app.log_groups_state.expanded_event = None;
        } else {
            app.log_groups_state.expanded_event = Some(app.log_groups_state.event_scroll_offset);
        }
    }
}

// ── CloudWatch Insights ───────────────────────────────────────────────────────

pub fn insights_next_item_dropdown(app: &mut App) {
    use crate::app::InsightsFocus;
    if app.insights_state.insights.insights_focus == InsightsFocus::LogGroupSearch
        && app.insights_state.insights.show_dropdown
        && !app.insights_state.insights.log_group_matches.is_empty()
    {
        let max = app.insights_state.insights.log_group_matches.len() - 1;
        app.insights_state.insights.dropdown_selected =
            (app.insights_state.insights.dropdown_selected + 1).min(max);
    }
}

pub fn insights_next_item_results(app: &mut App) {
    let max = app
        .insights_state
        .insights
        .query_results
        .len()
        .saturating_sub(1);
    if app.insights_state.insights.results_selected < max {
        app.insights_state.insights.results_selected += 1;
    }
}

pub fn insights_prev_item_dropdown(app: &mut App) {
    use crate::app::InsightsFocus;
    if app.insights_state.insights.insights_focus == InsightsFocus::LogGroupSearch
        && app.insights_state.insights.show_dropdown
        && !app.insights_state.insights.log_group_matches.is_empty()
    {
        app.insights_state.insights.dropdown_selected = app
            .insights_state
            .insights
            .dropdown_selected
            .saturating_sub(1);
    }
}

pub fn insights_prev_item_results(app: &mut App) {
    if app.insights_state.insights.results_selected > 0 {
        app.insights_state.insights.results_selected -= 1;
    }
}

pub fn insights_page_down(app: &mut App) {
    use crate::app::nav_page_down;
    let max = app.insights_state.insights.query_results.len();
    nav_page_down(&mut app.insights_state.insights.results_selected, max, 10);
}

pub fn insights_page_up(app: &mut App) {
    app.insights_state.insights.results_selected = app
        .insights_state
        .insights
        .results_selected
        .saturating_sub(10);
}

pub fn insights_expand_row(app: &mut App) {
    use crate::app::InsightsFocus;
    app.insights_state.insights.results_horizontal_scroll = app
        .insights_state
        .insights
        .results_horizontal_scroll
        .saturating_add(1); // caller checks max
    let _ = InsightsFocus::Query; // suppress unused import
}

pub fn insights_prev_pane(app: &mut App) {
    app.insights_state.insights.results_horizontal_scroll = app
        .insights_state
        .insights
        .results_horizontal_scroll
        .saturating_sub(1);
}

pub fn insights_refresh(app: &mut App) {
    app.log_groups_state.loading = true;
    app.insights_state.insights.query_completed = true;
}

pub fn insights_go_back_results(app: &mut App) {
    if app.insights_state.insights.expanded_result.is_some() {
        app.insights_state.insights.expanded_result = None;
    }
}

pub fn insights_breadcrumb() -> Vec<String> {
    vec!["CloudWatch".to_string(), "Logs Insights".to_string()]
}
