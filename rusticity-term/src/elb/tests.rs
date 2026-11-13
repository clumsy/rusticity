//! App-level tests for Load balancers and Target groups. Completions are
//! injected through the real poll path; no test makes an AWS request.
use super::*;
use crate::app::Service;
use crate::common::CyclicEnum;
use crate::keymap::{handle_key, Mode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};
use rusticity_core::elb::{
    HealthCheck, Listener, LoadBalancer, LoadBalancerDetail, Metric, ResourceRoute, SecurityGroup,
    Target, TargetGroup, TargetGroupDetail, ZoneMapping,
};

const LB: &str =
    "arn:aws:elasticloadbalancing:us-east-2:123456789012:loadbalancer/app/web/50dc6c495c0c9188";
const NLB: &str =
    "arn:aws:elasticloadbalancing:us-east-2:123456789012:loadbalancer/net/edge/7a1b2c3d4e5f6a7b";
const TG: &str =
    "arn:aws:elasticloadbalancing:us-east-2:123456789012:targetgroup/web-tg/73e2d6bc24d8a067";
const IP_TG: &str =
    "arn:aws:elasticloadbalancing:us-east-2:123456789012:targetgroup/ip-tg/1a2b3c4d5e6f7a8b";

fn lb(name: &str, arn: &str, lb_type: &str) -> LoadBalancer {
    LoadBalancer {
        name: name.into(),
        arn: arn.into(),
        lb_type: lb_type.into(),
        state: "active".into(),
        dns_name: format!("{name}.list-snapshot.example"),
        ..Default::default()
    }
}

fn tg(name: &str, arn: &str, target_type: &str) -> TargetGroup {
    TargetGroup {
        name: name.into(),
        arn: arn.into(),
        target_type: target_type.into(),
        protocol: "HTTP".into(),
        port: Some(80),
        load_balancer_arns: vec![LB.into()],
        ..Default::default()
    }
}

fn app(service: Service) -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-2".into()));
    app.config.account_id = "123456789012".into();
    app.current_service = service;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.load_balancers_state.list.rows.items =
        vec![lb("web", LB, "application"), lb("edge", NLB, "network")];
    let scope = ReadScope::from_app(&app, "elb:load-balancers");
    app.load_balancers_state.inventory.mark_loaded(scope);
    app.target_groups_state.list.rows.items =
        vec![tg("web-tg", TG, "instance"), tg("ip-tg", IP_TG, "ip")];
    let scope = ReadScope::from_app(&app, "elb:target-groups");
    app.target_groups_state.inventory.mark_loaded(scope);
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
    let mut terminal = Terminal::new(TestBackend::new(220, 60)).unwrap();
    terminal.draw(|f| crate::ui::render(f, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

fn lb_detail() -> LoadBalancerDetail {
    LoadBalancerDetail {
        load_balancer: LoadBalancer {
            scheme: "internet-facing".into(),
            vpc_id: "vpc-1".into(),
            ip_address_type: "ipv4".into(),
            hosted_zone: "Z35SXDOTRQ7X7K".into(),
            dns_name: "web-1.us-east-2.elb.amazonaws.com".into(),
            zones: vec![ZoneMapping {
                zone: "us-east-2a".into(),
                subnet_id: "subnet-a".into(),
                ..Default::default()
            }],
            ..lb("web", LB, "application")
        },
        listeners: Ok(vec![Listener {
            arn: "listener-arn".into(),
            protocol: "HTTP".into(),
            port: Some(80),
            default_action: "Forward to target group: web-tg (100%)".into(),
            rules: Some(2),
            ..Default::default()
        }]),
        resource_map: Ok(vec![ResourceRoute {
            listener: "HTTP:80".into(),
            rule: "Default".into(),
            target_group: "web-tg".into(),
            target_group_arn: TG.into(),
            targets: Some(2),
            healthy: Some(1),
            unhealthy: Some(1),
        }]),
        security_groups: Ok(vec![SecurityGroup {
            id: "sg-1".into(),
            name: "web-sg".into(),
            description: "web ingress".into(),
        }]),
        monitoring: Ok(vec![Metric {
            name: "RequestCount",
            unit: "Count",
            points: vec![(0, 1.0)],
        }]),
        attributes: Ok(vec![
            ("deletion_protection.enabled".into(), "false".into()),
            ("idle_timeout.timeout_seconds".into(), "60".into()),
        ]),
        tags: Ok(vec![("team".into(), "web".into())]),
    }
}

fn target(id: &str, state: &str) -> Target {
    Target {
        id: id.into(),
        port: Some(80),
        state: state.into(),
        ..Default::default()
    }
}

fn tg_detail() -> TargetGroupDetail {
    TargetGroupDetail {
        target_group: TargetGroup {
            vpc_id: "vpc-1".into(),
            protocol_version: "HTTP1".into(),
            health_check: HealthCheck {
                protocol: "HTTP".into(),
                path: "/health".into(),
                port: "traffic-port".into(),
                healthy_threshold: Some(5),
                unhealthy_threshold: Some(2),
                timeout_seconds: Some(5),
                interval_seconds: Some(30),
                success_codes: "200".into(),
                ..Default::default()
            },
            ..tg("web-tg", TG, "instance")
        },
        targets: Ok(vec![
            Target {
                name: "web-1".into(),
                ..target("i-0123456789abcdef0", "healthy")
            },
            Target {
                description: "Health checks failed".into(),
                ..target("i-0fffffffffffffff0", "unhealthy")
            },
        ]),
        monitoring: Ok(vec![]),
        attributes: Ok(vec![(
            "load_balancing.algorithm.type".into(),
            "round_robin".into(),
        )]),
        tags: Ok(vec![]),
    }
}

async fn finish_lb(app: &mut App, result: Result<LoadBalancerDetail, String>) {
    let view = app.load_balancers_state.view.as_mut().unwrap();
    assert!(view.read.read.start(async move { result.map(Box::new) }));
    tokio::task::yield_now().await;
    assert!(load_balancers::poll(app).await);
}

async fn finish_tg(app: &mut App, result: Result<TargetGroupDetail, String>) {
    let view = app.target_groups_state.view.as_mut().unwrap();
    assert!(view.read.read.start(async move { result.map(Box::new) }));
    tokio::task::yield_now().await;
    assert!(target_groups::poll(app).await);
}

fn goto_lb(app: &mut App, tab: load_balancers::Tab) {
    while app.load_balancers_state.view.as_ref().unwrap().tab != tab {
        press(app, KeyCode::Tab);
    }
}

fn goto_tg(app: &mut App, tab: target_groups::Tab) {
    while app.target_groups_state.view.as_ref().unwrap().tab != tab {
        press(app, KeyCode::Tab);
    }
}

async fn entered_lb() -> App {
    let mut app = app(Service::Ec2LoadBalancers);
    press(&mut app, KeyCode::Enter);
    finish_lb(&mut app, Ok(lb_detail())).await;
    app
}

async fn entered_tg() -> App {
    let mut app = app(Service::Ec2TargetGroups);
    press(&mut app, KeyCode::Enter);
    finish_tg(&mut app, Ok(tg_detail())).await;
    app
}

#[test]
fn both_services_are_in_the_picker_under_ec2() {
    let app = App::new_without_client("beta".into(), None);
    let services = &app.service_picker.services;
    let at = |name: &str| services.iter().position(|s| *s == name).unwrap();
    assert_eq!(at("EC2 › Load balancers"), at("EC2 › Instances") + 1);
    assert_eq!(at("EC2 › Target groups"), at("EC2 › Instances") + 2);
    assert_eq!(Service::Ec2LoadBalancers.name(), "EC2 › Load balancers");
    assert_eq!(Service::Ec2TargetGroups.name(), "EC2 › Target groups");
}

#[test]
fn tabs_follow_console_names_and_order() {
    let names = |lb_type| -> Vec<&str> {
        load_balancers::Tab::ALL
            .iter()
            .map(|t| t.name(lb_type))
            .collect()
    };
    assert_eq!(
        names("application"),
        [
            "Listeners and rules",
            "Network mapping",
            "Resource map",
            "Security",
            "Monitoring",
            "Attributes",
            "Tags",
        ]
    );
    assert_eq!(names("network")[0], "Listeners");
    let tg: Vec<_> = target_groups::Tab::ALL.iter().map(|t| t.name()).collect();
    assert_eq!(
        tg,
        [
            "Targets",
            "Monitoring",
            "Health checks",
            "Attributes",
            "Tags"
        ]
    );
}

#[tokio::test]
async fn the_list_never_claims_empty_before_success_and_refresh_clears_it() {
    let mut app = app(Service::Ec2LoadBalancers);
    let text = screen(&app);
    assert!(text.contains("Load balancers (2)") && text.contains("web"));
    for header in [
        "Name",
        "DNS name",
        "State",
        "VPC ID",
        "Availability Zones",
        "Type",
        "Date created",
    ] {
        assert!(text.contains(header), "{header}");
    }
    ctrl(&mut app, 'r');
    assert!(app.load_balancers_state.list.rows.items.is_empty());
    assert!(matches!(
        app.load_balancers_state.load_status(),
        ReadStatus::Loading { .. }
    ));
    let text = screen(&app);
    assert!(!text.contains("No load balancers") && !text.contains("list-snapshot"));
}

#[test]
fn enter_opens_the_exact_filtered_row_and_queues_its_read() {
    let mut app = app(Service::Ec2LoadBalancers);
    app.load_balancers_state.list.rows.filter = "edge".into();
    press(&mut app, KeyCode::Enter);
    let view = app.load_balancers_state.view.as_ref().unwrap();
    assert_eq!(view.source.arn, NLB);
    assert!(view.read.is_busy());
    assert_eq!(
        app.load_balancers_state.list.rows.filter, "edge",
        "list kept"
    );
    assert_eq!(app.breadcrumbs(), "EC2 > Load balancers > edge");
    assert!(
        screen(&app).contains("Listeners ⋮ Network mapping"),
        "NLB tab name"
    );
}

#[tokio::test]
async fn details_show_only_identity_until_the_fresh_read() {
    let mut app = app(Service::Ec2LoadBalancers);
    press(&mut app, KeyCode::Enter);
    let text = screen(&app);
    assert!(text.contains("Details") && text.contains(LB));
    assert!(text.contains("Loading…") && !text.contains("list-snapshot"));
    finish_lb(&mut app, Ok(lb_detail())).await;
    let text = screen(&app);
    for expected in [
        "Load balancer type: Application",
        "Status: Active",
        "Scheme: Internet-facing",
        "Load balancer IP address type: IPv4",
        "Hosted zone: Z35SXDOTRQ7X7K",
        "Availability Zones: subnet-a (us-east-2a)",
        "DNS name: web-1.us-east-2.elb.amazonaws.com",
    ] {
        assert!(text.contains(expected), "{expected}");
    }
}

#[tokio::test]
async fn every_load_balancer_tab_renders_its_data() {
    use load_balancers::Tab;
    let mut app = entered_lb().await;
    let text = screen(&app);
    assert!(text.contains("Listeners (1)") && text.contains("Forward to target group: web-tg"));
    assert!(text.contains("2 rules"));
    goto_lb(&mut app, Tab::NetworkMapping);
    let text = screen(&app);
    assert!(text.contains("Network mapping") && text.contains("Mappings (1)"));
    assert!(text.contains("subnet-a"));
    goto_lb(&mut app, Tab::ResourceMap);
    let text = screen(&app);
    assert!(text.contains("Resource map (1)") && text.contains("web-tg"));
    goto_lb(&mut app, Tab::Security);
    assert!(screen(&app).contains("web ingress"));
    goto_lb(&mut app, Tab::Monitoring);
    assert!(screen(&app).contains("RequestCount (Count)"));
    goto_lb(&mut app, Tab::Attributes);
    let text = screen(&app);
    assert!(text.contains("Deletion protection: Off"));
    assert!(text.contains("Connection idle timeout: 60 seconds"));
    goto_lb(&mut app, Tab::Tags);
    let text = screen(&app);
    assert!(text.contains("Tags (1)") && text.contains("team"));
    assert!(text.contains("Status: Active"), "the Details pane stays");
}

#[tokio::test]
async fn a_failed_section_fails_only_its_tab() {
    use load_balancers::Tab;
    let mut app = app(Service::Ec2LoadBalancers);
    press(&mut app, KeyCode::Enter);
    let mut d = lb_detail();
    d.security_groups = Err("Security groups unavailable: UnauthorizedOperation".into());
    finish_lb(&mut app, Ok(d)).await;
    goto_lb(&mut app, Tab::Security);
    let text = screen(&app);
    assert!(text.contains("UnauthorizedOperation") && !text.contains("web ingress"));
    goto_lb(&mut app, Tab::Tags);
    assert!(screen(&app).contains("team"));
}

#[tokio::test]
async fn refresh_clears_everything_and_a_failure_never_restores_it() {
    use load_balancers::Tab;
    let mut app = entered_lb().await;
    goto_lb(&mut app, Tab::Tags);
    ctrl(&mut app, 'r');
    let view = app.load_balancers_state.view.as_ref().unwrap();
    assert!(view.read.is_busy() && view.read.detail().is_none());
    assert!(view.tags.rows.items.is_empty() && view.listeners.rows.items.is_empty());
    assert_eq!(view.tab, Tab::Tags, "tab survives");
    assert!(!screen(&app).contains("team"));
    finish_lb(&mut app, Err("AccessDenied: DescribeLoadBalancers".into())).await;
    let text = screen(&app);
    assert!(text.contains("AccessDenied") && !text.contains("team"));
    assert!(text.contains("Status: unavailable"));
}

#[tokio::test]
async fn aws_context_change_closes_the_page() {
    let mut app = app(Service::Ec2LoadBalancers);
    press(&mut app, KeyCode::Enter);
    app.config.region = "eu-west-1".into();
    assert!(load_balancers::poll(&mut app).await);
    assert!(app.load_balancers_state.view.is_none());
}

#[tokio::test]
async fn esc_cancels_a_running_read_then_returns_to_the_list() {
    let mut app = app(Service::Ec2LoadBalancers);
    press(&mut app, KeyCode::Enter);
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let view = app.load_balancers_state.view.as_mut().unwrap();
    assert!(view.read.read.start(async move {
        let _keep = tx;
        std::future::pending::<Result<Box<LoadBalancerDetail>, String>>().await
    }));
    press(&mut app, KeyCode::Esc);
    let view = app.load_balancers_state.view.as_ref().unwrap();
    assert!(!view.read.is_busy() && view.read.error().unwrap().contains("Cancelled"));
    tokio::task::yield_now().await;
    assert!(rx.await.is_err(), "the read task was aborted");
    press(&mut app, KeyCode::Esc);
    assert!(app.load_balancers_state.view.is_none());
    assert_eq!(app.breadcrumbs(), "EC2 > Load balancers");
}

#[tokio::test]
async fn copy_console_and_cli_follow_the_current_view() {
    use load_balancers::Tab;
    let mut app = app(Service::Ec2LoadBalancers);
    assert!(app
        .get_console_url()
        .ends_with("/ec2/home?region=us-east-2#LoadBalancers:"));
    let command = crate::cli_command::invocation(&app).unwrap().display;
    assert!(
        command.starts_with("aws elbv2 describe-load-balancers "),
        "{command}"
    );
    press(&mut app, KeyCode::Enter);
    finish_lb(&mut app, Ok(lb_detail())).await;
    assert!(app
        .get_console_url()
        .contains("#LoadBalancer:loadBalancerArn=arn"));
    let buffer = ratatui::buffer::Buffer::empty(ratatui::layout::Rect::new(0, 0, 4, 1));
    press(&mut app, KeyCode::Char('y'));
    let mut copied = None;
    assert!(app.clipboard.complete_with(&buffer, |text| {
        copied = Some(text);
        Ok(())
    }));
    assert_eq!(copied.as_deref(), Some(LB));
    for (tab, operation) in [
        (Tab::Listeners, "aws elbv2 describe-listeners "),
        (Tab::NetworkMapping, "aws elbv2 describe-load-balancers "),
        (Tab::ResourceMap, "aws elbv2 describe-target-groups "),
        (
            Tab::Attributes,
            "aws elbv2 describe-load-balancer-attributes ",
        ),
        (Tab::Tags, "aws elbv2 describe-tags "),
    ] {
        goto_lb(&mut app, tab);
        let command = crate::cli_command::invocation(&app).unwrap().display;
        assert!(
            command.starts_with(operation) && command.contains(LB),
            "{command}"
        );
    }
}

#[tokio::test]
async fn preferences_route_to_the_tab_table_and_panes_are_read_only() {
    use crate::preferences::bindings::model_for;
    use load_balancers::Tab;
    let mut app = entered_lb().await;
    for (tab, count) in [
        (Tab::Listeners, 8),
        (Tab::NetworkMapping, 5),
        (Tab::ResourceMap, 6),
        (Tab::Security, 3),
        (Tab::Monitoring, 0),
        (Tab::Attributes, 0),
        (Tab::Tags, 2),
    ] {
        goto_lb(&mut app, tab);
        let model = model_for(&app, Service::Ec2LoadBalancers);
        assert_eq!(model.column_count(), count, "{tab:?}");
        press(&mut app, KeyCode::Char('p'));
        let expected = if count == 0 {
            Mode::Normal
        } else {
            Mode::ColumnSelector
        };
        assert_eq!(app.mode, expected, "{tab:?}");
        app.mode = Mode::Normal;
    }
}

#[tokio::test]
async fn the_sort_chooser_orders_a_detail_table() {
    let mut app = app(Service::Ec2LoadBalancers);
    press(&mut app, KeyCode::Enter);
    let mut d = lb_detail();
    d.tags = Ok(vec![("b".into(), "2".into()), ("a".into(), "1".into())]);
    finish_lb(&mut app, Ok(d)).await;
    goto_lb(&mut app, load_balancers::Tab::Tags);
    crate::sort_tests::sort_by(&mut app, "Key");
    let view = app.load_balancers_state.view.as_ref().unwrap();
    let keys: Vec<_> = view.tags.filtered().iter().map(|t| t.key.clone()).collect();
    assert_eq!(keys, ["a", "b"]);
}

#[tokio::test]
async fn target_groups_list_then_targets_with_health_and_counts() {
    use target_groups::Tab;
    let mut app = app(Service::Ec2TargetGroups);
    app.target_groups_state.list.rows.items[1]
        .load_balancer_arns
        .clear();
    let text = screen(&app);
    assert!(text.contains("Target groups (2)") && text.contains("Instance"));
    for header in [
        "Name",
        "ARN",
        "Port",
        "Protocol",
        "Target type",
        "Load balancer",
        "VPC ID",
    ] {
        assert!(text.contains(header), "{header}");
    }
    assert!(text.contains("None associated"));
    assert_eq!(
        target_groups::load_balancer_names(&[LB.into(), NLB.into()]),
        "web, edge"
    );
    let mut app = entered_tg().await;
    let text = screen(&app);
    for expected in [
        "Target type: Instance",
        "Protocol : Port: HTTP: 80",
        "Total targets: 2",
        "Healthy: 1",
        "Unhealthy: 1",
        "Registered targets (2)",
        "Instance ID",
        "web-1",
        "Health checks failed",
    ] {
        assert!(text.contains(expected), "{expected}");
    }
    goto_tg(&mut app, Tab::HealthChecks);
    let text = screen(&app);
    assert!(text.contains("Port: Traffic port"));
    assert!(text.contains("Healthy threshold: 5 consecutive health check successes"));
    goto_tg(&mut app, Tab::Attributes);
    assert!(screen(&app).contains("Load balancing algorithm: Round robin"));
    goto_tg(&mut app, Tab::Tags);
    assert!(screen(&app).contains("No tags are associated with this target group."));
    let command = crate::cli_command::invocation(&app).unwrap().display;
    assert!(command.starts_with("aws elbv2 describe-tags ") && command.contains(TG));
}

#[tokio::test]
async fn ip_targets_are_headed_by_ip_address() {
    let mut app = app(Service::Ec2TargetGroups);
    app.target_groups_state.list.rows.filter = "ip-tg".into();
    press(&mut app, KeyCode::Enter);
    let mut d = tg_detail();
    d.target_group = tg("ip-tg", IP_TG, "ip");
    d.targets = Ok(vec![target("10.0.0.5", "initial")]);
    finish_tg(&mut app, Ok(d)).await;
    let text = screen(&app);
    assert!(text.contains("IP address") && !text.contains("Instance ID"));
    assert!(text.contains("10.0.0.5") && text.contains("Initial"));
}

#[tokio::test]
async fn failed_targets_show_unavailable_counts_not_zero() {
    let mut app = app(Service::Ec2TargetGroups);
    press(&mut app, KeyCode::Enter);
    let mut d = tg_detail();
    d.targets = Err("Targets unavailable: AccessDenied".into());
    finish_tg(&mut app, Ok(d)).await;
    let text = screen(&app);
    assert!(text.contains("Total targets: Unavailable") && text.contains("AccessDenied"));
    assert!(!text.contains("web-1"));
}

#[test]
fn attributes_use_console_labels_then_raw_keys() {
    let fields = attribute_fields(&[
        ("zz.unknown".into(), "x".into()),
        ("routing.http2.enabled".into(), "true".into()),
        ("deletion_protection.enabled".into(), "false".into()),
        ("access_logs.s3.bucket".into(), String::new()),
    ]);
    let labels: Vec<_> = fields.iter().map(|(l, v)| format!("{l}: {v}")).collect();
    assert_eq!(
        labels,
        [
            "Deletion protection: Off",
            "HTTP/2: On",
            "access_logs.s3.bucket: -",
            "zz.unknown: x",
        ]
    );
}

#[tokio::test]
async fn the_tab_shows_a_spinner_while_its_read_runs() {
    use crate::app::Tab as AppTab;
    use crate::tab_runtime::{self, TabId};
    let mut app = app(Service::Ec2LoadBalancers);
    for _ in 0..2 {
        app.tabs.push(AppTab {
            id: TabId::new(),
            service: Service::Ec2LoadBalancers,
            title: "EC2 › Load balancers".into(),
            breadcrumb: "EC2 › Load balancers".into(),
        });
    }
    app.current_tab = 0;
    tab_runtime::sync_active(&mut app);
    // A fresh tab starts blank and queues its own read.
    app.current_tab = 1;
    tab_runtime::sync_active(&mut app);
    assert!(app.load_balancers_state.list.rows.items.is_empty());
    assert!(matches!(
        app.load_balancers_state.load_status(),
        ReadStatus::Loading { .. }
    ));
    let tabs: String = screen(&app).chars().skip(220).take(220).collect();
    assert!(
        tabs.contains("EC2 › Load balancers ✓ ⋮ EC2 › Load balancers ⠋"),
        "{}",
        tabs.trim_end()
    );
}
