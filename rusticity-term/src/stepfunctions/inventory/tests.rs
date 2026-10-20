//! Locally-injected completions only — no credentials, no AWS. Asserts the same
//! freshness contract as the SNS inventory tests for the State Machines root.
use super::*;
use crate::keymap::Action;
use rusticity_core::StateMachineMetadata;

fn app() -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    app.current_service = Service::StepFunctionsStateMachines;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.config.account_id = "123456789012".into();
    app
}

fn machine(name: &str) -> StateMachineMetadata {
    StateMachineMetadata {
        name: name.into(),
        arn: format!("arn:aws:states:us-east-1:123456789012:stateMachine:{name}"),
        machine_type: "STANDARD".into(),
        creation_date: "2024-01-01T00:00:00Z".into(),
    }
}

async fn finish(app: &mut App, result: Result<Vec<StateMachineMetadata>, String>) -> bool {
    let scope = scope(app);
    let state = &mut app.stepfunctions_state;
    state.inventory.request(scope);
    assert!(state.inventory.start(async move { result }));
    tokio::task::yield_now().await;
    poll(app).await
}

#[test]
fn request_clears_rows_resets_selection_drops_error_and_only_queues() {
    let mut app = app();
    app.stepfunctions_state.machines.items = vec![machine("stale")];
    app.stepfunctions_state.machines.selected = 4;
    app.stepfunctions_state.inventory_error = Some("old".into());

    request(&mut app);

    assert!(app.stepfunctions_state.machines.items.is_empty());
    assert_eq!(app.stepfunctions_state.machines.selected, 0);
    assert!(app.stepfunctions_state.inventory_error.is_none());
    assert!(app.stepfunctions_state.machines.loading);
    assert!(app.stepfunctions_state.inventory.is_pending());
    assert!(!app.stepfunctions_state.inventory.is_running());
    assert!(matches!(read_status(&app), ReadStatus::Loading { .. }));
}

#[tokio::test]
async fn success_populates_rows_and_marks_ready() {
    let mut app = app();
    assert!(finish(&mut app, Ok(vec![machine("a"), machine("b")])).await);
    assert_eq!(app.stepfunctions_state.machines.items.len(), 2);
    assert!(app.stepfunctions_state.inventory_error.is_none());
    assert!(!app.stepfunctions_state.machines.loading);
    assert_eq!(read_status(&app), ReadStatus::Ready);
}

#[tokio::test]
async fn successfully_empty_inventory_is_ready_and_seeded_rows_render() {
    let mut app = app();
    assert!(finish(&mut app, Ok(vec![])).await);
    assert!(app.stepfunctions_state.machines.items.is_empty());
    assert_eq!(read_status(&app), ReadStatus::Ready);

    app.stepfunctions_state.inventory.reset();
    app.stepfunctions_state.machines.items = vec![machine("seeded")];
    assert_eq!(app.stepfunctions_state.load_status(), ReadStatus::Ready);
    app.stepfunctions_state.machines.items.clear();
    assert_eq!(app.stepfunctions_state.load_status(), ReadStatus::Idle);
}

#[tokio::test]
async fn error_clears_stale_rows_records_inline_message_and_never_opens_modal() {
    let mut app = app();
    app.stepfunctions_state.machines.items = vec![machine("stale")];
    assert!(
        finish(
            &mut app,
            Err("AccessDenied: states:ListStateMachines\u{1b}".into())
        )
        .await
    );
    assert!(app.stepfunctions_state.machines.items.is_empty());
    assert!(!app.stepfunctions_state.machines.loading);
    let message = app.stepfunctions_state.inventory_error.as_ref().unwrap();
    assert!(message.contains("AccessDenied"));
    assert!(!message.contains('\u{1b}'));
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(read_status(&app), ReadStatus::Failed);
}

#[tokio::test]
async fn retry_from_error_requeues_and_recovers() {
    let mut app = app();
    assert!(finish(&mut app, Err("transient".into())).await);
    assert_eq!(read_status(&app), ReadStatus::Failed);

    assert!(handle_action(&mut app, &Action::Refresh));
    assert!(app.stepfunctions_state.inventory.is_pending());
    assert!(app.stepfunctions_state.inventory_error.is_none());

    assert!(finish(&mut app, Ok(vec![machine("recovered")])).await);
    assert_eq!(app.stepfunctions_state.machines.items.len(), 1);
    assert_eq!(read_status(&app), ReadStatus::Ready);
}

#[tokio::test]
async fn retry_load_from_error_modal_clears_modal_and_requeues() {
    let mut app = app();
    assert!(finish(&mut app, Err("boom".into())).await);
    app.mode = Mode::ErrorModal;
    app.error_message = Some("boom".into());

    assert!(handle_action(&mut app, &Action::RetryLoad));
    assert_eq!(app.mode, Mode::Normal);
    assert!(app.error_message.is_none());
    assert!(app.stepfunctions_state.inventory.is_pending());
}

#[tokio::test]
async fn result_from_a_different_aws_context_is_rejected() {
    let mut app = app();
    let original = scope(&app);
    app.stepfunctions_state.inventory.request(original.clone());
    assert!(app
        .stepfunctions_state
        .inventory
        .start(async move { Ok(vec![machine("from-old-account")]) }));
    tokio::task::yield_now().await;

    app.config.account_id = "999999999999".into();
    poll(&mut app).await;
    assert!(app.stepfunctions_state.machines.items.is_empty());
    assert!(!app
        .stepfunctions_state
        .inventory
        .matches_scope(&scope(&app)));
}

#[tokio::test]
async fn pending_focus_selects_exact_arn_after_results_and_clears_on_error() {
    let mut app = app();
    app.stepfunctions_state.pending_focus =
        Some("arn:aws:states:us-east-1:123456789012:stateMachine:b".into());
    assert!(finish(&mut app, Ok(vec![machine("a"), machine("b"), machine("c")])).await);
    assert_eq!(app.stepfunctions_state.machines.selected, 1);
    assert!(app.stepfunctions_state.pending_focus.is_none());

    app.stepfunctions_state.pending_focus = Some("missing".into());
    assert!(finish(&mut app, Err("later failure".into())).await);
    assert!(app.stepfunctions_state.pending_focus.is_none());
}

#[test]
fn handle_action_ignores_non_inventory_actions_and_inactive_service() {
    let mut app = app();
    assert!(!handle_action(&mut app, &Action::NextItem));
    app.stepfunctions_state.executions.machine = Some(machine("entered"));
    assert!(!active(&app));
    assert!(!handle_action(&mut app, &Action::Refresh));
}
