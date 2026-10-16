use crate::{
    app::{App, Service},
    inventory::{self, InventoryInput},
    keymap::Action,
    ui::stepfunctions::filtered_machines,
};

/// ARN of the state machine highlighted in the inventory (used for Yank on the
/// list). In the entered view, selection is the entered machine's ARN, not the
/// highlighted execution.
pub fn selected_arn(app: &App) -> Option<String> {
    if let Some(machine) = &app.stepfunctions_state.executions.machine {
        return Some(machine.arn.clone());
    }
    let machines = filtered_machines(app);
    app.stepfunctions_state
        .machines
        .get_selected(&machines)
        .map(|machine| machine.arn.clone())
}

/// Regional Step Functions console link.
///
/// The State Machines inventory always opens the regional list, regardless of
/// the highlighted/filtered/expanded row — deep links belong only to an entered
/// machine. The entered view deep-links to the SELECTED STATE MACHINE's console
/// route (never the selected execution).
pub fn console_url(app: &App) -> String {
    let entered = app
        .stepfunctions_state
        .executions
        .machine
        .as_ref()
        .map(|machine| machine.arn.as_str());
    super::console_url(&app.config.region, entered)
}

pub fn go_to_page(app: &mut App, page: usize) {
    if super::executions::active(app) {
        super::executions::go_to_page(app, page);
        return;
    }
    let count = filtered_machines(app).len();
    inventory::go_to_page(&mut app.stepfunctions_state.machines, page, count);
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !app.service_selected || app.current_service != Service::StepFunctionsStateMachines {
        return false;
    }
    if super::executions::handle_action(app, action) {
        return true;
    }
    if super::executions::active(app) {
        return false;
    }
    let filtered_count = filtered_machines(app).len();
    let selected_arn = selected_arn(app);
    inventory::handle_action(
        InventoryInput {
            table: &mut app.stepfunctions_state.machines,
            focus: &mut app.stepfunctions_state.input_focus,
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
