use super::*;
use crate::{
    app::{App, Service, ViewMode},
    common::{InputFocus, PageSize},
    keymap::{handle_key, Action, Mode},
    preferences::{bindings, PreferenceRow},
    ui::sns::filtered_topics,
};
use columns::Column;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, buffer::Buffer, layout::Rect, Terminal};
use rusticity_core::{
    finder::{Resource, ResourceKind},
    SnsTopic,
};

fn app() -> App {
    let mut app = App::new_without_client("test".into(), Some("us-east-1".into()));
    app.current_service = Service::SnsTopics;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.config.account_id = "123456789012".into();
    app.sns_state.topics.items = (0..25)
        .rev()
        .map(|i| {
            SnsTopic::from_arn(&format!(
                "arn:aws:sns:us-east-1:123456789012:topic-{i:03}{}",
                if i % 2 == 0 { ".fifo" } else { "" }
            ))
            .unwrap()
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
    bindings::model_for(app, Service::SnsTopics)
}

#[test]
fn sns_registration_and_picker_selection_are_complete() {
    let mut app = App::new_without_client("test".into(), Some("us-east-1".into()));
    assert_eq!(Service::SnsTopics.name(), "SNS › Topics");
    let services = &app.service_picker.services;
    let sns = services.iter().position(|s| *s == "SNS › Topics").unwrap();
    assert_eq!(services[sns - 1], "S3 › Buckets");
    assert_eq!(services[sns + 1], "SQS › Queues");
    app.mode = Mode::ServicePicker;
    app.service_picker.filter = "sns".into();
    app.service_picker.filter_active = false;
    app.handle_action(Action::Select);
    assert_eq!(app.current_service, Service::SnsTopics);
    assert_eq!(app.mode, Mode::Normal);
    assert!(app.sns_state.topics.loading);
    assert_eq!(app.tabs[0].service, Service::SnsTopics);
    assert!(app.breadcrumbs().contains("SNS"));
    assert_eq!(app.sns_state.column_ids, Column::ids());
    assert!(app.sns_state.topics.items.is_empty());
}

#[test]
fn sns_columns_are_shared_with_preferences_and_i18n() {
    let app = app();
    let mut labels = HashMap::new();
    init(&mut labels);
    assert_eq!(
        Column::ALL.map(Column::default_name),
        ["Name", "Type", "ARN"]
    );
    for col in Column::ALL {
        assert!(col.id().starts_with("column.sns.topic."));
        assert_eq!(Column::from_id(col.id()), Some(col));
        assert_eq!(labels[col.id()], col.default_name());
        assert_eq!(col.render(&SnsTopic::default()).0, "-");
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
fn sns_filter_sort_selection_copy_and_console_use_the_same_visible_row() {
    let mut app = app();
    assert_eq!(filtered_topics(&app)[0].name, "topic-000.fifo");
    for (filter, count) in [
        ("FIFO", 13),
        ("standard", 12),
        ("123456789012", 25),
        ("topic-024", 1),
        ("missing", 0),
    ] {
        app.sns_state.topics.filter = filter.into();
        assert_eq!(filtered_topics(&app).len(), count);
    }
    app.sns_state.topics.filter = "topic-024".into();
    let arn = "arn:aws:sns:us-east-1:123456789012:topic-024.fifo";
    assert_eq!(actions::selected_arn(&app).as_deref(), Some(arn));
    assert_eq!(app.get_console_url(), console_url("us-east-1", Some(arn)));
    press(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
    assert!(app
        .clipboard
        .complete_with(&Buffer::empty(Rect::new(0, 0, 1, 1)), |text| {
            assert_eq!(text, arn);
            Ok(())
        }));
    app.sns_state.topics.selected = usize::MAX;
    assert!(actions::selected_arn(&app).is_none());
    assert!(app.get_console_url().ends_with("#/topics"));
}

#[test]
fn sns_uses_shared_inventory_input_even_with_stale_legacy_view_modes() {
    for view_mode in [
        ViewMode::List,
        ViewMode::Detail,
        ViewMode::Events,
        ViewMode::InsightsResults,
    ] {
        let mut app = app();
        app.view_mode = view_mode;
        app.sns_state.topics.page_size = PageSize::Ten;
        app.handle_action(Action::NextItem);
        app.handle_action(Action::ExpandRow);
        assert_eq!(app.sns_state.topics.expanded_item, Some(1));
        app.handle_action(Action::PageDown);
        assert_eq!(app.sns_state.topics.selected, 11);
        assert!(app.sns_state.topics.expanded_item.is_none());
        press(&mut app, KeyCode::Char('i'), KeyModifiers::NONE);
        for c in "TOPIC".chars() {
            press(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
        }
        assert_eq!(app.sns_state.topics.selected, 0);
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(app.sns_state.input_focus, InputFocus::Pagination);
        press(&mut app, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(app.sns_state.topics.selected, 10);
        press(&mut app, KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(app.sns_state.topics.selected, 0);
        press(&mut app, KeyCode::Char('3'), KeyModifiers::NONE);
        press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(app.sns_state.topics.selected, 20);
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.sns_state.topics.filter, "TOPIC");
        assert!(!app.sns_state.topics.loading);
    }
}

#[test]
fn sns_preferences_toggle_actual_columns_protect_last_and_change_page_size() {
    let mut app = app();
    press(&mut app, KeyCode::Char('p'), KeyModifiers::NONE);
    assert_eq!(app.mode, Mode::ColumnSelector);
    let prefs = model(&app);
    let arn_index = prefs
        .rows()
        .iter()
        .position(|r| matches!(r,PreferenceRow::Column(c) if c.id==Column::Arn.id()))
        .unwrap();
    app.column_selector_index = arn_index;
    app.handle_action(Action::ToggleColumn);
    assert!(!app.sns_state.visible_column_ids.contains(&Column::Arn.id()));
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert!(!screen(&app, 220, 40).contains("⋮ ARN"));
    press(&mut app, KeyCode::Char('p'), KeyModifiers::NONE);
    for (index, row) in prefs.rows().iter().enumerate() {
        if matches!(row, PreferenceRow::Column(_)) {
            app.column_selector_index = index;
            app.handle_action(Action::ToggleColumn);
        }
    }
    // Set exactly one visible column and exercise the common last-column guard.
    app.sns_state.visible_column_ids = vec![Column::Name.id()];
    let index = model(&app)
        .rows()
        .iter()
        .position(|r| matches!(r,PreferenceRow::Column(c) if c.id==Column::Name.id()))
        .unwrap();
    app.column_selector_index = index;
    app.handle_action(Action::ToggleColumn);
    assert_eq!(app.sns_state.visible_column_ids, [Column::Name.id()]);
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
    app.sns_state.topics.selected = 20;
    app.sns_state.topics.expanded_item = Some(20);
    app.column_selector_index = index;
    app.handle_action(Action::ToggleColumn);
    assert_eq!(app.sns_state.topics.page_size, PageSize::Ten);
    assert_eq!(app.sns_state.topics.selected, 0);
    assert!(app.sns_state.topics.expanded_item.is_none());
    assert!(!app.sns_state.topics.loading);
    app.column_selector_index = 0;
    app.handle_action(Action::NextPreferences);
    assert_eq!(app.column_selector_index, model(&app).move_section(0, true));
}

#[test]
fn sns_refresh_and_error_retry_preserve_rows_and_do_not_refresh_logs() {
    let mut app = app();
    app.handle_action(Action::Refresh);
    assert!(app.sns_state.topics.loading);
    assert!(!app.log_groups_state.loading);
    app.sns_state.topics.loading = false;
    app.mode = Mode::ErrorModal;
    app.error_message = Some("AuthorizationError".into());
    app.handle_action(Action::RetryLoad);
    assert!(app.sns_state.topics.loading);
    assert_eq!(app.mode, Mode::Normal);
    assert!(app.error_message.is_none());
    assert_eq!(app.sns_state.topics.items.len(), 25);
}

#[test]
fn sns_rendering_and_status_hints_cover_all_common_actions() {
    let mut app = app();
    app.sns_state.topics.page_size = PageSize::Ten;
    app.sns_state.topics.selected = 12;
    app.sns_state.topics.expanded_item = Some(11);
    let text = screen(&app, 240, 45);
    assert!(text.contains("Topics (25)"));
    assert!(text.contains("1 [2] 3"));
    assert!(text.contains("► topic-012.fifo"));
    assert!(text.contains("▼ topic-011"));
    assert!(!text.contains("topic-009"));
    assert!(!text.contains("topic-020"));
    for hint in [
        "p preferences",
        "^p print",
        "y copy ARN",
        "^o console",
        "^r refresh",
        "^w close",
    ] {
        assert!(text.contains(hint), "missing {hint}");
    }
    press(&mut app, KeyCode::Char('p'), KeyModifiers::CONTROL);
    assert!(app.clipboard.snapshot_requested());
    app.sns_state.topics.items.clear();
    app.sns_state.topics.loading = true;
    assert!(screen(&app, 240, 45).contains("Loading SNS topics"));
    for (w, h) in [(20, 4), (80, 15)] {
        screen(&app, w, h);
    }
}

#[test]
fn sns_session_restore_and_finder_routes_are_registered() {
    use crate::session::{Session, SessionTab};
    for action in [Action::Select, Action::LoadSession] {
        let mut app = app();
        let mut session = Session::new(
            "test".into(),
            "us-east-1".into(),
            "123456789012".into(),
            "role".into(),
        );
        session.tabs.push(SessionTab {
            service: "SnsTopics".into(),
            title: "SNS › Topics".into(),
            breadcrumb: "SNS › Topics".into(),
            filter: None,
            selected_item: None,
        });
        app.sessions = vec![session];
        app.mode = Mode::SessionPicker;
        app.current_service = Service::CloudWatchLogGroups;
        app.handle_action(action);
        assert_eq!(app.current_service, Service::SnsTopics);
        assert_eq!(app.tabs[0].service, Service::SnsTopics);
    }
    let mut app = app();
    app.sns_state.topics.filter = "topic-024".into();
    app.handle_action(Action::OpenRelatedResources);
    let selected = app.resource_finder.related_source.clone().unwrap();
    assert_eq!(selected.kind, ResourceKind::Topic);
    assert_eq!(selected.id, "topic-024.fifo");
    assert!(selected.arn.ends_with(":topic-024.fifo"));
    assert_eq!(
        crate::finder::navigation::service(selected.kind),
        Service::SnsTopics
    );
    app.handle_action(Action::OpenResourceFinder);
    app.resource_finder.rows = vec![Resource {
        kind: ResourceKind::Topic,
        id: selected.id,
        name: selected.name,
        arn: selected.arn,
        region: "us-east-1".into(),
    }];
    app.handle_action(Action::Select);
    app.handle_action(Action::Select);
    assert_eq!(
        app.resource_finder.pending_open.as_ref().unwrap().kind,
        ResourceKind::Topic
    );
}

#[test]
fn sns_console_link_targets_selected_topic_and_keeps_region() {
    let arn = "arn:aws:sns:us-east-1:123456789012:orders.fifo";
    assert_eq!(console_url("us-east-1",Some(arn)),"https://us-east-1.console.aws.amazon.com/sns/v3/home?region=us-east-1#/topic/arn%3Aaws%3Asns%3Aus-east-1%3A123456789012%3Aorders.fifo");
    assert!(console_url("eu-west-1", None).ends_with("region=eu-west-1#/topics"));
    assert!(console_url("cn-north-1", None).starts_with("https://cn-north-1.console.amazonaws.cn/"));
}
