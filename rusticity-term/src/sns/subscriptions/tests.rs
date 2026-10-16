use super::*;
use crate::{
    common::PageSize,
    keymap::handle_key,
    preferences::{bindings, PreferenceRow},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};
const TOPIC: &str = "arn:aws:sns:us-east-1:123456789012:chosen";
fn app() -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    app.current_service = Service::SnsTopics;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.config.account_id = "123456789012".into();
    app.sns_state.topics.items = vec![
        SnsTopic::from_arn("arn:aws:sns:us-east-1:123456789012:other").unwrap(),
        SnsTopic::from_arn(TOPIC).unwrap(),
    ];
    app.sns_state.topics.filter = "CHOSEN".into();
    app
}
fn press(app: &mut App, key: KeyCode, mods: KeyModifiers) {
    if let Some(action) = handle_key(KeyEvent::new(key, mods), app.mode) {
        app.handle_action(action);
    }
}
fn open(app: &mut App) {
    press(app, KeyCode::Enter, KeyModifiers::NONE);
}
fn row(i: usize) -> SnsSubscription {
    SnsSubscription {
        subscription_arn: format!("{TOPIC}:{i:08}-1111-1111-1111-111111111111"),
        topic_arn: TOPIC.into(),
        protocol: "email".into(),
        endpoint: format!("user{i:03}@example.invalid"),
        owner: "123456789012".into(),
        status: "Confirmed".into(),
    }
}
async fn finish(app: &mut App, result: Result<Vec<SnsSubscription>, String>) {
    app.sns_state.subscriptions.running = Some(Running(tokio::spawn(async { result })));
    tokio::task::yield_now().await;
    assert!(poll(app).await);
}
fn loaded() -> App {
    let mut app = app();
    open(&mut app);
    let state = &mut app.sns_state.subscriptions;
    state.table.loading = false;
    state.loaded = true;
    state.table.items = (0..25).rev().map(row).collect();
    app
}
fn screen(app: &App, width: u16, height: u16) -> String {
    let mut t = Terminal::new(TestBackend::new(width, height)).unwrap();
    t.draw(|f| ui::render(f, app)).unwrap();
    t.backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}
fn prefs(app: &App) -> crate::preferences::Preferences {
    bindings::model_for(app, Service::SnsTopics)
}

#[test]
fn enter_uses_filtered_topic_and_escape_keeps_topics_filter_selection_and_preferences() {
    let mut app = app();
    app.sns_state.topics.expanded_item = Some(0);
    assert!(screen(&app, 240, 40).contains("subscriptions"));
    open(&mut app);
    assert_eq!(
        app.sns_state.subscriptions.topic.as_ref().unwrap().arn,
        TOPIC
    );
    assert!(app.sns_state.subscriptions.table.loading);
    assert!(!app.sns_state.topics.loading);
    assert!(app.breadcrumbs().contains("chosen"));
    assert!(app.breadcrumbs().contains("Subscriptions"));
    let text = screen(&app, 240, 40);
    assert!(text.contains("Details"));
    assert!(text.contains("Loading subscriptions"));
    assert!(!text.contains("No subscriptions for this topic"));
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert!(app.sns_state.subscriptions.topic.is_none());
    assert!(!app.sns_state.subscriptions.busy());
    assert_eq!(app.sns_state.topics.filter, "CHOSEN");
    assert_eq!(app.sns_state.topics.expanded_item, Some(0));
}
#[test]
fn filter_pagination_expansion_and_preferences_use_shared_active_table() {
    let mut app = loaded();
    app.sns_state.subscriptions.table.page_size = PageSize::Ten;
    press(&mut app, KeyCode::Down, KeyModifiers::NONE);
    open(&mut app);
    assert_eq!(app.sns_state.subscriptions.table.expanded_item, Some(1));
    press(&mut app, KeyCode::Char('i'), KeyModifiers::NONE);
    for c in "EMAIL".chars() {
        press(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
    }
    assert_eq!(filtered(&app).len(), 25);
    assert!(app.sns_state.subscriptions.table.expanded_item.is_none());
    press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(
        app.sns_state.subscriptions.input_focus,
        InputFocus::Pagination
    );
    press(&mut app, KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(app.sns_state.subscriptions.table.selected, 10);
    press(&mut app, KeyCode::Char('3'), KeyModifiers::NONE);
    open(&mut app);
    assert_eq!(app.sns_state.subscriptions.table.selected, 20);
    assert_eq!(app.sns_state.topics.selected, 0);
    assert_eq!(app.sns_state.topics.filter, "CHOSEN");
    press(&mut app, KeyCode::Char('p'), KeyModifiers::NONE);
    assert_eq!(app.mode, Mode::ColumnSelector);
    assert_eq!(
        prefs(&app)
            .columns()
            .map(|c| c.label.as_str())
            .collect::<Vec<_>>(),
        [
            "Protocol",
            "Endpoint",
            "Subscription ARN",
            "Status",
            "Owner"
        ]
    );
    let index = prefs(&app)
        .rows()
        .iter()
        .position(|r| matches!(r,PreferenceRow::Column(c) if c.id==Column::Endpoint.id()))
        .unwrap();
    app.column_selector_index = index;
    app.handle_action(Action::ToggleColumn);
    assert!(!app
        .sns_state
        .subscriptions
        .visible_column_ids
        .contains(&Column::Endpoint.id()));
    app.sns_state.subscriptions.visible_column_ids = vec![Column::Protocol.id()];
    let index = prefs(&app)
        .rows()
        .iter()
        .position(|r| matches!(r,PreferenceRow::Column(c) if c.id==Column::Protocol.id()))
        .unwrap();
    app.column_selector_index = index;
    app.handle_action(Action::ToggleColumn);
    assert_eq!(
        app.sns_state.subscriptions.visible_column_ids,
        [Column::Protocol.id()]
    );
    let index = prefs(&app)
        .rows()
        .iter()
        .position(|r| {
            matches!(
                r,
                PreferenceRow::PageSize {
                    value: PageSize::TwentyFive,
                    ..
                }
            )
        })
        .unwrap();
    app.column_selector_index = index;
    app.handle_action(Action::ToggleColumn);
    assert_eq!(
        app.sns_state.subscriptions.table.page_size,
        PageSize::TwentyFive
    );
    assert_eq!(app.sns_state.subscriptions.table.selected, 0);
    assert_eq!(app.sns_state.topics.page_size, PageSize::Fifty);
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL);
    assert_eq!(
        app.sns_state.subscriptions.visible_column_ids,
        [Column::Protocol.id()]
    );
    assert!(!app.sns_state.topics.loading);
}
#[test]
fn selected_subscription_copy_and_console_follow_filtered_row_and_pending_never_copies_fake_arn() {
    let mut app = loaded();
    app.sns_state.subscriptions.table.filter = "user024".into();
    let arn = row(24).subscription_arn;
    let buffer = Buffer::empty(Rect::new(0, 0, 1, 1));
    press(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
    assert!(app.clipboard.complete_with(&buffer, |s| {
        assert_eq!(s, arn);
        Ok(())
    }));
    assert!(app
        .get_console_url()
        .ends_with(&format!("#/subscription/{}", urlencoding::encode(&arn))));
    app.sns_state.subscriptions.table.items = vec![SnsSubscription {
        subscription_arn: "PendingConfirmation".into(),
        status: "Pending confirmation".into(),
        ..row(24)
    }];
    press(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
    assert!(app.clipboard.notice().unwrap().failed);
    assert!(!app
        .clipboard
        .complete_with(&buffer, |_| panic!("pending ARN must not be copied")));
    assert!(app
        .get_console_url()
        .ends_with(&format!("#/topic/{}", urlencoding::encode(TOPIC))));
}
#[test]
fn cli_copy_and_output_use_topic_scoped_subscription_read_even_for_pending_rows() {
    let mut app = loaded();
    app.sns_state.subscriptions.table.items = vec![SnsSubscription {
        subscription_arn: "PendingConfirmation".into(),
        status: "Pending confirmation".into(),
        ..row(0)
    }];
    let invocation = crate::cli_command::invocation(&app).unwrap();
    assert_eq!(
        &invocation.args[..2],
        ["sns", "list-subscriptions-by-topic"]
    );
    assert!(invocation.display.contains(TOPIC));
    assert!(!invocation.display.contains("PendingConfirmation"));
    press(&mut app, KeyCode::Char('y'), KeyModifiers::CONTROL);
    app.clipboard
        .complete_with(&Buffer::empty(Rect::new(0, 0, 1, 1)), |text| {
            assert_eq!(text, invocation.display);
            Ok(())
        });
    press(&mut app, KeyCode::Char('p'), KeyModifiers::CONTROL);
    assert!(app.clipboard.snapshot_requested());
}
#[tokio::test]
async fn empty_success_and_denied_reads_are_distinct_and_refresh_retains_previous_rows() {
    let mut app = app();
    open(&mut app);
    finish(&mut app, Err("AuthorizationError: denied".into())).await;
    let text = screen(&app, 240, 40);
    assert!(text.contains("Subscriptions unavailable"));
    assert!(!text.contains("No subscriptions for this topic"));
    press(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL);
    finish(&mut app, Ok(vec![])).await;
    assert!(screen(&app, 240, 40).contains("No subscriptions for this topic"));
    finish(&mut app, Ok(vec![row(0)])).await;
    press(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL);
    finish(&mut app, Err("ExpiredToken: expired".into())).await;
    assert_eq!(app.sns_state.subscriptions.table.items.len(), 1);
    let text = screen(&app, 240, 40);
    assert!(text.contains("previously loaded"));
    assert!(text.contains("user000"));
}
#[tokio::test]
async fn escape_context_switch_and_service_change_abort_pending_task() {
    for case in 0..6 {
        let mut app = app();
        open(&mut app);
        let task = tokio::spawn(std::future::pending::<Result<Vec<SnsSubscription>, String>>());
        let abort = task.abort_handle();
        app.sns_state.subscriptions.running = Some(Running(task));
        match case {
            0 => app.profile = "other".into(),
            1 => app.config.region = "eu-west-1".into(),
            2 => app.config.account_id = "other".into(),
            3 => app.config.role_arn = "other".into(),
            4 => app.current_service = Service::S3Buckets,
            _ => press(&mut app, KeyCode::Esc, KeyModifiers::NONE),
        }
        if case < 5 {
            assert!(poll(&mut app).await);
        }
        tokio::task::yield_now().await;
        assert!(abort.is_finished());
        assert!(!app.sns_state.subscriptions.busy());
        assert!(app.sns_state.subscriptions.topic.is_none());
    }
}
#[test]
fn columns_and_status_bar_are_complete_without_live_aws_data() {
    let mut app = loaded();
    let mut pending = row(1);
    pending.status = "Pending confirmation".into();
    pending.subscription_arn = "PendingConfirmation".into();
    app.sns_state.subscriptions.table.items = vec![row(0), pending];
    let text = screen(&app, 300, 40);
    for label in [
        "Protocol",
        "Endpoint",
        "Subscription ARN",
        "Status",
        "Pending confirmation",
        "Confirmed",
        "p preferences",
        "^p print",
        "^o console",
        "^r refresh",
        "^y copy CLI",
        "^e CLI output",
    ] {
        assert!(text.contains(label), "missing {label}");
    }
    for (w, h) in [(30, 8), (80, 18)] {
        screen(&app, w, h);
    }
    for col in Column::ALL {
        assert!(col.id().starts_with("column.sns.subscription."));
        assert_eq!(Column::from_id(col.id()), Some(col));
    }
}

#[test]
fn returning_from_subscriptions_restores_topics_list_console_link() {
    let mut app = loaded();
    assert!(app.get_console_url().contains("#/subscription/"));
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    assert!(app.sns_state.subscriptions.topic.is_none());
    // The topic remains selected, but the table must not inherit its deep link.
    assert_eq!(
        crate::sns::actions::selected_arn(&app).as_deref(),
        Some(TOPIC)
    );
    assert_eq!(
        app.get_console_url(),
        "https://us-east-1.console.aws.amazon.com/sns/v3/home?region=us-east-1#/topics"
    );
}
