use crate::app::{App, Service};
use crate::inventory::{self, InventoryInput};
use crate::keymap::Action;
use crate::rds;
use crate::ui::rds::filtered_instances;

pub fn console_url(app: &App) -> String {
    let instances = filtered_instances(app);
    match app.rds_state.instances.get_selected(&instances) {
        Some(instance) if !instance.identifier.is_empty() => {
            rds::console_url_instance(&app.config.region, &instance.identifier)
        }
        _ => rds::console_url_instances(&app.config.region),
    }
}

pub fn selected_arn(app: &App) -> Option<String> {
    let instances = filtered_instances(app);
    app.rds_state
        .instances
        .get_selected(&instances)
        .map(|instance| {
            if instance.arn.is_empty() {
                instance.identifier.clone()
            } else {
                instance.arn.clone()
            }
        })
}

pub fn go_to_page(app: &mut App, page: usize) {
    let count = filtered_instances(app).len();
    inventory::go_to_page(&mut app.rds_state.instances, page, count);
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !app.service_selected || app.current_service != Service::RdsInstances {
        return false;
    }
    let filtered_count = filtered_instances(app).len();
    let selected_arn = selected_arn(app);
    inventory::handle_action(
        InventoryInput {
            table: &mut app.rds_state.instances,
            focus: &mut app.rds_state.input_focus,
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
