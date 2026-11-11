//! App-level tests for EventBridge. Completions go through the real poll
//! path; no test makes an AWS request.
use super::*;
use crate::keymap::handle_key;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};
use rusticity_core::eventbridge::RuleMetric;

const BUS_ARN: &str = "arn:aws:events:us-east-2:123456789012:event-bus/orders";
const RULE_ARN: &str = "arn:aws:events:us-east-2:123456789012:rule/orders/nightly";

fn app() -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-2".into()));
    app.config.account_id = "123456789012".into();
    app.current_service = Service::EventBridgeEventBuses;
    app.service_selected = true;
    app.mode = Mode::Normal;
    request(&mut app);
    app
}

fn press(app: &mut App, key: KeyCode) {
    let action = handle_key(KeyEvent::new(key, KeyModifiers::NONE), app.mode).unwrap();
    app.handle_action(action);
}

fn ctrl(app: &mut App, c: char) {
    let action = handle_key(
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL),
        app.mode,
    )
    .unwrap();
    app.handle_action(action);
}

fn screen(app: &App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(200, 45)).unwrap();
    terminal.draw(|f| crate::ui::render(f, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

fn buses() -> Vec<EventBus> {
    vec![
        EventBus {
            name: "default".into(),
            arn: "arn:aws:events:us-east-2:123456789012:event-bus/default".into(),
            ..Default::default()
        },
        EventBus {
            name: "orders".into(),
            arn: BUS_ARN.into(),
            description: "Order events".into(),
            ..Default::default()
        },
    ]
}

fn rules() -> Vec<EventRule> {
    vec![
        EventRule {
            name: "hourly".into(),
            arn: "arn:aws:events:us-east-2:123456789012:rule/orders/hourly".into(),
            state: "DISABLED".into(),
            schedule_expression: "rate(1 hour)".into(),
            event_bus_name: "orders".into(),
            ..Default::default()
        },
        EventRule {
            name: "nightly".into(),
            arn: RULE_ARN.into(),
            state: "ENABLED".into(),
            event_pattern: r#"{"source":["orders"]}"#.into(),
            event_bus_name: "orders".into(),
            ..Default::default()
        },
    ]
}

fn detail() -> RuleDetail {
    RuleDetail {
        rule: EventRule {
            description: "Ship orders".into(),
            role_arn: "arn:aws:iam::123456789012:role/events".into(),
            ..rules()[1].clone()
        },
        created_by: "123456789012".into(),
        targets: Ok(vec![EventTarget {
            id: "ship".into(),
            arn: "arn:aws:lambda:us-east-2:123456789012:function:ship".into(),
            input: "$.detail".into(),
            ..Default::default()
        }]),
        tags: Ok(vec![("team".into(), "orders".into())]),
        metrics: Ok(RULE_METRICS
            .iter()
            .map(|name| RuleMetric {
                name,
                points: vec![(0, 1.0), (300, 2.0)],
            })
            .collect()),
    }
}

async fn finish_buses(app: &mut App, result: Result<Vec<EventBus>, String>) {
    assert!(app.eventbridge_state.read.start(async move { result }));
    tokio::task::yield_now().await;
    assert!(poll(app).await);
}

async fn finish_rules(app: &mut App, result: Result<Vec<EventRule>, String>) {
    let bus = app.eventbridge_state.bus.as_mut().unwrap();
    assert!(bus.read.start(async move { result }));
    tokio::task::yield_now().await;
    assert!(poll(app).await);
}

async fn finish_rule(app: &mut App, result: Result<RuleDetail, String>) {
    let rule = app
        .eventbridge_state
        .bus
        .as_mut()
        .unwrap()
        .rule
        .as_mut()
        .unwrap();
    assert!(rule.read.start(async move { result.map(Box::new) }));
    tokio::task::yield_now().await;
    assert!(poll(app).await);
}

/// Buses loaded, `orders` entered, rules loaded, `nightly` entered and read.
async fn on_rule() -> App {
    let mut app = app();
    finish_buses(&mut app, Ok(buses())).await;
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    finish_rules(&mut app, Ok(rules())).await;
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    finish_rule(&mut app, Ok(detail())).await;
    app
}

fn tab(app: &App) -> Tab {
    app.eventbridge_state.rule().unwrap().tab
}

fn goto(app: &mut App, target: Tab) {
    while tab(app) != target {
        press(app, KeyCode::Tab);
    }
}

#[test]
fn the_picker_lists_the_service() {
    let app = App::new_without_client("beta".into(), Some("us-east-2".into()));
    assert!(app
        .service_picker
        .services
        .contains(&"EventBridge › Event buses"));
    assert_eq!(
        Service::EventBridgeEventBuses.name(),
        "EventBridge › Event buses"
    );
}

#[tokio::test]
async fn buses_load_and_never_claim_empty_before_success() {
    let mut app = app();
    assert!(app.eventbridge_state.read.is_busy(), "read queued");
    let text = screen(&app);
    assert!(text.contains("Loading…") && !text.contains("No event buses"));
    finish_buses(&mut app, Ok(buses())).await;
    let text = screen(&app);
    assert!(text.contains("Event buses (2)") && text.contains("Order events"));
    assert_eq!(app.breadcrumbs(), "EventBridge > Event buses");
}

#[tokio::test]
async fn enter_walks_bus_then_rule_and_esc_walks_back() {
    let mut app = app();
    finish_buses(&mut app, Ok(buses())).await;
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.eventbridge_state.bus.as_ref().unwrap().bus.name,
        "orders"
    );
    assert!(app.eventbridge_state.bus.as_ref().unwrap().read.is_busy());
    assert_eq!(app.breadcrumbs(), "EventBridge > Event buses > orders");
    finish_rules(&mut app, Ok(rules())).await;
    let text = screen(&app);
    assert!(text.contains("Rules (2)") && text.contains("Scheduled") && text.contains("Standard"));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.breadcrumbs(),
        "EventBridge > Event buses > orders > nightly"
    );
    press(&mut app, KeyCode::Esc);
    assert!(app.eventbridge_state.rule().is_none());
    assert_eq!(
        app.eventbridge_state
            .bus
            .as_ref()
            .unwrap()
            .rules
            .rows
            .items
            .len(),
        2,
        "the rules level is untouched"
    );
    press(&mut app, KeyCode::Esc);
    assert!(app.eventbridge_state.bus.is_none());
    assert_eq!(app.breadcrumbs(), "EventBridge > Event buses");
}

#[tokio::test]
async fn rule_status_is_colored_like_everywhere() {
    let mut app = app();
    finish_buses(&mut app, Ok(buses())).await;
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    finish_rules(&mut app, Ok(rules())).await;
    let mut terminal = Terminal::new(TestBackend::new(200, 45)).unwrap();
    terminal.draw(|f| crate::ui::render(f, &app)).unwrap();
    let buffer = terminal.backend().buffer();
    let joined: String = buffer.content().iter().map(|c| c.symbol()).collect();
    let color = |word: &str| {
        let byte = joined.find(word).unwrap();
        buffer.content()[joined[..byte].chars().count()].fg
    };
    use ratatui::style::Color;
    assert_eq!(color("Enabled"), Color::Green);
    assert_eq!(color("Disabled"), Color::Yellow);
}

#[tokio::test]
async fn rule_summary_waits_for_the_fresh_read() {
    let mut app = app();
    finish_buses(&mut app, Ok(buses())).await;
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    finish_rules(&mut app, Ok(rules())).await;
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    let text = screen(&app);
    assert!(text.contains("Rule details") && text.contains("nightly"));
    assert!(text.contains("Loading…") && !text.contains("Ship orders"));
    finish_rule(&mut app, Ok(detail())).await;
    let text = screen(&app);
    assert!(text.contains("Ship orders") && text.contains("role/events"));
    assert!(text.contains("Event pattern ⋮ Targets ⋮ Monitoring ⋮ Tags"));
    assert!(text.contains(r#""source": ["#), "pretty JSON");
}

#[tokio::test]
async fn a_scheduled_rule_shows_its_schedule() {
    let mut app = app();
    finish_buses(&mut app, Ok(buses())).await;
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    finish_rules(&mut app, Ok(rules())).await;
    press(&mut app, KeyCode::Enter);
    let mut d = detail();
    d.rule = rules()[0].clone();
    finish_rule(&mut app, Ok(d)).await;
    let text = screen(&app);
    assert!(text.contains("Event schedule ⋮ Targets"));
    assert!(text.contains("Schedule expression: rate(1 hour)"));
}

#[tokio::test]
async fn targets_tags_and_monitoring_render_and_fail_alone() {
    let mut app = on_rule().await;
    goto(&mut app, Tab::Targets);
    let text = screen(&app);
    assert!(text.contains("Targets (1)") && text.contains("Lambda function"));
    assert!(text.contains("$.detail"));
    goto(&mut app, Tab::Monitoring);
    assert!(screen(&app).contains("Invocations (Count)"));
    goto(&mut app, Tab::Tags);
    let text = screen(&app);
    assert!(text.contains("Tags (1)") && text.contains("team"));

    let mut app = self::app();
    finish_buses(&mut app, Ok(buses())).await;
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    finish_rules(&mut app, Ok(rules())).await;
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    let mut d = detail();
    d.targets = Err("Targets unavailable: AccessDenied".into());
    finish_rule(&mut app, Ok(d)).await;
    goto(&mut app, Tab::Targets);
    let text = screen(&app);
    assert!(text.contains("Targets unavailable") && !text.contains("No targets"));
    goto(&mut app, Tab::Tags);
    assert!(screen(&app).contains("team"), "other tabs stay");
}

#[tokio::test]
async fn refresh_and_failure_never_show_old_rows() {
    let mut app = on_rule().await;
    goto(&mut app, Tab::Targets);
    ctrl(&mut app, 'r');
    let rule = app.eventbridge_state.rule().unwrap();
    assert!(rule.detail().is_none() && rule.targets.rows.items.is_empty());
    assert_eq!(rule.tab, Tab::Targets, "tab survives");
    assert!(!screen(&app).contains("Lambda function"));
    finish_rule(&mut app, Err("AccessDenied: events:DescribeRule".into())).await;
    let text = screen(&app);
    assert!(text.contains("AccessDenied") && !text.contains("Lambda function"));
    press(&mut app, KeyCode::Esc);
    ctrl(&mut app, 'r');
    let bus = app.eventbridge_state.bus.as_ref().unwrap();
    assert!(bus.rules.rows.items.is_empty() && bus.read.is_busy());
    assert!(!screen(&app).contains("hourly"));
}

#[tokio::test]
async fn esc_cancels_a_running_read_before_going_back() {
    let mut app = app();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    assert!(app.eventbridge_state.read.start(async move {
        let _keep = tx;
        std::future::pending::<Result<Vec<EventBus>, String>>().await
    }));
    press(&mut app, KeyCode::Esc);
    assert!(screen(&app).contains("Cancelled"));
    tokio::task::yield_now().await;
    assert!(rx.await.is_err(), "the read task was aborted");
}

#[tokio::test]
async fn aws_context_change_drops_every_level() {
    let mut app = on_rule().await;
    app.config.region = "eu-west-1".into();
    assert!(poll(&mut app).await);
    assert!(app.eventbridge_state.bus.is_none());
    assert!(app.eventbridge_state.buses.rows.items.is_empty());
}

#[tokio::test]
async fn filter_narrows_only_the_rules_table() {
    let mut app = app();
    finish_buses(&mut app, Ok(buses())).await;
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    finish_rules(&mut app, Ok(rules())).await;
    press(&mut app, KeyCode::Char('i'));
    for c in "night".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    press(&mut app, KeyCode::Enter);
    let text = screen(&app);
    assert!(text.contains("Rules (1)") && !text.contains("hourly"));
    assert!(app.eventbridge_state.buses.rows.filter.is_empty());
}

#[tokio::test]
async fn console_and_cli_follow_the_level() {
    let mut app = app();
    let base = "https://us-east-2.console.aws.amazon.com/events/home?region=us-east-2";
    assert_eq!(app.get_console_url(), format!("{base}#/eventbuses"));
    let cli = |app: &App| crate::cli_command::invocation(app).unwrap().display;
    assert!(cli(&app).starts_with("aws events list-event-buses"));
    finish_buses(&mut app, Ok(buses())).await;
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.get_console_url(),
        format!("{base}#/rules?eventbus=orders")
    );
    let command = cli(&app);
    assert!(command.starts_with("aws events list-rules") && command.contains("orders"));
    finish_rules(&mut app, Ok(rules())).await;
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    finish_rule(&mut app, Ok(detail())).await;
    assert_eq!(
        app.get_console_url(),
        format!("{base}#/eventbus/orders/rules/nightly")
    );
    assert!(cli(&app).starts_with("aws events describe-rule"));
    goto(&mut app, Tab::Targets);
    assert!(cli(&app).starts_with("aws events list-targets-by-rule"));
    goto(&mut app, Tab::Tags);
    let command = cli(&app);
    assert!(command.starts_with("aws events list-tags-for-resource") && command.contains(RULE_ARN));
}

#[tokio::test]
async fn yank_copies_the_arn_of_the_level() {
    let mut app = on_rule().await;
    let buffer = ratatui::buffer::Buffer::empty(ratatui::layout::Rect::new(0, 0, 4, 1));
    press(&mut app, KeyCode::Char('y'));
    let mut copied = None;
    assert!(app.clipboard.complete_with(&buffer, |text| {
        copied = Some(text);
        Ok(())
    }));
    assert_eq!(copied.as_deref(), Some(RULE_ARN));
}

#[tokio::test]
async fn preferences_follow_the_table_on_screen() {
    use crate::preferences::bindings::model_for;
    let mut app = on_rule().await;
    let count = |app: &App| model_for(app, Service::EventBridgeEventBuses).column_count();
    assert_eq!(count(&app), 0, "Event pattern has no columns");
    goto(&mut app, Tab::Targets);
    assert_eq!(count(&app), TARGET_COLUMNS.len());
    goto(&mut app, Tab::Tags);
    assert_eq!(count(&app), 2);
    press(&mut app, KeyCode::Esc);
    assert_eq!(count(&app), RULE_COLUMNS.len());
    press(&mut app, KeyCode::Esc);
    assert_eq!(count(&app), BUS_COLUMNS.len());
}

#[test]
fn target_types_follow_the_arn_service() {
    for (arn, kind) in [
        ("arn:aws:lambda:us-east-2:1:function:f", "Lambda function"),
        ("arn:aws:sqs:us-east-2:1:q", "SQS queue"),
        ("arn:aws:events:us-east-2:1:event-bus/b", "Event bus"),
        (
            "arn:aws:events:us-east-2:1:api-destination/d/x",
            "API destination",
        ),
        (
            "arn:aws:states:us-east-2:1:stateMachine:m",
            "Step Functions state machine",
        ),
        ("arn:aws:newsvc:us-east-2:1:thing", "newsvc"),
    ] {
        assert_eq!(view::target_type(arn), kind, "{arn}");
    }
    assert_eq!(view::discovery("STARTED"), "Started");
    assert_eq!(view::discovery(""), "");
    assert_eq!(view::state("ENABLED"), "Enabled");
    assert_eq!(view::state("DISABLED"), "Disabled");
}

#[test]
fn invalid_patterns_are_shown_as_written() {
    assert_eq!(view::pattern_lines("not json"), ["not json"]);
    assert_eq!(view::pattern_lines(r#"{"a":1}"#), ["{", r#"  "a": 1"#, "}"]);
}

#[test]
fn bus_columns_follow_the_console() {
    let names: Vec<_> = BUS_COLUMNS.iter().map(|(name, ..)| *name).collect();
    assert_eq!(
        names,
        [
            "Name",
            "Type",
            "Status",
            "ID",
            "Amazon Resource Name (ARN)",
            "Schema discovery",
            "Description",
            "Created on",
            "Last modified",
            "Owner account",
        ]
    );
    let bus = EventBus {
        name: "orders".into(),
        arn: BUS_ARN.into(),
        schema_discovery: "STARTED".into(),
        ..Default::default()
    };
    let cell = |name: &str| (BUS_COLUMNS.iter().find(|c| c.0 == name).unwrap().2)(&bus);
    assert_eq!(cell("Type"), "Custom Event Bus - Classic");
    assert_eq!(cell("Schema discovery"), "Started");
    assert_eq!(cell("Owner account"), "123456789012");
    assert!(cell("Status").is_empty() && cell("ID").is_empty());
}
