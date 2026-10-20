//! Root Step Functions state-machine inventory as an owned background read.
//! `request` clears the pane and queues a read; `poll` starts it in a Tokio
//! task bounded by a whole-read timeout and accepts the result only when it
//! still matches the owner's AWS context. A failed or superseded read never
//! leaves stale rows on screen.
#[cfg(test)]
use crate::async_read::ReadStatus;
use crate::{
    app::{App, Service},
    async_read::ReadScope,
    keymap::{Action, Mode},
};
use rusticity_core::{StateMachineMetadata, StepFunctionsClient};
use std::time::Duration;

const READ_TIMEOUT: Duration = Duration::from_secs(30);

fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "inventory:stepfunctions")
}

/// True while this session is viewing the State Machines root (not an entered
/// executions detail, which the executions module owns).
pub fn active(app: &App) -> bool {
    app.service_selected
        && app.current_service == Service::StepFunctionsStateMachines
        && app.stepfunctions_state.executions.machine.is_none()
}

/// Begin (or restart) the inventory read: clear rows + selection, drop any
/// error, mark loading and QUEUE the read. No HTTP here — `poll` issues it.
pub fn request(app: &mut App) {
    let scope = scope(app);
    let state = &mut app.stepfunctions_state;
    state.machines.items.clear();
    state.machines.reset();
    state.machines.loading = true;
    state.inventory_error = None;
    state.pending_focus = None;
    state.open_after_load = false;
    state.inventory.request(scope);
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app) {
        return false;
    }
    match (app.mode, action) {
        (Mode::Normal, Action::GoBack) if app.stepfunctions_state.inventory.is_busy() => {
            let state = &mut app.stepfunctions_state;
            state.inventory.cancel();
            state.machines.items.clear();
            state.machines.loading = false;
            state.inventory_error = Some("Cancelled — Ctrl+R to retry".into());
            state.pending_focus = None;
            state.open_after_load = false;
            true
        }
        (Mode::Normal, Action::Refresh) | (Mode::ErrorModal, Action::RetryLoad) => {
            if app.mode == Mode::ErrorModal {
                app.error_message = None;
                app.error_scroll = 0;
                app.mode = Mode::Normal;
            }
            request(app);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
fn read_status(app: &App) -> ReadStatus {
    app.stepfunctions_state.inventory.status()
}

pub async fn poll(app: &mut App) -> bool {
    let current_scope = scope(app);
    let state = &mut app.stepfunctions_state;
    if state.inventory.invalidate_if_changed(&current_scope) {
        state.machines.items.clear();
        state.machines.reset();
        state.machines.loading = false;
        state.inventory_error = None;
        state.pending_focus = None;
        state.open_after_load = false;
        return true;
    }

    let mut changed = false;
    if state.inventory.is_pending() {
        let config = app.config.clone();
        let state = &mut app.stepfunctions_state;
        if state.inventory.start(async move {
            match tokio::time::timeout(
                READ_TIMEOUT,
                StepFunctionsClient::new(config).list_state_machines(),
            )
            .await
            {
                Ok(result) => result.map_err(|e| format!("{e:#}")),
                Err(_) => Err("State machines read timed out after 30s".to_string()),
            }
        }) {
            changed = true;
        }
    }

    if let Some(result) = app
        .stepfunctions_state
        .inventory
        .take_ready(&current_scope)
        .await
    {
        let state = &mut app.stepfunctions_state;
        state.machines.loading = false;
        match result {
            Ok(rows) => {
                state.machines.items = rows;
                state.machines.reset();
                state.inventory_error = None;
                apply_pending_focus(app);
            }
            Err(error) => {
                state.machines.items.clear();
                state.machines.reset();
                state.pending_focus = None;
                state.open_after_load = false;
                state.inventory_error =
                    Some(sanitize(error.into_message("State machines read failed")));
            }
        }
        changed = true;
    }
    changed
}

fn apply_pending_focus(app: &mut App) {
    let open = std::mem::take(&mut app.stepfunctions_state.open_after_load);
    let Some(target) = app.stepfunctions_state.pending_focus.take() else {
        return;
    };
    if open {
        app.stepfunctions_state.machines.filter.clear();
    }
    if let Some(index) = crate::ui::stepfunctions::filtered_machines(app)
        .iter()
        .position(|machine: &&StateMachineMetadata| machine.arn == target || machine.name == target)
    {
        app.stepfunctions_state.machines.selected = index;
        if open {
            crate::finder::navigation::enter_loaded(app);
        }
    } else if open {
        app.stepfunctions_state.machines.items.clear();
        app.stepfunctions_state.inventory_error =
            Some(format!("Resource not found or not accessible: {target}"));
    }
}

fn sanitize(message: String) -> String {
    message
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

#[cfg(test)]
mod tests;
