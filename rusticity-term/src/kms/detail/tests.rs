//! App-level tests driving the real detail state machine: open, tab switching,
//! filter, selection, Esc, preferences, per-section errors, stale-on-refresh,
//! cancel-on-context-change, and the intended yank/console behavior.
//!
//! Keys use App::handle_action, so routing/early-consumption regressions are
//! tested along with the detail state. Async completions are injected locally;
//! tests never make AWS requests or open the OS browser.
use super::*;
use crate::keymap::handle_key;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};
use rusticity_core::kms::detail::{KmsAlias, KmsRotation, KmsTag};
use rusticity_core::kms::KmsKey as CoreKey;

fn core_key(arn: &str) -> CoreKey {
    CoreKey {
        key_id: "1234abcd-12ab-34cd-56ef-1234567890ab".into(),
        key_arn: arn.into(),
        description: "fresh desc".into(),
        key_state: "Enabled".into(),
        key_usage: "EncryptDecrypt".into(),
        key_spec: "SymmetricDefault".into(),
        key_manager: "Customer".into(),
        creation_date: "2026-01-02T03:04:05Z".into(),
        expiration_date: String::new(),
        deletion_date: String::new(),
        custom_key_store_id: String::new(),
        origin: "AwsKms".into(),
        enabled: true,
        alias: "prod-key".into(),
        multi_region: false,
    }
}

fn list_key(arn: &str) -> Key {
    Key {
        key_id: "1234abcd-12ab-34cd-56ef-1234567890ab".into(),
        key_arn: arn.into(),
        alias: "prod-key".into(),
        description: "list desc".into(),
        key_state: "Enabled".into(),
        key_usage: "EncryptDecrypt".into(),
        key_spec: "SymmetricDefault".into(),
        key_manager: "Customer".into(),
        creation_date: "2026-01-02T03:04:05Z".into(),
        expiration_date: String::new(),
        deletion_date: String::new(),
        custom_key_store_id: String::new(),
        origin: "AwsKms".into(),
        enabled: true,
        multi_region: false,
    }
}

const ARN: &str = "arn:aws:kms:us-east-1:123456789012:key/1234abcd-12ab-34cd-56ef-1234567890ab";

fn app() -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    app.config.account_id = "123456789012".into();
    app.current_service = Service::KmsKeys;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.kms_state.tab = crate::ui::kms::Tab::CustomerManaged;
    app.kms_state.keys.items = vec![list_key(ARN)];
    app
}

fn act(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    let action = handle_key(KeyEvent::new(key, modifiers), app.mode).unwrap();
    app.handle_action(action);
}

fn detail(
    aliases: Result<Vec<KmsAlias>, String>,
    tags: Result<Vec<KmsTag>, String>,
) -> KmsKeyDetail {
    KmsKeyDetail {
        metadata: core_key(ARN),
        aliases,
        rotation: Ok(KmsRotation {
            enabled: Some(true),
            rotation_period_days: Some(365),
            next_rotation_date: "2027-01-02T03:04:05Z".into(),
            on_demand_rotation_start_date: String::new(),
        }),
        policy: Ok("{\n  \"Version\": \"2012-10-17\"\n}".into()),
        tags,
    }
}

/// Simulate a completed worker fetch without AWS.
fn apply(app: &mut App, detail: KmsKeyDetail) {
    let view = app.kms_state.detail.view_mut().unwrap();
    super::apply_detail(view, detail);
    app.kms_state.detail.pending = false;
    app.kms_state.detail.running = None;
}

fn screen(app: &App, width: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, 45)).unwrap();
    terminal.draw(|f| crate::ui::render(f, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

#[test]
fn enter_opens_detail_for_cloned_selected_key_and_preserves_inventory() {
    let mut app = app();
    app.kms_state.keys.filter = "prod".into();
    assert!(!active(&app));
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert!(active(&app));
    assert!(app.kms_state.detail.source.is_some());
    assert_eq!(app.kms_state.detail.source.as_ref().unwrap().key_arn, ARN);
    // Parent inventory selection + filter untouched.
    assert_eq!(app.kms_state.keys.filter, "prod");
    assert_eq!(app.kms_state.tab, crate::ui::kms::Tab::CustomerManaged);
    assert!(app.kms_state.detail.busy(), "fetch scheduled on entry");
}

#[test]
fn tab_cycles_all_five_tabs() {
    use crate::common::CyclicEnum;
    assert_eq!(Tab::General.next(), Tab::Aliases);
    assert_eq!(Tab::Aliases.next(), Tab::Rotation);
    assert_eq!(Tab::Rotation.next(), Tab::Policy);
    assert_eq!(Tab::Policy.next(), Tab::Tags);
    assert_eq!(Tab::Tags.next(), Tab::General);
    assert_eq!(Tab::General.prev(), Tab::Tags);
}

#[test]
fn tab_switch_changes_active_tab() {
    let mut app = app();
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(app.kms_state.detail.view().unwrap().tab, Tab::General);
    act(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(app.kms_state.detail.view().unwrap().tab, Tab::Aliases);
    act(&mut app, KeyCode::BackTab, KeyModifiers::SHIFT);
    assert_eq!(app.kms_state.detail.view().unwrap().tab, Tab::General);
}

#[test]
fn aliases_and_tags_rows_render_from_worker_result() {
    let mut app = app();
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    apply(
        &mut app,
        detail(
            Ok(vec![KmsAlias {
                name: "alias/prod-key".into(),
                arn: "arn:aws:kms:us-east-1:123456789012:alias/prod-key".into(),
                creation_date: "2026-01-02T03:04:05Z".into(),
                last_updated_date: "2026-02-02T03:04:05Z".into(),
            }]),
            Ok(vec![KmsTag {
                key: "env".into(),
                value: "production".into(),
            }]),
        ),
    );
    // Aliases tab
    act(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    let out = screen(&app, 160);
    assert!(out.contains("alias/prod-key"), "alias name visible");
    // Tags tab
    for _ in 0..3 {
        act(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    }
    assert_eq!(app.kms_state.detail.view().unwrap().tab, Tab::Tags);
    let out = screen(&app, 160);
    assert!(out.contains("production"), "tag value visible");
}

#[test]
fn filter_scopes_table_and_does_not_leak_to_parent_inventory() {
    let mut app = app();
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    apply(
        &mut app,
        detail(
            Ok(vec![
                KmsAlias {
                    name: "alias/alpha".into(),
                    ..Default::default()
                },
                KmsAlias {
                    name: "alias/beta".into(),
                    ..Default::default()
                },
            ]),
            Ok(vec![]),
        ),
    );
    act(&mut app, KeyCode::Tab, KeyModifiers::NONE); // Aliases
    act(&mut app, KeyCode::Char('i'), KeyModifiers::NONE); // StartFilter
    assert_eq!(app.mode, Mode::FilterInput);
    for c in "beta".chars() {
        act(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
    }
    let view = app.kms_state.detail.view().unwrap();
    assert_eq!(view.data().table.filter, "beta");
    assert_eq!(view.data().filtered().len(), 1);
    // Parent inventory filter untouched.
    assert_eq!(app.kms_state.keys.filter, "");
}

#[test]
fn esc_closes_detail_and_returns_to_inventory() {
    let mut app = app();
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert!(active(&app));
    act(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert!(!active(&app));
    assert!(app.kms_state.detail.source.is_none());
}

#[test]
fn yank_copies_key_arn_on_every_tab() {
    use ratatui::{buffer::Buffer, layout::Rect};
    let mut app = app();
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    let buffer = Buffer::empty(Rect::new(0, 0, 4, 1));
    for _ in 0..Tab::ALL.len() {
        act(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
        let mut captured = None;
        assert!(app.clipboard.complete_with(&buffer, |text| {
            captured = Some(text);
            Ok(())
        }));
        assert_eq!(captured.as_deref(), Some(ARN));
        act(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    }
}

#[test]
fn per_section_error_is_visible_not_no_results() {
    let mut app = app();
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    apply(
        &mut app,
        detail(Err("AccessDeniedException: ListAliases".into()), Ok(vec![])),
    );
    act(&mut app, KeyCode::Tab, KeyModifiers::NONE); // Aliases
    let out = screen(&app, 160);
    assert!(out.contains("Warning"), "error surfaced as warning");
    assert!(out.contains("AccessDenied"), "error text visible");
}

#[test]
fn general_pane_prefers_fresh_metadata_over_list_snapshot() {
    let mut app = app();
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    // Before fetch: list snapshot description shown.
    let out = screen(&app, 160);
    assert!(out.contains("list desc"));
    apply(&mut app, detail(Ok(vec![]), Ok(vec![])));
    let out = screen(&app, 160);
    assert!(
        out.contains("fresh desc"),
        "fresh DescribeKey metadata preferred"
    );
}

#[test]
fn policy_tab_renders_json_readonly() {
    let mut app = app();
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    apply(&mut app, detail(Ok(vec![]), Ok(vec![])));
    // General -> Aliases -> Rotation -> Policy
    for _ in 0..3 {
        act(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    }
    assert_eq!(app.kms_state.detail.view().unwrap().tab, Tab::Policy);
    let out = screen(&app, 160);
    assert!(out.contains("2012-10-17"), "policy JSON rendered");
}

#[test]
fn rotation_pane_shows_tristate_and_period() {
    let mut app = app();
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    apply(&mut app, detail(Ok(vec![]), Ok(vec![])));
    act(&mut app, KeyCode::Tab, KeyModifiers::NONE); // Aliases
    act(&mut app, KeyCode::Tab, KeyModifiers::NONE); // Rotation
    assert_eq!(app.kms_state.detail.view().unwrap().tab, Tab::Rotation);
    let out = screen(&app, 160);
    assert!(out.contains("Enabled"));
    assert!(out.contains("365"));
}

#[tokio::test]
async fn context_change_cancels_detail_with_no_late_repaint() {
    let mut app = app();
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert!(active(&app));
    // Simulate a region change (scope moves out from under the detail).
    app.config.region = "eu-west-1".into();
    assert!(poll(&mut app).await, "poll reports the teardown");
    assert!(app.kms_state.detail.view().is_none());
    assert!(app.kms_state.detail.source.is_none());
}

#[tokio::test]
async fn leaving_kms_tears_down_detail() {
    let mut app = app();
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    app.current_service = Service::EcrRepositories;
    assert!(poll(&mut app).await);
    assert!(app.kms_state.detail.view().is_none());
}

#[tokio::test]
async fn stale_data_retained_and_marked_on_real_refresh_failure() {
    let mut app = app();
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    apply(
        &mut app,
        detail(
            Ok(vec![KmsAlias {
                name: "alias/keep".into(),
                ..Default::default()
            }]),
            Ok(vec![]),
        ),
    );
    act(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL);
    assert!(app.kms_state.detail.busy());
    assert_eq!(
        app.kms_state.detail.view().unwrap().tabs[Tab::Aliases as usize]
            .table
            .items
            .len(),
        1
    );
    // Inject the failing background completion, then let real poll update state.
    app.kms_state.detail.pending = false;
    let task = tokio::spawn(async { Err("AccessDeniedException: refresh".into()) });
    tokio::task::yield_now().await;
    app.kms_state.detail.running = Some(Running { task });
    assert!(poll(&mut app).await);
    let view = app.kms_state.detail.view().unwrap();
    assert_eq!(view.tabs[Tab::Aliases as usize].table.items.len(), 1);
    assert!(view
        .tabs
        .iter()
        .all(|tab| tab.error.as_deref().unwrap().contains("Stale")));
    assert!(view.metadata.is_some() && view.rotation.is_some() && view.policy.is_some());
    app.kms_state.detail.view_mut().unwrap().tab = Tab::Policy;
    let out = screen(&app, 160);
    assert!(out.contains("Stale") && out.contains("AccessDenied"));
    assert!(out.contains("2012-10-17"));
}

#[test]
fn preferences_bind_only_table_tabs_no_leakage_on_summary_tabs() {
    use crate::preferences::bindings;
    let mut app = app();
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    apply(&mut app, detail(Ok(vec![]), Ok(vec![])));
    // General tab: read-only empty model (no inventory column prefs).
    assert_eq!(app.kms_state.detail.view().unwrap().tab, Tab::General);
    let model = bindings::model_for(&app, Service::KmsKeys);
    assert_eq!(model.column_count(), 0, "summary tab leaks no columns");
    // Aliases tab: real columns present.
    act(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    let model = bindings::model_for(&app, Service::KmsKeys);
    assert!(model.column_count() >= 2, "alias columns present");
    // Tags tab: real columns present, independent of Aliases.
    for _ in 0..3 {
        act(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    }
    assert_eq!(app.kms_state.detail.view().unwrap().tab, Tab::Tags);
    let model = bindings::model_for(&app, Service::KmsKeys);
    assert!(model.column_count() >= 2, "tag columns present");
}

#[test]
fn table_tab_page_size_is_independent_per_table() {
    use crate::common::PageSize;
    let mut app = app();
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    apply(&mut app, detail(Ok(vec![]), Ok(vec![])));
    act(&mut app, KeyCode::Tab, KeyModifiers::NONE); // Aliases
    app.kms_state
        .detail
        .view_mut()
        .unwrap()
        .data_mut()
        .table
        .set_page_size(PageSize::Ten);
    assert_eq!(
        app.kms_state.detail.view().unwrap().data().table.page_size,
        PageSize::Ten
    );
}

#[tokio::test]
async fn cli_tab_does_not_close_detail_and_actions_use_entered_key() {
    let mut app = app();
    assert!(app.get_console_url().ends_with("#/kms/keys"));
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    apply(&mut app, detail(Ok(vec![]), Ok(vec![])));
    // A hidden or changed inventory selection must not change detail identity.
    app.kms_state.keys.filter = "missing".into();
    app.kms_state.keys.selected = usize::MAX;
    assert!(app
        .get_console_url()
        .ends_with("#/kms/keys/1234abcd-12ab-34cd-56ef-1234567890ab"));
    let command = crate::cli_command::invocation(&app).unwrap();
    assert_eq!(&command.args[..2], ["kms", "describe-key"]);
    assert!(command.display.contains(ARN));
    assert!(!super::handle_action(&mut app, &Action::CopyCliCommand));
    assert!(!super::handle_action(&mut app, &Action::OpenCliOutput));
    act(&mut app, KeyCode::Char('e'), KeyModifiers::CONTROL);
    assert!(matches!(app.current_service, Service::CliOutput(_)));
    assert!(!poll(&mut app).await);
    assert!(app.kms_state.detail.source.is_some());
    app.current_service = Service::KmsKeys;
    assert!(!poll(&mut app).await);
    act(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert!(app.get_console_url().ends_with("#/kms/keys"));
    assert_eq!(app.kms_state.keys.filter, "missing");
}

#[tokio::test]
async fn esc_and_scope_changes_abort_inflight_task_and_discard_data() {
    for change in 0..5 {
        let mut app = app();
        act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        app.kms_state.detail.pending = false;
        let task = tokio::spawn(std::future::pending::<Result<Output, String>>());
        let abort = task.abort_handle();
        app.kms_state.detail.running = Some(Running { task });
        match change {
            0 => act(&mut app, KeyCode::Esc, KeyModifiers::NONE),
            1 => app.profile = "other".into(),
            2 => app.config.region = "eu-west-1".into(),
            3 => app.config.account_id = "999999999999".into(),
            _ => app.config.role_arn = "arn:aws:iam::123456789012:role/other".into(),
        }
        if change != 0 {
            assert!(poll(&mut app).await);
        }
        tokio::task::yield_now().await;
        assert!(abort.is_finished());
        assert!(!app.kms_state.detail.busy());
        assert!(app.kms_state.detail.source.is_none());
    }
}

#[test]
fn section_failure_keeps_previous_data_while_other_sections_refresh() {
    let mut app = app();
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    apply(
        &mut app,
        detail(
            Ok(vec![KmsAlias {
                name: "alias/old".into(),
                ..Default::default()
            }]),
            Ok(vec![]),
        ),
    );
    act(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL);
    let mut refreshed = detail(
        Err("AccessDenied".into()),
        Ok(vec![KmsTag {
            key: "new".into(),
            value: "tag".into(),
        }]),
    );
    refreshed.policy = Err("AccessDenied policy".into());
    refreshed.rotation = Err("Unsupported rotation".into());
    refreshed.metadata.key_state = "Disabled".into();
    apply(&mut app, refreshed);
    let view = app.kms_state.detail.view().unwrap();
    assert_eq!(
        view.tabs[Tab::Aliases as usize].table.items[0].field("Alias"),
        "alias/old"
    );
    assert!(view.tabs[Tab::Aliases as usize]
        .error
        .as_deref()
        .unwrap()
        .contains("Stale"));
    assert_eq!(view.tabs[Tab::Tags as usize].table.items.len(), 1);
    assert!(view.tabs[Tab::Tags as usize].error.is_none());
    assert!(view.policy.is_some() && view.rotation.is_some());
    let out = screen(&app, 160);
    assert!(out.contains("Disabled"));
}

#[test]
fn policy_scrolling_is_bounded_and_refresh_does_not_erase_preferences() {
    let mut app = app();
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    let mut result = detail(Ok(vec![]), Ok(vec![]));
    result.policy = Ok((0..40).map(|n| format!("line {n}\n")).collect());
    apply(&mut app, result);
    app.kms_state.detail.view_mut().unwrap().tab = Tab::Policy;
    act(&mut app, KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(app.kms_state.detail.view().unwrap().policy_scroll, 1);
    act(&mut app, KeyCode::Char('d'), KeyModifiers::CONTROL);
    assert_eq!(app.kms_state.detail.view().unwrap().policy_scroll, 11);
    for _ in 0..100 {
        act(&mut app, KeyCode::Down, KeyModifiers::NONE);
    }
    assert_eq!(app.kms_state.detail.view().unwrap().policy_scroll, 39);
    act(&mut app, KeyCode::Char('u'), KeyModifiers::CONTROL);
    assert_eq!(app.kms_state.detail.view().unwrap().policy_scroll, 29);
    app.kms_state.detail.view_mut().unwrap().tab = Tab::Aliases;
    let data = app.kms_state.detail.view_mut().unwrap().data_mut();
    data.visible_column_ids = vec!["Alias"];
    data.table.set_page_size(crate::common::PageSize::Ten);
    act(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL);
    let data = app.kms_state.detail.view().unwrap().data();
    assert_eq!(data.visible_column_ids, ["Alias"]);
    assert_eq!(data.table.page_size, crate::common::PageSize::Ten);
}

#[test]
fn detail_tables_use_common_pagination_focus_and_last_column_guard() {
    use crate::common::PageSize;
    use crate::preferences::{bindings, PreferenceRow};
    let mut app = app();
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    apply(
        &mut app,
        detail(
            Ok((0..25)
                .map(|i| KmsAlias {
                    name: format!("alias/{i:02}"),
                    ..Default::default()
                })
                .collect()),
            Ok(vec![]),
        ),
    );
    act(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    app.kms_state
        .detail
        .view_mut()
        .unwrap()
        .data_mut()
        .table
        .page_size = PageSize::Ten;
    act(&mut app, KeyCode::Char('i'), KeyModifiers::NONE);
    act(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(
        app.kms_state.detail.view().unwrap().data().focus,
        InputFocus::Pagination
    );
    act(&mut app, KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(
        app.kms_state.detail.view().unwrap().data().table.selected,
        10
    );
    act(&mut app, KeyCode::Char('3'), KeyModifiers::NONE);
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        app.kms_state.detail.view().unwrap().data().table.selected,
        20
    );
    assert_eq!(app.kms_state.keys.selected, 0);
    app.kms_state
        .detail
        .view_mut()
        .unwrap()
        .data_mut()
        .visible_column_ids = vec!["Alias"];
    act(&mut app, KeyCode::Char('p'), KeyModifiers::NONE);
    assert_eq!(app.mode, Mode::ColumnSelector);
    let prefs = bindings::model_for(&app, Service::KmsKeys);
    app.column_selector_index = prefs
        .rows()
        .iter()
        .position(|row| matches!(row, PreferenceRow::Column(col) if col.id == "Alias"))
        .unwrap();
    app.handle_action(Action::ToggleColumn);
    assert_eq!(
        app.kms_state
            .detail
            .view()
            .unwrap()
            .data()
            .visible_column_ids,
        ["Alias"]
    );
    act(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert!(active(&app));
    act(&mut app, KeyCode::Char('p'), KeyModifiers::CONTROL);
    assert!(app.clipboard.snapshot_requested());
}

#[test]
fn status_hints_and_rendering_survive_narrow_areas_on_every_tab() {
    let mut app = app();
    act(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    let mut result = detail(
        Err("AccessDenied aliases".into()),
        Err("AccessDenied tags".into()),
    );
    result.rotation = Err("Unsupported rotation".into());
    result.policy = Err("AccessDenied policy".into());
    apply(&mut app, result);
    for tab in Tab::ALL {
        app.kms_state.detail.view_mut().unwrap().tab = *tab;
        for width in [1, 30, 80, 300] {
            let out = screen(&app, width);
            if width == 300 {
                assert!(
                    out.contains("^o") && out.contains("^p") && out.contains("^r"),
                    "{tab:?}: {out}"
                );
                if *tab != Tab::General {
                    assert!(out.contains("Denied") || out.contains("Unsupported"));
                }
            }
        }
    }
}
