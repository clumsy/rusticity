use super::*;
use crate::{
    app::{App, Service, ViewMode},
    common::{InputFocus, PageSize},
    keymap::{handle_key, Action, Mode},
    preferences::{bindings, PreferenceRow},
    ui::secretsmanager::filtered_secrets,
};
use columns::Column;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, buffer::Buffer, layout::Rect, Terminal};
use rusticity_core::{
    finder::{Resource, ResourceKind},
    SecretMetadata,
};
fn secret(i: usize) -> SecretMetadata {
    SecretMetadata {
        name: format!("prod/secret-{i:03}"),
        arn: format!(
            "arn:aws:secretsmanager:us-east-1:123456789012:secret:prod/secret-{i:03}-AbCdEf"
        ),
        description: format!("metadata row {i:03}"),
        rotation_enabled: Some(i.is_multiple_of(2)),
        created_date: "2026-10-01T12:34:56Z".into(),
        last_changed_date: "2026-10-15T12:34:56Z".into(),
        last_accessed_date: "2026-10-16".into(),
        last_rotated_date: String::new(),
        ..Default::default()
    }
}
fn app() -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    app.current_service = Service::SecretsManagerSecrets;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.config.account_id = "123456789012".into();
    app.secretsmanager_state.secrets.items = (0..25).rev().map(secret).collect();
    app
}
fn press(app: &mut App, key: KeyCode, mods: KeyModifiers) {
    if let Some(action) = handle_key(KeyEvent::new(key, mods), app.mode) {
        app.handle_action(action);
    }
}
fn screen(app: &App, width: u16, height: u16) -> String {
    let mut t = Terminal::new(TestBackend::new(width, height)).unwrap();
    t.draw(|f| crate::ui::render(f, app)).unwrap();
    t.backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}
fn prefs(app: &App) -> crate::preferences::Preferences {
    bindings::model_for(app, Service::SecretsManagerSecrets)
}

#[test]
fn service_picker_initialization_and_registration_are_complete() {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    assert_eq!(
        Service::SecretsManagerSecrets.name(),
        "Secrets Manager › Secrets"
    );
    assert_eq!(
        app.service_picker
            .services
            .iter()
            .filter(|s| **s == "Secrets Manager › Secrets")
            .count(),
        1
    );
    app.mode = Mode::ServicePicker;
    app.service_picker.filter = "Secrets Manager".into();
    app.service_picker.filter_active = false;
    app.handle_action(Action::Select);
    assert_eq!(app.current_service, Service::SecretsManagerSecrets);
    assert!(app.secretsmanager_state.secrets.loading);
    assert_eq!(app.tabs[0].service, Service::SecretsManagerSecrets);
    assert!(app.breadcrumbs().contains("Secrets Manager"));
    assert_eq!(app.secretsmanager_state.column_ids, Column::ids());
}
#[test]
fn columns_preserve_missing_metadata_and_date_precision() {
    let mut labels = HashMap::new();
    init(&mut labels);
    for col in Column::ALL {
        assert!(col.id().starts_with("column.secretsmanager.secret."));
        assert_eq!(Column::from_id(col.id()), Some(col));
        assert_eq!(labels[col.id()], col.default_name());
        assert_eq!(col.render(&SecretMetadata::default()).0, "-");
    }
    let mut row = secret(0);
    assert_eq!(Column::LastAccessed.render(&row).0, "2026-10-16");
    assert_eq!(
        Column::LastChanged.render(&row).0,
        "2026-10-15 12:34:56 (UTC)"
    );
    row.description = "escape\x1b text".into();
    assert!(!Column::Description.render(&row).0.contains('\x1b'));
    assert_eq!(Column::ALL.len(), 11);
    assert!(Column::from_id("secret_value").is_none());
}
#[test]
fn metadata_filtering_sorting_and_selected_arn_are_consistent() {
    let mut app = app();
    assert_eq!(filtered_secrets(&app)[0].name, "prod/secret-000");
    for (query, count) in [
        ("enabled", 13),
        ("disabled", 12),
        ("metadata row 024", 1),
        ("2026-10-16", 25),
        ("missing", 0),
        ("prod/secret-024 ", 0),
    ] {
        app.secretsmanager_state.secrets.filter = query.into();
        assert_eq!(filtered_secrets(&app).len(), count);
    }
    app.secretsmanager_state.secrets.filter = "SECRET-024".into();
    let expected = secret(24).arn;
    assert_eq!(
        actions::selected_arn(&app).as_deref(),
        Some(expected.as_str())
    );
    press(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
    assert!(app
        .clipboard
        .complete_with(&Buffer::empty(Rect::new(0, 0, 1, 1)), |s| {
            assert_eq!(s, expected);
            Ok(())
        }));
    assert_eq!(
        app.get_console_url(),
        "https://us-east-1.console.aws.amazon.com/secretsmanager/listsecrets?region=us-east-1"
    );
    app.secretsmanager_state.secrets.selected = usize::MAX;
    assert!(actions::selected_arn(&app).is_none());
    assert!(app.get_console_url().contains("/listsecrets?"));
}
#[test]
fn filter_and_pagination_reuse_shared_input_and_never_start_value_requests() {
    for legacy in [ViewMode::List, ViewMode::Detail, ViewMode::Events] {
        let mut app = app();
        app.view_mode = legacy;
        app.secretsmanager_state.secrets.page_size = PageSize::Ten;
        press(&mut app, KeyCode::Down, KeyModifiers::NONE);
        press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(app.secretsmanager_state.secrets.expanded_item, Some(1));
        press(&mut app, KeyCode::Char('i'), KeyModifiers::NONE);
        for c in "PROD".chars() {
            press(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
        }
        assert_eq!(app.secretsmanager_state.secrets.selected, 0);
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(app.secretsmanager_state.input_focus, InputFocus::Pagination);
        press(&mut app, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(app.secretsmanager_state.secrets.selected, 10);
        press(&mut app, KeyCode::Char('3'), KeyModifiers::NONE);
        press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(app.secretsmanager_state.secrets.selected, 20);
        assert!(!app.secretsmanager_state.secrets.loading);
        assert!(!app.log_groups_state.loading);
    }
}
#[test]
fn preferences_use_actual_columns_last_column_guard_and_page_size() {
    let mut app = app();
    press(&mut app, KeyCode::Char('p'), KeyModifiers::NONE);
    assert_eq!(app.mode, Mode::ColumnSelector);
    assert_eq!(
        prefs(&app)
            .columns()
            .map(|c| c.id.as_ref())
            .collect::<Vec<_>>(),
        Column::ids()
    );
    let i = prefs(&app)
        .rows()
        .iter()
        .position(|r| matches!(r,PreferenceRow::Column(c) if c.id==Column::Description.id()))
        .unwrap();
    app.column_selector_index = i;
    app.handle_action(Action::ToggleColumn);
    assert!(!app
        .secretsmanager_state
        .visible_column_ids
        .contains(&Column::Description.id()));
    app.secretsmanager_state.visible_column_ids = vec![Column::Name.id()];
    let i = prefs(&app)
        .rows()
        .iter()
        .position(|r| matches!(r,PreferenceRow::Column(c) if c.id==Column::Name.id()))
        .unwrap();
    app.column_selector_index = i;
    app.handle_action(Action::ToggleColumn);
    assert_eq!(
        app.secretsmanager_state.visible_column_ids,
        [Column::Name.id()]
    );
    let i = prefs(&app)
        .rows()
        .iter()
        .position(|r| {
            matches!(
                r,
                PreferenceRow::PageSize {
                    value: PageSize::Ten,
                    ..
                }
            )
        })
        .unwrap();
    app.secretsmanager_state.secrets.selected = 20;
    app.secretsmanager_state.secrets.expanded_item = Some(20);
    app.column_selector_index = i;
    app.handle_action(Action::ToggleColumn);
    assert_eq!(app.secretsmanager_state.secrets.page_size, PageSize::Ten);
    assert_eq!(app.secretsmanager_state.secrets.selected, 0);
    assert!(app.secretsmanager_state.secrets.expanded_item.is_none());
    app.column_selector_index = 0;
    app.handle_action(Action::NextPreferences);
    assert_eq!(app.column_selector_index, prefs(&app).move_section(0, true));
}
#[test]
fn all_advertised_inventory_shortcuts_and_loading_state_render() {
    let mut app = app();
    app.secretsmanager_state.secrets.page_size = PageSize::Ten;
    app.secretsmanager_state.secrets.selected = 12;
    let text = screen(&app, 300, 40);
    for label in [
        "Secrets (25)",
        "1 [2] 3",
        "► prod/secret-012",
        "p preferences",
        "^p print",
        "^o console",
        "^r refresh",
        "y copy ARN",
        "^y copy CLI",
        "^e CLI output",
    ] {
        assert!(text.contains(label), "missing {label}");
    }
    assert!(!text.contains("prod/secret-009"));
    assert!(!text.contains("prod/secret-020"));
    press(&mut app, KeyCode::Char('p'), KeyModifiers::CONTROL);
    assert!(app.clipboard.snapshot_requested());
    app.secretsmanager_state.secrets.items.clear();
    app.secretsmanager_state.secrets.loading = true;
    assert!(screen(&app, 200, 30).contains("Loading secret metadata"));
    for (w, h) in [(25, 5), (80, 15)] {
        screen(&app, w, h);
    }
}
#[test]
fn refresh_retry_and_sessions_remain_metadata_inventory() {
    for action in [Action::Select, Action::LoadSession] {
        let mut app = app();
        let mut session = crate::session::Session::new(
            "beta".into(),
            "us-east-1".into(),
            "123456789012".into(),
            "role".into(),
        );
        session.tabs.push(crate::session::SessionTab {
            service: "SecretsManagerSecrets".into(),
            title: "Secrets Manager › Secrets".into(),
            breadcrumb: "Secrets Manager › Secrets".into(),
            filter: None,
            selected_item: None,
        });
        app.sessions = vec![session];
        app.mode = Mode::SessionPicker;
        app.current_service = Service::CloudWatchLogGroups;
        app.handle_action(action);
        assert_eq!(app.current_service, Service::SecretsManagerSecrets);
        app.mode = Mode::Normal;
        app.handle_action(Action::Refresh);
        assert!(app.secretsmanager_state.secrets.loading);
        assert!(!app.log_groups_state.loading);
        app.secretsmanager_state.secrets.loading = false;
        app.mode = Mode::ErrorModal;
        app.error_message = Some("AccessDeniedException".into());
        app.handle_action(Action::RetryLoad);
        assert!(app.secretsmanager_state.secrets.loading);
        assert!(app.error_message.is_none());
        assert_eq!(app.secretsmanager_state.secrets.items.len(), 25);
    }
}
#[test]
fn finder_and_cli_use_full_arn_with_describe_secret_only() {
    let mut app = app();
    app.secretsmanager_state.secrets.filter = "024".into();
    let invocation = crate::cli_command::invocation(&app).unwrap();
    assert_eq!(&invocation.args[..2], ["secretsmanager", "describe-secret"]);
    assert!(invocation.args.iter().any(|s| s.contains(&secret(24).arn)));
    assert!(!invocation.display.contains("get-secret-value"));
    assert!(!invocation.display.contains("batch-get"));
    press(&mut app, KeyCode::Char('y'), KeyModifiers::CONTROL);
    assert!(app
        .clipboard
        .complete_with(&Buffer::empty(Rect::new(0, 0, 1, 1)), |s| {
            assert_eq!(s, invocation.display);
            Ok(())
        }));
    app.handle_action(Action::OpenRelatedResources);
    let selected = app.resource_finder.related_source.clone().unwrap();
    assert_eq!(selected.kind, ResourceKind::Secret);
    assert_eq!(selected.id, secret(24).arn);
    app.handle_action(Action::OpenResourceFinder);
    app.resource_finder.rows = vec![Resource {
        kind: ResourceKind::Secret,
        id: selected.id.clone(),
        name: selected.name,
        arn: selected.arn,
        region: "us-east-1".into(),
    }];
    app.handle_action(Action::Select);
    app.handle_action(Action::Select);
    let target = app.resource_finder.pending_open.as_ref().unwrap();
    assert_eq!(target.id, secret(24).arn);
    assert_eq!(
        crate::finder::navigation::service(target.kind),
        Service::SecretsManagerSecrets
    );
}
#[test]
fn console_table_link_uses_active_region() {
    assert_eq!(
        console_url("us-east-1"),
        "https://us-east-1.console.aws.amazon.com/secretsmanager/listsecrets?region=us-east-1"
    );
    assert!(console_url("eu-west-1").ends_with("/listsecrets?region=eu-west-1"));
    assert!(console_url("cn-north-1").contains("console.amazonaws.cn"));
}

#[test]
fn preferences_expose_exactly_the_eleven_requested_console_columns() {
    let app = app();
    let expected = [
        "Secret name",
        "Description",
        "Last retrieved (UTC)",
        "Created on (UTC)",
        "Deleted on (UTC)",
        "Last updated on (UTC)",
        "Last rotated date (UTC)",
        "Next rotation date (UTC)",
        "Replication type",
        "Managed by",
        "Type",
    ];
    assert_eq!(Column::ALL.map(Column::default_name), expected);
    assert_eq!(
        prefs(&app)
            .columns()
            .map(|c| c.label.as_str())
            .collect::<Vec<_>>(),
        expected
    );
    assert!(!prefs(&app)
        .columns()
        .any(|c| c.label == "ARN" || c.label == "Rotation"));
}

#[test]
fn added_metadata_columns_use_returned_fields_without_invented_defaults() {
    let mut row = secret(0);
    row.deleted_date = "2026-10-17T01:02:03Z".into();
    row.next_rotation_date = "2026-11-01T04:05:06Z".into();
    row.primary_region = "us-east-1".into();
    row.owning_service = "rds".into();
    row.secret_type = "ConfluentCloudApiKey".into();
    assert_eq!(Column::Deleted.render(&row).0, "2026-10-17 01:02:03 (UTC)");
    assert_eq!(
        Column::NextRotation.render(&row).0,
        "2026-11-01 04:05:06 (UTC)"
    );
    assert_eq!(Column::ReplicationType.render(&row).0, "Primary");
    row.primary_region = "eu-west-1".into();
    assert_eq!(Column::ReplicationType.render(&row).0, "Replica");
    row.primary_region.clear();
    assert_eq!(Column::ReplicationType.render(&row).0, "-");
    assert_eq!(Column::ManagedBy.render(&row).0, "rds");
    assert_eq!(Column::Type.render(&row).0, "ConfluentCloudApiKey");
}
