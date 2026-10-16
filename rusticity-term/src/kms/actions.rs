use crate::app::{App, Service};
use crate::common::{CyclicEnum, InputFocus};
use crate::preferences::bindings;
use crate::ui::kms::{filtered_kms_keys, FILTER_CONTROLS};

// ── Navigation ────────────────────────────────────────────────────────────────

pub fn next_item(app: &mut App) {
    let filtered = filtered_kms_keys(app);
    if !filtered.is_empty() {
        app.kms_state.keys.next_item(filtered.len());
    }
}

pub fn prev_item(app: &mut App) {
    app.kms_state.keys.prev_item();
}

pub fn page_down_filter_input(app: &mut App) {
    if app.kms_state.input_focus == InputFocus::Filter {
        let filtered = filtered_kms_keys(app);
        app.kms_state.keys.page_down(filtered.len());
    } else {
        let page_size = app.kms_state.keys.page_size.value();
        let filtered_count = filtered_kms_keys(app).len();
        app.kms_state.input_focus.handle_page_down(
            &mut app.kms_state.keys.selected,
            &mut app.kms_state.keys.scroll_offset,
            page_size,
            filtered_count,
        );
    }
}

pub fn page_down_normal(app: &mut App) {
    let filtered = filtered_kms_keys(app);
    app.kms_state.keys.page_down(filtered.len());
}

pub fn page_up_filter_input(app: &mut App) {
    if app.kms_state.input_focus == InputFocus::Filter {
        app.kms_state.keys.page_up();
    } else {
        let page_size = app.kms_state.keys.page_size.value();
        app.kms_state.input_focus.handle_page_up(
            &mut app.kms_state.keys.selected,
            &mut app.kms_state.keys.scroll_offset,
            page_size,
        );
    }
}

pub fn page_up_normal(app: &mut App) {
    app.kms_state.keys.page_up();
}

pub fn expand_row(app: &mut App) {
    app.kms_state.keys.toggle_expand();
}

pub fn prev_pane(app: &mut App) {
    app.kms_state.keys.collapse();
}

pub fn collapse_row(app: &mut App) {
    app.kms_state.keys.collapse();
}

pub fn go_to_page(app: &mut App, page: usize) {
    let filtered_count = filtered_kms_keys(app).len();
    app.kms_state.keys.goto_page(page, filtered_count);
}

// ── Filter ────────────────────────────────────────────────────────────────────

pub fn get_active_filter_mut(app: &mut App) -> Option<&mut String> {
    Some(&mut app.kms_state.keys.filter)
}

pub fn apply_filter_reset(app: &mut App) {
    app.kms_state.keys.reset();
}

pub fn start_filter(app: &mut App) {
    app.kms_state.input_focus = InputFocus::Filter;
}

pub fn next_filter_focus(app: &mut App) {
    app.kms_state.input_focus = app.kms_state.input_focus.next(&FILTER_CONTROLS);
}

pub fn prev_filter_focus(app: &mut App) {
    app.kms_state.input_focus = app.kms_state.input_focus.prev(&FILTER_CONTROLS);
}

pub fn is_pagination_focused(app: &App) -> bool {
    app.kms_state.input_focus == InputFocus::Pagination
}

// ── Tab ───────────────────────────────────────────────────────────────────────

pub fn next_detail_tab(app: &mut App) {
    app.kms_state.tab = app.kms_state.tab.next();
    app.kms_state.keys.reset();
    // No reload needed — both tabs read from the same keys.items, filtered client-side
}

pub fn prev_detail_tab(app: &mut App) {
    app.kms_state.tab = app.kms_state.tab.prev();
    app.kms_state.keys.reset();
    // No reload needed — both tabs read from the same keys.items, filtered client-side
}

// ── Yank / console URL / breadcrumb ──────────────────────────────────────────

pub fn yank(app: &App) {
    if let Some(key) = &app.kms_state.detail.source {
        app.clipboard.request_text(&key.key_arn);
        return;
    }
    let filtered = filtered_kms_keys(app);
    if let Some(key) = app.kms_state.keys.get_selected(&filtered) {
        app.clipboard.request_text(&key.key_arn);
    }
}

pub fn breadcrumb(app: &App) -> Vec<String> {
    let mut parts = vec!["KMS".to_string(), "Managed Keys".to_string()];
    if let Some(key) = &app.kms_state.detail.source {
        parts.push(key.key_id.clone());
    }
    parts
}

pub fn console_url(app: &App) -> String {
    use crate::kms;
    use crate::ui::kms::Tab;
    if let Some(key) = &app.kms_state.detail.source {
        return kms::console_url_key(&app.config.region, &key.key_id);
    }
    match app.kms_state.tab {
        Tab::AwsManaged => kms::console_url_aws_managed_keys(&app.config.region),
        Tab::CustomerManaged => kms::console_url_keys(&app.config.region),
    }
}

// ── Column preferences ────────────────────────────────────────────────────────

pub fn toggle_column(app: &mut App) {
    bindings::activate_for(app, Service::KmsKeys);
}

pub fn next_preferences(app: &mut App) {
    bindings::move_section_for(app, Service::KmsKeys, true);
}

pub fn prev_preferences(app: &mut App) {
    bindings::move_section_for(app, Service::KmsKeys, false);
}

pub fn column_selector_max(app: &App) -> usize {
    bindings::model_for(app, Service::KmsKeys).max_index()
}

pub fn column_count(app: &App) -> usize {
    bindings::model_for(app, Service::KmsKeys).column_count()
}
