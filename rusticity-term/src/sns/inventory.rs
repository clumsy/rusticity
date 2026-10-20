//! Root SNS Topics inventory as an owned background read. `request` clears the
//! pane and queues a read; `poll` starts it in a Tokio task bounded by a whole-
//! read timeout and accepts the result only when it still matches the owner's
//! AWS context. A failed or superseded read never leaves stale rows on screen.
#[cfg(test)]
use crate::async_read::ReadStatus;
use crate::{
    app::{App, Service},
    async_read::ReadScope,
    keymap::{Action, Mode},
};
use rusticity_core::{SnsClient, SnsTopic};
use std::time::Duration;

/// Whole-read budget. SNS list_topics is paginated and otherwise unbounded;
/// the guard turns a hung network read into a surfaced error, never a hang.
const READ_TIMEOUT: Duration = Duration::from_secs(30);

/// Scope is AWS identity only — the root inventory has no entered resource.
fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "inventory:sns")
}

/// True while this session is viewing the SNS Topics root (not an entered
/// subscriptions detail, which the subscriptions module owns).
pub fn active(app: &App) -> bool {
    app.service_selected
        && app.current_service == Service::SnsTopics
        && app.sns_state.subscriptions.topic.is_none()
}

/// Begin (or restart) the inventory read. Input-handler safe: it clears the
/// displayed rows and selection immediately, drops any stale error, and only
/// QUEUES the read — no HTTP is issued here. `poll` performs the request.
pub fn request(app: &mut App) {
    let scope = scope(app);
    let state = &mut app.sns_state;
    state.topics.items.clear();
    state.topics.reset();
    state.topics.loading = true;
    state.inventory_error = None;
    state.inventory.request(scope);
}

/// Handle only the root-inventory Refresh / RetryLoad. Entered detail and all
/// table navigation/filter input are handled elsewhere. Returns true when the
/// action was consumed.
pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app) {
        return false;
    }
    match (app.mode, action) {
        (Mode::Normal, Action::GoBack) if app.sns_state.inventory.is_busy() => {
            let state = &mut app.sns_state;
            state.inventory.cancel();
            state.topics.items.clear();
            state.topics.loading = false;
            state.inventory_error = Some("Cancelled — Ctrl+R to retry".into());
            state.pending_focus = None;
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

/// View-facing status: Loading/Failed consume the pane, Ready permits fresh
/// rows (including a successfully empty inventory). Never inferred from count.
#[cfg(test)]
fn read_status(app: &App) -> ReadStatus {
    app.sns_state.inventory.status()
}

/// Drive one step of the queued read. Starts the Tokio task when pending,
/// harvests a finished result only if the owner's AWS context is unchanged,
/// and applies success/error atomically. Returns true when the UI changed.
pub async fn poll(app: &mut App) -> bool {
    let current_scope = scope(app);
    // A context change (profile/region/account/role) invalidates cached data;
    // the shared AsyncRead drops it so no stale rows survive the switch.
    let state = &mut app.sns_state;
    if state.inventory.invalidate_if_changed(&current_scope) {
        state.topics.items.clear();
        state.topics.reset();
        state.topics.loading = false;
        state.inventory_error = None;
        state.pending_focus = None;
        return true;
    }

    let mut changed = false;
    if state.inventory.is_pending() {
        let config = app.config.clone();
        let state = &mut app.sns_state;
        if state.inventory.start(async move {
            match tokio::time::timeout(READ_TIMEOUT, SnsClient::new(config).list_topics()).await {
                Ok(result) => result.map_err(|e| format!("{e:#}")),
                Err(_) => Err("SNS topics read timed out after 30s".to_string()),
            }
        }) {
            changed = true;
        }
    }

    if let Some(result) = app.sns_state.inventory.take_ready(&current_scope).await {
        let state = &mut app.sns_state;
        state.topics.loading = false;
        match result {
            Ok(rows) => {
                state.topics.items = rows;
                state.topics.reset();
                state.inventory_error = None;
                apply_pending_focus(app);
            }
            Err(error) => {
                // Clear stale rows; surface inline, never an ErrorModal focus steal.
                state.topics.items.clear();
                state.topics.reset();
                state.pending_focus = None;
                state.inventory_error =
                    Some(sanitize(error.into_message("SNS topics read failed")));
            }
        }
        changed = true;
    }
    changed
}

/// Select the row a finder navigation queued, once the fresh rows are in.
fn apply_pending_focus(app: &mut App) {
    let Some(target) = app.sns_state.pending_focus.take() else {
        return;
    };
    if let Some(index) = crate::ui::sns::filtered_topics(app)
        .iter()
        .position(|topic: &&SnsTopic| topic.arn == target || topic.name == target)
    {
        app.sns_state.topics.selected = index;
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
