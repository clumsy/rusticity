//! RDS database list as a tab-owned background read. The optional columns
//! that are visible decide which enrichment reads run; enabling one queues a
//! fresh read, hiding one never reads. See `crate::list_read`.
use crate::{
    app::{App, Service},
    async_read::{ReadScope, ReadStatus},
    keymap::{Action, Mode},
};
use rusticity_core::RdsClient;

fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "inventory:rds")
}

/// True on the Databases list, not an entered database.
pub fn active(app: &App) -> bool {
    app.service_selected
        && app.current_service == Service::RdsInstances
        && app.rds_state.detail.source.is_none()
}

/// Input-handler safe: clears the rows at once and queues the read.
pub fn request(app: &mut App) {
    let scope = scope(app);
    let state = &mut app.rds_state;
    state.pending_focus = None;
    state.inventory.request(&mut state.instances, scope);
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app) {
        return false;
    }
    match (app.mode, action) {
        (Mode::Normal, Action::GoBack) if app.rds_state.inventory.is_busy() => {
            let state = &mut app.rds_state;
            state.pending_focus = None;
            state.inventory.cancel(&mut state.instances);
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
    app.rds_state.inventory.mark_loaded(scope);
}

pub async fn poll(app: &mut App) -> bool {
    let scope = scope(app);
    let config = app.config.clone();
    let options =
        crate::rds::instance::Column::inventory_options(&app.rds_state.visible_column_ids);
    let state = &mut app.rds_state;
    let changed = state
        .inventory
        .drive(
            &mut state.instances,
            &scope,
            "RDS databases read failed",
            || async move {
                RdsClient::new(config)
                    .list_instances_with_options(options)
                    .await
                    .map_err(|e| format!("{e:#}"))
            },
        )
        .await;
    if changed {
        apply_pending_focus(app);
    }
    changed
}

/// A finder or bookmark open: select the exact database once the list is in,
/// then enter it. A database that is gone is an error, never another row.
fn apply_pending_focus(app: &mut App) {
    let status = app.rds_state.inventory.status();
    if matches!(status, ReadStatus::Loading { .. } | ReadStatus::Idle) {
        return;
    }
    let Some(target) = app.rds_state.pending_focus.take() else {
        return;
    };
    if status == ReadStatus::Failed {
        return;
    }
    let state = &mut app.rds_state;
    state.instances.filter.clear();
    state
        .instances
        .items
        .retain(|row| row.arn == target || row.identifier == target);
    if state.instances.items.len() != 1 {
        state.inventory.fail(
            &mut state.instances,
            format!("Resource not found or not accessible: {target}"),
        );
        return;
    }
    state.instances.selected = 0;
    crate::finder::navigation::enter_loaded(app);
}
