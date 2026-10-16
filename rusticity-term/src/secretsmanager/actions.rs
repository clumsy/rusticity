use crate::{
    app::{App, Service},
    inventory::{self, InventoryInput},
    keymap::Action,
    ui::secretsmanager::filtered_secrets,
};

pub fn selected_arn(app: &App) -> Option<String> {
    let rows = filtered_secrets(app);
    app.secretsmanager_state
        .secrets
        .get_selected(&rows)
        .map(|s| s.arn.clone())
}
pub fn console_url(app: &App) -> String {
    super::console_url(&app.config.region)
}
pub fn go_to_page(app: &mut App, page: usize) {
    let count = filtered_secrets(app).len();
    inventory::go_to_page(&mut app.secretsmanager_state.secrets, page, count);
}
pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !app.service_selected || app.current_service != Service::SecretsManagerSecrets {
        return false;
    }
    let filtered_count = filtered_secrets(app).len();
    let selected_arn = selected_arn(app);
    inventory::handle_action(
        InventoryInput {
            table: &mut app.secretsmanager_state.secrets,
            focus: &mut app.secretsmanager_state.input_focus,
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
