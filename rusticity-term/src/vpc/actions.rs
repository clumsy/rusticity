use crate::app::{App, Service};
use crate::inventory::{self, InventoryInput};
use crate::keymap::Action;
use crate::ui::vpc::filtered_vpcs;
use crate::vpc;

pub fn console_url(app: &App) -> String {
    vpc::console_url(&app.config.region)
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

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !app.service_selected || app.current_service != Service::VpcVpcs {
        return false;
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
