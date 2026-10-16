use super::*;
use crate::{finder::Focus, keymap::handle_key};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

fn app() -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    app.mode = Mode::Normal;
    app.service_selected = true;
    app.current_service = Service::SnsTopics;
    app.config.account_id = "123456789012".into();
    app.sns_state.topics.items =
        vec![
            rusticity_core::SnsTopic::from_arn("arn:aws:sns:us-east-1:123456789012:test").unwrap(),
        ];
    app.tabs.push(Tab {
        service: Service::SnsTopics,
        title: "SNS › Topics".into(),
        breadcrumb: "SNS › Topics".into(),
    });
    app
}
fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    let key = KeyEvent::new(code, modifiers);
    if !super::handle_key(app, key) {
        if let Some(action) = handle_key(key, app.mode) {
            app.handle_action(action);
        }
    }
}
fn open(app: &mut App) -> u64 {
    press(app, KeyCode::Char('e'), KeyModifiers::CONTROL);
    active_id(app).unwrap()
}
fn view(app: &App) -> &View {
    app.cli_output.views.get(&active_id(app).unwrap()).unwrap()
}
fn screen(app: &App, width: u16, height: u16) -> (String, bool) {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| ui::render(f, app)).unwrap();
    let buffer = terminal.backend().buffer();
    (
        buffer.content().iter().map(|c| c.symbol()).collect(),
        buffer.content().iter().any(|c| c.fg == Color::Blue),
    )
}
async fn finish(app: &mut App, id: u64, result: Result<String, String>) {
    let view = app.cli_output.views.get_mut(&id).unwrap();
    view.pending = false;
    view.running = Some(Running(tokio::spawn(async { result })));
    tokio::task::yield_now().await;
    assert!(poll(app).await);
}

#[test]
fn opens_a_real_tab_with_same_command_without_changing_source() {
    let mut app = app();
    let invocation = cli_command::invocation(&app).unwrap();
    assert!(screen(&app, 180, 35).0.contains("^e CLI output"));
    let id = open(&mut app);
    assert_eq!(app.tabs.len(), 2);
    assert_eq!(app.tabs[1].service, Service::CliOutput(id));
    assert_eq!(app.tabs[0].service, Service::SnsTopics);
    assert!(view(&app).pending);
    assert!(view(&app).running.is_none());
    assert_eq!(
        view(&app).invocation.as_ref().unwrap().args,
        invocation.args
    );
    assert!(!app.sns_state.topics.loading);
    assert!(screen(&app, 180, 35).0.contains("Loading CLI JSON"));
    press(&mut app, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!(app.current_service, Service::SnsTopics);
    assert!(app.cli_output.views.contains_key(&id));
    press(&mut app, KeyCode::Char(']'), KeyModifiers::NONE);
    assert_eq!(active_id(&app), Some(id));
    press(&mut app, KeyCode::Char('w'), KeyModifiers::CONTROL);
    assert_eq!(app.tabs.len(), 1);
    assert_eq!(app.current_service, Service::SnsTopics);
    assert!(!app.cli_output.views.contains_key(&id));
}

#[tokio::test]
async fn common_json_renderer_scrolling_and_copy_work_in_output_tab() {
    let mut app = app();
    let id = open(&mut app);
    let text =
        serde_json::to_string_pretty(&serde_json::json!({"values":(0..40).collect::<Vec<_>>()}))
            .unwrap();
    finish(&mut app, id, Ok(text.clone())).await;
    let (rendered, highlight) = screen(&app, 180, 35);
    assert!(rendered.contains("CLI output · JSON"));
    assert!(rendered.contains("values"));
    assert!(highlight);
    press(&mut app, KeyCode::Char('d'), KeyModifiers::CONTROL);
    assert_eq!(view(&app).scroll, 20);
    press(&mut app, KeyCode::Char('u'), KeyModifiers::CONTROL);
    assert_eq!(view(&app).scroll, 0);
    let buffer = Buffer::empty(Rect::new(0, 0, 1, 1));
    press(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
    assert!(app.clipboard.complete_with(&buffer, |copied| {
        assert_eq!(copied, text);
        Ok(())
    }));
    press(&mut app, KeyCode::Char('y'), KeyModifiers::CONTROL);
    assert!(app.clipboard.complete_with(&buffer, |copied| {
        assert!(copied.starts_with("aws sns list-topics"));
        Ok(())
    }));
    press(&mut app, KeyCode::Char('p'), KeyModifiers::CONTROL);
    assert!(app.clipboard.snapshot_requested());
    for (w, h) in [(20, 5), (80, 15)] {
        screen(&app, w, h);
    }
}

#[tokio::test]
async fn multiple_output_tabs_finish_in_background_and_keep_independent_results() {
    let mut app = app();
    let first = open(&mut app);
    app.cli_output.views.get_mut(&first).unwrap().pending = false;
    press(&mut app, KeyCode::Char('['), KeyModifiers::NONE);
    let second = open(&mut app);
    app.cli_output.views.get_mut(&second).unwrap().pending = false;
    assert_ne!(first, second);
    assert_eq!(app.tabs.len(), 3);
    finish(&mut app, first, Ok("{\"first\":1}".into())).await;
    finish(&mut app, second, Ok("{\"second\":2}".into())).await;
    assert!(view(&app).text.contains("second"));
    press(&mut app, KeyCode::Char('['), KeyModifiers::NONE);
    assert!(view(&app).text.contains("first"));
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(active_id(&app), Some(second));
    assert!(view(&app).text.contains("second"));
    assert!(!app.cli_output.views.contains_key(&first));
}

#[tokio::test]
async fn finder_result_opens_separate_output_tab_without_opening_service_resource() {
    let mut app = app();
    app.mode = Mode::ResourceFinder;
    app.resource_finder.focus = Focus::Table;
    app.resource_finder.input = "@sns test".into();
    app.resource_finder.rows = vec![rusticity_core::finder::Resource {
        kind: rusticity_core::finder::ResourceKind::Topic,
        id: "test".into(),
        name: "test".into(),
        arn: "arn:aws:sns:us-east-1:123456789012:test".into(),
        region: "us-east-1".into(),
    }];
    let id = open(&mut app);
    finish(&mut app, id, Ok("{}".into())).await;
    assert!(app.resource_finder.pending_open.is_none());
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.resource_finder.input, "@sns test");
    assert_eq!(app.tabs.len(), 2);
}

#[tokio::test]
async fn errors_refresh_and_context_changes_never_retarget_a_captured_command() {
    let mut app = app();
    let id = open(&mut app);
    finish(&mut app, id, Err("ExpiredToken\x1b".into())).await;
    assert!(view(&app).failed);
    assert!(!view(&app).text.contains('\x1b'));
    let args = view(&app).invocation.as_ref().unwrap().args.clone();
    press(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL);
    assert!(view(&app).pending);
    assert_eq!(view(&app).invocation.as_ref().unwrap().args, args);
    app.profile = "other".into();
    assert!(poll(&mut app).await);
    assert!(view(&app).invocation.is_none());
    assert!(!app.cli_output.busy());
    assert!(view(&app).text.contains("context changed"));
    press(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL);
    assert!(!app.cli_output.busy());
}

#[tokio::test]
async fn closing_output_tab_or_clearing_tabs_aborts_running_task() {
    for clear_all in [false, true] {
        let mut app = app();
        let id = open(&mut app);
        let task = tokio::spawn(std::future::pending::<Result<String, String>>());
        let abort = task.abort_handle();
        let v = app.cli_output.views.get_mut(&id).unwrap();
        v.pending = false;
        v.running = Some(Running(task));
        if clear_all {
            app.tabs.clear();
            poll(&mut app).await;
        } else {
            press(&mut app, KeyCode::Char('w'), KeyModifiers::CONTROL);
        }
        tokio::task::yield_now().await;
        assert!(abort.is_finished());
        assert!(app.cli_output.views.is_empty());
    }
}

#[test]
fn controls_do_not_open_parent_preferences_or_mutate_source_and_shortcuts_remain() {
    let mut app = app();
    open(&mut app);
    for key in [
        KeyCode::Char('i'),
        KeyCode::Char('p'),
        KeyCode::Enter,
        KeyCode::Tab,
    ] {
        press(&mut app, key, KeyModifiers::NONE);
    }
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.sns_state.topics.selected, 0);
    let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
    assert_eq!(handle_key(key, Mode::Normal), Some(Action::OpenCliOutput));
    assert_eq!(handle_key(key, Mode::FilterInput), None);
    assert_eq!(
        handle_key(
            KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
            Mode::Normal
        ),
        Some(Action::ToggleExactMatch)
    );
    assert!(!app
        .service_picker
        .services
        .iter()
        .any(|name| name.contains("CLI output")));
    assert!(screen(&app, 240, 35).0.contains("^w/⎋ close / cancel"));
}

#[test]
fn every_service_opens_an_independent_output_tab_or_explicit_unsupported_result() {
    for service in [
        Service::ApiGatewayApis,
        Service::CloudWatchLogGroups,
        Service::CloudWatchInsights,
        Service::CloudWatchAlarms,
        Service::CloudTrailEvents,
        Service::RdsInstances,
        Service::S3Buckets,
        Service::SnsTopics,
        Service::SqsQueues,
        Service::VpcVpcs,
        Service::Ec2Instances,
        Service::EcrRepositories,
        Service::KmsKeys,
        Service::EfsFileSystems,
        Service::FsxFileSystems,
        Service::DynamoDbTables,
        Service::LambdaFunctions,
        Service::LambdaApplications,
        Service::CloudFormationStacks,
        Service::IamUsers,
        Service::IamRoles,
        Service::IamUserGroups,
    ] {
        let mut app = app();
        app.current_service = service;
        app.tabs[0].service = service;
        let id = open(&mut app);
        assert_eq!(app.tabs[0].service, service);
        assert_eq!(app.tabs[1].service, Service::CliOutput(id));
        assert!(view(&app).pending || view(&app).failed);
        press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(app.current_service, service);
        assert_eq!(app.tabs.len(), 1);
    }
}

#[test]
fn shared_tab_picker_and_g_prefix_are_not_swallowed_by_output_view() {
    let mut app = app();
    let id = open(&mut app);
    assert!(!super::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE)
    ));
    app.pending_key = Some('g');
    assert!(!super::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE)
    ));
    app.pending_key = None;
    app.handle_action(Action::OpenTabPicker);
    assert_eq!(app.mode, Mode::TabPicker);
    assert!(app
        .tabs
        .iter()
        .any(|tab| tab.service == Service::CliOutput(id)));
}
