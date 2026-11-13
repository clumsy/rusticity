//! Rendering for Load balancers and Target groups: the lists, the fixed
//! Details pane of an entered resource, its tab ribbon and each tab.
use super::{
    clean, ip_type_name, lb_type_name, load_balancers as lbs, scheme_name, style,
    target_groups as tgs, target_type_name, words,
};
use crate::{
    app::App,
    async_read::ReadStatus,
    common::CyclicEnum,
    spec_table::{self, Table, Texts},
    ui::{self, Summary},
};
use ratatui::prelude::*;
use rusticity_core::elb::{LoadBalancer, LoadBalancerDetail, Metric, Section, TargetGroupDetail};

fn value(text: &str) -> String {
    spec_table::value(text)
}

fn field(label: &str, text: String) -> Line<'static> {
    match style(&text).fg {
        Some(color) => ui::labeled_field_colored(label, text, color),
        None => ui::labeled_field(label, text),
    }
}

fn summary<'a, L: AsRef<str>>(title: &'a str, fields: Vec<(L, String)>) -> Summary<'a> {
    Summary::new(
        title,
        fields
            .into_iter()
            .map(|(label, text)| field(label.as_ref(), text))
            .collect(),
    )
}

/// A field from the fresh read, with a placeholder until then.
fn fresh<D>(status: ReadStatus, detail: Option<&D>, read: impl Fn(&D) -> String) -> String {
    match (status, detail) {
        (ReadStatus::Ready, Some(d)) => read(d),
        (ReadStatus::Failed, _) => "unavailable".into(),
        _ => "…".into(),
    }
}

/// The load balancer Details pane, in console order. Name and ARN come from
/// the entered row; every other field only from the fresh read.
pub(crate) fn load_balancer_fields(
    source: &LoadBalancer,
    detail: Option<&LoadBalancerDetail>,
    status: ReadStatus,
) -> Vec<(&'static str, String)> {
    let f = |read: fn(&LoadBalancer) -> String| fresh(status, detail, |d| read(&d.load_balancer));
    vec![
        (
            "Load balancer type",
            f(|l| value(&lb_type_name(&l.lb_type))),
        ),
        ("Status", f(|l| value(&words(&l.state)))),
        ("VPC", f(|l| value(&l.vpc_id))),
        (
            "Load balancer IP address type",
            f(|l| value(&ip_type_name(&l.ip_address_type))),
        ),
        ("Scheme", f(|l| value(&scheme_name(&l.scheme)))),
        ("Hosted zone", f(|l| value(&l.hosted_zone))),
        (
            "Availability Zones",
            f(|l| {
                let zones: Vec<String> = l
                    .zones
                    .iter()
                    .map(|z| format!("{} ({})", z.subnet_id, z.zone))
                    .collect();
                value(&zones.join(", "))
            }),
        ),
        (
            "Date created",
            f(|l| value(&super::timestamp(&l.created_time))),
        ),
        ("Load balancer ARN", value(&source.arn)),
        ("DNS name", f(|l| value(&l.dns_name))),
    ]
}

/// The target group Details pane, in console order. Target counts come from
/// the Targets read.
pub(crate) fn target_group_fields(
    detail: Option<&TargetGroupDetail>,
    status: ReadStatus,
) -> Vec<(&'static str, String)> {
    let f = |read: &dyn Fn(&TargetGroupDetail) -> String| fresh(status, detail, read);
    let counted = |states: &'static [&'static str]| {
        move |d: &TargetGroupDetail| match &d.targets {
            Ok(targets) => targets
                .iter()
                .filter(|t| states.is_empty() || states.contains(&t.state.as_str()))
                .count()
                .to_string(),
            Err(_) => "Unavailable".into(),
        }
    };
    vec![
        (
            "Target type",
            f(&|d| value(&target_type_name(&d.target_group.target_type))),
        ),
        (
            "Protocol : Port",
            f(&|d| {
                let tg = &d.target_group;
                match tg.port {
                    Some(port) => format!("{}: {port}", value(&tg.protocol)),
                    None => value(&tg.protocol),
                }
            }),
        ),
        (
            "Protocol version",
            f(&|d| value(&d.target_group.protocol_version)),
        ),
        ("VPC", f(&|d| value(&d.target_group.vpc_id))),
        (
            "IP address type",
            f(&|d| value(&ip_type_name(&d.target_group.ip_address_type))),
        ),
        (
            "Load balancer",
            f(&|d| {
                clean(&tgs::load_balancer_names(
                    &d.target_group.load_balancer_arns,
                ))
            }),
        ),
        ("Total targets", f(&counted(&[]))),
        ("Healthy", f(&counted(&["healthy"]))),
        (
            "Unhealthy",
            f(&counted(&["unhealthy", "unhealthy.draining"])),
        ),
        ("Unused", f(&counted(&["unused"]))),
        ("Initial", f(&counted(&["initial"]))),
        ("Draining", f(&counted(&["draining"]))),
    ]
}

pub(crate) fn health_check_fields(d: &TargetGroupDetail) -> Vec<(&'static str, String)> {
    let h = &d.target_group.health_check;
    let n = |v: Option<i32>, unit: &str| {
        v.map(|n| format!("{n} {unit}"))
            .unwrap_or_else(|| "-".into())
    };
    let mut fields = Vec::new();
    if h.enabled == Some(false) {
        fields.push(("Health checks", "Off".to_string()));
    }
    fields.extend([
        ("Protocol", value(&h.protocol)),
        ("Path", value(&h.path)),
        (
            "Port",
            match h.port.as_str() {
                "traffic-port" => "Traffic port".into(),
                port => value(port),
            },
        ),
        (
            "Healthy threshold",
            n(h.healthy_threshold, "consecutive health check successes"),
        ),
        (
            "Unhealthy threshold",
            n(h.unhealthy_threshold, "consecutive health check failures"),
        ),
        ("Timeout", n(h.timeout_seconds, "seconds")),
        ("Interval", n(h.interval_seconds, "seconds")),
        ("Success codes", value(&h.success_codes)),
    ]);
    fields
}

pub(crate) fn network_fields(l: &LoadBalancer) -> Vec<(&'static str, String)> {
    vec![
        ("VPC", value(&l.vpc_id)),
        ("IP address type", value(&ip_type_name(&l.ip_address_type))),
    ]
}

/// Console chart title: `RequestCount (Count)`.
pub(crate) fn chart_title(m: &Metric) -> String {
    format!("{} ({})", m.name, m.unit)
}

fn render_monitoring(frame: &mut Frame, section: &Section<Metric>, scroll: usize, area: Rect) {
    let metrics = match section {
        Ok(metrics) => metrics,
        Err(error) => {
            ui::loading::render(frame, area, ReadStatus::Failed, "Monitoring", Some(error));
            return;
        }
    };
    let titles: Vec<String> = metrics.iter().map(chart_title).collect();
    let charts: Vec<_> = metrics
        .iter()
        .zip(&titles)
        .map(|(m, title)| crate::ui::monitoring::MetricChart {
            title,
            data: &m.points,
            y_axis_label: m.unit,
            x_axis_label: None,
            ..Default::default()
        })
        .collect();
    crate::ui::monitoring::render_monitoring_tab(frame, area, &charts, &[], &[], &[], scroll);
}

/// A table tab: the read status, or a failed section after a good read.
#[allow(clippy::too_many_arguments)]
fn tab_table<T: 'static>(
    frame: &mut Frame,
    app: &App,
    table: &Table<T>,
    texts: Texts<'_>,
    status: ReadStatus,
    read_error: Option<&str>,
    section_error: Option<&str>,
    area: Rect,
) {
    let (status, error) = match section_error {
        Some(error) if status == ReadStatus::Ready => (ReadStatus::Failed, Some(error)),
        _ => (status, read_error),
    };
    spec_table::render(frame, app, table, texts, status, error, style, area);
}

/// The fixed Details pane, the ribbon, then the body area.
fn frame_detail(
    frame: &mut Frame,
    header: Summary<'_>,
    tabs: &[(&str, usize)],
    selected: usize,
    area: Rect,
) -> Rect {
    let chunks = Layout::vertical([
        Constraint::Length(header.height(area.width)),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(area);
    header.render_fill(frame, chunks[0]);
    ui::render_tabs(frame, chunks[1], tabs, &selected);
    chunks[2]
}

pub fn render_load_balancers(frame: &mut Frame, app: &App, area: Rect) {
    let state = &app.load_balancers_state;
    let Some(view) = &state.view else {
        spec_table::render(
            frame,
            app,
            &state.list,
            Texts {
                title: "Load balancers",
                placeholder: "Filter load balancers",
                no_items: "No load balancers in this Region.",
            },
            state.inventory.status(),
            state.inventory.error(),
            style,
            area,
        );
        return;
    };
    let status = view.read.status();
    let detail = view.read.detail();
    let header = summary(
        "Details",
        load_balancer_fields(&view.source, detail, status),
    );
    let lb_type = detail.map_or(view.source.lb_type.as_str(), |d| {
        d.load_balancer.lb_type.as_str()
    });
    let tabs: Vec<(&str, usize)> = lbs::Tab::ALL
        .iter()
        .enumerate()
        .map(|(i, t)| (t.name(lb_type), i))
        .collect();
    let selected = lbs::Tab::ALL
        .iter()
        .position(|t| *t == view.tab)
        .unwrap_or(0);
    let body = frame_detail(frame, header, &tabs, selected, area);
    let error = view.read.error();
    let section = view.section_error();
    let texts = |title, placeholder, no_items| Texts {
        title,
        placeholder,
        no_items,
    };
    match view.tab {
        lbs::Tab::Listeners => tab_table(
            frame,
            app,
            &view.listeners,
            texts(
                "Listeners",
                "Filter listeners",
                "No listeners. The load balancer accepts no traffic.",
            ),
            status,
            error,
            section,
            body,
        ),
        lbs::Tab::ResourceMap => tab_table(
            frame,
            app,
            &view.routes,
            texts(
                "Resource map",
                "Filter routes",
                "No listener forwards to a target group.",
            ),
            status,
            error,
            section,
            body,
        ),
        lbs::Tab::Security => tab_table(
            frame,
            app,
            &view.groups,
            texts(
                "Security groups",
                "Filter security groups",
                "No security groups are associated with this load balancer.",
            ),
            status,
            error,
            section,
            body,
        ),
        lbs::Tab::Tags => tab_table(
            frame,
            app,
            &view.tags,
            texts(
                "Tags",
                "Filter tags",
                "No tags are associated with this load balancer.",
            ),
            status,
            error,
            section,
            body,
        ),
        tab => {
            if ui::loading::render(frame, body, status, "load balancer", error) {
                return;
            }
            let d = detail.expect("Ready implies detail");
            match tab {
                lbs::Tab::NetworkMapping => {
                    let pane = summary("Network mapping", network_fields(&d.load_balancer));
                    let parts = Layout::vertical([
                        Constraint::Length(pane.height(body.width)),
                        Constraint::Min(0),
                    ])
                    .split(body);
                    pane.render(frame, parts[0]);
                    tab_table(
                        frame,
                        app,
                        &view.mappings,
                        texts("Mappings", "Filter mappings", "No mappings."),
                        status,
                        error,
                        None,
                        parts[1],
                    );
                }
                lbs::Tab::Monitoring => {
                    render_monitoring(frame, &d.monitoring, view.monitoring_scroll, body)
                }
                _ => match &d.attributes {
                    Ok(attributes) => {
                        summary("Attributes", super::attribute_fields(attributes))
                            .render(frame, body);
                    }
                    Err(error) => {
                        ui::loading::render(
                            frame,
                            body,
                            ReadStatus::Failed,
                            "Attributes",
                            Some(error),
                        );
                    }
                },
            }
        }
    }
}

pub fn render_target_groups(frame: &mut Frame, app: &App, area: Rect) {
    let state = &app.target_groups_state;
    let Some(view) = &state.view else {
        spec_table::render(
            frame,
            app,
            &state.list,
            Texts {
                title: "Target groups",
                placeholder: "Filter target groups",
                no_items: "No target groups in this Region.",
            },
            state.inventory.status(),
            state.inventory.error(),
            style,
            area,
        );
        return;
    };
    let status = view.read.status();
    let detail = view.read.detail();
    let header = summary("Details", target_group_fields(detail, status));
    let tabs: Vec<(&str, usize)> = tgs::Tab::ALL
        .iter()
        .enumerate()
        .map(|(i, t)| (t.name(), i))
        .collect();
    let selected = tgs::Tab::ALL
        .iter()
        .position(|t| *t == view.tab)
        .unwrap_or(0);
    let body = frame_detail(frame, header, &tabs, selected, area);
    let error = view.read.error();
    let section = view.section_error();
    match view.tab {
        tgs::Tab::Targets => tab_table(
            frame,
            app,
            &view.targets,
            Texts {
                title: "Registered targets",
                placeholder: "Filter targets",
                no_items: "No targets are registered in this target group.",
            },
            status,
            error,
            section,
            body,
        ),
        tgs::Tab::Tags => tab_table(
            frame,
            app,
            &view.tags,
            Texts {
                title: "Tags",
                placeholder: "Filter tags",
                no_items: "No tags are associated with this target group.",
            },
            status,
            error,
            section,
            body,
        ),
        tab => {
            if ui::loading::render(frame, body, status, "target group", error) {
                return;
            }
            let d = detail.expect("Ready implies detail");
            match tab {
                tgs::Tab::Monitoring => {
                    render_monitoring(frame, &d.monitoring, view.monitoring_scroll, body)
                }
                tgs::Tab::HealthChecks => {
                    summary("Health check settings", health_check_fields(d)).render(frame, body);
                }
                _ => match &d.attributes {
                    Ok(attributes) => {
                        summary("Attributes", super::attribute_fields(attributes))
                            .render(frame, body);
                    }
                    Err(error) => {
                        ui::loading::render(
                            frame,
                            body,
                            ReadStatus::Failed,
                            "Attributes",
                            Some(error),
                        );
                    }
                },
            }
        }
    }
}
