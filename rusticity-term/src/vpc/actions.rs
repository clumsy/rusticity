use crate::app::{App, Service};
use crate::common::CyclicEnum;
use crate::inventory::{self, InventoryInput};
use crate::keymap::Action;
use crate::ui::vpc::filtered_vpcs;
use crate::vpc;

pub fn console_url(app: &App) -> String {
    match &app.vpc_state.current_vpc {
        Some(vpc_id) => vpc::console_url_vpc_details(&app.aws.config.region, vpc_id),
        None => vpc::console_url(&app.aws.config.region),
    }
}

pub fn selected_arn(app: &App) -> Option<String> {
    let vpcs = filtered_vpcs(app);
    app.vpc_state.vpcs.get_selected(&vpcs).map(|vpc| {
        if vpc.arn.is_empty() {
            vpc.vpc_id.clone()
        } else {
            vpc.arn.clone()
        }
    })
}

pub fn go_to_page(app: &mut App, page: usize) {
    let count = filtered_vpcs(app).len();
    inventory::go_to_page(&mut app.vpc_state.vpcs, page, count);
}

pub fn select_item(app: &mut App) {
    if app.vpc_state.current_vpc.is_some() {
        return;
    }
    let filtered = filtered_vpcs(app);
    if let Some(vpc) = app.vpc_state.vpcs.get_selected(&filtered) {
        let vpc = vpc.clone();
        app.vpc_state.current_vpc = Some(vpc.vpc_id.clone());
        app.vpc_state.detail = Some(vpc);
        app.vpc_state.detail_tab = crate::ui::vpc::DetailTab::Cidrs;
        app.vpc_state.tags.items.clear();
        app.vpc_state.tags.reset();
        app.vpc_state.detail_loading = true;
        app.vpc_state.detail_error = None;
        app.update_current_tab_breadcrumb();
    }
}

pub fn go_back(app: &mut App) {
    app.vpc_state.current_vpc = None;
    app.vpc_state.detail = None;
    app.vpc_state.detail_tab = crate::ui::vpc::DetailTab::Cidrs;
    app.vpc_state.tags.items.clear();
    app.vpc_state.tags.reset();
    app.vpc_state.detail_error = None;
    app.update_current_tab_breadcrumb();
}

pub fn next_detail_tab(app: &mut App) {
    app.vpc_state.detail_tab = app.vpc_state.detail_tab.next();
}

pub fn prev_detail_tab(app: &mut App) {
    app.vpc_state.detail_tab = app.vpc_state.detail_tab.prev();
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !app.service_selected || app.current_service != Service::VpcVpcs {
        return false;
    }
    // Enter opens the detail view from the list, ahead of row expansion.
    if app.mode == crate::keymap::Mode::Normal && *action == Action::Select {
        select_item(app);
        return true;
    }
    // The detail view owns GoBack, tab cycling and refresh while it is open.
    if app.vpc_state.current_vpc.is_some() {
        match (app.mode, action) {
            (_, Action::GoBack) => {
                go_back(app);
                return true;
            }
            (_, Action::NextDetailTab) => {
                next_detail_tab(app);
                return true;
            }
            (_, Action::PrevDetailTab) => {
                prev_detail_tab(app);
                return true;
            }
            (_, Action::Refresh) => {
                app.vpc_state.detail_loading = true;
                app.vpc_state.detail_error = None;
                return true;
            }
            _ => return false,
        }
    }
    let filtered_count = filtered_vpcs(app).len();
    let selected_arn = selected_arn(app);
    inventory::handle_action(
        InventoryInput {
            table: &mut app.vpc_state.vpcs,
            focus: &mut app.vpc_state.input_focus,
            mode: &mut app.mode,
            page_input: &mut app.page_input,
            error_message: &mut app.error_message,
            error_scroll: &mut app.error_scroll,
            filtered_count,
            selected_arn,
            clipboard: &app.clipboard,
        },
        action,
    )
}
