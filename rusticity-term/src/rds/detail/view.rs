//! Rendering for the DB instance detail page: the fixed Summary pane, the tab
//! ribbon, summary panes, Monitoring charts, the Logs & events tables and the
//! interactive table tabs.
use super::*;
use crate::{
    common::{render_pagination_text, SortDirection},
    table::TableWindow,
    ui::{
        self,
        filter::{render_filter_bar, FilterConfig, FilterControl},
        loading::render_table,
        table::{Column, TableConfig},
        Summary,
    },
};
use ratatui::prelude::*;
use rusticity_core::rds::detail::sections::Section;

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

fn number(n: Option<i32>, unit: &str) -> String {
    n.map(|n| format!("{n}{unit}"))
        .unwrap_or_else(|| "-".into())
}

fn yes_no(flag: Option<bool>) -> String {
    match flag {
        Some(true) => "Yes".into(),
        Some(false) => "No".into(),
        None => "-".into(),
    }
}

fn enabled(flag: Option<bool>) -> String {
    match flag {
        Some(true) => "Enabled".into(),
        Some(false) => "Disabled".into(),
        None => "-".into(),
    }
}

pub(crate) fn timestamp(text: &str) -> String {
    clean(&crate::common::format_iso_timestamp(text))
}

/// The fixed Summary pane. Identity comes from the entered row; every other
/// field comes only from the fresh read, with a placeholder until then.
pub(crate) fn summary_fields(
    source: &RdsInstance,
    detail: Option<&RdsInstanceDetail>,
    status: ReadStatus,
) -> Vec<(&'static str, String)> {
    let fresh = |field: fn(&RdsInstanceDetail) -> String| match (status, detail) {
        (ReadStatus::Ready, Some(d)) => field(d),
        (ReadStatus::Failed, _) => "unavailable".into(),
        _ => "…".into(),
    };
    // Console Summary order.
    vec![
        ("DB identifier", value(&source.identifier)),
        ("CPU", fresh(|d| value(&d.cpu))),
        ("Status", fresh(|d| value(&capitalize(&d.status)))),
        ("Class", fresh(|d| value(&d.instance_class))),
        ("Role", fresh(|d| value(&d.role))),
        ("Current activity", fresh(|d| value(&d.current_activity))),
        ("Engine", fresh(|d| value(&engine_name(&d.engine)))),
        ("Region & AZ", fresh(|d| value(&d.availability_zone))),
        ("Recommendations", fresh(|d| value(&d.recommendations))),
    ]
}

/// Console style: `configuring-enhanced-monitoring` → `Configuring-enhanced-monitoring`.
pub(crate) fn capitalize(status: &str) -> String {
    let mut chars = status.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The console's engine names; an unknown engine keeps its API name.
pub(crate) fn engine_name(engine: &str) -> String {
    match engine {
        "postgres" => "PostgreSQL",
        "mysql" => "MySQL",
        "mariadb" => "MariaDB",
        "aurora-postgresql" => "Aurora PostgreSQL",
        "aurora-mysql" | "aurora" => "Aurora MySQL",
        "oracle-ee" => "Oracle Enterprise Edition",
        "oracle-ee-cdb" => "Oracle Enterprise Edition (CDB)",
        "oracle-se2" => "Oracle Standard Edition Two",
        "oracle-se2-cdb" => "Oracle Standard Edition Two (CDB)",
        "sqlserver-ee" => "SQL Server Enterprise Edition",
        "sqlserver-se" => "SQL Server Standard Edition",
        "sqlserver-ex" => "SQL Server Express Edition",
        "sqlserver-web" => "SQL Server Web Edition",
        "db2-se" => "Db2 Standard Edition",
        "db2-ae" => "Db2 Advanced Edition",
        other => other,
    }
    .into()
}

pub(crate) fn connectivity_fields(d: &RdsInstanceDetail) -> Vec<(&'static str, String)> {
    vec![
        ("Endpoint", value(&d.endpoint)),
        ("Port", number(d.port, "")),
        ("Availability Zone", value(&d.availability_zone)),
        ("VPC", value(&d.vpc)),
        ("Subnet group", value(&d.subnet_group)),
        ("Subnets", list(&d.subnets)),
        ("Network type", value(&d.network_type)),
        ("VPC security groups", list(&d.security_groups)),
        ("Publicly accessible", yes_no(d.publicly_accessible)),
        ("Certificate authority", value(&d.certificate_authority)),
        (
            "Certificate authority date",
            timestamp(&d.certificate_expiry),
        ),
    ]
}

/// `in-sync` -> `In sync`.
fn state(text: &str) -> String {
    capitalize(&text.replace('-', " "))
}

/// `postgresql-license` -> `Postgresql License`.
fn title_words(text: &str) -> String {
    text.split('-')
        .map(capitalize)
        .collect::<Vec<_>>()
        .join(" ")
}

/// `default:postgres-18 (In sync)` per group.
fn groups(groups: &[(String, String)]) -> String {
    let parts: Vec<String> = groups
        .iter()
        .map(|(name, status)| match status.as_str() {
            "" => name.clone(),
            status => format!("{name} ({})", state(status)),
        })
        .collect();
    list(&parts)
}

fn not_enabled(flag: Option<bool>) -> String {
    match flag {
        Some(true) => "Enabled".into(),
        Some(false) => "Not enabled".into(),
        None => "-".into(),
    }
}

/// Console storage type names.
pub(crate) fn storage_type(raw: &str) -> String {
    match raw {
        "gp2" => "General Purpose SSD (gp2)".into(),
        "gp3" => "General Purpose SSD (gp3)".into(),
        "io1" => "Provisioned IOPS SSD (io1)".into(),
        "io2" => "Provisioned IOPS SSD (io2)".into(),
        "standard" => "Magnetic".into(),
        other => value(other),
    }
}

/// The Configuration tab: four bordered panes, in console order.
pub(crate) fn configuration_panes(
    d: &RdsInstanceDetail,
) -> Vec<(&'static str, Vec<(&'static str, String)>)> {
    let extended_support = match d.engine_lifecycle_support.as_str() {
        "open-source-rds-extended-support" => "Enabled".into(),
        "open-source-rds-extended-support-disabled" => "Disabled".into(),
        other => value(other),
    };
    let mut storage = vec![("Encryption", not_enabled(d.encrypted))];
    if d.encrypted == Some(true) {
        storage.push(("AWS KMS key", value(&d.kms_key_id)));
    }
    storage.extend([
        ("Storage type", storage_type(&d.storage_type)),
        ("Storage", number(d.storage_gib, " GiB")),
        ("Provisioned IOPS", number(d.iops, " IOPS")),
        ("Storage throughput", number(d.storage_throughput, " MiBps")),
        (
            "Storage autoscaling",
            match d.max_storage_gib {
                Some(max) => format!("Enabled (max {max} GiB)"),
                None => "Disabled".into(),
            },
        ),
        (
            "Storage file system configuration",
            match d.storage_upgrade_available {
                Some(true) => "Upgrade available".into(),
                Some(false) => "Current".into(),
                None => "-".into(),
            },
        ),
    ]);
    vec![
        (
            "Instance",
            vec![
                ("DB instance ID", value(&d.identifier)),
                ("Engine version", value(&d.engine_version)),
                ("RDS Extended Support", extended_support),
                ("DB name", value(&d.db_name)),
                ("License model", value(&title_words(&d.license_model))),
                ("Option groups", groups(&d.option_groups)),
                ("Amazon Resource Name (ARN)", value(&d.arn)),
                ("Resource ID", value(&d.resource_id)),
                ("Created time", timestamp(&d.created_time)),
                ("DB instance parameter group", groups(&d.parameter_groups)),
                ("Deletion protection", enabled(d.deletion_protection)),
                (
                    "Architecture settings",
                    if d.multi_tenant == Some(true) {
                        "Multi-tenant architecture".into()
                    } else {
                        "Non-multitenant architecture".into()
                    },
                ),
                ("Capacity reservation", value(&d.capacity_reservation)),
            ],
        ),
        (
            "Instance class",
            vec![
                ("Instance class", value(&d.instance_class)),
                ("vCPU", value(&d.vcpu)),
                ("RAM", value(&d.memory)),
                ("Master username", value(&d.master_username)),
                ("IAM DB authentication", not_enabled(d.iam_auth)),
                ("Multi-AZ", yes_no(d.multi_az)),
                ("Secondary Zone", value(&d.secondary_zone)),
                (
                    "Master credentials ARN",
                    match d.master_secret_status.as_str() {
                        _ if d.master_secret_arn.is_empty() => "-".into(),
                        "" => clean(&d.master_secret_arn),
                        status => format!("{} ({})", clean(&d.master_secret_arn), state(status)),
                    },
                ),
                (
                    "Master credentials KMS key",
                    value(&d.master_secret_kms_key),
                ),
            ],
        ),
        ("Primary storage", storage),
        (
            "Monitoring",
            vec![
                (
                    "Monitoring type",
                    match d.database_insights_mode.as_str() {
                        "" => "-".into(),
                        mode => format!("Database Insights - {}", capitalize(mode)),
                    },
                ),
                ("Database counter metrics", enabled(d.performance_insights)),
                (
                    "Enhanced Monitoring",
                    match d.monitoring_interval {
                        Some(n) if n > 0 => format!("Enabled ({n} seconds)"),
                        Some(_) => "Disabled".into(),
                        None => "-".into(),
                    },
                ),
            ],
        ),
    ]
}

/// One Data migrations row, in `Tab::MIGRATION_COLUMNS` order.
pub(crate) fn migration_cells(
    m: &rusticity_core::rds::detail::sections::RdsDataMigration,
) -> Vec<String> {
    let count = |n: Option<i32>| n.map(|n| n.to_string()).unwrap_or_default();
    vec![
        m.name.clone(),
        capitalize(&m.status.to_lowercase()),
        m.progress.clone(),
        match m.migration_type.as_str() {
            "full-load" => "Full load".into(),
            "cdc" => "CDC".into(),
            "full-load-and-cdc" => "Full load and CDC".into(),
            other => other.into(),
        },
        m.server_name.clone(),
        m.port.map(|p| p.to_string()).unwrap_or_default(),
        match m.cloudwatch_logs {
            Some(true) => "Enabled".into(),
            Some(false) => "Disabled".into(),
            None => String::new(),
        },
        timestamp(&m.created_time),
        timestamp(&m.started_time),
        timestamp(&m.stopped_time),
        count(m.tables_loaded),
        count(m.tables_loading),
        count(m.tables_queued),
        count(m.tables_errored),
        m.elapsed_millis.map(elapsed).unwrap_or_default(),
        m.arn.clone(),
    ]
}

/// `3725000` ms -> `1h 2m 5s`.
pub(crate) fn elapsed(millis: i64) -> String {
    let total = millis.max(0) / 1000;
    let (h, m, s) = (total / 3600, total / 60 % 60, total % 60);
    match (h, m) {
        (0, 0) => format!("{s}s"),
        (0, _) => format!("{m}m {s}s"),
        _ => format!("{h}h {m}m {s}s"),
    }
}

pub(crate) fn maintenance_fields(d: &RdsInstanceDetail) -> Vec<(&'static str, String)> {
    vec![
        (
            "Auto minor version upgrade",
            enabled(d.auto_minor_version_upgrade),
        ),
        ("Maintenance window", value(&d.maintenance_window)),
        ("Pending modifications", value(&d.pending_changes)),
        (
            "Automated backups",
            match d.backup_retention_days {
                Some(0) => "Disabled".into(),
                Some(days) => format!("Enabled ({days} days)"),
                None => "-".into(),
            },
        ),
        ("Backup window", value(&d.backup_window)),
        ("Copy tags to snapshots", enabled(d.copy_tags_to_snapshot)),
        ("Latest restore time", timestamp(&d.latest_restorable_time)),
        ("Backup target", value(&d.backup_target)),
    ]
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

/// Enabled values green, Disabled and Not enabled yellow, as elsewhere.
pub(crate) fn state_color(text: &str) -> Option<Color> {
    if text.starts_with("Enabled") {
        Some(Color::Green)
    } else if text.starts_with("Disabled") || text == "Not enabled" {
        Some(Color::Yellow)
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

pub fn render(frame: &mut Frame, app: &App, area: Rect) -> bool {
    let state = &app.rds_state.detail;
    let (Some(source), Some(view)) = (state.source.as_ref(), state.view()) else {
        return false;
    };
    let loading = state.loading_status();
    let status = view.status(loading);
    let header = summary("Summary", summary_fields(source, view.detail(), status));
    let tabs: Vec<(&str, Tab)> = Tab::ALL.iter().map(|t| (t.name(), *t)).collect();
    let chunks = Layout::vertical([
        Constraint::Length(header.height(area.width)),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(area);
    header.render_fill(frame, chunks[0]);
    ui::render_tabs(frame, chunks[1], &tabs, &view.tab);
    let mut body = chunks[2];
    if view.tab == Tab::Connectivity {
        // The summary pane needs the fresh read; the tables follow it.
        if ui::loading::render(frame, body, status, "database details", view.error()) {
            return true;
        }
        let pane = summary(
            Tab::Connectivity.name(),
            connectivity_fields(view.detail().expect("Ready implies detail")),
        );
        let parts = Layout::vertical([
            Constraint::Length(pane.height(body.width)),
            Constraint::Min(0),
        ])
        .split(body);
        pane.render(frame, parts[0]);
        body = parts[1];
    }
    if view.tab.is_table() {
        render_table_tab(frame, app, view, status, body);
        return true;
    }
    if ui::loading::render(frame, body, status, "database details", view.error()) {
        return true;
    }
    let detail = view.detail().expect("Ready implies detail");
    let fields = match view.tab {
        Tab::Configuration => {
            let panes: Vec<_> = configuration_panes(detail)
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
            return true;
        }
        Tab::Maintenance => maintenance_fields(detail),
        Tab::Monitoring => {
            render_monitoring(frame, view, &detail.sections.monitoring, body);
            return true;
        }
        _ => unreachable!("handled above"),
    };
    summary(view.tab.name(), fields).render(frame, body);
    true
}

/// Console title: `CPUUtilization (Percent)`.
pub(crate) fn chart_title(name: &str, unit: &str) -> String {
    format!("{name} ({unit})")
}

fn render_monitoring(
    frame: &mut Frame,
    view: &View,
    section: &Section<rusticity_core::rds::detail::sections::RdsMetric>,
    area: Rect,
) {
    let metrics = match section {
        Ok(metrics) => metrics,
        Err(_) => {
            let error = view.monitoring_error();
            ui::loading::render(frame, area, ReadStatus::Failed, "Monitoring", error);
            return;
        }
    };
    let titles: Vec<String> = metrics
        .iter()
        .map(|m| chart_title(m.name, m.unit))
        .collect();
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
    crate::ui::monitoring::render_monitoring_tab(
        frame,
        area,
        &charts,
        &[],
        &[],
        &[],
        view.monitoring_scroll,
    );
}

pub(crate) fn size(bytes: Option<i64>) -> String {
    bytes
        .map(crate::common::format_bytes)
        .unwrap_or_else(|| "-".into())
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
        Slot::Tags => ("Filter tags", "No tags are associated with this database."),
        Slot::ZeroEtl => (
            "Filter zero-ETL integrations",
            "No zero-ETL integrations use this database as their source.",
        ),
        Slot::DataMigrations => (
            "Filter data migrations",
            "No homogeneous data migrations use this database.",
        ),
        Slot::Recommendations => (
            "Filter recommendations",
            "No recommendations for this database.",
        ),
        Slot::Alarms => ("Filter CloudWatch alarms", "No alarms watch this database."),
        Slot::Events => ("Filter recent events", "No events in the last 7 days."),
        Slot::Logs => ("Filter logs", "No log files."),
        Slot::Compute => (
            "Filter connected compute resources",
            "No compute resources are connected to this database.",
        ),
        Slot::SecurityGroupRules => ("Filter security group rules", "No security group rules."),
        Slot::Replication => ("Filter replication", "No replication."),
        Slot::IamRoles => (
            "Filter IAM roles",
            "No IAM roles are associated with this database.",
        ),
    }
}

/// A table tab: its filter bar, then its table. Logs & events adds a second
/// ribbon (CloudWatch alarms, Recent events, Logs) and shows one table at a
/// time, each with its own filter; `f`/`F` move along that ribbon.
fn render_table_tab(frame: &mut Frame, app: &App, view: &View, status: ReadStatus, area: Rect) {
    let focused = view.slot().expect("table tab");
    let area = if let Some((_, slots)) = view.tab.stack() {
        let parts = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area);
        let ribbon: Vec<(&str, Slot)> = slots.iter().map(|s| (s.name(), *s)).collect();
        ui::render_tabs(frame, parts[0], &ribbon, &focused);
        parts[1]
    } else {
        area
    };
    let table = view.table(focused);
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
    render_slot(frame, app, view, focused, status, parts[1]);
}

fn render_slot(
    frame: &mut Frame,
    app: &App,
    view: &View,
    slot: Slot,
    status: ReadStatus,
    area: Rect,
) {
    let table = view.table(slot);
    // A failed section fails only its own table.
    let (status, error) = match view.section_error(slot) {
        Some(error) if status == ReadStatus::Ready => (ReadStatus::Failed, Some(error)),
        _ => (status, view.error()),
    };
    let prepared = table
        .rows
        .prepare_view(table.filtered(), TableWindow::SelectedPage);
    let spec = slot.columns();
    let columns = columns(spec, &table.visible_column_ids);
    render_table(
        frame,
        TableConfig {
            items: prepared.items,
            selected_index: prepared.selected_index,
            expanded_index: prepared.expanded_index,
            columns: &columns,
            sort_column: "",
            sort_direction: SortDirection::Asc,
            title: ui::format_title(&format!("{} ({})", slot.name(), prepared.total_items)),
            area,
            get_expanded_content: Some(Box::new(move |row: &Row| {
                spec.iter()
                    .zip(&row.cells)
                    .map(|((name, _), cell)| (format!("{name}: {}", value(cell)), Style::default()))
                    .collect()
            })),
            is_active: app.mode == Mode::Normal,
        },
        status,
        slot.name(),
        error,
        ui::loading::EmptyState {
            no_items: texts(slot).1,
            has_unfiltered_items: !table.rows.items.is_empty(),
        },
    );
}
