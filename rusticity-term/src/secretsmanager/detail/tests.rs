//! App-level tests for the secret detail page: entry, all five tabs, section
//! errors, fresh-only refresh, context teardown, Esc, console, copy, CLI,
//! preferences, bookmarks and background tabs. Completions are injected
//! locally; no test makes an AWS request.
use super::*;
use crate::keymap::handle_key;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};
use rusticity_core::secretsmanager::detail::{SecretReplica, SecretRotation, SecretTag};

const ARN: &str = "arn:aws:secretsmanager:us-east-1:123456789012:secret:prod/db-AbCdEf";
const TWIN: &str = "arn:aws:secretsmanager:us-east-1:123456789012:secret:prod/db-ZyXwVu";

fn row(name: &str, arn: &str) -> SecretMetadata {
    SecretMetadata {
        name: name.into(),
        arn: arn.into(),
        description: "list snapshot description".into(),
        ..Default::default()
    }
}

fn app() -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    app.config.account_id = "123456789012".into();
    app.current_service = Service::SecretsManagerSecrets;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.secretsmanager_state.secrets.items = vec![row("prod/db", ARN), row("prod/db", TWIN)];
    crate::secretsmanager::inventory::mark_loaded(&mut app);
    app
}

fn act(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    let action = handle_key(KeyEvent::new(key, modifiers), app.mode).unwrap();
    app.handle_action(action);
}

fn press(app: &mut App, key: KeyCode) {
    act(app, key, KeyModifiers::NONE);
}

fn description(arn: &str) -> SecretDescription {
    SecretDescription {
        name: "prod/db".into(),
        arn: arn.into(),
        description: "fresh description".into(),
        kms_key_id: "alias/prod".into(),
        owning_service: "rds".into(),
        created_date: "2026-01-02T03:04:05Z".into(),
        last_accessed_date: "2026-10-30".into(),
        rotation: SecretRotation {
            enabled: Some(true),
            lambda_arn: "arn:aws:lambda:us-east-1:123456789012:function:rotate".into(),
            automatically_after_days: Some(30),
            schedule_expression: String::new(),
            window: "2h".into(),
            last_rotated_date: "2026-10-01T00:00:00Z".into(),
            next_rotation_date: "2026-10-31T00:00:00Z".into(),
        },
        replicas: vec![SecretReplica {
            region: "us-west-2".into(),
            status: "InSync".into(),
            ..Default::default()
        }],
        tags: vec![SecretTag {
            key: "team".into(),
            value: "data".into(),
        }],
        ..Default::default()
    }
}

fn detail(versions: Result<Vec<SecretVersion>, String>) -> SecretDetail {
    SecretDetail {
        metadata: description(ARN),
        versions,
    }
}

fn versions() -> Vec<SecretVersion> {
    vec![SecretVersion {
        version_id: "v-current".into(),
        stages: vec!["AWSCURRENT".into(), "AWSPENDING".into()],
        created_date: "2026-10-01T00:00:00Z".into(),
        last_accessed_date: "2026-10-30".into(),
    }]
}

/// Complete the queued read through the real poll path.
async fn finish(app: &mut App, result: Result<SecretDetail, String>) {
    assert!(app
        .secretsmanager_state
        .detail
        .read
        .start(async move { result.map(Box::new) }));
    tokio::task::yield_now().await;
    assert!(poll(app).await);
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

fn open_first(app: &mut App) {
    press(app, KeyCode::Enter);
    assert!(active(app));
}

fn goto(app: &mut App, tab: Tab) {
    while app.secretsmanager_state.detail.view().unwrap().tab != tab {
        press(app, KeyCode::Tab);
    }
}

#[test]
fn enter_opens_exact_filtered_row_and_preserves_inventory() {
    let mut app = app();
    app.secretsmanager_state.secrets.filter = "ZyXwVu".into();
    press(&mut app, KeyCode::Enter);
    assert!(active(&app));
    // Same display name, distinct ARN: the filtered row is the one entered.
    assert_eq!(
        app.secretsmanager_state.detail.source.as_ref().unwrap().arn,
        TWIN
    );
    assert_eq!(app.secretsmanager_state.secrets.filter, "ZyXwVu");
    assert!(
        app.secretsmanager_state.detail.busy(),
        "read queued on entry"
    );
    assert_eq!(app.breadcrumbs(), "Secrets Manager > Secrets > prod/db");
}

#[test]
fn enter_on_empty_inventory_does_nothing() {
    let mut app = app();
    app.secretsmanager_state.secrets.items.clear();
    press(&mut app, KeyCode::Enter);
    assert!(!active(&app));
}

#[test]
fn tabs_cycle_in_console_order_and_only_three_are_tables() {
    use crate::common::CyclicEnum;
    assert_eq!(
        Tab::ALL,
        &[
            Tab::Overview,
            Tab::Rotation,
            Tab::Versions,
            Tab::Replication,
            Tab::Tags
        ]
    );
    // Tab data is indexed by discriminant, so it must match the cycle order.
    for (index, tab) in Tab::ALL.iter().enumerate() {
        assert_eq!(*tab as usize, index);
    }
    let names = |tab: Tab| tab.columns().iter().map(|(n, _)| *n).collect::<Vec<_>>();
    assert_eq!(
        names(Tab::Versions),
        [
            "Version ID",
            "Staging labels",
            "Last accessed",
            "Created on (UTC)"
        ]
    );
    assert_eq!(
        names(Tab::Replication),
        [
            "Region",
            "Replication status",
            "Secret ARN",
            "Encryption key",
            "Last accessed date (UTC)"
        ]
    );
    assert_eq!(Tab::Overview.name(), "Overview");
    assert_eq!(Tab::Tags.next(), Tab::Overview);
    assert!(!Tab::Overview.is_table() && !Tab::Rotation.is_table());
    assert!(Tab::Replication.is_table() && Tab::Versions.is_table() && Tab::Tags.is_table());
}

#[tokio::test]
async fn loading_shows_identity_only_and_no_list_snapshot_metadata() {
    let mut app = app();
    open_first(&mut app);
    let text = screen(&app);
    assert!(text.contains("Secret details") && text.contains("prod/db") && text.contains(ARN));
    assert!(text.contains("Loading…"));
    assert!(!text.contains("list snapshot description"));
}

/// The rows of the fixed "Secret details" pane, read from the rendered screen.
fn header(app: &App) -> Vec<String> {
    let text = screen(app);
    let lines: Vec<String> = text
        .chars()
        .collect::<Vec<_>>()
        .chunks(200)
        .map(|line| line.iter().collect::<String>())
        .collect();
    let start = lines
        .iter()
        .position(|line| line.contains("Secret details"))
        .expect("Secret details pane");
    let end = start
        + lines[start + 1..]
            .iter()
            .position(|line| line.contains('╰'))
            .unwrap()
        + 1;
    lines[start..=end].to_vec()
}

#[tokio::test]
async fn secret_details_pane_is_fixed_across_every_tab() {
    let mut app = app();
    open_first(&mut app);
    finish(&mut app, Ok(detail(Ok(versions())))).await;
    let first = header(&app);
    let text = first.join("\n");
    for field in ["Secret name", "Secret ARN", "Description", "Encryption key"] {
        assert!(text.contains(field), "{field}");
    }
    assert!(text.contains("fresh description") && text.contains("alias/prod"));
    for _ in Tab::ALL {
        press(&mut app, KeyCode::Tab);
        assert_eq!(header(&app), first, "the pane never changes with the tab");
    }
    // Overview holds the remaining fields, not a second copy of the pane.
    assert_eq!(
        app.secretsmanager_state.detail.view().unwrap().tab,
        Tab::Overview
    );
    let fields = view::overview_fields(&description(ARN));
    let labels: Vec<_> = fields.iter().map(|(label, _)| *label).collect();
    assert!(labels.contains(&"Managed by") && !labels.contains(&"Secret ARN"));
}

#[tokio::test]
async fn secret_details_pane_never_shows_list_metadata_while_loading_or_failed() {
    let mut app = app();
    open_first(&mut app);
    let loading = header(&app).join("\n");
    assert!(loading.contains("…") && !loading.contains("list snapshot description"));
    finish(&mut app, Err("AccessDeniedException".into())).await;
    let failed = header(&app).join("\n");
    assert!(failed.contains("unavailable") && !failed.contains("list snapshot description"));
    assert!(failed.contains("prod/db"), "identity stays visible");
}

#[tokio::test]
async fn completion_fills_every_tab_from_fresh_metadata() {
    let mut app = app();
    open_first(&mut app);
    finish(&mut app, Ok(detail(Ok(versions())))).await;
    assert_eq!(
        app.secretsmanager_state.detail.read_status(),
        ReadStatus::Ready
    );

    let text = screen(&app);
    assert!(text.contains("fresh description") && text.contains("alias/prod"));
    assert!(text.contains("Managed by") && text.contains("rds"));
    assert!(!text.contains("list snapshot description"));

    goto(&mut app, Tab::Rotation);
    let text = screen(&app);
    assert!(text.contains("Enabled") && text.contains("Every 30 days"));
    assert!(text.contains("2h") && text.contains("function:rotate"));

    goto(&mut app, Tab::Replication);
    let text = screen(&app);
    assert!(text.contains("Replication (1)") && text.contains("us-west-2"));
    assert!(text.contains("InSync") && text.contains("Replication status"));
    // The replica ARN keeps the name and suffix, in the replica Region.
    assert!(text.contains("arn:aws:secretsmanager:us-west-2:123456789012:secret:prod/db-AbCdEf"));

    goto(&mut app, Tab::Versions);
    let text = screen(&app);
    assert!(text.contains("Versions (1)") && text.contains("v-current"));
    assert!(text.contains("AWSCURRENT, AWSPENDING"));

    goto(&mut app, Tab::Tags);
    let text = screen(&app);
    assert!(text.contains("Tags (1)") && text.contains("team") && text.contains("data"));
}

#[test]
fn replica_arn_swaps_only_the_region_segment() {
    assert_eq!(
        super::replica_arn(ARN, "eu-west-1"),
        "arn:aws:secretsmanager:eu-west-1:123456789012:secret:prod/db-AbCdEf"
    );
    // A name containing colons stays intact.
    assert_eq!(
        super::replica_arn(
            "arn:aws:secretsmanager:us-east-1:123456789012:secret:a:b-AbCdEf",
            "us-west-2"
        ),
        "arn:aws:secretsmanager:us-west-2:123456789012:secret:a:b-AbCdEf"
    );
    assert_eq!(super::replica_arn(ARN, ""), "");
    assert_eq!(super::replica_arn("not-an-arn", "us-west-2"), "");
}

#[tokio::test]
async fn failed_replica_message_is_in_the_expanded_row() {
    let mut app = app();
    open_first(&mut app);
    let mut failed = detail(Ok(versions()));
    failed.metadata.replicas[0].status = "Failed".into();
    failed.metadata.replicas[0].status_message = "AccessDenied on key".into();
    finish(&mut app, Ok(failed)).await;
    goto(&mut app, Tab::Replication);
    assert!(!screen(&app).contains("AccessDenied on key"));
    press(&mut app, KeyCode::Enter);
    assert!(screen(&app).contains("Status message: AccessDenied on key"));
}

#[test]
fn absent_rotation_flag_shows_disabled_like_the_console() {
    let mut meta = description(ARN);
    meta.rotation = SecretRotation {
        enabled: None,
        schedule_expression: "cron(0 4 ? * SUN *)".into(),
        automatically_after_days: Some(7),
        ..Default::default()
    };
    let fields = view::rotation_fields(&meta);
    let get = |label| fields.iter().find(|(l, _)| *l == label).unwrap().1.clone();
    assert_eq!(get("Rotation status"), "Disabled");
    assert_eq!(get("Rotation schedule"), "cron(0 4 ? * SUN *)");
    assert_eq!(get("Rotation function"), "-");
    assert_eq!(get("Next rotation date (UTC)"), "-");
}

#[tokio::test]
async fn empty_sections_say_so_only_after_success() {
    let mut app = app();
    open_first(&mut app);
    let mut empty = detail(Ok(vec![]));
    empty.metadata.replicas.clear();
    empty.metadata.tags.clear();
    goto(&mut app, Tab::Replication);
    assert!(!screen(&app).contains("not replicated"));
    finish(&mut app, Ok(empty)).await;
    assert!(screen(&app).contains("This secret is not replicated to other Regions."));
    goto(&mut app, Tab::Versions);
    assert!(screen(&app).contains("No versions are listed for this secret."));
    goto(&mut app, Tab::Tags);
    assert!(screen(&app).contains("No tags are associated with this secret."));
}

#[tokio::test]
async fn versions_failure_fails_only_its_tab() {
    let mut app = app();
    open_first(&mut app);
    finish(
        &mut app,
        Ok(detail(Err("AccessDeniedException: versions".into()))),
    )
    .await;
    assert_eq!(
        app.secretsmanager_state.detail.read_status(),
        ReadStatus::Failed
    );
    assert!(screen(&app).contains("fresh description"));
    goto(&mut app, Tab::Versions);
    let text = screen(&app);
    assert!(text.contains("AccessDeniedException: versions"));
    assert!(!text.contains("No versions"));
    goto(&mut app, Tab::Tags);
    assert!(screen(&app).contains("team"));
}

#[tokio::test]
async fn whole_failure_shows_errors_everywhere_and_no_data() {
    let mut app = app();
    open_first(&mut app);
    finish(&mut app, Err("ResourceNotFoundException: gone".into())).await;
    for tab in Tab::ALL {
        goto(&mut app, *tab);
        let text = screen(&app);
        assert!(text.contains("ResourceNotFoundException: gone"), "{tab:?}");
        assert!(!text.contains("fresh description"));
    }
}

#[tokio::test]
async fn refresh_clears_every_section_before_the_next_draw() {
    let mut app = app();
    open_first(&mut app);
    finish(&mut app, Ok(detail(Ok(versions())))).await;
    goto(&mut app, Tab::Versions);
    app.secretsmanager_state
        .detail
        .view_mut()
        .unwrap()
        .data_mut()
        .table
        .filter = "current".into();
    act(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL);
    assert!(app.secretsmanager_state.detail.busy());
    let view = app.secretsmanager_state.detail.view().unwrap();
    assert!(view.metadata().is_none());
    assert!(view.tabs.iter().all(|tab| tab.table.items.is_empty()));
    // Filter text and tab survive; rows do not.
    assert_eq!(view.tab, Tab::Versions);
    assert_eq!(view.data().table.filter, "current");
    let text = screen(&app);
    assert!(!text.contains("v-current") && text.contains("Loading…"));

    // A failed refresh never brings the old data back.
    finish(&mut app, Err("ThrottlingException".into())).await;
    let text = screen(&app);
    assert!(text.contains("ThrottlingException") && !text.contains("v-current"));
}

#[tokio::test]
async fn aws_context_change_closes_the_detail_without_repainting_it() {
    let mut app = app();
    open_first(&mut app);
    app.config.region = "eu-west-1".into();
    assert!(poll(&mut app).await);
    assert!(!active(&app));
    assert!(app.secretsmanager_state.detail.source.is_none());

    let mut app = self::app();
    open_first(&mut app);
    finish(&mut app, Ok(detail(Ok(versions())))).await;
    app.config.account_id = "999999999999".into();
    press(&mut app, KeyCode::Enter); // table toggle on summary is a no-op
    act(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL);
    assert!(
        !active(&app),
        "refresh under a new account closes, never reloads"
    );
}

#[tokio::test]
async fn esc_returns_to_untouched_inventory_and_aborts_the_read() {
    let mut app = app();
    app.secretsmanager_state.secrets.filter = "prod".into();
    open_first(&mut app);
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    assert!(app.secretsmanager_state.detail.read.start(async move {
        let _keep = tx;
        std::future::pending::<Result<Box<SecretDetail>, String>>().await
    }));
    press(&mut app, KeyCode::Esc);
    assert!(!active(&app));
    assert_eq!(app.secretsmanager_state.secrets.filter, "prod");
    assert_eq!(app.breadcrumbs(), "Secrets Manager > Secrets");
    // Dropping the read aborted the task, which dropped its sender.
    tokio::task::yield_now().await;
    assert!(rx.await.is_err());
}

#[tokio::test]
async fn console_and_copy_follow_the_current_view() {
    let mut app = app();
    assert!(app
        .get_console_url()
        .ends_with("/secretsmanager/listsecrets?region=us-east-1"));
    open_first(&mut app);
    assert_eq!(
        app.get_console_url(),
        "https://us-east-1.console.aws.amazon.com/secretsmanager/secret?name=prod%2Fdb&region=us-east-1"
    );
    let buffer = ratatui::buffer::Buffer::empty(ratatui::layout::Rect::new(0, 0, 4, 1));
    for _ in Tab::ALL {
        press(&mut app, KeyCode::Char('y'));
        let mut copied = None;
        assert!(app.clipboard.complete_with(&buffer, |text| {
            copied = Some(text);
            Ok(())
        }));
        assert_eq!(copied.as_deref(), Some(ARN), "copies the ARN on every tab");
        press(&mut app, KeyCode::Tab);
    }
    press(&mut app, KeyCode::Esc);
    assert!(app
        .get_console_url()
        .ends_with("/listsecrets?region=us-east-1"));
}

#[test]
fn preferences_route_to_the_entered_table_and_summaries_are_read_only() {
    use crate::preferences::bindings::model_for;
    let mut app = app();
    assert!(model_for(&app, Service::SecretsManagerSecrets).column_count() >= 11);
    open_first(&mut app);
    let model = model_for(&app, Service::SecretsManagerSecrets);
    assert!(!model.is_enabled() && model.column_count() == 0);
    goto(&mut app, Tab::Versions);
    assert_eq!(
        model_for(&app, Service::SecretsManagerSecrets).column_count(),
        4
    );
    goto(&mut app, Tab::Replication);
    assert_eq!(
        model_for(&app, Service::SecretsManagerSecrets).column_count(),
        5
    );
    assert!(screen(&app).contains("Replication status"));
    // Hiding a replication column changes only that table.
    let view = app.secretsmanager_state.detail.view_mut().unwrap();
    view.data_mut()
        .visible_column_ids
        .retain(|id| *id != "Replication status");
    let text = screen(&app);
    assert!(!text.contains("Replication status"));
    goto(&mut app, Tab::Versions);
    assert!(screen(&app).contains("Staging labels") && screen(&app).contains("Last accessed"));
}

#[tokio::test]
async fn filter_narrows_the_active_table_only() {
    let mut app = app();
    open_first(&mut app);
    let mut many = versions();
    many.push(SecretVersion {
        version_id: "v-previous".into(),
        stages: vec!["AWSPREVIOUS".into()],
        ..Default::default()
    });
    finish(&mut app, Ok(detail(Ok(many)))).await;
    goto(&mut app, Tab::Versions);
    press(&mut app, KeyCode::Char('i'));
    assert_eq!(app.mode, Mode::FilterInput);
    for c in "previous".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    press(&mut app, KeyCode::Enter);
    let text = screen(&app);
    assert!(text.contains("v-previous") && !text.contains("v-current"));
    assert!(app.secretsmanager_state.secrets.filter.is_empty());
    goto(&mut app, Tab::Tags);
    assert!(screen(&app).contains("team"));
}

#[test]
fn secrets_are_bookmarkable_from_inventory_and_detail() {
    use rusticity_core::finder::ResourceKind;
    assert!(crate::finder::navigation::has_detail(ResourceKind::Secret));
    let mut app = app();
    let selected = crate::finder::related::selected(&app).unwrap();
    assert_eq!(selected.arn, ARN);
    assert!(crate::bookmarks::available(&app));
    app.secretsmanager_state.secrets.filter = "ZyXwVu".into();
    open_first(&mut app);
    // The entered secret is the selection, even if the highlight moves.
    app.secretsmanager_state.secrets.filter.clear();
    app.secretsmanager_state.secrets.selected = 0;
    assert_eq!(crate::finder::related::selected(&app).unwrap().arn, TWIN);
    assert!(crate::bookmarks::available(&app));
    assert!(screen(&app).contains("bookmark"));
}

#[tokio::test]
async fn background_tab_keeps_loading_and_completes_without_focus() {
    use crate::app::Tab as AppTab;
    use crate::tab_runtime::{self, status_for, TabId};
    let mut app = app();
    let first = TabId::new();
    app.tabs.push(AppTab {
        id: first,
        service: Service::SecretsManagerSecrets,
        title: "Secrets Manager › Secrets".into(),
        breadcrumb: "Secrets Manager › Secrets".into(),
    });
    app.current_tab = 0;
    tab_runtime::sync_active(&mut app);
    // The fresh tab queued its list read; complete it with the fixture rows.
    app.secretsmanager_state.secrets.items = vec![row("prod/db", ARN), row("prod/db", TWIN)];
    crate::secretsmanager::inventory::mark_loaded(&mut app);
    open_first(&mut app);
    let (tx, rx) = tokio::sync::oneshot::channel();
    assert!(app
        .secretsmanager_state
        .detail
        .read
        .start(async move { rx.await.unwrap() }));
    assert!(matches!(
        status_for(&app, first),
        ReadStatus::Loading { .. }
    ));
    assert!(screen(&app).contains("Loading secret details"));

    // Open a second tab: the first read keeps running in the background.
    let second = TabId::new();
    app.tabs.push(AppTab {
        id: second,
        service: Service::SecretsManagerSecrets,
        title: "Second".into(),
        breadcrumb: "Second".into(),
    });
    app.current_tab = 1;
    tab_runtime::sync_active(&mut app);
    assert!(!active(&app), "the new tab starts on the inventory");
    assert!(matches!(
        status_for(&app, first),
        ReadStatus::Loading { .. }
    ));
    assert!(tab_runtime::has_background_work(&app));

    tx.send(Ok(Box::new(detail(Ok(versions()))))).unwrap();
    tokio::task::yield_now().await;
    assert!(tab_runtime::poll(&mut app).await);
    assert_eq!(app.current_tab, 1, "completion never steals focus");
    assert!(!active(&app));
    assert_eq!(status_for(&app, first), ReadStatus::Ready);
    assert!(screen(&app).contains("Secrets Manager › Secrets ✓"));

    app.handle_action(Action::PrevTab);
    assert!(active(&app));
    assert!(screen(&app).contains("fresh description"));
}
