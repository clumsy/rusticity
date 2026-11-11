//! App-level tests for the DB instance detail page. Completions are injected
//! through the real poll path; no test makes an AWS request.
use super::*;
use crate::keymap::handle_key;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};

const ARN: &str = "arn:aws:rds:us-east-2:123456789012:db:orders";
const TWIN: &str = "arn:aws:rds:us-east-2:123456789012:db:orders-replica";

fn row(identifier: &str, arn: &str) -> RdsInstance {
    RdsInstance {
        identifier: identifier.into(),
        arn: arn.into(),
        engine: "list-snapshot-engine".into(),
        ..Default::default()
    }
}

fn app() -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-2".into()));
    app.config.account_id = "123456789012".into();
    app.current_service = Service::RdsInstances;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.rds_state.instances.items = vec![row("orders", ARN), row("orders-replica", TWIN)];
    crate::rds::inventory::mark_loaded(&mut app);
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

fn detail() -> RdsInstanceDetail {
    RdsInstanceDetail {
        identifier: "orders".into(),
        arn: ARN.into(),
        status: "available".into(),
        role: "Instance".into(),
        engine: "postgres".into(),
        engine_version: "17.1".into(),
        instance_class: "db.t4g.micro".into(),
        availability_zone: "us-east-2a".into(),
        endpoint: "orders.example".into(),
        port: Some(5432),
        security_groups: vec!["sg-1 (active)".into()],
        publicly_accessible: Some(false),
        master_username: "admin".into(),
        deletion_protection: Some(true),
        max_storage_gib: None,
        backup_retention_days: Some(7),
        maintenance_window: "sun:05:00-sun:05:30".into(),
        tags: vec![("team".into(), "data".into())],
        ..Default::default()
    }
}

async fn finish(app: &mut App, result: Result<RdsInstanceDetail, String>) {
    assert!(app
        .rds_state
        .detail
        .read
        .start(async move { result.map(Box::new) }));
    tokio::task::yield_now().await;
    assert!(poll(app).await);
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

fn goto(app: &mut App, tab: Tab) {
    while app.rds_state.detail.view().unwrap().tab != tab {
        press(app, KeyCode::Tab);
    }
}

#[test]
fn tabs_follow_console_names_and_order() {
    let names: Vec<_> = Tab::ALL.iter().map(|t| t.name()).collect();
    assert_eq!(
        names,
        [
            "Connectivity & security",
            "Monitoring",
            "Logs & events",
            "Configuration",
            "Zero-ETL integrations",
            "Maintenance & backups",
            "Data migrations",
            "Tags",
            "Recommendations",
        ]
    );
    for (index, tab) in Tab::ALL.iter().enumerate() {
        assert_eq!(*tab as usize, index);
    }
    assert_eq!(Tab::Recommendations.next(), Tab::Connectivity);
}

#[test]
fn enter_opens_the_exact_filtered_row_and_keeps_the_inventory() {
    let mut app = app();
    app.rds_state.instances.filter = "replica".into();
    press(&mut app, KeyCode::Enter);
    assert!(active(&app));
    assert_eq!(app.rds_state.detail.source.as_ref().unwrap().arn, TWIN);
    assert_eq!(app.rds_state.instances.filter, "replica");
    assert!(app.rds_state.detail.busy(), "read queued on entry");
    assert_eq!(app.breadcrumbs(), "RDS > Databases > orders-replica");
}

#[test]
fn enter_on_an_empty_list_or_a_row_without_arn_does_nothing() {
    let mut app = app();
    app.rds_state.instances.items.clear();
    press(&mut app, KeyCode::Enter);
    assert!(!active(&app));
    app.rds_state.instances.items = vec![row("no-arn", "")];
    press(&mut app, KeyCode::Enter);
    assert!(!active(&app));
}

#[tokio::test]
async fn summary_shows_identity_only_until_the_fresh_read() {
    let mut app = app();
    press(&mut app, KeyCode::Enter);
    let text = screen(&app);
    assert!(text.contains("Summary") && text.contains("orders"));
    assert!(text.contains("Loading…") && text.contains("…"));
    assert!(!text.contains("list-snapshot-engine"));
    finish(&mut app, Ok(detail())).await;
    let text = screen(&app);
    assert!(text.contains("PostgreSQL") && text.contains("db.t4g.micro"));
    assert!(text.contains("Available"));
    assert!(!text.contains("list-snapshot-engine"));
}

#[tokio::test]
async fn every_tab_renders_its_fields_and_the_summary_stays_fixed() {
    let mut app = app();
    press(&mut app, KeyCode::Enter);
    finish(&mut app, Ok(detail())).await;
    let text = screen(&app);
    assert!(text.contains("orders.example") && text.contains("5432"));
    assert!(text.contains("sg-1 (active)") && text.contains("Publicly accessible"));
    goto(&mut app, Tab::Configuration);
    let text = screen(&app);
    assert!(text.contains("Master username") && text.contains("admin"));
    assert!(text.contains("Deletion protection") && text.contains("Enabled"));
    assert!(text.contains("Storage autoscaling") && text.contains("Disabled"));
    goto(&mut app, Tab::Maintenance);
    let text = screen(&app);
    assert!(text.contains("Enabled (7 days)") && text.contains("sun:05:00-sun:05:30"));
    goto(&mut app, Tab::Tags);
    let text = screen(&app);
    assert!(text.contains("Tags (1)") && text.contains("team") && text.contains("data"));
    assert!(
        text.contains("Summary") && text.contains("PostgreSQL"),
        "summary stays"
    );
}

#[test]
fn absent_values_show_a_dash_and_zero_retention_is_disabled() {
    let d = RdsInstanceDetail {
        backup_retention_days: Some(0),
        ..Default::default()
    };
    let get = |fields: Vec<(&str, String)>, label: &str| {
        fields.into_iter().find(|(l, _)| *l == label).unwrap().1
    };
    assert_eq!(
        get(view::maintenance_fields(&d), "Automated backups"),
        "Disabled"
    );
    assert_eq!(get(view::connectivity_fields(&d), "Port"), "-");
    assert_eq!(
        get(view::connectivity_fields(&d), "Publicly accessible"),
        "-"
    );
    let config: Vec<_> = view::configuration_panes(&d)
        .into_iter()
        .flat_map(|(_, fields)| fields)
        .collect();
    assert_eq!(get(config.clone(), "Encryption"), "-");
    assert_eq!(get(config, "Storage file system configuration"), "-");
}

#[tokio::test]
async fn empty_tags_say_so_only_after_success() {
    let mut app = app();
    press(&mut app, KeyCode::Enter);
    goto(&mut app, Tab::Tags);
    assert!(!screen(&app).contains("No tags"));
    finish(
        &mut app,
        Ok(RdsInstanceDetail {
            tags: vec![],
            ..detail()
        }),
    )
    .await;
    assert!(screen(&app).contains("No tags are associated with this database."));
}

#[tokio::test]
async fn refresh_clears_everything_and_a_failure_never_restores_it() {
    let mut app = app();
    press(&mut app, KeyCode::Enter);
    finish(&mut app, Ok(detail())).await;
    goto(&mut app, Tab::Tags);
    ctrl(&mut app, 'r');
    assert!(app.rds_state.detail.busy());
    let view = app.rds_state.detail.view().unwrap();
    assert!(view.detail().is_none());
    assert!(view.tables.iter().all(|t| t.rows.items.is_empty()));
    assert_eq!(view.tab, Tab::Tags, "tab survives");
    assert!(!screen(&app).contains("team"));
    finish(
        &mut app,
        Err("AccessDenied: rds:DescribeDBInstances".into()),
    )
    .await;
    let text = screen(&app);
    assert!(text.contains("AccessDenied") && !text.contains("team"));
    goto(&mut app, Tab::Connectivity);
    let text = screen(&app);
    assert!(text.contains("AccessDenied") && text.contains("unavailable"));
    assert!(!text.contains("orders.example"));
}

#[tokio::test]
async fn aws_context_change_closes_the_page() {
    let mut app = app();
    press(&mut app, KeyCode::Enter);
    app.config.region = "eu-west-1".into();
    assert!(poll(&mut app).await);
    assert!(!active(&app));
}

#[tokio::test]
async fn esc_returns_to_the_untouched_inventory_and_aborts_the_read() {
    let mut app = app();
    app.rds_state.instances.filter = "orders".into();
    press(&mut app, KeyCode::Enter);
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    assert!(app.rds_state.detail.read.start(async move {
        let _keep = tx;
        std::future::pending::<Result<Box<RdsInstanceDetail>, String>>().await
    }));
    press(&mut app, KeyCode::Esc);
    assert!(!active(&app));
    assert_eq!(app.rds_state.instances.filter, "orders");
    assert_eq!(app.breadcrumbs(), "RDS > Databases");
    tokio::task::yield_now().await;
    assert!(rx.await.is_err(), "the read task was aborted");
}

#[tokio::test]
async fn console_copy_and_cli_follow_the_current_view() {
    let mut app = app();
    let list = "https://us-east-2.console.aws.amazon.com/rds/home?region=us-east-2#databases:";
    assert_eq!(app.get_console_url(), list);
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.get_console_url(),
        "https://us-east-2.console.aws.amazon.com/rds/home?region=us-east-2#database:id=orders;is-cluster=false"
    );
    let buffer = ratatui::buffer::Buffer::empty(ratatui::layout::Rect::new(0, 0, 4, 1));
    for _ in Tab::ALL {
        press(&mut app, KeyCode::Char('y'));
        let mut copied = None;
        assert!(app.clipboard.complete_with(&buffer, |text| {
            copied = Some(text);
            Ok(())
        }));
        assert_eq!(copied.as_deref(), Some(ARN));
        press(&mut app, KeyCode::Tab);
    }
    let command = crate::cli_command::invocation(&app).unwrap().display;
    assert!(
        command.starts_with("aws rds describe-db-instances "),
        "{command}"
    );
    goto(&mut app, Tab::Tags);
    let command = crate::cli_command::invocation(&app).unwrap().display;
    assert!(
        command.starts_with("aws rds list-tags-for-resource "),
        "{command}"
    );
    assert!(command.contains(ARN));
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.get_console_url(), list);
}

#[test]
fn preferences_route_to_tags_and_summary_tabs_are_read_only() {
    use crate::preferences::bindings::model_for;
    let mut app = app();
    let inventory_columns = model_for(&app, Service::RdsInstances).column_count();
    assert!(inventory_columns > 2);
    press(&mut app, KeyCode::Enter);
    goto(&mut app, Tab::Configuration);
    let model = model_for(&app, Service::RdsInstances);
    assert!(!model.is_enabled() && model.column_count() == 0);
    for (tab, count) in [
        (Tab::Tags, 2),
        (Tab::ZeroEtl, 9),
        (Tab::DataMigrations, 16),
        (Tab::Recommendations, 10),
        (Tab::Monitoring, 0),
        (Tab::LogsEvents, 3),
        (Tab::Connectivity, 6),
    ] {
        goto(&mut app, tab);
        assert_eq!(
            model_for(&app, Service::RdsInstances).column_count(),
            count,
            "{tab:?}"
        );
    }
}

#[tokio::test]
async fn tags_filter_narrows_only_the_tags_table() {
    let mut app = app();
    press(&mut app, KeyCode::Enter);
    finish(
        &mut app,
        Ok(RdsInstanceDetail {
            tags: vec![
                ("env".into(), "prod".into()),
                ("team".into(), "data".into()),
            ],
            ..detail()
        }),
    )
    .await;
    goto(&mut app, Tab::Tags);
    press(&mut app, KeyCode::Char('i'));
    for c in "prod".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    press(&mut app, KeyCode::Enter);
    let text = screen(&app);
    assert!(text.contains("env") && !text.contains("team"));
    assert!(app.rds_state.instances.filter.is_empty());
}

#[test]
fn databases_are_bookmarkable_and_the_entered_one_is_the_selection() {
    use rusticity_core::finder::ResourceKind;
    assert!(crate::finder::navigation::has_detail(
        ResourceKind::Database
    ));
    let mut app = app();
    assert!(crate::bookmarks::available(&app));
    app.rds_state.instances.filter = "replica".into();
    press(&mut app, KeyCode::Enter);
    app.rds_state.instances.filter.clear();
    app.rds_state.instances.selected = 0;
    assert_eq!(crate::finder::related::selected(&app).unwrap().arn, TWIN);
    assert!(screen(&app).contains("bookmark"));
}

#[tokio::test]
async fn background_tab_keeps_loading_and_never_steals_focus() {
    use crate::app::Tab as AppTab;
    use crate::tab_runtime::{self, status_for, TabId};
    let mut app = app();
    let first = TabId::new();
    app.tabs.push(AppTab {
        id: first,
        service: Service::RdsInstances,
        title: "RDS › Databases".into(),
        breadcrumb: "RDS › Databases".into(),
    });
    app.current_tab = 0;
    tab_runtime::sync_active(&mut app);
    // The fresh tab queued its list read; complete it with the fixture rows.
    app.rds_state.instances.items = vec![row("orders", ARN), row("orders-replica", TWIN)];
    crate::rds::inventory::mark_loaded(&mut app);
    press(&mut app, KeyCode::Enter);
    let (tx, rx) = tokio::sync::oneshot::channel();
    assert!(app
        .rds_state
        .detail
        .read
        .start(async move { rx.await.unwrap() }));
    assert!(screen(&app).contains("Loading database details"));
    app.tabs.push(AppTab {
        id: TabId::new(),
        service: Service::RdsInstances,
        title: "Second".into(),
        breadcrumb: "Second".into(),
    });
    app.current_tab = 1;
    tab_runtime::sync_active(&mut app);
    assert!(matches!(
        status_for(&app, first),
        ReadStatus::Loading { .. }
    ));
    tx.send(Ok(Box::new(detail()))).unwrap();
    tokio::task::yield_now().await;
    assert!(tab_runtime::poll(&mut app).await);
    assert_eq!(app.current_tab, 1, "completion never steals focus");
    assert_eq!(status_for(&app, first), ReadStatus::Ready);
    app.handle_action(Action::PrevTab);
    assert!(active(&app) && screen(&app).contains("orders.example"));
}

mod console_tabs {
    use super::*;
    use rusticity_core::rds::detail::sections::{
        RdsAlarm, RdsComputeResource, RdsDataMigration, RdsEvent, RdsIamRole, RdsIntegration,
        RdsLogFile, RdsMetric, RdsRecommendation, RdsReplica, RdsSecurityGroupRule, Sections,
        METRICS,
    };

    fn full() -> RdsInstanceDetail {
        RdsInstanceDetail {
            cpu: "3.2%".into(),
            current_activity: "0.01 AAS".into(),
            recommendations: "1".into(),
            status: "configuring-enhanced-monitoring".into(),
            iam_roles: vec![RdsIamRole {
                role_arn: "arn:aws:iam::123456789012:role/rds-s3-import".into(),
                feature: "s3Import".into(),
                status: "ACTIVE".into(),
            }],
            sections: Sections {
                compute: Ok(vec![RdsComputeResource {
                    identifier: "i-0abc".into(),
                    resource_type: "EC2 instance".into(),
                    availability_zone: "us-east-2a".into(),
                    db_security_group: "rds-ec2-1 (sg-db)".into(),
                    compute_security_group: "ec2-rds-1 (sg-ec2)".into(),
                    proxy: String::new(),
                }]),
                security_group_rules: Ok(vec![
                    RdsSecurityGroupRule {
                        security_group: "default (sg-1)".into(),
                        rule_type: "CIDR/IP - Outbound".into(),
                        rule: "0.0.0.0/0".into(),
                    },
                    RdsSecurityGroupRule {
                        security_group: "default (sg-1)".into(),
                        rule_type: "EC2 Security Group - Inbound".into(),
                        rule: "sg-1".into(),
                    },
                ]),
                replication: Ok(vec![RdsReplica {
                    identifier: "orders".into(),
                    role: "Instance".into(),
                    availability_zone: "us-east-2a".into(),
                    ..Default::default()
                }]),
                monitoring: Ok(METRICS
                    .iter()
                    .map(|(name, unit)| RdsMetric {
                        name,
                        unit,
                        points: vec![(0, 1.0), (300, 2.0)],
                    })
                    .collect()),
                alarms: Ok(vec![RdsAlarm {
                    name: "orders-cpu-high".into(),
                    state: "OK".into(),
                    condition: "CPUUtilization > 80".into(),
                }]),
                events: Ok(vec![RdsEvent {
                    time: "2026-01-02T00:00:00Z".into(),
                    message: "Finished applying modification".into(),
                }]),
                logs: Ok(vec![RdsLogFile {
                    name: "error/postgres.log".into(),
                    last_written: "2026-01-02T00:00:00Z".into(),
                    size: Some(979),
                }]),
                integrations: Ok(vec![RdsIntegration {
                    name: "orders-to-redshift".into(),
                    status: "active".into(),
                    source: "orders".into(),
                    source_arn: ARN.into(),
                    destination_account: "210987654321".into(),
                    destination_type: "Amazon Redshift".into(),
                    destination_arn: "arn:aws:redshift:us-east-2:210987654321:namespace:n".into(),
                    created_time: "2026-01-01T00:00:00Z".into(),
                    ..Default::default()
                }]),
                migrations: Ok(vec![RdsDataMigration {
                    name: "orders-copy".into(),
                    status: "RUNNING".into(),
                    progress: "45%".into(),
                    migration_type: "full-load-and-cdc".into(),
                    server_name: "orders.example".into(),
                    port: Some(5432),
                    cloudwatch_logs: Some(true),
                    tables_loaded: Some(3),
                    elapsed_millis: Some(3_725_000),
                    arn: "arn:aws:dms:us-east-2:123456789012:data-migration:m".into(),
                    ..Default::default()
                }]),
                recommendations: Ok(vec![
                    RdsRecommendation {
                        severity: "low".into(),
                        detection: "Enhanced Monitoring is off".into(),
                        status: "active".into(),
                        resource_identifier: "orders".into(),
                        ..Default::default()
                    },
                    RdsRecommendation {
                        severity: "medium".into(),
                        detection: "Old engine version".into(),
                        status: "resolved".into(),
                        ..Default::default()
                    },
                ]),
            },
            ..detail()
        }
    }

    async fn opened(detail: RdsInstanceDetail) -> App {
        let mut app = app();
        press(&mut app, KeyCode::Enter);
        finish(&mut app, Ok(detail)).await;
        app
    }

    #[tokio::test]
    async fn summary_follows_console_fields_and_order() {
        let app = opened(full()).await;
        let source = app.rds_state.detail.source.as_ref().unwrap();
        let view = app.rds_state.detail.view().unwrap();
        let fields = view::summary_fields(source, view.detail(), ReadStatus::Ready);
        assert_eq!(
            fields,
            [
                ("DB identifier", "orders".to_string()),
                ("CPU", "3.2%".into()),
                ("Status", "Configuring-enhanced-monitoring".into()),
                ("Class", "db.t4g.micro".into()),
                ("Role", "Instance".into()),
                ("Current activity", "0.01 AAS".into()),
                ("Engine", "PostgreSQL".into()),
                ("Region & AZ", "us-east-2a".into()),
                ("Recommendations", "1".into()),
            ]
        );
        let text = screen(&app);
        assert!(text.contains("Configuring-enhanced-monitoring") && text.contains("Region & AZ"));
    }

    #[test]
    fn engine_names_and_status_follow_the_console() {
        assert_eq!(view::engine_name("postgres"), "PostgreSQL");
        assert_eq!(view::engine_name("aurora-mysql"), "Aurora MySQL");
        assert_eq!(view::engine_name("custom-new"), "custom-new");
        assert_eq!(view::capitalize("available"), "Available");
        assert_eq!(view::capitalize(""), "");
    }

    #[tokio::test]
    async fn unavailable_summary_metrics_say_so() {
        let app = opened(RdsInstanceDetail {
            cpu: "Unavailable".into(),
            current_activity: "Unavailable".into(),
            recommendations: "Unavailable".into(),
            ..full()
        })
        .await;
        let source = app.rds_state.detail.source.as_ref().unwrap();
        let view = app.rds_state.detail.view().unwrap();
        let fields = view::summary_fields(source, view.detail(), ReadStatus::Ready);
        for label in ["CPU", "Current activity", "Recommendations"] {
            let value = &fields.iter().find(|(l, _)| *l == label).unwrap().1;
            assert_eq!(value, "Unavailable", "{label}");
        }
    }

    #[tokio::test]
    async fn monitoring_draws_console_charts_and_scrolls() {
        let mut app = opened(full()).await;
        goto(&mut app, Tab::Monitoring);
        let text = screen(&app);
        assert!(text.contains("CPUUtilization (Percent)"), "{text}");
        press(&mut app, KeyCode::Down);
        assert_eq!(app.rds_state.detail.view().unwrap().monitoring_scroll, 1);
        assert!(!screen(&app).contains("CPUUtilization (Percent)"));
        for _ in 0..40 {
            press(&mut app, KeyCode::Down);
        }
        assert_eq!(
            app.rds_state.detail.view().unwrap().monitoring_scroll,
            METRICS.len() - 1
        );
        press(&mut app, KeyCode::Up);
        assert_eq!(
            app.rds_state.detail.view().unwrap().monitoring_scroll,
            METRICS.len() - 2
        );
    }

    #[tokio::test]
    async fn logs_and_events_show_one_table_at_a_time_under_a_second_ribbon() {
        let mut app = opened(full()).await;
        goto(&mut app, Tab::LogsEvents);
        let text = screen(&app);
        assert!(
            text.contains("CloudWatch alarms ⋮ Recent events ⋮ Logs"),
            "ribbon"
        );
        assert!(text.contains("CloudWatch alarms (1)") && text.contains("orders-cpu-high"));
        assert!(!text.contains("Finished applying") && !text.contains("error/postgres.log"));
        press(&mut app, KeyCode::Char('f'));
        let text = screen(&app);
        assert!(text.contains("Recent events (1)") && text.contains("Finished applying"));
        assert!(!text.contains("orders-cpu-high"));
        press(&mut app, KeyCode::Char('f'));
        let text = screen(&app);
        assert!(text.contains("Logs (1)") && text.contains("error/postgres.log"));
        assert!(text.contains("979 B"));
    }

    #[tokio::test]
    async fn a_failed_section_fails_only_its_own_tab() {
        let mut detail = full();
        detail.sections.alarms = Err("CloudWatch alarms unavailable: AccessDenied".into());
        detail.sections.monitoring = Err("CloudWatch metrics unavailable: AccessDenied".into());
        detail.sections.integrations =
            Err("Zero-ETL integrations unavailable: AccessDenied".into());
        let mut app = opened(detail).await;
        goto(&mut app, Tab::Monitoring);
        let text = screen(&app);
        assert!(text.contains("CloudWatch metrics unavailable") && !text.contains("(Percent)"));
        goto(&mut app, Tab::LogsEvents);
        let text = screen(&app);
        assert!(text.contains("CloudWatch alarms unavailable"));
        press(&mut app, KeyCode::Char('F'));
        assert!(
            screen(&app).contains("error/postgres.log"),
            "other tables stay"
        );
        goto(&mut app, Tab::ZeroEtl);
        let text = screen(&app);
        assert!(text.contains("Zero-ETL integrations unavailable"));
        assert!(
            !text.contains("No zero-ETL integrations"),
            "never claims empty"
        );
        goto(&mut app, Tab::Tags);
        assert!(screen(&app).contains("team"));
    }

    #[tokio::test]
    async fn zero_etl_and_recommendations_are_filterable_tables() {
        let mut app = opened(full()).await;
        goto(&mut app, Tab::ZeroEtl);
        let text = screen(&app);
        assert!(text.contains("Zero-ETL integrations (1)") && text.contains("orders-to-redshift"));
        assert!(text.contains("Integration name") && text.contains("Active"));
        assert!(text.contains("Destination AWS account") && text.contains("210987654321"));
        assert!(text.contains("Amazon Redshift"));
        goto(&mut app, Tab::Recommendations);
        let text = screen(&app);
        assert!(text.contains("Recommendations (2)") && text.contains("Severity"));
        press(&mut app, KeyCode::Char('i'));
        for c in "resolved".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        press(&mut app, KeyCode::Enter);
        let text = screen(&app);
        // Ten columns share the width; cells may be shortened.
        assert!(text.contains("Recommendations (1)") && text.contains("Resolved"));
        assert!(!text.contains("Active"));
        goto(&mut app, Tab::Tags);
        assert!(app
            .rds_state
            .detail
            .view()
            .unwrap()
            .active_table()
            .rows
            .filter
            .is_empty());
    }

    #[tokio::test]
    async fn empty_sections_say_so_only_after_success() {
        let mut app = app();
        press(&mut app, KeyCode::Enter);
        goto(&mut app, Tab::ZeroEtl);
        assert!(!screen(&app).contains("No zero-ETL"));
        finish(&mut app, Ok(detail())).await;
        assert!(screen(&app).contains("No zero-ETL integrations use this database"));
        goto(&mut app, Tab::LogsEvents);
        press(&mut app, KeyCode::Char('f'));
        assert!(screen(&app).contains("No events in the last 7 days."));
    }

    #[tokio::test]
    async fn data_migrations_are_a_filterable_table() {
        let mut app = opened(full()).await;
        goto(&mut app, Tab::DataMigrations);
        let text = screen(&app);
        assert!(text.contains("Data migrations (1)") && text.contains("orders-copy"));
        // Sixteen columns share the width; headers may be shortened.
        assert!(text.contains("Migration prog") && text.contains("45%"));
        let cells = view::migration_cells(&full().sections.migrations.unwrap()[0]);
        assert_eq!(cells.len(), Tab::MIGRATION_COLUMNS.len());
        assert_eq!(cells[1], "Running");
        assert_eq!(cells[3], "Full load and CDC");
        assert_eq!(cells[6], "Enabled");
        assert_eq!(cells[14], "1h 2m 5s");
        press(&mut app, KeyCode::Char('i'));
        for c in "nomatch".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        press(&mut app, KeyCode::Enter);
        assert!(!screen(&app).contains("orders-copy"));
    }

    #[tokio::test]
    async fn failed_data_migrations_never_claim_empty() {
        let mut detail = full();
        detail.sections.migrations = Err("Data migrations unavailable: AccessDenied".into());
        let mut app = opened(detail).await;
        goto(&mut app, Tab::DataMigrations);
        let text = screen(&app);
        assert!(text.contains("Data migrations unavailable"));
        assert!(!text.contains("No homogeneous data migrations"));
    }

    #[test]
    fn elapsed_time_is_readable() {
        assert_eq!(view::elapsed(0), "0s");
        assert_eq!(view::elapsed(61_000), "1m 1s");
        assert_eq!(view::elapsed(3_725_000), "1h 2m 5s");
    }

    #[tokio::test]
    async fn configuration_has_four_console_panes() {
        let d = RdsInstanceDetail {
            identifier: "rusticity-test-pg".into(),
            engine_version: "18.3".into(),
            engine_lifecycle_support: "open-source-rds-extended-support".into(),
            license_model: "postgresql-license".into(),
            option_groups: vec![("default:postgres-18".into(), "in-sync".into())],
            parameter_groups: vec![("default.postgres18".into(), "in-sync".into())],
            deletion_protection: Some(false),
            vcpu: "2".into(),
            memory: "1 GB".into(),
            master_username: "rusticity".into(),
            iam_auth: Some(false),
            multi_az: Some(false),
            master_secret_arn: "arn:aws:secretsmanager:us-east-1:1:secret:rds!db".into(),
            master_secret_status: "active".into(),
            master_secret_kms_key: "aws/secretsmanager".into(),
            encrypted: Some(false),
            storage_type: "gp3".into(),
            storage_gib: Some(20),
            iops: Some(3000),
            storage_throughput: Some(125),
            storage_upgrade_available: Some(false),
            database_insights_mode: "standard".into(),
            performance_insights: Some(false),
            monitoring_interval: Some(0),
            ..Default::default()
        };
        let panes = view::configuration_panes(&d);
        let titles: Vec<_> = panes.iter().map(|(t, _)| *t).collect();
        assert_eq!(
            titles,
            [
                "Instance",
                "Instance class",
                "Primary storage",
                "Monitoring"
            ]
        );
        let labels = |i: usize| -> Vec<&str> { panes[i].1.iter().map(|(l, _)| *l).collect() };
        assert_eq!(
            labels(0),
            [
                "DB instance ID",
                "Engine version",
                "RDS Extended Support",
                "DB name",
                "License model",
                "Option groups",
                "Amazon Resource Name (ARN)",
                "Resource ID",
                "Created time",
                "DB instance parameter group",
                "Deletion protection",
                "Architecture settings",
                "Capacity reservation",
            ]
        );
        assert_eq!(
            labels(1),
            [
                "Instance class",
                "vCPU",
                "RAM",
                "Master username",
                "IAM DB authentication",
                "Multi-AZ",
                "Secondary Zone",
                "Master credentials ARN",
                "Master credentials KMS key",
            ]
        );
        assert_eq!(
            labels(2),
            [
                "Encryption",
                "Storage type",
                "Storage",
                "Provisioned IOPS",
                "Storage throughput",
                "Storage autoscaling",
                "Storage file system configuration",
            ]
        );
        assert_eq!(
            labels(3),
            [
                "Monitoring type",
                "Database counter metrics",
                "Enhanced Monitoring"
            ]
        );
        let get = |label: &str| {
            panes
                .iter()
                .flat_map(|(_, f)| f)
                .find(|(l, _)| *l == label)
                .unwrap()
                .1
                .clone()
        };
        assert_eq!(get("RDS Extended Support"), "Enabled");
        assert_eq!(get("License model"), "Postgresql License");
        assert_eq!(get("Option groups"), "default:postgres-18 (In sync)");
        assert_eq!(get("Architecture settings"), "Non-multitenant architecture");
        assert_eq!(get("IAM DB authentication"), "Not enabled");
        assert_eq!(
            get("Master credentials ARN"),
            "arn:aws:secretsmanager:us-east-1:1:secret:rds!db (Active)"
        );
        assert_eq!(get("Encryption"), "Not enabled");
        assert_eq!(get("Storage type"), "General Purpose SSD (gp3)");
        assert_eq!(get("Provisioned IOPS"), "3000 IOPS");
        assert_eq!(get("Storage file system configuration"), "Current");
        assert_eq!(get("Monitoring type"), "Database Insights - Standard");
        assert_eq!(get("Enhanced Monitoring"), "Disabled");
        let encrypted = RdsInstanceDetail {
            encrypted: Some(true),
            kms_key_id: "key-1".into(),
            ..d.clone()
        };
        let storage = &view::configuration_panes(&encrypted)[2].1;
        assert_eq!(storage[1], ("AWS KMS key", "key-1".to_string()));

        let mut app = opened(RdsInstanceDetail { ..d }).await;
        goto(&mut app, Tab::Configuration);
        let text = screen(&app);
        for title in [
            "Instance ",
            "Instance class",
            "Primary storage",
            "Monitoring",
        ] {
            assert!(text.contains(&format!("─ {title}")), "{title}");
        }
    }

    #[test]
    fn enabled_and_disabled_values_are_colored() {
        use ratatui::style::Color;
        assert_eq!(view::state_color("Enabled"), Some(Color::Green));
        assert_eq!(view::state_color("Enabled (7 days)"), Some(Color::Green));
        assert_eq!(view::state_color("Disabled"), Some(Color::Yellow));
        assert_eq!(view::state_color("Not enabled"), Some(Color::Yellow));
        assert_eq!(view::state_color("db.t4g.micro"), None);
    }

    #[tokio::test]
    async fn maintenance_states_render_in_color() {
        let mut app = opened(RdsInstanceDetail {
            auto_minor_version_upgrade: Some(true),
            backup_retention_days: Some(0),
            ..full()
        })
        .await;
        goto(&mut app, Tab::Maintenance);
        let mut terminal = Terminal::new(TestBackend::new(200, 45)).unwrap();
        terminal.draw(|f| crate::ui::render(f, &app)).unwrap();
        let buffer = terminal.backend().buffer();
        let colors = |word: &str| -> Vec<ratatui::style::Color> {
            let width = buffer.area.width as usize;
            let text: Vec<&str> = buffer.content().iter().map(|c| c.symbol()).collect();
            let joined: String = text.concat();
            let mut found = Vec::new();
            let mut from = 0;
            while let Some(i) = joined[from..].find(word) {
                let index = joined[..from + i].chars().count();
                found.push(buffer.content()[index].fg);
                from += i + word.len();
                let _ = width;
            }
            found
        };
        assert!(colors("Enabled").contains(&ratatui::style::Color::Green));
        assert!(colors("Disabled").contains(&ratatui::style::Color::Yellow));
    }

    fn two_events() -> RdsInstanceDetail {
        let mut d = full();
        d.sections.events = Ok(vec![
            RdsEvent {
                time: "2026-01-02T00:00:00Z".into(),
                message: "DB instance created".into(),
            },
            RdsEvent {
                time: "2026-01-01T00:00:00Z".into(),
                message: "DB instance restarted".into(),
            },
        ]);
        d
    }

    fn focus(app: &App) -> Slot {
        app.rds_state.detail.view().unwrap().slot().unwrap()
    }

    #[test]
    fn f_and_shift_f_map_to_table_focus_only_in_normal_mode() {
        let key = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
        assert_eq!(
            handle_key(key('f'), Mode::Normal),
            Some(Action::NextTableFocus)
        );
        assert_eq!(
            handle_key(
                KeyEvent::new(KeyCode::Char('F'), KeyModifiers::SHIFT),
                Mode::Normal
            ),
            Some(Action::PrevTableFocus)
        );
        assert_eq!(
            handle_key(key('f'), Mode::FilterInput),
            Some(Action::FilterInput('f'))
        );
    }

    #[tokio::test]
    async fn f_cycles_the_logs_and_events_tables_and_wraps() {
        let mut app = opened(two_events()).await;
        goto(&mut app, Tab::LogsEvents);
        assert_eq!(focus(&app), Slot::Alarms);
        press(&mut app, KeyCode::Char('f'));
        assert_eq!(focus(&app), Slot::Events);
        press(&mut app, KeyCode::Char('f'));
        assert_eq!(focus(&app), Slot::Logs);
        press(&mut app, KeyCode::Char('f'));
        assert_eq!(focus(&app), Slot::Alarms, "wraps");
        app.handle_action(Action::PrevTableFocus);
        assert_eq!(focus(&app), Slot::Logs, "F goes back and wraps");
        assert_eq!(app.rds_state.detail.view().unwrap().tab, Tab::LogsEvents);
    }

    #[tokio::test]
    async fn arrows_move_only_inside_the_focused_table_and_selection_is_kept() {
        let mut app = opened(two_events()).await;
        goto(&mut app, Tab::LogsEvents);
        press(&mut app, KeyCode::Char('f'));
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Down);
        let view = app.rds_state.detail.view().unwrap();
        assert_eq!(
            view.table(Slot::Events).rows.selected,
            1,
            "stops at the last row"
        );
        assert_eq!(view.table(Slot::Alarms).rows.selected, 0);
        assert_eq!(
            focus(&app),
            Slot::Events,
            "never spills into the next table"
        );
        press(&mut app, KeyCode::Char('f'));
        press(&mut app, KeyCode::Char('F'));
        let view = app.rds_state.detail.view().unwrap();
        assert_eq!(
            view.table(Slot::Events).rows.selected,
            1,
            "kept across focus moves"
        );
    }

    #[tokio::test]
    async fn filter_and_preferences_follow_the_focused_table() {
        use crate::preferences::bindings::model_for;
        let mut app = opened(two_events()).await;
        goto(&mut app, Tab::LogsEvents);
        press(&mut app, KeyCode::Char('f'));
        assert_eq!(model_for(&app, Service::RdsInstances).column_count(), 2);
        press(&mut app, KeyCode::Char('i'));
        for c in "restarted".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        press(&mut app, KeyCode::Enter);
        let text = screen(&app);
        assert!(text.contains("Recent events (1)") && !text.contains("DB instance created"));
        assert!(
            text.contains("restarted"),
            "the filter bar shows this table filter"
        );
        press(&mut app, KeyCode::Char('f'));
        let text = screen(&app);
        assert!(
            text.contains("Logs (1)") && text.contains("Filter logs"),
            "own filter"
        );
        assert!(!text.contains("restarted"));
        press(&mut app, KeyCode::Char('F'));
        assert!(screen(&app).contains("Recent events (1)"), "filter kept");
        let view = app.rds_state.detail.view().unwrap();
        assert!(view.table(Slot::Alarms).rows.filter.is_empty());
        assert!(view.table(Slot::Logs).rows.filter.is_empty());
    }

    #[tokio::test]
    async fn the_ribbon_marks_the_shown_table_and_the_status_bar_offers_f() {
        let mut app = opened(two_events()).await;
        goto(&mut app, Tab::LogsEvents);
        press(&mut app, KeyCode::Char('f'));
        let mut terminal = Terminal::new(TestBackend::new(200, 45)).unwrap();
        terminal.draw(|f| crate::ui::render(f, &app)).unwrap();
        let buffer = terminal.backend().buffer();
        let width = buffer.area.width as usize;
        let lines: Vec<String> = buffer
            .content()
            .chunks(width)
            .map(|row| row.iter().map(|c| c.symbol()).collect())
            .collect();
        let y = lines
            .iter()
            .position(|l| l.starts_with("CloudWatch alarms ⋮ Recent events"))
            .unwrap();
        let style = |word: &str| {
            let x = lines[y].find(word).unwrap();
            let x = lines[y][..x].chars().count();
            buffer.content()[y * width + x].style()
        };
        // Same highlight as the main tab ribbon.
        let selected = ratatui::style::Style::default().patch(crate::ui::tab_style(true));
        let shown = style("Recent events");
        assert_eq!(
            (shown.fg, shown.add_modifier),
            (selected.fg.or(shown.fg), selected.add_modifier)
        );
        assert_ne!(style("CloudWatch alarms"), shown);
        assert!(screen(&app).contains("next / previous table"));
        goto(&mut app, Tab::Tags);
        assert!(!screen(&app).contains("next / previous table"));
    }

    #[tokio::test]
    async fn refresh_clears_every_logs_and_events_table() {
        let mut app = opened(two_events()).await;
        goto(&mut app, Tab::LogsEvents);
        ctrl(&mut app, 'r');
        let view = app.rds_state.detail.view().unwrap();
        for slot in LOGS_EVENTS {
            assert!(view.table(slot).rows.items.is_empty(), "{slot:?}");
        }
        assert!(!screen(&app).contains("DB instance created"));
    }

    #[tokio::test]
    async fn connectivity_shows_its_pane_and_four_tables_under_a_ribbon() {
        let mut app = opened(full()).await;
        let text = screen(&app);
        assert!(
            text.contains("orders.example") && text.contains("Publicly accessible"),
            "pane"
        );
        assert!(text.contains(
            "Connected compute resources ⋮ Security group rules ⋮ Replication ⋮ Manage IAM roles"
        ));
        assert!(text.contains("Connected compute resources (1)") && text.contains("i-0abc"));
        assert!(text.contains("Compute resource security group") && text.contains("ec2-rds-1"));
        press(&mut app, KeyCode::Char('f'));
        let text = screen(&app);
        assert!(text.contains("Security group rules (2)") && text.contains("CIDR/IP - Outbound"));
        assert!(text.contains("EC2 Security Group - Inbound") && !text.contains("i-0abc"));
        press(&mut app, KeyCode::Char('f'));
        let text = screen(&app);
        assert!(text.contains("Replication (1)") && text.contains("Replication source"));
        press(&mut app, KeyCode::Char('f'));
        let text = screen(&app);
        assert!(text.contains("Manage IAM roles (1)") && text.contains("rds-s3-import"));
        assert!(text.contains("s3Import") && text.contains("Active"));
        press(&mut app, KeyCode::Char('f'));
        assert_eq!(focus(&app), Slot::Compute, "wraps");
        assert!(screen(&app).contains("next / previous table"));
    }

    #[tokio::test]
    async fn connectivity_and_logs_keep_their_own_ribbon_position() {
        let mut app = opened(full()).await;
        press(&mut app, KeyCode::Char('f'));
        goto(&mut app, Tab::LogsEvents);
        assert_eq!(focus(&app), Slot::Alarms);
        press(&mut app, KeyCode::Char('F'));
        assert_eq!(focus(&app), Slot::Logs);
        goto(&mut app, Tab::Connectivity);
        assert_eq!(focus(&app), Slot::SecurityGroupRules);
    }

    #[tokio::test]
    async fn connectivity_tables_wait_for_the_read_and_fail_alone() {
        let mut app = app();
        press(&mut app, KeyCode::Enter);
        let text = screen(&app);
        assert!(text.contains("Loading…") && !text.contains("Connected compute resources"));
        let mut d = full();
        d.sections.compute = Err("Connected compute resources unavailable: AccessDenied".into());
        finish(&mut app, Ok(d)).await;
        let text = screen(&app);
        assert!(text.contains("Connected compute resources unavailable"));
        assert!(!text.contains("No compute resources"), "never claims empty");
        assert!(text.contains("orders.example"), "pane still shows");
        press(&mut app, KeyCode::Char('f'));
        assert!(screen(&app).contains("CIDR/IP - Outbound"));
    }
}
