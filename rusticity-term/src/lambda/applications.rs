use crate::app::App;
use crate::common::{CyclicEnum, InputFocus, PageSize};
use crate::ui::lambda::{
    filtered_lambda_applications, ApplicationDetailTab as LambdaApplicationDetailTab,
    FILTER_CONTROLS,
};

// ── Navigation ────────────────────────────────────────────────────────────────

pub fn next_item(app: &mut App) {
    if app.lambda_application_state.current_application.is_some() {
        if app.lambda_application_state.detail_tab == LambdaApplicationDetailTab::Overview {
            let len = app.lambda_application_state.resources.items.len();
            if len > 0 {
                app.lambda_application_state.resources.next_item(len);
            }
        } else {
            let len = app.lambda_application_state.deployments.items.len();
            if len > 0 {
                app.lambda_application_state.deployments.next_item(len);
            }
        }
    } else {
        let filtered = filtered_lambda_applications(app);
        if !filtered.is_empty() {
            app.lambda_application_state.table.selected =
                (app.lambda_application_state.table.selected + 1).min(filtered.len() - 1);
            app.lambda_application_state.table.snap_to_page();
        }
    }
}

pub fn prev_item(app: &mut App) {
    if app.lambda_application_state.current_application.is_some()
        && app.lambda_application_state.detail_tab == LambdaApplicationDetailTab::Overview
    {
        app.lambda_application_state.resources.selected = app
            .lambda_application_state
            .resources
            .selected
            .saturating_sub(1);
    } else if app.lambda_application_state.current_application.is_some()
        && app.lambda_application_state.detail_tab == LambdaApplicationDetailTab::Deployments
    {
        app.lambda_application_state.deployments.selected = app
            .lambda_application_state
            .deployments
            .selected
            .saturating_sub(1);
    } else {
        app.lambda_application_state.table.selected = app
            .lambda_application_state
            .table
            .selected
            .saturating_sub(1);
        app.lambda_application_state.table.snap_to_page();
    }
}

pub fn page_down_filter_input(app: &mut App) {
    if app.lambda_application_state.current_application.is_some() {
        if app.lambda_application_state.detail_tab == LambdaApplicationDetailTab::Deployments {
            let page_size = app.lambda_application_state.deployments.page_size.value();
            let filtered_count = app.lambda_application_state.deployments.items.len();
            app.lambda_application_state
                .deployment_input_focus
                .handle_page_down(
                    &mut app.lambda_application_state.deployments.selected,
                    &mut app.lambda_application_state.deployments.scroll_offset,
                    page_size,
                    filtered_count,
                );
        } else {
            let page_size = app.lambda_application_state.resources.page_size.value();
            let filtered_count = app.lambda_application_state.resources.items.len();
            app.lambda_application_state
                .resource_input_focus
                .handle_page_down(
                    &mut app.lambda_application_state.resources.selected,
                    &mut app.lambda_application_state.resources.scroll_offset,
                    page_size,
                    filtered_count,
                );
        }
    } else {
        let page_size = app.lambda_application_state.table.page_size.value();
        let filtered_count = filtered_lambda_applications(app).len();
        app.lambda_application_state.input_focus.handle_page_down(
            &mut app.lambda_application_state.table.selected,
            &mut app.lambda_application_state.table.scroll_offset,
            page_size,
            filtered_count,
        );
    }
}

pub fn page_down_normal(app: &mut App) {
    let len = filtered_lambda_applications(app).len();
    app.lambda_application_state.table.page_down(len);
}

pub fn page_up_filter_input(app: &mut App) {
    if app.lambda_application_state.current_application.is_some() {
        if app.lambda_application_state.detail_tab == LambdaApplicationDetailTab::Deployments {
            let page_size = app.lambda_application_state.deployments.page_size.value();
            app.lambda_application_state
                .deployment_input_focus
                .handle_page_up(
                    &mut app.lambda_application_state.deployments.selected,
                    &mut app.lambda_application_state.deployments.scroll_offset,
                    page_size,
                );
        } else {
            let page_size = app.lambda_application_state.resources.page_size.value();
            app.lambda_application_state
                .resource_input_focus
                .handle_page_up(
                    &mut app.lambda_application_state.resources.selected,
                    &mut app.lambda_application_state.resources.scroll_offset,
                    page_size,
                );
        }
    } else {
        let page_size = app.lambda_application_state.table.page_size.value();
        app.lambda_application_state.input_focus.handle_page_up(
            &mut app.lambda_application_state.table.selected,
            &mut app.lambda_application_state.table.scroll_offset,
            page_size,
        );
    }
}

pub fn page_up_normal(app: &mut App) {
    app.lambda_application_state.table.page_up();
}

pub fn expand_row(app: &mut App) {
    if app.lambda_application_state.current_application.is_some() {
        if app.lambda_application_state.detail_tab == LambdaApplicationDetailTab::Overview {
            app.lambda_application_state.resources.toggle_expand();
        } else {
            app.lambda_application_state.deployments.toggle_expand();
        }
    } else {
        app.lambda_application_state.table.expand();
    }
}

pub fn prev_pane(app: &mut App) {
    if app.lambda_application_state.current_application.is_some() {
        if app.lambda_application_state.detail_tab == LambdaApplicationDetailTab::Overview {
            app.lambda_application_state.resources.collapse();
        } else {
            app.lambda_application_state.deployments.collapse();
        }
    } else if app.lambda_application_state.table.has_expanded_item() {
        app.lambda_application_state.table.collapse();
    }
}

pub fn go_to_page(app: &mut App, page: usize) {
    let filtered_count = filtered_lambda_applications(app).len();
    app.lambda_application_state
        .table
        .goto_page(page, filtered_count);
}

// ── Filter ────────────────────────────────────────────────────────────────────

pub fn apply_filter_reset(app: &mut App) {
    if app.lambda_application_state.current_application.is_some() {
        app.lambda_application_state.deployments.reset();
        app.lambda_application_state.resources.reset();
    } else {
        app.lambda_application_state.table.reset();
    }
}

pub fn start_filter(app: &mut App) {
    if app.lambda_application_state.current_application.is_some() {
        if app.lambda_application_state.detail_tab == LambdaApplicationDetailTab::Overview {
            app.lambda_application_state.resource_input_focus = InputFocus::Filter;
        } else {
            app.lambda_application_state.deployment_input_focus = InputFocus::Filter;
        }
    } else {
        app.lambda_application_state.input_focus = InputFocus::Filter;
    }
}

pub fn is_pagination_focused(app: &App) -> bool {
    if app.lambda_application_state.current_application.is_some() {
        if app.lambda_application_state.detail_tab == LambdaApplicationDetailTab::Deployments {
            app.lambda_application_state.deployment_input_focus == InputFocus::Pagination
        } else {
            app.lambda_application_state.resource_input_focus == InputFocus::Pagination
        }
    } else {
        app.lambda_application_state.input_focus == InputFocus::Pagination
    }
}

pub fn is_filter_focused(app: &App) -> bool {
    if app.lambda_application_state.current_application.is_some() {
        if app.lambda_application_state.detail_tab == LambdaApplicationDetailTab::Deployments {
            app.lambda_application_state.deployment_input_focus == InputFocus::Filter
        } else {
            app.lambda_application_state.resource_input_focus == InputFocus::Filter
        }
    } else {
        app.lambda_application_state.input_focus == InputFocus::Filter
    }
}

pub fn next_filter_focus(app: &mut App) {
    if app.lambda_application_state.current_application.is_some() {
        if app.lambda_application_state.detail_tab == LambdaApplicationDetailTab::Deployments {
            app.lambda_application_state.deployment_input_focus = app
                .lambda_application_state
                .deployment_input_focus
                .next(&FILTER_CONTROLS);
        } else {
            app.lambda_application_state.resource_input_focus = app
                .lambda_application_state
                .resource_input_focus
                .next(&FILTER_CONTROLS);
        }
    } else {
        app.lambda_application_state.input_focus = app
            .lambda_application_state
            .input_focus
            .next(&FILTER_CONTROLS);
    }
}

pub fn prev_filter_focus(app: &mut App) {
    if app.lambda_application_state.current_application.is_some() {
        if app.lambda_application_state.detail_tab == LambdaApplicationDetailTab::Deployments {
            app.lambda_application_state.deployment_input_focus = app
                .lambda_application_state
                .deployment_input_focus
                .prev(&FILTER_CONTROLS);
        } else {
            app.lambda_application_state.resource_input_focus = app
                .lambda_application_state
                .resource_input_focus
                .prev(&FILTER_CONTROLS);
        }
    } else {
        app.lambda_application_state.input_focus = app
            .lambda_application_state
            .input_focus
            .prev(&FILTER_CONTROLS);
    }
}

// ── Actions ───────────────────────────────────────────────────────────────────

pub fn select_item(app: &mut App) {
    if app.lambda_application_state.current_application.is_some() {
        return; // already in detail view, Enter does nothing at the application level
    }
    let filtered = filtered_lambda_applications(app);
    if let Some(app_item) = app.lambda_application_state.table.get_selected(&filtered) {
        let app_name = app_item.name.clone();
        app.lambda_application_state.current_application = Some(app_name.clone());
        app.lambda_application_state.detail_tab = LambdaApplicationDetailTab::Overview;

        use crate::lambda::{Deployment, Resource};
        app.lambda_application_state.resources.items = vec![
            Resource {
                logical_id: "ApiGatewayRestApi".to_string(),
                physical_id: "abc123xyz".to_string(),
                resource_type: "AWS::ApiGateway::RestApi".to_string(),
                last_modified: "2025-01-10 14:30:00 (UTC)".to_string(),
            },
            Resource {
                logical_id: "LambdaFunction".to_string(),
                physical_id: format!("{}-function", app_name),
                resource_type: "AWS::Lambda::Function".to_string(),
                last_modified: "2025-01-10 14:25:00 (UTC)".to_string(),
            },
            Resource {
                logical_id: "DynamoDBTable".to_string(),
                physical_id: format!("{}-table", app_name),
                resource_type: "AWS::DynamoDB::Table".to_string(),
                last_modified: "2025-01-09 10:15:00 (UTC)".to_string(),
            },
        ];
        app.lambda_application_state.deployments.items = vec![Deployment {
            deployment_id: "d-ABC123XYZ".to_string(),
            resource_type: "AWS::Serverless::Application".to_string(),
            last_updated: "2025-01-10 14:30:00 (UTC)".to_string(),
            status: "Succeeded".to_string(),
        }];
        app.update_current_tab_breadcrumb();
    }
}

pub fn go_back(app: &mut App) {
    app.lambda_application_state.current_application = None;
    app.update_current_tab_breadcrumb();
}

pub fn next_detail_tab(app: &mut App) {
    app.lambda_application_state.detail_tab = app.lambda_application_state.detail_tab.next();
}

pub fn prev_detail_tab(app: &mut App) {
    app.lambda_application_state.detail_tab = app.lambda_application_state.detail_tab.prev();
}

pub fn refresh(app: &mut App) {
    app.lambda_application_state.table.loading = true;
}

// ── Column preferences ────────────────────────────────────────────────────────

pub fn toggle_column(app: &mut App) {
    let idx = app.column_selector_index;
    if app.lambda_application_state.current_application.is_some() {
        if app.lambda_application_state.detail_tab == LambdaApplicationDetailTab::Overview {
            if idx > 0 && idx <= app.lambda_resource_column_ids.len() {
                if let Some(col) = app.lambda_resource_column_ids.get(idx - 1) {
                    if let Some(pos) = app
                        .lambda_resource_visible_column_ids
                        .iter()
                        .position(|c| c == col)
                    {
                        if app.lambda_resource_visible_column_ids.len() > 1 {
                            app.lambda_resource_visible_column_ids.remove(pos);
                        }
                    } else {
                        app.lambda_resource_visible_column_ids.push(*col);
                    }
                }
            } else if idx == app.lambda_resource_column_ids.len() + 3 {
                app.lambda_application_state.resources.page_size = PageSize::Ten;
            } else if idx == app.lambda_resource_column_ids.len() + 4 {
                app.lambda_application_state.resources.page_size = PageSize::TwentyFive;
            } else if idx == app.lambda_resource_column_ids.len() + 5 {
                app.lambda_application_state.resources.page_size = PageSize::Fifty;
            }
        } else {
            if idx > 0 && idx <= app.lambda_deployment_column_ids.len() {
                if let Some(col) = app.lambda_deployment_column_ids.get(idx - 1) {
                    if let Some(pos) = app
                        .lambda_deployment_visible_column_ids
                        .iter()
                        .position(|c| c == col)
                    {
                        if app.lambda_deployment_visible_column_ids.len() > 1 {
                            app.lambda_deployment_visible_column_ids.remove(pos);
                        }
                    } else {
                        app.lambda_deployment_visible_column_ids.push(*col);
                    }
                }
            } else if idx == app.lambda_deployment_column_ids.len() + 3 {
                app.lambda_application_state.deployments.page_size = PageSize::Ten;
            } else if idx == app.lambda_deployment_column_ids.len() + 4 {
                app.lambda_application_state.deployments.page_size = PageSize::TwentyFive;
            } else if idx == app.lambda_deployment_column_ids.len() + 5 {
                app.lambda_application_state.deployments.page_size = PageSize::Fifty;
            }
        }
    } else {
        if idx > 0 && idx <= app.lambda_application_column_ids.len() {
            if let Some(col) = app.lambda_application_column_ids.get(idx - 1) {
                if let Some(pos) = app
                    .lambda_application_visible_column_ids
                    .iter()
                    .position(|c| *c == *col)
                {
                    if app.lambda_application_visible_column_ids.len() > 1 {
                        app.lambda_application_visible_column_ids.remove(pos);
                    }
                } else {
                    app.lambda_application_visible_column_ids.push(*col);
                }
            }
        } else if idx == app.lambda_application_column_ids.len() + 3 {
            app.lambda_application_state.table.page_size = PageSize::Ten;
        } else if idx == app.lambda_application_column_ids.len() + 4 {
            app.lambda_application_state.table.page_size = PageSize::TwentyFive;
        } else if idx == app.lambda_application_column_ids.len() + 5 {
            app.lambda_application_state.table.page_size = PageSize::Fifty;
        }
    }
}

pub fn next_preferences(app: &mut App) {
    let page_size_idx = app.lambda_application_column_ids.len() + 2;
    if app.column_selector_index < page_size_idx {
        app.column_selector_index = page_size_idx;
    } else {
        app.column_selector_index = 0;
    }
}

pub fn prev_preferences(app: &mut App) {
    let page_size_idx = app.lambda_application_column_ids.len() + 2;
    if app.column_selector_index >= page_size_idx {
        app.column_selector_index = 0;
    } else {
        app.column_selector_index = page_size_idx;
    }
}

pub fn column_selector_max(app: &App) -> usize {
    app.lambda_application_column_ids.len() + 5
}

pub fn column_count(app: &App) -> usize {
    app.lambda_application_column_ids.len()
}

// ── Breadcrumb / console URL ──────────────────────────────────────────────────

pub fn breadcrumb() -> Vec<String> {
    vec!["Lambda".to_string(), "Applications".to_string()]
}

pub fn console_url(app: &App) -> String {
    use crate::lambda;
    if let Some(app_name) = &app.lambda_application_state.current_application {
        lambda::console_url_application_detail(
            &app.config.region,
            app_name,
            &app.lambda_application_state.detail_tab,
        )
    } else {
        lambda::console_url_applications(&app.config.region)
    }
}

pub fn get_active_filter_mut(app: &mut App) -> Option<&mut String> {
    if app.lambda_application_state.current_application.is_some() {
        if app.lambda_application_state.detail_tab == LambdaApplicationDetailTab::Deployments {
            Some(&mut app.lambda_application_state.deployments.filter)
        } else {
            Some(&mut app.lambda_application_state.resources.filter)
        }
    } else {
        Some(&mut app.lambda_application_state.table.filter)
    }
}
