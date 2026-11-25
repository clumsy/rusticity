//! App-level tests for VPC › Subnets and VPC › Security groups. Reads are
//! injected through the shared list read; no test makes an AWS request.
use super::{security_groups, subnets};
use crate::app::{App, Service};
use crate::async_read::ReadStatus;
use crate::common::SortDirection;
use crate::keymap::{handle_key, Action, Mode};
use crate::preferences::{bindings, PreferenceRow, Preferences};
use crate::sort::DEFAULT_VALUE;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};
use rusticity_core::vpc::{SecurityGroup, Subnet};

fn subnet(id: &str, name: &str) -> Subnet {
    Subnet {
        subnet_id: id.into(),
        name: name.into(),
        state: "available".into(),
        vpc_id: "vpc-a".into(),
        ipv4_cidr: "10.0.1.0/24".into(),
        available_ipv4: Some(250),
        availability_zone: "us-east-2a".into(),
        route_table: "rtb-a".into(),
        network_acl: "acl-a".into(),
        default_for_az: Some(true),
        arn: format!("arn:aws:ec2:us-east-2:123456789012:subnet/{id}"),
        ..Default::default()
    }
}

fn group(id: &str, name: &str, inbound: usize) -> SecurityGroup {
    SecurityGroup {
        group_id: id.into(),
        group_name: name.into(),
        vpc_id: "vpc-a".into(),
        description: format!("{name} access"),
        owner_id: "123456789012".into(),
        inbound_rules: inbound,
        outbound_rules: 1,
        arn: format!("arn:aws:ec2:us-east-2:123456789012:security-group/{id}"),
        ..Default::default()
    }
}

fn app(service: Service) -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-2".into()));
    app.aws.config.account_id = "123456789012".into();
    app.current_service = service;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.subnets_state.list.rows.items = vec![
        subnet("subnet-b", "private-b"),
        subnet("subnet-a", "public-a"),
    ];
    subnets::mark_loaded(&mut app);
    app.security_groups_state.list.rows.items =
        vec![group("sg-b", "web", 3), group("sg-a", "default", 1)];
    security_groups::mark_loaded(&mut app);
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

fn screen(app: &App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(240, 40)).unwrap();
    terminal.draw(|f| crate::ui::render(f, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

fn sort_rows(model: &Preferences) -> Vec<(&'static str, bool, Option<SortDirection>)> {
    model
        .rows()
        .iter()
        .filter_map(|row| match row {
            PreferenceRow::Sort {
                value,
                selected,
                direction,
                ..
            } => Some((*value, *selected, *direction)),
            _ => None,
        })
        .collect()
}

fn sorted(app: &App, service: Service) -> Option<(&'static str, Option<SortDirection>)> {
    sort_rows(&bindings::model_for(app, service))
        .into_iter()
        .find(|(_, selected, _)| *selected)
        .map(|(value, _, direction)| (value, direction))
}

fn subnet_ids(app: &App) -> Vec<String> {
    app.subnets_state
        .list
        .filtered()
        .iter()
        .map(|s| s.subnet_id.clone())
        .collect()
}

#[test]
fn both_lists_follow_vpcs_in_the_picker() {
    let app = App::new_without_client("beta".into(), None);
    let services = &app.service_picker.services;
    let at = |name: &str| services.iter().position(|s| *s == name).unwrap();
    assert_eq!(at("VPC › Subnets"), at("VPC › VPCs") + 1);
    assert_eq!(at("VPC › Security groups"), at("VPC › VPCs") + 2);
    assert_eq!(Service::VpcSubnets.name(), "VPC › Subnets");
    assert_eq!(Service::VpcSecurityGroups.name(), "VPC › Security groups");
}

#[test]
fn subnets_show_console_columns_ordered_by_subnet_id() {
    let app = app(Service::VpcSubnets);
    let text = screen(&app);
    assert!(text.contains("Subnets (2)"), "{text}");
    for header in [
        "Subnet ID ↑",
        "State",
        "VPC",
        "IPv4 CIDR",
        "IPv6 CIDR",
        "Available IPv4 addresses",
        "Availability Zone",
        "Route table",
        "Network ACL",
        "Default subnet",
    ] {
        assert!(text.contains(header), "{header}");
    }
    for hidden in ["Name", "Owner ID", "Subnet ARN"] {
        assert!(!text.contains(hidden), "{hidden} is shown once chosen");
    }
    for cell in ["Available", "10.0.1.0/24", "250", "rtb-a", "acl-a", "Yes"] {
        assert!(text.contains(cell), "{cell}");
    }
    assert_eq!(subnet_ids(&app), ["subnet-a", "subnet-b"]);
}

#[test]
fn every_subnet_column_is_in_preferences_and_enter_expands_all_fields() {
    let mut app = app(Service::VpcSubnets);
    let model = bindings::model_for(&app, Service::VpcSubnets);
    let column = |id: &str| model.columns().find(|c| c.id == id).unwrap().visible;
    let ids: Vec<_> = model.columns().map(|c| c.id.to_string()).collect();
    assert_eq!(
        ids,
        [
            "Subnet ID",
            "ARN",
            "State",
            "VPC",
            "Block Public Access",
            "Automated internet firewall connectivity type",
            "Outpost ID",
            "IPv4 CIDR",
            "IPv6 CIDR",
            "IPv6 CIDR association ID",
            "Available IPv4 addresses",
            "Availability Zone",
            "Network border group",
            "Route table",
            "Network ACL",
            "Default subnet",
            "Auto-assign public IPv4 address",
            "Auto-assign customer-owned IPv4 address",
            "Customer-owned IPv4 pool",
            "Auto-assign IPv6 address",
            "Owner ID",
            "Subnet ARN",
            "Enable DNS64",
        ],
        "the console columns, in console order"
    );
    assert!(column("Subnet ID") && !column("Owner ID") && !column("Enable DNS64"));
    app.subnets_state.list.rows.items[1].network_border_group = "us-east-2".into();
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.subnets_state.list.rows.expanded_item, Some(0));
    let text = screen(&app);
    assert!(text.contains("Network border group: us-east-2"), "{text}");
}

#[test]
fn columns_fit_header_and_content_and_the_last_takes_the_rest() {
    let mut app = app(Service::VpcSubnets);
    app.subnets_state.list.rows.items[0].subnet_id = "subnet-0c389dc8fe7220914".into();
    let text = screen(&app);
    let lines: Vec<String> = text
        .chars()
        .collect::<Vec<_>>()
        .chunks(240)
        .map(|c| c.iter().collect())
        .collect();
    let header = lines.iter().find(|l| l.contains("Subnet ID ↑")).unwrap();
    let at = |pat: &str| header[..header.find(pat).unwrap()].chars().count();
    // `► subnet-0c389dc8fe7220914` is 26 cells, wider than `  Subnet ID ↑`;
    // one cell of column spacing follows it.
    assert_eq!(
        at("⋮ State") - at("  Subnet ID"),
        27,
        "the ID column fits its widest value: {header}"
    );
    let last = &header[header.find("⋮ Default subnet").unwrap()..];
    assert!(
        last.ends_with("  │"),
        "the last column reaches the border: {header}"
    );
}

#[test]
fn security_groups_count_permission_entries() {
    let app = app(Service::VpcSecurityGroups);
    let text = screen(&app);
    assert!(text.contains("Security groups (2)"), "{text}");
    for header in [
        "Security group ID ↑",
        "Security group name",
        "VPC ID",
        "Description",
        "Owner",
        "Inbound rules count",
        "Outbound rules count",
    ] {
        assert!(text.contains(header), "{header}");
    }
    assert!(text.contains("3 Permission entries") && text.contains("1 Permission entry"));
}

#[test]
fn sort_by_marks_the_fixed_column_and_offers_no_default_order() {
    for (service, column, other) in [
        (Service::VpcSubnets, "Subnet ID", "State"),
        (
            Service::VpcSecurityGroups,
            "Security group ID",
            "Security group name",
        ),
    ] {
        let mut app = app(service);
        let rows = sort_rows(&bindings::model_for(&app, service));
        assert!(
            rows.iter().all(|(v, ..)| *v != DEFAULT_VALUE),
            "{service:?}"
        );
        assert_eq!(
            sorted(&app, service),
            Some((column, Some(SortDirection::Asc)))
        );
        crate::sort_tests::sort_by(&mut app, column);
        assert_eq!(
            sorted(&app, service),
            Some((column, Some(SortDirection::Desc)))
        );
        assert!(screen(&app).contains(&format!("{column} ↓")), "{service:?}");
        crate::sort_tests::sort_by(&mut app, other);
        assert_eq!(
            sorted(&app, service),
            Some((other, Some(SortDirection::Asc)))
        );
    }
    let mut app = app(Service::VpcSubnets);
    crate::sort_tests::sort_by(&mut app, "Subnet ID");
    assert_eq!(subnet_ids(&app), ["subnet-b", "subnet-a"]);
}

#[test]
fn a_spec_table_without_a_fixed_order_keeps_default_order() {
    let app = app(Service::Ec2TargetGroups);
    let rows = sort_rows(&bindings::model_for(&app, Service::Ec2TargetGroups));
    assert_eq!(rows[0], (DEFAULT_VALUE, true, None));
}

#[test]
fn the_filter_matches_the_name_tag_without_a_name_column() {
    for (service, name) in [
        (Service::VpcSubnets, "private-b"),
        (Service::VpcSecurityGroups, "web-tagged"),
    ] {
        let mut app = app(service);
        app.security_groups_state.list.rows.items[0].name = "web-tagged".into();
        app.handle_action(Action::StartFilter);
        for c in name.chars() {
            app.handle_action(Action::FilterInput(c));
        }
        let shown = match service {
            Service::VpcSubnets => app.subnets_state.list.filtered().len(),
            _ => app.security_groups_state.list.filtered().len(),
        };
        assert_eq!(shown, 1, "{service:?} {name}");
        let model = bindings::model_for(&app, service);
        assert!(model.columns().all(|c| c.id != "Name"), "{service:?}");
    }
}

#[test]
fn the_filter_matches_hidden_columns_too() {
    let mut app = app(Service::VpcSubnets);
    app.subnets_state.list.rows.items[0].network_acl = "acl-private".into();
    app.handle_action(Action::StartFilter);
    for c in "acl-private".chars() {
        app.handle_action(Action::FilterInput(c));
    }
    assert_eq!(subnet_ids(&app), ["subnet-b"]);
}

#[tokio::test]
async fn refresh_clears_rows_and_a_failed_read_never_restores_them() {
    let mut app = app(Service::VpcSubnets);
    ctrl(&mut app, 'r');
    assert!(app.subnets_state.list.rows.items.is_empty());
    assert!(matches!(
        app.subnets_state.load_status(),
        ReadStatus::Loading { .. }
    ));
    let text = screen(&app);
    assert!(!text.contains("No subnets") && !text.contains("public-a"));
    let scope = subnets::scope(&app);
    let state = &mut app.subnets_state;
    assert!(
        state
            .drive(&scope, "Subnets read", async {
                Err(anyhow::anyhow!("UnauthorizedOperation"))
            })
            .await
    );
    tokio::task::yield_now().await;
    assert!(
        state
            .drive(&scope, "Subnets read", std::future::pending())
            .await
    );
    assert_eq!(state.load_status(), ReadStatus::Failed);
    let error = state.inventory.error().unwrap();
    assert!(error.contains("UnauthorizedOperation"), "{error}");
    assert!(state.list.rows.items.is_empty());
    assert!(screen(&app).contains("UnauthorizedOperation"));

    ctrl(&mut app, 'r');
    let state = &mut app.subnets_state;
    state
        .drive(&scope, "Subnets read", async {
            Ok(vec![subnet("subnet-c", "fresh")])
        })
        .await;
    tokio::task::yield_now().await;
    state
        .drive(&scope, "Subnets read", std::future::pending())
        .await;
    assert_eq!(state.load_status(), ReadStatus::Ready);
    assert_eq!(subnet_ids(&app), ["subnet-c"]);
}

#[test]
fn esc_cancels_a_running_read() {
    let mut app = app(Service::VpcSecurityGroups);
    ctrl(&mut app, 'r');
    press(&mut app, KeyCode::Esc);
    let state = &app.security_groups_state;
    assert_eq!(state.load_status(), ReadStatus::Failed);
    assert!(state.inventory.error().unwrap().contains("Cancelled"));
    assert!(state.list.rows.items.is_empty());
    assert_eq!(app.current_service, Service::VpcSecurityGroups);
}

#[test]
fn console_and_copy_follow_the_list() {
    let mut app = app(Service::VpcSubnets);
    assert_eq!(
        app.get_console_url(),
        "https://us-east-2.console.aws.amazon.com/vpcconsole/home?region=us-east-2#subnets:"
    );
    assert_eq!(
        subnets::selected_arn(&app).as_deref(),
        Some("arn:aws:ec2:us-east-2:123456789012:subnet/subnet-a")
    );
    press(&mut app, KeyCode::Down);
    assert_eq!(
        subnets::selected_arn(&app).as_deref(),
        Some("arn:aws:ec2:us-east-2:123456789012:subnet/subnet-b")
    );
    let app = self::app(Service::VpcSecurityGroups);
    assert_eq!(
        app.get_console_url(),
        "https://us-east-2.console.aws.amazon.com/vpcconsole/home?region=us-east-2#SecurityGroups:"
    );
    assert_eq!(
        security_groups::selected_arn(&app).as_deref(),
        Some("arn:aws:ec2:us-east-2:123456789012:security-group/sg-a")
    );
}
