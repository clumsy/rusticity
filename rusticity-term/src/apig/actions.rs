use crate::apig::resource::Resource as ApigResource;
use crate::apig::route::Route;
use crate::app::App;
use crate::common::{CyclicEnum, InputFocus, PageSize};
use crate::ui::apig::{filtered_apis, ApiDetailTab, FILTER_CONTROLS};
use crate::ui::tree::TreeItem;
use std::collections::HashMap;

// ── Filter helpers ────────────────────────────────────────────────────────────

pub fn get_active_filter_mut(app: &mut App) -> Option<&mut String> {
    if app.apig_state.current_api.is_some() && app.apig_state.detail_tab == ApiDetailTab::Routes {
        Some(&mut app.apig_state.route_filter)
    } else {
        Some(&mut app.apig_state.apis.filter)
    }
}

pub fn apply_filter_reset(app: &mut App) {
    app.apig_state.apis.reset();
}

pub fn is_pagination_focused(app: &App) -> bool {
    app.apig_state.input_focus == InputFocus::Pagination
}

/// Whether FilterInput character should be passed to the filter.
pub fn filter_char_allowed(app: &App) -> bool {
    app.apig_state.current_api.is_some()
        && app.apig_state.detail_tab == ApiDetailTab::Routes
        && app.apig_state.input_focus == InputFocus::Filter
}

pub fn start_filter(app: &mut App) {
    app.apig_state.input_focus = InputFocus::Filter;
}

pub fn next_filter_focus(app: &mut App) {
    app.apig_state.input_focus = app.apig_state.input_focus.next(&FILTER_CONTROLS);
}

pub fn prev_filter_focus(app: &mut App) {
    app.apig_state.input_focus = app.apig_state.input_focus.prev(&FILTER_CONTROLS);
}

// ── Navigation ────────────────────────────────────────────────────────────────

pub fn next_item(app: &mut App) {
    if let Some(api) = &app.apig_state.current_api {
        if app.apig_state.detail_tab == ApiDetailTab::Routes {
            let protocol = api.protocol_type.to_uppercase();
            if protocol == "REST" {
                let total_rows = crate::ui::tree::TreeRenderer::count_visible_rows(
                    &app.apig_state.resources.items,
                    &app.apig_state.expanded_resources,
                    &app.apig_state.resource_children,
                );
                if total_rows > 0 {
                    app.apig_state.resources.selected =
                        (app.apig_state.resources.selected + 1).min(total_rows - 1);
                }
            } else {
                let total_rows = crate::ui::tree::TreeRenderer::count_visible_rows(
                    &app.apig_state.routes.items,
                    &app.apig_state.expanded_routes,
                    &app.apig_state.route_children,
                );
                if total_rows > 0 {
                    app.apig_state.routes.selected =
                        (app.apig_state.routes.selected + 1).min(total_rows - 1);
                }
            }
        }
    } else {
        let filtered = filtered_apis(app);
        app.apig_state.apis.next_item(filtered.len());
    }
}

pub fn prev_item(app: &mut App) {
    if let Some(api) = &app.apig_state.current_api {
        if app.apig_state.detail_tab == ApiDetailTab::Routes {
            let protocol = api.protocol_type.to_uppercase();
            if protocol == "REST" {
                if app.apig_state.resources.selected > 0 {
                    app.apig_state.resources.selected -= 1;
                }
            } else {
                if app.apig_state.routes.selected > 0 {
                    app.apig_state.routes.selected -= 1;
                }
            }
        }
    } else {
        app.apig_state.apis.prev_item();
    }
}

// ── Detail tabs ───────────────────────────────────────────────────────────────

pub fn next_detail_tab(app: &mut App) {
    app.apig_state.detail_tab = app.apig_state.detail_tab.next();
}

pub fn prev_detail_tab(app: &mut App) {
    app.apig_state.detail_tab = app.apig_state.detail_tab.prev();
}

// ── Expand / collapse / prev_pane ─────────────────────────────────────────────

pub fn expand_row(app: &mut App) {
    if let Some(api) = &app.apig_state.current_api {
        if app.apig_state.detail_tab == ApiDetailTab::Routes {
            let protocol = api.protocol_type.to_uppercase();
            if protocol == "REST" {
                let (filtered_items, filtered_children) = crate::ui::apig::filter_tree_items(
                    &app.apig_state.resources.items,
                    &app.apig_state.resource_children,
                    &app.apig_state.route_filter,
                );
                let selected_row = app.apig_state.resources.selected;
                let mut current_row = 0;
                if let Some(resource_id) =
                    find_resource_at_row(app, &filtered_items, selected_row, &mut current_row)
                {
                    if app.apig_state.expanded_resources.contains(&resource_id) {
                        let total_rows = crate::ui::tree::TreeRenderer::count_visible_rows(
                            &filtered_items,
                            &app.apig_state.expanded_resources,
                            &filtered_children,
                        );
                        if selected_row + 1 < total_rows {
                            app.apig_state.resources.selected = selected_row + 1;
                        }
                    } else {
                        app.apig_state.expanded_resources.insert(resource_id);
                    }
                }
            } else {
                let (filtered_items, filtered_children) = crate::ui::apig::filter_tree_items(
                    &app.apig_state.routes.items,
                    &app.apig_state.route_children,
                    &app.apig_state.route_filter,
                );
                let selected_row = app.apig_state.routes.selected;
                let mut current_row = 0;
                if let Some(route_key) =
                    find_route_at_row(app, &filtered_items, selected_row, &mut current_row)
                {
                    if app.apig_state.expanded_routes.contains(&route_key) {
                        let total_rows = crate::ui::tree::TreeRenderer::count_visible_rows(
                            &filtered_items,
                            &app.apig_state.expanded_routes,
                            &filtered_children,
                        );
                        if selected_row + 1 < total_rows {
                            app.apig_state.routes.selected = selected_row + 1;
                        }
                    } else {
                        app.apig_state.expanded_routes.insert(route_key);
                    }
                }
            }
        }
    } else {
        app.apig_state.apis.expand();
    }
}

pub fn collapse_row(app: &mut App) {
    if let Some(api) = &app.apig_state.current_api {
        if app.apig_state.detail_tab == ApiDetailTab::Routes {
            let protocol = api.protocol_type.to_uppercase();
            if protocol == "REST" {
                let (filtered_items, _) = crate::ui::apig::filter_tree_items(
                    &app.apig_state.resources.items,
                    &app.apig_state.resource_children,
                    &app.apig_state.route_filter,
                );
                let selected_row = app.apig_state.resources.selected;
                let mut current_row = 0;
                if let Some(resource_id) =
                    find_resource_at_row(app, &filtered_items, selected_row, &mut current_row)
                {
                    if app.apig_state.expanded_resources.contains(&resource_id) {
                        app.apig_state.expanded_resources.remove(&resource_id);
                    } else if let Some(parent_row) =
                        find_resource_parent_row(app, &filtered_items, &resource_id)
                    {
                        app.apig_state.resources.selected = parent_row;
                    }
                }
            } else {
                let (filtered_items, _) = crate::ui::apig::filter_tree_items(
                    &app.apig_state.routes.items,
                    &app.apig_state.route_children,
                    &app.apig_state.route_filter,
                );
                let selected_row = app.apig_state.routes.selected;
                let mut current_row = 0;
                if let Some(route_key) =
                    find_route_at_row(app, &filtered_items, selected_row, &mut current_row)
                {
                    if app.apig_state.expanded_routes.contains(&route_key) {
                        app.apig_state.expanded_routes.remove(&route_key);
                    } else if let Some(parent_row) =
                        find_parent_row(app, &filtered_items, &route_key)
                    {
                        app.apig_state.routes.selected = parent_row;
                    }
                }
            }
        }
    } else {
        app.apig_state.apis.collapse();
    }
}

pub fn prev_pane(app: &mut App) {
    app.apig_state.apis.collapse();
}

// ── Select / go back ──────────────────────────────────────────────────────────

pub fn select_item(app: &mut App) {
    if app.apig_state.current_api.is_none() {
        let filtered = filtered_apis(app);
        if let Some(api) = app.apig_state.apis.get_selected(&filtered) {
            let protocol = api.protocol_type.to_uppercase();
            app.apig_state.current_api = Some((*api).clone());
            if protocol == "REST" {
                app.apig_state.resources.loading = true;
            } else {
                app.apig_state.routes.loading = true;
            }
            app.update_current_tab_breadcrumb();
        }
    }
}

pub fn go_back(app: &mut App) {
    app.apig_state.current_api = None;
    app.apig_state.routes.items.clear();
    app.apig_state.detail_tab = ApiDetailTab::Routes;
    app.update_current_tab_breadcrumb();
}

// ── Column preferences ────────────────────────────────────────────────────────

pub fn column_selector_max(app: &App) -> usize {
    app.apig_api_column_ids.len() + 6
}

pub fn column_count(app: &App) -> usize {
    app.apig_api_column_ids.len()
}

pub fn next_preferences(app: &mut App) {
    let num_columns = app.apig_api_column_ids.len();
    let page_size_idx = num_columns + 2;
    if app.column_selector_index < page_size_idx {
        app.column_selector_index = page_size_idx;
    } else {
        app.column_selector_index = 0;
    }
}

pub fn prev_preferences(app: &mut App) {
    let num_columns = app.apig_api_column_ids.len();
    let page_size_idx = num_columns + 2;
    if app.column_selector_index == 0 {
        app.column_selector_index = page_size_idx;
    } else {
        app.column_selector_index = 0;
    }
}

pub fn toggle_column(app: &mut App) {
    let idx = app.column_selector_index;
    if let Some(api) = &app.apig_state.current_api {
        if app.apig_state.detail_tab == ApiDetailTab::Routes {
            if api.protocol_type.to_uppercase() == "REST" {
                // REST API — resource columns; first column is locked
                if idx > 1 && idx <= app.apig_resource_column_ids.len() {
                    if let Some(col) = app.apig_resource_column_ids.get(idx - 1) {
                        if let Some(pos) = app
                            .apig_resource_visible_column_ids
                            .iter()
                            .position(|c| c == col)
                        {
                            app.apig_resource_visible_column_ids.remove(pos);
                        } else {
                            app.apig_resource_visible_column_ids.push(*col);
                        }
                    }
                }
            } else {
                // HTTP/WebSocket — route columns; first column is locked
                if idx > 1 && idx <= app.apig_route_column_ids.len() {
                    if let Some(col) = app.apig_route_column_ids.get(idx - 1) {
                        if let Some(pos) = app
                            .apig_route_visible_column_ids
                            .iter()
                            .position(|c| c == col)
                        {
                            app.apig_route_visible_column_ids.remove(pos);
                        } else {
                            app.apig_route_visible_column_ids.push(*col);
                        }
                    }
                }
            }
            // idx == 1 is the locked first column — do nothing
        }
    } else {
        // API list view
        if idx > 0 && idx <= app.apig_api_column_ids.len() {
            if let Some(col) = app.apig_api_column_ids.get(idx - 1) {
                if let Some(pos) = app
                    .apig_api_visible_column_ids
                    .iter()
                    .position(|c| c == col)
                {
                    app.apig_api_visible_column_ids.remove(pos);
                } else {
                    app.apig_api_visible_column_ids.push(*col);
                }
            }
        } else if idx == app.apig_api_column_ids.len() + 3 {
            app.apig_state.apis.page_size = PageSize::Ten;
        } else if idx == app.apig_api_column_ids.len() + 4 {
            app.apig_state.apis.page_size = PageSize::TwentyFive;
        } else if idx == app.apig_api_column_ids.len() + 5 {
            app.apig_state.apis.page_size = PageSize::Fifty;
        } else if idx == app.apig_api_column_ids.len() + 6 {
            app.apig_state.apis.page_size = PageSize::OneHundred;
        }
    }
}

// ── Breadcrumb / console URL ──────────────────────────────────────────────────

pub fn breadcrumb(app: &App) -> Vec<String> {
    let mut parts = vec!["API Gateway".to_string()];
    if let Some(api) = &app.apig_state.current_api {
        parts.push(api.name.clone());
    } else {
        parts.push("APIs".to_string());
    }
    parts
}

pub fn console_url(app: &App) -> String {
    use crate::apig;
    if let Some(api) = &app.apig_state.current_api {
        if app.apig_state.detail_tab == ApiDetailTab::Routes {
            let protocol = api.protocol_type.to_uppercase();
            if protocol == "REST" {
                let (filtered_items, _) = crate::ui::apig::filter_tree_items(
                    &app.apig_state.resources.items,
                    &app.apig_state.resource_children,
                    &app.apig_state.route_filter,
                );
                let resource_id = if app.apig_state.resources.selected < filtered_items.len() {
                    let mut current_row = 0;
                    find_resource_at_row(
                        app,
                        &filtered_items,
                        app.apig_state.resources.selected,
                        &mut current_row,
                    )
                } else {
                    None
                };
                apig::console_url_resources(&app.config.region, &api.id, resource_id.as_deref())
            } else {
                let (filtered_items, filtered_children) = crate::ui::apig::filter_tree_items(
                    &app.apig_state.routes.items,
                    &app.apig_state.route_children,
                    &app.apig_state.route_filter,
                );
                let total_rows = crate::ui::tree::TreeRenderer::count_visible_rows(
                    &filtered_items,
                    &app.apig_state.expanded_routes,
                    &filtered_children,
                );
                let route_id = if app.apig_state.routes.selected < total_rows {
                    let mut current_row = 0;
                    find_route_id_at_row_with_children(
                        app,
                        &filtered_items,
                        &filtered_children,
                        app.apig_state.routes.selected,
                        &mut current_row,
                    )
                } else {
                    None
                };
                apig::console_url_routes(&app.config.region, &api.id, route_id.as_deref())
            }
        } else {
            apig::console_url_api(&app.config.region, &api.id)
        }
    } else {
        apig::console_url_apis(&app.config.region)
    }
}

pub fn yank(app: &App) {
    use crate::app::copy_to_clipboard;
    if let Some(_api) = &app.apig_state.current_api {
        if app.apig_state.detail_tab == ApiDetailTab::Routes {
            let (filtered_routes, _) = crate::ui::apig::filter_tree_items(
                &app.apig_state.routes.items,
                &app.apig_state.route_children,
                &app.apig_state.route_filter,
            );
            let filtered_refs: Vec<&Route> = filtered_routes.iter().collect();
            if let Some(route) = app.apig_state.routes.get_selected(&filtered_refs) {
                if !route.arn.is_empty() {
                    copy_to_clipboard(&route.arn);
                }
            }
        }
    }
}

// ── Tree traversal helpers ────────────────────────────────────────────────────

pub fn find_route_at_row(
    app: &App,
    routes: &[Route],
    target_row: usize,
    current_row: &mut usize,
) -> Option<String> {
    for route in routes {
        if *current_row == target_row {
            return Some(route.route_key.clone());
        }
        *current_row += 1;
        if route.is_expandable() && app.apig_state.expanded_routes.contains(&route.route_key) {
            if let Some(children) = app.apig_state.route_children.get(&route.route_key) {
                if let Some(key) = find_route_at_row(app, children, target_row, current_row) {
                    return Some(key);
                }
            }
        }
    }
    None
}

pub fn find_route_id_at_row_with_children(
    app: &App,
    routes: &[Route],
    children_map: &HashMap<String, Vec<Route>>,
    target_row: usize,
    current_row: &mut usize,
) -> Option<String> {
    for route in routes {
        if *current_row == target_row {
            if !route.target.is_empty() {
                return Some(route.route_id.clone());
            } else {
                return None;
            }
        }
        *current_row += 1;
        if route.is_expandable() && app.apig_state.expanded_routes.contains(&route.route_key) {
            if let Some(children) = children_map.get(&route.route_key) {
                if let Some(id) = find_route_id_at_row_with_children(
                    app,
                    children,
                    children_map,
                    target_row,
                    current_row,
                ) {
                    return Some(id);
                }
            }
        }
    }
    None
}

pub fn find_parent_row(app: &App, routes: &[Route], child_key: &str) -> Option<usize> {
    let mut current_row = 0;
    find_parent_row_recursive(app, routes, child_key, &mut current_row)
}

fn find_parent_row_recursive(
    app: &App,
    routes: &[Route],
    child_key: &str,
    current_row: &mut usize,
) -> Option<usize> {
    for route in routes {
        let parent_row = *current_row;
        *current_row += 1;
        if route.is_expandable() && app.apig_state.expanded_routes.contains(&route.route_key) {
            if let Some(children) = app.apig_state.route_children.get(&route.route_key) {
                for child in children {
                    if child.route_key == child_key {
                        return Some(parent_row);
                    }
                }
                if let Some(row) = find_parent_row_recursive(app, children, child_key, current_row)
                {
                    return Some(row);
                }
            }
        }
    }
    None
}

pub fn find_resource_at_row(
    app: &App,
    resources: &[ApigResource],
    target_row: usize,
    current_row: &mut usize,
) -> Option<String> {
    for resource in resources {
        if *current_row == target_row {
            return Some(resource.id.clone());
        }
        *current_row += 1;
        if app.apig_state.expanded_resources.contains(&resource.id) {
            if let Some(children) = app.apig_state.resource_children.get(&resource.id) {
                if let Some(id) = find_resource_at_row(app, children, target_row, current_row) {
                    return Some(id);
                }
            }
        }
    }
    None
}

pub fn find_resource_parent_row(
    app: &App,
    resources: &[ApigResource],
    child_id: &str,
) -> Option<usize> {
    let mut current_row = 0;
    find_resource_parent_row_recursive(app, resources, child_id, &mut current_row)
}

fn find_resource_parent_row_recursive(
    app: &App,
    resources: &[ApigResource],
    child_id: &str,
    current_row: &mut usize,
) -> Option<usize> {
    for resource in resources {
        let parent_row = *current_row;
        *current_row += 1;
        if app.apig_state.expanded_resources.contains(&resource.id) {
            if let Some(children) = app.apig_state.resource_children.get(&resource.id) {
                for child in children {
                    if child.id == child_id {
                        return Some(parent_row);
                    }
                }
                if let Some(row) =
                    find_resource_parent_row_recursive(app, children, child_id, current_row)
                {
                    return Some(row);
                }
            }
        }
    }
    None
}
