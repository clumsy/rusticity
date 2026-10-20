//! Locally-injected completions only — no credentials, no AWS. Each test drives
//! the module's own AsyncRead the way the live `poll` would, then asserts the
//! freshness contract: request clears immediately, success populates, error
//! clears stale rows and records an inline message, and a changed AWS context
//! never lets an obsolete result land.
use super::*;
use crate::keymap::Action;
use rusticity_core::SnsTopic;

fn app() -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    app.current_service = Service::SnsTopics;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.config.account_id = "123456789012".into();
    app
}

fn topic(name: &str) -> SnsTopic {
    SnsTopic::from_arn(&format!("arn:aws:sns:us-east-1:123456789012:{name}")).unwrap()
}

/// Inject a completion for the CURRENT scope exactly as `poll` would harvest it.
async fn finish(app: &mut App, result: Result<Vec<SnsTopic>, String>) -> bool {
    let scope = scope(app);
    let state = &mut app.sns_state;
    state.inventory.request(scope);
    assert!(state.inventory.start(async move { result }));
    tokio::task::yield_now().await;
    poll(app).await
}

#[test]
fn request_clears_rows_resets_selection_drops_error_and_only_queues() {
    let mut app = app();
    app.sns_state.topics.items = vec![topic("stale")];
    app.sns_state.topics.selected = 7;
    app.sns_state.inventory_error = Some("old failure".into());

    request(&mut app);

    // Displayed data is gone the instant the read is requested.
    assert!(app.sns_state.topics.items.is_empty());
    assert_eq!(app.sns_state.topics.selected, 0);
    assert!(app.sns_state.inventory_error.is_none());
    assert!(app.sns_state.topics.loading);
    // Queued, not executed: nothing is running yet.
    assert!(app.sns_state.inventory.is_pending());
    assert!(!app.sns_state.inventory.is_running());
    assert!(matches!(read_status(&app), ReadStatus::Loading { .. }));
}

#[tokio::test]
async fn success_populates_rows_and_marks_ready() {
    let mut app = app();
    assert!(finish(&mut app, Ok(vec![topic("a"), topic("b")])).await);
    assert_eq!(app.sns_state.topics.items.len(), 2);
    assert!(app.sns_state.inventory_error.is_none());
    assert!(!app.sns_state.topics.loading);
    assert_eq!(read_status(&app), ReadStatus::Ready);
}

#[tokio::test]
async fn successfully_empty_inventory_is_ready_not_loading() {
    let mut app = app();
    assert!(finish(&mut app, Ok(vec![])).await);
    assert!(app.sns_state.topics.items.is_empty());
    assert!(app.sns_state.inventory_error.is_none());
    assert_eq!(read_status(&app), ReadStatus::Ready);
    // A populated legacy/seeded inventory with an Idle AsyncRead renders too.
    app.sns_state.inventory.reset();
    app.sns_state.topics.items = vec![topic("seeded")];
    assert_eq!(app.sns_state.load_status(), ReadStatus::Ready);
    app.sns_state.topics.items.clear();
    assert_eq!(app.sns_state.load_status(), ReadStatus::Idle);
}

#[tokio::test]
async fn error_clears_stale_rows_records_inline_message_and_never_opens_modal() {
    let mut app = app();
    app.sns_state.topics.items = vec![topic("stale")];
    assert!(finish(&mut app, Err("AccessDenied: sns:ListTopics\u{1b}".into())).await);
    assert!(app.sns_state.topics.items.is_empty());
    assert!(!app.sns_state.topics.loading);
    let message = app.sns_state.inventory_error.as_ref().unwrap();
    assert!(message.contains("AccessDenied"));
    assert!(!message.contains('\u{1b}')); // control chars sanitized
    assert_eq!(app.mode, Mode::Normal); // no ErrorModal focus steal
    assert_eq!(read_status(&app), ReadStatus::Failed);
}

#[tokio::test]
async fn retry_from_error_requeues_and_recovers() {
    let mut app = app();
    assert!(finish(&mut app, Err("transient".into())).await);
    assert_eq!(read_status(&app), ReadStatus::Failed);

    // Ctrl+R on the failed root inventory re-requests.
    assert!(handle_action(&mut app, &Action::Refresh));
    assert!(app.sns_state.inventory.is_pending());
    assert!(app.sns_state.inventory_error.is_none());

    assert!(finish(&mut app, Ok(vec![topic("recovered")])).await);
    assert_eq!(app.sns_state.topics.items.len(), 1);
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
    assert!(app.sns_state.inventory.is_pending());
}

#[tokio::test]
async fn result_from_a_different_aws_context_is_rejected_and_data_never_goes_stale() {
    let mut app = app();
    // Queue + start a read, then the user switches region before it lands.
    let original = scope(&app);
    app.sns_state.inventory.request(original.clone());
    assert!(app
        .sns_state
        .inventory
        .start(async move { Ok(vec![topic("from-old-region")]) }));
    tokio::task::yield_now().await;

    app.config.region = "eu-west-1".into();
    // poll must drop the obsolete scope's result, leaving no stale rows.
    poll(&mut app).await;
    assert!(app.sns_state.topics.items.is_empty());
    assert!(!app.sns_state.inventory.matches_scope(&scope(&app)));
}

#[tokio::test]
async fn pending_focus_selects_exact_arn_after_results_and_clears_on_error() {
    let mut app = app();
    app.sns_state.pending_focus = Some("arn:aws:sns:us-east-1:123456789012:b".into());
    assert!(finish(&mut app, Ok(vec![topic("a"), topic("b"), topic("c")])).await);
    // filtered_topics sorts by name; "b" is index 1.
    assert_eq!(app.sns_state.topics.selected, 1);
    assert!(app.sns_state.pending_focus.is_none());

    // A failed read drops any queued focus rather than selecting a stale row.
    app.sns_state.pending_focus = Some("arn:aws:sns:us-east-1:123456789012:z".into());
    assert!(finish(&mut app, Err("later failure".into())).await);
    assert!(app.sns_state.pending_focus.is_none());
}

#[test]
fn handle_action_ignores_non_inventory_actions_and_inactive_service() {
    let mut app = app();
    assert!(!handle_action(&mut app, &Action::NextItem));
    // Entered subscriptions detail is not the root inventory.
    app.sns_state.subscriptions.topic = Some(topic("entered"));
    assert!(!active(&app));
    assert!(!handle_action(&mut app, &Action::Refresh));
}
