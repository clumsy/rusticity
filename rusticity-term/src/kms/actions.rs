use crate::app::App;
use crate::common::{CyclicEnum, InputFocus, PageSize};
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
    use crate::app::copy_to_clipboard;
    let filtered = filtered_kms_keys(app);
    if let Some(key) = app.kms_state.keys.get_selected(&filtered) {
        copy_to_clipboard(&key.key_arn);
    }
}

pub fn breadcrumb(_app: &App) -> Vec<String> {
    // Keep stable — the tab switcher inside the view shows the sub-tab name.
    // Using the sub-tab name here would cause the session tab title to
    // change on every tab switch, making it disappear/reappear.
    vec!["KMS".to_string(), "Managed Keys".to_string()]
}

pub fn console_url(app: &App) -> String {
    use crate::kms;
    use crate::ui::kms::Tab;
    let filtered = filtered_kms_keys(app);
    if let Some(key) = app.kms_state.keys.get_selected(&filtered) {
        kms::console_url_key(&app.config.region, &key.key_id)
    } else {
        match app.kms_state.tab {
            Tab::AwsManaged => kms::console_url_aws_managed_keys(&app.config.region),
            Tab::CustomerManaged => kms::console_url_keys(&app.config.region),
        }
    }
}

// ── Column preferences ────────────────────────────────────────────────────────

pub fn toggle_column(app: &mut App) {
    let idx = app.column_selector_index;
    let tab_cols = app.kms_state.tab_column_ids();
    // idx 0 = Alias (locked, always visible — skip)
    // idx 1..N = columns (1-based into tab's column list, col at tab_cols[idx-1])
    // idx N+2 = PageSize section header
    // idx N+3..N+6 = Ten, TwentyFive, Fifty, OneHundred
    if idx > 0 && idx <= tab_cols.len() {
        if let Some(col_id) = tab_cols.get(idx - 1) {
            // Alias is always visible — don't allow toggling it off
            if *col_id == crate::kms::key::Column::Alias.id() {
                return;
            }
            let visible = app.kms_state.visible_column_ids_mut();
            if let Some(pos) = visible.iter().position(|c| c == col_id) {
                if visible.len() > 1 {
                    visible.remove(pos);
                }
            } else {
                visible.push(col_id);
            }
        }
    } else {
        let n = tab_cols.len();
        if idx == n + 3 {
            app.kms_state.keys.page_size = PageSize::Ten;
        } else if idx == n + 4 {
            app.kms_state.keys.page_size = PageSize::TwentyFive;
        } else if idx == n + 5 {
            app.kms_state.keys.page_size = PageSize::Fifty;
        } else if idx == n + 6 {
            app.kms_state.keys.page_size = PageSize::OneHundred;
        }
    }
}

pub fn next_preferences(app: &mut App) {
    let n = app.kms_state.tab_column_ids().len();
    let page_size_idx = n + 2;
    if app.column_selector_index < page_size_idx {
        app.column_selector_index = page_size_idx;
    } else {
        app.column_selector_index = 0;
    }
}

pub fn prev_preferences(app: &mut App) {
    let n = app.kms_state.tab_column_ids().len();
    let page_size_idx = n + 2;
    if app.column_selector_index >= page_size_idx {
        app.column_selector_index = 0;
    } else {
        app.column_selector_index = page_size_idx;
    }
}

pub fn column_selector_max(app: &App) -> usize {
    app.kms_state.tab_column_ids().len() + 6
}

pub fn column_count(app: &App) -> usize {
    app.kms_state.tab_column_ids().len()
}
