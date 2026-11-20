//! EFS file-system inventory as a tab-owned background read, bounded by 30s.
//! See `crate::inventory_read`.
use crate::{
    app::{App, Service},
    async_read::ReadScope,
    keymap::{Action, Mode},
};
use rusticity_core::{AwsConfig, EfsClient};
use std::time::Duration;

const READ_TIMEOUT: Duration = Duration::from_secs(30);

fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "inventory:efs")
}

pub fn active(app: &App) -> bool {
    app.service_selected && app.current_service == Service::EfsFileSystems
}

/// Rows of one file-systems read. Shared by the tab-owned poll and the inline
/// finder/bookmark loader.
pub(crate) async fn fetch(config: AwsConfig) -> anyhow::Result<Vec<super::fs::FileSystem>> {
    Ok(EfsClient::new(config)
        .list_file_systems()
        .await?
        .into_iter()
        .map(|f| super::fs::FileSystem {
            file_system_id: f.file_system_id,
            file_system_arn: f.file_system_arn,
            name: f.name,
            creation_token: f.creation_token,
            encrypted: f.encrypted,
            kms_key_id: f.kms_key_id,
            total_size: f.total_size,
            size_in_standard: f.size_in_standard,
            size_in_ia: f.size_in_ia,
            size_in_archive: f.size_in_archive,
            provisioned_throughput: f.provisioned_throughput,
            throughput_mode: f.throughput_mode,
            life_cycle_state: f.life_cycle_state,
            number_of_mount_targets: f.number_of_mount_targets,
            owner_id: f.owner_id,
            creation_time: f.creation_time,
            performance_mode: f.performance_mode,
            availability_zone: f.availability_zone,
            replication_overwrite_protection: f.replication_overwrite_protection,
            dns_name: f.dns_name,
            tags: f.tags,
            total_size_bytes: f.total_size_bytes,
            size_in_standard_bytes: f.size_in_standard_bytes,
            size_in_ia_bytes: f.size_in_ia_bytes,
            size_in_archive_bytes: f.size_in_archive_bytes,
        })
        .collect())
}

/// Input-handler safe: clears the rows at once and queues the read.
pub fn request(app: &mut App) {
    let scope = scope(app);
    let state = &mut app.efs_state;
    state.inventory.request(&mut state.file_systems, scope);
}

/// Rows came from an in-line read for `scope` (finder or bookmark open).
pub(crate) fn mark_loaded(app: &mut App) {
    let scope = scope(app);
    app.efs_state.inventory.mark_loaded(scope);
}

/// Refresh, retry and Esc-to-cancel for the inventory read.
pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app) {
        return false;
    }
    match (app.mode, action) {
        (Mode::Normal, Action::GoBack)
            if app.efs_state.current_file_system.is_none() && app.efs_state.inventory.is_busy() =>
        {
            let state = &mut app.efs_state;
            state.inventory.cancel(&mut state.file_systems);
            true
        }
        (Mode::Normal, Action::Refresh) | (Mode::ErrorModal, Action::RetryLoad)
            if app.efs_state.current_file_system.is_none() =>
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
    let state = &mut app.efs_state;
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
        app.current_service = Service::EfsFileSystems;
        app.tabs.push(Tab {
            id: TabId::new(),
            service: Service::EfsFileSystems,
            title: "EFS › File Systems".into(),
            breadcrumb: "EFS › File Systems".into(),
        });
        for i in 0..rows {
            app.efs_state
                .file_systems
                .items
                .push(super::super::fs::FileSystem {
                    file_system_id: format!("fs-{i}"),
                    name: String::new(),
                    ..Default::default()
                });
        }
        app
    }

    async fn settle(app: &mut App) -> Option<bool> {
        let scope = scope(app);
        let config = app.aws.config.clone();
        tokio::task::yield_now().await;
        app.efs_state
            .inventory
            .poll(
                &mut app.efs_state.file_systems,
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
        assert!(app.efs_state.file_systems.items.is_empty());
        assert!(matches!(
            app.efs_state.load_status(),
            ReadStatus::Loading { .. }
        ));
    }

    #[tokio::test]
    async fn failure_keeps_rows_empty_and_a_region_change_drops_them() {
        let mut app = app_with_rows(2);
        request(&mut app);
        assert!(app
            .efs_state
            .inventory
            .start_for_test(async { Err("AccessDenied".to_string()) }));
        assert_eq!(settle(&mut app).await, Some(true));
        assert!(app.efs_state.file_systems.items.is_empty());
        assert_eq!(app.efs_state.load_status(), ReadStatus::Failed);

        app.efs_state
            .file_systems
            .items
            .push(super::super::fs::FileSystem {
                file_system_id: "late".into(),
                name: String::new(),
                ..Default::default()
            });
        app.aws.config.region = "eu-west-1".into();
        let scope = scope(&app);
        let config = app.aws.config.clone();
        assert_eq!(
            app.efs_state
                .inventory
                .poll(
                    &mut app.efs_state.file_systems,
                    &scope,
                    "File systems",
                    READ_TIMEOUT,
                    || fetch(config),
                )
                .await,
            None
        );
        assert!(app.efs_state.file_systems.items.is_empty());
    }
}
