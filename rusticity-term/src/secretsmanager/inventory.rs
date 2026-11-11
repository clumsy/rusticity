//! Secret metadata list as a tab-owned background read (ListSecrets is bounded
//! by 30s in core). See `crate::list_read`. A finder or bookmark open enters
//! the exact secret by ARN once the list is in.
use crate::{
    app::{App, Service},
    async_read::{ReadScope, ReadStatus},
    keymap::{Action, Mode},
};
use rusticity_core::SecretsManagerClient;

fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "inventory:secretsmanager")
}

/// True on the secrets list, not an entered secret.
pub fn active(app: &App) -> bool {
    app.service_selected
        && app.current_service == Service::SecretsManagerSecrets
        && app.secretsmanager_state.detail.source.is_none()
}

/// Input-handler safe: clears the rows at once and queues the read.
pub fn request(app: &mut App) {
    let scope = scope(app);
    let state = &mut app.secretsmanager_state;
    state.pending_focus = None;
    state.inventory.request(&mut state.secrets, scope);
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app) {
        return false;
    }
    match (app.mode, action) {
        (Mode::Normal, Action::GoBack) if app.secretsmanager_state.inventory.is_busy() => {
            let state = &mut app.secretsmanager_state;
            state.pending_focus = None;
            state.inventory.cancel(&mut state.secrets);
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
    app.secretsmanager_state.inventory.mark_loaded(scope);
}

pub async fn poll(app: &mut App) -> bool {
    let scope = scope(app);
    let config = app.config.clone();
    let state = &mut app.secretsmanager_state;
    let changed = state
        .inventory
        .drive(
            &mut state.secrets,
            &scope,
            "Secret metadata read failed",
            || async move {
                SecretsManagerClient::new(config)
                    .list_secrets()
                    .await
                    .map_err(|e| format!("{e:#}"))
            },
        )
        .await;
    if changed && app.secretsmanager_state.inventory.status() == ReadStatus::Ready {
        if let Some(target) = app.secretsmanager_state.pending_focus.take() {
            let state = &mut app.secretsmanager_state;
            state.secrets.filter.clear();
            // The ARN is the identity: same-named secrets stay distinct.
            state.secrets.items.retain(|row| row.arn == target);
            if state.secrets.items.len() == 1 {
                state.secrets.selected = 0;
                crate::finder::navigation::enter_loaded(app);
            } else {
                state.inventory.fail(
                    &mut state.secrets,
                    format!("Resource not found or not accessible: {target}"),
                );
            }
        }
    }
    changed
}
