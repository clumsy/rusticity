//! VPC list as a tab-owned background read, bounded by 30s. See
//! `crate::list_read`. VPCs have no detail view, so a finder open filters the
//! list to the exact VPC once it is in.
use crate::{
    app::{App, Service},
    async_read::{ReadScope, ReadStatus},
    keymap::{Action, Mode},
};
use rusticity_core::VpcClient;
use std::time::Duration;

const READ_TIMEOUT: Duration = Duration::from_secs(30);

fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "inventory:vpc")
}

pub fn active(app: &App) -> bool {
    app.service_selected && app.current_service == Service::VpcVpcs
}

/// Input-handler safe: clears the rows at once and queues the read.
pub fn request(app: &mut App) {
    let scope = scope(app);
    let state = &mut app.vpc_state;
    state.pending_focus = None;
    state.inventory.request(&mut state.vpcs, scope);
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app) {
        return false;
    }
    match (app.mode, action) {
        (Mode::Normal, Action::GoBack) if app.vpc_state.inventory.is_busy() => {
            let state = &mut app.vpc_state;
            state.pending_focus = None;
            state.inventory.cancel(&mut state.vpcs);
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

/// Tests: the rows in the table count as a fresh list read.
#[cfg(test)]
pub(crate) fn mark_loaded(app: &mut App) {
    let scope = scope(app);
    app.vpc_state.inventory.mark_loaded(scope);
}

pub async fn poll(app: &mut App) -> bool {
    let scope = scope(app);
    let config = app.config.clone();
    let state = &mut app.vpc_state;
    let changed = state
        .inventory
        .drive(&mut state.vpcs, &scope, "VPCs read failed", || async move {
            match tokio::time::timeout(
                READ_TIMEOUT,
                VpcClient::new(config).list_vpcs_with_properties(),
            )
            .await
            {
                Ok(result) => result.map_err(|e| format!("{e:#}")),
                Err(_) => Err(format!("VPCs read timed out after {READ_TIMEOUT:?}")),
            }
        })
        .await;
    if changed && app.vpc_state.inventory.status() == ReadStatus::Ready {
        if let Some(target) = app.vpc_state.pending_focus.take() {
            if app.vpc_state.vpcs.items.iter().any(|v| v.vpc_id == target) {
                app.vpc_state.vpcs.filter = target.clone();
                let selected = crate::ui::vpc::filtered_vpcs(app)
                    .iter()
                    .position(|v| v.vpc_id == target)
                    .unwrap_or(0);
                app.vpc_state.vpcs.selected = selected;
            } else {
                let state = &mut app.vpc_state;
                state.inventory.fail(
                    &mut state.vpcs,
                    format!("Resource not found or not accessible: {target}"),
                );
            }
        }
    }
    changed
}
