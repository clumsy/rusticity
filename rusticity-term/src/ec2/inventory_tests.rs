//! EC2 instance list as a tab-owned read. Completions are injected through
//! the shared drive path; no test makes an AWS request.
use super::inventory;
use crate::app::{App, Ec2Instance, Service};
use crate::async_read::{ReadScope, ReadStatus};
use crate::keymap::{Action, Mode};
use ratatui::{backend::TestBackend, Terminal};

fn app() -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-2".into()));
    app.config.account_id = "123456789012".into();
    app.current_service = Service::Ec2Instances;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app
}

fn instance(id: &str, state: &str) -> Ec2Instance {
    Ec2Instance {
        instance_id: id.into(),
        name: format!("{id}-name"),
        state: state.into(),
        ..Default::default()
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

async fn finish(app: &mut App, result: Result<Vec<Ec2Instance>, String>) {
    let scope = ReadScope::from_app(app, "inventory:ec2");
    let state = &mut app.ec2_state;
    assert!(
        state
            .inventory
            .drive(
                &mut state.table,
                &scope,
                "EC2 instances read failed",
                || async move { result }
            )
            .await
    );
    tokio::task::yield_now().await;
    assert!(inventory::poll(app).await);
}

#[tokio::test]
async fn the_list_loads_and_never_claims_empty_before_success() {
    let mut app = app();
    inventory::request(&mut app);
    let text = screen(&app);
    assert!(text.contains("Loading…") && !text.contains("No instances"));
    finish(&mut app, Ok(vec![instance("i-0a", "running")])).await;
    assert_eq!(app.ec2_state.load_status(), ReadStatus::Ready);
    assert!(screen(&app).contains("i-0a"));
    inventory::request(&mut app);
    finish(&mut app, Ok(vec![])).await;
    assert!(screen(&app).contains("No instances in this Region."));
}

#[tokio::test]
async fn refresh_clears_at_once_and_a_failure_never_shows_old_rows() {
    let mut app = app();
    inventory::request(&mut app);
    finish(&mut app, Ok(vec![instance("i-0a", "running")])).await;
    app.handle_action(Action::Refresh);
    assert!(app.ec2_state.inventory.is_busy());
    assert!(!screen(&app).contains("i-0a"), "cleared at once");
    assert!(!app.log_groups_state.loading, "no log group reload");
    finish(
        &mut app,
        Err("UnauthorizedOperation: ec2:DescribeInstances".into()),
    )
    .await;
    let text = screen(&app);
    assert!(text.contains("UnauthorizedOperation") && !text.contains("i-0a"));
    assert_eq!(app.ec2_state.load_status(), ReadStatus::Failed);
    assert_ne!(app.mode, Mode::ErrorModal, "an inline error, no modal");
}

#[tokio::test]
async fn esc_cancels_and_context_change_drops_rows() {
    let mut app = app();
    inventory::request(&mut app);
    app.handle_action(Action::GoBack);
    assert!(!app.ec2_state.inventory.is_busy());
    assert!(screen(&app).contains("Cancelled"));
    let mut app = self::app();
    inventory::request(&mut app);
    finish(&mut app, Ok(vec![instance("i-0a", "running")])).await;
    app.config.region = "eu-west-1".into();
    assert!(inventory::poll(&mut app).await);
    assert!(app.ec2_state.table.items.is_empty());
}

#[tokio::test]
async fn a_finder_open_enters_the_exact_instance_even_if_stopped() {
    let mut app = app();
    app.ec2_state.state_filter = crate::ui::ec2::StateFilter::Running;
    inventory::request(&mut app);
    app.ec2_state.pending_focus = Some("i-0b".into());
    finish(
        &mut app,
        Ok(vec![
            instance("i-0a", "running"),
            instance("i-0b", "stopped"),
        ]),
    )
    .await;
    assert_eq!(app.ec2_state.current_instance.as_deref(), Some("i-0b"));
    let mut app = self::app();
    inventory::request(&mut app);
    app.ec2_state.pending_focus = Some("i-gone".into());
    finish(&mut app, Ok(vec![instance("i-0a", "running")])).await;
    assert!(app.ec2_state.current_instance.is_none());
    let text = screen(&app);
    assert!(text.contains("Resource not found") && !text.contains("i-0a"));
}
