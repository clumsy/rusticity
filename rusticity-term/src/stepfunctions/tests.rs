use super::*;
use crate::{
    app::{App, Service, ViewMode},
    common::{InputFocus, PageSize},
    keymap::{handle_key, Action, Mode},
    preferences::{bindings, PreferenceRow},
    ui::stepfunctions::filtered_machines,
};
use columns::Column;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, buffer::Buffer, layout::Rect, Terminal};
use rusticity_core::{ExecutionMetadata, StateMachineMetadata};
use std::collections::HashMap;

const ACCOUNT: &str = "123456789012";

fn sm(name: &str, machine_type: &str) -> StateMachineMetadata {
    StateMachineMetadata {
        name: name.to_string(),
        arn: format!("arn:aws:states:us-east-1:{ACCOUNT}:stateMachine:{name}"),
        machine_type: machine_type.to_string(),
        creation_date: "2024-01-02T03:04:05Z".to_string(),
    }
}

fn exec(name: &str, status: &str, duration_millis: Option<u64>) -> ExecutionMetadata {
    ExecutionMetadata {
        name: name.to_string(),
        arn: format!("arn:aws:states:us-east-1:{ACCOUNT}:execution:wf:{name}"),
        state_machine_arn: format!("arn:aws:states:us-east-1:{ACCOUNT}:stateMachine:wf"),
        status: status.to_string(),
        start_date: "2024-01-02T03:04:05Z".to_string(),
        stop_date: "2024-01-02T03:05:05Z".to_string(),
        duration_millis,
        ..Default::default()
    }
}

fn app() -> App {
    let mut app = App::new_without_client("test".into(), Some("us-east-1".into()));
    app.current_service = Service::StepFunctionsStateMachines;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.config.account_id = ACCOUNT.into();
    app.stepfunctions_state.machines.items = (0..25)
        .rev()
        .map(|i| {
            sm(
                &format!("wf-{i:03}"),
                if i % 2 == 0 { "STANDARD" } else { "EXPRESS" },
            )
        })
        .collect();
    app
}

fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    app.handle_action(handle_key(KeyEvent::new(code, modifiers), app.mode).unwrap());
}

fn screen(app: &App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| crate::ui::render(f, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

fn model(app: &App) -> crate::preferences::Preferences {
    bindings::model_for(app, Service::StepFunctionsStateMachines)
}

#[test]
fn state_machine_columns_are_shared_with_preferences_and_i18n() {
    let app = app();
    let mut labels = HashMap::new();
    init(&mut labels);
    assert_eq!(
        Column::ALL.map(Column::default_name),
        ["Name", "Type", "Created (UTC)", "ARN"]
    );
    for col in Column::ALL {
        assert!(col.id().starts_with("column.stepfunctions.statemachine."));
        assert_eq!(Column::from_id(col.id()), Some(col));
        assert_eq!(labels[col.id()], col.default_name());
        assert_eq!(col.render(&StateMachineMetadata::default()).0, "-");
    }
    assert!(Column::from_id("unknown").is_none());
    assert_eq!(
        model(&app)
            .columns()
            .map(|c| c.id.as_ref())
            .collect::<Vec<_>>(),
        Column::ids()
    );
}

#[test]
fn execution_columns_registered_and_duration_is_human_readable_with_missing_dash() {
    use executions::Column as ExecColumn;
    let mut labels = HashMap::new();
    init(&mut labels);
    assert_eq!(
        ExecColumn::ALL.map(ExecColumn::default_name),
        [
            "Name",
            "Status",
            "Start Time",
            "End Time",
            "Duration",
            "Version",
            "Alias",
            "Redrive count",
            "Last redrive time",
        ]
    );
    for col in ExecColumn::ALL {
        assert!(col.id().starts_with("column.stepfunctions.execution."));
        assert_eq!(labels[col.id()], col.default_name());
    }
    // Missing duration → "-" (never a fabricated elapsed-since-now value).
    assert_eq!(
        ExecColumn::Duration.render(&exec("r", "RUNNING", None)).0,
        "-"
    );
    // Human-readable units.
    assert_eq!(
        ExecColumn::Duration
            .render(&exec("a", "SUCCEEDED", Some(500)))
            .0,
        "500ms"
    );
    assert_eq!(
        ExecColumn::Duration
            .render(&exec("b", "SUCCEEDED", Some(65_000)))
            .0,
        "1m 5s"
    );
    assert_eq!(
        ExecColumn::Duration
            .render(&exec("c", "SUCCEEDED", Some(90_061_000)))
            .0,
        "1d 1h 1m 1s"
    );
}

#[test]
fn ctrl_o_from_inventory_always_opens_regional_list_never_a_row() {
    assert_eq!(
        handle_key(
            KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
            Mode::Normal
        ),
        Some(Action::OpenInConsole)
    );
    for (region, domain) in [
        ("us-east-1", "aws.amazon.com"),
        ("eu-west-1", "aws.amazon.com"),
        ("cn-north-1", "amazonaws.cn"),
    ] {
        for view in [ViewMode::List, ViewMode::Detail, ViewMode::Events] {
            let mut app = self::app();
            app.config.region = region.into();
            app.view_mode = view;
            let expected = format!(
                "https://{region}.console.{domain}/states/home?region={region}#/statemachines"
            );
            for (filter, selected) in [
                ("", 0usize),
                ("", 11),
                ("wf-024", 0),
                ("missing", 0),
                ("", usize::MAX),
            ] {
                app.stepfunctions_state.machines.filter = filter.into();
                app.stepfunctions_state.machines.selected = selected;
                app.stepfunctions_state.machines.expanded_item = Some(selected);
                assert!(app.stepfunctions_state.executions.machine.is_none());
                assert_eq!(app.get_console_url(), expected);
            }
            app.stepfunctions_state.machines.items.clear();
            assert_eq!(app.get_console_url(), expected);
        }
    }
}

#[test]
fn entered_view_console_deep_links_to_selected_state_machine_not_execution() {
    let mut app = self::app();
    // wf-024 is even → STANDARD.
    app.stepfunctions_state.machines.filter = "wf-024".into();
    assert_eq!(filtered_machines(&app).len(), 1);
    app.handle_action(Action::Select);
    assert!(super::executions::active(&app));
    let arn = format!("arn:aws:states:us-east-1:{ACCOUNT}:stateMachine:wf-024");
    assert_eq!(
        app.get_console_url(),
        super::console_url("us-east-1", Some(&arn))
    );
    assert!(app
        .get_console_url()
        .contains("#/statemachines/view/arn%3Aaws%3Astates"));
    // A highlighted execution does not change the link.
    app.stepfunctions_state.executions.table.items = vec![exec("x", "SUCCEEDED", Some(1000))];
    app.stepfunctions_state.executions.table.selected = 0;
    assert!(app
        .get_console_url()
        .ends_with(&format!("view/{}", urlencoding::encode(&arn))));
}

#[test]
fn enter_opens_the_filtered_machine_and_go_back_restores_inventory() {
    let mut app = self::app();
    app.stepfunctions_state.machines.filter = "wf-012".into();
    app.handle_action(Action::Select);
    let machine = app.stepfunctions_state.executions.machine.as_ref().unwrap();
    assert_eq!(machine.name, "wf-012");
    assert!(super::executions::active(&app));
    app.handle_action(Action::GoBack);
    assert!(app.stepfunctions_state.executions.machine.is_none());
    // Parent inventory filter/selection are retained across close.
    assert_eq!(app.stepfunctions_state.machines.filter, "wf-012");
}

#[tokio::test]
async fn express_machine_opens_summary_but_never_requests_and_shows_message() {
    let mut app = self::app();
    // wf-001 is odd → EXPRESS.
    app.stepfunctions_state.machines.filter = "wf-001".into();
    app.handle_action(Action::Select);
    assert!(super::executions::active(&app));
    assert!(app.stepfunctions_state.executions.unsupported());
    // Enter must NOT put the table into a loading state for Express.
    assert!(!app.stepfunctions_state.executions.table.loading);
    // poll issues no request and clears any loading flag without spawning work.
    app.stepfunctions_state.executions.table.loading = true;
    let changed = super::executions::poll(&mut app).await;
    assert!(changed);
    assert!(!app.stepfunctions_state.executions.table.loading);
    // A second poll is a steady-state no-op (still no request).
    assert!(!super::executions::poll(&mut app).await);
    // Refresh on Express is a no-op; it never re-enters loading.
    app.handle_action(Action::Refresh);
    assert!(!app.stepfunctions_state.executions.table.loading);
    // The summary renders an explicit unsupported notice, not an empty table.
    let text = screen(&app, 220, 30);
    assert!(text.contains("not available for EXPRESS"));
    assert!(!text.contains("No executions for this state machine"));
}

#[tokio::test]
async fn standard_poll_aborts_on_scope_change_and_preserves_cli_tab() {
    let mut app = self::app();
    app.stepfunctions_state.machines.filter = "wf-024".into(); // STANDARD
    app.handle_action(Action::Select);
    assert!(app.stepfunctions_state.executions.table.loading);

    // Leaving the service (to an unrelated service) closes the view.
    app.current_service = Service::VpcVpcs;
    assert!(super::executions::poll(&mut app).await);
    assert!(app.stepfunctions_state.executions.machine.is_none());

    // CLI output tab is explicitly preserved (not closed) while entered.
    let mut app = self::app();
    app.stepfunctions_state.machines.filter = "wf-024".into();
    app.handle_action(Action::Select);
    app.stepfunctions_state.executions.table.loading = false;
    app.current_service = Service::CliOutput(0);
    // No scope change, CLI tab allowed → view stays open, no forced close.
    let _ = super::executions::poll(&mut app).await;
    assert!(app.stepfunctions_state.executions.machine.is_some());

    // A profile/region/account/role change aborts and closes.
    app.current_service = Service::StepFunctionsStateMachines;
    app.config.region = "eu-west-1".into();
    assert!(super::executions::poll(&mut app).await);
    assert!(app.stepfunctions_state.executions.machine.is_none());
}

#[test]
fn stale_rows_are_kept_on_error_and_rendered_with_a_warning() {
    let mut app = self::app();
    app.stepfunctions_state.machines.filter = "wf-024".into();
    app.handle_action(Action::Select);
    let state = &mut app.stepfunctions_state.executions;
    state.table.loading = false;
    state.table.items = vec![exec("prev", "SUCCEEDED", Some(1000))];
    // Simulate a loaded-then-failed-refresh: rows stay, warning shown.
    state.loaded = true;
    state.error = Some("AccessDenied".into());
    let text = screen(&app, 220, 30);
    assert!(text.contains("Refresh failed; showing previously loaded executions"));
    assert!(text.contains("prev"));
}

#[test]
fn executions_use_shared_filter_focus_and_pagination() {
    let mut app = self::app();
    app.stepfunctions_state.machines.filter = "wf-024".into();
    app.handle_action(Action::Select);
    let state = &mut app.stepfunctions_state.executions;
    state.table.loading = false;
    state.loaded = true;
    state.table.page_size = PageSize::Ten;
    state.table.items = (0..25)
        .map(|i| exec(&format!("e{i:03}"), "SUCCEEDED", Some(i as u64 * 1000)))
        .collect();
    // Filtering by status narrows rows via the shared field filter.
    app.stepfunctions_state.executions.table.filter = "e01".into();
    assert_eq!(super::executions::filtered(&app).len(), 10);
    app.stepfunctions_state.executions.table.filter.clear();

    // Tab moves focus to pagination; Right/Left page through shared handler.
    press(&mut app, KeyCode::Char('i'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(
        app.stepfunctions_state.executions.input_focus,
        InputFocus::Pagination
    );
    press(&mut app, KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(app.stepfunctions_state.executions.table.selected, 10);
    press(&mut app, KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(app.stepfunctions_state.executions.table.selected, 0);
}

#[test]
fn inventory_uses_shared_input_even_with_stale_legacy_view_modes() {
    for view_mode in [ViewMode::List, ViewMode::Detail, ViewMode::Events] {
        let mut app = self::app();
        app.view_mode = view_mode;
        app.stepfunctions_state.machines.page_size = PageSize::Ten;
        app.handle_action(Action::NextItem);
        app.handle_action(Action::ExpandRow);
        assert_eq!(app.stepfunctions_state.machines.expanded_item, Some(1));
        app.handle_action(Action::PageDown);
        assert_eq!(app.stepfunctions_state.machines.selected, 11);
        assert!(app.stepfunctions_state.machines.expanded_item.is_none());
        press(&mut app, KeyCode::Char('i'), KeyModifiers::NONE);
        for c in "WF".chars() {
            press(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
        }
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(app.stepfunctions_state.input_focus, InputFocus::Pagination);
    }
}

#[test]
fn preferences_toggle_actual_columns_protect_last_and_change_page_size() {
    let mut app = self::app();
    press(&mut app, KeyCode::Char('p'), KeyModifiers::NONE);
    assert_eq!(app.mode, Mode::ColumnSelector);
    let prefs = model(&app);
    let arn_index = prefs
        .rows()
        .iter()
        .position(|r| matches!(r, PreferenceRow::Column(c) if c.id == Column::Arn.id()))
        .unwrap();
    app.column_selector_index = arn_index;
    app.handle_action(Action::ToggleColumn);
    assert!(!app
        .stepfunctions_state
        .visible_column_ids
        .contains(&Column::Arn.id()));

    // Last-column guard: reduce to one visible column, toggling it is refused.
    app.stepfunctions_state.visible_column_ids = vec![Column::Name.id()];
    let index = model(&app)
        .rows()
        .iter()
        .position(|r| matches!(r, PreferenceRow::Column(c) if c.id == Column::Name.id()))
        .unwrap();
    app.column_selector_index = index;
    app.handle_action(Action::ToggleColumn);
    assert_eq!(
        app.stepfunctions_state.visible_column_ids,
        [Column::Name.id()]
    );

    // Page size change applies to the state machines table.
    let index = model(&app)
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
    app.stepfunctions_state.machines.selected = 20;
    app.column_selector_index = index;
    app.handle_action(Action::ToggleColumn);
    assert_eq!(app.stepfunctions_state.machines.page_size, PageSize::Ten);
    assert_eq!(app.stepfunctions_state.machines.selected, 0);
}

#[test]
fn state_machine_and_execution_page_sizes_are_independent() {
    let mut app = self::app();
    app.stepfunctions_state
        .machines
        .set_page_size(PageSize::Ten);
    app.stepfunctions_state.machines.filter = "wf-024".into();
    app.handle_action(Action::Select); // enter STANDARD machine
    app.stepfunctions_state
        .executions
        .table
        .set_page_size(PageSize::Fifty);
    // The executions-scoped preference source reports the executions page size.
    let exec_model = bindings::model_for(&app, Service::StepFunctionsStateMachines);
    assert!(exec_model.rows().iter().any(|r| matches!(
        r,
        PreferenceRow::PageSize {
            value: PageSize::Fifty,
            selected: true,
            ..
        }
    )));
    // The inventory page size is unchanged.
    assert_eq!(app.stepfunctions_state.machines.page_size, PageSize::Ten);
}

#[test]
fn inventory_renders_title_hints_and_loading() {
    let mut app = self::app();
    app.stepfunctions_state.machines.page_size = PageSize::Ten;
    let text = screen(&app, 240, 45);
    assert!(text.contains("State machines (25)"));
    for hint in ["p preferences", "y copy ARN", "^o console", "^r refresh"] {
        assert!(text.contains(hint), "missing {hint}");
    }
    app.stepfunctions_state.machines.items.clear();
    app.stepfunctions_state.machines.loading = true;
    assert!(screen(&app, 240, 45).contains("Loading state machines"));
}

#[test]
fn copy_arn_uses_highlighted_inventory_row() {
    let mut app = self::app();
    app.stepfunctions_state.machines.filter = "wf-024".into();
    let arn = format!("arn:aws:states:us-east-1:{ACCOUNT}:stateMachine:wf-024");
    assert_eq!(actions::selected_arn(&app).as_deref(), Some(arn.as_str()));
    press(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
    assert!(app
        .clipboard
        .complete_with(&Buffer::empty(Rect::new(0, 0, 1, 1)), |text| {
            assert_eq!(text, arn);
            Ok(())
        }));
}

#[test]
fn console_link_builder_covers_partitions_and_encodes_arn_once() {
    let arn = "arn:aws:states:us-east-1:123456789012:stateMachine:orders";
    assert_eq!(
        console_url("us-east-1", Some(arn)),
        "https://us-east-1.console.aws.amazon.com/states/home?region=us-east-1#/statemachines/view/arn%3Aaws%3Astates%3Aus-east-1%3A123456789012%3AstateMachine%3Aorders"
    );
    assert!(console_url("eu-west-1", None).ends_with("region=eu-west-1#/statemachines"));
    assert!(console_url("cn-north-1", None).starts_with("https://cn-north-1.console.amazonaws.cn/"));
}

#[test]
fn execution_cli_uses_entered_machine_even_if_inventory_selection_changes() {
    let mut app = self::app();
    app.stepfunctions_state.machines.filter = "wf-024".into();
    app.handle_action(Action::Select);
    let arn = app
        .stepfunctions_state
        .executions
        .machine
        .as_ref()
        .unwrap()
        .arn
        .clone();
    app.stepfunctions_state.machines.filter = "missing".into();
    app.stepfunctions_state.machines.selected = usize::MAX;
    let cmd = crate::cli_command::invocation(&app).unwrap();
    assert_eq!(&cmd.args[..2], ["stepfunctions", "list-executions"]);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&cmd.args[3]).unwrap(),
        serde_json::json!({"stateMachineArn":arn})
    );
    assert!(!cmd.display.contains("describe-execution"));
    app.handle_action(Action::GoBack);
    assert!(app.get_console_url().ends_with("#/statemachines"));
    assert_eq!(
        &crate::cli_command::invocation(&app).unwrap().args[..2],
        ["stepfunctions", "list-state-machines"]
    );
    app.stepfunctions_state.machines.filter = "wf-001".into();
    app.stepfunctions_state.machines.selected = 0;
    assert_eq!(
        &crate::cli_command::invocation(&app).unwrap().args[..2],
        ["stepfunctions", "list-state-machines"]
    );
    app.handle_action(Action::Select);
    assert!(crate::cli_command::invocation(&app)
        .err()
        .unwrap()
        .to_uppercase()
        .contains("EXPRESS"));
}

#[test]
fn execution_display_sorts_fractional_times_numerically() {
    let mut app = self::app();
    let mut whole = exec("whole", "RUNNING", None);
    whole.start_date = "2026-10-16T12:00:00Z".into();
    let mut fraction = exec("fraction", "RUNNING", None);
    fraction.start_date = "2026-10-16T12:00:00.5Z".into();
    app.stepfunctions_state.executions.table.items = vec![whole, fraction];
    assert_eq!(super::executions::filtered(&app)[0].name, "fraction");
}

#[test]
fn both_session_restore_paths_and_retry_keep_state_machine_inventory() {
    for action in [Action::Select, Action::LoadSession] {
        let mut app = self::app();
        let mut session = crate::session::Session::new(
            "test".into(),
            "us-east-1".into(),
            ACCOUNT.into(),
            String::new(),
        );
        session.tabs.push(crate::session::SessionTab {
            service: "StepFunctionsStateMachines".into(),
            title: "Step Functions › State machines".into(),
            breadcrumb: "Step Functions › State machines".into(),
            filter: None,
            selected_item: None,
        });
        app.sessions = vec![session];
        app.mode = Mode::SessionPicker;
        app.current_service = Service::CloudWatchLogGroups;
        app.handle_action(action);
        assert_eq!(app.current_service, Service::StepFunctionsStateMachines);
        app.mode = Mode::Normal;
        app.handle_action(Action::Refresh);
        assert!(app.stepfunctions_state.machines.loading);
        assert!(!app.log_groups_state.loading);
        app.stepfunctions_state.machines.loading = false;
        app.mode = Mode::ErrorModal;
        app.error_message = Some("AccessDenied".into());
        app.handle_action(Action::RetryLoad);
        assert!(app.stepfunctions_state.machines.loading);
        assert!(app.error_message.is_none());
        assert_eq!(app.stepfunctions_state.machines.items.len(), 25);
    }
}

#[test]
fn finder_selection_and_clipboard_share_full_machine_identity() {
    let mut app = self::app();
    app.stepfunctions_state.machines.filter = "wf-024".into();
    let expected = format!("arn:aws:states:us-east-1:{ACCOUNT}:stateMachine:wf-024");
    assert_eq!(
        super::actions::selected_arn(&app).as_deref(),
        Some(expected.as_str())
    );
    press(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
    assert!(app.clipboard.complete_with(
        &ratatui::buffer::Buffer::empty(ratatui::layout::Rect::new(0, 0, 1, 1)),
        |text| {
            assert_eq!(text, expected);
            Ok(())
        }
    ));
    let resource = rusticity_core::finder::Resource::from_arn(&expected).unwrap();
    assert_eq!(
        resource.kind,
        rusticity_core::finder::ResourceKind::StateMachine
    );
    assert_eq!(resource.id, expected);
    assert_eq!(
        crate::finder::navigation::service(resource.kind),
        Service::StepFunctionsStateMachines
    );
}

#[test]
fn execution_preferences_expose_exact_requested_columns_in_order() {
    use executions::Column as ExecColumn;
    let mut app = self::app();
    app.stepfunctions_state.machines.filter = "wf-024".into();
    app.handle_action(Action::Select);
    press(&mut app, KeyCode::Char('p'), KeyModifiers::NONE);
    assert_eq!(app.mode, Mode::ColumnSelector);
    let prefs = model(&app);
    assert_eq!(
        prefs
            .columns()
            .map(|column| column.label.as_str())
            .collect::<Vec<_>>(),
        [
            "Name",
            "Status",
            "Start Time",
            "End Time",
            "Duration",
            "Version",
            "Alias",
            "Redrive count",
            "Last redrive time"
        ]
    );
    assert_eq!(
        prefs
            .columns()
            .map(|column| column.id.as_ref())
            .collect::<Vec<_>>(),
        ExecColumn::ids()
    );
    assert!(ExecColumn::from_id("column.stepfunctions.execution.arn").is_none());
    // ARN still belongs to inventory preferences, but not the execution schema.
    assert_eq!(
        Column::ALL.map(Column::default_name),
        ["Name", "Type", "Created (UTC)", "ARN"]
    );
    let redrive = prefs.rows().iter().position(|row| matches!(row, PreferenceRow::Column(column) if column.id == ExecColumn::RedriveCount.id())).unwrap();
    app.column_selector_index = redrive;
    app.handle_action(Action::ToggleColumn);
    assert!(app
        .stepfunctions_state
        .executions
        .visible_column_ids
        .contains(&ExecColumn::RedriveCount.id()));
}

#[test]
fn execution_metadata_columns_render_real_values_and_missing_dashes() {
    use executions::Column as C;
    let mut row = exec("run", "SUCCEEDED", Some(5000));
    row.state_machine_version_arn = format!("{}:42", row.state_machine_arn);
    row.state_machine_alias_arn = format!("{}:PROD", row.state_machine_arn);
    row.redrive_count = Some(2);
    row.redrive_date = "2026-10-16T22:00:03Z".into();
    assert_eq!(C::Version.render(&row).0, "42");
    assert_eq!(C::Alias.render(&row).0, "PROD");
    assert_eq!(C::RedriveCount.render(&row).0, "2");
    assert_eq!(C::LastRedriveTime.render(&row).0, "2026-10-16T22:00:03Z");
    row.redrive_count = Some(0);
    assert_eq!(C::RedriveCount.render(&row).0, "0");
    for column in C::ALL {
        assert_eq!(column.render(&ExecutionMetadata::default()).0, "-");
    }
    // Do not extract a plausible-looking qualifier from a different parent.
    row.state_machine_version_arn =
        "arn:aws:states:us-east-1:123456789012:stateMachine:other:9".into();
    assert_eq!(C::Version.render(&row).0, "-");
}

#[test]
fn execution_new_fields_filter_render_and_keep_arn_copy_and_table_cli() {
    use executions::Column as C;
    let mut app = self::app();
    app.stepfunctions_state.machines.filter = "wf-024".into();
    app.handle_action(Action::Select);
    let mut row = exec("run", "SUCCEEDED", Some(5000));
    row.state_machine_version_arn = format!("{}:42", row.state_machine_arn);
    row.state_machine_alias_arn = format!("{}:PROD", row.state_machine_arn);
    row.redrive_count = Some(987);
    row.redrive_date = "2026-10-16T22:00:03Z".into();
    let arn = row.arn.clone();
    app.stepfunctions_state.executions.table.items = vec![row];
    app.stepfunctions_state.executions.table.loading = false;
    app.stepfunctions_state.executions.loaded = true;
    app.stepfunctions_state.executions.visible_column_ids = C::ids();
    for query in ["PROD", "987", "2026-10-16", ":42"] {
        app.stepfunctions_state.executions.table.filter = query.into();
        assert_eq!(super::executions::filtered(&app).len(), 1);
    }
    app.stepfunctions_state.executions.table.filter.clear();
    let text = screen(&app, 300, 30);
    for label in [
        "Start Time",
        "End Time",
        "Version",
        "Alias",
        "Redrive count",
        "Last redrive time",
    ] {
        assert!(text.contains(label), "missing header {label}");
    }
    assert!(text.contains("PROD") && text.contains("987"));
    press(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
    assert!(app
        .clipboard
        .complete_with(&Buffer::empty(Rect::new(0, 0, 1, 1)), |text| {
            assert_eq!(text, arn);
            Ok(())
        }));
    assert_eq!(
        &crate::cli_command::invocation(&app).unwrap().args[..2],
        ["stepfunctions", "list-executions"]
    );
}
