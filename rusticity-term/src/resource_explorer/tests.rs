//! App-level tests for Resource Explorer › Resources. Searches finish through
//! the real poll path with injected results; no test makes an AWS request.
use super::*;
use crate::keymap::handle_key;
use crate::preferences::{bindings, PreferenceRow};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};

fn resource(id: &str, tags: Option<&[(&str, &str)]>) -> ExplorerResource {
    ExplorerResource {
        arn: format!("arn:aws:ec2:us-east-2:123456789012:instance/{id}"),
        identifier: id.into(),
        service: "ec2".into(),
        resource_type: "ec2:instance".into(),
        region: "us-east-2".into(),
        owner_account: "123456789012".into(),
        application: String::new(),
        tags: tags.map(|tags| {
            tags.iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        }),
        last_reported_at: "2026-11-20T10:00:00Z".into(),
    }
}

fn found() -> ResourceSearch {
    ResourceSearch {
        resources: vec![
            ExplorerResource {
                application: "Billing".into(),
                ..resource("i-0b", Some(&[("Name", "web"), ("Stage", "prod")]))
            },
            resource("i-0a", Some(&[("Stage", "")])),
        ],
        complete: true,
        ..Default::default()
    }
}

fn app() -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-2".into()));
    app.aws.config.account_id = "123456789012".into();
    app.current_service = Service::ResourceExplorerResources;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app
}

fn press(app: &mut App, key: KeyCode) {
    let action = handle_key(KeyEvent::new(key, KeyModifiers::NONE), app.mode).unwrap();
    app.handle_action(action);
}

fn ctrl(app: &mut App, c: char) {
    let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL);
    let action = handle_key(key, app.mode).unwrap();
    app.handle_action(action);
}

fn typed(app: &mut App, text: &str) {
    for c in text.chars() {
        press(app, KeyCode::Char(c));
    }
}

fn screen(app: &App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(200, 40)).unwrap();
    terminal.draw(|f| crate::ui::render(f, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

async fn finish(app: &mut App, result: Result<ResourceSearch, String>) {
    let scope = scope(app);
    let read = app.resource_explorer_state.read.as_mut().unwrap();
    read.drive(&scope, "Resource search failed", || async move {
        result.map(Box::new)
    })
    .await;
    tokio::task::yield_now().await;
    assert!(poll(app).await, "the result arrived");
}

async fn searched() -> App {
    let mut app = app();
    request(&mut app);
    finish(&mut app, Ok(found())).await;
    app
}

fn identifiers(app: &App) -> Vec<String> {
    app.resource_explorer_state
        .list
        .filtered()
        .iter()
        .map(|r| r.identifier.clone())
        .collect()
}

fn busy(app: &App) -> bool {
    matches!(
        app.resource_explorer_state.load_status(),
        ReadStatus::Loading { .. }
    )
}

#[test]
fn the_service_is_resource_explorer_resources_and_opening_it_searches() {
    assert_eq!(
        Service::ResourceExplorerResources.name(),
        "Resource Explorer › Resources"
    );
    let mut app = App::new_without_client("beta".into(), None);
    assert!(app
        .service_picker
        .services
        .contains(&"Resource Explorer › Resources"));
    app.mode = Mode::ServicePicker;
    app.service_picker.filter = "resource explorer".into();
    app.handle_action(Action::Select);
    assert_eq!(app.current_service, Service::ResourceExplorerResources);
    assert!(busy(&app), "the new tab searched at once");
    assert_eq!(
        app.resource_explorer_state.query, "",
        "everything in the view"
    );
    let crumbs = app.breadcrumbs();
    assert!(
        crumbs.contains("Resource Explorer") && crumbs.contains("Resources"),
        "{crumbs}"
    );
}

#[tokio::test]
async fn results_show_the_console_columns_ordered_by_identifier() {
    let app = searched().await;
    let text = screen(&app);
    assert!(text.contains("Resources (2)"), "{text}");
    let header = [
        "Identifier ↑",
        "Service",
        "Resource type",
        "Region",
        "Owner account",
        "Application",
        "Tags",
    ];
    let mut at = 0;
    for name in header {
        let found = text[at..].find(name).unwrap_or_else(|| panic!("{name}"));
        at += found;
    }
    for cell in [
        "i-0a",
        "ec2:instance",
        "us-east-2",
        "123456789012",
        "Billing",
    ] {
        assert!(text.contains(cell), "{cell}");
    }
    assert_eq!(identifiers(&app), ["i-0a", "i-0b"]);
    assert!(
        text.contains("Query template"),
        "the dropdown is in the filter line"
    );
}

#[tokio::test]
async fn typing_edits_the_query_and_enter_searches_it() {
    let mut app = searched().await;
    press(&mut app, KeyCode::Char('i'));
    typed(&mut app, "service:s3");
    assert_eq!(
        identifiers(&app),
        ["i-0a", "i-0b"],
        "the query is not a local filter"
    );
    assert!(!busy(&app), "no search while typing");
    press(&mut app, KeyCode::Enter);
    let state = &app.resource_explorer_state;
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(state.query, "service:s3");
    assert!(state.list.rows.items.is_empty(), "old results go at once");
    assert!(busy(&app));
    assert!(!screen(&app).contains("i-0b"));
}

#[tokio::test]
async fn esc_puts_back_the_query_on_screen() {
    let mut app = searched().await;
    press(&mut app, KeyCode::Char('i'));
    typed(&mut app, "tag:none");
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.resource_explorer_state.list.rows.filter, "");
    assert!(!busy(&app));
    assert_eq!(identifiers(&app), ["i-0a", "i-0b"]);
}

#[tokio::test]
async fn a_template_without_a_value_searches_at_once() {
    let mut app = searched().await;
    press(&mut app, KeyCode::Char('i'));
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.resource_explorer_state.list.dropdown_choice(), Some(0));
    let text = screen(&app);
    for name in TEMPLATES {
        assert!(text.contains(name), "the open list shows {name}");
    }
    press(&mut app, KeyCode::Down);
    assert_eq!(app.resource_explorer_state.list.dropdown_choice(), Some(1));
    typed(&mut app, "x");
    press(&mut app, KeyCode::Enter);
    let state = &app.resource_explorer_state;
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(state.query, "tag:none");
    assert_eq!(state.list.rows.filter, "tag:none");
    assert!(busy(&app));
}

#[tokio::test]
async fn a_template_that_takes_a_value_waits_for_it() {
    let mut app = searched().await;
    press(&mut app, KeyCode::Char('i'));
    press(&mut app, KeyCode::Tab);
    for _ in 0..3 {
        press(&mut app, KeyCode::Down);
    }
    press(&mut app, KeyCode::Enter);
    let state = &app.resource_explorer_state;
    assert_eq!(app.mode, Mode::FilterInput);
    assert_eq!(state.list.focus, InputFocus::Filter);
    assert_eq!(state.list.rows.filter, "tag:none resourcetype:");
    assert!(!busy(&app), "nothing is searched without the type");
    typed(&mut app, "ec2:volume");
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.resource_explorer_state.query,
        "tag:none resourcetype:ec2:volume"
    );
    assert!(busy(&app));
}

#[test]
fn every_template_has_a_query() {
    assert_eq!(TEMPLATES.len(), TEMPLATE_QUERIES.len());
}

#[tokio::test]
async fn a_partial_result_says_there_are_more() {
    let mut app = app();
    request(&mut app);
    let mut partial = found();
    partial.complete = false;
    finish(&mut app, Ok(partial)).await;
    assert!(screen(&app).contains("Resources (2+)"));
}

#[tokio::test]
async fn enter_on_a_result_shows_its_arn_and_every_tag() {
    let mut app = searched().await;
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    let text = screen(&app);
    for line in [
        "ARN: arn:aws:ec2:us-east-2:123456789012:instance/i-0b",
        "Last reported:",
        "Tag: Name: web",
        "Tag: Stage: prod",
    ] {
        assert!(text.contains(line), "{line}");
    }
    press(&mut app, KeyCode::Up);
    press(&mut app, KeyCode::Enter);
    assert!(screen(&app).contains("Tag: Stage: (empty value)"));
}

#[tokio::test]
async fn a_view_without_tags_says_so() {
    let mut app = app();
    request(&mut app);
    let found = ResourceSearch {
        resources: vec![resource("i-0c", None)],
        complete: true,
        ..Default::default()
    };
    finish(&mut app, Ok(found)).await;
    press(&mut app, KeyCode::Enter);
    assert!(screen(&app).contains("Tags: the view does not include tags"));
}

#[tokio::test]
async fn refresh_repeats_the_query_on_screen_and_a_failure_shows_no_rows() {
    let mut app = searched().await;
    app.resource_explorer_state.query = "service:ec2".into();
    app.resource_explorer_state.list.rows.filter = "service:ec2".into();
    ctrl(&mut app, 'r');
    assert_eq!(app.resource_explorer_state.query, "service:ec2");
    assert!(app.resource_explorer_state.list.rows.items.is_empty());
    assert!(busy(&app));
    finish(
        &mut app,
        Err("UnauthorizedException: no default view in this Region".into()),
    )
    .await;
    assert_eq!(
        app.resource_explorer_state.load_status(),
        ReadStatus::Failed
    );
    let text = screen(&app);
    assert!(text.contains("no default view"), "{text}");
    assert!(!text.contains("i-0b"));
}

#[test]
fn esc_cancels_a_running_search() {
    let mut app = app();
    request(&mut app);
    press(&mut app, KeyCode::Esc);
    let state = &app.resource_explorer_state;
    assert_eq!(state.load_status(), ReadStatus::Failed);
    assert!(state
        .read
        .as_ref()
        .unwrap()
        .error()
        .unwrap()
        .contains("Cancelled"));
    assert_eq!(app.current_service, Service::ResourceExplorerResources);
}

#[tokio::test]
async fn a_new_aws_context_searches_again_and_drops_the_old_rows() {
    let mut app = searched().await;
    app.aws.config.region = "eu-west-1".into();
    assert!(poll(&mut app).await);
    assert!(app.resource_explorer_state.list.rows.items.is_empty());
    assert!(busy(&app));
}

#[tokio::test]
async fn sort_by_marks_identifier_and_offers_no_default_order() {
    let mut app = searched().await;
    let rows: Vec<(&'static str, bool)> =
        bindings::model_for(&app, Service::ResourceExplorerResources)
            .rows()
            .iter()
            .filter_map(|row| match row {
                PreferenceRow::Sort {
                    value, selected, ..
                } => Some((*value, *selected)),
                _ => None,
            })
            .collect();
    assert_eq!(rows[0], ("Identifier", true));
    assert!(rows.iter().all(|(v, _)| *v != crate::sort::DEFAULT_VALUE));
    crate::sort_tests::sort_by(&mut app, "Identifier");
    assert_eq!(identifiers(&app), ["i-0b", "i-0a"]);
}

#[tokio::test]
async fn copy_console_and_cli_follow_the_query() {
    let mut app = searched().await;
    assert_eq!(
        app.get_console_url(),
        "https://us-east-2.console.aws.amazon.com/resource-explorer/home?region=us-east-2#/search"
    );
    assert_eq!(
        selected_arn(&app).as_deref(),
        Some("arn:aws:ec2:us-east-2:123456789012:instance/i-0a")
    );
    press(&mut app, KeyCode::Char('i'));
    typed(&mut app, "-tag:none");
    assert_eq!(
        cli_input(&app),
        serde_json::json!({"QueryString": ""}),
        "the command is the query on screen, not the typed one"
    );
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        cli_input(&app),
        serde_json::json!({"QueryString": "-tag:none"})
    );
}
