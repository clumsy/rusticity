use super::*;
use crate::app::{Service, ViewMode};
use crate::keymap::handle_key;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rusticity_core::finder::ResourceKind;

fn app() -> App {
    App::new_without_client("test".into(), Some("us-east-2".into()))
}
fn resource(kind: ResourceKind, id: &str) -> Resource {
    Resource {
        kind,
        id: id.into(),
        name: id.into(),
        arn: String::new(),
        region: "us-east-2".into(),
    }
}
fn screen(app: &App) -> String {
    use ratatui::{backend::TestBackend, Terminal};
    let mut terminal = Terminal::new(TestBackend::new(180, 30)).unwrap();
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
fn finder_space_f_menu_and_typing_do_not_trigger_service_shortcuts() {
    let mut app = app();
    app.mode = Mode::Normal;
    app.handle_action(
        handle_key(
            KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
            app.mode,
        )
        .unwrap(),
    );
    assert!(screen(&app).contains("f find resource"));
    app.handle_action(
        handle_key(
            KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE),
            app.mode,
        )
        .unwrap(),
    );
    assert_eq!(app.mode, Mode::ResourceFinder);
    for c in "@dynamodb req".chars() {
        app.handle_action(
            handle_key(
                KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
                app.mode,
            )
            .unwrap(),
        );
    }
    assert_eq!(app.resource_finder.input, "@dynamodb req");
    assert!(app.running);
    assert!(app.resource_finder.query.as_ref().unwrap().ready());
    assert!(screen(&app).contains("AWS Resources"));
    assert!(screen(&app).contains("Resource type"));
    app.handle_action(Action::FilterBackspace);
    assert!(!app.resource_finder.query.as_ref().unwrap().ready());
}

#[test]
fn finder_stale_updates_are_ignored_and_new_results_keep_selection_stable() {
    let mut state = State {
        input: "orders".into(),
        ..Default::default()
    };
    state.changed();
    let old = state.generation;
    state.accept(
        old,
        SearchUpdate::Results(vec![resource(ResourceKind::Table, "orders")]),
    );
    state.selected = 0;
    state.accept(
        old,
        SearchUpdate::Results(vec![resource(ResourceKind::Bucket, "orders-archive")]),
    );
    assert_eq!(state.rows[state.selected].id, "orders");
    state.accept(
        old,
        SearchUpdate::Results(vec![resource(ResourceKind::Table, "orders")]),
    );
    assert_eq!(state.rows.len(), 2);
    state.input = "different".into();
    state.changed();
    state.accept(
        old,
        SearchUpdate::Results(vec![resource(ResourceKind::Table, "orders")]),
    );
    state.accept(old, SearchUpdate::Finished { limited: true });
    assert!(state.rows.is_empty());
    assert!(!state.limited);
}

#[tokio::test]
async fn finder_escape_cancels_search_and_preserves_underlying_view() {
    let mut app = app();
    app.current_service = Service::DynamoDbTables;
    app.service_selected = true;
    app.view_mode = ViewMode::List;
    app.dynamodb_state.tables.filter = "original".into();
    open(&mut app);
    let (tx, receiver) = mpsc::channel(1);
    let task = tokio::spawn(async move {
        std::future::pending::<()>().await;
        drop(tx);
    });
    let abort = task.abort_handle();
    app.resource_finder.running = Some(RunningSearch {
        task,
        receiver,
        generation: app.resource_finder.generation,
    });
    app.resource_finder.searching = true;
    app.handle_action(Action::CloseMenu);
    tokio::task::yield_now().await;
    assert!(abort.is_finished());
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.current_service, Service::DynamoDbTables);
    assert_eq!(app.dynamodb_state.tables.filter, "original");
    assert!(!app.resource_finder.searching);
}

#[tokio::test]
async fn finder_does_not_start_before_threshold_or_debounce() {
    let mut app = app();
    open(&mut app);
    for c in "@s3 ab".chars() {
        app.handle_action(Action::FilterInput(c));
    }
    app.resource_finder.changed = Instant::now() - Duration::from_secs(1);
    poll(&mut app);
    assert!(app.resource_finder.running.is_none());
    app.handle_action(Action::FilterInput('c'));
    poll(&mut app);
    assert!(app.resource_finder.running.is_none());
    assert!(app.resource_finder.query.as_ref().unwrap().ready());
}

#[test]
fn finder_profile_region_and_credentials_changes_drop_cached_results() {
    let mut app = app();
    open(&mut app);
    let first_cache = app.resource_finder.cache.clone();
    app.resource_finder
        .rows
        .push(resource(ResourceKind::Table, "orders"));
    app.config.region = "eu-west-1".into();
    open(&mut app);
    assert!(app.resource_finder.rows.is_empty());
    assert!(!Arc::ptr_eq(&first_cache, &app.resource_finder.cache));
    let cache = app.resource_finder.cache.clone();
    app.profile = "other".into();
    open(&mut app);
    assert!(!Arc::ptr_eq(&cache, &app.resource_finder.cache));
}

#[test]
fn finder_enter_selects_exact_resource_and_empty_results_do_nothing() {
    let mut app = app();
    open(&mut app);
    assert_eq!(app.resource_finder.focus, Focus::Filter);
    app.handle_action(Action::Select);
    assert_eq!(app.resource_finder.focus, Focus::Table);
    app.handle_action(Action::Select);
    assert!(app.resource_finder.pending_open.is_none());
    app.resource_finder.rows = vec![
        resource(ResourceKind::Table, "orders"),
        resource(ResourceKind::Bucket, "archive"),
    ];
    app.handle_action(Action::NextItem);
    app.handle_action(Action::Select);
    assert_eq!(
        app.resource_finder.pending_open.as_ref().unwrap().id,
        "archive"
    );
    assert!(app.resource_finder.opening);
    assert_eq!(
        navigation::service(ResourceKind::Table),
        Service::DynamoDbTables
    );
    assert_eq!(
        navigation::service(ResourceKind::Bucket),
        Service::S3Buckets
    );
    assert_eq!(
        navigation::service(ResourceKind::Database),
        Service::RdsInstances
    );
}

#[test]
fn finder_result_cap_and_errors_are_honest() {
    let mut state = State {
        input: "orders".into(),
        ..Default::default()
    };
    state.changed();
    state.accept(
        state.generation,
        SearchUpdate::Results(
            (0..RESULT_LIMIT + 5)
                .map(|i| resource(ResourceKind::Table, &format!("orders-{i}")))
                .collect(),
        ),
    );
    assert_eq!(state.rows.len(), RESULT_LIMIT);
    assert!(state.limited);
    assert!(state.message().contains("partial"));
    state.accept(
        state.generation,
        SearchUpdate::Warning("s3 AccessDenied".into()),
    );
    assert!(state.message().contains("AccessDenied"));
}

#[tokio::test]
async fn finder_rejects_cross_region_targets_without_changing_context() {
    let mut app = app();
    let old_service = app.current_service;
    let mut target = resource(ResourceKind::Table, "orders");
    target.region = "eu-west-1".into();
    assert!(navigation::open_resource(&mut app, target).await.is_err());
    assert_eq!(app.current_service, old_service);
    assert_eq!(app.config.region, "us-east-2");
}

fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    let action = handle_key(KeyEvent::new(code, modifiers), app.mode).unwrap();
    app.handle_action(action);
}

#[test]
fn finder_only_the_focused_pane_handles_edits_and_navigation() {
    let mut app = app();
    open(&mut app);
    for c in "@s3 test".chars() {
        press(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
    }
    app.resource_finder.rows = vec![
        resource(ResourceKind::Bucket, "test-one"),
        resource(ResourceKind::Bucket, "test-two"),
    ];
    for code in [
        KeyCode::Down,
        KeyCode::PageDown,
        KeyCode::Up,
        KeyCode::PageUp,
    ] {
        press(&mut app, code, KeyModifiers::NONE);
        assert_eq!(
            app.resource_finder.selected, 0,
            "Filter focus must not navigate the table"
        );
    }
    let generation = app.resource_finder.generation;
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(app.resource_finder.focus, Focus::Table);
    assert!(
        app.resource_finder.pending_open.is_none(),
        "Enter in filter only transfers focus"
    );
    assert!(!app.resource_finder.opening);
    press(&mut app, KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(app.resource_finder.selected, 1);
    for (code, modifiers) in [
        (KeyCode::Char('x'), KeyModifiers::NONE),
        (KeyCode::Backspace, KeyModifiers::NONE),
        (KeyCode::Char('w'), KeyModifiers::CONTROL),
    ] {
        press(&mut app, code, modifiers);
    }
    assert_eq!(app.resource_finder.input, "@s3 test");
    assert_eq!(app.resource_finder.rows.len(), 2);
    assert_eq!(app.resource_finder.generation, generation);
    press(&mut app, KeyCode::Char('i'), KeyModifiers::NONE);
    assert_eq!(app.resource_finder.focus, Focus::Filter);
    assert_eq!(app.resource_finder.selected, 1);
    press(&mut app, KeyCode::Char('i'), KeyModifiers::NONE);
    assert_eq!(
        app.resource_finder.input, "@s3 testi",
        "i is literal text when filter already focused"
    );
    assert!(
        app.resource_finder.rows.is_empty(),
        "An actual edit invalidates old results"
    );
}

#[tokio::test]
async fn finder_focus_switches_preserve_search_and_accept_streaming_results() {
    let mut app = app();
    open(&mut app);
    app.resource_finder.input = "orders".into();
    app.resource_finder.changed();
    let generation = app.resource_finder.generation;
    let (tx, receiver) = mpsc::channel(1);
    let task = tokio::spawn(async move {
        std::future::pending::<()>().await;
        drop(tx);
    });
    let abort = task.abort_handle();
    app.resource_finder.running = Some(RunningSearch {
        task,
        receiver,
        generation,
    });
    app.resource_finder.searching = true;
    app.resource_finder.rows = vec![resource(ResourceKind::Table, "orders")];
    for (code, modifiers, expected) in [
        (KeyCode::Tab, KeyModifiers::NONE, Focus::Table),
        (KeyCode::BackTab, KeyModifiers::SHIFT, Focus::Filter),
        (KeyCode::Tab, KeyModifiers::SHIFT, Focus::Table),
        (KeyCode::Char('/'), KeyModifiers::NONE, Focus::Filter),
        (KeyCode::Enter, KeyModifiers::NONE, Focus::Table),
    ] {
        press(&mut app, code, modifiers);
        assert_eq!(app.resource_finder.focus, expected);
        assert_eq!(app.resource_finder.generation, generation);
        assert_eq!(app.resource_finder.input, "orders");
        assert!(app.resource_finder.searching);
        assert!(!abort.is_finished());
    }
    app.resource_finder.accept(
        generation,
        SearchUpdate::Results(vec![resource(ResourceKind::Bucket, "orders-archive")]),
    );
    assert_eq!(app.resource_finder.rows.len(), 2);
    assert_eq!(app.resource_finder.focus, Focus::Table);
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    tokio::task::yield_now().await;
    assert!(abort.is_finished());
    assert_eq!(app.mode, Mode::Normal);
    open(&mut app);
    assert_eq!(
        app.resource_finder.focus,
        Focus::Filter,
        "Every open starts with the filter focused"
    );
}

#[test]
fn finder_renders_exactly_one_active_pane_and_no_duplicate_context_in_title() {
    use ratatui::{backend::TestBackend, prelude::*, Terminal};
    let mut app = app();
    app.profile = "finder-test-profile".into();
    open(&mut app);
    app.resource_finder.rows = vec![resource(ResourceKind::Table, "orders")];
    for focus in [Focus::Filter, Focus::Table] {
        app.resource_finder.focus = focus;
        let mut terminal = Terminal::new(TestBackend::new(180, 30)).unwrap();
        terminal
            .draw(|f| crate::ui::finder::render(f, &app, f.area()))
            .unwrap();
        let area = Rect::new(0, 0, 180, 30)
            .centered(Constraint::Percentage(90), Constraint::Percentage(80));
        let buffer = terminal.backend().buffer();
        assert_eq!(
            buffer[(area.x, area.y)].fg == Color::Green,
            focus == Focus::Filter
        );
        assert_eq!(
            buffer[(area.x, area.y + 3)].fg == Color::Green,
            focus == Focus::Table
        );
        let input: String = (area.x..area.right())
            .map(|x| buffer[(x, area.y + 1)].symbol())
            .collect();
        assert_eq!(input.contains('█'), focus == Focus::Filter);
        let title: String = (area.x..area.right())
            .map(|x| buffer[(x, area.y + 3)].symbol())
            .collect();
        assert!(title.contains("AWS Resources"));
        assert!(!title.contains("Find resource"));
        assert!(!title.contains("finder-test-profile"));
        assert!(!title.contains("us-east-2"));
        let full = screen(&app);
        if focus == Focus::Filter {
            assert!(full.contains(" INSERT "));
            assert!(full.contains("⏎ results"));
            assert!(!full.contains("Enter / Tab: results"));
        } else {
            assert!(full.contains(" NORMAL "));
            assert!(full.contains("i filter"));
            assert!(!full.contains("i / Tab: filter"));
        }
    }
}

#[test]
fn finder_refresh_keeps_focus_and_escape_cancels_from_either_pane() {
    for focus in [Focus::Filter, Focus::Table] {
        let mut app = app();
        open(&mut app);
        app.resource_finder.focus = focus;
        press(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL);
        assert_eq!(app.resource_finder.focus, focus);
        assert_eq!(app.mode, Mode::ResourceFinder);
        press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.resource_finder.pending_open.is_none());
    }
}
