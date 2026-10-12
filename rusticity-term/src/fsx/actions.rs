use crate::app::{App, Service};
use crate::common::{CyclicEnum, InputFocus};
use crate::preferences::bindings;
use crate::ui::fsx::{filtered_fsx_file_systems, DetailTab, FILTER_CONTROLS};

/// Tags on the currently-open file system (Tags detail tab).
fn current_fs_tag_count(app: &App) -> usize {
    current_fs(app).map(|fs| fs.tags.len()).unwrap_or(0)
}

fn current_fs_update_count(app: &App) -> usize {
    current_fs(app).map(|fs| fs.updates.len()).unwrap_or(0)
}

fn current_fs(app: &App) -> Option<&rusticity_core::fsx::FsxFileSystem> {
    app.fsx_state.current_file_system.as_deref().and_then(|id| {
        app.fsx_state
            .file_systems
            .items
            .iter()
            .find(|f| f.file_system_id == id)
    })
}

// ── Navigation ────────────────────────────────────────────────────────────────

pub fn next_item(app: &mut App) {
    if app.fsx_state.current_file_system.is_some() {
        match app.fsx_state.detail_tab {
            DetailTab::Tags => {
                let n = current_fs_tag_count(app);
                if n > 0 {
                    app.fsx_state.tags_table.next_item(n);
                }
            }
            DetailTab::Updates => {
                let n = current_fs_update_count(app);
                if n > 0 {
                    app.fsx_state.updates_table.next_item(n);
                }
            }
            DetailTab::Backups => {
                let n = app.fsx_state.backups.items.len();
                if n > 0 {
                    app.fsx_state.backups.next_item(n);
                }
            }
            _ => {}
        }
        return;
    }
    let filtered = filtered_fsx_file_systems(app);
    if !filtered.is_empty() {
        app.fsx_state.file_systems.next_item(filtered.len());
    }
}

pub fn prev_item(app: &mut App) {
    if app.fsx_state.current_file_system.is_some() {
        match app.fsx_state.detail_tab {
            DetailTab::Tags => app.fsx_state.tags_table.prev_item(),
            DetailTab::Updates => app.fsx_state.updates_table.prev_item(),
            DetailTab::Backups => app.fsx_state.backups.prev_item(),
            _ => {}
        }
        return;
    }
    app.fsx_state.file_systems.prev_item();
}

pub fn page_down_filter_input(app: &mut App) {
    if app.fsx_state.input_focus == InputFocus::Filter {
        let filtered = filtered_fsx_file_systems(app);
        app.fsx_state.file_systems.page_down(filtered.len());
    } else {
        let page_size = app.fsx_state.file_systems.page_size.value();
        let filtered_count = filtered_fsx_file_systems(app).len();
        app.fsx_state.input_focus.handle_page_down(
            &mut app.fsx_state.file_systems.selected,
            &mut app.fsx_state.file_systems.scroll_offset,
            page_size,
            filtered_count,
        );
    }
}

pub fn page_down_normal(app: &mut App) {
    let filtered = filtered_fsx_file_systems(app);
    app.fsx_state.file_systems.page_down(filtered.len());
}

pub fn page_up_filter_input(app: &mut App) {
    if app.fsx_state.input_focus == InputFocus::Filter {
        app.fsx_state.file_systems.page_up();
    } else {
        let page_size = app.fsx_state.file_systems.page_size.value();
        app.fsx_state.input_focus.handle_page_up(
            &mut app.fsx_state.file_systems.selected,
            &mut app.fsx_state.file_systems.scroll_offset,
            page_size,
        );
    }
}

pub fn page_up_normal(app: &mut App) {
    app.fsx_state.file_systems.page_up();
}

pub fn expand_row(app: &mut App) {
    if app.fsx_state.current_file_system.is_some() {
        match app.fsx_state.detail_tab {
            DetailTab::Tags => app.fsx_state.tags_table.toggle_expand(),
            DetailTab::Updates => app.fsx_state.updates_table.toggle_expand(),
            DetailTab::Backups => app.fsx_state.backups.toggle_expand(),
            _ => {}
        }
    } else {
        app.fsx_state.file_systems.toggle_expand();
    }
}

pub fn prev_pane(app: &mut App) {
    collapse_row(app);
}

pub fn collapse_row(app: &mut App) {
    if app.fsx_state.current_file_system.is_some() {
        match app.fsx_state.detail_tab {
            DetailTab::Tags => app.fsx_state.tags_table.collapse(),
            DetailTab::Updates => app.fsx_state.updates_table.collapse(),
            DetailTab::Backups => app.fsx_state.backups.collapse(),
            _ => {}
        }
    } else {
        app.fsx_state.file_systems.collapse();
    }
}

pub fn go_to_page(app: &mut App, page: usize) {
    let filtered_count = filtered_fsx_file_systems(app).len();
    app.fsx_state.file_systems.goto_page(page, filtered_count);
}

// ── Filter ────────────────────────────────────────────────────────────────────

pub fn get_active_filter_mut(app: &mut App) -> Option<&mut String> {
    Some(&mut app.fsx_state.file_systems.filter)
}

pub fn apply_filter_reset(app: &mut App) {
    app.fsx_state.file_systems.reset();
}

pub fn start_filter(app: &mut App) {
    app.fsx_state.input_focus = InputFocus::Filter;
}

pub fn next_filter_focus(app: &mut App) {
    app.fsx_state.input_focus = app.fsx_state.input_focus.next(&FILTER_CONTROLS);
}

pub fn prev_filter_focus(app: &mut App) {
    app.fsx_state.input_focus = app.fsx_state.input_focus.prev(&FILTER_CONTROLS);
}

pub fn is_pagination_focused(app: &App) -> bool {
    app.fsx_state.input_focus == InputFocus::Pagination
}

// ── Yank / console URL / breadcrumb ──────────────────────────────────────────

pub fn yank(app: &App) {
    use crate::app::copy_to_clipboard;
    if let Some(fs_id) = &app.fsx_state.current_file_system {
        // Detail view — copy the ARN (fall back to the ID).
        if let Some(fs) = app
            .fsx_state
            .file_systems
            .items
            .iter()
            .find(|f| &f.file_system_id == fs_id)
        {
            if fs.file_system_arn.is_empty() {
                copy_to_clipboard(&fs.file_system_id);
            } else {
                copy_to_clipboard(&fs.file_system_arn);
            }
        }
    } else {
        let filtered = filtered_fsx_file_systems(app);
        if let Some(fs) = app.fsx_state.file_systems.get_selected(&filtered) {
            copy_to_clipboard(&fs.file_system_id);
        }
    }
}

pub fn breadcrumb(app: &App) -> Vec<String> {
    let mut parts = vec!["FSx".to_string()];
    if let Some(fs_id) = &app.fsx_state.current_file_system {
        parts.push(fs_id.clone());
    } else {
        parts.push("File Systems".to_string());
    }
    parts
}

pub fn console_url(app: &App) -> String {
    use crate::fsx;
    if let Some(fs_id) = &app.fsx_state.current_file_system {
        return fsx::console_url_file_system(&app.config.region, fs_id);
    }
    let filtered = filtered_fsx_file_systems(app);
    if let Some(fs) = app.fsx_state.file_systems.get_selected(&filtered) {
        fsx::console_url_file_system(&app.config.region, &fs.file_system_id)
    } else {
        fsx::console_url_file_systems(&app.config.region)
    }
}

// ── Select / go back / detail tabs ───────────────────────────────────────────

pub fn select_item(app: &mut App) {
    if app.fsx_state.current_file_system.is_none() {
        let filtered = filtered_fsx_file_systems(app);
        if let Some(fs) = app.fsx_state.file_systems.get_selected(&filtered) {
            app.fsx_state.current_file_system = Some(fs.file_system_id.clone());
            app.fsx_state.detail_tab = DetailTab::NetworkSecurity;
            reset_detail_tables(app);
        }
    }
}

pub fn go_back(app: &mut App) {
    app.fsx_state.current_file_system = None;
    app.fsx_state.detail_tab = DetailTab::NetworkSecurity;
}

pub fn next_detail_tab(app: &mut App) {
    app.fsx_state.detail_tab = app.fsx_state.detail_tab.next();
    reset_detail_tables(app);
    maybe_load_backups(app);
}

pub fn prev_detail_tab(app: &mut App) {
    app.fsx_state.detail_tab = app.fsx_state.detail_tab.prev();
    reset_detail_tables(app);
    maybe_load_backups(app);
}

fn reset_detail_tables(app: &mut App) {
    app.fsx_state.tags_table.reset();
    app.fsx_state.updates_table.reset();
    app.fsx_state.backups.reset();
}

/// Mark backups as needing a load when the Backups tab becomes active.
fn maybe_load_backups(app: &mut App) {
    if app.fsx_state.detail_tab == DetailTab::Backups {
        app.fsx_state.backups.items.clear();
        app.fsx_state.backups_loading = true;
    }
}

// ── Column preferences ────────────────────────────────────────────────────────

pub fn toggle_column(app: &mut App) {
    bindings::activate_for(app, Service::FsxFileSystems);
}

pub fn next_preferences(app: &mut App) {
    bindings::move_section_for(app, Service::FsxFileSystems, true);
}

pub fn prev_preferences(app: &mut App) {
    bindings::move_section_for(app, Service::FsxFileSystems, false);
}

pub fn column_selector_max(app: &App) -> usize {
    bindings::model_for(app, Service::FsxFileSystems).max_index()
}

pub fn column_count(app: &App) -> usize {
    bindings::model_for(app, Service::FsxFileSystems).column_count()
}
