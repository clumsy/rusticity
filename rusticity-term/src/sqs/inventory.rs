//! SQS queues root inventory as a tab-owned background read (see
//! `crate::inventory_read`). An entered queue keeps its existing loaders.
use crate::{
    app::{App, Service},
    async_read::ReadScope,
    keymap::{Action, Mode},
};
use rusticity_core::SqsClient;
use std::time::Duration;

/// ListQueues plus per-queue GetQueueAttributes can take a while on large
/// accounts; a hung read still ends as a visible error, never a spinner.
const TIMEOUT: Duration = Duration::from_secs(60);

fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "inventory:sqs")
}

pub fn active(app: &App) -> bool {
    app.service_selected
        && app.current_service == Service::SqsQueues
        && app.sqs_state.current_queue.is_none()
}

pub fn request(app: &mut App) {
    let scope = scope(app);
    let state = &mut app.sqs_state;
    state.inventory.request(&mut state.queues, scope);
}

/// Rows were loaded in-line by a finder or bookmark open.
pub(crate) fn mark_loaded(app: &mut App) {
    let scope = scope(app);
    app.sqs_state.inventory.mark_loaded(scope);
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app) {
        return false;
    }
    match (app.mode, action) {
        (Mode::Normal, Action::GoBack) => {
            let state = &mut app.sqs_state;
            state.inventory.cancel(&mut state.queues)
        }
        (Mode::Normal, Action::Refresh) | (Mode::ErrorModal, Action::RetryLoad) => {
            crate::inventory::dismiss_error(app);
            request(app);
            true
        }
        _ => false,
    }
}

pub async fn poll(app: &mut App) -> bool {
    let scope = scope(app);
    let config = app.config.clone();
    let state = &mut app.sqs_state;
    let fetch = move || async move {
        SqsClient::new(config)
            .list_queues("")
            .await
            .map(crate::ui::sqs::queue_rows)
    };
    match state
        .inventory
        .poll(&mut state.queues, &scope, "SQS queues", TIMEOUT, fetch)
        .await
    {
        Some(changed) => changed,
        None => {
            // AWS context changed: no entered queue may outlive its rows.
            state.current_queue = None;
            true
        }
    }
}
