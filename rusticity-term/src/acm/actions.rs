use crate::{
    app::{App, Service},
    inventory::{self, InventoryInput},
    keymap::Action,
    ui::acm::filtered_certificates,
};

pub fn selected_arn(app: &App) -> Option<String> {
    let rows = filtered_certificates(app);
    app.acm_state
        .certificates
        .get_selected(&rows)
        .map(|cert| cert.arn.clone())
}

pub fn console_url(app: &App) -> String {
    super::console_url(&app.config.region)
}

pub fn go_to_page(app: &mut App, page: usize) {
    let count = filtered_certificates(app).len();
    inventory::go_to_page(&mut app.acm_state.certificates, page, count);
}

/// Shared table input (navigation, filter, pagination, expansion, copy).
pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !app.service_selected || app.current_service != Service::AcmCertificates {
        return false;
    }
    let filtered_count = filtered_certificates(app).len();
    let selected_arn = selected_arn(app);
    inventory::handle_action(
        InventoryInput {
            table: &mut app.acm_state.certificates,
            focus: &mut app.acm_state.input_focus,
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
