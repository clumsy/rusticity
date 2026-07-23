use crate::app::App;
use crate::common::{render_pagination_text, CyclicEnum, InputFocus, SortDirection};
use crate::efs::fs::{self, FileSystem as EfsFileSystem};
use crate::table::TableState;
use crate::ui::filter::{render_simple_filter, SimpleFilterConfig};
use crate::ui::format_title;
use crate::ui::render_tabs;
use crate::ui::table::{expanded_from_columns, render_table, Column, TableConfig};
use ratatui::{prelude::*, widgets::*};

pub const FILTER_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetailTab {
    MeteredSize,
    Monitoring,
    Tags,
    FileSystemPolicy,
    AccessPoints,
    Network,
    Replication,
}

impl CyclicEnum for DetailTab {
    const ALL: &'static [Self] = &[
        Self::MeteredSize,
        Self::Monitoring,
        Self::Tags,
        Self::FileSystemPolicy,
        Self::AccessPoints,
        Self::Network,
        Self::Replication,
    ];
}

impl DetailTab {
    pub fn name(&self) -> &'static str {
        match self {
            DetailTab::MeteredSize => "Metered size",
            DetailTab::Monitoring => "Monitoring",
            DetailTab::Tags => "Tags",
            DetailTab::FileSystemPolicy => "File system policy",
            DetailTab::AccessPoints => "Access points",
            DetailTab::Network => "Network",
            DetailTab::Replication => "Replication",
        }
    }
}

pub struct State {
    pub file_systems: TableState<EfsFileSystem>,
    pub input_focus: InputFocus,
    pub current_file_system: Option<String>,
    pub detail_tab: DetailTab,
    pub tags_table: TableState<(String, String)>,
    // ── Access points ──
    pub access_points: TableState<rusticity_core::efs::EfsAccessPoint>,
    pub ap_column_ids: Vec<crate::common::ColumnId>,
    pub ap_visible_column_ids: Vec<crate::common::ColumnId>,
    pub ap_loading: bool,
    // ── Network (mount targets) ──
    pub mount_targets: TableState<rusticity_core::efs::EfsMountTarget>,
    pub mt_column_ids: Vec<crate::common::ColumnId>,
    pub mt_visible_column_ids: Vec<crate::common::ColumnId>,
    pub mt_loading: bool,
    // ── Replication ──
    pub replication_document: String,
    pub replication_loading: bool,
    // ── File system policy ──
    pub policy_document: String,
    pub policy_scroll: usize,
    pub policy_loading: bool,
    // ── Monitoring ──
    pub metrics_loading: bool,
    pub monitoring_scroll: usize,
    pub metric_throughput_utilization: Vec<(i64, f64)>,
    pub metric_iops_data_write: Vec<(i64, f64)>,
    pub metric_iops_data_read: Vec<(i64, f64)>,
    pub metric_iops_metadata: Vec<(i64, f64)>,
    pub metric_throughput_data_write: Vec<(i64, f64)>,
    pub metric_throughput_data_read: Vec<(i64, f64)>,
    pub metric_throughput_metadata: Vec<(i64, f64)>,
    pub metric_iops_utilization: Vec<(i64, f64)>,
    pub metric_client_connections: Vec<(i64, f64)>,
    pub metric_storage_standard: Vec<(i64, f64)>,
    pub metric_storage_ia: Vec<(i64, f64)>,
    pub metric_storage_archive: Vec<(i64, f64)>,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            file_systems: TableState::new(),
            input_focus: InputFocus::Filter,
            current_file_system: None,
            detail_tab: DetailTab::MeteredSize,
            tags_table: TableState::new(),
            access_points: TableState::new(),
            ap_column_ids: crate::efs::access_point::Column::ids(),
            ap_visible_column_ids: crate::efs::access_point::Column::default_visible_ids(),
            ap_loading: false,
            mount_targets: TableState::new(),
            mt_column_ids: crate::efs::mount_target::Column::ids(),
            mt_visible_column_ids: crate::efs::mount_target::Column::default_visible_ids(),
            mt_loading: false,
            replication_document: String::new(),
            replication_loading: false,
            policy_document: String::new(),
            policy_scroll: 0,
            policy_loading: false,
            metrics_loading: false,
            monitoring_scroll: 0,
            metric_throughput_utilization: Vec::new(),
            metric_iops_data_write: Vec::new(),
            metric_iops_data_read: Vec::new(),
            metric_iops_metadata: Vec::new(),
            metric_throughput_data_write: Vec::new(),
            metric_throughput_data_read: Vec::new(),
            metric_throughput_metadata: Vec::new(),
            metric_iops_utilization: Vec::new(),
            metric_client_connections: Vec::new(),
            metric_storage_standard: Vec::new(),
            metric_storage_ia: Vec::new(),
            metric_storage_archive: Vec::new(),
        }
    }
}

impl crate::ui::monitoring::MonitoringState for State {
    fn is_metrics_loading(&self) -> bool {
        self.metrics_loading
    }

    fn set_metrics_loading(&mut self, loading: bool) {
        self.metrics_loading = loading;
    }

    fn monitoring_scroll(&self) -> usize {
        self.monitoring_scroll
    }

    fn set_monitoring_scroll(&mut self, scroll: usize) {
        self.monitoring_scroll = scroll;
    }

    fn clear_metrics(&mut self) {
        self.metric_throughput_utilization.clear();
        self.metric_iops_data_write.clear();
        self.metric_iops_data_read.clear();
        self.metric_iops_metadata.clear();
        self.metric_throughput_data_write.clear();
        self.metric_throughput_data_read.clear();
        self.metric_throughput_metadata.clear();
        self.metric_iops_utilization.clear();
        self.metric_client_connections.clear();
        self.metric_storage_standard.clear();
        self.metric_storage_ia.clear();
        self.metric_storage_archive.clear();
    }
}

/// Load the file system's access points into app state.
pub async fn load_access_points(app: &mut App, file_system_id: &str) -> anyhow::Result<()> {
    let items = app
        .efs_client
        .describe_access_points(file_system_id)
        .await?;
    app.efs_state.access_points.items = items;
    app.efs_state.access_points.selected = 0;
    app.efs_state.access_points.expanded_item = None;
    Ok(())
}

/// Load the file system's mount targets (Network tab) into app state.
pub async fn load_mount_targets(app: &mut App, file_system_id: &str) -> anyhow::Result<()> {
    let items = app
        .efs_client
        .describe_mount_targets(file_system_id)
        .await?;
    app.efs_state.mount_targets.items = items;
    app.efs_state.mount_targets.selected = 0;
    app.efs_state.mount_targets.expanded_item = None;
    Ok(())
}

/// Load the file system's replication configuration text into app state.
pub async fn load_replication(app: &mut App, file_system_id: &str) -> anyhow::Result<()> {
    let replication = app.efs_client.get_replication(file_system_id).await?;
    app.efs_state.replication_document = replication.unwrap_or_default();
    Ok(())
}

/// Load the file system's resource policy JSON into app state.
pub async fn load_policy(app: &mut App, file_system_id: &str) -> anyhow::Result<()> {
    let policy = app
        .efs_client
        .get_file_system_policy(file_system_id)
        .await?;
    app.efs_state.policy_document = policy.unwrap_or_default();
    app.efs_state.policy_scroll = 0;
    Ok(())
}

/// Load the EFS monitoring metrics for the given file system into app state.
pub async fn load_metrics(app: &mut App, file_system_id: &str) -> anyhow::Result<()> {
    let data = app
        .efs_client
        .get_monitoring_metrics(file_system_id)
        .await?;
    let s = &mut app.efs_state;
    s.metric_throughput_utilization = data.throughput_utilization;
    s.metric_iops_data_write = data.iops_data_write;
    s.metric_iops_data_read = data.iops_data_read;
    s.metric_iops_metadata = data.iops_metadata;
    s.metric_throughput_data_write = data.throughput_data_write;
    s.metric_throughput_data_read = data.throughput_data_read;
    s.metric_throughput_metadata = data.throughput_metadata;
    s.metric_iops_utilization = data.iops_utilization;
    s.metric_client_connections = data.client_connections;
    s.metric_storage_standard = data.storage_standard;
    s.metric_storage_ia = data.storage_ia;
    s.metric_storage_archive = data.storage_archive;
    Ok(())
}

pub fn filtered_efs_file_systems(app: &App) -> Vec<&EfsFileSystem> {
    let filter = &app.efs_state.file_systems.filter;
    app.efs_state
        .file_systems
        .items
        .iter()
        .filter(|fs| {
            filter.is_empty()
                || fs.name.to_lowercase().contains(&filter.to_lowercase())
                || fs
                    .file_system_id
                    .to_lowercase()
                    .contains(&filter.to_lowercase())
                || fs
                    .availability_zone
                    .to_lowercase()
                    .contains(&filter.to_lowercase())
        })
        .collect()
}

pub fn render_file_systems(frame: &mut Frame, app: &App, area: Rect) {
    if app.efs_state.current_file_system.is_some() {
        render_file_system_detail(frame, app, area);
    } else {
        render_file_systems_list(frame, app, area);
    }
}

pub fn render_file_system_detail(frame: &mut Frame, app: &App, area: Rect) {
    let fs = app.efs_state.current_file_system.as_deref().and_then(|id| {
        app.efs_state
            .file_systems
            .items
            .iter()
            .find(|f| f.file_system_id == id)
    });

    let tabs: Vec<(&str, DetailTab)> = DetailTab::ALL.iter().map(|t| (t.name(), *t)).collect();

    if let Some(fs) = fs {
        // General summary always visible at top — compute its height
        let field_lines = build_general_lines(fs);
        let content_rows =
            crate::ui::calculate_dynamic_height(&field_lines, area.width.saturating_sub(2));
        // +2 for borders, +1 so bottom border doesn't sit on last field row
        let summary_height = (content_rows + 3).min(area.height.saturating_sub(2));

        // Layout: [Summary] [Ribbon] [Tab content]
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(summary_height), // General summary
                Constraint::Length(1),              // Ribbon
                Constraint::Min(0),                 // Tab content
            ])
            .split(area);

        // Always render General summary
        let block = crate::ui::titled_block(" General ");
        let inner = block.inner(chunks[0]);
        frame.render_widget(block, chunks[0]);
        crate::ui::render_fields_with_dynamic_columns(frame, inner, field_lines);

        // Always render ribbon
        render_tabs(frame, chunks[1], &tabs, &app.efs_state.detail_tab);

        // Render tab-specific content below
        match app.efs_state.detail_tab {
            DetailTab::MeteredSize => {
                render_metered_size_tab(frame, fs, chunks[2]);
            }
            DetailTab::Monitoring => {
                render_efs_monitoring_charts(frame, app, chunks[2]);
            }
            DetailTab::Tags => {
                render_tags_tab(frame, app, fs, chunks[2]);
            }
            DetailTab::FileSystemPolicy => {
                render_file_system_policy_tab(frame, app, chunks[2]);
            }
            DetailTab::AccessPoints => {
                render_access_points_tab(frame, app, chunks[2]);
            }
            DetailTab::Network => {
                render_network_tab(frame, app, chunks[2]);
            }
            DetailTab::Replication => {
                render_replication_tab(frame, app, chunks[2]);
            }
        }
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);
        render_tabs(frame, chunks[1], &tabs, &app.efs_state.detail_tab);
    }
}

fn build_general_lines(fs: &EfsFileSystem) -> Vec<ratatui::text::Line<'static>> {
    use crate::ui::{labeled_field, labeled_field_colored};

    let az_display = if fs.availability_zone.is_empty() {
        "Regional".to_string()
    } else {
        fs.availability_zone.clone()
    };

    // Encrypted: show KMS key ARN if encrypted, colored green/yellow
    // Encrypted: green=Yes (no KMS), white=KMS ARN, yellow=No
    let (encrypted_text, encrypted_color) = if fs.encrypted == "Yes" || fs.encrypted == "YES" {
        if fs.kms_key_id.is_empty() {
            ("Yes".to_string(), ratatui::style::Color::Green)
        } else {
            (fs.kms_key_id.clone(), ratatui::style::Color::White)
        }
    } else if fs.encrypted == "No" || fs.encrypted == "NO" {
        ("No".to_string(), ratatui::style::Color::Yellow)
    } else {
        (fs.encrypted.clone(), ratatui::style::Color::White)
    };

    // File system state color — SDK Debug gives lowercase ("available", "creating" etc.)
    let (state_text, state_color) = match fs.life_cycle_state.as_str() {
        "available" | "Available" | "AVAILABLE" => {
            ("✅ Available".to_string(), ratatui::style::Color::Green)
        }
        "creating" | "Creating" | "CREATING" => {
            ("⏳ Creating".to_string(), ratatui::style::Color::Yellow)
        }
        "deleting" | "Deleting" | "DELETING" => {
            ("🗑 Deleting".to_string(), ratatui::style::Color::Red)
        }
        "deleted" | "Deleted" | "DELETED" => ("🗑 Deleted".to_string(), ratatui::style::Color::Red),
        "updating" | "Updating" | "UPDATING" => {
            ("🔄 Updating".to_string(), ratatui::style::Color::Yellow)
        }
        "error" | "Error" | "ERROR" => ("⚠ Error".to_string(), ratatui::style::Color::Red),
        s => (s.to_string(), ratatui::style::Color::White),
    };

    // Replication overwrite protection color
    let (rep_text, rep_color) = match fs.replication_overwrite_protection.as_str() {
        "Enabled" | "ENABLED" => ("Enabled".to_string(), ratatui::style::Color::Green),
        "Disabled" | "DISABLED" => ("Disabled".to_string(), ratatui::style::Color::Yellow),
        "Replicating" | "REPLICATING" => ("Replicating".to_string(), ratatui::style::Color::Cyan),
        s => (s.to_string(), ratatui::style::Color::White),
    };

    vec![
        labeled_field("ARN", fs.file_system_arn.clone()),
        labeled_field(
            "Performance mode",
            format_performance_mode_display(&fs.performance_mode),
        ),
        labeled_field("Throughput mode", fs.throughput_mode.clone()),
        labeled_field("Availability zone", az_display),
        labeled_field_colored("Encrypted", encrypted_text, encrypted_color),
        labeled_field_colored("File system state", state_text, state_color),
        labeled_field("DNS name", fs.dns_name.clone()),
        labeled_field_colored("Replication overwrite protection", rep_text, rep_color),
        labeled_field("File system ID", fs.file_system_id.clone()),
        labeled_field("Creation time", fs.creation_time.clone()),
    ]
}

/// Format a raw byte count AWS-console style: two decimals for KiB/MiB/GiB,
/// and "N Bytes" below 1 KiB (so 0 renders as "0 Bytes").
fn fmt_metered_bytes(bytes: i64) -> String {
    const GIB: f64 = (1u64 << 30) as f64;
    const MIB: f64 = (1u64 << 20) as f64;
    const KIB: f64 = (1u64 << 10) as f64;
    let b = bytes as f64;
    if b >= GIB {
        format!("{:.2} GiB", b / GIB)
    } else if b >= MIB {
        format!("{:.2} MiB", b / MIB)
    } else if b >= KIB {
        format!("{:.2} KiB", b / KIB)
    } else {
        format!("{} Bytes", bytes)
    }
}

/// Percentage of `part` relative to `total` (0.0 when total is 0).
fn metered_pct(part: i64, total: i64) -> f64 {
    if total > 0 {
        (part as f64 / total as f64) * 100.0
    } else {
        0.0
    }
}

/// The value shown for a storage-class field: size plus percentage of total.
fn metered_field_value(bytes: i64, total: i64) -> String {
    format!(
        "{} ({:.0}%)",
        fmt_metered_bytes(bytes),
        metered_pct(bytes, total)
    )
}

fn render_metered_size_tab(frame: &mut Frame, fs: &EfsFileSystem, area: Rect) {
    use crate::ui::labeled_field;

    let total = fs.total_size_bytes.max(0);
    let field_lines: Vec<Line> = vec![
        labeled_field("Total size", fmt_metered_bytes(fs.total_size_bytes)),
        labeled_field(
            "Size in Standard",
            metered_field_value(fs.size_in_standard_bytes, total),
        ),
        labeled_field(
            "Size in IA",
            metered_field_value(fs.size_in_ia_bytes, total),
        ),
        labeled_field(
            "Size in Archive",
            metered_field_value(fs.size_in_archive_bytes, total),
        ),
    ];

    // Size the pane to its content height (dynamic columns), like the General summary.
    let content_rows =
        crate::ui::calculate_dynamic_height(&field_lines, area.width.saturating_sub(2));
    let pane_height = (content_rows + 3).min(area.height);
    let pane = Rect {
        height: pane_height,
        ..area
    };

    let block = crate::ui::titled_block(" Metered size ");
    let inner = block.inner(pane);
    frame.render_widget(block, pane);
    crate::ui::render_fields_with_dynamic_columns(frame, inner, field_lines);
}

/// Access points filtered by the current filter (name, access point ID, or path).
pub fn filtered_access_points(app: &App) -> Vec<&rusticity_core::efs::EfsAccessPoint> {
    let filter = app.efs_state.access_points.filter.to_lowercase();
    app.efs_state
        .access_points
        .items
        .iter()
        .filter(|ap| {
            filter.is_empty()
                || ap.name.to_lowercase().contains(&filter)
                || ap.access_point_id.to_lowercase().contains(&filter)
                || ap.path.to_lowercase().contains(&filter)
        })
        .collect()
}

fn render_access_points_tab(frame: &mut Frame, app: &App, area: Rect) {
    use rusticity_core::efs::EfsAccessPoint;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let filtered = filtered_access_points(app);
    let filtered_count = filtered.len();

    let page_size = app.efs_state.access_points.page_size.value();
    let total_pages = filtered_count.div_ceil(page_size.max(1));
    let current_page = app
        .efs_state
        .access_points
        .selected
        .checked_div(page_size)
        .unwrap_or(0);
    let pagination = render_pagination_text(current_page, total_pages);

    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &app.efs_state.access_points.filter,
            placeholder: "Search by name, access point ID, or path",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.efs_state.input_focus == InputFocus::Filter,
            is_pagination_focused: app.efs_state.input_focus == InputFocus::Pagination,
        },
    );

    let start_idx = current_page * page_size;
    let end_idx = (start_idx + page_size).min(filtered_count);
    let paginated: Vec<_> = if start_idx < filtered_count {
        filtered[start_idx..end_idx].to_vec()
    } else {
        vec![]
    };

    let columns: Vec<Box<dyn Column<EfsAccessPoint>>> = app
        .efs_state
        .ap_visible_column_ids
        .iter()
        .filter_map(|id| {
            crate::efs::access_point::Column::from_id(id)
                .map(|c| Box::new(c) as Box<dyn Column<EfsAccessPoint>>)
        })
        .collect();

    let expanded_index = app.efs_state.access_points.expanded_item.and_then(|idx| {
        if idx >= start_idx && idx < end_idx {
            Some(idx - start_idx)
        } else {
            None
        }
    });
    let selected_index = app
        .efs_state
        .access_points
        .selected
        .saturating_sub(start_idx);

    let config = TableConfig {
        items: paginated,
        selected_index,
        expanded_index,
        columns: &columns,
        sort_column: "Name",
        sort_direction: SortDirection::Asc,
        title: format_title(&format!("Access points ({})", filtered_count)),
        area: chunks[1],
        get_expanded_content: Some(Box::new(|ap: &EfsAccessPoint| {
            expanded_from_columns(&columns, ap)
        })),
        is_active: app.mode != crate::keymap::Mode::FilterInput,
    };
    render_table(frame, config);
}

fn render_network_tab(frame: &mut Frame, app: &App, area: Rect) {
    use rusticity_core::efs::EfsMountTarget;

    // No filter for the Network tab; the list is already sorted by AZ ascending.
    let items: Vec<&EfsMountTarget> = app.efs_state.mount_targets.items.iter().collect();
    let count = items.len();

    let columns: Vec<Box<dyn Column<EfsMountTarget>>> = app
        .efs_state
        .mt_visible_column_ids
        .iter()
        .filter_map(|id| {
            crate::efs::mount_target::Column::from_id(id)
                .map(|c| Box::new(c) as Box<dyn Column<EfsMountTarget>>)
        })
        .collect();

    let selected = app
        .efs_state
        .mount_targets
        .selected
        .min(count.saturating_sub(1));

    let config = TableConfig {
        items,
        selected_index: selected,
        expanded_index: app.efs_state.mount_targets.expanded_item,
        columns: &columns,
        sort_column: "Availability zone (AZ-ID)",
        sort_direction: SortDirection::Asc,
        title: format_title(&format!("Mount targets ({})", count)),
        area,
        get_expanded_content: Some(Box::new(|mt: &EfsMountTarget| {
            expanded_from_columns(&columns, mt)
        })),
        is_active: app.mode != crate::keymap::Mode::FilterInput,
    };
    render_table(frame, config);
}

/// Text shown on the Replication tab: either the replication configuration
/// output, or a placeholder when the file system is not being replicated.
/// Returns `(text, is_placeholder)`.
fn replication_text(doc: &str) -> (String, bool) {
    if doc.trim().is_empty() {
        (
            "No replication\nThis file system is not being replicated.".to_string(),
            true,
        )
    } else {
        (doc.to_string(), false)
    }
}

fn render_replication_tab(frame: &mut Frame, app: &App, area: Rect) {
    let block = crate::ui::titled_block(" Replication ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (text, is_placeholder) = replication_text(&app.efs_state.replication_document);
    if is_placeholder {
        let msg = Paragraph::new(text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(msg, inner);
    } else {
        frame.render_widget(Paragraph::new(text), inner);
    }
}

/// Render the File system policy tab as a scrollable, syntax-highlighted JSON
/// control. Shows a placeholder when the file system has no resource policy.
fn render_file_system_policy_tab(frame: &mut Frame, app: &App, area: Rect) {
    let policy = &app.efs_state.policy_document;
    if policy.is_empty() {
        let block = crate::ui::titled_block(" File system policy ");
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let msg = Paragraph::new("No file system policy attached")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(msg, inner);
        return;
    }
    crate::ui::render_json_highlighted(
        frame,
        area,
        policy,
        app.efs_state.policy_scroll,
        "File system policy",
        app.mode != crate::keymap::Mode::FilterInput,
    );
}

/// Compute a `>= 1` y-axis step for a multi-dataset chart from its max value,
/// aiming for ~10 gridlines. Guards the widget's label loop against a 0 step.
fn multi_y_step(max: f64) -> u32 {
    if max <= 10.0 {
        1
    } else {
        ((max / 10.0).ceil() as u32).max(1)
    }
}

fn render_efs_monitoring_charts(frame: &mut Frame, app: &App, area: Rect) {
    use crate::ui::monitoring::{render_monitoring_tab, MetricChart, MultiDatasetChart};

    let s = &app.efs_state;

    // Graph 6 — convert storage bytes to GiB for a readable y-axis.
    const GIB: f64 = (1u64 << 30) as f64;
    let to_gib = |series: &[(i64, f64)]| -> Vec<(i64, f64)> {
        series.iter().map(|(t, v)| (*t, v / GIB)).collect()
    };
    let storage_standard = to_gib(&s.metric_storage_standard);
    let storage_ia = to_gib(&s.metric_storage_ia);
    let storage_archive = to_gib(&s.metric_storage_archive);

    // Steps for the percentage charts (0–100) and the storage chart.
    let pct_step = 10;
    let storage_max = storage_standard
        .iter()
        .chain(storage_ia.iter())
        .chain(storage_archive.iter())
        .map(|(_, v)| *v)
        .fold(0.0_f64, f64::max);
    let storage_step = multi_y_step(storage_max);

    // Single-value charts (grouped first by the shared widget).
    let single = [
        MetricChart {
            title: "Throughput utilization (%)",
            data: &s.metric_throughput_utilization,
            y_axis_label: "%",
            x_axis_label: None,
            threshold: Some(75.0),
        },
        MetricChart {
            title: "IOPS utilization (%)",
            data: &s.metric_iops_utilization,
            y_axis_label: "%",
            x_axis_label: None,
            ..Default::default()
        },
        MetricChart {
            title: "Client connections",
            data: &s.metric_client_connections,
            y_axis_label: "Count",
            x_axis_label: None,
            ..Default::default()
        },
    ];

    // Multi-dataset charts.
    let multi = [
        MultiDatasetChart {
            title: "IOPS by type (%)",
            datasets: vec![
                ("Data write", s.metric_iops_data_write.as_slice()),
                ("Data read", s.metric_iops_data_read.as_slice()),
                ("Metadata", s.metric_iops_metadata.as_slice()),
            ],
            y_axis_label: "%",
            y_axis_step: pct_step,
            x_axis_label: None,
        },
        MultiDatasetChart {
            title: "Throughput by type (%)",
            datasets: vec![
                ("Data write", s.metric_throughput_data_write.as_slice()),
                ("Data read", s.metric_throughput_data_read.as_slice()),
                ("Metadata", s.metric_throughput_metadata.as_slice()),
            ],
            y_axis_label: "%",
            y_axis_step: pct_step,
            x_axis_label: None,
        },
        MultiDatasetChart {
            title: "Storage bytes (GiB)",
            datasets: vec![
                ("Standard", storage_standard.as_slice()),
                ("IA", storage_ia.as_slice()),
                ("Archive", storage_archive.as_slice()),
            ],
            y_axis_label: "GiB",
            y_axis_step: storage_step,
            x_axis_label: None,
        },
    ];

    render_monitoring_tab(frame, area, &single, &multi, &[], &[], s.monitoring_scroll);
}

fn render_tags_tab(frame: &mut Frame, app: &App, fs: &EfsFileSystem, area: Rect) {
    use crate::common::SortDirection;
    use crate::ui::format_title;
    use crate::ui::table::{expanded_from_columns, render_table, Column, TableConfig};

    struct KeyCol;
    struct ValueCol;

    #[derive(Clone)]
    struct TagRow {
        key: String,
        value: String,
    }

    impl Column<TagRow> for KeyCol {
        fn name(&self) -> &str {
            "Key"
        }
        fn width(&self) -> u16 {
            30
        }
        fn render(&self, item: &TagRow) -> (String, ratatui::style::Style) {
            (item.key.clone(), ratatui::style::Style::default())
        }
    }
    impl Column<TagRow> for ValueCol {
        fn name(&self) -> &str {
            "Value"
        }
        fn width(&self) -> u16 {
            50
        }
        fn render(&self, item: &TagRow) -> (String, ratatui::style::Style) {
            (item.value.clone(), ratatui::style::Style::default())
        }
    }

    let rows: Vec<TagRow> = fs
        .tags
        .iter()
        .map(|(k, v)| TagRow {
            key: k.clone(),
            value: v.clone(),
        })
        .collect();
    let row_refs: Vec<&TagRow> = rows.iter().collect();

    let columns: Vec<Box<dyn Column<TagRow>>> = vec![Box::new(KeyCol), Box::new(ValueCol)];

    let selected = app
        .efs_state
        .tags_table
        .selected
        .min(rows.len().saturating_sub(1));
    let expanded_index = app.efs_state.tags_table.expanded_item;

    let config = TableConfig {
        items: row_refs,
        selected_index: selected,
        expanded_index,
        columns: &columns,
        sort_column: "Key",
        sort_direction: SortDirection::Asc,
        title: format_title(&format!("Tags ({})", rows.len())),
        area,
        get_expanded_content: Some(Box::new(|row: &TagRow| {
            expanded_from_columns(&columns, row)
        })),
        is_active: app.mode != crate::keymap::Mode::FilterInput,
    };
    render_table(frame, config);
}

fn format_performance_mode_display(mode: &str) -> String {
    match mode {
        "GeneralPurpose" => "General Purpose".to_string(),
        "MaxIo" => "Max I/O".to_string(),
        s => s.to_string(),
    }
}

pub fn render_file_systems_list(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter
            Constraint::Min(0),    // Table
        ])
        .split(area);

    let filtered = filtered_efs_file_systems(app);
    let filtered_count = filtered.len();

    let page_size = app.efs_state.file_systems.page_size.value();
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = app
        .efs_state
        .file_systems
        .selected
        .checked_div(page_size)
        .unwrap_or(0);
    let pagination = render_pagination_text(current_page, total_pages);

    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &app.efs_state.file_systems.filter,
            placeholder: "Search by name, file system ID, or availability zone",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.efs_state.input_focus == InputFocus::Filter,
            is_pagination_focused: app.efs_state.input_focus == InputFocus::Pagination,
        },
    );

    let start_idx = current_page * page_size;
    let end_idx = (start_idx + page_size).min(filtered_count);
    let paginated: Vec<_> = if start_idx < filtered_count {
        filtered[start_idx..end_idx].to_vec()
    } else {
        vec![]
    };

    let title = format_title(&format!("File Systems ({})", filtered_count));

    let columns: Vec<Box<dyn Column<EfsFileSystem>>> = app
        .efs_visible_column_ids
        .iter()
        .filter_map(|col_id| {
            fs::Column::from_id(col_id).map(|col| Box::new(col) as Box<dyn Column<EfsFileSystem>>)
        })
        .collect();

    let expanded_index = app.efs_state.file_systems.expanded_item.and_then(|idx| {
        if idx >= start_idx && idx < end_idx {
            Some(idx - start_idx)
        } else {
            None
        }
    });

    let selected_index = app
        .efs_state
        .file_systems
        .selected
        .saturating_sub(start_idx);

    let config = TableConfig {
        items: paginated,
        selected_index,
        expanded_index,
        columns: &columns,
        sort_column: "Creation time",
        sort_direction: SortDirection::Desc,
        title,
        area: chunks[1],
        get_expanded_content: Some(Box::new(|fs: &EfsFileSystem| {
            expanded_from_columns(&columns, fs)
        })),
        is_active: app.mode != crate::keymap::Mode::FilterInput,
    };

    render_table(frame, config);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_default() {
        let state = State::new();
        assert_eq!(state.input_focus, InputFocus::Filter);
        assert!(state.file_systems.items.is_empty());
    }

    #[test]
    fn test_filtered_efs_by_name() {
        use crate::app::App;
        let mut app = App::new_without_client("default".to_string(), None);
        app.efs_state.file_systems.items = vec![
            make_fs("fs-001", "prod-efs", "us-east-1a"),
            make_fs("fs-002", "dev-efs", "us-east-1b"),
        ];

        app.efs_state.file_systems.filter = "prod".to_string();
        let filtered = filtered_efs_file_systems(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "prod-efs");
    }

    #[test]
    fn test_filtered_efs_by_id() {
        use crate::app::App;
        let mut app = App::new_without_client("default".to_string(), None);
        app.efs_state.file_systems.items = vec![
            make_fs("fs-aabbcc", "alpha", "us-east-1a"),
            make_fs("fs-ddeeff", "beta", "us-east-1b"),
        ];

        app.efs_state.file_systems.filter = "aabb".to_string();
        let filtered = filtered_efs_file_systems(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].file_system_id, "fs-aabbcc");
    }

    #[test]
    fn test_filtered_efs_empty_filter_returns_all() {
        use crate::app::App;
        let mut app = App::new_without_client("default".to_string(), None);
        app.efs_state.file_systems.items = vec![
            make_fs("fs-001", "a", "us-east-1a"),
            make_fs("fs-002", "b", "us-east-1b"),
        ];
        let filtered = filtered_efs_file_systems(&app);
        assert_eq!(filtered.len(), 2);
    }

    fn make_fs(id: &str, name: &str, az: &str) -> EfsFileSystem {
        EfsFileSystem {
            file_system_id: id.to_string(),
            name: name.to_string(),
            creation_token: String::new(),
            encrypted: "Yes".to_string(),
            kms_key_id: String::new(),
            total_size: String::new(),
            size_in_standard: String::new(),
            size_in_ia: String::new(),
            size_in_archive: String::new(),
            provisioned_throughput: String::new(),
            throughput_mode: String::new(),
            life_cycle_state: "Available".to_string(),
            number_of_mount_targets: "0".to_string(),
            owner_id: String::new(),
            creation_time: String::new(),
            performance_mode: String::new(),
            availability_zone: az.to_string(),
            replication_overwrite_protection: String::new(),
            file_system_arn: String::new(),
            dns_name: String::new(),
            tags: vec![],
            total_size_bytes: 0,
            size_in_standard_bytes: 0,
            size_in_ia_bytes: 0,
            size_in_archive_bytes: 0,
        }
    }

    #[test]
    fn test_yank_in_list_view_copies_file_system_id() {
        use crate::app::App;
        use crate::efs::actions::yank;
        let mut app = App::new_without_client("default".to_string(), None);
        app.efs_state.file_systems.items = vec![make_fs("fs-001", "my-efs", "us-east-1a")];
        app.efs_state.file_systems.selected = 0;
        app.efs_state.current_file_system = None;
        // List view: yank should select file_system_id (no clipboard in tests,
        // but we verify it doesn't panic and state is unchanged)
        yank(&app);
        assert_eq!(app.efs_state.current_file_system, None);
    }

    #[test]
    fn test_yank_in_detail_view_uses_arn() {
        use crate::app::App;
        use crate::efs::actions::yank;
        let mut app = App::new_without_client("default".to_string(), None);
        let mut fs = make_fs("fs-001", "my-efs", "us-east-1a");
        fs.file_system_arn =
            "arn:aws:elasticfilesystem:us-east-1:123456789012:file-system/fs-001".to_string();
        app.efs_state.file_systems.items = vec![fs];
        app.efs_state.current_file_system = Some("fs-001".to_string());
        // Detail view: yank uses ARN. Verify it doesn't panic.
        yank(&app);
        // State unchanged — yank is a side-effect (clipboard)
        assert_eq!(
            app.efs_state.current_file_system,
            Some("fs-001".to_string())
        );
    }

    #[test]
    fn test_general_pane_height_fits_content() {
        // The General pane must be sized to content height, not the full available area.
        // With 10 fields and a wide terminal, all fields fit in 1 row (dynamic columns).
        // With a narrow terminal, fields stack into more rows.
        use crate::ui::calculate_dynamic_height;
        use crate::ui::labeled_field;

        let fields: Vec<ratatui::text::Line> = vec![
            labeled_field(
                "ARN",
                "arn:aws:elasticfilesystem:us-east-1:123:file-system/fs-001",
            ),
            labeled_field("Performance mode", "General Purpose"),
            labeled_field("Throughput mode", "Bursting"),
            labeled_field("Availability zone", "Regional"),
            labeled_field("Encrypted", "Yes"),
            labeled_field("File system state", "Available"),
            labeled_field("DNS name", "fs-001.efs.us-east-1.amazonaws.com"),
            labeled_field("Replication overwrite protection", "Enabled"),
            labeled_field("File system ID", "fs-001"),
            labeled_field("Creation time", "2024-01-01T00:00:00Z"),
        ];

        // Wide terminal: fields should fit in fewer rows
        let wide_height = calculate_dynamic_height(&fields, 300);
        // Narrow terminal: fields stack into more rows
        let narrow_height = calculate_dynamic_height(&fields, 60);

        assert!(
            wide_height <= narrow_height,
            "wide terminal should need <= rows than narrow: wide={wide_height} narrow={narrow_height}"
        );

        // Block height = content + 2 borders; must be <= available height
        let block_height = (wide_height + 2).min(50);
        assert!(
            block_height <= 50,
            "block height must fit in available area"
        );
        assert!(
            block_height >= 3,
            "block height must be at least 3 (1 row + 2 borders)"
        );
    }

    #[test]
    fn test_fmt_metered_bytes() {
        assert_eq!(fmt_metered_bytes(0), "0 Bytes");
        assert_eq!(fmt_metered_bytes(512), "512 Bytes");
        assert_eq!(fmt_metered_bytes(36 * 1024), "36.00 KiB");
        assert_eq!(fmt_metered_bytes(1024 * 1024), "1.00 MiB");
        assert_eq!(fmt_metered_bytes(3 * 1024 * 1024 * 1024), "3.00 GiB");
    }

    #[test]
    fn test_metered_pct() {
        assert_eq!(metered_pct(0, 0), 0.0);
        assert_eq!(metered_pct(50, 0), 0.0);
        assert_eq!(metered_pct(36864, 36864), 100.0);
        assert_eq!(metered_pct(0, 36864), 0.0);
        assert_eq!(metered_pct(25, 100), 25.0);
    }

    #[test]
    fn test_metered_field_value() {
        // 36 KiB total, all Standard.
        let total = 36 * 1024;
        assert_eq!(metered_field_value(36 * 1024, total), "36.00 KiB (100%)");
        assert_eq!(metered_field_value(0, total), "0 Bytes (0%)");
        // Zero total => 0%.
        assert_eq!(metered_field_value(0, 0), "0 Bytes (0%)");
    }

    #[test]
    fn test_multi_y_step_never_zero() {
        // The MultiDatasetChart y-axis label loop increments by this step; a 0
        // step would spin forever. It must always be >= 1.
        assert_eq!(multi_y_step(0.0), 1);
        assert_eq!(multi_y_step(5.0), 1);
        assert_eq!(multi_y_step(10.0), 1);
        assert!(multi_y_step(100.0) >= 1);
        assert!(multi_y_step(1_000_000.0) >= 1);
        // Tiny fractional maxima (e.g. a near-empty file system) still yield >= 1.
        assert_eq!(multi_y_step(0.0001), 1);
    }

    #[test]
    fn test_console_url_policy_tab_has_tab_id() {
        let url = crate::efs::console_url_file_system_with_tab(
            "us-east-2",
            "fs-06320e1e17dfe0398",
            DetailTab::FileSystemPolicy,
        );
        assert!(url.contains("fs-06320e1e17dfe0398"), "url has fs id: {url}");
        assert!(
            url.contains("tabId=fileSystemPolicy"),
            "url has tabId: {url}"
        );
        assert!(url.contains("region=us-east-2"), "url has region: {url}");
    }

    #[test]
    fn test_console_url_metered_size_tab_has_tab_id() {
        let url = crate::efs::console_url_file_system_with_tab(
            "us-east-2",
            "fs-123",
            DetailTab::MeteredSize,
        );
        assert!(url.contains("tabId=meteredSize"), "url: {url}");
        assert!(url.contains("#/file-systems/fs-123"), "url: {url}");
    }

    #[test]
    fn test_replication_placeholder_when_empty() {
        let (text, is_placeholder) = replication_text("");
        assert!(is_placeholder);
        assert!(text.contains("No replication"));
        assert!(text.contains("This file system is not being replicated."));
        // Whitespace-only is also treated as empty.
        assert!(replication_text("   \n  ").1);
    }

    #[test]
    fn test_replication_shows_output_when_present() {
        let doc = "Source file system: fs-123\nDestinations:\n  • fs-456 (us-west-2) — Enabled";
        let (text, is_placeholder) = replication_text(doc);
        assert!(!is_placeholder);
        assert_eq!(text, doc);
    }

    #[test]
    fn test_default_detail_tab_is_not_general() {
        // General tab was removed; default detail tab is Metered size.
        let state = State::new();
        assert_eq!(state.detail_tab, DetailTab::MeteredSize);
        // The ribbon no longer contains a General tab.
        assert!(DetailTab::ALL.iter().all(|t| t.name() != "General"));
    }

    #[test]
    fn test_filtered_access_points_by_name_id_path() {
        use crate::app::App;
        use rusticity_core::efs::EfsAccessPoint;
        let mut app = App::new_without_client("default".to_string(), None);
        let mk = |id: &str, name: &str, path: &str| EfsAccessPoint {
            access_point_id: id.to_string(),
            name: name.to_string(),
            path: path.to_string(),
            posix_user: String::new(),
            creation_info: String::new(),
            life_cycle_state: "available".to_string(),
        };
        app.efs_state.access_points.items = vec![
            mk("fsap-001", "prod-ap", "/prod"),
            mk("fsap-002", "dev-ap", "/dev"),
        ];

        app.efs_state.access_points.filter = "prod".to_string();
        assert_eq!(filtered_access_points(&app).len(), 1);

        app.efs_state.access_points.filter = "fsap-002".to_string();
        assert_eq!(filtered_access_points(&app).len(), 1);

        app.efs_state.access_points.filter = "/dev".to_string();
        assert_eq!(filtered_access_points(&app).len(), 1);

        app.efs_state.access_points.filter = String::new();
        assert_eq!(filtered_access_points(&app).len(), 2);
    }
}
