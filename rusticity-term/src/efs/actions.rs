use crate::app::App;
use crate::common::{ColumnId, InputFocus, PageSize};
use crate::ui::efs::{
    filtered_access_points, filtered_efs_file_systems, DetailTab, FILTER_CONTROLS,
};

/// Number of access-point columns visible in the preferences selector.
fn ap_col_count(app: &App) -> usize {
    app.efs_state.ap_column_ids.len()
}

/// Number of mount-target columns visible in the preferences selector.
fn mt_col_count(app: &App) -> usize {
    app.efs_state.mt_column_ids.len()
}

/// Toggle a column's visibility within `visible`, keeping at least one visible.
fn toggle_visible(visible: &mut Vec<ColumnId>, all: &[ColumnId], idx: usize) {
    if idx > 0 && idx <= all.len() {
        let col = all[idx - 1];
        if let Some(pos) = visible.iter().position(|c| *c == col) {
            if visible.len() > 1 {
                visible.remove(pos);
            }
        } else {
            visible.push(col);
        }
    }
}

/// Apply a page-size selection when `idx` lands in the page-size section that
/// follows `n` columns (indices `n+3 ..= n+6`). Returns to the first page.
fn set_page_size_by_idx<T>(table: &mut crate::table::TableState<T>, idx: usize, n: usize) {
    let ps = if idx == n + 3 {
        PageSize::Ten
    } else if idx == n + 4 {
        PageSize::TwentyFive
    } else if idx == n + 5 {
        PageSize::Fifty
    } else if idx == n + 6 {
        PageSize::OneHundred
    } else {
        return;
    };
    table.set_page_size(ps);
}

// ── Navigation ────────────────────────────────────────────────────────────────

pub fn next_item(app: &mut App) {
    if app.efs_state.current_file_system.is_some() {
        match app.efs_state.detail_tab {
            DetailTab::Tags => {
                let tag_count = current_fs_tag_count(app);
                if tag_count > 0 {
                    app.efs_state.tags_table.next_item(tag_count);
                }
            }
            DetailTab::AccessPoints => {
                let n = filtered_access_points(app).len();
                if n > 0 {
                    app.efs_state.access_points.next_item(n);
                }
            }
            DetailTab::Network => {
                let n = app.efs_state.mount_targets.items.len();
                if n > 0 {
                    app.efs_state.mount_targets.next_item(n);
                }
            }
            _ => {}
        }
    } else {
        let filtered = filtered_efs_file_systems(app);
        if !filtered.is_empty() {
            app.efs_state.file_systems.next_item(filtered.len());
        }
    }
}

fn current_fs_tag_count(app: &App) -> usize {
    app.efs_state
        .current_file_system
        .as_deref()
        .and_then(|id| {
            app.efs_state
                .file_systems
                .items
                .iter()
                .find(|f| f.file_system_id == id)
        })
        .map(|fs| fs.tags.len())
        .unwrap_or(0)
}

pub fn prev_item(app: &mut App) {
    if app.efs_state.current_file_system.is_some() {
        match app.efs_state.detail_tab {
            DetailTab::Tags => app.efs_state.tags_table.prev_item(),
            DetailTab::AccessPoints => app.efs_state.access_points.prev_item(),
            DetailTab::Network => app.efs_state.mount_targets.prev_item(),
            _ => {}
        }
    } else {
        app.efs_state.file_systems.prev_item();
    }
}

pub fn page_down_filter_input(app: &mut App) {
    if app.efs_state.current_file_system.is_some()
        && app.efs_state.detail_tab == DetailTab::AccessPoints
    {
        let n = filtered_access_points(app).len();
        app.efs_state.access_points.page_down(n);
        return;
    }
    if app.efs_state.input_focus == InputFocus::Filter {
        let filtered = filtered_efs_file_systems(app);
        app.efs_state.file_systems.page_down(filtered.len());
    } else {
        let page_size = app.efs_state.file_systems.page_size.value();
        let filtered_count = filtered_efs_file_systems(app).len();
        app.efs_state.input_focus.handle_page_down(
            &mut app.efs_state.file_systems.selected,
            &mut app.efs_state.file_systems.scroll_offset,
            page_size,
            filtered_count,
        );
    }
}

pub fn page_down_normal(app: &mut App) {
    if app.efs_state.current_file_system.is_some() {
        match app.efs_state.detail_tab {
            DetailTab::AccessPoints => {
                let n = filtered_access_points(app).len();
                app.efs_state.access_points.page_down(n);
            }
            DetailTab::Network => {
                let n = app.efs_state.mount_targets.items.len();
                app.efs_state.mount_targets.page_down(n);
            }
            _ => {}
        }
        return;
    }
    let filtered = filtered_efs_file_systems(app);
    app.efs_state.file_systems.page_down(filtered.len());
}

pub fn page_up_filter_input(app: &mut App) {
    if app.efs_state.current_file_system.is_some()
        && app.efs_state.detail_tab == DetailTab::AccessPoints
    {
        app.efs_state.access_points.page_up();
        return;
    }
    if app.efs_state.input_focus == InputFocus::Filter {
        app.efs_state.file_systems.page_up();
    } else {
        let page_size = app.efs_state.file_systems.page_size.value();
        app.efs_state.input_focus.handle_page_up(
            &mut app.efs_state.file_systems.selected,
            &mut app.efs_state.file_systems.scroll_offset,
            page_size,
        );
    }
}

pub fn page_up_normal(app: &mut App) {
    if app.efs_state.current_file_system.is_some() {
        match app.efs_state.detail_tab {
            DetailTab::AccessPoints => app.efs_state.access_points.page_up(),
            DetailTab::Network => app.efs_state.mount_targets.page_up(),
            _ => {}
        }
        return;
    }
    app.efs_state.file_systems.page_up();
}

pub fn expand_row(app: &mut App) {
    if app.efs_state.current_file_system.is_some() {
        match app.efs_state.detail_tab {
            DetailTab::Tags => app.efs_state.tags_table.toggle_expand(),
            DetailTab::AccessPoints => app.efs_state.access_points.toggle_expand(),
            DetailTab::Network => app.efs_state.mount_targets.toggle_expand(),
            _ => {}
        }
    } else {
        app.efs_state.file_systems.toggle_expand();
    }
}

pub fn prev_pane(app: &mut App) {
    collapse_row(app);
}

pub fn collapse_row(app: &mut App) {
    if app.efs_state.current_file_system.is_some() {
        match app.efs_state.detail_tab {
            DetailTab::Tags => app.efs_state.tags_table.collapse(),
            DetailTab::AccessPoints => app.efs_state.access_points.collapse(),
            DetailTab::Network => app.efs_state.mount_targets.collapse(),
            _ => {}
        }
    } else {
        app.efs_state.file_systems.collapse();
    }
}

pub fn go_to_page(app: &mut App, page: usize) {
    if app.efs_state.current_file_system.is_some()
        && app.efs_state.detail_tab == DetailTab::AccessPoints
    {
        let n = filtered_access_points(app).len();
        app.efs_state.access_points.goto_page(page, n);
        return;
    }
    let filtered_count = filtered_efs_file_systems(app).len();
    app.efs_state.file_systems.goto_page(page, filtered_count);
}

// ── Filter ────────────────────────────────────────────────────────────────────

pub fn get_active_filter_mut(app: &mut App) -> Option<&mut String> {
    if app.efs_state.current_file_system.is_some()
        && app.efs_state.detail_tab == DetailTab::AccessPoints
    {
        return Some(&mut app.efs_state.access_points.filter);
    }
    Some(&mut app.efs_state.file_systems.filter)
}

pub fn apply_filter_reset(app: &mut App) {
    if app.efs_state.current_file_system.is_some()
        && app.efs_state.detail_tab == DetailTab::AccessPoints
    {
        app.efs_state.access_points.reset();
        return;
    }
    app.efs_state.file_systems.reset();
}

pub fn start_filter(app: &mut App) {
    app.efs_state.input_focus = InputFocus::Filter;
}

pub fn next_filter_focus(app: &mut App) {
    app.efs_state.input_focus = app.efs_state.input_focus.next(&FILTER_CONTROLS);
}

pub fn prev_filter_focus(app: &mut App) {
    app.efs_state.input_focus = app.efs_state.input_focus.prev(&FILTER_CONTROLS);
}

pub fn is_pagination_focused(app: &App) -> bool {
    app.efs_state.input_focus == InputFocus::Pagination
}

// ── Yank / console URL / breadcrumb ──────────────────────────────────────────

pub fn yank(app: &App) {
    use crate::app::copy_to_clipboard;
    if let Some(fs_id) = &app.efs_state.current_file_system {
        // On the File system policy tab, copy the policy JSON.
        if app.efs_state.detail_tab == crate::ui::efs::DetailTab::FileSystemPolicy
            && !app.efs_state.policy_document.is_empty()
        {
            copy_to_clipboard(&app.efs_state.policy_document);
            return;
        }
        // Otherwise in detail view — copy the ARN
        if let Some(fs) = app
            .efs_state
            .file_systems
            .items
            .iter()
            .find(|f| &f.file_system_id == fs_id)
        {
            copy_to_clipboard(&fs.file_system_arn);
        }
    } else {
        // In list view — copy the file system ID
        let filtered = filtered_efs_file_systems(app);
        if let Some(fs) = app.efs_state.file_systems.get_selected(&filtered) {
            copy_to_clipboard(&fs.file_system_id);
        }
    }
}

// ── Select / go back ──────────────────────────────────────────────────────────

pub fn select_item(app: &mut App) {
    if app.efs_state.current_file_system.is_none() {
        let filtered = filtered_efs_file_systems(app);
        if let Some(fs) = app.efs_state.file_systems.get_selected(&filtered) {
            app.efs_state.current_file_system = Some(fs.file_system_id.clone());
            app.efs_state.detail_tab = crate::ui::efs::DetailTab::MeteredSize;
        }
    }
}

pub fn go_back(app: &mut App) {
    app.efs_state.current_file_system = None;
    app.efs_state.detail_tab = crate::ui::efs::DetailTab::MeteredSize;
}

pub fn next_detail_tab(app: &mut App) {
    use crate::common::CyclicEnum;
    app.efs_state.detail_tab = app.efs_state.detail_tab.next();
    trigger_tab_load(app);
}

pub fn prev_detail_tab(app: &mut App) {
    use crate::common::CyclicEnum;
    app.efs_state.detail_tab = app.efs_state.detail_tab.prev();
    trigger_tab_load(app);
}

/// When switching to the Monitoring tab, mark metrics as loading so the event
/// loop fetches fresh CloudWatch data and reset the chart scroll. When switching
/// to the File system policy tab, mark the policy as loading.
fn trigger_tab_load(app: &mut App) {
    use crate::ui::monitoring::MonitoringState;
    match app.efs_state.detail_tab {
        crate::ui::efs::DetailTab::Monitoring => {
            app.efs_state.set_metrics_loading(true);
            app.efs_state.set_monitoring_scroll(0);
            app.efs_state.clear_metrics();
        }
        crate::ui::efs::DetailTab::FileSystemPolicy => {
            app.efs_state.policy_loading = true;
            app.efs_state.policy_scroll = 0;
        }
        crate::ui::efs::DetailTab::AccessPoints => {
            app.efs_state.ap_loading = true;
            app.efs_state.access_points.reset();
        }
        crate::ui::efs::DetailTab::Network => {
            app.efs_state.mt_loading = true;
            app.efs_state.mount_targets.reset();
        }
        crate::ui::efs::DetailTab::Replication => {
            app.efs_state.replication_loading = true;
        }
        _ => {}
    }
}

/// Number of charts in the EFS Monitoring tab; the last scrollable index is
/// `EFS_MONITORING_CHARTS - 1`.
const EFS_MONITORING_CHARTS: usize = 6;

/// Scroll the active detail tab down (Ctrl+D / ScrollDown).
pub fn scroll_down_detail(app: &mut App) {
    use crate::ui::monitoring::MonitoringState;
    match app.efs_state.detail_tab {
        crate::ui::efs::DetailTab::Monitoring => {
            let max = EFS_MONITORING_CHARTS.saturating_sub(1);
            app.efs_state
                .set_monitoring_scroll((app.efs_state.monitoring_scroll() + 1).min(max));
        }
        crate::ui::efs::DetailTab::FileSystemPolicy => {
            let max = app
                .efs_state
                .policy_document
                .lines()
                .count()
                .saturating_sub(1);
            app.efs_state.policy_scroll = (app.efs_state.policy_scroll + 1).min(max);
        }
        _ => {}
    }
}

/// Scroll the active detail tab up (Ctrl+U / ScrollUp).
pub fn scroll_up_detail(app: &mut App) {
    use crate::ui::monitoring::MonitoringState;
    match app.efs_state.detail_tab {
        crate::ui::efs::DetailTab::Monitoring => {
            app.efs_state
                .set_monitoring_scroll(app.efs_state.monitoring_scroll().saturating_sub(1));
        }
        crate::ui::efs::DetailTab::FileSystemPolicy => {
            app.efs_state.policy_scroll = app.efs_state.policy_scroll.saturating_sub(1);
        }
        _ => {}
    }
}

pub fn breadcrumb(app: &App) -> Vec<String> {
    let mut parts = vec!["EFS".to_string()];
    if let Some(fs_id) = &app.efs_state.current_file_system {
        parts.push(fs_id.clone());
    } else {
        parts.push("File Systems".to_string());
    }
    parts
}

pub fn console_url(app: &App) -> String {
    use crate::efs;
    // In detail view, link to the current file system's active tab.
    if let Some(fs_id) = &app.efs_state.current_file_system {
        return efs::console_url_file_system_with_tab(
            &app.config.region,
            fs_id,
            app.efs_state.detail_tab,
        );
    }
    let filtered = filtered_efs_file_systems(app);
    if let Some(fs) = app.efs_state.file_systems.get_selected(&filtered) {
        efs::console_url_file_system(&app.config.region, &fs.file_system_id)
    } else {
        efs::console_url_file_systems(&app.config.region)
    }
}

// ── Column preferences ────────────────────────────────────────────────────────

pub fn toggle_column(app: &mut App) {
    let idx = app.column_selector_index;
    if app.efs_state.current_file_system.is_some() {
        match app.efs_state.detail_tab {
            DetailTab::Tags => {
                // Key/Value columns are always visible — only page size configurable.
                set_page_size_by_idx(&mut app.efs_state.tags_table, idx, 2);
                return;
            }
            DetailTab::AccessPoints => {
                let all = app.efs_state.ap_column_ids.clone();
                toggle_visible(&mut app.efs_state.ap_visible_column_ids, &all, idx);
                set_page_size_by_idx(&mut app.efs_state.access_points, idx, all.len());
                return;
            }
            DetailTab::Network => {
                let all = app.efs_state.mt_column_ids.clone();
                toggle_visible(&mut app.efs_state.mt_visible_column_ids, &all, idx);
                set_page_size_by_idx(&mut app.efs_state.mount_targets, idx, all.len());
                return;
            }
            _ => return,
        }
    }
    // File systems list.
    if idx > 0 && idx <= app.efs_column_ids.len() {
        if let Some(col) = app.efs_column_ids.get(idx - 1) {
            if let Some(pos) = app.efs_visible_column_ids.iter().position(|c| c == col) {
                if app.efs_visible_column_ids.len() > 1 {
                    app.efs_visible_column_ids.remove(pos);
                }
            } else {
                app.efs_visible_column_ids.push(*col);
            }
        }
    } else {
        set_page_size_by_idx(
            &mut app.efs_state.file_systems,
            idx,
            app.efs_column_ids.len(),
        );
    }
}

/// Number of toggleable columns for the active view (used by the prefs cursor).
fn active_prefs_col_count(app: &App) -> usize {
    if app.efs_state.current_file_system.is_some() {
        match app.efs_state.detail_tab {
            DetailTab::Tags => 2,
            DetailTab::AccessPoints => ap_col_count(app),
            DetailTab::Network => mt_col_count(app),
            _ => app.efs_column_ids.len(),
        }
    } else {
        app.efs_column_ids.len()
    }
}

pub fn next_preferences(app: &mut App) {
    let page_size_idx = active_prefs_col_count(app) + 2;
    if app.column_selector_index < page_size_idx {
        app.column_selector_index = page_size_idx;
    } else {
        app.column_selector_index = 0;
    }
}

pub fn prev_preferences(app: &mut App) {
    let page_size_idx = active_prefs_col_count(app) + 2;
    if app.column_selector_index >= page_size_idx {
        app.column_selector_index = 0;
    } else {
        app.column_selector_index = page_size_idx;
    }
}

pub fn column_selector_max(app: &App) -> usize {
    active_prefs_col_count(app) + 6
}

pub fn column_count(app: &App) -> usize {
    active_prefs_col_count(app)
}
