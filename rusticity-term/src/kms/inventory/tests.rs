//! Locally-injected completions only — no credentials, no AWS. Asserts the same
//! freshness contract as the SNS inventory tests for the KMS keys root, plus
//! the Core→UI key mapping the production future performs before injection.
use super::*;
use crate::keymap::Action;
use crate::ui::kms::Tab;

fn app() -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    app.current_service = Service::KmsKeys;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.config.account_id = "123456789012".into();
    app.kms_state.tab = Tab::CustomerManaged;
    app
}

/// A UI key as the production future would produce it (after Core→UI mapping).
fn ui_key(key_id: &str, alias: &str) -> UiKey {
    UiKey {
        key_id: key_id.into(),
        key_arn: format!("arn:aws:kms:us-east-1:123456789012:key/{key_id}"),
        alias: alias.into(),
        description: String::new(),
        key_state: "Enabled".into(),
        key_usage: "EncryptDecrypt".into(),
        key_spec: "SymmetricDefault".into(),
        key_manager: "Customer".into(),
        creation_date: String::new(),
        expiration_date: String::new(),
        deletion_date: String::new(),
        custom_key_store_id: String::new(),
        origin: "AwsKms".into(),
        multi_region: false,
        enabled: true,
    }
}

async fn finish(app: &mut App, result: Result<Vec<UiKey>, String>) -> bool {
    let scope = scope(app);
    let state = &mut app.kms_state;
    state.inventory.request(scope);
    assert!(state.inventory.start(async move { result }));
    tokio::task::yield_now().await;
    poll(app).await
}

#[test]
fn core_keys_map_to_ui_keys_field_for_field() {
    let core = rusticity_core::kms::KmsKey {
        key_id: "k1".into(),
        key_arn: "arn:1".into(),
        description: "d".into(),
        key_state: "Enabled".into(),
        key_usage: "EncryptDecrypt".into(),
        key_spec: "SymmetricDefault".into(),
        key_manager: "Customer".into(),
        creation_date: "c".into(),
        expiration_date: "e".into(),
        deletion_date: "x".into(),
        custom_key_store_id: "s".into(),
        origin: "AwsKms".into(),
        enabled: true,
        alias: "alias/a".into(),
        multi_region: true,
    };
    let ui = to_ui_key(core);
    assert_eq!(ui.key_id, "k1");
    assert_eq!(ui.key_arn, "arn:1");
    assert_eq!(ui.alias, "alias/a");
    assert_eq!(ui.key_manager, "Customer");
    assert!(ui.enabled && ui.multi_region);
    assert_eq!(ui.custom_key_store_id, "s");
    assert_eq!(ui.deletion_date, "x");
}

#[test]
fn request_clears_rows_resets_selection_drops_error_and_only_queues() {
    let mut app = app();
    app.kms_state.keys.items = vec![ui_key("stale", "alias/stale")];
    app.kms_state.keys.selected = 3;
    app.kms_state.inventory_error = Some("old".into());

    request(&mut app);

    assert!(app.kms_state.keys.items.is_empty());
    assert_eq!(app.kms_state.keys.selected, 0);
    assert!(app.kms_state.inventory_error.is_none());
    assert!(app.kms_state.keys.loading);
    assert!(app.kms_state.inventory.is_pending());
    assert!(!app.kms_state.inventory.is_running());
    assert!(matches!(read_status(&app), ReadStatus::Loading { .. }));
}

#[tokio::test]
async fn success_populates_rows_and_marks_ready() {
    let mut app = app();
    assert!(
        finish(
            &mut app,
            Ok(vec![ui_key("a", "alias/a"), ui_key("b", "alias/b")])
        )
        .await
    );
    assert_eq!(app.kms_state.keys.items.len(), 2);
    assert!(app.kms_state.inventory_error.is_none());
    assert!(!app.kms_state.keys.loading);
    assert_eq!(read_status(&app), ReadStatus::Ready);
}

#[tokio::test]
async fn successfully_empty_inventory_is_ready_and_seeded_rows_render() {
    let mut app = app();
    assert!(finish(&mut app, Ok(vec![])).await);
    assert!(app.kms_state.keys.items.is_empty());
    assert_eq!(read_status(&app), ReadStatus::Ready);

    app.kms_state.inventory.reset();
    app.kms_state.keys.items = vec![ui_key("seeded", "alias/seeded")];
    assert_eq!(app.kms_state.load_status(), ReadStatus::Ready);
    app.kms_state.keys.items.clear();
    assert_eq!(app.kms_state.load_status(), ReadStatus::Idle);
}

#[tokio::test]
async fn error_clears_stale_rows_records_inline_message_and_never_opens_modal() {
    let mut app = app();
    app.kms_state.keys.items = vec![ui_key("stale", "alias/stale")];
    assert!(finish(&mut app, Err("AccessDenied: kms:ListKeys\u{1b}".into())).await);
    assert!(app.kms_state.keys.items.is_empty());
    assert!(!app.kms_state.keys.loading);
    let message = app.kms_state.inventory_error.as_ref().unwrap();
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
    assert!(app.kms_state.inventory.is_pending());
    assert!(app.kms_state.inventory_error.is_none());

    assert!(finish(&mut app, Ok(vec![ui_key("recovered", "alias/recovered")])).await);
    assert_eq!(app.kms_state.keys.items.len(), 1);
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
    assert!(app.kms_state.inventory.is_pending());
}

#[tokio::test]
async fn result_from_a_different_aws_context_is_rejected() {
    let mut app = app();
    let original = scope(&app);
    app.kms_state.inventory.request(original.clone());
    assert!(app
        .kms_state
        .inventory
        .start(async move { Ok(vec![ui_key("old-profile", "alias/old")]) }));
    tokio::task::yield_now().await;

    app.profile = "other".into();
    poll(&mut app).await;
    assert!(app.kms_state.keys.items.is_empty());
    assert!(!app.kms_state.inventory.matches_scope(&scope(&app)));
}

#[tokio::test]
async fn pending_focus_selects_exact_key_id_after_results_and_clears_on_error() {
    let mut app = app();
    app.kms_state.pending_focus = Some("b".into());
    assert!(
        finish(
            &mut app,
            Ok(vec![
                ui_key("a", "alias/a"),
                ui_key("b", "alias/b"),
                ui_key("c", "alias/c"),
            ]),
        )
        .await
    );
    // filtered_kms_keys preserves input order on the Customer tab; "b" is index 1.
    assert_eq!(app.kms_state.keys.selected, 1);
    assert!(app.kms_state.pending_focus.is_none());

    app.kms_state.pending_focus = Some("missing".into());
    assert!(finish(&mut app, Err("later failure".into())).await);
    assert!(app.kms_state.pending_focus.is_none());
}

#[test]
fn handle_action_ignores_non_inventory_actions_and_entered_detail() {
    let mut app = app();
    assert!(!handle_action(&mut app, &Action::NextItem));
    // An entered key detail is not the root inventory.
    app.kms_state.detail.source = Some(ui_key("entered", "alias/entered"));
    assert!(!active(&app));
    assert!(!handle_action(&mut app, &Action::Refresh));
}

#[tokio::test]
async fn navigation_enters_exact_key_despite_duplicate_aliases() {
    let mut app = app();
    let wanted = ui_key("wanted-key", "same-label");
    let mut other = ui_key("other-key", "same-label");
    other.key_manager = "Aws".into();
    app.kms_state.pending_focus = Some(wanted.key_arn.clone());
    app.kms_state.open_after_load = true;
    assert!(finish(&mut app, Ok(vec![other, wanted.clone()])).await);
    assert_eq!(
        app.kms_state.detail.source.as_ref().unwrap().key_arn,
        wanted.key_arn
    );
    assert_eq!(app.kms_state.tab, Tab::CustomerManaged);
    assert!(app.kms_state.keys.filter.is_empty());
    assert!(app.kms_state.detail.busy());
    assert!(!app.kms_state.open_after_load);
}
