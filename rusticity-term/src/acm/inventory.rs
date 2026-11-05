//! ACM certificate inventory as a tab-owned background read. `request` clears
//! the rows and queues a read; `poll` starts it and accepts the result only
//! while the owner's AWS context is unchanged. A failed, cancelled or
//! superseded read never leaves older rows on screen.
use crate::{
    app::{App, Service},
    async_read::ReadScope,
    keymap::{Action, Mode},
};
use rusticity_core::AcmClient;

fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "inventory:acm")
}

pub fn active(app: &App) -> bool {
    app.service_selected && app.current_service == Service::AcmCertificates
}

/// Input-handler safe: clears rows at once and only queues the read.
pub fn request(app: &mut App) {
    let scope = scope(app);
    let state = &mut app.acm_state;
    state.certificates.items.clear();
    state.certificates.reset();
    state.error = None;
    state.inventory.request(scope);
}

/// Refresh, retry and Esc-to-cancel for the inventory read.
pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app) {
        return false;
    }
    match (app.mode, action) {
        (Mode::Normal, Action::GoBack) if app.acm_state.inventory.is_busy() => {
            let state = &mut app.acm_state;
            state.inventory.cancel();
            state.certificates.items.clear();
            state.error = Some("Cancelled — Ctrl+R to retry".into());
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

/// Start a queued read, harvest a finished one. Returns true when the UI changed.
pub async fn poll(app: &mut App) -> bool {
    let current = scope(app);
    let state = &mut app.acm_state;
    // A profile/Region/account/role change drops the rows with no late repaint.
    if state.inventory.invalidate_if_changed(&current) {
        state.certificates.items.clear();
        state.certificates.reset();
        state.error = None;
        return true;
    }
    let mut changed = false;
    if state.inventory.is_pending() {
        let config = app.config.clone();
        changed = app.acm_state.inventory.start(async move {
            AcmClient::new(config)
                .list_certificates()
                .await
                .map_err(|e| format!("{e:#}"))
        });
    }
    if let Some(result) = app.acm_state.inventory.take_ready(&current).await {
        let state = &mut app.acm_state;
        state.certificates.items.clear();
        state.certificates.reset();
        match result {
            Ok(rows) => {
                state.certificates.items = rows;
                state.error = None;
            }
            Err(error) => {
                state.error = Some(sanitize(error.into_message("ACM certificates read failed")))
            }
        }
        changed = true;
    }
    changed
}

fn sanitize(message: String) -> String {
    message
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}
