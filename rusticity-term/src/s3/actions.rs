use crate::app::App;
use crate::common::{CyclicEnum, InputFocus, PageSize};
use crate::ui::s3::{calculate_filtered_bucket_rows, BucketType as S3BucketType};

const FILTER_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

// ── Filter ────────────────────────────────────────────────────────────────────

pub fn apply_filter_reset(app: &mut App) {
    if app.s3_state.current_bucket.is_some() {
        app.s3_state.selected_object = 0;
    } else {
        app.s3_state.buckets.reset();
        app.s3_state.selected_row = 0;
        app.s3_state.bucket_scroll_offset = 0;
    }
}

pub fn reset_on_service_select(app: &mut App) {
    app.s3_state.selected_row = 0;
    app.s3_state.selected_object = 0;
}

pub fn start_filter(app: &mut App) {
    app.s3_state.input_focus = InputFocus::Filter;
}

pub fn next_filter_focus(app: &mut App) {
    app.s3_state.input_focus = app.s3_state.input_focus.next(&FILTER_CONTROLS);
}

pub fn prev_filter_focus(app: &mut App) {
    app.s3_state.input_focus = app.s3_state.input_focus.prev(&FILTER_CONTROLS);
}

pub fn is_pagination_focused(app: &App) -> bool {
    app.s3_state.input_focus == InputFocus::Pagination
}

// ── Navigation — pagination ───────────────────────────────────────────────────

pub fn page_down_filter_input(app: &mut App) {
    if app.s3_state.input_focus == InputFocus::Pagination {
        let page_size = app.s3_state.buckets.page_size.value();
        let total_rows = calculate_filtered_bucket_rows(app);
        let last_page = if total_rows > page_size {
            ((total_rows - 1) / page_size) * page_size
        } else {
            0
        };
        app.s3_state.selected_row = (app.s3_state.selected_row + page_size).min(last_page);
        app.s3_state.bucket_scroll_offset = app.s3_state.selected_row;
    }
}

pub fn page_up_filter_input(app: &mut App) {
    if app.s3_state.input_focus == InputFocus::Pagination {
        let page_size = app.s3_state.buckets.page_size.value();
        app.s3_state.selected_row = app.s3_state.selected_row.saturating_sub(page_size);
        app.s3_state.bucket_scroll_offset = app.s3_state.selected_row;
    }
}

// ── Actions ───────────────────────────────────────────────────────────────────

pub fn go_back(app: &mut App) {
    if !app.s3_state.prefix_stack.is_empty() {
        app.s3_state.prefix_stack.pop();
        app.s3_state.buckets.loading = true;
    } else {
        app.s3_state.current_bucket = None;
        app.s3_state.objects.clear();
    }
}

pub fn next_detail_tab(app: &mut App) {
    if app.s3_state.current_bucket.is_some() {
        app.s3_state.object_tab = app.s3_state.object_tab.next();
    } else {
        app.s3_state.bucket_type = match app.s3_state.bucket_type {
            S3BucketType::GeneralPurpose => S3BucketType::Directory,
            S3BucketType::Directory => S3BucketType::GeneralPurpose,
        };
        app.s3_state.buckets.reset();
    }
}

pub fn prev_detail_tab(app: &mut App) {
    if app.s3_state.current_bucket.is_some() {
        app.s3_state.object_tab = app.s3_state.object_tab.prev();
    }
}

// ── Column preferences ────────────────────────────────────────────────────────

pub fn toggle_column(app: &mut App) {
    let idx = app.column_selector_index;
    if idx > 0 && idx <= app.s3_bucket_column_ids.len() {
        if let Some(col) = app.s3_bucket_column_ids.get(idx - 1) {
            App::toggle_column_visibility(
                &mut app.s3_bucket_visible_column_ids,
                &app.s3_bucket_column_ids,
                *col,
            );
        }
    } else if idx == app.s3_bucket_column_ids.len() + 3 {
        app.s3_state.buckets.page_size = PageSize::Ten;
    } else if idx == app.s3_bucket_column_ids.len() + 4 {
        app.s3_state.buckets.page_size = PageSize::TwentyFive;
    } else if idx == app.s3_bucket_column_ids.len() + 5 {
        app.s3_state.buckets.page_size = PageSize::Fifty;
    } else if idx == app.s3_bucket_column_ids.len() + 6 {
        app.s3_state.buckets.page_size = PageSize::OneHundred;
    }
}

pub fn next_preferences(app: &mut App) {
    let page_size_idx = app.s3_bucket_column_ids.len() + 2;
    if app.column_selector_index < page_size_idx {
        app.column_selector_index = page_size_idx;
    } else {
        app.column_selector_index = 0;
    }
}

pub fn prev_preferences(app: &mut App) {
    let page_size_idx = app.s3_bucket_column_ids.len() + 2;
    if app.column_selector_index >= page_size_idx {
        app.column_selector_index = 0;
    } else {
        app.column_selector_index = page_size_idx;
    }
}

pub fn column_selector_max(app: &App) -> usize {
    app.s3_bucket_column_ids.len() + 6
}

pub fn column_count(app: &App) -> usize {
    app.s3_bucket_column_ids.len()
}

// ── Breadcrumb / console URL ──────────────────────────────────────────────────

pub fn breadcrumb(app: &App) -> Vec<String> {
    let mut parts = vec!["S3".to_string()];
    if let Some(bucket) = &app.s3_state.current_bucket {
        parts.push(bucket.clone());
        if let Some(prefix) = app.s3_state.prefix_stack.last() {
            parts.push(prefix.trim_end_matches('/').to_string());
        }
    } else {
        parts.push("Buckets".to_string());
    }
    parts
}

pub fn console_url(app: &App) -> String {
    use crate::s3;
    if let Some(bucket_name) = &app.s3_state.current_bucket {
        let prefix = app.s3_state.prefix_stack.join("");
        s3::console_url_bucket(&app.config.region, bucket_name, &prefix)
    } else {
        s3::console_url_buckets(&app.config.region)
    }
}
