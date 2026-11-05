//! Lambda functions root inventory as a tab-owned background read (see
//! `crate::inventory_read`). An entered function keeps its existing loaders.
use crate::{
    app::{App, Service},
    async_read::ReadScope,
    keymap::{Action, Mode},
};
use rusticity_core::LambdaClient;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(60);

fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "inventory:lambda")
}

pub fn active(app: &App) -> bool {
    app.service_selected
        && app.current_service == Service::LambdaFunctions
        && app.lambda_state.current_function.is_none()
}

pub fn request(app: &mut App) {
    let scope = scope(app);
    let state = &mut app.lambda_state;
    state.inventory.request(&mut state.table, scope);
}

/// Rows were loaded in-line by a finder or bookmark open.
pub(crate) fn mark_loaded(app: &mut App) {
    let scope = scope(app);
    app.lambda_state.inventory.mark_loaded(scope);
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app) {
        return false;
    }
    match (app.mode, action) {
        (Mode::Normal, Action::GoBack) => {
            let state = &mut app.lambda_state;
            state.inventory.cancel(&mut state.table)
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
    let state = &mut app.lambda_state;
    let fetch = move || async move {
        LambdaClient::new(config)
            .list_functions()
            .await
            .map(crate::ui::lambda::function_rows)
    };
    match state
        .inventory
        .poll(&mut state.table, &scope, "Lambda functions", TIMEOUT, fetch)
        .await
    {
        Some(changed) => changed,
        None => {
            // AWS context changed: no entered function may outlive its rows.
            state.current_function = None;
            state.current_version = None;
            state.current_alias = None;
            true
        }
    }
}
