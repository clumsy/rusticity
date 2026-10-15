use super::*;
use crate::keymap::handle_key;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};

fn object(key: &str, folder: bool) -> Object {
    Object {
        key: key.into(),
        is_prefix: folder,
        size: 12,
        last_modified: String::new(),
        storage_class: "STANDARD".into(),
    }
}
fn app() -> App {
    let mut app = App::new_without_client("test".into(), Some("us-east-1".into()));
    app.current_service = Service::S3Buckets;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.s3_state.current_bucket = Some("bucket".into());
    app.s3_state.objects = vec![
        object("ignored.txt", false),
        object("folder/", true),
        object("note.json", false),
    ];
    app
}
fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    app.handle_action(handle_key(KeyEvent::new(code, modifiers), app.mode).unwrap());
}
fn preview() -> TextPreview {
    TextPreview {
        text: "{\n  \"enabled\": true\n}".into(),
        format: TextFormat::Json,
        truncated: false,
    }
}

#[test]
fn object_enter_previews_the_filtered_row_and_folder_enter_still_navigates() {
    let mut app = app();
    app.s3_state.object_filter = "note".into();
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    let pending = app.s3_state.object_view.pending.as_ref().unwrap();
    assert_eq!(pending.key, "note.json");
    assert!(pending.destination.is_none());
    assert!(!app.s3_state.buckets.loading);
    assert!(app.s3_state.prefix_stack.is_empty());
    app.s3_state.object_view = State::default();
    app.s3_state.object_filter = "folder".into();
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.s3_state.object_view.pending.is_none());
    assert_eq!(app.s3_state.prefix_stack, ["folder/"]);
    assert!(app.s3_state.buckets.loading);
}

#[test]
fn ctrl_d_downloads_only_in_objects_list_and_uses_the_expanded_visible_row() {
    let mut app = app();
    app.s3_state.object_filter = "folder".into();
    app.s3_state.expanded_prefixes.insert("folder/".into());
    app.s3_state
        .prefix_preview
        .insert("folder/".into(), vec![object("folder/nested.txt", false)]);
    app.s3_state.selected_object = 1;
    assert_eq!(app.s3_state.calculate_total_object_rows(), 2);
    press(&mut app, KeyCode::Char('d'), KeyModifiers::CONTROL);
    let pending = app.s3_state.object_view.pending.as_ref().unwrap();
    assert_eq!(pending.key, "folder/nested.txt");
    assert_eq!(pending.bucket, "bucket");
    assert!(pending.destination.as_ref().unwrap().is_absolute());
    assert_eq!(app.s3_state.selected_object, 1);
    // A second press cannot launch a second write.
    press(&mut app, KeyCode::Char('d'), KeyModifiers::CONTROL);
    assert!(app
        .s3_state
        .object_view
        .notice
        .as_ref()
        .unwrap()
        .message
        .contains("progress"));
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert!(!app.s3_state.object_view.busy());
    assert_eq!(app.s3_state.current_bucket.as_deref(), Some("bucket"));
    app.s3_state.object_tab = ObjectTab::Properties;
    assert!(!handle_action(&mut app, &Action::PageDown));
    app.current_service = Service::LambdaFunctions;
    assert!(!handle_action(&mut app, &Action::PageDown));
}

#[test]
fn stale_selection_and_folders_never_queue_downloads() {
    let mut app = app();
    app.s3_state.selected_object = usize::MAX;
    press(&mut app, KeyCode::Char('d'), KeyModifiers::CONTROL);
    assert!(!app.s3_state.object_view.busy());
    app.s3_state.selected_object = 1;
    press(&mut app, KeyCode::Char('d'), KeyModifiers::CONTROL);
    assert!(!app.s3_state.object_view.busy());
}

#[test]
fn preview_uses_common_highlighting_scrolls_and_esc_restores_objects() {
    let mut app = app();
    app.s3_state.selected_object = 2;
    app.s3_state.object_filter = "note".into();
    app.s3_state.object_view.preview = Some(Preview {
        key: "note.json".into(),
        content: preview(),
        scroll: 0,
    });
    let mut terminal = Terminal::new(TestBackend::new(100, 20)).unwrap();
    terminal
        .draw(|frame| ui::s3::render_buckets(frame, &app, frame.area()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let text: String = buffer.content().iter().map(|cell| cell.symbol()).collect();
    assert!(text.contains("note.json"));
    assert!(text.contains("1 │ {"));
    assert!(text.contains("\"enabled\""));
    assert!(buffer.content().iter().any(|cell| cell.fg == Color::Blue));
    press(&mut app, KeyCode::Char('d'), KeyModifiers::CONTROL);
    assert_eq!(app.s3_state.object_view.preview.as_ref().unwrap().scroll, 2);
    assert!(!app.s3_state.object_view.busy());
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert!(app.s3_state.object_view.preview.is_none());
    assert_eq!(app.s3_state.selected_object, 2);
    assert_eq!(app.s3_state.object_filter, "note");
    assert_eq!(app.s3_state.current_bucket.as_deref(), Some("bucket"));
}

#[tokio::test]
async fn completed_preview_is_applied_and_scope_changes_abort_stale_work() {
    let mut app = app();
    let captured = scope(&app);
    app.s3_state.object_view.scope = captured.clone();
    let task = tokio::spawn(async { Ok(Output::Preview(preview())) });
    while !task.is_finished() {
        tokio::task::yield_now().await;
    }
    app.s3_state.object_view.running = Some(Running {
        task,
        key: "note.json".into(),
        scope: captured.clone(),
    });
    assert!(poll(&mut app).await);
    assert!(app.s3_state.object_view.preview.is_some());
    let task = tokio::spawn(async { std::future::pending::<Result<Output, String>>().await });
    let abort = task.abort_handle();
    app.s3_state.object_view.running = Some(Running {
        task,
        key: "note.json".into(),
        scope: captured,
    });
    app.profile = "other-profile".into();
    assert!(poll(&mut app).await);
    tokio::task::yield_now().await;
    assert!(abort.is_finished());
    assert!(app.s3_state.object_view.preview.is_none());
    assert!(!app.s3_state.object_view.busy());
}

#[tokio::test]
async fn transfer_errors_are_visible_without_replacing_aws_error_state() {
    let mut app = app();
    app.error_message = Some("unrelated AWS error".into());
    let captured = scope(&app);
    app.s3_state.object_view.scope = captured.clone();
    let task = tokio::spawn(async { Err("AccessDenied".into()) });
    while !task.is_finished() {
        tokio::task::yield_now().await;
    }
    app.s3_state.object_view.running = Some(Running {
        task,
        key: "note.json".into(),
        scope: captured,
    });
    assert!(poll(&mut app).await);
    let notice = app.s3_state.object_view.notice.as_ref().unwrap();
    assert!(notice.failed);
    assert!(notice.message.contains("AccessDenied"));
    assert_eq!(app.error_message.as_deref(), Some("unrelated AWS error"));
    assert_eq!(app.mode, Mode::Normal);
}
