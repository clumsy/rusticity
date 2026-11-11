//! EC2 instance list as a tab-owned background read, bounded by 30s. See
//! `crate::list_read`. Only the list is owned here; the entered instance keeps
//! its Tags and Monitoring loaders. A finder or bookmark open enters the exact
//! instance once the list is in.
use crate::{
    app::{App, Service},
    async_read::{ReadScope, ReadStatus},
    keymap::{Action, Mode},
};
use rusticity_core::Ec2Client;
use std::time::Duration;

const READ_TIMEOUT: Duration = Duration::from_secs(30);

fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "inventory:ec2")
}

/// True on the instance list, not an entered instance.
pub fn active(app: &App) -> bool {
    app.service_selected
        && app.current_service == Service::Ec2Instances
        && app.ec2_state.current_instance.is_none()
}

/// Input-handler safe: clears the rows at once and queues the read.
pub fn request(app: &mut App) {
    let scope = scope(app);
    let state = &mut app.ec2_state;
    state.pending_focus = None;
    state.inventory.request(&mut state.table, scope);
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app) {
        return false;
    }
    match (app.mode, action) {
        (Mode::Normal, Action::GoBack) if app.ec2_state.inventory.is_busy() => {
            let state = &mut app.ec2_state;
            state.pending_focus = None;
            state.inventory.cancel(&mut state.table);
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

pub async fn poll(app: &mut App) -> bool {
    let scope = scope(app);
    let config = app.config.clone();
    let state = &mut app.ec2_state;
    let changed = state
        .inventory
        .drive(
            &mut state.table,
            &scope,
            "EC2 instances read failed",
            || async move {
                match tokio::time::timeout(READ_TIMEOUT, Ec2Client::new(config).list_instances())
                    .await
                {
                    Ok(result) => result.map(App::ec2_rows).map_err(|e| format!("{e:#}")),
                    Err(_) => Err(format!(
                        "EC2 instances read timed out after {READ_TIMEOUT:?}"
                    )),
                }
            },
        )
        .await;
    if changed && app.ec2_state.inventory.status() == ReadStatus::Ready {
        if let Some(target) = app.ec2_state.pending_focus.take() {
            let state = &mut app.ec2_state;
            state.table.filter.clear();
            state.state_filter = crate::ui::ec2::StateFilter::AllStates;
            state.table.items.retain(|row| row.instance_id == target);
            if state.table.items.len() == 1 {
                state.table.selected = 0;
                crate::finder::navigation::enter_loaded(app);
            } else {
                state.inventory.fail(
                    &mut state.table,
                    format!("Resource not found or not accessible: {target}"),
                );
            }
        }
    }
    changed
}
