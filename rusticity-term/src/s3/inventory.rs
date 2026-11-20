//! S3 bucket inventory as a tab-owned background read, bounded by 30s. See
//! `crate::inventory_read`. The bucket list spans every Region of the account,
//! so a result is accepted for the AWS context it was requested in; per-bucket
//! object previews stay on the legacy inline sync pass.
use crate::{
    app::{App, Service},
    async_read::ReadScope,
    keymap::{Action, Mode},
};
use rusticity_core::{AwsConfig, S3Client};
use std::time::Duration;

const READ_TIMEOUT: Duration = Duration::from_secs(30);

fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "inventory:s3")
}

pub fn active(app: &App) -> bool {
    app.service_selected && app.current_service == Service::S3Buckets
}

/// Rows of one buckets read. Shared by the tab-owned poll and the inline
/// finder/bookmark loader.
pub(crate) async fn fetch(config: AwsConfig) -> anyhow::Result<Vec<super::Bucket>> {
    Ok(S3Client::new(config)
        .list_buckets()
        .await?
        .into_iter()
        .map(|(name, region, creation_date)| super::Bucket {
            name,
            region,
            creation_date,
        })
        .collect())
}

/// Input-handler safe: clears the rows and derived tree state at once and
/// queues the read.
pub fn request(app: &mut App) {
    let scope = scope(app);
    let state = &mut app.s3_state;
    state.bucket_preview.clear();
    state.bucket_errors.clear();
    state.prefix_preview.clear();
    state.expanded_prefixes.clear();
    state.sync_pending = false;
    state.inventory.request(&mut state.buckets, scope);
}

/// Rows came from an in-line read for `scope` (finder or bookmark open).
pub(crate) fn mark_loaded(app: &mut App) {
    let scope = scope(app);
    app.s3_state.inventory.mark_loaded(scope);
}

/// Refresh, retry and Esc-to-cancel for the inventory read.
pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app) {
        return false;
    }
    match (app.mode, action) {
        (Mode::Normal, Action::GoBack)
            if app.s3_state.current_bucket.is_none() && app.s3_state.inventory.is_busy() =>
        {
            let state = &mut app.s3_state;
            state.inventory.cancel(&mut state.buckets);
            true
        }
        (Mode::Normal, Action::Refresh) | (Mode::ErrorModal, Action::RetryLoad)
            if app.s3_state.current_bucket.is_none() =>
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
    let state = &mut app.s3_state;
    let Some(changed) = state
        .inventory
        .poll(&mut state.buckets, &scope, "Buckets", READ_TIMEOUT, || {
            fetch(config)
        })
        .await
    else {
        // AWS context changed: the derived preview tree is stale as well.
        state.bucket_preview.clear();
        state.bucket_errors.clear();
        state.prefix_preview.clear();
        state.expanded_prefixes.clear();
        return true;
    };
    if changed && app.s3_state.load_status() == crate::async_read::ReadStatus::Ready {
        app.s3_state.sync_pending = true;
    }
    changed
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
        app.current_service = Service::S3Buckets;
        app.tabs.push(Tab {
            id: TabId::new(),
            service: Service::S3Buckets,
            title: "S3 › Buckets".into(),
            breadcrumb: "S3 › Buckets".into(),
        });
        for i in 0..rows {
            app.s3_state.buckets.items.push(super::super::Bucket {
                name: format!("b-{i}"),
                region: "us-east-1".into(),
                creation_date: String::new(),
            });
        }
        app
    }

    async fn settle(app: &mut App) -> Option<bool> {
        let scope = scope(app);
        for _ in 0..50 {
            let config = app.aws.config.clone();
            tokio::task::yield_now().await;
            let stepped = app
                .s3_state
                .inventory
                .poll(
                    &mut app.s3_state.buckets,
                    &scope,
                    "Buckets",
                    READ_TIMEOUT,
                    || fetch(config),
                )
                .await;
            if stepped == Some(true) {
                return Some(true);
            }
        }
        Some(false)
    }

    #[test]
    fn request_clears_rows_and_derived_tree_state() {
        let mut app = app_with_rows(2);
        app.s3_state.expanded_prefixes.insert("b-0".into());
        app.s3_state.bucket_preview.insert("b-0".into(), Vec::new());
        app.s3_state.sync_pending = true;
        request(&mut app);
        assert!(app.s3_state.buckets.items.is_empty());
        assert!(app.s3_state.expanded_prefixes.is_empty());
        assert!(app.s3_state.bucket_preview.is_empty());
        assert!(!app.s3_state.sync_pending);
        assert!(matches!(
            app.s3_state.load_status(),
            ReadStatus::Loading { .. }
        ));
    }

    #[tokio::test]
    async fn failure_and_context_change_never_restore_old_rows() {
        let mut app = app_with_rows(2);
        request(&mut app);
        assert!(app
            .s3_state
            .inventory
            .start_for_test(async { Err("AccessDenied".to_string()) }));
        assert_eq!(settle(&mut app).await, Some(true));
        assert!(app.s3_state.buckets.items.is_empty());
        assert_eq!(app.s3_state.load_status(), ReadStatus::Failed);
        assert!(app
            .s3_state
            .inventory
            .error()
            .unwrap()
            .contains("AccessDenied"));

        app.s3_state.buckets.items.push(super::super::Bucket {
            name: "late".into(),
            region: "us-east-1".into(),
            creation_date: String::new(),
        });
        app.aws.config.region = "eu-west-1".into();
        let scope = scope(&app);
        let config = app.aws.config.clone();
        assert_eq!(
            app.s3_state
                .inventory
                .poll(
                    &mut app.s3_state.buckets,
                    &scope,
                    "Buckets",
                    READ_TIMEOUT,
                    || fetch(config),
                )
                .await,
            None
        );
        assert!(app.s3_state.buckets.items.is_empty());
    }

    #[tokio::test]
    async fn a_fresh_read_marks_the_legacy_sync_pass_pending() {
        let mut app = app_with_rows(0);
        request(&mut app);
        assert!(!app.s3_state.sync_pending);
        // A successful read is simulated without touching the network.
        assert!(app.s3_state.inventory.start_for_test(async {
            Ok(vec![super::super::Bucket {
                name: "b".into(),
                region: "us-east-1".into(),
                creation_date: String::new(),
            }])
        }));
        // The module poll harvests the finished read and arms the sync pass.
        let mut armed = false;
        for _ in 0..50 {
            tokio::task::yield_now().await;
            if poll(&mut app).await {
                armed = true;
                break;
            }
        }
        assert!(armed);
        assert_eq!(
            app.s3_state.load_status(),
            crate::async_read::ReadStatus::Ready
        );
        assert_eq!(app.s3_state.buckets.items.len(), 1);
        assert!(app.s3_state.sync_pending);
    }

    #[test]
    fn esc_cancels_only_the_list_read() {
        let mut app = app_with_rows(0);
        app.mode = Mode::Normal;
        request(&mut app);
        assert!(handle_action(&mut app, &Action::GoBack));
        assert_eq!(app.s3_state.load_status(), ReadStatus::Failed);

        let mut app = app_with_rows(0);
        app.mode = Mode::Normal;
        app.s3_state.current_bucket = Some("b".into());
        request(&mut app);
        assert!(!handle_action(&mut app, &Action::GoBack));
    }
}
