use crate::app::{App, CloudTrailDetailFocus};
use crate::common::{CyclicEnum, InputFocus};

pub fn apply_filter_reset(app: &mut App) {
    app.cloudtrail_state.table.reset();
}

pub fn is_pagination_focused(app: &App) -> bool {
    app.cloudtrail_state.input_focus == InputFocus::Pagination
}

pub fn next_filter_focus(app: &mut App) {
    const FILTER_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];
    app.cloudtrail_state.input_focus = app.cloudtrail_state.input_focus.next(&FILTER_CONTROLS);
}

pub fn prev_filter_focus(app: &mut App) {
    const FILTER_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];
    app.cloudtrail_state.input_focus = app.cloudtrail_state.input_focus.prev(&FILTER_CONTROLS);
}

pub fn next_detail_tab(app: &mut App) {
    app.cloudtrail_state.detail_focus = app.cloudtrail_state.detail_focus.next();
}

pub fn prev_detail_tab(app: &mut App) {
    app.cloudtrail_state.detail_focus = app.cloudtrail_state.detail_focus.prev();
}

pub fn next_item(app: &mut App) {
    if app.cloudtrail_state.current_event.is_some()
        && app.cloudtrail_state.detail_focus == CloudTrailDetailFocus::EventRecord
    {
        if let Some(event) = &app.cloudtrail_state.current_event {
            let line_count = event.cloud_trail_event_json.lines().count();
            let max_scroll = line_count.saturating_sub(1);
            app.cloudtrail_state.event_json_scroll =
                (app.cloudtrail_state.event_json_scroll + 1).min(max_scroll);
        }
    } else {
        // Paginated within current page
        let page_size = app.cloudtrail_state.table.page_size.value();
        let current_page = app.cloudtrail_state.table.selected / page_size;
        let page_end = (current_page + 1) * page_size;
        let filtered_count = app.cloudtrail_state.table.items.len();
        if app.cloudtrail_state.table.selected < filtered_count.saturating_sub(1) {
            app.cloudtrail_state.table.selected = (app.cloudtrail_state.table.selected + 1)
                .min(page_end - 1)
                .min(filtered_count - 1);
        }
    }
}

pub fn prev_item(app: &mut App) {
    if app.cloudtrail_state.current_event.is_some()
        && app.cloudtrail_state.detail_focus == CloudTrailDetailFocus::EventRecord
    {
        app.cloudtrail_state.event_json_scroll =
            app.cloudtrail_state.event_json_scroll.saturating_sub(1);
    } else {
        let page_size = app.cloudtrail_state.table.page_size.value();
        let current_page = app.cloudtrail_state.table.selected / page_size;
        let page_start = current_page * page_size;
        if app.cloudtrail_state.table.selected > page_start {
            app.cloudtrail_state.table.selected -= 1;
        }
    }
}

pub fn page_down_filter_input(app: &mut App) {
    let page_size = app.cloudtrail_state.table.page_size.value();
    let filtered_count = app.cloudtrail_state.table.items.len();
    let current_page = app.cloudtrail_state.table.selected / page_size;
    let total_pages = filtered_count.div_ceil(page_size);
    if current_page + 1 < total_pages {
        app.cloudtrail_state.table.selected = (current_page + 1) * page_size;
        app.cloudtrail_state.table.scroll_offset = app.cloudtrail_state.table.selected;
    }
}

pub fn page_down_fast(app: &mut App) {
    if app.cloudtrail_state.current_event.is_some() {
        if let Some(event) = &app.cloudtrail_state.current_event {
            let lines = event.cloud_trail_event_json.lines().count();
            let max_scroll = lines.saturating_sub(1);
            app.cloudtrail_state.event_json_scroll =
                (app.cloudtrail_state.event_json_scroll + 10).min(max_scroll);
        }
    }
}

pub fn page_down_normal(app: &mut App) {
    let filtered_count = app.cloudtrail_state.table.items.len();
    if filtered_count > 0 {
        app.cloudtrail_state.table.page_down(filtered_count);
    }
}

pub fn page_up_filter_input(app: &mut App) {
    let page_size = app.cloudtrail_state.table.page_size.value();
    let current_page = app.cloudtrail_state.table.selected / page_size;
    if current_page > 0 {
        app.cloudtrail_state.table.selected = (current_page - 1) * page_size;
        app.cloudtrail_state.table.scroll_offset = app.cloudtrail_state.table.selected;
    }
}

pub fn page_up_fast(app: &mut App) {
    if app.cloudtrail_state.current_event.is_some() {
        app.cloudtrail_state.event_json_scroll =
            app.cloudtrail_state.event_json_scroll.saturating_sub(10);
    }
}

pub fn page_up_normal(app: &mut App) {
    app.cloudtrail_state.table.page_up();
}

pub fn select_item(app: &mut App) {
    if app.cloudtrail_state.current_event.is_none() {
        let filtered: Vec<_> = app.cloudtrail_state.table.items.iter().collect();
        if let Some(event) = app.cloudtrail_state.table.get_selected(&filtered) {
            app.cloudtrail_state.current_event = Some((*event).clone());
            app.cloudtrail_state.event_json_scroll = 0;
            app.update_current_tab_breadcrumb();
        }
    }
}

pub fn go_back(app: &mut App) {
    app.cloudtrail_state.current_event = None;
    app.update_current_tab_breadcrumb();
}

pub fn column_selector_max(app: &App) -> usize {
    if app.cloudtrail_state.current_event.is_some()
        && app.cloudtrail_state.detail_focus == CloudTrailDetailFocus::Resources
    {
        app.cloudtrail_resource_column_ids.len()
    } else {
        app.cloudtrail_event_column_ids.len() + 6
    }
}

pub fn column_count(app: &App) -> usize {
    if app.cloudtrail_state.current_event.is_some()
        && app.cloudtrail_state.detail_focus == CloudTrailDetailFocus::Resources
    {
        app.cloudtrail_resource_column_ids.len()
    } else {
        app.cloudtrail_event_column_ids.len()
    }
}

pub fn yank(app: &App) {
    use crate::app::copy_to_clipboard;
    if let Some(event) = &app.cloudtrail_state.current_event {
        copy_to_clipboard(&event.cloud_trail_event_json);
    }
}

pub fn breadcrumb(app: &App) -> Vec<String> {
    let mut parts = vec!["CloudTrail".to_string(), "Event History".to_string()];
    if let Some(event) = &app.cloudtrail_state.current_event {
        parts.push(event.event_name.clone());
    }
    parts
}

pub fn console_url(app: &App) -> String {
    if let Some(event) = &app.cloudtrail_state.current_event {
        format!(
            "https://{}.console.aws.amazon.com/cloudtrailv2/home?region={}#/events/{}",
            app.config.region, app.config.region, event.event_id
        )
    } else {
        format!(
            "https://{}.console.aws.amazon.com/cloudtrail/home?region={}#/events",
            app.config.region, app.config.region
        )
    }
}
