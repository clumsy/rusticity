use super::*;
use crate::app::{App, Service, ViewMode};
use crate::common::{InputFocus, PageSize};
use crate::keymap::{handle_key, Action, Mode};
use crate::preferences::{bindings, PreferenceRow};
use crate::ui::vpc::filtered_vpcs;
use columns::Column;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, style::Color, Terminal};
use rusticity_core::vpc::Vpc;

fn app() -> App {
    let mut app = App::new_without_client("test".into(), Some("us-east-2".into()));
    app.current_service = Service::VpcVpcs;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.vpc_state.vpcs.items = (0..25)
        .rev()
        .map(|i| Vpc {
            vpc_id: format!("vpc-{i:03}"),
            name: format!("network-{i:03}"),
            state: "available".into(),
            ipv4_cidrs: format!("10.{i}.0.0/16"),
            ipv6_cidrs: "2001:db8::/56".into(),
            is_default: Some(false),
            tenancy: "default".into(),
            dhcp_options_id: "dopt-test".into(),
            owner_id: "123456789012".into(),
            region: "us-east-2".into(),
            arn: format!("arn:aws:ec2:us-east-2:123456789012:vpc/vpc-{i:03}"),
            ..Default::default()
        })
        .collect();
    app
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

#[test]
fn vpc_registration_and_service_selection() {
    let mut app = App::new_without_client("test".into(), None);
    assert_eq!(Service::VpcVpcs.name(), "VPC › VPCs");
    assert!(app.service_picker.services.contains(&"VPC › VPCs"));
    app.mode = Mode::ServicePicker;
    app.service_picker.filter = "vpc".into();
    app.service_picker.filter_active = false;
    app.handle_action(Action::Select);
    assert_eq!(app.current_service, Service::VpcVpcs);
    assert_eq!(app.mode, Mode::Normal);
    assert!(app.vpc_state.vpcs.loading);
    assert_eq!(app.tabs[0].title, "VPC › VPCs");
    assert!(app.breadcrumbs().contains("VPC"));
    assert_eq!(app.vpc_state.column_ids, Column::ids());
    assert_eq!(
        app.vpc_state.visible_column_ids,
        Column::default_visible_ids()
    );
}

#[test]
fn vpc_columns_i18n_and_unknown_values() {
    let mut i18n = HashMap::new();
    init(&mut i18n);
    assert_eq!(Column::ALL.len(), 13);
    for column in Column::ALL {
        assert!(column.id().starts_with("column.vpc.vpc."));
        assert_eq!(Column::from_id(column.id()), Some(column));
        assert_eq!(column.name(), column.default_name());
        assert_eq!(i18n[column.id()], column.default_name());
        let (value, style) = column.render(&Vpc::default());
        assert_eq!(value, "-");
        assert_eq!(style.fg, Some(Color::White));
        let expanded =
            crate::ui::table::expanded_from_columns(&[column.into_table_column()], &Vpc::default());
        assert_eq!(expanded[0].1.fg, Some(Color::White));
    }
    let vpc = Vpc {
        state: "AVAILABLE".into(),
        is_default: Some(false),
        ..Default::default()
    };
    assert_eq!(Column::State.render(&vpc).0, "✅ Available");
    assert_eq!(Column::State.render(&vpc).1.fg, Some(Color::Green));
    assert_eq!(Column::DefaultVpc.render(&vpc).0, "No");
    assert_eq!(Column::DefaultVpc.render(&vpc).1.fg, Some(Color::White));
    let positive = Vpc {
        is_default: Some(true),
        ..Default::default()
    };
    assert_eq!(
        Column::DefaultVpc.render(&positive).1.fg,
        Some(Color::Green)
    );
    let pending = Vpc {
        state: "pending".into(),
        ..Default::default()
    };
    assert_eq!(Column::State.render(&pending).1.fg, Some(Color::Yellow));
    assert_eq!(Column::from_id("unknown"), None);
}

#[test]
fn vpc_inventory_controls_preserve_filter_page_and_expansion_contracts() {
    for view_mode in [
        ViewMode::List,
        ViewMode::Detail,
        ViewMode::Events,
        ViewMode::InsightsResults,
    ] {
        let mut app = app();
        app.view_mode = view_mode;
        app.vpc_state.vpcs.page_size = PageSize::Ten;
        app.handle_action(Action::NextItem);
        assert_eq!(
            actions::selected_arn(&app).unwrap(),
            "arn:aws:ec2:us-east-2:123456789012:vpc/vpc-001"
        );
        app.handle_action(Action::Select);
        assert_eq!(app.vpc_state.vpcs.expanded_item, Some(1));
        app.handle_action(Action::PageDown);
        assert_eq!(app.vpc_state.vpcs.selected, 11);
        assert_eq!(app.vpc_state.vpcs.expanded_item, None);
        app.handle_action(Action::StartFilter);
        for c in "NETWORK".chars() {
            app.handle_action(Action::FilterInput(c));
        }
        assert_eq!(filtered_vpcs(&app).len(), 25);
        assert_eq!(app.vpc_state.vpcs.selected, 0);
        app.handle_action(Action::NextFilterFocus);
        assert_eq!(app.vpc_state.input_focus, InputFocus::Pagination);
        app.handle_action(Action::PageDown);
        assert_eq!(app.vpc_state.vpcs.selected, 10);
        app.handle_action(Action::ApplyFilter);
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.log_groups_state.log_groups.selected, 0);
        assert!(app.log_groups_state.log_groups.filter.is_empty());
    }
}

#[test]
fn vpc_metadata_filter_and_urls_use_the_same_selected_row() {
    let mut app = app();
    for query in [
        "NETWORK",
        "VPC-",
        "AVAILABLE",
        "10.",
        "2001:DB8",
        "123456789012",
        "DOPT-TEST",
    ] {
        app.vpc_state.vpcs.filter = query.into();
        assert_eq!(filtered_vpcs(&app).len(), 25);
    }
    app.vpc_state.vpcs.filter = "network-024".into();
    assert!(actions::console_url(&app).ends_with("#vpcs:"));
    assert!(actions::selected_arn(&app).unwrap().ends_with("vpc-024"));
    app.vpc_state.vpcs.filter = "not-found".into();
    assert!(actions::selected_arn(&app).is_none());
    assert!(actions::console_url(&app).ends_with("#vpcs:"));
    assert_eq!(
        console_url("eu-west-1"),
        "https://eu-west-1.console.aws.amazon.com/vpcconsole/home?region=eu-west-1#vpcs:"
    );
}

#[test]
fn vpc_preferences_and_print_use_common_components() {
    let mut app = app();
    app.view_mode = ViewMode::Events;
    let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
    app.handle_action(handle_key(key, app.mode).unwrap());
    assert!(app.snapshot_requested);
    assert!(screen(&app, 240, 45).contains("^p print"));
    app.handle_action(Action::OpenColumnSelector);
    let model = bindings::active(&app).unwrap();
    assert_eq!(model.column_count(), 13);
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
    // VPC ID is the console's default property and cannot be hidden.
    app.column_selector_index = 1;
    app.handle_action(Action::ToggleColumn);
    assert!(app
        .vpc_state
        .visible_column_ids
        .contains(&Column::VpcId.id()));
    app.column_selector_index = 2;
    app.handle_action(Action::ToggleColumn);
    assert!(!app
        .vpc_state
        .visible_column_ids
        .contains(&Column::State.id()));
    app.handle_action(Action::ToggleColumn);
    assert!(app
        .vpc_state
        .visible_column_ids
        .contains(&Column::State.id()));
    app.vpc_state.visible_column_ids = vec![Column::VpcId.id()];
    app.column_selector_index = 1;
    app.handle_action(Action::ToggleColumn);
    assert_eq!(app.vpc_state.visible_column_ids, [Column::VpcId.id()]);
    app.vpc_state.vpcs.selected = 20;
    app.vpc_state.vpcs.expanded_item = Some(20);
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
    assert_eq!(app.vpc_state.vpcs.page_size, PageSize::Ten);
    assert_eq!(app.vpc_state.vpcs.selected, 0);
    assert_eq!(app.vpc_state.vpcs.expanded_item, None);
    assert!(!app.vpc_state.vpcs.loading);
}

#[test]
fn vpc_refresh_retry_and_page_input_stay_scoped_to_vpc() {
    let mut app = app();
    app.vpc_state.vpcs.page_size = PageSize::Ten;
    app.handle_action(Action::FilterInput('3'));
    app.handle_action(Action::OpenColumnSelector);
    assert_eq!(app.vpc_state.vpcs.selected, 20);
    assert_eq!(app.mode, Mode::Normal);
    actions::go_to_page(&mut app, usize::MAX);
    assert!(app.vpc_state.vpcs.selected < 25);
    app.handle_action(Action::Refresh);
    assert!(app.vpc_state.vpcs.loading);
    assert!(!app.log_groups_state.loading);
    app.vpc_state.vpcs.loading = false;
    app.mode = Mode::ErrorModal;
    app.error_message = Some("AccessDenied".into());
    app.handle_action(Action::RetryLoad);
    assert!(app.error_message.is_none());
    assert_eq!(app.mode, Mode::Normal);
    assert!(app.vpc_state.vpcs.loading);
    assert_eq!(app.vpc_state.vpcs.items.len(), 25);
}

#[test]
fn vpc_rendering_has_green_tick_expansion_pagination_and_no_fake_optional_values() {
    let mut app = app();
    app.vpc_state.vpcs.page_size = PageSize::Ten;
    app.vpc_state.vpcs.selected = 12;
    app.vpc_state.vpcs.expanded_item = Some(11);
    let text = screen(&app, 240, 45);
    assert!(text.contains("VPCs (25)"));
    assert!(text.contains("1 [2] 3"));
    assert!(text.contains("► vpc-012"));
    assert!(text.contains("▼ vpc-011"));
    assert!(text.contains("vpc-019"));
    assert!(!text.contains("vpc-009"));
    assert!(!text.contains("vpc-020"));
    let mut terminal = Terminal::new(TestBackend::new(240, 45)).unwrap();
    terminal.draw(|f| crate::ui::render(f, &app)).unwrap();
    let ticks: Vec<_> = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .filter(|c| c.symbol().contains('✅'))
        .collect();
    assert!(ticks.len() >= 2);
    assert!(ticks.iter().all(|c| c.fg == Color::Green));
    app.vpc_state.vpcs.filter = "nothing".into();
    assert!(screen(&app, 240, 45).contains("VPCs (0)"));
    app.vpc_state.vpcs.items.clear();
    app.vpc_state.vpcs.loading = true;
    assert!(screen(&app, 240, 45).contains("Loading VPCs..."));
    for (w, h) in [(20, 4), (80, 15)] {
        screen(&app, w, h);
    }
}

#[test]
fn vpc_session_restoration_uses_both_entrypoints() {
    use crate::session::{Session, SessionTab};
    for action in [Action::Select, Action::LoadSession] {
        let mut app = app();
        let mut session = Session::new(
            "test".into(),
            "us-east-2".into(),
            "123456789012".into(),
            "test-role".into(),
        );
        session.tabs.push(SessionTab {
            service: "VpcVpcs".into(),
            title: "VPC › VPCs".into(),
            breadcrumb: "VPC › VPCs".into(),
            filter: None,
            selected_item: None,
        });
        app.sessions = vec![session];
        app.mode = Mode::SessionPicker;
        app.current_service = Service::CloudWatchLogGroups;
        app.handle_action(action);
        assert_eq!(app.current_service, Service::VpcVpcs);
        assert_eq!(app.tabs[0].service, Service::VpcVpcs);
        assert!(screen(&app, 240, 45).contains("VPCs (25)"));
    }
}

#[test]
fn vpc_finder_selection_routes_to_the_vpc_inventory() {
    use rusticity_core::finder::{Resource, ResourceKind};
    let mut app = app();
    app.handle_action(Action::OpenResourceFinder);
    app.resource_finder.rows = vec![Resource {
        kind: ResourceKind::Vpc,
        id: "vpc-024".into(),
        name: "network-024".into(),
        arn: "arn:aws:ec2:us-east-2:123456789012:vpc/vpc-024".into(),
        region: "us-east-2".into(),
    }];
    // Enter moves from filter to results, then opens the exact VPC target.
    app.handle_action(Action::Select);
    app.handle_action(Action::Select);
    let target = app.resource_finder.pending_open.as_ref().unwrap();
    assert_eq!(target.kind, ResourceKind::Vpc);
    assert_eq!(target.id, "vpc-024");
    assert_eq!(
        crate::finder::navigation::service(target.kind),
        Service::VpcVpcs
    );
}

#[test]
fn vpc_preferences_have_exactly_the_thirteen_console_properties() {
    let app = app();
    let expected = [
        "VPC ID",
        "State",
        "Encryption control ID",
        "Encryption control mode",
        "Block Public Access",
        "IPv4 CIDR",
        "IPv6 CIDR",
        "DHCP option set",
        "Main route table",
        "Main network ACL",
        "Tenancy",
        "Default VPC",
        "Owner ID",
    ];
    assert_eq!(Column::ALL.map(Column::default_name), expected);
    let model = bindings::active(&app).unwrap();
    assert_eq!(model.column_count(), 13);
    assert_eq!(
        model
            .columns()
            .map(|c| c.label.as_str())
            .collect::<Vec<_>>(),
        expected
    );
    assert!(model.columns().next().unwrap().locked);
    assert_eq!(Column::from_id("column.vpc.vpc.name"), None);
    assert_eq!(Column::from_id("column.vpc.vpc.region"), None);
    assert_eq!(Column::from_id("column.vpc.vpc.arn"), None);
}

#[test]
fn vpc_expansion_shows_all_properties_even_when_only_id_is_visible() {
    let mut app = app();
    app.vpc_state.vpcs.items.truncate(1);
    let vpc = &mut app.vpc_state.vpcs.items[0];
    vpc.encryption_control_id = "vpc-enc-test".into();
    vpc.encryption_control_mode = "monitor".into();
    vpc.block_public_access = "off".into();
    vpc.main_route_table = "rtb-main".into();
    vpc.main_network_acl = "acl-main".into();
    app.vpc_state.visible_column_ids = vec![Column::VpcId.id()];
    app.vpc_state.vpcs.expanded_item = Some(0);
    let text = screen(&app, 220, 40);
    for column in Column::ALL {
        assert!(
            text.contains(&format!("{}:", column.default_name())),
            "Missing expanded property {column:?}"
        );
    }
    for value in [
        "vpc-enc-test",
        "monitor",
        "Off",
        "rtb-main",
        "acl-main",
        "dopt-test",
    ] {
        assert!(text.contains(value), "Missing expanded value {value}");
    }
    assert!(!text.contains("Name:"));
    assert!(!text.contains("ARN:"));
    let hidden = Column::MainNetworkAcl.render(&Vpc {
        main_network_acl: "Unavailable".into(),
        ..Default::default()
    });
    assert_eq!(hidden.0, "Unavailable");
    assert_eq!(hidden.1.fg, Some(Color::Yellow));
}

#[test]
fn vpc_console_action_always_opens_the_regional_table_page() {
    let mut app = app();
    assert_eq!(
        actions::console_url(&app),
        "https://us-east-2.console.aws.amazon.com/vpcconsole/home?region=us-east-2#vpcs:"
    );
    app.vpc_state.vpcs.selected = 12;
    assert!(actions::console_url(&app).ends_with("#vpcs:"));
    app.vpc_state.vpcs.expanded_item = Some(12);
    assert!(actions::console_url(&app).ends_with("#vpcs:"));
    // Do not hardcode a session-specific account hostname from a copied browser URL.
    assert!(!actions::console_url(&app).contains("280156627303-"));
}

#[test]
fn vpc_ipv6_width_fits_contents_instead_of_stretching_on_wide_terminals() {
    use ratatui::text::Line;
    let mut app = app();
    app.vpc_state.vpcs.items.truncate(1);
    let full_cidr = "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff/128";
    for value in ["", "2001:db8::/56", full_cidr] {
        app.vpc_state.vpcs.items[0].ipv6_cidrs = value.into();
        let mut positions = Vec::new();
        for width in [180, 300, 400] {
            let mut terminal = Terminal::new(TestBackend::new(width, 20)).unwrap();
            terminal
                .draw(|f| crate::ui::vpc::render_vpcs(f, &app, f.area()))
                .unwrap();
            let buffer = terminal.backend().buffer();
            let header: String = (0..width).map(|x| buffer[(x, 4)].symbol()).collect();
            let position = |label: &str| Line::raw(&header[..header.find(label).unwrap()]).width();
            let ipv6 = position("IPv6 CIDR");
            let tenancy = position("Tenancy");
            let needed = value
                .chars()
                .count()
                .saturating_add(2)
                .max("⋮ IPv6 CIDR".chars().count());
            assert_eq!(
                tenancy - ipv6,
                needed + 1,
                "IPv6 must only consume content/header width + spacing"
            );
            assert!(header.contains("VPC ID ↑"));
            let row: String = (0..width).map(|x| buffer[(x, 5)].symbol()).collect();
            if !value.is_empty() {
                assert!(row.contains(value), "CIDR clipped at width {width}: {row}");
            }
            positions.push(tenancy);
        }
        assert!(
            positions.windows(2).all(|pair| pair[0] == pair[1]),
            "Extra terminal space must not grow a VPC column"
        );
    }
}

#[test]
fn vpc_compact_columns_handle_empty_tables_reordering_and_long_values() {
    use ratatui::text::Line;
    let mut app = app();
    app.vpc_state.visible_column_ids = vec![
        Column::Ipv6Cidrs.id(),
        Column::VpcId.id(),
        Column::Owner.id(),
    ];
    let long_cidrs = "2001:db8::/56, ".repeat(20);
    for empty in [false, true] {
        if empty {
            app.vpc_state.vpcs.items.clear();
        } else {
            app.vpc_state.vpcs.items[0].ipv6_cidrs = long_cidrs.clone();
        }
        let mut terminal = Terminal::new(TestBackend::new(400, 40)).unwrap();
        terminal
            .draw(|f| crate::ui::vpc::render_vpcs(f, &app, f.area()))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let header: String = (0..400).map(|x| buffer[(x, 4)].symbol()).collect();
        let ipv6 = Line::raw(&header[..header.find("IPv6 CIDR").unwrap()]).width();
        let id = Line::raw(&header[..header.find("VPC ID ↑").unwrap()]).width();
        assert_eq!(id - ipv6, if empty { 12 } else { 46 });
        assert!(header.contains("Owner ID"));
    }
}

#[test]
fn vpc_block_public_access_uses_semantic_icons_and_colors_in_rows_and_details() {
    use crate::ui::table::expanded_from_columns;
    for (raw, expected, color) in [
        ("off", "❌ Off", Color::White),
        ("OFF", "❌ Off", Color::White),
        ("block-ingress", "✅ Block ingress", Color::Green),
        (
            "block-bidirectional",
            "✅ Block bidirectional",
            Color::Green,
        ),
        ("future-mode", "future-mode", Color::White),
        ("", "-", Color::White),
    ] {
        let vpc = Vpc {
            block_public_access: raw.into(),
            ..Default::default()
        };
        let column = Column::BlockPublicAccess.into_table_column();
        let (value, style) = column.render(&vpc);
        assert_eq!(value, expected);
        assert_eq!(style.fg, Some(color));
        let expanded = expanded_from_columns(&[column], &vpc);
        assert_eq!(expanded[0].0, format!("Block Public Access: {expected}"));
        assert_eq!(expanded[0].1.fg, Some(color));
    }
}
