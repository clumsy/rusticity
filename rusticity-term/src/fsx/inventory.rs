//! FSx file-system inventory as a tab-owned background read, bounded by 30s.
//! See `crate::inventory_read`.
use crate::{
    app::{App, Service},
    async_read::ReadScope,
    keymap::{Action, Mode},
};
use rusticity_core::fsx::FsxFileSystem;
use rusticity_core::{AwsConfig, FsxClient};
use std::time::Duration;

const READ_TIMEOUT: Duration = Duration::from_secs(30);

fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "inventory:fsx")
}

pub fn active(app: &App) -> bool {
    app.service_selected && app.current_service == Service::FsxFileSystems
}

/// Rows of one file-systems read. Shared by the tab-owned poll and the inline
/// finder/bookmark loader.
pub(crate) async fn fetch(config: AwsConfig) -> anyhow::Result<Vec<FsxFileSystem>> {
    FsxClient::new(config).list_file_systems().await
}

/// Input-handler safe: clears the rows at once and queues the read.
pub fn request(app: &mut App) {
    let scope = scope(app);
    let state = &mut app.fsx_state;
    state.inventory.request(&mut state.file_systems, scope);
}

/// Rows came from an in-line read for `scope` (finder or bookmark open).
pub(crate) fn mark_loaded(app: &mut App) {
    let scope = scope(app);
    app.fsx_state.inventory.mark_loaded(scope);
}

/// Refresh, retry and Esc-to-cancel for the inventory read.
pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app) {
        return false;
    }
    match (app.mode, action) {
        (Mode::Normal, Action::GoBack)
            if app.fsx_state.current_file_system.is_none() && app.fsx_state.inventory.is_busy() =>
        {
            let state = &mut app.fsx_state;
            state.inventory.cancel(&mut state.file_systems);
            true
        }
        (Mode::Normal, Action::Refresh) | (Mode::ErrorModal, Action::RetryLoad)
            if app.fsx_state.current_file_system.is_none() =>
        {
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
    let config = app.aws.config.clone();
    let state = &mut app.fsx_state;
    state
        .inventory
        .poll(
            &mut state.file_systems,
            &scope,
            "File systems",
            READ_TIMEOUT,
            || fetch(config),
        )
        .await
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Service, Tab};
    use crate::async_read::ReadStatus;
    use crate::tab_runtime::TabId;

    fn app_with_rows(rows: usize) -> App {
        let mut app = crate::app::App::new_without_client("t".into(), Some("us-east-1".into()));
        app.service_selected = true;
        app.current_service = Service::FsxFileSystems;
        app.tabs.push(Tab {
            id: TabId::new(),
            service: Service::FsxFileSystems,
            title: "FSx › File Systems".into(),
            breadcrumb: "FSx › File Systems".into(),
        });
        for i in 0..rows {
            app.fsx_state.file_systems.items.push(FsxFileSystem {
                file_system_id: format!("fs-{i}"),
                ..Default::default()
            });
        }
        app
    }

    async fn settle(app: &mut App) -> Option<bool> {
        let scope = scope(app);
        let config = app.aws.config.clone();
        tokio::task::yield_now().await;
        app.fsx_state
            .inventory
            .poll(
                &mut app.fsx_state.file_systems,
                &scope,
                "File systems",
                READ_TIMEOUT,
                || fetch(config),
            )
            .await
    }

    #[test]
    fn request_clears_rows_and_queues_the_read() {
        let mut app = app_with_rows(2);
        request(&mut app);
        assert!(app.fsx_state.file_systems.items.is_empty());
        assert!(matches!(
            app.fsx_state.load_status(),
            ReadStatus::Loading { .. }
        ));
    }

    #[tokio::test]
    async fn failure_keeps_rows_empty_and_cancel_reports_retry() {
        let mut app = app_with_rows(2);
        request(&mut app);
        assert!(app
            .fsx_state
            .inventory
            .start_for_test(async { Err("AccessDenied".to_string()) }));
        assert_eq!(settle(&mut app).await, Some(true));
        assert!(app.fsx_state.file_systems.items.is_empty());
        assert_eq!(app.fsx_state.load_status(), ReadStatus::Failed);

        request(&mut app);
        app.mode = Mode::Normal;
        assert!(handle_action(&mut app, &Action::GoBack));
        assert_eq!(app.fsx_state.load_status(), ReadStatus::Failed);
        assert!(app.fsx_state.file_systems.items.is_empty());
    }
}
