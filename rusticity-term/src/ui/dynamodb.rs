use crate::app::App;
use crate::common::{
    filter_by_fields, format_bytes, format_iso_timestamp, render_pagination_text,
    render_partial_pagination, CyclicEnum, InputFocus, SortDirection, UTC_TIMESTAMP_WIDTH,
};
use crate::dynamodb::items::{self, ItemRequestContext};
use crate::dynamodb::table;
use crate::table::{TableState, TableWindow};
use crate::tags::{self, TagSort, TagsConfig};
use crate::ui::filter::{render_simple_filter, SimpleFilterConfig};
use crate::ui::format_title;
use crate::ui::render_tabs;
use crate::ui::table::{expanded_from_columns, render_table, Column, TableConfig};
use crate::ui::Summary;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use rusticity_core::dynamodb::{
    DdbBackup, DdbIndex, DdbItem, DdbItemCursor, DdbMonitoringData, DdbReplica, DdbTable,
    DdbTableSettings,
};

pub const FILTER_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

/// Dropdowns on the Items tab filter row.
pub const ITEM_OPERATION: InputFocus = InputFocus::Dropdown("DdbItemOperation");
pub const ITEM_INDEX: InputFocus = InputFocus::Dropdown("DdbItemIndex");
pub const ITEM_PROJECTION: InputFocus = InputFocus::Dropdown("DdbItemProjection");

/// Focus order on the Items tab: the three dropdowns, then the text input and
/// pagination.
pub const ITEMS_FILTER_CONTROLS: [InputFocus; 5] = [
    ITEM_OPERATION,
    ITEM_INDEX,
    ITEM_PROJECTION,
    InputFocus::Filter,
    InputFocus::Pagination,
];

/// Maximum evaluated items per request; AWS may stop earlier at its response-size limit.
pub const ITEM_LIMIT: i32 = 100;

/// Items tab operation. Scan is the default, matching the console.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ItemOperation {
    #[default]
    Scan,
    Query,
}

impl CyclicEnum for ItemOperation {
    const ALL: &'static [Self] = &[Self::Scan, Self::Query];
}

impl ItemOperation {
    pub fn name(&self) -> &'static str {
        match self {
            ItemOperation::Scan => "Scan",
            ItemOperation::Query => "Query",
        }
    }
}

/// Which attributes a Scan or Query returns.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ItemProjection {
    #[default]
    AllAttributes,
    KeysOnly,
}

impl CyclicEnum for ItemProjection {
    const ALL: &'static [Self] = &[Self::AllAttributes, Self::KeysOnly];
}

impl ItemProjection {
    pub fn name(&self) -> &'static str {
        match self {
            ItemProjection::AllAttributes => "All attributes",
            ItemProjection::KeysOnly => "Keys only",
        }
    }
}

/// Fixed column names for the Indexes detail table.
pub const INDEXES_COLUMNS: [&str; 10] = [
    "Name",
    "Type",
    "Status",
    "Partition key",
    "Sort key",
    "Attributes",
    "Read capacity mode",
    "Write capacity mode",
    "Size",
    "Item count",
];
/// Fixed column names for the Global tables (replicas) detail table.
pub const REPLICAS_COLUMNS: [&str; 5] = ["Region", "Status", "Table class", "KMS key", "Indexes"];
/// Fixed column names for the Backups detail table.
pub const BACKUPS_COLUMNS: [&str; 6] = [
    "Backup name",
    "Status",
    "Type",
    "Size",
    "Creation time",
    "Expiry time",
];
/// Fixed column names for the Tags detail table.
pub const TAGS_COLUMNS: [&str; 2] = ["Key", "Value"];

/// Fixed column names for the active detail tab, if it is a fixed-column table.
pub fn detail_fixed_columns(detail_tab: DetailTab) -> Option<&'static [&'static str]> {
    match detail_tab {
        DetailTab::Indexes => Some(&INDEXES_COLUMNS),
        DetailTab::GlobalTables => Some(&REPLICAS_COLUMNS),
        DetailTab::Backups => Some(&BACKUPS_COLUMNS),
        DetailTab::Tags => Some(&TAGS_COLUMNS),
        _ => None,
    }
}

/// Detail-view ribbon tabs (mirrors the DynamoDB console).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetailTab {
    Items,
    Settings,
    Indexes,
    Monitor,
    GlobalTables,
    Backups,
    ExportsStreams,
    AdditionalSettings,
    Tags,
}

impl CyclicEnum for DetailTab {
    const ALL: &'static [Self] = &[
        Self::Items,
        Self::Settings,
        Self::Indexes,
        Self::Monitor,
        Self::GlobalTables,
        Self::Backups,
        Self::ExportsStreams,
        Self::AdditionalSettings,
        Self::Tags,
    ];
}

impl DetailTab {
    pub fn name(&self) -> &'static str {
        match self {
            DetailTab::Items => "Items",
            DetailTab::Settings => "Settings",
            DetailTab::Indexes => "Indexes",
            DetailTab::Monitor => "Monitor",
            DetailTab::GlobalTables => "Global tables",
            DetailTab::Backups => "Backups",
            DetailTab::ExportsStreams => "Exports and streams",
            DetailTab::AdditionalSettings => "Additional settings",
            DetailTab::Tags => "Tags",
        }
    }

    /// The matching dynamodbv2 console tab id (used for the console URL).
    pub fn console_tab(&self) -> &'static str {
        match self {
            DetailTab::Items => "item-explorer",
            DetailTab::Settings => "overview",
            DetailTab::Indexes => "indexes",
            DetailTab::Monitor => "monitoring",
            DetailTab::GlobalTables => "globaltables",
            DetailTab::Backups => "backups",
            DetailTab::ExportsStreams => "exportsandstreams",
            DetailTab::AdditionalSettings => "additionalsettings",
            DetailTab::Tags => "tags",
        }
    }
}

pub struct State {
    pub tables: TableState<DdbTable>,
    pub input_focus: InputFocus,
    pub current_table: Option<String>,
    pub detail_tab: DetailTab,
    pub items: TableState<DdbItem>,
    pub items_loading: bool,
    pub items_cursor: Option<DdbItemCursor>,
    pub items_context: Option<ItemRequestContext>,
    /// Whether the scheduled/failed request continues the current result set.
    pub items_append: bool,
    pub items_failed: bool,
    /// Items tab text input: a client-side filter for Scan, the partition-key
    /// value for Query. Kept apart from the table-list filter.
    pub item_filter: String,
    pub item_operation: ItemOperation,
    /// 0 = the table itself, 1.. = secondary indexes in ribbon order.
    pub item_index: usize,
    pub item_projection: ItemProjection,
    /// Attribute columns hidden in Preferences. Item columns are discovered from
    /// the data, so they are tracked by name and default to all visible.
    pub item_hidden_columns: Vec<String>,
    /// Partition-key value the last Query ran with, so it only re-runs on change.
    pub executed_query: String,
    pub indexes_table: TableState<DdbIndex>,
    pub replicas_table: TableState<DdbReplica>,
    pub backups: TableState<DdbBackup>,
    pub backups_loading: bool,
    pub settings: DdbTableSettings,
    pub settings_loading: bool,
    pub tags_table: TableState<(String, String)>,
    pub tags_loading: bool,
    /// Resource-based policy documents (Additional settings tab).
    pub table_policy: Option<String>,
    pub stream_policy: Option<String>,
    pub permissions_loading: bool,
    /// Monitor tab.
    pub metrics: DdbMonitoringData,
    pub metrics_loading: bool,
    pub monitoring_scroll: usize,
}

/// Number of charts on the Monitor tab; the last scrollable index is one less.
pub const MONITORING_CHARTS: usize = 4;

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
        self.metrics = DdbMonitoringData::default();
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            tables: TableState::new(),
            input_focus: InputFocus::Filter,
            current_table: None,
            detail_tab: DetailTab::Items,
            items: TableState::new(),
            items_loading: false,
            items_cursor: None,
            items_context: None,
            items_append: false,
            items_failed: false,
            item_filter: String::new(),
            item_operation: ItemOperation::default(),
            item_index: 0,
            item_projection: ItemProjection::default(),
            item_hidden_columns: Vec::new(),
            executed_query: String::new(),
            indexes_table: TableState::new(),
            replicas_table: TableState::new(),
            backups: TableState::new(),
            backups_loading: false,
            settings: DdbTableSettings::default(),
            settings_loading: false,
            tags_table: TableState::new(),
            tags_loading: false,
            table_policy: None,
            stream_policy: None,
            permissions_loading: false,
            metrics: DdbMonitoringData::default(),
            metrics_loading: false,
            monitoring_scroll: 0,
        }
    }
}

pub fn filtered_dynamodb_tables(app: &App) -> Vec<&DdbTable> {
    filter_by_fields(
        &app.dynamodb_state.tables.items,
        &app.dynamodb_state.tables.filter,
        |t| [&t.name, &t.status, &t.partition_key],
    )
}

/// The table currently open in the detail view.
pub fn current_table(app: &App) -> Option<&DdbTable> {
    app.dynamodb_state
        .current_table
        .as_deref()
        .and_then(|name| {
            app.dynamodb_state
                .tables
                .items
                .iter()
                .find(|t| t.name == name)
        })
}

/// True for a queued Items request or changed server request context. Local
/// Scan filtering and navigation do not fetch additional pages.
pub fn needs_items_load(app: &App) -> bool {
    items::needs_load(app)
}

pub fn render_tables(frame: &mut Frame, app: &App, area: Rect) {
    if app.dynamodb_state.current_table.is_some() {
        render_table_detail(frame, app, area);
    } else {
        render_tables_list(frame, app, area);
    }
}

// ── Formatting helpers ────────────────────────────────────────────────────────

/// Green for "Enabled", yellow for "Disabled".
pub fn enabled_disabled_color(value: &str) -> Color {
    match value.to_uppercase().as_str() {
        "ENABLED" => Color::Green,
        "DISABLED" => Color::Yellow,
        _ => Color::White,
    }
}

/// "On-demand" or "Provisioned (5 RCU)"-style capacity description.
pub fn format_capacity(mode: &str, units: i64, unit_label: &str) -> String {
    if mode == "Provisioned" && units > 0 {
        format!("{} ({} {})", mode, units, unit_label)
    } else {
        mode.to_string()
    }
}

/// Show "-" for empty values so panes never render a dangling label.
fn or_dash(value: &str) -> String {
    if value.is_empty() {
        "-".to_string()
    } else {
        value.to_string()
    }
}

fn build_summary_lines(t: &DdbTable) -> Vec<Line<'static>> {
    use crate::ui::{labeled_field, labeled_field_colored};

    let (status_text, status_color) = table::format_status(&t.status);

    let mut lines: Vec<Line> = vec![
        labeled_field("Table name", t.name.clone()),
        labeled_field_colored("Status", status_text.to_string(), status_color),
        labeled_field("Partition key", or_dash(&t.partition_key)),
        labeled_field("Sort key", or_dash(&t.sort_key)),
        labeled_field(
            "Read capacity mode",
            format_capacity(&t.read_capacity_mode, t.provisioned_read_units, "RCU"),
        ),
        labeled_field(
            "Write capacity mode",
            format_capacity(&t.write_capacity_mode, t.provisioned_write_units, "WCU"),
        ),
        labeled_field("Table class", t.table_class.clone()),
        labeled_field("Total size", format_bytes(t.total_size_bytes)),
        labeled_field("Item count", t.item_count.to_string()),
        labeled_field("Indexes", t.index_count().to_string()),
        labeled_field_colored(
            "Deletion protection",
            t.deletion_protection.clone(),
            enabled_disabled_color(&t.deletion_protection),
        ),
    ];
    if !t.creation_time.is_empty() {
        lines.push(labeled_field(
            "Creation time",
            format_iso_timestamp(&t.creation_time),
        ));
    }
    lines
}

pub fn render_table_detail(frame: &mut Frame, app: &App, area: Rect) {
    let tabs: Vec<(&str, DetailTab)> = DetailTab::ALL.iter().map(|t| (t.name(), *t)).collect();

    let Some(t) = current_table(app) else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);
        render_tabs(frame, chunks[1], &tabs, &app.dynamodb_state.detail_tab);
        return;
    };

    // The Items tab is a full-height explorer: ribbon on top, no Summary pane.
    if app.dynamodb_state.detail_tab == DetailTab::Items {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);
        render_tabs(frame, chunks[0], &tabs, &app.dynamodb_state.detail_tab);
        render_items_tab(frame, app, t, chunks[1]);
        return;
    }

    let summary = Summary::new("Summary", build_summary_lines(t));
    let summary_height = summary
        .height(area.width)
        .min(area.height.saturating_sub(2));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(summary_height),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    summary.render(frame, chunks[0]);

    render_tabs(frame, chunks[1], &tabs, &app.dynamodb_state.detail_tab);

    match app.dynamodb_state.detail_tab {
        // Handled above — it renders without the Summary pane.
        DetailTab::Items => {}
        DetailTab::Settings => render_settings(frame, app, t, chunks[2]),
        DetailTab::Indexes => render_indexes_tab(frame, app, t, chunks[2]),
        DetailTab::Monitor => render_monitor_tab(frame, app, t, chunks[2]),
        DetailTab::GlobalTables => render_global_tables_tab(frame, app, t, chunks[2]),
        DetailTab::Backups => render_backups_tab(frame, app, chunks[2]),
        DetailTab::ExportsStreams => render_exports_streams(frame, t, chunks[2]),
        DetailTab::AdditionalSettings => render_additional_settings(frame, app, chunks[2]),
        DetailTab::Tags => render_tags_tab(frame, app, chunks[2]),
    }
}

/// A simple label/width/extractor column adapter for fixed detail tables.
struct FieldCol<T: 'static> {
    label: &'static str,
    width: u16,
    get: fn(&T) -> String,
}

impl<T> Column<T> for FieldCol<T> {
    fn name(&self) -> std::borrow::Cow<'_, str> {
        self.label.into()
    }
    fn width(&self) -> u16 {
        self.width
    }
    fn render(&self, item: &T) -> (String, Style) {
        ((self.get)(item), Style::default())
    }
}

fn render_settings(frame: &mut Frame, app: &App, t: &DdbTable, area: Rect) {
    use crate::ui::{labeled_field, labeled_field_colored, render_fields_with_dynamic_columns};

    let s = &app.dynamodb_state.settings;
    let ttl_status = if app.dynamodb_state.settings_loading {
        "Loading...".to_string()
    } else {
        or_dash(&s.ttl_status)
    };

    let general: Vec<Line> = vec![
        labeled_field("ARN", or_dash(&t.table_arn)),
        labeled_field("Table ID", or_dash(&t.table_id)),
        labeled_field("Encryption type", or_dash(&t.encryption_type)),
        labeled_field("KMS key ARN", or_dash(&t.kms_key_arn)),
        labeled_field_colored(
            "Time to Live (TTL)",
            ttl_status.clone(),
            enabled_disabled_color(&ttl_status),
        ),
        labeled_field("TTL attribute", or_dash(&s.ttl_attribute)),
        labeled_field_colored(
            "Deletion protection",
            t.deletion_protection.clone(),
            enabled_disabled_color(&t.deletion_protection),
        ),
    ];
    let capacity: Vec<Line> = vec![
        labeled_field(
            "Read capacity",
            format_capacity(&t.read_capacity_mode, t.provisioned_read_units, "RCU"),
        ),
        labeled_field(
            "Write capacity",
            format_capacity(&t.write_capacity_mode, t.provisioned_write_units, "WCU"),
        ),
        labeled_field("Table class", t.table_class.clone()),
        labeled_field("Replicas", t.replicas.len().to_string()),
    ];

    let summary = Summary::new("General information", general);
    let h1 = summary.height(area.width).min(area.height);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(h1), Constraint::Min(0)])
        .split(area);

    summary.render(frame, chunks[0]);

    let block2 = crate::ui::titled_block(" Capacity ");
    let inner2 = block2.inner(chunks[1]);
    frame.render_widget(block2, chunks[1]);
    render_fields_with_dynamic_columns(frame, inner2, capacity);
}

fn render_exports_streams(frame: &mut Frame, t: &DdbTable, area: Rect) {
    use crate::ui::{labeled_field, labeled_field_colored, render_fields_with_dynamic_columns};

    let lines: Vec<Line> = vec![
        labeled_field_colored(
            "DynamoDB stream",
            t.stream_enabled.clone(),
            enabled_disabled_color(&t.stream_enabled),
        ),
        labeled_field("View type", or_dash(&t.stream_view_type)),
        labeled_field("Latest stream ARN", or_dash(&t.latest_stream_arn)),
    ];

    let block = crate::ui::titled_block(" DynamoDB stream details ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    render_fields_with_dynamic_columns(frame, inner, lines);
}

fn render_additional_settings(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_policy_pane(
        frame,
        chunks[0],
        " Resource-based policy for table ",
        app.dynamodb_state.table_policy.as_deref(),
        "No table resource-based policy to display",
        app.dynamodb_state.permissions_loading,
    );
    render_policy_pane(
        frame,
        chunks[1],
        " Resource-based policy for active stream ",
        app.dynamodb_state.stream_policy.as_deref(),
        "No stream resource-based policy to display",
        app.dynamodb_state.permissions_loading,
    );
}

/// A JSON policy pane, or a centered placeholder when there is no policy.
fn render_policy_pane(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    policy: Option<&str>,
    empty_message: &str,
    loading: bool,
) {
    let block = crate::ui::titled_block(title.to_string());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if loading {
        let p = Paragraph::new("Loading policy...")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(p, inner);
        return;
    }

    match policy {
        Some(json) if !json.is_empty() => {
            frame.render_widget(Paragraph::new(json.to_string()), inner);
        }
        _ => {
            let p = Paragraph::new(empty_message.to_string())
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            frame.render_widget(p, inner);
        }
    }
}

fn render_indexes_tab(frame: &mut Frame, app: &App, t: &DdbTable, area: Rect) {
    let rows: Vec<&DdbIndex> = t
        .global_secondary_indexes
        .iter()
        .chain(t.local_secondary_indexes.iter())
        .collect();

    let columns: Vec<Box<dyn Column<DdbIndex>>> = vec![
        Box::new(FieldCol {
            label: "Name",
            width: 26,
            get: |i: &DdbIndex| i.name.clone(),
        }),
        Box::new(FieldCol {
            label: "Type",
            width: 8,
            get: |i: &DdbIndex| i.index_type.clone(),
        }),
        Box::new(FieldCol {
            label: "Status",
            width: 12,
            get: |i: &DdbIndex| i.status.clone(),
        }),
        Box::new(FieldCol {
            label: "Partition key",
            width: 20,
            get: |i: &DdbIndex| or_dash(&i.partition_key),
        }),
        Box::new(FieldCol {
            label: "Sort key",
            width: 20,
            get: |i: &DdbIndex| or_dash(&i.sort_key),
        }),
        Box::new(FieldCol {
            label: "Attributes",
            width: 20,
            get: |i: &DdbIndex| i.projection.clone(),
        }),
        Box::new(FieldCol {
            label: "Read capacity mode",
            width: 18,
            get: |i: &DdbIndex| i.read_capacity_mode.clone(),
        }),
        Box::new(FieldCol {
            label: "Write capacity mode",
            width: 19,
            get: |i: &DdbIndex| i.write_capacity_mode.clone(),
        }),
        Box::new(FieldCol {
            label: "Size",
            width: 12,
            get: |i: &DdbIndex| format_bytes(i.size_bytes),
        }),
        Box::new(FieldCol {
            label: "Item count",
            width: 11,
            get: |i: &DdbIndex| i.item_count.to_string(),
        }),
    ];

    let selected = app
        .dynamodb_state
        .indexes_table
        .selected
        .min(rows.len().saturating_sub(1));

    let config = TableConfig {
        items: rows.clone(),
        selected_index: selected,
        expanded_index: app.dynamodb_state.indexes_table.expanded_item,
        columns: &columns,
        sort_column: "Name",
        sort_direction: SortDirection::Asc,
        title: format_title(&format!("Indexes ({})", rows.len())),
        area,
        get_expanded_content: Some(Box::new(|i: &DdbIndex| expanded_from_columns(&columns, i))),
        is_active: app.mode != crate::keymap::Mode::FilterInput,
    };
    render_table(frame, config);
}

fn render_global_tables_tab(frame: &mut Frame, app: &App, t: &DdbTable, area: Rect) {
    use crate::ui::labeled_field_colored;

    // Settings replication pane. The API does not expose per-setting overrides,
    // so this reflects whether the table has replicas at all.
    let status = if t.replicas.is_empty() {
        "Disabled"
    } else {
        "Enabled"
    };
    let repl_lines: Vec<Line> = vec![labeled_field_colored(
        "Settings replication status",
        status.to_string(),
        enabled_disabled_color(status),
    )];
    let summary = Summary::new("Settings replication", repl_lines);
    let repl_height = summary.height(area.width).min(area.height);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(repl_height), Constraint::Min(0)])
        .split(area);

    summary.render(frame, chunks[0]);

    let area = chunks[1];
    let rows: Vec<&DdbReplica> = t.replicas.iter().collect();

    let columns: Vec<Box<dyn Column<DdbReplica>>> = vec![
        Box::new(FieldCol {
            label: "Region",
            width: 16,
            get: |r: &DdbReplica| r.region.clone(),
        }),
        Box::new(FieldCol {
            label: "Status",
            width: 14,
            get: |r: &DdbReplica| r.status.clone(),
        }),
        Box::new(FieldCol {
            label: "Table class",
            width: 14,
            get: |r: &DdbReplica| or_dash(&r.table_class),
        }),
        Box::new(FieldCol {
            label: "KMS key",
            width: 30,
            get: |r: &DdbReplica| or_dash(&r.kms_key_id),
        }),
        Box::new(FieldCol {
            label: "Indexes",
            width: 8,
            get: |r: &DdbReplica| r.index_count.to_string(),
        }),
    ];

    let selected = app
        .dynamodb_state
        .replicas_table
        .selected
        .min(rows.len().saturating_sub(1));

    let config = TableConfig {
        items: rows.clone(),
        selected_index: selected,
        expanded_index: app.dynamodb_state.replicas_table.expanded_item,
        columns: &columns,
        sort_column: "Region",
        sort_direction: SortDirection::Asc,
        title: format_title(&format!("Replicas ({})", rows.len())),
        area,
        get_expanded_content: Some(Box::new(|r: &DdbReplica| {
            expanded_from_columns(&columns, r)
        })),
        is_active: app.mode != crate::keymap::Mode::FilterInput,
    };
    render_table(frame, config);
}

fn render_backups_tab(frame: &mut Frame, app: &App, area: Rect) {
    use crate::ui::{labeled_field, labeled_field_colored};

    // Point-in-time recovery pane.
    let s = &app.dynamodb_state.settings;
    let pitr_status = if app.dynamodb_state.settings_loading {
        "Loading...".to_string()
    } else if s.pitr_status.is_empty() {
        "Off".to_string()
    } else if s.pitr_status == "Enabled" {
        "On".to_string()
    } else {
        "Off".to_string()
    };
    let mut pitr_lines: Vec<Line> = vec![labeled_field_colored(
        "Status",
        pitr_status.clone(),
        if pitr_status == "On" {
            Color::Green
        } else {
            Color::Yellow
        },
    )];
    if s.recovery_period_days > 0 {
        pitr_lines.push(labeled_field(
            "Backup recovery period",
            format!("{} days", s.recovery_period_days),
        ));
    }
    if !s.earliest_restorable_time.is_empty() {
        pitr_lines.push(labeled_field(
            "Earliest restore point",
            format_iso_timestamp(&s.earliest_restorable_time),
        ));
    }
    if !s.latest_restorable_time.is_empty() {
        pitr_lines.push(labeled_field(
            "Latest restore point",
            format_iso_timestamp(&s.latest_restorable_time),
        ));
    }

    let summary = Summary::new("Point-in-time recovery (PITR)", pitr_lines);
    let pitr_height = summary.height(area.width).min(area.height);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(pitr_height), Constraint::Min(0)])
        .split(area);

    summary.render(frame, chunks[0]);

    let area = chunks[1];

    if app.dynamodb_state.backups_loading {
        let block = crate::ui::titled_block(" Backups ");
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let p = Paragraph::new("Loading backups...")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(p, inner);
        return;
    }

    let rows: Vec<&DdbBackup> = app.dynamodb_state.backups.items.iter().collect();

    let columns: Vec<Box<dyn Column<DdbBackup>>> = vec![
        Box::new(FieldCol {
            label: "Backup name",
            width: 30,
            get: |b: &DdbBackup| b.name.clone(),
        }),
        Box::new(FieldCol {
            label: "Status",
            width: 12,
            get: |b: &DdbBackup| b.status.clone(),
        }),
        Box::new(FieldCol {
            label: "Type",
            width: 12,
            get: |b: &DdbBackup| b.backup_type.clone(),
        }),
        Box::new(FieldCol {
            label: "Size",
            width: 12,
            get: |b: &DdbBackup| format_bytes(b.size_bytes),
        }),
        Box::new(FieldCol {
            label: "Creation time",
            width: UTC_TIMESTAMP_WIDTH,
            get: |b: &DdbBackup| format_iso_timestamp(&b.creation_time),
        }),
        Box::new(FieldCol {
            label: "Expiry time",
            width: UTC_TIMESTAMP_WIDTH,
            get: |b: &DdbBackup| format_iso_timestamp(&b.expiry_time),
        }),
    ];

    let selected = app
        .dynamodb_state
        .backups
        .selected
        .min(rows.len().saturating_sub(1));

    let config = TableConfig {
        items: rows,
        selected_index: selected,
        expanded_index: app.dynamodb_state.backups.expanded_item,
        columns: &columns,
        sort_column: "Creation time",
        sort_direction: SortDirection::Desc,
        title: format_title(&format!(
            "Backups ({})",
            app.dynamodb_state.backups.items.len()
        )),
        area,
        get_expanded_content: Some(Box::new(|b: &DdbBackup| expanded_from_columns(&columns, b))),
        is_active: app.mode != crate::keymap::Mode::FilterInput,
    };
    render_table(frame, config);
}

/// Monitor tab — CloudWatch charts for capacity, throttling and latency.
fn render_monitor_tab(frame: &mut Frame, app: &App, t: &DdbTable, area: Rect) {
    use crate::ui::monitoring::{render_monitoring_tab, MetricChart, MultiDatasetChart};

    if app.dynamodb_state.metrics_loading {
        let block = crate::ui::titled_block(" Monitor ");
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let p = Paragraph::new("Loading metrics...")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(p, inner);
        return;
    }

    let m = &app.dynamodb_state.metrics;

    // Provisioned tables get their capacity drawn as a threshold line.
    let read_threshold = (t.provisioned_read_units > 0).then_some(t.provisioned_read_units as f64);
    let write_threshold =
        (t.provisioned_write_units > 0).then_some(t.provisioned_write_units as f64);

    let single = vec![
        MetricChart {
            title: "Read capacity (units/second)",
            data: &m.consumed_read_capacity,
            y_axis_label: "Units/s",
            x_axis_label: None,
            threshold: read_threshold,
        },
        MetricChart {
            title: "Write capacity (units/second)",
            data: &m.consumed_write_capacity,
            y_axis_label: "Units/s",
            x_axis_label: None,
            threshold: write_threshold,
        },
    ];

    let multi = vec![
        MultiDatasetChart {
            title: "Throttled requests and events",
            datasets: vec![
                ("Read throttle events", m.read_throttle_events.as_slice()),
                ("Write throttle events", m.write_throttle_events.as_slice()),
                ("Throttled requests", m.throttled_requests.as_slice()),
            ],
            y_axis_label: "Count",
            y_axis_step: 1,
            x_axis_label: None,
        },
        MultiDatasetChart {
            title: "Successful request latency (ms)",
            datasets: vec![
                ("GetItem", m.latency_get_item.as_slice()),
                ("PutItem", m.latency_put_item.as_slice()),
                ("Query", m.latency_query.as_slice()),
                ("Scan", m.latency_scan.as_slice()),
            ],
            y_axis_label: "ms",
            y_axis_step: 1,
            x_axis_label: None,
        },
    ];

    render_monitoring_tab(
        frame,
        area,
        &single,
        &multi,
        &[],
        &[],
        app.dynamodb_state.monitoring_scroll,
    );
}

/// Index names selectable on the Items tab: the table itself first, then every
/// secondary index in the order the Indexes tab lists them.
pub fn item_index_options(t: &DdbTable) -> Vec<String> {
    let mut options = vec![format!("[Table] {}", t.name)];
    options.extend(
        t.global_secondary_indexes
            .iter()
            .chain(t.local_secondary_indexes.iter())
            .map(|i| format!("[Index] {}", i.name)),
    );
    options
}

/// The index name to scan or query, or `None` for the base table.
pub fn selected_item_index(t: &DdbTable, idx: usize) -> Option<String> {
    if idx == 0 {
        return None;
    }
    t.global_secondary_indexes
        .iter()
        .chain(t.local_secondary_indexes.iter())
        .nth(idx - 1)
        .map(|i| i.name.clone())
}

/// Strip the "(S)" type suffix a key label carries: "order_id (S)" -> "order_id".
pub fn key_attribute_name(label: &str) -> &str {
    label.split(" (").next().unwrap_or(label).trim()
}

/// Attribute type letter from a key label: "order_id (N)" -> "N".
pub fn key_attribute_type(label: &str) -> String {
    label
        .rsplit_once('(')
        .and_then(|(_, rest)| rest.split(')').next())
        .unwrap_or("S")
        .to_string()
}

/// Partition-key label ("customer_id (S)") of the selected table or index.
pub fn selected_partition_key_label(t: &DdbTable, idx: usize) -> String {
    if idx == 0 {
        return t.partition_key.clone();
    }
    t.global_secondary_indexes
        .iter()
        .chain(t.local_secondary_indexes.iter())
        .nth(idx - 1)
        .map(|i| i.partition_key.clone())
        .unwrap_or_else(|| t.partition_key.clone())
}

/// Key attributes of the selected table or index, used for the "Keys only"
/// projection and for ordering the item columns.
pub fn selected_key_attributes(t: &DdbTable, idx: usize) -> Vec<String> {
    let (pk, sk) = if idx == 0 {
        (t.partition_key.clone(), t.sort_key.clone())
    } else {
        match t
            .global_secondary_indexes
            .iter()
            .chain(t.local_secondary_indexes.iter())
            .nth(idx - 1)
        {
            Some(i) => (i.partition_key.clone(), i.sort_key.clone()),
            None => (t.partition_key.clone(), t.sort_key.clone()),
        }
    };
    let mut keys = Vec::new();
    for label in [pk, sk] {
        if !label.is_empty() {
            keys.push(key_attribute_name(&label).to_string());
        }
    }
    keys
}

/// Items matching the tab's text filter. For Query the text is the partition-key
/// value (applied server side), so only Scan filters client side.
pub fn filtered_items(app: &App) -> Vec<&DdbItem> {
    // Query applies the partition-key value server side; only Scan filters here.
    // Short-circuit Query to preserve "no client-side filtering" semantics even
    // when the shared filter text is non-empty.
    if app.dynamodb_state.item_operation == ItemOperation::Query {
        return app.dynamodb_state.items.items.iter().collect();
    }
    filter_by_fields(
        &app.dynamodb_state.items.items,
        &app.dynamodb_state.item_filter,
        |i| {
            i.attributes
                .iter()
                .flat_map(|(k, v)| [k.as_str(), v.as_str()])
        },
    )
}

/// One column per attribute, keys first then the remaining names alphabetically.
struct ItemCol {
    label: String,
}

impl Column<DdbItem> for ItemCol {
    fn name(&self) -> std::borrow::Cow<'_, str> {
        self.label.as_str().into()
    }
    fn width(&self) -> u16 {
        (self.label.len().max(18) as u16).min(40)
    }
    fn render(&self, item: &DdbItem) -> (String, Style) {
        let value = item.get(&self.label);
        if value.is_empty() {
            ("-".to_string(), Style::default().fg(Color::DarkGray))
        } else {
            (value.to_string(), Style::default())
        }
    }
}

fn item_columns(t: &DdbTable, app: &App) -> Vec<Box<dyn Column<DdbItem>>> {
    item_visible_column_names(t, app)
        .into_iter()
        .map(|label| Box::new(ItemCol { label }) as Box<dyn Column<DdbItem>>)
        .collect()
}

/// Every attribute name across the loaded items: the keys of the selected table
/// or index first (in key order), then the remaining names alphabetically.
///
/// This is the actual column list of the DynamoDB table being explored, and it
/// is what both the Items table and its Preferences panel are built from — so
/// the toggles always match the columns on screen. Names come from the whole
/// result set rather than the current page, keeping the layout stable while
/// paging.
pub fn item_column_names(t: &DdbTable, app: &App) -> Vec<String> {
    let mut names = selected_key_attributes(t, app.dynamodb_state.item_index);
    let mut rest: Vec<String> = Vec::new();
    for item in &app.dynamodb_state.items.items {
        for attr in item.attributes.keys() {
            if !names.contains(attr) && !rest.contains(attr) {
                rest.push(attr.clone());
            }
        }
    }
    rest.sort();
    names.extend(rest);
    names
}

/// [`item_column_names`] minus the ones hidden in Preferences. Hiding every
/// column would leave an empty table, so an all-hidden set falls back to all.
pub fn item_visible_column_names(t: &DdbTable, app: &App) -> Vec<String> {
    let all = item_column_names(t, app);
    let visible: Vec<String> = all
        .iter()
        .filter(|n| !app.dynamodb_state.item_hidden_columns.contains(n))
        .cloned()
        .collect();
    if visible.is_empty() {
        all
    } else {
        visible
    }
}

fn render_items_tab(frame: &mut Frame, app: &App, t: &DdbTable, area: Rect) {
    use crate::common::render_dropdown;
    use crate::ui::filter::{render_filter_bar, FilterConfig, FilterControl};

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let state = &app.dynamodb_state;
    let index_options = item_index_options(t);
    let index_label = index_options
        .get(state.item_index)
        .cloned()
        .unwrap_or_else(|| index_options[0].clone());

    let view = state
        .items
        .prepare_view(filtered_items(app), TableWindow::SelectedPage);
    let pagination = if items::results_complete(app) {
        render_pagination_text(view.current_page, view.total_pages)
    } else {
        render_partial_pagination(view.current_page, view.total_pages)
    };

    let placeholder = if state.item_operation == ItemOperation::Query {
        "Enter a partition key value to query"
    } else {
        "Filter the scanned items"
    };

    render_filter_bar(
        frame,
        FilterConfig {
            filter_text: &state.item_filter,
            placeholder,
            mode: app.mode,
            is_input_focused: state.input_focus == InputFocus::Filter,
            controls: vec![
                FilterControl {
                    text: state.item_operation.name().to_string(),
                    is_focused: state.input_focus == ITEM_OPERATION,
                },
                FilterControl {
                    text: index_label,
                    is_focused: state.input_focus == ITEM_INDEX,
                },
                FilterControl {
                    text: state.item_projection.name().to_string(),
                    is_focused: state.input_focus == ITEM_PROJECTION,
                },
                FilterControl {
                    text: pagination.clone(),
                    is_focused: state.input_focus == InputFocus::Pagination,
                },
            ],
            area: chunks[0],
        },
    );

    if state.items_loading && !state.items_append {
        let block = crate::ui::titled_block(t.name.clone());
        let inner = block.inner(chunks[1]);
        frame.render_widget(block, chunks[1]);
        let p = Paragraph::new("Loading items...")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(p, inner);
        return;
    }

    // Query needs a partition-key value before it can return anything.
    if state.item_operation == ItemOperation::Query && state.item_filter.is_empty() {
        let block = crate::ui::titled_block(t.name.clone());
        let inner = block.inner(chunks[1]);
        frame.render_widget(block, chunks[1]);
        let key = selected_key_attributes(t, state.item_index)
            .first()
            .cloned()
            .unwrap_or_default();
        let p = Paragraph::new(format!("Enter a value for {} to run the query", key))
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(p, inner);
        return;
    }

    let columns = item_columns(t, app);

    let config = TableConfig {
        items: view.items,
        selected_index: view.selected_index,
        expanded_index: view.expanded_index,
        columns: &columns,
        sort_column: "",
        sort_direction: SortDirection::Asc,
        title: format_title(&t.name),
        area: chunks[1],
        get_expanded_content: Some(Box::new(|i: &DdbItem| expanded_from_columns(&columns, i))),
        is_active: app.mode != crate::keymap::Mode::FilterInput,
    };
    render_table(frame, config);

    // Dropdown overlays render last so they sit on top of the table. An open
    // dropdown always lists every option so the selection can be changed.
    {
        let after_operation = (index_label_width(&index_options, state.item_index)
            + state.item_projection.name().len()) as u16
            + pagination.len() as u16
            + 9;
        if state.input_focus == ITEM_OPERATION {
            let names: Vec<&str> = ItemOperation::ALL.iter().map(|o| o.name()).collect();
            let selected = ItemOperation::ALL
                .iter()
                .position(|o| *o == state.item_operation)
                .unwrap_or(0);
            render_dropdown(frame, &names, selected, chunks[0], after_operation);
        } else if state.input_focus == ITEM_INDEX {
            let after = (state.item_projection.name().len() + pagination.len()) as u16 + 6;
            render_dropdown(
                frame,
                &index_options,
                state.item_index.min(index_options.len().saturating_sub(1)),
                chunks[0],
                after,
            );
        } else if state.input_focus == ITEM_PROJECTION {
            let names: Vec<&str> = ItemProjection::ALL.iter().map(|p| p.name()).collect();
            let selected = ItemProjection::ALL
                .iter()
                .position(|p| *p == state.item_projection)
                .unwrap_or(0);
            let after = pagination.len() as u16 + 3;
            render_dropdown(frame, &names, selected, chunks[0], after);
        }
    }
}

/// Rendered width of the index control, used to place the dropdown overlay.
fn index_label_width(options: &[String], selected: usize) -> usize {
    options
        .get(selected)
        .or_else(|| options.first())
        .map(|s| s.len())
        .unwrap_or(0)
}

fn render_tags_tab(frame: &mut Frame, app: &App, area: Rect) {
    tags::render(
        frame,
        area,
        TagsConfig {
            table: &app.dynamodb_state.tags_table,
            items: &app.dynamodb_state.tags_table.items,
            columns: tags::columns(),
            sort: TagSort::Key,
            window: TableWindow::SelectedPage,
            focus: app.dynamodb_state.input_focus,
            mode: app.mode,
            loading: app.dynamodb_state.tags_loading,
        },
    );
}

pub fn render_tables_list(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let view = app
        .dynamodb_state
        .tables
        .prepare_view(filtered_dynamodb_tables(app), TableWindow::SelectedPage);
    let pagination = render_pagination_text(view.current_page, view.total_pages);

    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &app.dynamodb_state.tables.filter,
            placeholder: "Search by table name, status, or partition key",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.dynamodb_state.input_focus == InputFocus::Filter,
            is_pagination_focused: app.dynamodb_state.input_focus == InputFocus::Pagination,
        },
    );

    let title = format_title(&format!("Tables ({})", view.total_items));

    let columns: Vec<Box<dyn Column<DdbTable>>> = app
        .dynamodb_visible_column_ids
        .iter()
        .filter_map(|col_id| {
            table::Column::from_id(col_id).map(|col| Box::new(col) as Box<dyn Column<DdbTable>>)
        })
        .collect();

    let config = TableConfig {
        items: view.items,
        selected_index: view.selected_index,
        expanded_index: view.expanded_index,
        columns: &columns,
        sort_column: "Name",
        sort_direction: SortDirection::Asc,
        title,
        area: chunks[1],
        get_expanded_content: Some(Box::new(|t: &DdbTable| expanded_from_columns(&columns, t))),
        is_active: app.mode != crate::keymap::Mode::FilterInput,
    };

    render_table(frame, config);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusticity_core::dynamodb::DdbIndex;

    fn make_table(name: &str, pk: &str) -> DdbTable {
        DdbTable {
            name: name.to_string(),
            status: "ACTIVE".to_string(),
            partition_key: pk.to_string(),
            read_capacity_mode: "On-demand".to_string(),
            write_capacity_mode: "On-demand".to_string(),
            table_class: "Standard".to_string(),
            deletion_protection: "Disabled".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_prepared_items_second_page_and_stale_selection() {
        use crate::common::PageSize;
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".into());
        app.dynamodb_state.detail_tab = DetailTab::Items;
        app.dynamodb_state.items.items = (0..25)
            .map(|i| item_with(&[("order_id", &format!("record{i:02}")), ("status", "MATCH")]))
            .collect();
        app.dynamodb_state
            .items
            .items
            .push(item_with(&[("order_id", "excluded")]));
        app.dynamodb_state.item_filter = "match".into();
        app.dynamodb_state.items.page_size = PageSize::Ten;
        app.dynamodb_state.items.selected = 12;
        app.dynamodb_state.items.expanded_item = Some(11);
        app.dynamodb_state.items.next_token = Some("unchanged-server-cursor".into());

        let content = render_to_string(&app, 200, 45);
        assert!(content.contains(&render_pagination_text(1, 3)));
        assert!(content.contains("record10"));
        assert!(content.contains("record19"));
        assert!(content.contains("▼ record11"));
        assert!(content.contains("► record12"));
        assert!(!content.contains("record09"));
        assert!(!content.contains("record20"));
        assert!(!content.contains("excluded"));

        // A narrower result set can arrive before navigation state is reset.
        app.dynamodb_state.item_filter = "missing".into();
        let content = render_to_string(&app, 200, 45);
        assert!(!content.contains("record12"));
        assert_eq!(app.dynamodb_state.items.selected, 12);
        assert_eq!(app.dynamodb_state.items.expanded_item, Some(11));
        assert_eq!(
            app.dynamodb_state.items.next_token.as_deref(),
            Some("unchanged-server-cursor")
        );
        assert!(!app.dynamodb_state.items_loading);
    }

    #[test]
    fn test_summary_and_general_borders_follow_content_without_blank_rows() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".into());
        app.dynamodb_state.detail_tab = DetailTab::Settings;
        app.dynamodb_state.tables.items[0].creation_time = "2026-07-27T15:11:44Z".into();
        app.dynamodb_state.tables.items[0].encryption_type = "Owned by Amazon DynamoDB".into();
        app.dynamodb_state.settings.ttl_status = "Enabled".into();
        app.dynamodb_state.settings.ttl_attribute = "expires_at".into();

        for width in [90, 160, 300] {
            let mut terminal = Terminal::new(TestBackend::new(width, 60)).unwrap();
            // Also exercise a pane that does not start at the terminal origin.
            let area = Rect::new(2, 1, width - 4, 58);
            terminal
                .draw(|f| render_table_detail(f, &app, area))
                .unwrap();
            let buffer = terminal.backend().buffer();
            let rows: Vec<String> = (area.y..area.bottom())
                .map(|y| {
                    (area.x..area.right())
                        .map(|x| buffer[(x, y)].symbol())
                        .collect()
                })
                .collect();
            for title in ["Summary", "General information"] {
                let top = rows.iter().position(|row| row.contains(title)).unwrap();
                let bottom = (top + 1..rows.len())
                    .find(|&y| rows[y].starts_with('╰'))
                    .unwrap();
                assert!(bottom > top + 1);
                assert!(
                    rows[bottom - 1]
                        .chars()
                        .any(|c| !c.is_whitespace() && c != '│'),
                    "{title} has a blank row above its bottom border at width {width}"
                );
                for row in &rows[top + 1..bottom] {
                    assert!(!row.contains('╰'), "Content overlaps a border: {row}");
                }
            }
            let content = rows.join("\n");
            for label in [
                "Table name:",
                "Status:",
                "Partition key:",
                "Sort key:",
                "Read capacity mode:",
                "Write capacity mode:",
                "Table class:",
                "Total size:",
                "Item count:",
                "Indexes:",
                "Creation time:",
                "ARN:",
                "Table ID:",
                "Encryption type:",
                "KMS key ARN:",
                "Time to Live (TTL):",
                "TTL attribute:",
            ] {
                assert!(content.contains(label), "Missing {label} at width {width}");
            }
            assert_eq!(content.matches("Deletion protection:").count(), 2);
            let summary_bottom = rows.iter().position(|row| row.starts_with('╰')).unwrap();
            assert!(rows[summary_bottom + 1].starts_with("Items"));
            assert!(rows[summary_bottom + 2].contains("General information"));
            let general_top = rows
                .iter()
                .position(|row| row.contains("General information"))
                .unwrap();
            let general_bottom = (general_top + 1..rows.len())
                .find(|&y| rows[y].starts_with('╰'))
                .unwrap();
            assert!(rows[general_bottom + 1].contains("Capacity"));
            if width == 300 {
                assert_eq!(summary_bottom, 5, "Four content rows plus two borders");
                assert_eq!(
                    general_bottom - general_top,
                    4,
                    "Three content rows plus two borders"
                );
            }
        }
    }

    #[test]
    fn test_summary_and_general_render_in_tiny_viewports() {
        use ratatui::{backend::TestBackend, Terminal};
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".into());
        app.dynamodb_state.detail_tab = DetailTab::Settings;
        for width in [1, 2, 12, 80] {
            for height in [1, 2, 3, 8] {
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                terminal
                    .draw(|f| render_table_detail(f, &app, f.area()))
                    .unwrap();
            }
        }
    }

    fn items_rows(app: &App) -> Vec<String> {
        use ratatui::{backend::TestBackend, Terminal};
        let mut terminal = Terminal::new(TestBackend::new(220, 20)).unwrap();
        let table = current_table(app).unwrap();
        terminal
            .draw(|f| render_items_tab(f, app, table, f.area()))
            .unwrap();
        let buffer = terminal.backend().buffer();
        (0..20)
            .map(|y| (0..220).map(|x| buffer[(x, y)].symbol()).collect())
            .collect()
    }

    #[test]
    fn test_items_plain_title_has_no_result_summary_or_status_row() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".into());
        app.dynamodb_state.items.items = (0..152)
            .map(|i| item_with(&[("order_id", &format!("record{i:03}"))]))
            .collect();
        app.dynamodb_state.items_context = items::context(&app);
        let rows = items_rows(&app);
        // Filter consumes three rows; the table border is immediately next.
        assert!(rows[3].starts_with("╭─ orders ─"));
        let border = rows[3].strip_prefix("╭─ orders ─").unwrap();
        assert!(border.strip_suffix('╮').unwrap().chars().all(|c| c == '─'));
        assert!(rows[4].contains("order_id"));
        assert!(rows[5].contains("record000"));
        let screen = rows.join("\n");
        assert!(!screen.contains("loaded"));
        assert!(!screen.contains("matching"));
        assert!(!screen.contains("No more pages"));
    }

    #[test]
    fn test_items_placeholders_keep_messages_in_body_not_title() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".into());
        app.dynamodb_state.items_loading = true;
        let rows = items_rows(&app);
        assert!(rows[3].starts_with("╭─ orders ─"));
        assert!(!rows[3].contains("Loading"));
        assert!(rows[4].contains("Loading items..."));

        app.dynamodb_state.items_loading = false;
        app.dynamodb_state.item_operation = ItemOperation::Query;
        let rows = items_rows(&app);
        assert!(rows[3].starts_with("╭─ orders ─"));
        assert!(!rows[3].contains("query"));
        assert!(rows[4].contains("to run the query"));
    }

    #[test]
    fn test_state_default() {
        let state = State::new();
        assert_eq!(state.input_focus, InputFocus::Filter);
        assert!(state.tables.items.is_empty());
        assert_eq!(state.current_table, None);
        assert_eq!(state.detail_tab, DetailTab::Items);
        assert!(state.item_hidden_columns.is_empty());
    }

    #[test]
    fn test_filtered_by_name_status_partition_key() {
        let mut app = App::new_without_client("default".to_string(), None);
        app.dynamodb_state.tables.items = vec![
            make_table("orders", "order_id (S)"),
            make_table("users", "user_id (S)"),
        ];

        app.dynamodb_state.tables.filter = "ord".to_string();
        assert_eq!(filtered_dynamodb_tables(&app).len(), 1);

        app.dynamodb_state.tables.filter = "user_id".to_string();
        assert_eq!(filtered_dynamodb_tables(&app).len(), 1);

        app.dynamodb_state.tables.filter = "active".to_string();
        assert_eq!(filtered_dynamodb_tables(&app).len(), 2);

        app.dynamodb_state.tables.filter = String::new();
        assert_eq!(filtered_dynamodb_tables(&app).len(), 2);
    }

    #[test]
    fn test_detail_tab_cycles_through_all_tabs() {
        let mut tab = DetailTab::Items;
        let mut seen = vec![tab];
        for _ in 0..DetailTab::ALL.len() - 1 {
            tab = tab.next();
            seen.push(tab);
        }
        assert_eq!(seen.len(), 9);
        assert_eq!(tab.next(), DetailTab::Items);
        assert_eq!(DetailTab::ALL.len(), 9);
        // Items then Settings lead; Monitor follows Indexes; Tags is last.
        assert_eq!(DetailTab::ALL[0], DetailTab::Items);
        assert_eq!(DetailTab::ALL[1], DetailTab::Settings);
        assert_eq!(DetailTab::ALL[2], DetailTab::Indexes);
        assert_eq!(DetailTab::ALL[3], DetailTab::Monitor);
        assert_eq!(DetailTab::ALL[DetailTab::ALL.len() - 1], DetailTab::Tags);
    }

    #[test]
    fn test_console_tab_ids_are_unique() {
        let mut ids: Vec<&str> = DetailTab::ALL.iter().map(|t| t.console_tab()).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count);
    }

    #[test]
    fn test_format_capacity() {
        assert_eq!(format_capacity("On-demand", 0, "RCU"), "On-demand");
        assert_eq!(
            format_capacity("Provisioned", 5, "RCU"),
            "Provisioned (5 RCU)"
        );
        // Provisioned with no units falls back to the bare mode.
        assert_eq!(format_capacity("Provisioned", 0, "WCU"), "Provisioned");
    }

    #[test]
    fn test_enabled_disabled_color() {
        assert_eq!(enabled_disabled_color("Enabled"), Color::Green);
        assert_eq!(enabled_disabled_color("Disabled"), Color::Yellow);
        assert_eq!(enabled_disabled_color(""), Color::White);
    }

    #[test]
    fn test_summary_includes_keys_capacity_and_indexes() {
        let mut t = make_table("orders", "order_id (S)");
        t.sort_key = "created_at (N)".to_string();
        t.read_capacity_mode = "Provisioned".to_string();
        t.provisioned_read_units = 5;
        t.write_capacity_mode = "Provisioned".to_string();
        t.provisioned_write_units = 7;
        t.total_size_bytes = 2048;
        t.item_count = 11;
        t.global_secondary_indexes = vec![DdbIndex::default()];

        let text: String = build_summary_lines(&t)
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect::<Vec<_>>()
            .join("");
        assert!(text.contains("order_id (S)"));
        assert!(text.contains("created_at (N)"));
        assert!(text.contains("Provisioned (5 RCU)"));
        assert!(text.contains("Provisioned (7 WCU)"));
        assert!(text.contains("Standard"));
    }

    #[test]
    fn test_summary_shows_dash_when_no_sort_key() {
        let t = make_table("orders", "order_id (S)");
        let text: String = build_summary_lines(&t)
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect::<Vec<_>>()
            .join("");
        assert!(text.contains("Sort key"));
        assert!(text.contains('-'));
    }

    #[test]
    fn test_detail_fixed_columns() {
        assert_eq!(
            detail_fixed_columns(DetailTab::Indexes),
            Some(&INDEXES_COLUMNS[..])
        );
        assert_eq!(
            detail_fixed_columns(DetailTab::GlobalTables),
            Some(&REPLICAS_COLUMNS[..])
        );
        assert_eq!(
            detail_fixed_columns(DetailTab::Backups),
            Some(&BACKUPS_COLUMNS[..])
        );
        assert_eq!(
            detail_fixed_columns(DetailTab::Tags),
            Some(&TAGS_COLUMNS[..])
        );
        assert_eq!(detail_fixed_columns(DetailTab::Settings), None);
        assert_eq!(detail_fixed_columns(DetailTab::Monitor), None);
        assert_eq!(detail_fixed_columns(DetailTab::AdditionalSettings), None);
    }

    #[test]
    fn test_current_table_lookup() {
        let mut app = App::new_without_client("default".to_string(), None);
        app.dynamodb_state.tables.items = vec![make_table("orders", "pk (S)")];
        assert!(current_table(&app).is_none());
        app.dynamodb_state.current_table = Some("orders".to_string());
        assert_eq!(current_table(&app).map(|t| t.name.as_str()), Some("orders"));
    }

    /// Render the whole UI to a test backend and return the flattened content.
    fn render_to_string(app: &App, width: u16, height: u16) -> String {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| crate::ui::render(f, app)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>()
    }

    fn app_with_table() -> App {
        let mut app = App::new_without_client("default".to_string(), None);
        app.current_service = crate::app::Service::DynamoDbTables;
        app.service_selected = true;
        app.mode = crate::keymap::Mode::Normal;
        let mut t = make_table("orders", "order_id (S)");
        t.sort_key = "created_at (N)".to_string();
        t.table_arn = "arn:aws:dynamodb:us-east-2:123456789012:table/orders".to_string();
        t.global_secondary_indexes = vec![DdbIndex {
            name: "customer-index".to_string(),
            index_type: "Global".to_string(),
            status: "ACTIVE".to_string(),
            partition_key: "customer_id (S)".to_string(),
            projection: "KEYS_ONLY".to_string(),
            ..Default::default()
        }];
        app.dynamodb_state.tables.items = vec![t];
        app
    }

    /// A `DdbItem` with the given attribute pairs.
    fn item_with(attrs: &[(&str, &str)]) -> DdbItem {
        let mut item = DdbItem::default();
        for (k, v) in attrs {
            item.attributes.insert(k.to_string(), v.to_string());
        }
        item
    }

    #[test]
    fn test_render_tables_list() {
        let app = app_with_table();
        // Wide enough for all 12 default columns.
        let content = render_to_string(&app, 240, 30);
        assert!(content.contains("Tables (1)"));
        assert!(content.contains("orders"));
        assert!(content.contains("Partition key"));
        assert!(content.contains("Replication Regions"));
        assert!(content.contains("Table class"));
    }

    #[test]
    fn test_render_detail_every_tab() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        for tab in DetailTab::ALL {
            app.dynamodb_state.detail_tab = *tab;
            let content = render_to_string(&app, 160, 40);
            // The ribbon shows on every tab; Items is the only one without the
            // Summary pane.
            assert!(content.contains(tab.name()), "tab {:?}", tab);
            if *tab != DetailTab::Items {
                assert!(content.contains("Summary"), "tab {:?}", tab);
            } else {
                assert!(!content.contains("Summary"), "tab {:?}", tab);
            }
        }
    }

    #[test]
    fn test_render_indexes_tab_shows_index_rows() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Indexes;
        let content = render_to_string(&app, 200, 40);
        assert!(content.contains("Indexes (1)"));
        assert!(content.contains("customer-index"));
    }

    #[test]
    fn test_preferences_uses_tab_columns_not_list_columns() {
        // Regression: detail tabs must not expose the Tables list columns in the
        // preferences selector.
        let mut app = app_with_table();
        app.mode = crate::keymap::Mode::ColumnSelector;

        // List view → table columns.
        let content = render_to_string(&app, 160, 40);
        assert!(content.contains("Partition key"));
        assert!(content.contains("Read capacity mode"));

        // Indexes tab → the 10 fixed index columns (the summary pane behind the
        // popup still shows table fields, so assert on the counts).
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Indexes;
        assert_eq!(crate::dynamodb::actions::column_count(&app), 10);
        assert_eq!(crate::dynamodb::actions::column_selector_max(&app), 10 + 6);

        // Global tables tab → the 5 replica columns only.
        app.dynamodb_state.detail_tab = DetailTab::GlobalTables;
        assert_eq!(crate::dynamodb::actions::column_count(&app), 5);
        let content = render_to_string(&app, 160, 40);
        assert!(content.contains("Region"));

        // A non-table tab falls back to the list columns.
        app.dynamodb_state.detail_tab = DetailTab::Settings;
        assert_eq!(
            crate::dynamodb::actions::column_count(&app),
            app.dynamodb_column_ids.len()
        );
    }

    #[test]
    fn test_detail_tab_change_marks_lazy_data_for_reload() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::GlobalTables;

        // Backups needs both the backup list and the PITR settings.
        crate::dynamodb::actions::next_detail_tab(&mut app);
        assert_eq!(app.dynamodb_state.detail_tab, DetailTab::Backups);
        assert!(app.dynamodb_state.backups_loading);
        assert!(app.dynamodb_state.settings_loading);
        app.dynamodb_state.settings_loading = false;

        crate::dynamodb::actions::next_detail_tab(&mut app);
        assert_eq!(app.dynamodb_state.detail_tab, DetailTab::ExportsStreams);

        // Additional settings loads the resource-based policies.
        crate::dynamodb::actions::next_detail_tab(&mut app);
        assert_eq!(app.dynamodb_state.detail_tab, DetailTab::AdditionalSettings);
        assert!(app.dynamodb_state.permissions_loading);

        // Tags loads the tag list.
        crate::dynamodb::actions::next_detail_tab(&mut app);
        assert_eq!(app.dynamodb_state.detail_tab, DetailTab::Tags);
        assert!(app.dynamodb_state.tags_loading);

        // Wrapping around to Items schedules a scan, then Settings loads TTL.
        crate::dynamodb::actions::next_detail_tab(&mut app);
        assert_eq!(app.dynamodb_state.detail_tab, DetailTab::Items);
        assert!(app.dynamodb_state.items_loading);

        crate::dynamodb::actions::next_detail_tab(&mut app);
        assert_eq!(app.dynamodb_state.detail_tab, DetailTab::Settings);
        assert!(app.dynamodb_state.settings_loading);
    }

    #[test]
    fn test_tags_tab_renders_tag_rows() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Tags;
        app.dynamodb_state.tags_table.items = vec![
            ("Name".to_string(), "rusticity-test".to_string()),
            ("Owner".to_string(), "azzhipa".to_string()),
        ];
        let content = render_to_string(&app, 200, 40);
        assert!(content.contains("Tags (2)"));
        assert!(content.contains("rusticity-test"));
        assert_eq!(crate::dynamodb::actions::column_count(&app), 2);

        app.dynamodb_state.tags_loading = true;
        let content = render_to_string(&app, 200, 40);
        assert!(content.contains("Loading tags"));
    }

    #[test]
    fn test_timestamps_use_the_common_utc_formatter() {
        // Raw SDK timestamps must never reach the screen.
        let mut t = make_table("orders", "order_id (S)");
        t.creation_time = "2026-07-27T15:11:44.813999Z".to_string();
        let text: String = build_summary_lines(&t)
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect::<Vec<_>>()
            .join("");
        assert!(text.contains("2026-07-27 15:11:44 (UTC)"), "{}", text);
        assert!(!text.contains("2026-07-27T15:11:44.813999Z"));

        // The Backups pane formats the PITR restore points too.
        let mut app = app_with_table();
        app.dynamodb_state.tables.items[0].creation_time = t.creation_time.clone();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Backups;
        app.dynamodb_state.settings = DdbTableSettings {
            pitr_status: "Enabled".to_string(),
            recovery_period_days: 35,
            earliest_restorable_time: "2026-07-27T15:12:15Z".to_string(),
            latest_restorable_time: "2026-07-27T15:44:10.181999Z".to_string(),
            ..Default::default()
        };
        let content = render_to_string(&app, 200, 40);
        assert!(content.contains("2026-07-27 15:12:15 (UTC)"));
        assert!(content.contains("2026-07-27 15:44:10 (UTC)"));
        assert!(!content.contains(".181999Z"));
    }

    #[test]
    fn test_selecting_a_table_opens_items_and_schedules_a_scan() {
        let mut app = app_with_table();
        crate::dynamodb::actions::select_item(&mut app);
        assert_eq!(app.dynamodb_state.current_table.as_deref(), Some("orders"));
        // Items is the landing tab and its scan is queued right away.
        assert_eq!(app.dynamodb_state.detail_tab, DetailTab::Items);
        assert!(app.dynamodb_state.items_loading);
        assert!(needs_items_load(&app));
        // TTL is only fetched once the Settings tab is opened.
        assert!(!app.dynamodb_state.settings_loading);

        crate::dynamodb::actions::next_detail_tab(&mut app);
        assert_eq!(app.dynamodb_state.detail_tab, DetailTab::Settings);
        assert!(app.dynamodb_state.settings_loading);
    }

    #[test]
    fn test_backups_tab_shows_pitr_pane() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Backups;
        app.dynamodb_state.settings = DdbTableSettings {
            pitr_status: "Enabled".to_string(),
            recovery_period_days: 35,
            earliest_restorable_time: "2026-07-27T15:12:15Z".to_string(),
            latest_restorable_time: "2026-07-27T15:33:18Z".to_string(),
            ..Default::default()
        };
        let content = render_to_string(&app, 200, 40);
        assert!(content.contains("Point-in-time recovery (PITR)"));
        assert!(content.contains("35 days"));
        assert!(content.contains("Earliest restore point"));
        assert!(content.contains("Latest restore point"));
    }

    #[test]
    fn test_global_tables_tab_shows_settings_replication() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::GlobalTables;
        let content = render_to_string(&app, 200, 40);
        assert!(content.contains("Settings replication"));
        assert!(content.contains("Disabled"));
        assert!(content.contains("Replicas (0)"));
    }

    #[test]
    fn test_additional_settings_shows_policy_panes() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::AdditionalSettings;
        let content = render_to_string(&app, 200, 40);
        assert!(content.contains("Resource-based policy for table"));
        assert!(content.contains("No table resource-based policy to display"));
        assert!(content.contains("No stream resource-based policy to display"));

        app.dynamodb_state.table_policy = Some("{\n  \"Version\": \"2012-10-17\"\n}".to_string());
        let content = render_to_string(&app, 200, 40);
        assert!(content.contains("2012-10-17"));
    }

    #[test]
    fn test_settings_tab_shows_ttl() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Settings;
        app.dynamodb_state.settings = DdbTableSettings {
            ttl_status: "Enabled".to_string(),
            ttl_attribute: "expires_at".to_string(),
            ..Default::default()
        };
        let content = render_to_string(&app, 200, 40);
        assert!(content.contains("Time to Live (TTL)"));
        assert!(content.contains("expires_at"));
    }

    #[test]
    fn test_console_url_follows_active_tab() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Indexes;
        let url = crate::dynamodb::actions::console_url(&app);
        assert!(url.contains("#table?name=orders&tab=indexes"), "{}", url);

        app.dynamodb_state.detail_tab = DetailTab::Monitor;
        let url = crate::dynamodb::actions::console_url(&app);
        assert!(url.contains("tab=monitoring"), "{}", url);

        app.dynamodb_state.current_table = None;
        let url = crate::dynamodb::actions::console_url(&app);
        assert!(url.ends_with("#tables"), "{}", url);
    }

    #[test]
    fn test_monitor_tab_loads_metrics_and_scrolls() {
        use crate::ui::monitoring::MonitoringState;
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Indexes;

        // Entering Monitor flags a metrics load and resets the scroll.
        app.dynamodb_state.monitoring_scroll = 3;
        crate::dynamodb::actions::next_detail_tab(&mut app);
        assert_eq!(app.dynamodb_state.detail_tab, DetailTab::Monitor);
        assert!(app.dynamodb_state.metrics_loading);
        assert_eq!(app.dynamodb_state.monitoring_scroll, 0);

        // Ctrl+D / Ctrl+U scroll within the chart count and clamp at both ends.
        app.dynamodb_state.set_metrics_loading(false);
        for _ in 0..MONITORING_CHARTS + 2 {
            crate::dynamodb::actions::scroll_down_detail(&mut app);
        }
        assert_eq!(app.dynamodb_state.monitoring_scroll, MONITORING_CHARTS - 1);
        for _ in 0..MONITORING_CHARTS + 2 {
            crate::dynamodb::actions::scroll_up_detail(&mut app);
        }
        assert_eq!(app.dynamodb_state.monitoring_scroll, 0);
    }

    #[test]
    fn test_monitor_tab_renders_charts() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Monitor;
        app.dynamodb_state.metrics = rusticity_core::dynamodb::DdbMonitoringData {
            consumed_read_capacity: vec![(0, 1.0), (60, 2.0)],
            consumed_write_capacity: vec![(0, 0.5)],
            ..Default::default()
        };
        let content = render_to_string(&app, 200, 60);
        assert!(content.contains("Read capacity"));
        assert!(content.contains("Write capacity"));

        // While loading, the tab shows a placeholder instead of charts.
        app.dynamodb_state.metrics_loading = true;
        let content = render_to_string(&app, 200, 60);
        assert!(content.contains("Loading metrics"));
    }

    #[test]
    fn test_items_loading_pane_names_the_table() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Items;
        app.dynamodb_state.items_loading = true;
        let content = render_to_string(&app, 200, 40);
        // The pane is titled with the table name only — the ribbon already says
        // which tab is open.
        assert!(content.contains("─ orders ─"));
        assert!(!content.contains("Items returned"));
        assert!(content.contains("Loading items"));
    }

    #[test]
    fn test_items_tab_is_first_and_hides_summary() {
        assert_eq!(DetailTab::ALL[0], DetailTab::Items);

        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Items;
        let content = render_to_string(&app, 200, 40);
        // Ribbon stays, Summary pane does not.
        assert!(content.contains("Items"));
        assert!(content.contains("Settings"));
        assert!(!content.contains("Summary"));

        // Another tab still shows the Summary pane.
        app.dynamodb_state.detail_tab = DetailTab::Settings;
        let content = render_to_string(&app, 200, 40);
        assert!(content.contains("Summary"));
    }

    #[test]
    fn test_items_filter_row_controls_and_defaults() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Items;
        let content = render_to_string(&app, 200, 40);
        // Scan is the default operation; the table is the default index.
        assert!(content.contains("Scan"));
        assert!(content.contains("[Table] orders"));
        assert!(content.contains("All attributes"));

        // Focus order: operation, index, projection, filter input, pagination.
        assert_eq!(
            ITEMS_FILTER_CONTROLS,
            [
                ITEM_OPERATION,
                ITEM_INDEX,
                ITEM_PROJECTION,
                InputFocus::Filter,
                InputFocus::Pagination,
            ]
        );
        assert_eq!(app.dynamodb_state.item_operation, ItemOperation::Scan);
        assert_eq!(
            app.dynamodb_state.item_projection,
            ItemProjection::AllAttributes
        );
    }

    #[test]
    fn test_open_dropdown_lists_every_option() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Items;

        // Operation dropdown shows both Scan and Query.
        app.dynamodb_state.input_focus = ITEM_OPERATION;
        let content = render_to_string(&app, 200, 40);
        assert!(content.contains("Scan"));
        assert!(content.contains("Query"));

        // Index dropdown shows the table and every secondary index.
        app.dynamodb_state.input_focus = ITEM_INDEX;
        let content = render_to_string(&app, 200, 40);
        assert!(content.contains("[Table] orders"));
        assert!(content.contains("customer-index"));

        // Projection dropdown shows both projections.
        app.dynamodb_state.input_focus = ITEM_PROJECTION;
        let content = render_to_string(&app, 200, 40);
        assert!(content.contains("All attributes"));
        assert!(content.contains("Keys only"));
    }

    #[test]
    fn test_dropdowns_cycle_and_schedule_a_reload() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Items;

        app.dynamodb_state.input_focus = ITEM_OPERATION;
        crate::dynamodb::actions::next_dropdown_value(&mut app);
        assert_eq!(app.dynamodb_state.item_operation, ItemOperation::Query);
        // Query without a key value has nothing to run yet.
        assert!(!app.dynamodb_state.items_loading);
        crate::dynamodb::actions::prev_dropdown_value(&mut app);
        assert_eq!(app.dynamodb_state.item_operation, ItemOperation::Scan);
        assert!(app.dynamodb_state.items_loading);

        // The index dropdown wraps across the table plus its one index.
        app.dynamodb_state.input_focus = ITEM_INDEX;
        crate::dynamodb::actions::next_dropdown_value(&mut app);
        assert_eq!(app.dynamodb_state.item_index, 1);
        crate::dynamodb::actions::next_dropdown_value(&mut app);
        assert_eq!(app.dynamodb_state.item_index, 0);

        app.dynamodb_state.input_focus = ITEM_PROJECTION;
        crate::dynamodb::actions::next_dropdown_value(&mut app);
        assert_eq!(app.dynamodb_state.item_projection, ItemProjection::KeysOnly);
    }

    #[test]
    fn test_items_render_dynamic_attribute_columns() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Items;
        let mut item = DdbItem::default();
        item.attributes
            .insert("order_id".to_string(), "o1".to_string());
        item.attributes
            .insert("created_at".to_string(), "1".to_string());
        item.attributes
            .insert("status".to_string(), "SHIPPED".to_string());
        app.dynamodb_state.items.items = vec![item];

        let content = render_to_string(&app, 200, 40);
        // The pane title is the table name on its own.
        assert!(content.contains("─ orders ─"));
        assert!(!content.contains("Items returned"));
        assert!(content.contains("order_id"));
        assert!(content.contains("SHIPPED"));

        // Client-side filtering applies to Scan results and uses the Items tab's
        // own filter, not the table-list filter.
        app.dynamodb_state.tables.filter = "nomatch".to_string();
        assert_eq!(filtered_items(&app).len(), 1);
        app.dynamodb_state.item_filter = "shipped".to_string();
        assert_eq!(filtered_items(&app).len(), 1);
        app.dynamodb_state.item_filter = "nomatch".to_string();
        assert_eq!(filtered_items(&app).len(), 0);
    }

    #[test]
    fn test_item_column_names_are_keys_first_then_sorted() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Items;
        app.dynamodb_state.items.items = vec![
            item_with(&[("status", "SHIPPED"), ("order_id", "o1")]),
            item_with(&[("amount", "12"), ("created_at", "1"), ("order_id", "o2")]),
        ];

        let t = current_table(&app).unwrap();
        // Partition key, then sort key, then the rest alphabetically — and the
        // set spans every loaded item, not just the visible page.
        assert_eq!(
            item_column_names(t, &app),
            vec!["order_id", "created_at", "amount", "status"]
        );
    }

    #[test]
    fn test_items_preferences_lists_the_tables_own_columns() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Items;
        app.dynamodb_state.items.items = vec![item_with(&[
            ("order_id", "o1"),
            ("created_at", "1"),
            ("status", "SHIPPED"),
        ])];

        // Four attributes: the two keys plus status (created_at is the sort key).
        assert_eq!(crate::dynamodb::actions::column_count(&app), 3);
        assert_eq!(crate::dynamodb::actions::column_selector_max(&app), 9);

        app.mode = crate::keymap::Mode::ColumnSelector;
        let content = render_to_string(&app, 200, 40);
        assert!(content.contains("Preferences"));
        assert!(content.contains("status"));
        assert!(content.contains("Page size"));
        // The Tables list columns must not leak into the Items preferences.
        assert!(!content.contains("Replication Regions"));
        assert!(!content.contains("Table class"));
    }

    #[test]
    fn test_toggling_an_item_column_hides_it_but_keeps_one_visible() {
        use crate::dynamodb::actions::toggle_column;
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Items;
        app.dynamodb_state.items.items = vec![item_with(&[
            ("order_id", "o1"),
            ("created_at", "1"),
            ("status", "SHIPPED"),
        ])];
        let names = |app: &App| item_visible_column_names(current_table(app).unwrap(), app);
        assert_eq!(names(&app), vec!["order_id", "created_at", "status"]);

        // Index 0 is the "Columns" header; columns start at 1.
        app.column_selector_index = 3;
        toggle_column(&mut app);
        assert_eq!(app.dynamodb_state.item_hidden_columns, vec!["status"]);
        assert_eq!(names(&app), vec!["order_id", "created_at"]);
        let content = render_to_string(&app, 200, 40);
        assert!(!content.contains("SHIPPED"));

        // Toggling again brings it back.
        toggle_column(&mut app);
        assert!(app.dynamodb_state.item_hidden_columns.is_empty());
        assert_eq!(names(&app), vec!["order_id", "created_at", "status"]);

        // The last visible column cannot be hidden.
        for idx in [1, 2, 3] {
            app.column_selector_index = idx;
            toggle_column(&mut app);
        }
        assert_eq!(app.dynamodb_state.item_hidden_columns.len(), 2);
        assert_eq!(names(&app).len(), 1);
    }

    #[test]
    fn test_items_preferences_page_size_is_actionable() {
        use crate::common::PageSize;
        use crate::dynamodb::actions::toggle_column;
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Items;
        app.dynamodb_state.items.items = vec![item_with(&[("order_id", "o1")])];

        // 2 columns (partition + sort key) -> page size radios at 5..=8.
        assert_eq!(crate::dynamodb::actions::column_count(&app), 2);
        app.column_selector_index = 7;
        toggle_column(&mut app);
        assert_eq!(app.dynamodb_state.items.page_size, PageSize::Fifty);

        app.column_selector_index = 5;
        toggle_column(&mut app);
        assert_eq!(app.dynamodb_state.items.page_size, PageSize::Ten);
    }

    #[test]
    fn test_query_needs_a_partition_key_value() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".to_string());
        app.dynamodb_state.detail_tab = DetailTab::Items;
        app.dynamodb_state.item_operation = ItemOperation::Query;

        let content = render_to_string(&app, 200, 40);
        assert!(content.contains("─ orders ─"));
        assert!(content.contains("to run the query"));
        assert!(!needs_items_load(&app));

        // Typing a value and leaving the filter input runs the query once.
        app.dynamodb_state.item_filter = "o1".to_string();
        assert!(needs_items_load(&app));
        app.dynamodb_state.executed_query = "o1".to_string();
        assert!(!needs_items_load(&app));

        // While still typing (filter input mode) the query does not re-run.
        app.dynamodb_state.item_filter = "o2".to_string();
        app.mode = crate::keymap::Mode::FilterInput;
        assert!(!needs_items_load(&app));
    }

    #[test]
    fn test_key_attribute_helpers() {
        assert_eq!(key_attribute_name("order_id (S)"), "order_id");
        assert_eq!(key_attribute_name("order_id"), "order_id");
        assert_eq!(key_attribute_type("created_at (N)"), "N");
        assert_eq!(key_attribute_type("order_id"), "S");

        let t = &app_with_table().dynamodb_state.tables.items[0].clone();
        assert_eq!(
            selected_key_attributes(t, 0),
            vec!["order_id".to_string(), "created_at".to_string()]
        );
        assert_eq!(
            selected_key_attributes(t, 1),
            vec!["customer_id".to_string()]
        );
        assert_eq!(selected_item_index(t, 0), None);
        assert_eq!(selected_item_index(t, 1).as_deref(), Some("customer-index"));

        // The query key type follows the selected table or index schema.
        assert_eq!(key_attribute_type(&selected_partition_key_label(t, 0)), "S");
        assert_eq!(key_attribute_type(&selected_partition_key_label(t, 1)), "S");
        let mut numeric = t.clone();
        numeric.partition_key = "seq (N)".to_string();
        assert_eq!(
            key_attribute_type(&selected_partition_key_label(&numeric, 0)),
            "N"
        );
    }

    #[test]
    fn test_tab_keys_reach_items_and_flag_a_load() {
        use crate::keymap::Action;
        let mut app = app_with_table();
        crate::dynamodb::actions::select_item(&mut app);
        assert_eq!(app.dynamodb_state.detail_tab, DetailTab::Items);

        // Tab moves forward off Items.
        app.handle_action(Action::NextDetailTab);
        assert_eq!(app.dynamodb_state.detail_tab, DetailTab::Settings);

        // BackTab from Settings lands on Items and schedules the scan.
        app.dynamodb_state.items_loading = false;
        app.handle_action(Action::PrevDetailTab);
        assert_eq!(app.dynamodb_state.detail_tab, DetailTab::Items);
        assert!(app.dynamodb_state.items_loading);
        assert!(needs_items_load(&app));
    }

    #[test]
    fn test_breadcrumb() {
        let mut app = app_with_table();
        assert_eq!(
            crate::dynamodb::actions::breadcrumb(&app),
            vec!["DynamoDB".to_string(), "Tables".to_string()]
        );
        app.dynamodb_state.current_table = Some("orders".to_string());
        assert_eq!(
            crate::dynamodb::actions::breadcrumb(&app),
            vec!["DynamoDB".to_string(), "orders".to_string()]
        );
    }

    #[test]
    fn tables_filter_searches_name_status_partition_key_in_source_order() {
        let mut app = App::new_without_client("default".to_string(), None);
        app.dynamodb_state.tables.items = vec![
            make_table("zeta", "pk_z (S)"),
            make_table("alpha", "pk_a (S)"),
        ];
        app.dynamodb_state.tables.items[1].status = "CREATING".to_string();

        // Partition-key match.
        app.dynamodb_state.tables.filter = "pk_z".to_string();
        let f = filtered_dynamodb_tables(&app);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].name, "zeta");

        // Status match (case-insensitive).
        app.dynamodb_state.tables.filter = "creating".to_string();
        let f = filtered_dynamodb_tables(&app);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].name, "alpha");

        // Empty filter keeps every row in source order (so the renderer's own
        // column sort still decides display order).
        app.dynamodb_state.tables.filter = String::new();
        let f = filtered_dynamodb_tables(&app);
        assert_eq!(
            f.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
            ["zeta", "alpha"]
        );
    }

    #[test]
    fn tables_filter_is_unicode_case_insensitive_and_does_not_trim() {
        let mut app = App::new_without_client("default".to_string(), None);
        app.dynamodb_state.tables.items = vec![make_table("café-orders", "id (S)")];
        app.dynamodb_state.tables.filter = "CAFÉ".to_string();
        assert_eq!(filtered_dynamodb_tables(&app).len(), 1);
        // Whitespace is significant.
        app.dynamodb_state.tables.filter = " café-orders ".to_string();
        assert!(filtered_dynamodb_tables(&app).is_empty());
    }

    #[test]
    fn items_filter_matches_dynamic_attribute_keys_and_values() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".into());
        app.dynamodb_state.item_operation = ItemOperation::Scan;
        app.dynamodb_state.items.items = vec![
            item_with(&[("order_id", "A-1"), ("status", "SHIPPED")]),
            item_with(&[("order_id", "B-2"), ("region", "us-east-1")]),
        ];

        // Value match (case-insensitive).
        app.dynamodb_state.item_filter = "shipped".into();
        let f = filtered_items(&app);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].get("order_id"), "A-1");

        // Attribute-key match — a key present on only one row.
        app.dynamodb_state.item_filter = "region".into();
        let f = filtered_items(&app);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].get("order_id"), "B-2");
    }

    #[test]
    fn items_filter_empty_returns_all_and_query_never_filters_client_side() {
        let mut app = app_with_table();
        app.dynamodb_state.current_table = Some("orders".into());
        app.dynamodb_state.items.items = vec![
            item_with(&[("order_id", "A-1")]),
            item_with(&[("order_id", "B-2")]),
        ];

        // Scan + empty filter → every row.
        app.dynamodb_state.item_operation = ItemOperation::Scan;
        app.dynamodb_state.item_filter = String::new();
        assert_eq!(filtered_items(&app).len(), 2);

        // Query ignores the client-side text filter entirely (filter applied
        // server side as the partition-key value).
        app.dynamodb_state.item_operation = ItemOperation::Query;
        app.dynamodb_state.item_filter = "no-such-value".into();
        assert_eq!(filtered_items(&app).len(), 2);
    }
}
