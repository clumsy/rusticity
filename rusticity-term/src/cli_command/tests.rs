use super::*;
use crate::{ecr::image::Image, keymap::handle_key};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, buffer::Buffer, layout::Rect, Terminal};
use rusticity_core::finder::{Resource, ResourceKind};

fn app(service: Service) -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    app.current_service = service;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.config.account_id = "123456789012".into();
    app
}
fn press(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    if let Some(action) = handle_key(KeyEvent::new(key, modifiers), app.mode) {
        app.handle_action(action);
    }
}
fn copy(app: &mut App) -> String {
    press(app, KeyCode::Char('y'), KeyModifiers::CONTROL);
    let mut copied = String::new();
    assert!(app
        .clipboard
        .complete_with(&Buffer::empty(Rect::new(0, 0, 1, 1)), |text| {
            copied = text;
            Ok(())
        }));
    copied
}
fn screen(app: &App, width: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, 35)).unwrap();
    terminal.draw(|frame| ui::render(frame, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}
fn topic(name: &str) -> rusticity_core::SnsTopic {
    rusticity_core::SnsTopic::from_arn(&format!("arn:aws:sns:us-east-1:123456789012:{name}"))
        .unwrap()
}
fn image(tag: &str, digest: &str) -> Image {
    Image {
        tag: tag.into(),
        digest: digest.into(),
        uri: format!("registry/repo:{tag}"),
        artifact_type: String::new(),
        pushed_at: String::new(),
        last_pull_time: String::new(),
        size_bytes: 0,
    }
}

#[test]
fn ctrl_y_has_its_own_action_and_does_not_type_into_filters() {
    let key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL);
    assert_eq!(handle_key(key, Mode::Normal), Some(Action::CopyCliCommand));
    assert_eq!(
        handle_key(key, Mode::ResourceFinder),
        Some(Action::CopyCliCommand)
    );
    for mode in [
        Mode::FilterInput,
        Mode::ServicePicker,
        Mode::InsightsInput,
        Mode::ProfilePicker,
        Mode::ColumnSelector,
    ] {
        assert_eq!(handle_key(key, mode), None);
    }
    assert_eq!(
        handle_key(
            KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE),
            Mode::Normal
        ),
        Some(Action::Yank)
    );
}

#[test]
fn filtered_sns_row_copies_command_with_current_profile_and_region_without_loading() {
    let mut app = app(Service::SnsTopics);
    app.sns_state.topics.items = vec![topic("other"), topic("chosen")];
    app.sns_state.topics.filter = "CHOSEN".into();
    let before = app.sns_state.topics.items.clone();
    let text = copy(&mut app);
    assert!(text.starts_with("aws sns get-topic-attributes "));
    assert!(text.contains("123456789012:chosen"));
    assert!(!text.contains(":other"));
    assert!(text.contains("--profile='beta'"));
    assert!(text.contains("--region='us-east-1'"));
    assert_eq!(app.sns_state.topics.items, before);
    assert_eq!(app.sns_state.topics.filter, "CHOSEN");
    assert!(!app.sns_state.topics.loading);
    assert!(!app.log_groups_state.loading);
    assert_eq!(app.mode, Mode::Normal);
    app.clipboard = Default::default(); // Dismiss the transient copy notice to inspect hints.
    assert!(screen(&app, 240).contains("^y copy CLI"));
    // Existing resource yank remains an ARN, not a CLI command.
    press(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
    app.clipboard
        .complete_with(&Buffer::empty(Rect::new(0, 0, 1, 1)), |text| {
            assert_eq!(text, "arn:aws:sns:us-east-1:123456789012:chosen");
            Ok(())
        });
}

#[test]
fn finder_copy_targets_focused_result_not_underlying_service() {
    let mut app = app(Service::SnsTopics);
    app.mode = Mode::ResourceFinder;
    app.resource_finder.focus = Focus::Table;
    app.resource_finder.rows = vec![Resource {
        kind: ResourceKind::Instance,
        id: "i-123".into(),
        name: "Friendly".into(),
        arn: String::new(),
        region: "us-east-1".into(),
    }];
    let text = copy(&mut app);
    assert!(text.starts_with("aws ec2 describe-instances "));
    assert!(text.contains("i-123"));
    assert!(app.resource_finder.pending_open.is_none());
    assert_eq!(app.mode, Mode::ResourceFinder);
    app.clipboard = Default::default(); // Dismiss the transient copy notice to inspect hints.
    assert!(screen(&app, 240).contains("^y copy CLI"));
    app.resource_finder.focus = Focus::Filter;
    assert!(!shortcut_visible(&app));
    press(&mut app, KeyCode::Char('y'), KeyModifiers::CONTROL);
    assert!(app.clipboard.notice().unwrap().failed);
    assert!(app.resource_finder.input.is_empty());
}

#[test]
fn unsupported_and_empty_selections_report_errors_without_copying_stale_text() {
    let mut app = app(Service::CloudTrailEvents);
    app.clipboard.request_text("stale request");
    press(&mut app, KeyCode::Char('y'), KeyModifiers::CONTROL);
    assert!(app.clipboard.notice().unwrap().failed);
    assert!(!app
        .clipboard
        .complete_with(&Buffer::empty(Rect::new(0, 0, 1, 1)), |_| panic!(
            "must not touch clipboard"
        )));
    app.current_service = Service::SnsTopics;
    press(&mut app, KeyCode::Char('y'), KeyModifiers::CONTROL);
    assert!(app
        .clipboard
        .notice()
        .unwrap()
        .message
        .contains("No supported resource"));
    assert!(!app.sns_state.topics.loading);
}

#[test]
fn ecr_image_list_and_open_detail_use_immutable_digest_and_intercept_service_actions() {
    let mut app = app(Service::EcrRepositories);
    app.ecr_state.current_repository = Some("team/repo".into());
    app.ecr_state.images.items = vec![
        image("ignore", "sha256:other"),
        image("chosen", "sha256:correct"),
    ];
    app.ecr_state.images.filter = "chosen".into();
    let list = copy(&mut app);
    assert!(list.starts_with("aws ecr describe-images "));
    assert!(list.contains("sha256:correct"));
    assert!(list.contains("team/repo"));
    assert!(!app.ecr_state.repositories.loading);
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    app.ecr_state.images.items[1].digest = "sha256:changed".into();
    for _ in 0..5 {
        assert_eq!(copy(&mut app), list);
        app.clipboard = Default::default(); // Dismiss the transient copy notice to inspect hints.
        assert!(screen(&app, 100).contains("^y copy CLI"));
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    }
}

#[test]
fn s3_object_uses_visible_key_and_bucket_region_while_folder_refuses() {
    let mut app = app(Service::S3Buckets);
    app.s3_state.current_bucket = Some("bucket".into());
    app.s3_state.object_tab = ui::s3::ObjectTab::Objects;
    app.s3_state.buckets.items = vec![crate::s3::Bucket {
        name: "bucket".into(),
        region: "eu-west-1".into(),
        creation_date: String::new(),
    }];
    app.s3_state.objects = vec![crate::s3::Object {
        key: "file://literal's $(text)".into(),
        is_prefix: false,
        size: 0,
        last_modified: String::new(),
        storage_class: String::new(),
    }];
    let text = copy(&mut app);
    assert!(text.starts_with("aws s3api head-object "));
    assert!(text.contains("--region='eu-west-1'"));
    assert!(text.contains("file://literal"));
    assert!(!app.s3_state.object_view.busy());
    app.clipboard = Default::default(); // Dismiss the transient copy notice to inspect hints.
    assert!(screen(&app, 100).contains("^y copy CLI"));
    app.s3_state.objects[0].is_prefix = true;
    press(&mut app, KeyCode::Char('y'), KeyModifiers::CONTROL);
    assert!(app
        .clipboard
        .notice()
        .unwrap()
        .message
        .contains("folder prefix"));
}

#[test]
fn api_protocol_comes_from_selected_or_open_api() {
    let mut app = app(Service::ApiGatewayApis);
    app.apig_state.apis.items = vec![crate::apig::api::RestApi {
        id: "http-api".into(),
        name: "chosen".into(),
        protocol_type: "HTTP".into(),
        description: String::new(),
        created_date: String::new(),
        api_key_source: String::new(),
        endpoint_configuration: String::new(),
        disable_execute_api_endpoint: false,
        status: String::new(),
    }];
    assert!(copy(&mut app).starts_with("aws apigatewayv2 get-api "));
    app.apig_state.current_api = Some(app.apig_state.apis.items[0].clone());
    app.apig_state.current_api.as_mut().unwrap().protocol_type = "REST".into();
    app.apig_state.current_api.as_mut().unwrap().id = "rest-api".into();
    let text = copy(&mut app);
    assert!(text.starts_with("aws apigateway get-rest-api "));
    assert!(text.contains("rest-api"));
}

#[test]
fn current_profile_changes_are_reflected_and_foreign_finder_accounts_are_refused() {
    let mut app = app(Service::SnsTopics);
    app.sns_state.topics.items = vec![topic("chosen")];
    assert!(copy(&mut app).contains("--profile='beta'"));
    app.profile = "different".into();
    assert!(copy(&mut app).contains("--profile='different'"));
    app.config.account_id = "999999999999".into();
    press(&mut app, KeyCode::Char('y'), KeyModifiers::CONTROL);
    assert!(app
        .clipboard
        .notice()
        .unwrap()
        .message
        .contains("switch profiles"));
}
