use super::*;
use crate::app::{App, Service, ViewMode};
use crate::common::{InputFocus, PageSize};
use crate::keymap::{Action, Mode};
use crate::ui::rds::filtered_instances;
use instance::Column;
use rusticity_core::rds::RdsInstance;

fn app() -> App {
    let mut app = App::new_without_client("test".into(), Some("us-east-2".into()));
    app.service_selected = true;
    app.current_service = Service::RdsInstances;
    app.mode = Mode::Normal;
    app.rds_state.instances.items = (0..25)
        .rev()
        .map(|i| RdsInstance {
            identifier: format!("database-{i:02}"),
            arn: format!("arn:aws:rds:us-east-2:123456789012:db:database-{i:02}"),
            status: "available".into(),
            engine: "postgres".into(),
            engine_version: "17.6".into(),
            instance_class: "db.t4g.micro".into(),
            availability_zone: "us-east-2a".into(),
            region: "us-east-2".into(),
            endpoint: "orders.example.invalid".into(),
            port: Some(5432),
            ..Default::default()
        })
        .collect();
    app
}

#[test]
fn rds_service_registration_and_picker_selection() {
    let mut app = App::new_without_client("test".into(), None);
    assert_eq!(Service::RdsInstances.name(), "RDS › Databases");
    assert!(app.service_picker.services.contains(&"RDS › Databases"));
    app.mode = Mode::ServicePicker;
    app.service_picker.filter = "rds".into();
    app.service_picker.filter_active = false;
    app.handle_action(Action::Select);
    assert_eq!(app.current_service, Service::RdsInstances);
    assert_eq!(app.mode, Mode::Normal);
    assert!(app.service_selected);
    assert!(app.rds_state.instances.loading);
    assert_eq!(app.tabs.last().unwrap().title, "RDS › Databases");
    assert!(app.breadcrumbs().contains("RDS"));
    assert_eq!(app.rds_state.column_ids, Column::ids());
    assert_eq!(
        app.rds_state.visible_column_ids,
        Column::default_visible_ids()
    );
}

#[test]
fn rds_column_ids_optional_values_and_status_styles() {
    use ratatui::style::Color;
    assert_eq!(Column::ALL.len(), 36);
    let mut i18n = HashMap::new();
    init(&mut i18n);
    for column in Column::ALL {
        assert!(column.id().starts_with("column.rds.instance."));
        assert_eq!(Column::from_id(column.id()), Some(column));
        assert_eq!(i18n[column.id()], column.default_name());
        assert_eq!(column.render(&RdsInstance::default()).0, "-");
    }
    assert_eq!(Column::from_id("unknown"), None);
    assert_eq!(instance::status_color("available"), Color::Green);
    assert_eq!(instance::status_color("creating"), Color::Yellow);
    assert_eq!(instance::status_color("failed"), Color::Red);
    assert_eq!(instance::status_color("new-service-status"), Color::White);
    let instance = RdsInstance {
        status: "new-service-status".into(),
        ..Default::default()
    };
    assert_eq!(Column::Status.render(&instance).0, "new-service-status");
}

#[test]
fn rds_filters_and_navigates_without_legacy_cloudwatch_fallthrough() {
    for view in [
        ViewMode::List,
        ViewMode::Detail,
        ViewMode::Events,
        ViewMode::InsightsResults,
    ] {
        let mut app = app();
        app.view_mode = view;
        app.rds_state.instances.page_size = PageSize::Ten;
        app.handle_action(Action::NextItem);
        assert_eq!(app.rds_state.instances.selected, 1);
        assert_eq!(
            actions::selected_arn(&app).unwrap(),
            "arn:aws:rds:us-east-2:123456789012:db:database-01"
        );
        app.handle_action(Action::Select);
        assert_eq!(app.rds_state.instances.expanded_item, Some(1));
        app.handle_action(Action::PageDown);
        assert_eq!(app.rds_state.instances.selected, 11);
        assert_eq!(app.rds_state.instances.expanded_item, None);
        app.handle_action(Action::StartFilter);
        for c in "POSTGRES".chars() {
            app.handle_action(Action::FilterInput(c));
        }
        assert_eq!(filtered_instances(&app).len(), 25);
        app.handle_action(Action::NextFilterFocus);
        assert_eq!(app.rds_state.input_focus, InputFocus::Pagination);
        app.handle_action(Action::PageDown);
        assert_eq!(app.rds_state.instances.selected, 10);
        app.handle_action(Action::ApplyFilter);
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.log_groups_state.log_groups.selected, 0);
        assert!(app.log_groups_state.log_groups.filter.is_empty());
        assert!(!app.log_groups_state.loading);
    }
}

#[test]
fn rds_refresh_retry_and_out_of_range_navigation_are_safe() {
    let mut app = app();
    actions::go_to_page(&mut app, usize::MAX);
    assert!(app.rds_state.instances.selected < 25);
    app.handle_action(Action::Refresh);
    assert!(app.rds_state.instances.loading);
    assert!(!app.log_groups_state.loading);
    app.rds_state.instances.loading = false;
    app.mode = Mode::ErrorModal;
    app.error_message = Some("AccessDenied".into());
    app.handle_action(Action::RetryLoad);
    assert!(app.error_message.is_none());
    assert_eq!(app.mode, Mode::Normal);
    assert!(app.rds_state.instances.loading);
    assert_eq!(app.rds_state.instances.items.len(), 25);
    assert!(!app.log_groups_state.loading);
}

#[test]
fn rds_inventory_urls_ignore_selection_and_detail_builder_encodes_identifier() {
    let app = app();
    assert!(actions::console_url(&app).ends_with("#databases:"));
    assert!(console_url_instance("eu-west-1", "db name").contains("id=db%20name;"));
    assert_eq!(
        console_url_instances("us-east-2"),
        "https://us-east-2.console.aws.amazon.com/rds/home?region=us-east-2#databases:"
    );
}

fn screen(app: &App, width: u16, height: u16) -> String {
    use ratatui::{backend::TestBackend, Terminal};
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

#[test]
fn rds_renders_page_selection_expansion_and_empty_or_loading_views() {
    let mut app = app();
    app.rds_state.instances.page_size = PageSize::Ten;
    app.rds_state.instances.selected = 12;
    app.rds_state.instances.expanded_item = Some(11);
    let text = screen(&app, 240, 45);
    assert!(text.contains("Databases (25)"));
    assert!(text.contains("1 [2] 3"));
    assert!(text.contains("► database-12"));
    assert!(text.contains("▼ database-11"));
    assert!(text.contains("database-19"));
    assert!(!text.contains("database-09"));
    assert!(!text.contains("database-20"));
    assert!(text.contains("17.6"));
    assert!(text.contains("orders.example.invalid"));
    assert!(text.contains("y copy ARN"));

    app.rds_state.instances.filter = "no-such-instance".into();
    assert!(screen(&app, 240, 45).contains("Databases (0)"));
    app.rds_state.instances.items.clear();
    app.rds_state.instances.loading = true;
    assert!(screen(&app, 240, 45).contains("Loading databases..."));
    for (width, height) in [(20, 4), (80, 15), (120, 25)] {
        screen(&app, width, height);
    }
}

#[test]
fn rds_shared_preferences_toggle_last_column_guard_and_page_size_reset() {
    use crate::preferences::{bindings, PreferenceRow};
    let mut app = app();
    app.view_mode = ViewMode::Events;
    app.handle_action(Action::OpenColumnSelector);
    assert_eq!(app.mode, Mode::ColumnSelector);
    let model = bindings::active(&app).unwrap();
    assert_eq!(model.column_count(), 36);
    assert_eq!(
        model
            .columns()
            .map(|c| c.label.as_str())
            .collect::<Vec<_>>(),
        Column::ALL
            .iter()
            .map(|c| c.default_name())
            .collect::<Vec<_>>()
    );
    let text = screen(&app, 240, 45);
    assert!(text.contains("Preferences"));
    assert!(text.contains("Engine version"));
    app.column_selector_index = 1;
    app.handle_action(Action::ToggleColumn);
    assert!(!app
        .rds_state
        .visible_column_ids
        .contains(&Column::Identifier.id()));
    app.handle_action(Action::ToggleColumn);
    assert!(app
        .rds_state
        .visible_column_ids
        .contains(&Column::Identifier.id()));
    app.rds_state.visible_column_ids = vec![Column::Identifier.id()];
    app.handle_action(Action::ToggleColumn);
    assert_eq!(app.rds_state.visible_column_ids, [Column::Identifier.id()]);

    app.rds_state.instances.selected = 21;
    app.rds_state.instances.expanded_item = Some(21);
    app.rds_state.instances.scroll_offset = 10;
    app.column_selector_index = model
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
    app.handle_action(Action::ToggleColumn);
    assert_eq!(app.rds_state.instances.page_size, PageSize::Ten);
    assert_eq!(app.rds_state.instances.selected, 0);
    assert_eq!(app.rds_state.instances.scroll_offset, 0);
    assert_eq!(app.rds_state.instances.expanded_item, None);
    assert!(!app.rds_state.instances.loading);
}

#[test]
fn rds_restores_from_both_session_entrypoints() {
    use crate::session::{Session, SessionTab};
    for load_action in [Action::Select, Action::LoadSession] {
        let mut app = app();
        let mut session = Session::new(
            "test".into(),
            "us-east-2".into(),
            "123456789012".into(),
            "test-role".into(),
        );
        session.tabs.push(SessionTab {
            service: "RdsInstances".into(),
            title: "RDS › DB Instances".into(),
            breadcrumb: "RDS › DB Instances".into(),
            filter: None,
            selected_item: None,
        });
        app.sessions = vec![session];
        app.session_picker_selected = 0;
        app.mode = Mode::SessionPicker;
        app.current_service = Service::CloudWatchLogGroups;
        app.handle_action(load_action);
        assert_eq!(app.current_service, Service::RdsInstances);
        assert!(app.service_selected);
        assert_eq!(app.tabs[0].service, Service::RdsInstances);
        assert_eq!(app.tabs[0].title, "RDS › Databases");
        assert_eq!(app.tabs[0].breadcrumb, "RDS › Databases");
        assert!(screen(&app, 240, 45).contains("Databases"));
    }
}

#[test]
fn rds_numeric_page_input_does_not_open_preferences_or_touch_other_services() {
    let mut app = app();
    app.rds_state.instances.page_size = PageSize::Ten;
    app.handle_action(Action::FilterInput('3'));
    app.handle_action(Action::OpenColumnSelector);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.rds_state.instances.selected, 20);
    assert!(app.page_input.is_empty());
    assert_eq!(app.log_groups_state.log_groups.selected, 0);
}

#[test]
fn rds_case_insensitive_filter_covers_inventory_fields_and_yank_selection() {
    let mut app = app();
    for filter in [
        "DATABASE-",
        "POSTGRES",
        "AVAILABLE",
        "T4G",
        "US-EAST-2A",
        "EXAMPLE.INVALID",
    ] {
        app.rds_state.instances.filter = filter.into();
        assert_eq!(filtered_instances(&app).len(), 25);
    }
    app.rds_state.instances.filter = "database-24".into();
    app.rds_state.instances.selected = 0;
    assert!(actions::selected_arn(&app)
        .unwrap()
        .ends_with("database-24"));
    app.rds_state.instances.filter = "nothing".into();
    assert_eq!(actions::selected_arn(&app), None);
    assert!(actions::console_url(&app).ends_with("#databases:"));
}

#[test]
fn rds_print_hint_and_ctrl_p_use_the_shared_snapshot_action() {
    use crate::keymap::handle_key;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    for view in [ViewMode::List, ViewMode::Detail, ViewMode::Events] {
        for populated in [false, true] {
            let mut app = app();
            app.view_mode = view;
            if !populated {
                app.rds_state.instances.items.clear();
            }
            let text = screen(&app, 240, 45);
            assert!(
                text.contains("^p print"),
                "Missing RDS print hint for {view:?}"
            );
            assert!(text.contains("p preferences"));
            assert!(text.contains("y copy ARN"));
            let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
            let action = handle_key(key, app.mode).unwrap();
            assert_eq!(action, Action::CopyToClipboard);
            assert!(!app.clipboard.snapshot_requested());
            app.handle_action(action);
            assert!(app.clipboard.snapshot_requested());
            assert_eq!(app.mode, Mode::Normal);
            assert_eq!(app.current_service, Service::RdsInstances);
            assert!(!app.rds_state.instances.loading);
        }
    }
}

#[test]
fn rds_plain_p_still_opens_preferences_instead_of_printing() {
    use crate::keymap::handle_key;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut app = app();
    let action = handle_key(
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
        app.mode,
    )
    .unwrap();
    assert_eq!(action, Action::OpenColumnSelector);
    app.handle_action(action);
    assert_eq!(app.mode, Mode::ColumnSelector);
    assert!(!app.clipboard.snapshot_requested());
}

#[test]
fn rds_preferences_match_requested_database_column_order() {
    let expected = [
        "DB identifier",
        "Status",
        "DB cluster identifier",
        "Role",
        "Engine",
        "Engine version",
        "Upgrade rollout order",
        "RDS Extended Support",
        "Region & AZ",
        "Size",
        "Recommendations",
        "CPU",
        "Current activity",
        "Maintenance",
        "VPC",
        "Multi-AZ",
        "Storage type",
        "Storage",
        "Provisioned IOPS",
        "Storage throughput",
        "Security groups",
        "DB subnet group name",
        "DB Certificate Expiry",
        "Pending changes",
        "Character set",
        "Option group",
        "Created time",
        "Encrypted",
        "Priority tier",
        "Replica lag",
        "Replica State",
        "Parameter group",
        "Clone group ID",
    ];
    let names: Vec<_> = Column::ALL.iter().map(|c| c.default_name()).collect();
    assert_eq!(&names[..expected.len()], &expected);
    assert_eq!(&names[expected.len()..], &["Endpoint", "Port", "ARN"]);
    assert_eq!(
        Column::from_id("column.rds.instance.class"),
        Some(Column::Class)
    );
    assert_eq!(Column::Class.default_name(), "Size");
    assert_eq!(
        Column::from_id("column.rds.instance.availability_zone"),
        Some(Column::RegionAz)
    );
}

#[test]
fn rds_available_is_green_tick_in_table_and_expanded_row() {
    use ratatui::{backend::TestBackend, style::Color, Terminal};
    for raw in ["available", "Available", "AVAILABLE"] {
        let instance = RdsInstance {
            status: raw.into(),
            ..Default::default()
        };
        let (text, style) = Column::Status.render(&instance);
        assert_eq!(text, "✅ Available");
        assert_eq!(style.fg, Some(Color::Green));
    }
    let mut app = app();
    app.rds_state.instances.selected = 0;
    app.rds_state.instances.expanded_item = Some(0);
    let mut terminal = Terminal::new(TestBackend::new(240, 45)).unwrap();
    terminal.draw(|f| crate::ui::render(f, &app)).unwrap();
    let cells = terminal.backend().buffer().content();
    let ticks: Vec<_> = cells
        .iter()
        .filter(|cell| cell.symbol().contains('✅'))
        .collect();
    assert!(
        ticks.len() >= 2,
        "Main row and expanded details both use the status renderer"
    );
    assert!(ticks.iter().all(|cell| cell.fg == Color::Green));
    assert!(screen(&app, 240, 45).contains("Databases (25)"));
    assert!(!screen(&app, 240, 45).contains("DB instances"));
}

#[test]
fn rds_columns_format_units_dates_optional_bools_and_unknown_metrics() {
    let i = RdsInstance {
        region: "us-east-2".into(),
        availability_zone: "us-east-2a".into(),
        storage_gib: Some(20),
        storage_throughput: Some(125),
        iops: Some(3000),
        multi_az: Some(false),
        encrypted: Some(true),
        priority_tier: Some(0),
        certificate_expiry: "2027-01-02T03:04:05Z".into(),
        created_time: "2026-01-02T03:04:05Z".into(),
        cpu: "Unavailable".into(),
        ..Default::default()
    };
    for (column, expected) in [
        (Column::RegionAz, "us-east-2 / us-east-2a"),
        (Column::Storage, "20 GiB"),
        (Column::StorageThroughput, "125 MiB/s"),
        (Column::Iops, "3000"),
        (Column::MultiAz, "No"),
        (Column::Encrypted, "Yes"),
        (Column::PriorityTier, "0"),
        (Column::Cpu, "Unavailable"),
        (Column::CurrentActivity, "-"),
        (Column::CertificateExpiry, "2027-01-02 03:04:05 (UTC)"),
        (Column::CreatedTime, "2026-01-02 03:04:05 (UTC)"),
    ] {
        assert_eq!(column.render(&i).0, expected);
    }
}

#[test]
fn rds_optional_enrichment_only_loads_for_enabled_columns() {
    use crate::preferences::bindings;
    use rusticity_core::rds::InventoryOptions;
    assert_eq!(
        Column::inventory_options(&Column::default_visible_ids()),
        InventoryOptions::default()
    );
    let mut app = app();
    app.mode = Mode::ColumnSelector;
    for column in [
        Column::Cpu,
        Column::CurrentActivity,
        Column::Recommendations,
        Column::Maintenance,
        Column::CloneGroupId,
        Column::ReplicaLag,
    ] {
        app.rds_state.instances.loading = false;
        let model = bindings::active(&app).unwrap();
        app.column_selector_index = model
            .rows()
            .iter()
            .position(|row| {
                matches!(row,
            crate::preferences::PreferenceRow::Column(c) if c.id == column.id())
            })
            .unwrap();
        app.handle_action(Action::ToggleColumn);
        assert!(
            app.rds_state.instances.loading,
            "Enable {column:?} must schedule its read"
        );
        app.rds_state.instances.loading = false;
        app.handle_action(Action::ToggleColumn);
        assert!(
            !app.rds_state.instances.loading,
            "Hiding a column doesn't make extra API calls"
        );
    }
}
