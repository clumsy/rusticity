//! The same App-level contract, checked for each migrated root inventory:
//! SQS queues, Lambda functions and DynamoDB tables. No test makes an AWS
//! request; completions are injected through the real tab-runtime poll.
use crate::{
    app::{App, Service, Tab},
    async_read::ReadStatus,
    keymap::{Action, Mode},
    tab_runtime::{self, status_for, TabId},
};

fn app_with_tab(service: Service) -> (App, TabId) {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    app.config.account_id = "123456789012".into();
    let id = TabId::new();
    app.tabs.push(Tab {
        id,
        service,
        title: service.name().into(),
        breadcrumb: service.name().into(),
    });
    app.current_tab = 0;
    app.current_service = service;
    app.service_selected = true;
    app.mode = Mode::Normal;
    tab_runtime::sync_active(&mut app);
    (app, id)
}

fn screen(app: &App) -> String {
    use ratatui::{backend::TestBackend, Terminal};
    let mut t = Terminal::new(TestBackend::new(200, 30)).unwrap();
    t.draw(|f| crate::ui::render(f, app)).unwrap();
    t.backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

macro_rules! inventory_contract {
    (
        $module:ident,
        service: $service:expr,
        state: $state:ident,
        table: $table:ident,
        row_type: $ty:ty,
        row: $row:expr,
        enter: $enter:expr,
        entered: $entered:expr,
        loading_label: $label:expr,
        empty: $empty:expr $(,)?
    ) => {
        mod $module {
            use super::*;

            fn rows() -> Vec<$ty> {
                vec![$row("alpha"), $row("beta")]
            }

            fn enter(app: &mut App) {
                ($enter)(app)
            }

            fn entered(app: &App) -> bool {
                ($entered)(app)
            }

            async fn finish(app: &mut App, result: Result<Vec<$ty>, String>) {
                assert!(app.$state.inventory.start_for_test(async move { result }));
                tokio::task::yield_now().await;
                assert!(tab_runtime::poll(app).await);
            }

            #[tokio::test]
            async fn new_tab_queues_a_read_and_loading_never_blocks_or_shows_rows() {
                let (app, id) = app_with_tab($service);
                assert!(matches!(status_for(&app, id), ReadStatus::Loading { .. }));
                assert!(tab_runtime::has_background_work(&app));
                let text = screen(&app);
                assert!(text.contains("Loading…"), "table body says Loading");
                assert!(text.contains($label), "status bar names the read");
                assert!(!text.contains($empty), "never claims empty while loading");
            }

            #[tokio::test]
            async fn success_fills_rows_and_marks_the_tab() {
                let (mut app, id) = app_with_tab($service);
                finish(&mut app, Ok(rows())).await;
                assert_eq!(status_for(&app, id), ReadStatus::Ready);
                assert_eq!(app.$state.$table.items.len(), 2);
                let text = screen(&app);
                assert!(text.contains("alpha") && text.contains("beta"));
                assert!(text.contains(" ✓"));
            }

            #[tokio::test]
            async fn empty_success_says_so_only_after_the_read() {
                let (mut app, id) = app_with_tab($service);
                finish(&mut app, Ok(vec![])).await;
                assert_eq!(status_for(&app, id), ReadStatus::Ready);
                assert!(screen(&app).contains($empty));
            }

            #[tokio::test]
            async fn refresh_clears_rows_at_once_and_failure_never_restores_them() {
                let (mut app, id) = app_with_tab($service);
                finish(&mut app, Ok(rows())).await;
                app.handle_action(Action::Refresh);
                assert!(app.$state.$table.items.is_empty());
                assert!(matches!(status_for(&app, id), ReadStatus::Loading { .. }));
                assert!(!screen(&app).contains("alpha"));
                assert!(!app.log_groups_state.loading, "no hidden log-group reload");

                finish(&mut app, Err("AccessDeniedException: denied".into())).await;
                assert_eq!(status_for(&app, id), ReadStatus::Failed);
                let text = screen(&app);
                assert!(text.contains("AccessDeniedException") && !text.contains("alpha"));
                assert_eq!(app.mode, Mode::Normal, "inline error, no modal focus steal");
            }

            #[tokio::test]
            async fn esc_cancels_a_busy_read_with_a_retry_hint() {
                let (mut app, id) = app_with_tab($service);
                assert!(app.$state.inventory.start_for_test(async {
                    std::future::pending::<Result<Vec<$ty>, String>>().await
                }));
                app.handle_action(Action::GoBack);
                assert_eq!(status_for(&app, id), ReadStatus::Failed);
                assert!(screen(&app).contains("Cancelled"));
                assert!(!tab_runtime::has_background_work(&app));
            }

            #[tokio::test]
            async fn background_tab_finishes_without_stealing_focus() {
                let (mut app, first) = app_with_tab($service);
                let (tx, rx) = tokio::sync::oneshot::channel();
                assert!(app
                    .$state
                    .inventory
                    .start_for_test(async move { rx.await.unwrap() }));
                let second = TabId::new();
                app.tabs.push(Tab {
                    id: second,
                    service: $service,
                    title: "Second".into(),
                    breadcrumb: "Second".into(),
                });
                app.current_tab = 1;
                tab_runtime::sync_active(&mut app);
                assert!(matches!(
                    status_for(&app, first),
                    ReadStatus::Loading { .. }
                ));

                tx.send(Ok(rows())).unwrap();
                tokio::task::yield_now().await;
                assert!(tab_runtime::poll(&mut app).await);
                assert_eq!(app.current_tab, 1);
                assert_eq!(status_for(&app, first), ReadStatus::Ready);
                assert!(
                    app.$state.$table.items.is_empty(),
                    "second tab owns its rows"
                );
                app.handle_action(Action::PrevTab);
                assert_eq!(app.$state.$table.items.len(), 2);
            }

            #[tokio::test]
            async fn refresh_inside_an_entered_detail_keeps_its_rows() {
                let (mut app, id) = app_with_tab($service);
                finish(&mut app, Ok(rows())).await;
                enter(&mut app);
                app.handle_action(Action::Refresh);
                assert!(entered(&app), "still entered");
                assert_eq!(app.$state.$table.items.len(), 2);
                assert_eq!(status_for(&app, id), ReadStatus::Ready, "no stuck spinner");
            }

            #[tokio::test]
            async fn aws_context_change_drops_rows_and_closes_the_detail() {
                let (mut app, _) = app_with_tab($service);
                finish(&mut app, Ok(rows())).await;
                enter(&mut app);
                app.config.region = "eu-west-1".into();
                tab_runtime::poll(&mut app).await;
                assert!(app.$state.$table.items.is_empty());
                assert!(!entered(&app));
            }

            #[tokio::test]
            async fn inline_finder_load_is_not_replaced_by_the_queued_read() {
                let (mut app, id) = app_with_tab($service);
                // A finder open loads one row in-line, then marks it loaded.
                app.$state.$table.items = vec![$row("only")];
                crate::$module::inventory::mark_loaded(&mut app);
                assert!(
                    !tab_runtime::poll(&mut app).await,
                    "queued read was dropped"
                );
                assert_eq!(status_for(&app, id), ReadStatus::Ready);
                assert_eq!(app.$state.$table.items.len(), 1);
            }
        }
    };
}

fn queue(name: &str) -> crate::sqs::Queue {
    crate::sqs::Queue {
        name: name.into(),
        url: format!("https://sqs.us-east-1.amazonaws.com/123456789012/{name}"),
        ..Default::default()
    }
}

fn function(name: &str) -> crate::lambda::Function {
    crate::lambda::Function {
        name: name.into(),
        arn: format!("arn:aws:lambda:us-east-1:123456789012:function:{name}"),
        application: None,
        description: String::new(),
        package_type: "Zip".into(),
        runtime: "python3.12".into(),
        architecture: String::new(),
        code_size: 0,
        code_sha256: String::new(),
        memory_mb: 128,
        timeout_seconds: 30,
        last_modified: String::new(),
        layers: vec![],
    }
}

fn table(name: &str) -> rusticity_core::dynamodb::DdbTable {
    rusticity_core::dynamodb::DdbTable {
        name: name.into(),
        status: "ACTIVE".into(),
        ..Default::default()
    }
}

inventory_contract!(
    sqs,
    service: Service::SqsQueues,
    state: sqs_state,
    table: queues,
    row_type: crate::sqs::Queue,
    row: queue,
    enter: |app: &mut App| app.sqs_state.current_queue = Some(app.sqs_state.queues.items[0].url.clone()),
    entered: |app: &App| app.sqs_state.current_queue.is_some(),
    loading_label: "Loading SQS queues",
    empty: "No queues in this Region.",
);

inventory_contract!(
    lambda,
    service: Service::LambdaFunctions,
    state: lambda_state,
    table: table,
    row_type: crate::lambda::Function,
    row: function,
    enter: |app: &mut App| app.lambda_state.current_function = Some("alpha".into()),
    entered: |app: &App| app.lambda_state.current_function.is_some(),
    loading_label: "Loading Lambda functions",
    empty: "No functions in this Region.",
);

inventory_contract!(
    dynamodb,
    service: Service::DynamoDbTables,
    state: dynamodb_state,
    table: tables,
    row_type: rusticity_core::dynamodb::DdbTable,
    row: table,
    enter: |app: &mut App| app.dynamodb_state.current_table = Some("alpha".into()),
    entered: |app: &App| app.dynamodb_state.current_table.is_some(),
    loading_label: "Loading DynamoDB tables",
    empty: "No tables in this Region.",
);
