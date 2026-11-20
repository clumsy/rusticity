//! ECR repository inventory as a tab-owned background read, bounded by 30s.
//! See `crate::inventory_read`. The read is scoped to the chosen Private/Public
//! tab: a result is accepted only for the side and AWS context it was requested
//! in, so switching sides mid-read never delivers the other list.
use crate::{
    app::{App, Service},
    async_read::ReadScope,
    keymap::{Action, Mode},
};
use rusticity_core::{AwsConfig, EcrClient};
use std::time::Duration;

const READ_TIMEOUT: Duration = Duration::from_secs(30);

fn scope(app: &App) -> ReadScope {
    let side = match app.ecr_state.tab {
        crate::ui::ecr::Tab::Private => "private",
        crate::ui::ecr::Tab::Public => "public",
    };
    ReadScope::from_app(app, format!("inventory:ecr:{side}"))
}

pub fn active(app: &App) -> bool {
    app.service_selected && app.current_service == Service::EcrRepositories
}

/// Rows of one repositories read for `tab`. Shared by the tab-owned poll and
/// the inline finder/bookmark loader.
pub(crate) async fn fetch(
    config: AwsConfig,
    tab: crate::ui::ecr::Tab,
) -> anyhow::Result<Vec<super::repo::Repository>> {
    let repos = match tab {
        crate::ui::ecr::Tab::Private => EcrClient::new(config).list_private_repositories().await?,
        crate::ui::ecr::Tab::Public => EcrClient::new(config).list_public_repositories().await?,
    };
    let mut rows: Vec<_> = repos
        .into_iter()
        .map(|r| super::repo::Repository {
            name: r.name,
            uri: r.uri,
            created_at: r.created_at,
            tag_immutability: r.tag_immutability,
            encryption_type: r.encryption_type,
        })
        .collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(rows)
}

/// Input-handler safe: clears the rows at once and queues the read.
pub fn request(app: &mut App) {
    let scope = scope(app);
    let state = &mut app.ecr_state;
    state.inventory.request(&mut state.repositories, scope);
}

/// Rows came from an in-line read for `scope` (finder or bookmark open).
pub(crate) fn mark_loaded(app: &mut App) {
    let scope = scope(app);
    app.ecr_state.inventory.mark_loaded(scope);
}

/// Refresh, retry and Esc-to-cancel for the inventory read.
pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app) {
        return false;
    }
    match (app.mode, action) {
        (Mode::Normal, Action::GoBack)
            if app.ecr_state.current_repository.is_none() && app.ecr_state.inventory.is_busy() =>
        {
            let state = &mut app.ecr_state;
            state.inventory.cancel(&mut state.repositories);
            true
        }
        (Mode::Normal, Action::Refresh) | (Mode::ErrorModal, Action::RetryLoad)
            if app.ecr_state.current_repository.is_none() =>
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
    let tab = app.ecr_state.tab;
    let state = &mut app.ecr_state;
    state
        .inventory
        .poll(
            &mut state.repositories,
            &scope,
            "Repositories",
            READ_TIMEOUT,
            || fetch(config, tab),
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
        app.current_service = Service::EcrRepositories;
        app.tabs.push(Tab {
            id: TabId::new(),
            service: Service::EcrRepositories,
            title: "ECR › Repositories".into(),
            breadcrumb: "ECR › Repositories".into(),
        });
        for i in 0..rows {
            app.ecr_state
                .repositories
                .items
                .push(super::super::repo::Repository {
                    name: format!("r-{i}"),
                    uri: String::new(),
                    created_at: String::new(),
                    tag_immutability: String::new(),
                    encryption_type: String::new(),
                });
        }
        app
    }

    async fn settle(app: &mut App) -> Option<bool> {
        let scope = scope(app);
        let config = app.aws.config.clone();
        let tab = app.ecr_state.tab;
        tokio::task::yield_now().await;
        app.ecr_state
            .inventory
            .poll(
                &mut app.ecr_state.repositories,
                &scope,
                "Repositories",
                READ_TIMEOUT,
                || fetch(config, tab),
            )
            .await
    }

    #[test]
    fn request_clears_rows_for_the_chosen_side() {
        let mut app = app_with_rows(2);
        request(&mut app);
        assert!(app.ecr_state.repositories.items.is_empty());
        assert!(matches!(
            app.ecr_state.load_status(),
            ReadStatus::Loading { .. }
        ));
        // The other side has its own scope: its queued read is not this one.
        app.ecr_state.tab = crate::ui::ecr::Tab::Public;
        assert_ne!(
            scope(&app),
            ReadScope::from_app(&app, "inventory:ecr:private")
        );
    }

    #[tokio::test]
    async fn failure_keeps_the_table_empty_and_reports_the_error() {
        let mut app = app_with_rows(2);
        request(&mut app);
        assert!(app
            .ecr_state
            .inventory
            .start_for_test(async { Err("AccessDenied".to_string()) }));
        assert_eq!(settle(&mut app).await, Some(true));
        assert!(app.ecr_state.repositories.items.is_empty());
        assert_eq!(app.ecr_state.load_status(), ReadStatus::Failed);
        assert!(app
            .ecr_state
            .inventory
            .error()
            .unwrap()
            .contains("AccessDenied"));
    }

    #[tokio::test]
    async fn switching_side_mid_read_drops_the_queued_read_and_rows() {
        let mut app = app_with_rows(0);
        request(&mut app);
        app.ecr_state.tab = crate::ui::ecr::Tab::Public;
        app.ecr_state
            .repositories
            .items
            .push(super::super::repo::Repository {
                name: "late".into(),
                uri: String::new(),
                created_at: String::new(),
                tag_immutability: String::new(),
                encryption_type: String::new(),
            });
        let scope = scope(&app);
        let config = app.aws.config.clone();
        let tab = app.ecr_state.tab;
        assert_eq!(
            app.ecr_state
                .inventory
                .poll(
                    &mut app.ecr_state.repositories,
                    &scope,
                    "Repositories",
                    READ_TIMEOUT,
                    || fetch(config, tab),
                )
                .await,
            None
        );
        assert!(app.ecr_state.repositories.items.is_empty());
    }
}
