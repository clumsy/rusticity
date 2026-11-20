//! CloudFormation stack inventory as a tab-owned background read, bounded by
//! 30s. See `crate::inventory_read`. The read is scoped to the View-nested
//! toggle: a result is accepted only for the setting and AWS context it was
//! requested in. Stack detail tabs keep their legacy inline loaders.
use crate::{
    app::{App, Service},
    async_read::ReadScope,
    keymap::{Action, Mode},
};
use rusticity_core::{AwsConfig, CloudFormationClient};
use std::time::Duration;

const READ_TIMEOUT: Duration = Duration::from_secs(30);

fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(
        app,
        format!("inventory:cfn:nested={}", app.cfn_state.view_nested),
    )
}

pub fn active(app: &App) -> bool {
    app.service_selected && app.current_service == Service::CloudFormationStacks
}

/// Rows of one stacks read for the nested setting requested. Shared by the
/// tab-owned poll and the inline finder/bookmark loader.
pub(crate) async fn fetch(config: AwsConfig, nested: bool) -> anyhow::Result<Vec<super::Stack>> {
    let mut rows: Vec<_> = CloudFormationClient::new(config)
        .list_stacks(nested)
        .await?
        .into_iter()
        .map(|s| super::Stack {
            name: s.name,
            stack_id: s.stack_id,
            status: s.status,
            created_time: s.created_time,
            updated_time: s.updated_time,
            deleted_time: s.deleted_time,
            drift_status: s.drift_status,
            last_drift_check_time: s.last_drift_check_time,
            status_reason: s.status_reason,
            description: s.description,
            detailed_status: String::new(),
            root_stack: s.root_stack,
            parent_stack: s.parent_stack,
            termination_protection: false,
            iam_role: String::new(),
            tags: Vec::new(),
            stack_policy: String::new(),
            rollback_monitoring_time: String::new(),
            rollback_alarms: Vec::new(),
            notification_arns: Vec::new(),
        })
        .collect();
    // Newest first, as the console lists stacks.
    rows.sort_by(|a, b| b.created_time.cmp(&a.created_time));
    Ok(rows)
}

/// Input-handler safe: clears the rows at once and queues the read.
pub fn request(app: &mut App) {
    let scope = scope(app);
    let state = &mut app.cfn_state;
    state.expanded_items.clear();
    state.inventory.request(&mut state.table, scope);
}

/// Rows came from an in-line read for `scope` (finder or bookmark open).
pub(crate) fn mark_loaded(app: &mut App) {
    let scope = scope(app);
    app.cfn_state.inventory.mark_loaded(scope);
}

/// Refresh, retry and Esc-to-cancel for the inventory read.
pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app) {
        return false;
    }
    match (app.mode, action) {
        (Mode::Normal, Action::GoBack)
            if app.cfn_state.current_stack.is_none() && app.cfn_state.inventory.is_busy() =>
        {
            let state = &mut app.cfn_state;
            state.inventory.cancel(&mut state.table);
            true
        }
        (Mode::Normal, Action::Refresh) | (Mode::ErrorModal, Action::RetryLoad)
            if app.cfn_state.current_stack.is_none() =>
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
    let nested = app.cfn_state.view_nested;
    let state = &mut app.cfn_state;
    state
        .inventory
        .poll(&mut state.table, &scope, "Stacks", READ_TIMEOUT, || {
            fetch(config, nested)
        })
        .await
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Service, Tab};
    use crate::async_read::ReadStatus;
    use crate::tab_runtime::TabId;

    fn stack(name: &str) -> super::super::Stack {
        super::super::Stack {
            name: name.into(),
            stack_id: String::new(),
            status: String::new(),
            created_time: String::new(),
            updated_time: String::new(),
            deleted_time: String::new(),
            drift_status: String::new(),
            last_drift_check_time: String::new(),
            status_reason: String::new(),
            description: String::new(),
            detailed_status: String::new(),
            root_stack: String::new(),
            parent_stack: String::new(),
            termination_protection: false,
            iam_role: String::new(),
            tags: Vec::new(),
            stack_policy: String::new(),
            rollback_monitoring_time: String::new(),
            rollback_alarms: Vec::new(),
            notification_arns: Vec::new(),
        }
    }

    fn app_with_rows(rows: usize) -> App {
        let mut app = crate::app::App::new_without_client("t".into(), Some("us-east-1".into()));
        app.service_selected = true;
        app.current_service = Service::CloudFormationStacks;
        app.tabs.push(Tab {
            id: TabId::new(),
            service: Service::CloudFormationStacks,
            title: "CloudFormation › Stacks".into(),
            breadcrumb: "CloudFormation › Stacks".into(),
        });
        for i in 0..rows {
            app.cfn_state.table.items.push(stack(&format!("s-{i}")));
        }
        app
    }

    async fn settle(app: &mut App) -> Option<bool> {
        let scope = scope(app);
        let config = app.aws.config.clone();
        let nested = app.cfn_state.view_nested;
        tokio::task::yield_now().await;
        app.cfn_state
            .inventory
            .poll(
                &mut app.cfn_state.table,
                &scope,
                "Stacks",
                READ_TIMEOUT,
                || fetch(config, nested),
            )
            .await
    }

    #[test]
    fn request_clears_rows_and_nested_expansions() {
        let mut app = app_with_rows(2);
        app.cfn_state.expanded_items.insert("s-0".into());
        request(&mut app);
        assert!(app.cfn_state.table.items.is_empty());
        assert!(app.cfn_state.expanded_items.is_empty());
        assert!(matches!(
            app.cfn_state.load_status(),
            ReadStatus::Loading { .. }
        ));
    }

    #[tokio::test]
    async fn failure_and_a_nested_toggle_never_restore_old_rows() {
        let mut app = app_with_rows(2);
        request(&mut app);
        assert!(app
            .cfn_state
            .inventory
            .start_for_test(async { Err("AccessDenied".to_string()) }));
        assert_eq!(settle(&mut app).await, Some(true));
        assert!(app.cfn_state.table.items.is_empty());
        assert_eq!(app.cfn_state.load_status(), ReadStatus::Failed);

        // Toggling View nested changes the scope: the queued read is dropped.
        request(&mut app);
        app.cfn_state.view_nested = !app.cfn_state.view_nested;
        app.cfn_state.table.items.push(stack("late"));
        let scope = scope(&app);
        let config = app.aws.config.clone();
        let nested = app.cfn_state.view_nested;
        assert_eq!(
            app.cfn_state
                .inventory
                .poll(
                    &mut app.cfn_state.table,
                    &scope,
                    "Stacks",
                    READ_TIMEOUT,
                    || fetch(config, nested),
                )
                .await,
            None
        );
        assert!(app.cfn_state.table.items.is_empty());
    }
}
