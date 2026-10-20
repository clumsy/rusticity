//! Root KMS keys inventory as an owned background read. `request` clears the
//! pane and queues a read; `poll` starts it in a Tokio task bounded by a whole-
//! read timeout — important here because the core `list_keys` (list keys, then
//! aliases, then describe each) is otherwise unbounded — and accepts the result
//! only when it still matches the owner's AWS context. A failed or superseded
//! read never leaves stale rows on screen. Core keys are mapped to the UI key
//! type with the same field mapping as `App::load_kms_keys`.
#[cfg(test)]
use crate::async_read::ReadStatus;
use crate::{
    app::{App, Service},
    async_read::ReadScope,
    keymap::{Action, Mode},
    kms::key::Key as UiKey,
};
use rusticity_core::KmsClient;
use std::time::Duration;

/// Whole-read budget. Guards the unbounded multi-call `list_keys` so a hung
/// read surfaces as an error rather than freezing the inventory.
const READ_TIMEOUT: Duration = Duration::from_secs(30);

fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "inventory:kms")
}

/// True while this session is viewing the KMS keys root (not an entered key
/// detail, which the kms::detail module owns).
pub fn active(app: &App) -> bool {
    app.service_selected
        && app.current_service == Service::KmsKeys
        && !crate::kms::detail::active(app)
}

/// Begin (or restart) the inventory read: clear rows + selection, drop any
/// error, mark loading and QUEUE the read. No HTTP here — `poll` issues it.
pub fn request(app: &mut App) {
    let scope = scope(app);
    let state = &mut app.kms_state;
    state.keys.items.clear();
    state.keys.reset();
    state.keys.loading = true;
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
        (Mode::Normal, Action::GoBack) if app.kms_state.inventory.is_busy() => {
            let state = &mut app.kms_state;
            state.inventory.cancel();
            state.keys.items.clear();
            state.keys.loading = false;
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
    app.kms_state.inventory.status()
}

pub async fn poll(app: &mut App) -> bool {
    let current_scope = scope(app);
    let state = &mut app.kms_state;
    if state.inventory.invalidate_if_changed(&current_scope) {
        state.keys.items.clear();
        state.keys.reset();
        state.keys.loading = false;
        state.inventory_error = None;
        state.pending_focus = None;
        state.open_after_load = false;
        return true;
    }

    let mut changed = false;
    if state.inventory.is_pending() {
        let config = app.config.clone();
        let state = &mut app.kms_state;
        if state.inventory.start(async move {
            match tokio::time::timeout(READ_TIMEOUT, KmsClient::new(config).list_keys()).await {
                Ok(result) => result
                    .map(|keys| keys.into_iter().map(to_ui_key).collect::<Vec<UiKey>>())
                    .map_err(|e| format!("{e:#}")),
                Err(_) => Err("KMS keys read timed out after 30s".to_string()),
            }
        }) {
            changed = true;
        }
    }

    if let Some(result) = app.kms_state.inventory.take_ready(&current_scope).await {
        let state = &mut app.kms_state;
        state.keys.loading = false;
        match result {
            Ok(rows) => {
                state.keys.items = rows;
                state.keys.reset();
                state.inventory_error = None;
                apply_pending_focus(app);
            }
            Err(error) => {
                state.keys.items.clear();
                state.keys.reset();
                state.pending_focus = None;
                state.open_after_load = false;
                state.inventory_error = Some(sanitize(error.into_message("KMS keys read failed")));
            }
        }
        changed = true;
    }
    changed
}

/// Core -> UI key mapping, identical field-by-field to `App::load_kms_keys` so
/// the async path and the legacy blocking loader populate the same rows.
fn to_ui_key(k: rusticity_core::kms::KmsKey) -> UiKey {
    UiKey {
        key_id: k.key_id,
        key_arn: k.key_arn,
        alias: k.alias,
        description: k.description,
        key_state: k.key_state,
        key_usage: k.key_usage,
        key_spec: k.key_spec,
        key_manager: k.key_manager,
        creation_date: k.creation_date,
        expiration_date: k.expiration_date,
        deletion_date: k.deletion_date,
        custom_key_store_id: k.custom_key_store_id,
        origin: k.origin,
        multi_region: k.multi_region,
        enabled: k.enabled,
    }
}

fn apply_pending_focus(app: &mut App) {
    let open = std::mem::take(&mut app.kms_state.open_after_load);
    let Some(target) = app.kms_state.pending_focus.take() else {
        return;
    };
    if open {
        app.kms_state.keys.filter.clear();
    }
    if let Some(key) = app
        .kms_state
        .keys
        .items
        .iter()
        .find(|key| key.key_id == target || key.key_arn == target)
    {
        app.kms_state.tab = if key.key_manager.eq_ignore_ascii_case("aws") {
            crate::ui::kms::Tab::AwsManaged
        } else {
            crate::ui::kms::Tab::CustomerManaged
        };
    }
    if let Some(index) = crate::ui::kms::filtered_kms_keys(app)
        .iter()
        .position(|key: &&UiKey| key.key_id == target || key.key_arn == target)
    {
        app.kms_state.keys.selected = index;
        if open {
            crate::finder::navigation::enter_loaded(app);
        }
    } else if open {
        app.kms_state.keys.items.clear();
        app.kms_state.inventory_error =
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
