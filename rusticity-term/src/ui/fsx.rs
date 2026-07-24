use crate::app::App;
use crate::common::{render_pagination_text, CyclicEnum, InputFocus, SortDirection};
use crate::fsx::fs;
use crate::table::TableState;
use crate::ui::filter::{render_simple_filter, SimpleFilterConfig};
use crate::ui::format_title;
use crate::ui::render_tabs;
use crate::ui::table::{expanded_from_columns, render_table, Column, TableConfig};
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use rusticity_core::fsx::FsxFileSystem;

pub const FILTER_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

/// Fixed column names for the Tags detail table (single source for table + prefs).
pub const TAGS_COLUMNS: [&str; 2] = ["Key", "Value"];
/// Fixed column names for the Updates detail table.
pub const UPDATES_COLUMNS: [&str; 5] = [
    "Update type",
    "Target value",
    "Status",
    "Progress %",
    "Request time",
];
/// Fixed column names for the Backups detail table.
pub const BACKUPS_COLUMNS: [&str; 15] = [
    "Backup name",
    "Backup ID",
    "File system type",
    "Lifecycle state",
    "Progress %",
    "Type",
    "Deployment Type",
    "Storage class",
    "Storage",
    "Resource ID",
    "Resource Name",
    "Backup time",
    "Active directory",
    "File system Lustre version",
    "KMS key ID",
];

/// Fixed column names for the active detail tab, if it is a fixed-column table.
pub fn detail_fixed_columns(detail_tab: DetailTab) -> Option<&'static [&'static str]> {
    match detail_tab {
        DetailTab::Tags => Some(&TAGS_COLUMNS),
        DetailTab::Updates => Some(&UPDATES_COLUMNS),
        DetailTab::Backups => Some(&BACKUPS_COLUMNS),
        _ => None,
    }
}

/// Detail-view ribbon tabs (mirrors the AWS FSx console).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetailTab {
    NetworkSecurity,
    MonitoringPerformance,
    Administration,
    DataRepository,
    Backups,
    Updates,
    Tags,
}

impl CyclicEnum for DetailTab {
    const ALL: &'static [Self] = &[
        Self::NetworkSecurity,
        Self::MonitoringPerformance,
        Self::Administration,
        Self::DataRepository,
        Self::Backups,
        Self::Updates,
        Self::Tags,
    ];
}

impl DetailTab {
    pub fn name(&self) -> &'static str {
        match self {
            DetailTab::NetworkSecurity => "Network & security",
            DetailTab::MonitoringPerformance => "Monitoring & performance",
            DetailTab::Administration => "Administration",
            DetailTab::DataRepository => "Data repository",
            DetailTab::Backups => "Backups",
            DetailTab::Updates => "Updates",
            DetailTab::Tags => "Tags",
        }
    }
}

pub struct State {
    pub file_systems: TableState<FsxFileSystem>,
    pub input_focus: InputFocus,
    pub current_file_system: Option<String>,
    pub detail_tab: DetailTab,
    pub tags_table: TableState<(String, String)>,
    pub updates_table: TableState<rusticity_core::fsx::FsxUpdate>,
    pub backups: TableState<rusticity_core::fsx::FsxBackup>,
    pub backups_loading: bool,
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
            detail_tab: DetailTab::NetworkSecurity,
            tags_table: TableState::new(),
            updates_table: TableState::new(),
            backups: TableState::new(),
            backups_loading: false,
        }
    }
}

pub fn filtered_fsx_file_systems(app: &App) -> Vec<&FsxFileSystem> {
    let filter = app.fsx_state.file_systems.filter.to_lowercase();
    app.fsx_state
        .file_systems
        .items
        .iter()
        .filter(|fs| {
            filter.is_empty()
                || fs.name.to_lowercase().contains(&filter)
                || fs.file_system_id.to_lowercase().contains(&filter)
                || fs.file_system_type.to_lowercase().contains(&filter)
        })
        .collect()
}

pub fn render_file_systems(frame: &mut Frame, app: &App, area: Rect) {
    if app.fsx_state.current_file_system.is_some() {
        render_file_system_detail(frame, app, area);
    } else {
        render_file_systems_list(frame, app, area);
    }
}

// ── Formatting helpers ────────────────────────────────────────────────────────

/// Insert a space before a trailing digit: "Persistent2" -> "Persistent 2".
pub fn format_deployment_type(raw: &str) -> String {
    if let Some(last) = raw.chars().last() {
        if last.is_ascii_digit() && raw.len() > 1 {
            let (head, tail) = raw.split_at(raw.len() - 1);
            return format!("{} {}", head, tail);
        }
    }
    raw.to_string()
}

/// Storage class in upper case: "Ssd" -> "SSD".
pub fn format_storage_class(raw: &str) -> String {
    raw.to_uppercase()
}

/// Data compression type in upper case: "Lz4" -> "LZ4", "None" -> "NONE".
pub fn format_compression(raw: &str) -> String {
    raw.to_uppercase()
}

/// Storage capacity in decimal TB from raw GiB: 309600 -> "309.6 TB".
pub fn format_capacity_tb(gib: i64) -> String {
    if gib <= 0 {
        return "-".to_string();
    }
    format!("{:.1} TB", gib as f64 / 1000.0)
}

/// Format a weekly maintenance start ("d:HH:MM") as "Thursday 07:00 UTC".
pub fn format_weekly_maintenance(raw: &str) -> String {
    let parts: Vec<&str> = raw.split(':').collect();
    if parts.len() == 3 {
        let day = match parts[0] {
            "1" => "Monday",
            "2" => "Tuesday",
            "3" => "Wednesday",
            "4" => "Thursday",
            "5" => "Friday",
            "6" => "Saturday",
            "7" => "Sunday",
            _ => parts[0],
        };
        return format!("{} {}:{} UTC", day, parts[1], parts[2]);
    }
    raw.to_string()
}

/// Green for "Enabled", yellow for "Disabled" (used for EFA / Root Squash).
pub fn enabled_disabled_color(value: &str) -> Color {
    match value {
        "Enabled" | "ENABLED" | "enabled" => Color::Green,
        "Disabled" | "DISABLED" | "disabled" => Color::Yellow,
        _ => Color::White,
    }
}

fn lifecycle_color(status: &str) -> Color {
    match status {
        "AVAILABLE" | "Available" | "available" => Color::Green,
        "CREATING" | "UPDATING" | "Creating" | "Updating" => Color::Yellow,
        "FAILED" | "DELETING" | "MISCONFIGURED" | "MISCONFIGURED_UNAVAILABLE" => Color::Red,
        _ => Color::White,
    }
}

fn build_summary_lines(fs: &FsxFileSystem) -> Vec<Line<'static>> {
    use crate::ui::{labeled_field, labeled_field_colored};

    let mut lines: Vec<Line> = vec![
        labeled_field("File system ID", fs.file_system_id.clone()),
        labeled_field_colored(
            "Lifecycle state",
            fs.status.clone(),
            lifecycle_color(&fs.status),
        ),
        labeled_field("File system type", fs.file_system_type.clone()),
    ];
    if !fs.deployment_type.is_empty() {
        lines.push(labeled_field(
            "Deployment type",
            format_deployment_type(&fs.deployment_type),
        ));
    }
    if !fs.data_compression_type.is_empty() {
        lines.push(labeled_field(
            "Data compression type",
            format_compression(&fs.data_compression_type),
        ));
    }
    if !fs.storage_class.is_empty() {
        lines.push(labeled_field(
            "Storage class",
            format_storage_class(&fs.storage_class),
        ));
    }
    lines.push(labeled_field(
        "Storage capacity",
        format_capacity_tb(fs.storage_capacity_gib),
    ));
    if fs.per_unit_throughput > 0 {
        lines.push(labeled_field(
            "Throughput per unit of storage",
            format!("{} MB/s/TiB", fs.per_unit_throughput),
        ));
    }
    if fs.total_throughput_mbps > 0 {
        lines.push(labeled_field(
            "Total throughput",
            format!("{} MB/s", fs.total_throughput_mbps),
        ));
    }
    if !fs.efa.is_empty() {
        lines.push(labeled_field_colored(
            "EFA",
            fs.efa.clone(),
            enabled_disabled_color(&fs.efa),
        ));
    }
    if !fs.root_squash.is_empty() {
        lines.push(labeled_field_colored(
            "Root Squash",
            fs.root_squash.clone(),
            enabled_disabled_color(&fs.root_squash),
        ));
    }
    if !fs.lustre_version.is_empty() {
        lines.push(labeled_field("Lustre version", fs.lustre_version.clone()));
    }
    if !fs.subnet_ids.is_empty() {
        lines.push(labeled_field("Subnet IDs", fs.subnet_ids.clone()));
    }
    lines.push(labeled_field("Creation time", fs.creation_time.clone()));
    if !fs.mount_name.is_empty() {
        lines.push(labeled_field("Mount name", fs.mount_name.clone()));
    }
    lines
}

pub fn render_file_system_detail(frame: &mut Frame, app: &App, area: Rect) {
    let fs = app.fsx_state.current_file_system.as_deref().and_then(|id| {
        app.fsx_state
            .file_systems
            .items
            .iter()
            .find(|f| f.file_system_id == id)
    });

    let tabs: Vec<(&str, DetailTab)> = DetailTab::ALL.iter().map(|t| (t.name(), *t)).collect();

    let Some(fs) = fs else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);
        render_tabs(frame, chunks[1], &tabs, &app.fsx_state.detail_tab);
        return;
    };

    // Summary pane sized to its content height (dynamic columns).
    let field_lines = build_summary_lines(fs);
    let content_rows =
        crate::ui::calculate_dynamic_height(&field_lines, area.width.saturating_sub(2));
    let summary_height = (content_rows + 3).min(area.height.saturating_sub(2));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(summary_height),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    let block = crate::ui::titled_block(" Summary ");
    let inner = block.inner(chunks[0]);
    frame.render_widget(block, chunks[0]);
    crate::ui::render_fields_with_dynamic_columns(frame, inner, field_lines);

    render_tabs(frame, chunks[1], &tabs, &app.fsx_state.detail_tab);

    match app.fsx_state.detail_tab {
        DetailTab::NetworkSecurity => render_network_security(frame, fs, chunks[2]),
        DetailTab::Administration => render_administration(frame, fs, chunks[2]),
        DetailTab::Updates => render_updates_tab(frame, app, fs, chunks[2]),
        DetailTab::Backups => render_backups_tab(frame, app, chunks[2]),
        DetailTab::Tags => render_tags_tab(frame, app, fs, chunks[2]),
        other => {
            let msg = format!("{} — not yet implemented", other.name());
            let p = Paragraph::new(msg)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            frame.render_widget(p, chunks[2]);
        }
    }
}

/// A simple label/width/extractor column adapter for fixed detail tables.
struct FieldCol<T: 'static> {
    label: &'static str,
    width: u16,
    get: fn(&T) -> String,
}

impl<T> Column<T> for FieldCol<T> {
    fn name(&self) -> &str {
        self.label
    }
    fn width(&self) -> u16 {
        self.width
    }
    fn render(&self, item: &T) -> (String, Style) {
        ((self.get)(item), Style::default())
    }
}

fn render_network_security(frame: &mut Frame, fs: &FsxFileSystem, area: Rect) {
    use crate::ui::{calculate_dynamic_height, labeled_field, render_fields_with_dynamic_columns};

    let net_lines: Vec<Line> = vec![
        labeled_field("VPC", fs.vpc_id.clone()),
        labeled_field("DNS name", fs.dns_name.clone()),
        labeled_field("Network interface IDs", fs.network_interface_ids.clone()),
        labeled_field("KMS key ID", fs.kms_key_id.clone()),
    ];
    let subnet_lines: Vec<Line> = vec![
        labeled_field("Subnet", fs.subnet_ids.clone()),
        // FSx returns subnet IDs, not AZ names — resolving AZ needs an EC2 call.
        labeled_field("Availability Zone", "-"),
        labeled_field("Network interface(s)", "See the Amazon EC2 Console"),
    ];

    let w = area.width.saturating_sub(2);
    let h1 = (calculate_dynamic_height(&net_lines, w) + 3).min(area.height);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(h1), Constraint::Min(0)])
        .split(area);

    let block1 = crate::ui::titled_block(" Network & security ");
    let inner1 = block1.inner(chunks[0]);
    frame.render_widget(block1, chunks[0]);
    render_fields_with_dynamic_columns(frame, inner1, net_lines);

    let first_subnet = fs.subnet_ids.split(',').next().unwrap_or("").trim();
    let subnet_title = if first_subnet.is_empty() {
        " Subnet ".to_string()
    } else {
        format!(" Subnet ({}) ", first_subnet)
    };
    let block2 = crate::ui::titled_block(subnet_title);
    let inner2 = block2.inner(chunks[1]);
    frame.render_widget(block2, chunks[1]);
    render_fields_with_dynamic_columns(frame, inner2, subnet_lines);
}

fn render_administration(frame: &mut Frame, fs: &FsxFileSystem, area: Rect) {
    use crate::ui::{labeled_field, render_fields_with_dynamic_columns};

    let mut lines: Vec<Line> = Vec::new();
    if !fs.weekly_maintenance_start.is_empty() {
        lines.push(labeled_field(
            "Weekly maintenance window",
            format_weekly_maintenance(&fs.weekly_maintenance_start),
        ));
    }
    if fs.automatic_backup_retention_days > 0 {
        lines.push(labeled_field(
            "Automatic backup retention",
            format!("{} days", fs.automatic_backup_retention_days),
        ));
    } else {
        lines.push(labeled_field("Automatic backups", "Disabled"));
    }
    if !fs.daily_automatic_backup_start.is_empty() {
        lines.push(labeled_field(
            "Daily automatic backup window",
            format!("{} UTC", fs.daily_automatic_backup_start),
        ));
    }

    let block = crate::ui::titled_block(" Settings ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    render_fields_with_dynamic_columns(frame, inner, lines);
}

fn render_updates_tab(frame: &mut Frame, app: &App, fs: &FsxFileSystem, area: Rect) {
    use rusticity_core::fsx::FsxUpdate;

    let rows: Vec<&FsxUpdate> = fs.updates.iter().collect();
    let columns: Vec<Box<dyn Column<FsxUpdate>>> = vec![
        Box::new(FieldCol {
            label: "Update type",
            width: 26,
            get: |u: &FsxUpdate| u.update_type.clone(),
        }),
        Box::new(FieldCol {
            label: "Target value",
            width: 18,
            get: |u: &FsxUpdate| u.target_value.clone(),
        }),
        Box::new(FieldCol {
            label: "Status",
            width: 18,
            get: |u: &FsxUpdate| u.status.clone(),
        }),
        Box::new(FieldCol {
            label: "Progress %",
            width: 12,
            get: |u: &FsxUpdate| u.progress_percent.clone(),
        }),
        Box::new(FieldCol {
            label: "Request time",
            width: 26,
            get: |u: &FsxUpdate| u.request_time.clone(),
        }),
    ];

    let selected = app
        .fsx_state
        .updates_table
        .selected
        .min(rows.len().saturating_sub(1));

    let config = TableConfig {
        items: rows,
        selected_index: selected,
        expanded_index: app.fsx_state.updates_table.expanded_item,
        columns: &columns,
        sort_column: "Request time",
        sort_direction: SortDirection::Asc,
        title: format_title(&format!("Updates ({})", fs.updates.len())),
        area,
        get_expanded_content: Some(Box::new(|u: &FsxUpdate| expanded_from_columns(&columns, u))),
        is_active: app.mode != crate::keymap::Mode::FilterInput,
    };
    render_table(frame, config);
}

fn render_backups_tab(frame: &mut Frame, app: &App, area: Rect) {
    use rusticity_core::fsx::FsxBackup;

    if app.fsx_state.backups_loading {
        let block = crate::ui::titled_block(" Backups ");
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let p = Paragraph::new("Loading backups...")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(p, inner);
        return;
    }

    let rows: Vec<&FsxBackup> = app.fsx_state.backups.items.iter().collect();
    let columns: Vec<Box<dyn Column<FsxBackup>>> = vec![
        Box::new(FieldCol {
            label: "Backup name",
            width: 22,
            get: |b: &FsxBackup| b.name.clone(),
        }),
        Box::new(FieldCol {
            label: "Backup ID",
            width: 24,
            get: |b: &FsxBackup| b.backup_id.clone(),
        }),
        Box::new(FieldCol {
            label: "File system type",
            width: 16,
            get: |b: &FsxBackup| b.file_system_type.clone(),
        }),
        Box::new(FieldCol {
            label: "Lifecycle state",
            width: 16,
            get: |b: &FsxBackup| b.lifecycle_state.clone(),
        }),
        Box::new(FieldCol {
            label: "Progress %",
            width: 12,
            get: |b: &FsxBackup| b.progress_percent.clone(),
        }),
        Box::new(FieldCol {
            label: "Type",
            width: 16,
            get: |b: &FsxBackup| b.backup_type.clone(),
        }),
        Box::new(FieldCol {
            label: "Deployment Type",
            width: 18,
            get: |b: &FsxBackup| b.deployment_type.clone(),
        }),
        Box::new(FieldCol {
            label: "Storage class",
            width: 13,
            get: |b: &FsxBackup| b.storage_class.clone(),
        }),
        Box::new(FieldCol {
            label: "Storage",
            width: 14,
            get: |b: &FsxBackup| b.storage.clone(),
        }),
        Box::new(FieldCol {
            label: "Resource ID",
            width: 22,
            get: |b: &FsxBackup| b.resource_id.clone(),
        }),
        Box::new(FieldCol {
            label: "Resource Name",
            width: 22,
            get: |b: &FsxBackup| b.resource_name.clone(),
        }),
        Box::new(FieldCol {
            label: "Backup time",
            width: 26,
            get: |b: &FsxBackup| b.backup_time.clone(),
        }),
        Box::new(FieldCol {
            label: "Active directory",
            width: 18,
            get: |b: &FsxBackup| b.active_directory.clone(),
        }),
        Box::new(FieldCol {
            label: "File system Lustre version",
            width: 26,
            get: |b: &FsxBackup| b.lustre_version.clone(),
        }),
        Box::new(FieldCol {
            label: "KMS key ID",
            width: 30,
            get: |b: &FsxBackup| b.kms_key_id.clone(),
        }),
    ];

    let selected = app
        .fsx_state
        .backups
        .selected
        .min(rows.len().saturating_sub(1));

    let config = TableConfig {
        items: rows,
        selected_index: selected,
        expanded_index: app.fsx_state.backups.expanded_item,
        columns: &columns,
        sort_column: "Backup time",
        sort_direction: SortDirection::Desc,
        title: format_title(&format!("Backups ({})", app.fsx_state.backups.items.len())),
        area,
        get_expanded_content: Some(Box::new(|b: &FsxBackup| expanded_from_columns(&columns, b))),
        is_active: app.mode != crate::keymap::Mode::FilterInput,
    };
    render_table(frame, config);
}

fn render_tags_tab(frame: &mut Frame, app: &App, fs: &FsxFileSystem, area: Rect) {
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
        fn render(&self, item: &TagRow) -> (String, Style) {
            (item.key.clone(), Style::default())
        }
    }
    impl Column<TagRow> for ValueCol {
        fn name(&self) -> &str {
            "Value"
        }
        fn width(&self) -> u16 {
            50
        }
        fn render(&self, item: &TagRow) -> (String, Style) {
            (item.value.clone(), Style::default())
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
        .fsx_state
        .tags_table
        .selected
        .min(rows.len().saturating_sub(1));

    let config = TableConfig {
        items: row_refs,
        selected_index: selected,
        expanded_index: app.fsx_state.tags_table.expanded_item,
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

pub fn render_file_systems_list(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let filtered = filtered_fsx_file_systems(app);
    let filtered_count = filtered.len();

    let page_size = app.fsx_state.file_systems.page_size.value();
    let total_pages = filtered_count.div_ceil(page_size.max(1));
    let current_page = app
        .fsx_state
        .file_systems
        .selected
        .checked_div(page_size)
        .unwrap_or(0);
    let pagination = render_pagination_text(current_page, total_pages);

    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &app.fsx_state.file_systems.filter,
            placeholder: "Search by name, file system ID, or type",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.fsx_state.input_focus == InputFocus::Filter,
            is_pagination_focused: app.fsx_state.input_focus == InputFocus::Pagination,
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

    let columns: Vec<Box<dyn Column<FsxFileSystem>>> = app
        .fsx_visible_column_ids
        .iter()
        .filter_map(|col_id| {
            fs::Column::from_id(col_id).map(|col| Box::new(col) as Box<dyn Column<FsxFileSystem>>)
        })
        .collect();

    let expanded_index = app.fsx_state.file_systems.expanded_item.and_then(|idx| {
        if idx >= start_idx && idx < end_idx {
            Some(idx - start_idx)
        } else {
            None
        }
    });

    let selected_index = app
        .fsx_state
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
        get_expanded_content: Some(Box::new(|fs: &FsxFileSystem| {
            expanded_from_columns(&columns, fs)
        })),
        is_active: app.mode != crate::keymap::Mode::FilterInput,
    };

    render_table(frame, config);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use rusticity_core::fsx::FsxFileSystem;

    fn make_fs(id: &str, name: &str, fstype: &str) -> FsxFileSystem {
        FsxFileSystem {
            name: name.to_string(),
            file_system_id: id.to_string(),
            file_system_type: fstype.to_string(),
            status: "AVAILABLE".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_state_default() {
        let state = State::new();
        assert_eq!(state.input_focus, InputFocus::Filter);
        assert!(state.file_systems.items.is_empty());
        assert_eq!(state.current_file_system, None);
        assert_eq!(state.detail_tab, DetailTab::NetworkSecurity);
    }

    #[test]
    fn test_filtered_by_name_id_type() {
        let mut app = App::new_without_client("default".to_string(), None);
        app.fsx_state.file_systems.items = vec![
            make_fs("fs-001", "prod-lustre", "LUSTRE"),
            make_fs("fs-002", "dev-windows", "WINDOWS"),
        ];

        app.fsx_state.file_systems.filter = "prod".to_string();
        assert_eq!(filtered_fsx_file_systems(&app).len(), 1);

        app.fsx_state.file_systems.filter = "fs-002".to_string();
        assert_eq!(filtered_fsx_file_systems(&app).len(), 1);

        app.fsx_state.file_systems.filter = "windows".to_string();
        assert_eq!(filtered_fsx_file_systems(&app).len(), 1);

        app.fsx_state.file_systems.filter = String::new();
        assert_eq!(filtered_fsx_file_systems(&app).len(), 2);
    }

    #[test]
    fn test_format_deployment_type() {
        assert_eq!(format_deployment_type("Persistent2"), "Persistent 2");
        assert_eq!(format_deployment_type("Persistent1"), "Persistent 1");
        assert_eq!(format_deployment_type("Scratch2"), "Scratch 2");
        // No trailing digit — unchanged.
        assert_eq!(format_deployment_type("MultiAz"), "MultiAz");
    }

    #[test]
    fn test_format_storage_class_and_compression() {
        assert_eq!(format_storage_class("Ssd"), "SSD");
        assert_eq!(format_storage_class("Hdd"), "HDD");
        assert_eq!(format_compression("Lz4"), "LZ4");
        assert_eq!(format_compression("None"), "NONE");
    }

    #[test]
    fn test_format_capacity_tb() {
        assert_eq!(format_capacity_tb(309600), "309.6 TB");
        assert_eq!(format_capacity_tb(1200), "1.2 TB");
        assert_eq!(format_capacity_tb(0), "-");
    }

    #[test]
    fn test_detail_tab_cycles_through_seven() {
        let mut tab = DetailTab::NetworkSecurity;
        let mut seen = vec![tab];
        for _ in 0..6 {
            tab = tab.next();
            seen.push(tab);
        }
        assert_eq!(seen.len(), 7);
        assert_eq!(tab.next(), DetailTab::NetworkSecurity); // wraps around
        assert_eq!(DetailTab::ALL.len(), 7);
    }

    #[test]
    fn test_summary_includes_lustre_fields_when_present() {
        let mut fs = make_fs("fs-abc", "kirin", "Lustre");
        fs.deployment_type = "Persistent2".to_string();
        fs.data_compression_type = "Lz4".to_string();
        fs.storage_class = "Ssd".to_string();
        fs.storage_capacity_gib = 309600;
        fs.per_unit_throughput = 1000;
        fs.total_throughput_mbps = 309600;
        fs.efa = "Disabled".to_string();
        fs.root_squash = "Disabled".to_string();
        fs.lustre_version = "2.12".to_string();
        fs.mount_name = "v4kavb4v".to_string();

        let lines = build_summary_lines(&fs);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect::<Vec<_>>()
            .join("");
        assert!(text.contains("Persistent 2"));
        assert!(text.contains("LZ4"));
        assert!(text.contains("SSD"));
        assert!(text.contains("309.6 TB"));
        assert!(text.contains("1000 MB/s/TiB"));
        assert!(text.contains("309600 MB/s"));
        assert!(text.contains("2.12"));
        assert!(text.contains("v4kavb4v"));
    }

    #[test]
    fn test_column_count_uses_tags_columns_on_tags_tab() {
        // Regression: the Tags detail tab must NOT expose the File systems
        // columns in the preferences selector — only the 2 fixed Key/Value cols.
        let mut app = App::new_without_client("default".to_string(), None);
        app.fsx_state.file_systems.items = vec![make_fs("fs-1", "n", "LUSTRE")];

        // List view → all 13 file-system columns.
        assert_eq!(crate::fsx::actions::column_count(&app), 13);

        // Detail + Tags tab → 2 columns (Key/Value).
        app.fsx_state.current_file_system = Some("fs-1".to_string());
        app.fsx_state.detail_tab = DetailTab::Tags;
        assert_eq!(crate::fsx::actions::column_count(&app), 2);
        assert_eq!(crate::fsx::actions::column_selector_max(&app), 2 + 6);

        // A non-Tags detail tab falls back to the list columns.
        app.fsx_state.detail_tab = DetailTab::NetworkSecurity;
        assert_eq!(crate::fsx::actions::column_count(&app), 13);
    }

    #[test]
    fn test_enabled_disabled_color() {
        assert_eq!(enabled_disabled_color("Enabled"), Color::Green);
        assert_eq!(enabled_disabled_color("Disabled"), Color::Yellow);
        assert_eq!(enabled_disabled_color("other"), Color::White);
    }

    #[test]
    fn test_summary_omits_lustre_fields_for_windows() {
        let mut fs = make_fs("fs-win", "winfs", "Windows");
        fs.deployment_type = "MultiAz".to_string();
        fs.storage_class = "Ssd".to_string();
        fs.storage_capacity_gib = 1024;
        let lines = build_summary_lines(&fs);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect::<Vec<_>>()
            .join("");
        assert!(!text.contains("Lustre version"));
        assert!(!text.contains("Mount name"));
        assert!(!text.contains("MB/s/TiB"));
    }

    #[test]
    fn test_format_weekly_maintenance() {
        assert_eq!(format_weekly_maintenance("4:07:00"), "Thursday 07:00 UTC");
        assert_eq!(format_weekly_maintenance("1:00:00"), "Monday 00:00 UTC");
        assert_eq!(format_weekly_maintenance("7:23:30"), "Sunday 23:30 UTC");
        // Malformed input is passed through unchanged.
        assert_eq!(format_weekly_maintenance("weird"), "weird");
    }

    #[test]
    fn test_detail_fixed_columns() {
        assert_eq!(
            detail_fixed_columns(DetailTab::Tags),
            Some(&TAGS_COLUMNS[..])
        );
        assert_eq!(
            detail_fixed_columns(DetailTab::Updates),
            Some(&UPDATES_COLUMNS[..])
        );
        assert_eq!(
            detail_fixed_columns(DetailTab::Backups),
            Some(&BACKUPS_COLUMNS[..])
        );
        assert_eq!(detail_fixed_columns(DetailTab::NetworkSecurity), None);
        assert_eq!(detail_fixed_columns(DetailTab::Administration), None);
        // Column counts match the requested tables.
        assert_eq!(UPDATES_COLUMNS.len(), 5);
        assert_eq!(BACKUPS_COLUMNS.len(), 15);
    }

    #[test]
    fn test_column_count_per_detail_tab() {
        let mut app = App::new_without_client("default".to_string(), None);
        app.fsx_state.file_systems.items = vec![make_fs("fs-1", "n", "LUSTRE")];
        // List view → 13 file-system columns.
        assert_eq!(crate::fsx::actions::column_count(&app), 13);

        app.fsx_state.current_file_system = Some("fs-1".to_string());
        app.fsx_state.detail_tab = DetailTab::Tags;
        assert_eq!(crate::fsx::actions::column_count(&app), 2);
        app.fsx_state.detail_tab = DetailTab::Updates;
        assert_eq!(crate::fsx::actions::column_count(&app), 5);
        app.fsx_state.detail_tab = DetailTab::Backups;
        assert_eq!(crate::fsx::actions::column_count(&app), 15);
        // Non-table detail tabs fall back to the list columns.
        app.fsx_state.detail_tab = DetailTab::NetworkSecurity;
        assert_eq!(crate::fsx::actions::column_count(&app), 13);
    }
}
