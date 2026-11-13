//! Rendering for the EC2 instance detail page: the fixed Instance summary,
//! the console tabs, each tab pane, the Monitoring charts and the tables.
use super::*;
use crate::{
    common::{render_pagination_text, SortDirection},
    table::TableWindow,
    tags::{TagSort, TagsConfig},
    ui::{
        self,
        filter::{render_filter_bar, FilterConfig, FilterControl},
        loading::render_table,
        table::{Column, TableConfig},
        Summary,
    },
};
use ratatui::prelude::*;
use rusticity_core::ec2::detail::{Ec2Metric, Section, StatusChecks};

pub(super) fn clean(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

fn value(text: &str) -> String {
    if text.is_empty() {
        "-".into()
    } else {
        clean(text)
    }
}

fn list(values: &[String]) -> String {
    value(&values.join(", "))
}

fn number(n: Option<i32>) -> String {
    n.map(|n| n.to_string()).unwrap_or_else(|| "-".into())
}

fn enabled(flag: Option<bool>) -> String {
    match flag {
        Some(true) => "Enabled".into(),
        Some(false) => "Disabled".into(),
        None => "-".into(),
    }
}

fn yes_no(flag: Option<bool>) -> String {
    match flag {
        Some(true) => "Yes".into(),
        Some(false) => "No".into(),
        None => "-".into(),
    }
}

/// `enabled` -> `Enabled`.
fn capitalize(raw: &str) -> String {
    let mut chars = raw.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The last path segment of an ARN: an instance profile or Outpost name.
fn arn_name(arn: &str) -> String {
    value(arn.rsplit('/').next().unwrap_or_default())
}

/// `IP name: ip-10-0-0-5.ec2.internal`, as the console words it.
fn hostname_type(d: &Ec2InstanceDetail) -> String {
    match d.hostname_type.as_str() {
        "ip-name" => format!("IP name: {}", clean(&d.private_dns)),
        "resource-name" => format!("Resource name: {}", clean(&d.private_dns)),
        other => value(other),
    }
}

fn rbn_hostname(d: &Ec2InstanceDetail) -> String {
    match d.hostname_type.as_str() {
        "" => "-".into(),
        kind => enabled(Some(kind == "resource-name")),
    }
}

/// Instance summary, in console order. The ID comes from the entered row;
/// every other field comes only from the fresh read.
pub(crate) fn summary_fields(
    instance_id: &str,
    detail: Option<&Ec2InstanceDetail>,
    status: ReadStatus,
) -> Vec<Line<'static>> {
    let fresh = |label: &str, read: &dyn Fn(&Ec2InstanceDetail) -> String| {
        let text = match (status, detail) {
            (ReadStatus::Ready, Some(d)) => read(d),
            (ReadStatus::Failed, _) => "unavailable".into(),
            _ => "…".into(),
        };
        field(label, text)
    };
    let state = match (status, detail) {
        (ReadStatus::Ready, Some(d)) => {
            let (text, color) = crate::ec2::format_state(&d.state);
            ui::labeled_field_colored("Instance state", text, color)
        }
        (ReadStatus::Failed, _) => ui::labeled_field("Instance state", "unavailable"),
        _ => ui::labeled_field("Instance state", "…"),
    };
    vec![
        ui::labeled_field("Instance ID", value(instance_id)),
        fresh("Public IPv4 address", &|d| value(&d.public_ipv4)),
        fresh("Private IPv4 addresses", &|d| list(&d.private_ipv4s)),
        fresh("IPv6 address", &|d| list(&d.ipv6_addresses)),
        state,
        fresh("Public DNS", &|d| value(&d.public_dns)),
        fresh("Hostname type", &hostname_type),
        fresh("Private IP DNS name (IPv4 only)", &|d| {
            value(&d.private_dns)
        }),
        fresh("Instance type", &|d| value(&d.instance_type)),
        fresh("Elastic IP addresses", &|d| list(&d.elastic_ips)),
        fresh(
            "Auto-assigned IP address",
            &|d| match d.auto_assigned_ip.as_str() {
                "" => "-".into(),
                ip => format!("{} [Public IP]", clean(ip)),
            },
        ),
        fresh("VPC ID", &|d| value(&d.vpc_id)),
        fresh("IAM Role", &|d| arn_name(&d.iam_instance_profile_arn)),
        fresh("Subnet ID", &|d| value(&d.subnet_id)),
        fresh("IMDSv2", &|d| value(&capitalize(&d.http_tokens))),
        fresh("Availability Zone", &|d| value(&d.availability_zone)),
        fresh("Managed", &|d| match d.managed {
            Some(managed) => managed.to_string(),
            None => "-".into(),
        }),
        fresh("Operator", &|d| value(&d.operator)),
    ]
}

/// The Details tab: three bordered panes, in console order.
pub(crate) fn details_panes(
    d: &Ec2InstanceDetail,
) -> Vec<(&'static str, Vec<(&'static str, String)>)> {
    vec![
        (
            "Instance details",
            vec![
                ("AMI ID", value(&d.image_id)),
                ("Monitoring", value(&d.monitoring)),
                ("Platform details", value(&d.platform_details)),
                ("AMI name", value(&d.ami_name)),
                ("Allowed image", value(&d.allowed_image)),
                ("Termination protection", value(&d.termination_protection)),
                ("Stop protection", value(&d.stop_protection)),
                ("Launch time", value(&d.launch_time)),
                ("AMI location", value(&d.ami_location)),
                (
                    "Instance reboot migration",
                    value(&capitalize(&d.reboot_migration)),
                ),
                (
                    "Instance auto-recovery",
                    value(&capitalize(&d.auto_recovery)),
                ),
                (
                    "Lifecycle",
                    match d.lifecycle.as_str() {
                        "" => "normal".into(),
                        other => clean(other),
                    },
                ),
                ("Stop-hibernate behavior", enabled(d.hibernation)),
                ("AMI Launch index", number(d.ami_launch_index)),
                ("Key pair assigned at launch", value(&d.key_name)),
                ("State transition reason", value(&d.state_transition_reason)),
                ("Credit specification", value(&d.credit_specification)),
                ("Kernel ID", value(&d.kernel_id)),
                (
                    "State transition message",
                    value(&d.state_transition_message),
                ),
                ("Usage operation", value(&d.usage_operation)),
                ("RAM disk ID", value(&d.ramdisk_id)),
                ("Owner", value(&d.owner_id)),
                ("Enclaves Support", enabled(d.enclaves)),
                ("Boot mode", value(&d.boot_mode)),
                ("Current instance boot mode", value(&d.current_boot_mode)),
                (
                    "Allow tags in instance metadata",
                    value(&capitalize(&d.metadata_tags)),
                ),
                ("Use RBN as guest OS hostname", rbn_hostname(d)),
                (
                    "Answer RBN DNS hostname IPv4",
                    enabled(d.resource_name_a_record),
                ),
            ],
        ),
        (
            "Host and placement group",
            vec![
                ("Host ID", value(&d.host_id)),
                ("Affinity", value(&d.affinity)),
                ("Placement group", value(&d.placement_group)),
                ("Host resource group name", value(&d.host_resource_group)),
                ("Tenancy", value(&d.tenancy)),
                ("Placement group ID", value(&d.placement_group_id)),
                ("Virtualization type", value(&d.virtualization_type)),
                ("Reservation", value(&d.reservation_id)),
                ("Partition number", number(d.partition_number)),
                ("Number of vCPUs", number(d.vcpus)),
            ],
        ),
        (
            "Capacity reservation",
            vec![
                ("Capacity Reservation ID", value(&d.capacity_reservation_id)),
                (
                    "Capacity Reservation setting",
                    value(&d.capacity_reservation_preference),
                ),
            ],
        ),
    ]
}

/// `ok` -> `System reachability check passed`.
fn check(kind: &str, raw: &str) -> String {
    match raw {
        "ok" => format!("{kind} reachability check passed"),
        "impaired" => format!("{kind} reachability check failed"),
        "insufficient-data" => "Insufficient data".into(),
        "not-applicable" => "Not applicable".into(),
        other => value(&capitalize(other)),
    }
}

pub(crate) fn status_fields(checks: &Result<StatusChecks, String>) -> Vec<(&'static str, String)> {
    let (system, instance, ebs) = match checks {
        Ok(c) => (
            check("System", &c.system),
            check("Instance", &c.instance),
            check("Attached EBS", &c.attached_ebs),
        ),
        Err(_) => (
            "Unavailable".into(),
            "Unavailable".into(),
            "Unavailable".into(),
        ),
    };
    vec![
        ("System status checks", system),
        ("Instance status checks", instance),
        ("Attached EBS status checks", ebs),
    ]
}

pub(crate) fn security_fields(d: &Ec2InstanceDetail) -> Vec<(&'static str, String)> {
    let groups: Vec<String> = d
        .security_groups
        .iter()
        .map(|(id, name)| match name.as_str() {
            "" => id.clone(),
            name => format!("{id} ({name})"),
        })
        .collect();
    vec![
        ("IAM Role", arn_name(&d.iam_instance_profile_arn)),
        ("Owner ID", value(&d.owner_id)),
        ("Launch time", value(&d.launch_time)),
        ("Security groups", list(&groups)),
    ]
}

pub(crate) fn networking_fields(d: &Ec2InstanceDetail) -> Vec<(&'static str, String)> {
    vec![
        ("Public IPv4 address", value(&d.public_ipv4)),
        ("Private IPv4 addresses", list(&d.private_ipv4s)),
        ("VPC ID", value(&d.vpc_id)),
        ("Public DNS", value(&d.public_dns)),
        ("Subnet ID", value(&d.subnet_id)),
        ("IPv6 addresses", list(&d.ipv6_addresses)),
        (
            "Secondary private IPv4 addresses",
            list(&d.secondary_private_ipv4s),
        ),
        ("Availability Zone", value(&d.availability_zone)),
        ("Carrier IP addresses (ephemeral)", list(&d.carrier_ips)),
        ("Outpost ID", arn_name(&d.outpost_arn)),
        ("Use RBN as guest OS hostname", rbn_hostname(d)),
        (
            "Answer RBN DNS hostname IPv4",
            enabled(d.resource_name_a_record),
        ),
        ("Hostname type", hostname_type(d)),
        ("Private IP DNS name (IPv4 only)", value(&d.private_dns)),
    ]
}

pub(crate) fn storage_fields(d: &Ec2InstanceDetail) -> Vec<(&'static str, String)> {
    vec![
        ("Root device name", value(&d.root_device_name)),
        ("Root device type", value(&d.root_device_type)),
        ("EBS optimization", enabled(d.ebs_optimized)),
    ]
}

/// The pane above each tab table, titled as in the console.
fn pane(d: &Ec2InstanceDetail, tab: Tab) -> Option<(&'static str, Vec<(&'static str, String)>)> {
    Some(match tab {
        Tab::StatusAndAlarms => ("Status checks", status_fields(&d.sections.status_checks)),
        Tab::Security => ("Security details", security_fields(d)),
        Tab::Networking => ("Networking details", networking_fields(d)),
        Tab::Storage => ("Root device details", storage_fields(d)),
        _ => return None,
    })
}

/// Table cells, in `Slot::columns` order.
pub(super) fn rows(d: &Ec2InstanceDetail, slot: Slot) -> Vec<Vec<String>> {
    let s = &d.sections;
    let rules = |section: &Section<rusticity_core::ec2::detail::Ec2SecurityGroupRule>| {
        section
            .iter()
            .flatten()
            .map(|r| {
                vec![
                    r.name.clone(),
                    r.rule_id.clone(),
                    r.port_range.clone(),
                    r.protocol.clone(),
                    r.peer.clone(),
                    r.security_group.clone(),
                    r.description.clone(),
                ]
            })
            .collect()
    };
    match slot {
        Slot::Alarms => s
            .alarms
            .iter()
            .flatten()
            .map(|a| vec![a.name.clone(), a.state.clone(), a.condition.clone()])
            .collect(),
        Slot::InboundRules => rules(&s.inbound_rules),
        Slot::OutboundRules => rules(&s.outbound_rules),
        Slot::NetworkInterfaces => d
            .network_interfaces
            .iter()
            .map(|n| {
                vec![
                    n.id.clone(),
                    n.description.clone(),
                    n.ipv4_prefixes.join(", "),
                    n.ipv6_prefixes.join(", "),
                    n.public_ipv4.clone(),
                    n.private_ipv4.clone(),
                    n.private_dns.clone(),
                    n.ipv6_addresses.join(", "),
                    n.attach_time.clone(),
                    capitalize(&n.attachment_status),
                    n.vpc_id.clone(),
                    n.subnet_id.clone(),
                    yes_no(n.delete_on_termination),
                    n.security_groups.join(", "),
                    n.interface_type.clone(),
                ]
            })
            .collect(),
        Slot::BlockDevices => s
            .block_devices
            .iter()
            .flatten()
            .map(|b| {
                vec![
                    b.volume_id.clone(),
                    b.device_name.clone(),
                    b.size_gib.map(|n| n.to_string()).unwrap_or_default(),
                    capitalize(&b.attachment_status),
                    b.attach_time.clone(),
                    yes_no(b.encrypted),
                    b.kms_key_id.clone(),
                    yes_no(b.delete_on_termination),
                ]
            })
            .collect(),
    }
}

/// Enabled and passed checks green, Disabled yellow, failed checks red.
pub(crate) fn state_color(text: &str) -> Option<Color> {
    if text.starts_with("Enabled") || text.ends_with("check passed") {
        Some(Color::Green)
    } else if text.starts_with("Disabled") {
        Some(Color::Yellow)
    } else if text.ends_with("check failed") {
        Some(Color::Red)
    } else {
        None
    }
}

fn field(label: &str, text: String) -> Line<'static> {
    match state_color(&text) {
        Some(color) => ui::labeled_field_colored(label, text, color),
        None => ui::labeled_field(label, text),
    }
}

fn summary<'a>(title: &'a str, fields: Vec<(&'static str, String)>) -> Summary<'a> {
    Summary::new(
        title,
        fields
            .into_iter()
            .map(|(label, text)| field(label, text))
            .collect(),
    )
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let ec2 = &app.ec2_state;
    let Some(instance_id) = ec2.current_instance.as_deref() else {
        return;
    };
    let state = &ec2.detail;
    let status = state.status();
    let header = Summary::new(
        "Instance summary",
        summary_fields(instance_id, state.detail(), status),
    );
    let tabs: Vec<(&str, Tab)> = Tab::ALL.iter().map(|t| (t.name(), *t)).collect();
    let chunks = Layout::vertical([
        Constraint::Length(header.height(area.width)),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(area);
    header.render_fill(frame, chunks[0]);
    ui::render_tabs(frame, chunks[1], &tabs, &ec2.detail_tab);
    let body = chunks[2];
    if ui::loading::render(frame, body, status, "instance details", state.error()) {
        return;
    }
    let detail = state.detail().expect("Ready implies detail");
    match ec2.detail_tab {
        Tab::Details => {
            let panes: Vec<_> = details_panes(detail)
                .into_iter()
                .map(|(title, fields)| summary(title, fields))
                .collect();
            let areas = Layout::vertical(
                panes
                    .iter()
                    .map(|p| Constraint::Length(p.height(body.width)))
                    .chain([Constraint::Min(0)]),
            )
            .split(body);
            for (pane, area) in panes.into_iter().zip(areas.iter()) {
                pane.render(frame, *area);
            }
        }
        Tab::Monitoring => render_monitoring(frame, state, &detail.sections.monitoring, body),
        Tab::Tags => render_tags(frame, app, body),
        tab => {
            let (title, fields) = pane(detail, tab).expect("table tab has a pane");
            let pane = summary(title, fields);
            let parts = Layout::vertical([
                Constraint::Length(pane.height(body.width)),
                Constraint::Min(0),
            ])
            .split(body);
            pane.render(frame, parts[0]);
            render_table_tab(frame, app, parts[1]);
        }
    }
}

fn render_tags(frame: &mut Frame, app: &App, area: Rect) {
    let ec2 = &app.ec2_state;
    let columns = ec2
        .tag_visible_column_ids
        .iter()
        .filter_map(|id| crate::ec2::tag::Column::from_id(id))
        .map(|c| Box::new(c) as Box<dyn Column<InstanceTag>>)
        .collect();
    crate::tags::render(
        frame,
        area,
        TagsConfig {
            table: &ec2.tags,
            items: &ec2.tags.items,
            columns,
            sort: TagSort::Value,
            window: TableWindow::SelectedPage,
            focus: ec2.input_focus,
            mode: app.mode,
            loading: false,
        },
    );
}

/// Chart titles as before: the 3-hour average or sum next to the name.
pub(crate) fn chart_title(metric: &Ec2Metric) -> (String, &'static str) {
    let values = || metric.points.iter().map(|(_, v)| v);
    let sum: f64 = values().sum();
    let avg = if metric.points.is_empty() {
        0.0
    } else {
        sum / metric.points.len() as f64
    };
    match metric.name {
        "CPUUtilization" => (format!("CPU utilization (%) [avg: {avg:.2}]"), "%"),
        "NetworkIn" => (format!("Network in (bytes) [sum: {sum:.0}]"), "bytes"),
        "NetworkOut" => (format!("Network out (bytes) [sum: {sum:.0}]"), "bytes"),
        "NetworkPacketsIn" => (
            format!("Network packets in (count) [sum: {sum:.0}]"),
            "count",
        ),
        "NetworkPacketsOut" => (
            format!("Network packets out (count) [sum: {sum:.0}]"),
            "count",
        ),
        "MetadataNoToken" => (
            format!("Metadata no token (count) [sum: {sum:.0}]"),
            "count",
        ),
        other => (other.to_string(), ""),
    }
}

fn render_monitoring(frame: &mut Frame, state: &State, section: &Section<Ec2Metric>, area: Rect) {
    let metrics = match section {
        Ok(metrics) => metrics,
        Err(error) => {
            ui::loading::render(frame, area, ReadStatus::Failed, "Monitoring", Some(error));
            return;
        }
    };
    let titles: Vec<_> = metrics.iter().map(chart_title).collect();
    let charts: Vec<_> = metrics
        .iter()
        .zip(&titles)
        .map(|(m, (title, unit))| crate::ui::monitoring::MetricChart {
            title,
            data: &m.points,
            y_axis_label: unit,
            x_axis_label: None,
            ..Default::default()
        })
        .collect();
    crate::ui::monitoring::render_monitoring_tab(
        frame,
        area,
        &charts,
        &[],
        &[],
        &[],
        state.monitoring_scroll,
    );
}

fn columns(spec: &'static [(&'static str, u16)], visible: &[&str]) -> Vec<Box<dyn Column<Row>>> {
    spec.iter()
        .enumerate()
        .filter(|(_, (id, _))| visible.contains(id))
        .map(|(index, (name, width))| {
            Box::new(ui::table::OwnedColumn::new(
                name,
                name,
                *width,
                false,
                move |row: &Row| {
                    (
                        value(row.cells.get(index).map(String::as_str).unwrap_or_default()),
                        Style::default(),
                    )
                },
            )) as Box<dyn Column<Row>>
        })
        .collect()
}

/// Filter placeholder and the empty-result text of each table.
fn texts(slot: Slot) -> (&'static str, &'static str) {
    match slot {
        Slot::Alarms => ("Filter alarms", "No alarms watch this instance."),
        Slot::InboundRules => ("Filter inbound rules", "No inbound rules."),
        Slot::OutboundRules => ("Filter outbound rules", "No outbound rules."),
        Slot::NetworkInterfaces => ("Filter network interfaces", "No network interfaces."),
        Slot::BlockDevices => ("Filter block devices", "No EBS volumes are attached."),
    }
}

/// The tab table with its filter bar; Security adds a second ribbon
/// (Inbound rules, Outbound rules) and `f`/`F` move along it.
fn render_table_tab(frame: &mut Frame, app: &App, area: Rect) {
    let ec2 = &app.ec2_state;
    let Some(focused) = slot(ec2) else {
        return;
    };
    let slots = tab_tables(ec2.detail_tab);
    let area = if slots.len() > 1 {
        let parts = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area);
        let ribbon: Vec<(&str, Slot)> = slots.iter().map(|s| (s.name(), *s)).collect();
        ui::render_tabs(frame, parts[0], &ribbon, &focused);
        parts[1]
    } else {
        area
    };
    let state = &ec2.detail;
    let table = state.table(focused);
    let parts = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);
    let prepared = table
        .rows
        .prepare_view(table.filtered(), TableWindow::SelectedPage);
    render_filter_bar(
        frame,
        FilterConfig {
            filter_text: &table.rows.filter,
            placeholder: texts(focused).0,
            mode: app.mode,
            is_input_focused: table.focus == InputFocus::Filter,
            controls: vec![FilterControl {
                text: render_pagination_text(prepared.current_page, prepared.total_pages),
                is_focused: table.focus == InputFocus::Pagination,
            }],
            area: parts[0],
        },
    );
    // A failed section fails only its own table.
    let (status, error) = match state.section_error(focused) {
        Some(error) => (ReadStatus::Failed, Some(error)),
        None => (ReadStatus::Ready, None),
    };
    let spec = focused.columns();
    let columns = columns(spec, &table.visible_column_ids);
    let sort_header = table.sort.header(
        &table.visible_column_ids,
        |id| Some(id.to_string()),
        ("", SortDirection::Asc),
    );
    render_table(
        frame,
        TableConfig {
            items: prepared.items,
            selected_index: prepared.selected_index,
            expanded_index: prepared.expanded_index,
            columns: &columns,
            sort_column: &sort_header.0,
            sort_direction: sort_header.1,
            title: ui::format_title(&format!("{} ({})", focused.name(), prepared.total_items)),
            area: parts[1],
            get_expanded_content: Some(Box::new(move |row: &Row| {
                spec.iter()
                    .zip(&row.cells)
                    .map(|((name, _), cell)| (format!("{name}: {}", value(cell)), Style::default()))
                    .collect()
            })),
            is_active: app.mode == Mode::Normal,
        },
        status,
        focused.name(),
        error,
        ui::loading::EmptyState {
            no_items: texts(focused).1,
            has_unfiltered_items: !table.rows.items.is_empty(),
        },
    );
}
