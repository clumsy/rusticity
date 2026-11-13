//! App-level tests for the EC2 instance detail page. Completions are injected
//! through the real poll path; no test makes an AWS request.
use super::*;
use crate::app::Ec2Instance;
use crate::keymap::handle_key;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};
use rusticity_core::ec2::detail::{
    Ec2Alarm, Ec2BlockDevice, Ec2Metric, Ec2NetworkInterface, Ec2SecurityGroupRule, Sections,
    StatusChecks,
};

const ID: &str = "i-0123456789abcdef0";
const TWIN: &str = "i-0fedcba9876543210";

fn row(id: &str) -> Ec2Instance {
    Ec2Instance {
        instance_id: id.into(),
        name: format!("{id}-name"),
        state: "running".into(),
        instance_type: "list-snapshot-type".into(),
        ..Default::default()
    }
}

fn app() -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-2".into()));
    app.config.account_id = "123456789012".into();
    app.current_service = Service::Ec2Instances;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.ec2_state.table.items = vec![row(ID), row(TWIN)];
    let scope = ReadScope::from_app(&app, "inventory:ec2");
    app.ec2_state.inventory.mark_loaded(scope);
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

fn rule(id: &str, peer: &str) -> Ec2SecurityGroupRule {
    Ec2SecurityGroupRule {
        rule_id: id.into(),
        port_range: "443".into(),
        protocol: "TCP".into(),
        peer: peer.into(),
        security_group: "web".into(),
        ..Default::default()
    }
}

fn detail() -> Ec2InstanceDetail {
    Ec2InstanceDetail {
        instance_id: ID.into(),
        name: "web-1".into(),
        state: "running".into(),
        instance_type: "t3.micro".into(),
        public_ipv4: "3.0.0.1".into(),
        private_ipv4s: vec!["10.0.0.5".into()],
        private_dns: "ip-10-0-0-5.us-east-2.compute.internal".into(),
        hostname_type: "ip-name".into(),
        auto_assigned_ip: "3.0.0.1".into(),
        vpc_id: "vpc-1".into(),
        subnet_id: "subnet-1".into(),
        availability_zone: "us-east-2a".into(),
        iam_instance_profile_arn: "arn:aws:iam::123456789012:instance-profile/web-role".into(),
        http_tokens: "required".into(),
        image_id: "ami-1".into(),
        ami_name: "al2023-ami".into(),
        termination_protection: "Enabled".into(),
        stop_protection: "Disabled".into(),
        owner_id: "123456789012".into(),
        vcpus: Some(2),
        root_device_name: "/dev/xvda".into(),
        root_device_type: "ebs".into(),
        ebs_optimized: Some(true),
        security_groups: vec![("sg-1".into(), "web".into())],
        tags: vec![("team".into(), "data".into())],
        network_interfaces: vec![Ec2NetworkInterface {
            id: "eni-1".into(),
            attachment_status: "attached".into(),
            delete_on_termination: Some(true),
            ..Default::default()
        }],
        sections: Sections {
            status_checks: Ok(StatusChecks {
                system: "ok".into(),
                instance: "impaired".into(),
                attached_ebs: "ok".into(),
            }),
            alarms: Ok(vec![Ec2Alarm {
                name: "cpu-high".into(),
                state: "OK".into(),
                condition: "CPUUtilization > 80".into(),
            }]),
            inbound_rules: Ok(vec![rule("sgr-in", "0.0.0.0/0")]),
            outbound_rules: Ok(vec![rule("sgr-out", "::/0")]),
            block_devices: Ok(vec![Ec2BlockDevice {
                volume_id: "vol-1".into(),
                device_name: "/dev/xvda".into(),
                size_gib: Some(8),
                encrypted: Some(true),
                ..Default::default()
            }]),
            monitoring: Ok(rusticity_core::ec2::detail::METRICS
                .iter()
                .map(|name| Ec2Metric {
                    name,
                    points: vec![(0, 1.0)],
                })
                .collect()),
        },
        ..Default::default()
    }
}

async fn finish(app: &mut App, result: Result<Ec2InstanceDetail, String>) {
    assert!(app
        .ec2_state
        .detail
        .read
        .start(async move { result.map(Box::new) }));
    tokio::task::yield_now().await;
    assert!(poll(app).await);
}

fn screen(app: &App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(200, 60)).unwrap();
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
    while app.ec2_state.detail_tab != tab {
        press(app, KeyCode::Tab);
    }
}

async fn entered() -> App {
    let mut app = app();
    press(&mut app, KeyCode::Enter);
    finish(&mut app, Ok(detail())).await;
    app
}

#[test]
fn tabs_follow_console_names_and_order() {
    let names: Vec<_> = Tab::ALL.iter().map(|t| t.name()).collect();
    assert_eq!(
        names,
        [
            "Details",
            "Status and alarms",
            "Monitoring",
            "Security",
            "Networking",
            "Storage",
            "Tags",
        ]
    );
    let tables: Vec<Vec<_>> = Tab::ALL
        .iter()
        .map(|t| tab_tables(*t).iter().map(|s| s.name()).collect())
        .collect();
    assert_eq!(tables[1], ["Alarms"]);
    assert_eq!(tables[3], ["Inbound rules", "Outbound rules"]);
    assert_eq!(tables[4], ["Network interfaces"]);
    assert_eq!(tables[5], ["Block devices"]);
    assert!(tables[0].is_empty() && tables[2].is_empty() && tables[6].is_empty());
}

#[test]
fn enter_opens_the_exact_filtered_row_on_details_and_queues_the_read() {
    let mut app = app();
    app.ec2_state.detail_tab = Tab::Storage;
    app.ec2_state.tags.items = vec![InstanceTag {
        key: "stale".into(),
        value: "tag".into(),
    }];
    app.ec2_state.table.filter = TWIN.into();
    press(&mut app, KeyCode::Enter);
    assert!(active(&app));
    assert_eq!(app.ec2_state.current_instance.as_deref(), Some(TWIN));
    assert_eq!(app.ec2_state.detail_tab, Tab::Details);
    assert!(app.ec2_state.tags.items.is_empty(), "no tags from before");
    assert!(app.ec2_state.detail.read.is_busy(), "read queued on entry");
    assert_eq!(app.ec2_state.table.filter, TWIN, "list filter kept");
    assert_eq!(app.breadcrumbs(), format!("EC2 > {TWIN}"));
}

#[tokio::test]
async fn summary_shows_identity_only_until_the_fresh_read() {
    let mut app = app();
    press(&mut app, KeyCode::Enter);
    let text = screen(&app);
    assert!(text.contains("Instance summary") && text.contains(ID));
    assert!(text.contains("Loading…") && text.contains("…"));
    assert!(!text.contains("list-snapshot-type"));
    finish(&mut app, Ok(detail())).await;
    let text = screen(&app);
    assert!(text.contains("t3.micro") && text.contains("Running"));
    assert!(text.contains("IP name: ip-10-0-0-5.us-east-2.compute.internal"));
    assert!(text.contains("3.0.0.1 [Public IP]"));
    assert!(text.contains("IAM Role: web-role") && text.contains("IMDSv2: Required"));
    assert!(!text.contains("list-snapshot-type"));
}

#[tokio::test]
async fn every_tab_renders_its_pane_and_table_and_the_summary_stays_fixed() {
    let mut app = entered().await;
    let text = screen(&app);
    for expected in [
        "Instance details",
        "AMI name: al2023-ami",
        "Termination protection: Enabled",
        "Stop protection: Disabled",
        "Host and placement group",
        "Number of vCPUs: 2",
        "Capacity reservation",
    ] {
        assert!(text.contains(expected), "{expected}");
    }
    goto(&mut app, Tab::StatusAndAlarms);
    let text = screen(&app);
    assert!(text.contains("System reachability check passed"));
    assert!(text.contains("Instance reachability check failed"));
    assert!(text.contains("Alarms (1)") && text.contains("cpu-high"));
    goto(&mut app, Tab::Monitoring);
    assert!(screen(&app).contains("CPU utilization (%)"));
    goto(&mut app, Tab::Security);
    let text = screen(&app);
    assert!(text.contains("Security details") && text.contains("sg-1 (web)"));
    assert!(text.contains("Inbound rules (1)") && text.contains("sgr-in"));
    assert!(!text.contains("sgr-out"));
    press(&mut app, KeyCode::Char('f'));
    let text = screen(&app);
    assert!(text.contains("Outbound rules (1)") && text.contains("sgr-out"));
    assert!(text.contains("Destination"));
    goto(&mut app, Tab::Networking);
    let text = screen(&app);
    assert!(text.contains("Networking details") && text.contains("Network interfaces (1)"));
    assert!(text.contains("eni-1"));
    goto(&mut app, Tab::Storage);
    let text = screen(&app);
    assert!(text.contains("Root device details") && text.contains("EBS optimization: Enabled"));
    assert!(text.contains("Block devices (1)") && text.contains("vol-1"));
    goto(&mut app, Tab::Tags);
    let text = screen(&app);
    assert!(text.contains("Tags (1)") && text.contains("team") && text.contains("data"));
    assert!(
        text.contains("Instance summary") && text.contains("t3.micro"),
        "summary stays"
    );
}

#[test]
fn absent_values_show_a_dash_and_on_demand_is_normal() {
    let d = Ec2InstanceDetail::default();
    let get = |fields: Vec<(&str, String)>, label: &str| {
        fields.into_iter().find(|(l, _)| *l == label).unwrap().1
    };
    let details: Vec<_> = view::details_panes(&d)
        .into_iter()
        .flat_map(|(_, fields)| fields)
        .collect();
    assert_eq!(get(details.clone(), "Lifecycle"), "normal");
    assert_eq!(get(details.clone(), "AMI name"), "-");
    assert_eq!(get(details.clone(), "Number of vCPUs"), "-");
    assert_eq!(get(details, "Use RBN as guest OS hostname"), "-");
    assert_eq!(get(view::storage_fields(&d), "EBS optimization"), "-");
    assert_eq!(get(view::security_fields(&d), "IAM Role"), "-");
    let unavailable = view::status_fields(&Err("denied".into()));
    assert!(unavailable.iter().all(|(_, v)| v == "Unavailable"));
}

#[tokio::test]
async fn a_failed_section_fails_only_its_table() {
    let mut app = app();
    press(&mut app, KeyCode::Enter);
    let mut d = detail();
    d.sections.block_devices = Err("Block devices unavailable: UnauthorizedOperation".into());
    finish(&mut app, Ok(d)).await;
    goto(&mut app, Tab::Storage);
    let text = screen(&app);
    assert!(text.contains("UnauthorizedOperation"));
    assert!(text.contains("Root device details") && text.contains("/dev/xvda"));
    assert!(!text.contains("vol-1"));
    goto(&mut app, Tab::Networking);
    assert!(screen(&app).contains("eni-1"));
}

#[tokio::test]
async fn refresh_clears_everything_and_a_failure_never_restores_it() {
    let mut app = entered().await;
    goto(&mut app, Tab::Tags);
    ctrl(&mut app, 'r');
    assert!(app.ec2_state.detail.read.is_busy());
    assert!(app.ec2_state.detail.detail().is_none());
    assert!(app.ec2_state.tags.items.is_empty());
    assert!(app
        .ec2_state
        .detail
        .tables
        .iter()
        .all(|t| t.rows.items.is_empty()));
    assert_eq!(app.ec2_state.detail_tab, Tab::Tags, "tab survives");
    assert!(!screen(&app).contains("team"));
    finish(
        &mut app,
        Err("UnauthorizedOperation: ec2:DescribeInstances".into()),
    )
    .await;
    let text = screen(&app);
    assert!(text.contains("UnauthorizedOperation") && !text.contains("team"));
    goto(&mut app, Tab::Details);
    let text = screen(&app);
    assert!(text.contains("UnauthorizedOperation") && text.contains("unavailable"));
    assert!(!text.contains("al2023-ami") && !text.contains("t3.micro"));
}

#[tokio::test]
async fn aws_context_change_closes_the_page() {
    let mut app = app();
    press(&mut app, KeyCode::Enter);
    app.config.region = "eu-west-1".into();
    assert!(poll(&mut app).await);
    assert!(!active(&app));
    assert!(app.ec2_state.tags.items.is_empty());
}

#[tokio::test]
async fn esc_returns_to_the_untouched_list_and_aborts_the_read() {
    let mut app = app();
    app.ec2_state.table.filter = ID.into();
    press(&mut app, KeyCode::Enter);
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    assert!(app.ec2_state.detail.read.start(async move {
        let _keep = tx;
        std::future::pending::<Result<Box<Ec2InstanceDetail>, String>>().await
    }));
    press(&mut app, KeyCode::Esc);
    assert!(!active(&app));
    assert_eq!(app.ec2_state.table.filter, ID);
    assert_eq!(app.breadcrumbs(), "EC2 > Instances");
    tokio::task::yield_now().await;
    assert!(rx.await.is_err(), "the read task was aborted");
}

#[tokio::test]
async fn monitoring_scrolls_with_arrows_and_paging_within_the_charts() {
    let mut app = entered().await;
    goto(&mut app, Tab::Monitoring);
    for _ in 0..10 {
        press(&mut app, KeyCode::Down);
    }
    assert_eq!(app.ec2_state.detail.monitoring_scroll, 5);
    ctrl(&mut app, 'u');
    assert_eq!(app.ec2_state.detail.monitoring_scroll, 4);
    ctrl(&mut app, 'd');
    assert_eq!(app.ec2_state.detail.monitoring_scroll, 5);
    press(&mut app, KeyCode::Up);
    assert_eq!(app.ec2_state.detail.monitoring_scroll, 4);
}

#[tokio::test]
async fn copy_console_and_cli_follow_the_current_view() {
    let mut app = entered().await;
    assert!(app
        .get_console_url()
        .ends_with(&format!("#InstanceDetails:instanceId={ID}")));
    let buffer = ratatui::buffer::Buffer::empty(ratatui::layout::Rect::new(0, 0, 4, 1));
    for _ in Tab::ALL {
        press(&mut app, KeyCode::Char('y'));
        let mut copied = None;
        assert!(app.clipboard.complete_with(&buffer, |text| {
            copied = Some(text);
            Ok(())
        }));
        assert_eq!(copied.as_deref(), Some(ID));
        press(&mut app, KeyCode::Tab);
    }
    goto(&mut app, Tab::Security);
    let command = crate::cli_command::invocation(&app).unwrap().display;
    assert!(
        command.starts_with("aws ec2 describe-security-group-rules ") && command.contains("sg-1"),
        "{command}"
    );
}

#[tokio::test]
async fn preferences_route_to_the_tab_table_and_panes_are_read_only() {
    use crate::preferences::bindings::model_for;
    let mut app = entered().await;
    let model = model_for(&app, Service::Ec2Instances);
    assert!(!model.is_enabled() && model.column_count() == 0, "Details");
    for (tab, count) in [
        (Tab::StatusAndAlarms, 3),
        (Tab::Monitoring, 0),
        (Tab::Security, 7),
        (Tab::Networking, 15),
        (Tab::Storage, 8),
        (Tab::Tags, 2),
    ] {
        goto(&mut app, tab);
        assert_eq!(
            model_for(&app, Service::Ec2Instances).column_count(),
            count,
            "{tab:?}"
        );
    }
    goto(&mut app, Tab::Storage);
    press(&mut app, KeyCode::Char('p'));
    assert_eq!(app.mode, Mode::ColumnSelector);
    press(&mut app, KeyCode::Esc);
    goto(&mut app, Tab::Details);
    press(&mut app, KeyCode::Char('p'));
    assert_eq!(app.mode, Mode::Normal, "no preferences on a pane");
}

#[tokio::test]
async fn the_sort_chooser_orders_a_tab_table() {
    let mut app = app();
    press(&mut app, KeyCode::Enter);
    let mut d = detail();
    d.sections.inbound_rules = Ok(vec![
        rule("sgr-b", "10.0.0.0/8"),
        rule("sgr-a", "0.0.0.0/0"),
    ]);
    finish(&mut app, Ok(d)).await;
    goto(&mut app, Tab::Security);
    crate::sort_tests::sort_by(&mut app, "Security group rule ID");
    let table = app.ec2_state.detail.table(Slot::InboundRules);
    let ids: Vec<_> = table
        .filtered()
        .iter()
        .map(|r| r.cells[1].clone())
        .collect();
    assert_eq!(ids, ["sgr-a", "sgr-b"]);
}

#[tokio::test]
async fn table_filter_narrows_only_its_table() {
    let mut app = app();
    press(&mut app, KeyCode::Enter);
    let mut d = detail();
    d.sections.inbound_rules = Ok(vec![
        rule("sgr-b", "10.0.0.0/8"),
        rule("sgr-a", "0.0.0.0/0"),
    ]);
    finish(&mut app, Ok(d)).await;
    goto(&mut app, Tab::Security);
    press(&mut app, KeyCode::Char('i'));
    for c in "10.0".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    press(&mut app, KeyCode::Enter);
    let text = screen(&app);
    assert!(text.contains("sgr-b") && !text.contains("sgr-a"));
    assert!(app.ec2_state.table.filter.is_empty());
    assert!(app
        .ec2_state
        .detail
        .table(Slot::OutboundRules)
        .rows
        .filter
        .is_empty());
}

#[tokio::test]
async fn tags_filter_and_sort_by_value_before_paging() {
    let mut app = app();
    press(&mut app, KeyCode::Enter);
    let mut d = detail();
    d.tags = (0..25)
        .rev()
        .map(|i| (format!("wanted-{i:02}"), format!("rank{i:02}")))
        .chain([("excluded".to_string(), "rank-negative".to_string())])
        .collect();
    finish(&mut app, Ok(d)).await;
    goto(&mut app, Tab::Tags);
    app.ec2_state.tags.filter = "WANTED".into();
    app.ec2_state.tags.page_size = crate::common::PageSize::Ten;
    app.ec2_state.tags.selected = 12;
    let text = screen(&app);
    assert!(text.contains("Tags (25)"));
    assert!(text.contains("rank10") && text.contains("rank19"));
    assert!(text.find("rank10").unwrap() < text.find("rank19").unwrap());
    assert!(!text.contains("rank09") && !text.contains("rank20"));
    assert!(!text.contains("rank-negative"));
}

#[tokio::test]
async fn empty_tables_say_so_only_after_success() {
    let mut app = app();
    press(&mut app, KeyCode::Enter);
    goto(&mut app, Tab::Storage);
    assert!(!screen(&app).contains("No EBS volumes"));
    let mut d = detail();
    d.sections.block_devices = Ok(vec![]);
    finish(&mut app, Ok(d)).await;
    assert!(screen(&app).contains("No EBS volumes are attached."));
}
