//! DynamoDB tables root inventory as a tab-owned background read (see
//! `crate::inventory_read`). An entered table keeps its existing loaders.
use crate::{
    app::{App, Service},
    async_read::ReadScope,
    keymap::{Action, Mode},
};
use rusticity_core::DynamoDbClient;
use std::time::Duration;

/// ListTables plus per-table DescribeTable; bounded so a hung read surfaces.
const TIMEOUT: Duration = Duration::from_secs(60);

fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "inventory:dynamodb")
}

pub fn active(app: &App) -> bool {
    app.service_selected
        && app.current_service == Service::DynamoDbTables
        && app.dynamodb_state.current_table.is_none()
}

pub fn request(app: &mut App) {
    let scope = scope(app);
    let state = &mut app.dynamodb_state;
    state.inventory.request(&mut state.tables, scope);
}

/// Rows were loaded in-line by a finder or bookmark open.
pub(crate) fn mark_loaded(app: &mut App) {
    let scope = scope(app);
    app.dynamodb_state.inventory.mark_loaded(scope);
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app) {
        return false;
    }
    match (app.mode, action) {
        (Mode::Normal, Action::GoBack) => {
            let state = &mut app.dynamodb_state;
            state.inventory.cancel(&mut state.tables)
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
    let state = &mut app.dynamodb_state;
    let fetch = move || async move { DynamoDbClient::new(config).list_tables().await };
    match state
        .inventory
        .poll(&mut state.tables, &scope, "DynamoDB tables", TIMEOUT, fetch)
        .await
    {
        Some(changed) => changed,
        None => {
            // AWS context changed: no entered table may outlive its rows.
            state.current_table = None;
            true
        }
    }
}
