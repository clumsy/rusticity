use crate::app::App;
use crate::common::{CyclicEnum, InputFocus, PageSize};
use crate::keymap::Mode;
use crate::ui::lambda::{filtered_lambda_functions, DetailTab as LambdaDetailTab, FILTER_CONTROLS};
use crate::ui::monitoring::MonitoringState;

// ── Navigation ────────────────────────────────────────────────────────────────

pub fn next_item(app: &mut App) {
    if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Code
    {
        if let Some(func_name) = &app.lambda_state.current_function.clone() {
            if let Some(func) = app
                .lambda_state
                .table
                .items
                .iter()
                .find(|f| f.name == *func_name)
            {
                let max = func.layers.len().saturating_sub(1);
                if !func.layers.is_empty() {
                    app.lambda_state.layer_selected =
                        (app.lambda_state.layer_selected + 1).min(max);
                }
            }
        }
    } else if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Versions
    {
        let filtered: Vec<_> = app
            .lambda_state
            .version_table
            .items
            .iter()
            .filter(|v| {
                app.lambda_state.version_table.filter.is_empty()
                    || v.version
                        .to_lowercase()
                        .contains(&app.lambda_state.version_table.filter.to_lowercase())
                    || v.aliases
                        .to_lowercase()
                        .contains(&app.lambda_state.version_table.filter.to_lowercase())
                    || v.description
                        .to_lowercase()
                        .contains(&app.lambda_state.version_table.filter.to_lowercase())
            })
            .collect();
        if !filtered.is_empty() {
            app.lambda_state.version_table.selected =
                (app.lambda_state.version_table.selected + 1).min(filtered.len() - 1);
            app.lambda_state.version_table.snap_to_page();
        }
    } else if app.lambda_state.current_function.is_some()
        && (app.lambda_state.detail_tab == LambdaDetailTab::Aliases
            || (app.lambda_state.current_version.is_some()
                && app.lambda_state.detail_tab == LambdaDetailTab::Configuration))
    {
        let version_filter = app.lambda_state.current_version.clone();
        let filtered: Vec<_> = app
            .lambda_state
            .alias_table
            .items
            .iter()
            .filter(|a| {
                (version_filter.is_none() || a.versions.contains(version_filter.as_ref().unwrap()))
                    && (app.lambda_state.alias_table.filter.is_empty()
                        || a.name
                            .to_lowercase()
                            .contains(&app.lambda_state.alias_table.filter.to_lowercase())
                        || a.versions
                            .to_lowercase()
                            .contains(&app.lambda_state.alias_table.filter.to_lowercase())
                        || a.description
                            .to_lowercase()
                            .contains(&app.lambda_state.alias_table.filter.to_lowercase()))
            })
            .collect();
        if !filtered.is_empty() {
            app.lambda_state.alias_table.selected =
                (app.lambda_state.alias_table.selected + 1).min(filtered.len() - 1);
            app.lambda_state.alias_table.snap_to_page();
        }
    } else if app.lambda_state.current_function.is_none() {
        let filtered = filtered_lambda_functions(app);
        if !filtered.is_empty() {
            app.lambda_state.table.next_item(filtered.len());
            app.lambda_state.table.snap_to_page();
        }
    }
}

pub fn prev_item(app: &mut App) {
    if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Code
    {
        app.lambda_state.layer_selected = app.lambda_state.layer_selected.saturating_sub(1);
    } else if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Versions
    {
        app.lambda_state.version_table.prev_item();
    } else if app.lambda_state.current_function.is_some()
        && (app.lambda_state.detail_tab == LambdaDetailTab::Aliases
            || (app.lambda_state.current_version.is_some()
                && app.lambda_state.detail_tab == LambdaDetailTab::Configuration))
    {
        app.lambda_state.alias_table.prev_item();
    } else if app.lambda_state.current_function.is_none() {
        app.lambda_state.table.prev_item();
    }
}

pub fn page_down_filter_input(app: &mut App) {
    if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Versions
        && app.lambda_state.version_input_focus == InputFocus::Pagination
    {
        let page_size = app.lambda_state.version_table.page_size.value();
        let filtered_count: usize = app
            .lambda_state
            .version_table
            .items
            .iter()
            .filter(|v| {
                app.lambda_state.version_table.filter.is_empty()
                    || v.version
                        .to_lowercase()
                        .contains(&app.lambda_state.version_table.filter.to_lowercase())
                    || v.aliases
                        .to_lowercase()
                        .contains(&app.lambda_state.version_table.filter.to_lowercase())
                    || v.description
                        .to_lowercase()
                        .contains(&app.lambda_state.version_table.filter.to_lowercase())
            })
            .count();
        let target = app.lambda_state.version_table.selected + page_size;
        app.lambda_state.version_table.selected = target.min(filtered_count.saturating_sub(1));
    } else if app.lambda_state.current_function.is_some()
        && (app.lambda_state.detail_tab == LambdaDetailTab::Aliases
            || (app.lambda_state.current_version.is_some()
                && app.lambda_state.detail_tab == LambdaDetailTab::Configuration))
        && app.lambda_state.alias_input_focus == InputFocus::Pagination
    {
        let page_size = app.lambda_state.alias_table.page_size.value();
        let version_filter = app.lambda_state.current_version.clone();
        let filtered_count = app
            .lambda_state
            .alias_table
            .items
            .iter()
            .filter(|a| {
                (version_filter.is_none() || a.versions.contains(version_filter.as_ref().unwrap()))
                    && (app.lambda_state.alias_table.filter.is_empty()
                        || a.name
                            .to_lowercase()
                            .contains(&app.lambda_state.alias_table.filter.to_lowercase())
                        || a.versions
                            .to_lowercase()
                            .contains(&app.lambda_state.alias_table.filter.to_lowercase())
                        || a.description
                            .to_lowercase()
                            .contains(&app.lambda_state.alias_table.filter.to_lowercase()))
            })
            .count();
        let target = app.lambda_state.alias_table.selected + page_size;
        app.lambda_state.alias_table.selected = target.min(filtered_count.saturating_sub(1));
    } else if app.lambda_state.current_function.is_none() {
        let page_size = app.lambda_state.table.page_size.value();
        let filtered_count = filtered_lambda_functions(app).len();
        app.lambda_state.input_focus.handle_page_down(
            &mut app.lambda_state.table.selected,
            &mut app.lambda_state.table.scroll_offset,
            page_size,
            filtered_count,
        );
    }
}

pub fn page_down_normal(app: &mut App) {
    let len = filtered_lambda_functions(app).len();
    app.lambda_state.table.page_down(len);
}

pub fn page_up_filter_input(app: &mut App) {
    if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Versions
        && app.lambda_state.version_input_focus == InputFocus::Pagination
    {
        let page_size = app.lambda_state.version_table.page_size.value();
        app.lambda_state.version_table.selected = app
            .lambda_state
            .version_table
            .selected
            .saturating_sub(page_size);
    } else if app.lambda_state.current_function.is_some()
        && (app.lambda_state.detail_tab == LambdaDetailTab::Aliases
            || (app.lambda_state.current_version.is_some()
                && app.lambda_state.detail_tab == LambdaDetailTab::Configuration))
        && app.lambda_state.alias_input_focus == InputFocus::Pagination
    {
        let page_size = app.lambda_state.alias_table.page_size.value();
        app.lambda_state.alias_table.selected = app
            .lambda_state
            .alias_table
            .selected
            .saturating_sub(page_size);
    } else if app.lambda_state.current_function.is_none() {
        let page_size = app.lambda_state.table.page_size.value();
        app.lambda_state.input_focus.handle_page_up(
            &mut app.lambda_state.table.selected,
            &mut app.lambda_state.table.scroll_offset,
            page_size,
        );
    }
}

pub fn page_up_normal(app: &mut App) {
    app.lambda_state.table.page_up();
}

pub fn scroll_up(app: &mut App) {
    if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Monitor
        && !app.lambda_state.is_metrics_loading()
    {
        app.lambda_state
            .set_monitoring_scroll(app.lambda_state.monitoring_scroll().saturating_sub(1));
    }
}

pub fn scroll_down(app: &mut App) {
    if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Monitor
        && !app.lambda_state.is_metrics_loading()
    {
        app.lambda_state
            .set_monitoring_scroll((app.lambda_state.monitoring_scroll() + 1).min(9));
    }
}

pub fn expand_row(app: &mut App) {
    if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Code
    {
        if app.lambda_state.layer_expanded != Some(app.lambda_state.layer_selected) {
            app.lambda_state.layer_expanded = Some(app.lambda_state.layer_selected);
        } else {
            app.lambda_state.layer_expanded = None;
        }
    } else if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Versions
    {
        app.lambda_state.version_table.toggle_expand();
    } else if app.lambda_state.current_function.is_some()
        && (app.lambda_state.detail_tab == LambdaDetailTab::Aliases
            || (app.lambda_state.current_version.is_some()
                && app.lambda_state.detail_tab == LambdaDetailTab::Configuration))
    {
        app.lambda_state.alias_table.toggle_expand();
    } else if app.lambda_state.current_function.is_none() {
        app.lambda_state.table.expand();
    }
}

pub fn collapse_row(app: &mut App) {
    app.lambda_state.table.collapse();
}

pub fn prev_pane(app: &mut App) {
    if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Code
    {
        app.lambda_state.layer_expanded = None;
    } else if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Versions
    {
        app.lambda_state.version_table.collapse();
    } else if app.lambda_state.current_function.is_some()
        && (app.lambda_state.detail_tab == LambdaDetailTab::Aliases
            || (app.lambda_state.current_version.is_some()
                && app.lambda_state.detail_tab == LambdaDetailTab::Configuration))
    {
        app.lambda_state.alias_table.collapse();
    } else if app.lambda_state.current_function.is_none() {
        app.lambda_state.table.collapse();
    }
}

pub fn go_to_page(app: &mut App, page: usize) {
    if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Versions
    {
        let filtered_count = app
            .lambda_state
            .version_table
            .filtered(|v| {
                app.lambda_state.version_table.filter.is_empty()
                    || v.version
                        .to_lowercase()
                        .contains(&app.lambda_state.version_table.filter.to_lowercase())
                    || v.aliases
                        .to_lowercase()
                        .contains(&app.lambda_state.version_table.filter.to_lowercase())
                    || v.description
                        .to_lowercase()
                        .contains(&app.lambda_state.version_table.filter.to_lowercase())
            })
            .len();
        app.lambda_state
            .version_table
            .goto_page(page, filtered_count);
    } else {
        let filtered_count = filtered_lambda_functions(app).len();
        app.lambda_state.table.goto_page(page, filtered_count);
    }
}

// ── Filter ────────────────────────────────────────────────────────────────────

pub fn apply_filter_reset(app: &mut App) {
    if app.lambda_state.current_version.is_some() || app.lambda_state.current_function.is_some() {
        app.lambda_state.version_table.reset();
        app.lambda_state.alias_table.reset();
    } else {
        app.lambda_state.table.reset();
    }
}

pub fn start_filter(app: &mut App) {
    if app.lambda_state.current_version.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Configuration
    {
        app.lambda_state.alias_input_focus = InputFocus::Filter;
    } else if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Versions
    {
        app.lambda_state.version_input_focus = InputFocus::Filter;
    } else if app.lambda_state.current_function.is_none() {
        app.lambda_state.input_focus = InputFocus::Filter;
    }
}

pub fn is_pagination_focused(app: &App) -> bool {
    if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Versions
    {
        app.lambda_state.version_input_focus == InputFocus::Pagination
    } else if app.lambda_state.current_function.is_none() {
        app.lambda_state.input_focus == InputFocus::Pagination
    } else {
        false
    }
}

pub fn next_filter_focus(app: &mut App) {
    if app.lambda_state.current_version.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Configuration
    {
        app.lambda_state.alias_input_focus =
            app.lambda_state.alias_input_focus.next(&FILTER_CONTROLS);
    } else if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Versions
    {
        app.lambda_state.version_input_focus =
            app.lambda_state.version_input_focus.next(&FILTER_CONTROLS);
    } else if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Aliases
    {
        app.lambda_state.alias_input_focus =
            app.lambda_state.alias_input_focus.next(&FILTER_CONTROLS);
    } else if app.lambda_state.current_function.is_none() {
        app.lambda_state.input_focus = app.lambda_state.input_focus.next(&FILTER_CONTROLS);
    }
}

pub fn prev_filter_focus(app: &mut App) {
    if app.lambda_state.current_version.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Configuration
    {
        app.lambda_state.alias_input_focus =
            app.lambda_state.alias_input_focus.prev(&FILTER_CONTROLS);
    } else if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Versions
    {
        app.lambda_state.version_input_focus =
            app.lambda_state.version_input_focus.prev(&FILTER_CONTROLS);
    } else if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Aliases
    {
        app.lambda_state.alias_input_focus =
            app.lambda_state.alias_input_focus.prev(&FILTER_CONTROLS);
    } else if app.lambda_state.current_function.is_none() {
        app.lambda_state.input_focus = app.lambda_state.input_focus.prev(&FILTER_CONTROLS);
    }
}

pub fn filter_char_allowed(app: &App) -> bool {
    if app.lambda_state.current_version.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Configuration
    {
        app.lambda_state.alias_input_focus == InputFocus::Filter
    } else if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Versions
    {
        app.lambda_state.version_input_focus == InputFocus::Filter
    } else if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Aliases
    {
        app.lambda_state.alias_input_focus == InputFocus::Filter
    } else {
        false
    }
}

// ── Actions ───────────────────────────────────────────────────────────────────

pub fn select_item(app: &mut App) {
    if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Versions
    {
        if app.mode == Mode::Normal {
            let page_size = app.lambda_state.version_table.page_size.value();
            let filtered: Vec<_> = app
                .lambda_state
                .version_table
                .items
                .iter()
                .filter(|v| {
                    app.lambda_state.version_table.filter.is_empty()
                        || v.version
                            .to_lowercase()
                            .contains(&app.lambda_state.version_table.filter.to_lowercase())
                        || v.aliases
                            .to_lowercase()
                            .contains(&app.lambda_state.version_table.filter.to_lowercase())
                })
                .collect();
            let current_page = app.lambda_state.version_table.selected / page_size;
            let start_idx = current_page * page_size;
            let end_idx = (start_idx + page_size).min(filtered.len());
            let paginated: Vec<_> = filtered[start_idx..end_idx].to_vec();
            let page_index = app.lambda_state.version_table.selected % page_size;
            if let Some(version) = paginated.get(page_index) {
                app.lambda_state.current_version = Some(version.version.clone());
                app.lambda_state.detail_tab = LambdaDetailTab::Code;
            }
        } else {
            if app.lambda_state.version_table.expanded_item
                == Some(app.lambda_state.version_table.selected)
            {
                app.lambda_state.version_table.collapse();
            } else {
                app.lambda_state.version_table.expanded_item =
                    Some(app.lambda_state.version_table.selected);
            }
        }
    } else if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Aliases
    {
        let filtered: Vec<_> = app
            .lambda_state
            .alias_table
            .items
            .iter()
            .filter(|a| {
                app.lambda_state.alias_table.filter.is_empty()
                    || a.name
                        .to_lowercase()
                        .contains(&app.lambda_state.alias_table.filter.to_lowercase())
                    || a.versions
                        .to_lowercase()
                        .contains(&app.lambda_state.alias_table.filter.to_lowercase())
            })
            .collect();
        if let Some(alias) = app.lambda_state.alias_table.get_selected(&filtered) {
            app.lambda_state.current_alias = Some(alias.name.clone());
        }
    } else if app.lambda_state.current_function.is_none() {
        let filtered = filtered_lambda_functions(app);
        if let Some(func) = app.lambda_state.table.get_selected(&filtered) {
            app.lambda_state.current_function = Some(func.name.clone());
            app.lambda_state.detail_tab = LambdaDetailTab::Code;
            app.update_current_tab_breadcrumb();
        }
    }
}

pub fn go_back(app: &mut App) {
    if app.lambda_state.current_version.is_some() {
        app.lambda_state.current_version = None;
        app.lambda_state.detail_tab = LambdaDetailTab::Versions;
    } else if app.lambda_state.current_alias.is_some() {
        app.lambda_state.current_alias = None;
        app.lambda_state.detail_tab = LambdaDetailTab::Aliases;
    } else if app.lambda_state.current_function.is_some() {
        app.lambda_state.current_function = None;
        app.update_current_tab_breadcrumb();
    }
}

pub fn next_detail_tab(app: &mut App) {
    if app.lambda_state.current_version.is_some() {
        app.lambda_state.version_detail_tab = app.lambda_state.version_detail_tab.next();
        app.lambda_state.detail_tab = app.lambda_state.version_detail_tab.to_detail_tab();
        if app.lambda_state.detail_tab == LambdaDetailTab::Monitor {
            app.lambda_state.set_metrics_loading(true);
            app.lambda_state.set_monitoring_scroll(0);
            app.lambda_state.clear_metrics();
        }
    } else {
        app.lambda_state.detail_tab = app.lambda_state.detail_tab.next();
        if app.lambda_state.detail_tab == LambdaDetailTab::Monitor {
            app.lambda_state.set_metrics_loading(true);
            app.lambda_state.set_monitoring_scroll(0);
            app.lambda_state.clear_metrics();
        }
    }
}

pub fn prev_detail_tab(app: &mut App) {
    if app.lambda_state.current_version.is_some() {
        app.lambda_state.version_detail_tab = app.lambda_state.version_detail_tab.prev();
        app.lambda_state.detail_tab = app.lambda_state.version_detail_tab.to_detail_tab();
        if app.lambda_state.detail_tab == LambdaDetailTab::Monitor {
            app.lambda_state.set_metrics_loading(true);
            app.lambda_state.set_monitoring_scroll(0);
            app.lambda_state.clear_metrics();
        }
    } else {
        app.lambda_state.detail_tab = app.lambda_state.detail_tab.prev();
        if app.lambda_state.detail_tab == LambdaDetailTab::Monitor {
            app.lambda_state.set_metrics_loading(true);
            app.lambda_state.set_monitoring_scroll(0);
            app.lambda_state.clear_metrics();
        }
    }
}

pub fn refresh(app: &mut App) {
    app.lambda_state.table.loading = true;
}

pub fn yank(app: &App) {
    use crate::app::copy_to_clipboard;
    let filtered = filtered_lambda_functions(app);
    if let Some(func) = app.lambda_state.table.get_selected(&filtered) {
        copy_to_clipboard(&func.arn);
    }
}

// ── Column preferences ────────────────────────────────────────────────────────

pub fn toggle_column(app: &mut App) {
    let idx = app.column_selector_index;
    if app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Versions
    {
        if idx > 0 && idx <= app.lambda_state.version_column_ids.len() {
            if let Some(col) = app.lambda_state.version_column_ids.get(idx - 1) {
                if let Some(pos) = app
                    .lambda_state
                    .version_visible_column_ids
                    .iter()
                    .position(|c| *c == *col)
                {
                    if app.lambda_state.version_visible_column_ids.len() > 1 {
                        app.lambda_state.version_visible_column_ids.remove(pos);
                    }
                } else {
                    app.lambda_state
                        .version_visible_column_ids
                        .push(col.clone());
                }
            }
        } else if idx == app.lambda_state.version_column_ids.len() + 3 {
            app.lambda_state.version_table.page_size = PageSize::Ten;
        } else if idx == app.lambda_state.version_column_ids.len() + 4 {
            app.lambda_state.version_table.page_size = PageSize::TwentyFive;
        } else if idx == app.lambda_state.version_column_ids.len() + 5 {
            app.lambda_state.version_table.page_size = PageSize::Fifty;
        } else if idx == app.lambda_state.version_column_ids.len() + 6 {
            app.lambda_state.version_table.page_size = PageSize::OneHundred;
        }
    } else if (app.lambda_state.current_function.is_some()
        && app.lambda_state.detail_tab == LambdaDetailTab::Aliases)
        || (app.lambda_state.current_version.is_some()
            && app.lambda_state.detail_tab == LambdaDetailTab::Configuration)
    {
        if idx > 0 && idx <= app.lambda_state.alias_column_ids.len() {
            if let Some(col) = app.lambda_state.alias_column_ids.get(idx - 1) {
                if let Some(pos) = app
                    .lambda_state
                    .alias_visible_column_ids
                    .iter()
                    .position(|c| *c == *col)
                {
                    if app.lambda_state.alias_visible_column_ids.len() > 1 {
                        app.lambda_state.alias_visible_column_ids.remove(pos);
                    }
                } else {
                    app.lambda_state.alias_visible_column_ids.push(col.clone());
                }
            }
        } else if idx == app.lambda_state.alias_column_ids.len() + 3 {
            app.lambda_state.alias_table.page_size = PageSize::Ten;
        } else if idx == app.lambda_state.alias_column_ids.len() + 4 {
            app.lambda_state.alias_table.page_size = PageSize::TwentyFive;
        } else if idx == app.lambda_state.alias_column_ids.len() + 5 {
            app.lambda_state.alias_table.page_size = PageSize::Fifty;
        } else if idx == app.lambda_state.alias_column_ids.len() + 6 {
            app.lambda_state.alias_table.page_size = PageSize::OneHundred;
        }
    } else {
        if idx > 0 && idx <= app.lambda_state.function_column_ids.len() {
            if let Some(col) = app.lambda_state.function_column_ids.get(idx - 1) {
                if let Some(pos) = app
                    .lambda_state
                    .function_visible_column_ids
                    .iter()
                    .position(|c| c == col)
                {
                    if app.lambda_state.function_visible_column_ids.len() > 1 {
                        app.lambda_state.function_visible_column_ids.remove(pos);
                    }
                } else {
                    app.lambda_state.function_visible_column_ids.push(*col);
                }
            }
        } else if idx == app.lambda_state.function_column_ids.len() + 3 {
            app.lambda_state.table.page_size = PageSize::Ten;
        } else if idx == app.lambda_state.function_column_ids.len() + 4 {
            app.lambda_state.table.page_size = PageSize::TwentyFive;
        } else if idx == app.lambda_state.function_column_ids.len() + 5 {
            app.lambda_state.table.page_size = PageSize::Fifty;
        } else if idx == app.lambda_state.function_column_ids.len() + 6 {
            app.lambda_state.table.page_size = PageSize::OneHundred;
        }
    }
}

pub fn next_preferences(app: &mut App) {
    let page_size_idx = app.lambda_state.function_column_ids.len() + 2;
    if app.column_selector_index < page_size_idx {
        app.column_selector_index = page_size_idx;
    } else {
        app.column_selector_index = 0;
    }
}

pub fn prev_preferences(app: &mut App) {
    let page_size_idx = app.lambda_state.function_column_ids.len() + 2;
    if app.column_selector_index >= page_size_idx {
        app.column_selector_index = 0;
    } else {
        app.column_selector_index = page_size_idx;
    }
}

pub fn column_selector_max(app: &App) -> usize {
    app.lambda_state.function_column_ids.len() + 6
}

pub fn column_count(app: &App) -> usize {
    app.lambda_state.function_column_ids.len()
}

// ── Breadcrumb / console URL ──────────────────────────────────────────────────

pub fn breadcrumb(app: &App) -> Vec<String> {
    let mut parts = vec!["Lambda".to_string()];
    if let Some(func) = &app.lambda_state.current_function {
        parts.push(func.clone());
    } else {
        parts.push("Functions".to_string());
    }
    parts
}

pub fn console_url(app: &App) -> String {
    use crate::lambda;
    if let Some(func_name) = &app.lambda_state.current_function {
        if let Some(version) = &app.lambda_state.current_version {
            lambda::console_url_function_version(
                &app.config.region,
                func_name,
                version,
                &app.lambda_state.detail_tab,
            )
        } else {
            lambda::console_url_function_detail(&app.config.region, func_name)
        }
    } else {
        lambda::console_url_functions(&app.config.region)
    }
}
