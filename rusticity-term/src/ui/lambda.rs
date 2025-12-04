use crate::app::App;
use crate::common::CyclicEnum;
use crate::common::{
    format_bytes, format_duration_seconds, format_memory_mb, render_pagination_text, ColumnId,
    InputFocus, SortDirection,
};
use crate::keymap::Mode;
use crate::lambda::{
    format_architecture, format_runtime, Alias, AliasColumn, Application as LambdaApplication,
    Deployment, Function as LambdaFunction, FunctionColumn as LambdaColumn, Layer, LayerColumn,
    Resource, Version, VersionColumn,
};
use crate::table::TableState;
use crate::ui::table::{expanded_from_columns, render_table, Column as TableColumn, TableConfig};
use crate::ui::{block_height, labeled_field, render_tabs, section_header, vertical};
use ratatui::{prelude::*, widgets::*};

pub const FILTER_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

pub struct State {
    pub table: TableState<LambdaFunction>,
    pub current_function: Option<String>,
    pub current_version: Option<String>,
    pub current_alias: Option<String>,
    pub detail_tab: DetailTab,
    pub version_detail_tab: VersionDetailTab,
    pub function_visible_column_ids: Vec<ColumnId>,
    pub function_column_ids: Vec<ColumnId>,
    pub version_table: TableState<Version>,
    pub version_visible_column_ids: Vec<String>,
    pub version_column_ids: Vec<String>,
    pub alias_table: TableState<Alias>,
    pub alias_visible_column_ids: Vec<String>,
    pub alias_column_ids: Vec<String>,
    pub layer_visible_column_ids: Vec<String>,
    pub layer_column_ids: Vec<String>,
    pub input_focus: InputFocus,
    pub version_input_focus: InputFocus,
    pub alias_input_focus: InputFocus,
    pub layer_selected: usize,
    pub layer_expanded: Option<usize>,
    pub monitoring_scroll: usize,
    pub metric_data_invocations: Vec<(i64, f64)>,
    pub metric_data_duration_min: Vec<(i64, f64)>,
    pub metric_data_duration_avg: Vec<(i64, f64)>,
    pub metric_data_duration_max: Vec<(i64, f64)>,
    pub metric_data_errors: Vec<(i64, f64)>,
    pub metric_data_success_rate: Vec<(i64, f64)>,
    pub metric_data_throttles: Vec<(i64, f64)>,
    pub metric_data_concurrent_executions: Vec<(i64, f64)>,
    pub metric_data_recursive_invocations_dropped: Vec<(i64, f64)>,
    pub metric_data_async_event_age_min: Vec<(i64, f64)>,
    pub metric_data_async_event_age_avg: Vec<(i64, f64)>,
    pub metric_data_async_event_age_max: Vec<(i64, f64)>,
    pub metric_data_async_events_received: Vec<(i64, f64)>,
    pub metric_data_async_events_dropped: Vec<(i64, f64)>,
    pub metric_data_destination_delivery_failures: Vec<(i64, f64)>,
    pub metric_data_dead_letter_errors: Vec<(i64, f64)>,
    pub metric_data_iterator_age: Vec<(i64, f64)>,
    pub metrics_loading: bool,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            table: TableState::new(),
            current_function: None,
            current_version: None,
            current_alias: None,
            detail_tab: DetailTab::Code,
            version_detail_tab: VersionDetailTab::Code,
            function_visible_column_ids: LambdaColumn::visible(),
            function_column_ids: LambdaColumn::ids(),
            version_table: TableState::new(),
            version_visible_column_ids: VersionColumn::all()
                .iter()
                .map(|c| c.name().to_string())
                .collect(),
            version_column_ids: VersionColumn::all()
                .iter()
                .map(|c| c.name().to_string())
                .collect(),
            alias_table: TableState::new(),
            alias_visible_column_ids: AliasColumn::all()
                .iter()
                .map(|c| c.name().to_string())
                .collect(),
            alias_column_ids: AliasColumn::all()
                .iter()
                .map(|c| c.name().to_string())
                .collect(),
            layer_visible_column_ids: LayerColumn::all()
                .iter()
                .map(|c| c.name().to_string())
                .collect(),
            layer_column_ids: LayerColumn::all()
                .iter()
                .map(|c| c.name().to_string())
                .collect(),
            input_focus: InputFocus::Filter,
            version_input_focus: InputFocus::Filter,
            alias_input_focus: InputFocus::Filter,
            layer_selected: 0,
            layer_expanded: None,
            monitoring_scroll: 0,
            metric_data_invocations: Vec::new(),
            metric_data_duration_min: Vec::new(),
            metric_data_duration_avg: Vec::new(),
            metric_data_duration_max: Vec::new(),
            metric_data_errors: Vec::new(),
            metric_data_success_rate: Vec::new(),
            metric_data_throttles: Vec::new(),
            metric_data_concurrent_executions: Vec::new(),
            metric_data_recursive_invocations_dropped: Vec::new(),
            metric_data_async_event_age_min: Vec::new(),
            metric_data_async_event_age_avg: Vec::new(),
            metric_data_async_event_age_max: Vec::new(),
            metric_data_async_events_received: Vec::new(),
            metric_data_async_events_dropped: Vec::new(),
            metric_data_destination_delivery_failures: Vec::new(),
            metric_data_dead_letter_errors: Vec::new(),
            metric_data_iterator_age: Vec::new(),
            metrics_loading: false,
        }
    }
}

use crate::ui::monitoring::MonitoringState;

impl MonitoringState for State {
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
        self.metric_data_invocations.clear();
        self.metric_data_duration_min.clear();
        self.metric_data_duration_avg.clear();
        self.metric_data_duration_max.clear();
        self.metric_data_errors.clear();
        self.metric_data_success_rate.clear();
        self.metric_data_throttles.clear();
        self.metric_data_concurrent_executions.clear();
        self.metric_data_recursive_invocations_dropped.clear();
        self.metric_data_async_event_age_min.clear();
        self.metric_data_async_event_age_avg.clear();
        self.metric_data_async_event_age_max.clear();
        self.metric_data_async_events_received.clear();
        self.metric_data_async_events_dropped.clear();
        self.metric_data_destination_delivery_failures.clear();
        self.metric_data_dead_letter_errors.clear();
        self.metric_data_iterator_age.clear();
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetailTab {
    Code,
    Monitor,
    Configuration,
    Aliases,
    Versions,
}

impl CyclicEnum for DetailTab {
    const ALL: &'static [Self] = &[
        Self::Code,
        Self::Monitor,
        Self::Configuration,
        Self::Aliases,
        Self::Versions,
    ];
}

impl DetailTab {
    pub const VERSION_TABS: &'static [Self] = &[Self::Code, Self::Monitor, Self::Configuration];

    pub fn name(&self) -> &'static str {
        match self {
            DetailTab::Code => "Code",
            DetailTab::Monitor => "Monitor",
            DetailTab::Configuration => "Configuration",
            DetailTab::Aliases => "Aliases",
            DetailTab::Versions => "Versions",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VersionDetailTab {
    Code,
    Monitor,
    Configuration,
}

impl CyclicEnum for VersionDetailTab {
    const ALL: &'static [Self] = &[Self::Code, Self::Monitor, Self::Configuration];
}

impl VersionDetailTab {
    pub fn name(&self) -> &'static str {
        match self {
            VersionDetailTab::Code => "Code",
            VersionDetailTab::Monitor => "Monitor",
            VersionDetailTab::Configuration => "Configuration",
        }
    }

    pub fn to_detail_tab(&self) -> DetailTab {
        match self {
            VersionDetailTab::Code => DetailTab::Code,
            VersionDetailTab::Monitor => DetailTab::Monitor,
            VersionDetailTab::Configuration => DetailTab::Configuration,
        }
    }

    pub fn from_detail_tab(tab: DetailTab) -> Self {
        match tab {
            DetailTab::Code => VersionDetailTab::Code,
            DetailTab::Monitor => VersionDetailTab::Monitor,
            _ => VersionDetailTab::Configuration,
        }
    }
}

pub struct ApplicationState {
    pub table: TableState<LambdaApplication>,
    pub input_focus: InputFocus,
    pub current_application: Option<String>,
    pub detail_tab: ApplicationDetailTab,
    pub deployments: TableState<Deployment>,
    pub deployment_input_focus: InputFocus,
    pub resources: TableState<Resource>,
    pub resource_input_focus: InputFocus,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ApplicationDetailTab {
    Overview,
    Deployments,
}

impl CyclicEnum for ApplicationDetailTab {
    const ALL: &'static [Self] = &[Self::Overview, Self::Deployments];
}

impl ApplicationDetailTab {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Deployments => "Deployments",
        }
    }
}

impl Default for ApplicationState {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationState {
    pub fn new() -> Self {
        Self {
            table: TableState::new(),
            input_focus: InputFocus::Filter,
            current_application: None,
            detail_tab: ApplicationDetailTab::Overview,
            deployments: TableState::new(),
            deployment_input_focus: InputFocus::Filter,
            resources: TableState::new(),
            resource_input_focus: InputFocus::Filter,
        }
    }
}

pub fn render_functions(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    if app.lambda_state.current_alias.is_some() {
        render_alias_detail(frame, app, area);
        return;
    }

    if app.lambda_state.current_version.is_some() {
        render_version_detail(frame, app, area);
        return;
    }

    if app.lambda_state.current_function.is_some() {
        render_detail(frame, app, area);
        return;
    }

    let chunks = vertical(
        [
            Constraint::Length(3), // Filter
            Constraint::Min(0),    // Table
        ],
        area,
    );

    // Filter
    let page_size = app.lambda_state.table.page_size.value();
    let filtered_count: usize = app
        .lambda_state
        .table
        .items
        .iter()
        .filter(|f| {
            app.lambda_state.table.filter.is_empty()
                || f.name
                    .to_lowercase()
                    .contains(&app.lambda_state.table.filter.to_lowercase())
                || f.description
                    .to_lowercase()
                    .contains(&app.lambda_state.table.filter.to_lowercase())
                || f.runtime
                    .to_lowercase()
                    .contains(&app.lambda_state.table.filter.to_lowercase())
        })
        .count();

    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = app.lambda_state.table.selected / page_size;
    let pagination = render_pagination_text(current_page, total_pages);

    crate::ui::filter::render_simple_filter(
        frame,
        chunks[0],
        crate::ui::filter::SimpleFilterConfig {
            filter_text: &app.lambda_state.table.filter,
            placeholder: "Filter by attributes or search by keyword",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.lambda_state.input_focus == InputFocus::Filter,
            is_pagination_focused: app.lambda_state.input_focus == InputFocus::Pagination,
        },
    );

    // Table
    let filtered: Vec<_> = app
        .lambda_state
        .table
        .items
        .iter()
        .filter(|f| {
            app.lambda_state.table.filter.is_empty()
                || f.name
                    .to_lowercase()
                    .contains(&app.lambda_state.table.filter.to_lowercase())
                || f.description
                    .to_lowercase()
                    .contains(&app.lambda_state.table.filter.to_lowercase())
                || f.runtime
                    .to_lowercase()
                    .contains(&app.lambda_state.table.filter.to_lowercase())
        })
        .collect();

    let start_idx = current_page * page_size;
    let end_idx = (start_idx + page_size).min(filtered.len());
    let paginated: Vec<_> = filtered[start_idx..end_idx].to_vec();

    let title = format!(" Lambda functions ({}) ", filtered.len());

    let mut columns: Vec<Box<dyn TableColumn<LambdaFunction>>> = vec![];
    for col_id in &app.lambda_state.function_visible_column_ids {
        if let Some(column) = LambdaColumn::from_id(col_id) {
            columns.push(Box::new(column));
        }
    }

    let expanded_index = if let Some(expanded) = app.lambda_state.table.expanded_item {
        if expanded >= start_idx && expanded < end_idx {
            Some(expanded - start_idx)
        } else {
            None
        }
    } else {
        None
    };

    let config = TableConfig {
        items: paginated,
        selected_index: app.lambda_state.table.selected % page_size,
        expanded_index,
        columns: &columns,
        sort_column: "Last modified",
        sort_direction: SortDirection::Desc,
        title,
        area: chunks[1],
        get_expanded_content: Some(Box::new(|func: &LambdaFunction| {
            expanded_from_columns(&columns, func)
        })),
        is_active: app.mode != Mode::FilterInput,
    };

    render_table(frame, config);
}

pub fn render_detail(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    // Build overview lines first to calculate height
    let overview_lines = if let Some(func_name) = &app.lambda_state.current_function {
        if let Some(func) = app
            .lambda_state
            .table
            .items
            .iter()
            .find(|f| f.name == *func_name)
        {
            vec![
                labeled_field(
                    "Description",
                    if func.description.is_empty() {
                        "-"
                    } else {
                        &func.description
                    },
                ),
                labeled_field("Last modified", &func.last_modified),
                labeled_field("Function ARN", &func.arn),
                labeled_field("Application", func.application.as_deref().unwrap_or("-")),
            ]
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let overview_height = if overview_lines.is_empty() {
        0
    } else {
        overview_lines.len() as u16 + 2
    };

    let chunks = vertical(
        [
            Constraint::Length(overview_height),
            Constraint::Length(1), // Tabs
            Constraint::Min(0),    // Content
        ],
        area,
    );

    // Function overview
    if !overview_lines.is_empty() {
        let overview_block = Block::default()
            .title(" Function overview ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default());

        let overview_inner = overview_block.inner(chunks[0]);
        frame.render_widget(overview_block, chunks[0]);
        frame.render_widget(Paragraph::new(overview_lines), overview_inner);
    }

    // Tabs
    let tabs: Vec<(&str, DetailTab)> = DetailTab::ALL
        .iter()
        .map(|tab| (tab.name(), *tab))
        .collect();

    render_tabs(frame, chunks[1], &tabs, &app.lambda_state.detail_tab);

    // Content area
    if app.lambda_state.detail_tab == DetailTab::Code {
        // Show Code properties
        if let Some(func_name) = &app.lambda_state.current_function {
            if let Some(func) = app
                .lambda_state
                .table
                .items
                .iter()
                .find(|f| f.name == *func_name)
            {
                // Build lines first to calculate heights
                let code_lines = vec![
                    labeled_field("Package size", format_bytes(func.code_size)),
                    labeled_field("SHA256 hash", &func.code_sha256),
                    labeled_field("Last modified", &func.last_modified),
                    section_header(
                        "Encryption with AWS KMS customer managed KMS key",
                        chunks[2].width.saturating_sub(2),
                    ),
                    Line::from(Span::styled(
                        "To edit customer managed key encryption, you must upload a new .zip deployment package.",
                        Style::default().fg(Color::DarkGray),
                    )),
                    labeled_field("AWS KMS key ARN", ""),
                    labeled_field("Key alias", ""),
                    labeled_field("Status", ""),
                ];

                let runtime_lines = vec![
                    labeled_field("Runtime", format_runtime(&func.runtime)),
                    labeled_field("Handler", ""),
                    labeled_field("Architecture", format_architecture(&func.architecture)),
                    section_header(
                        "Runtime management configuration",
                        chunks[2].width.saturating_sub(2),
                    ),
                    labeled_field("Runtime version ARN", ""),
                    labeled_field("Update runtime version", "Auto"),
                ];

                let chunks_content = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(block_height(&code_lines)),
                        Constraint::Length(block_height(&runtime_lines)),
                        Constraint::Min(0), // Layers
                    ])
                    .split(chunks[2]);

                // Code properties section
                let code_block = Block::default()
                    .title(" Code properties ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded);

                let code_inner = code_block.inner(chunks_content[0]);
                frame.render_widget(code_block, chunks_content[0]);

                frame.render_widget(Paragraph::new(code_lines), code_inner);

                // Runtime settings section
                let runtime_block = Block::default()
                    .title(" Runtime settings ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded);

                let runtime_inner = runtime_block.inner(chunks_content[1]);
                frame.render_widget(runtime_block, chunks_content[1]);

                frame.render_widget(Paragraph::new(runtime_lines), runtime_inner);

                // Layers section
                let layer_refs: Vec<&Layer> = func.layers.iter().collect();
                let title = format!(" Layers ({}) ", layer_refs.len());

                let columns: Vec<Box<dyn TableColumn<Layer>>> = vec![
                    Box::new(LayerColumn::MergeOrder),
                    Box::new(LayerColumn::Name),
                    Box::new(LayerColumn::LayerVersion),
                    Box::new(LayerColumn::CompatibleRuntimes),
                    Box::new(LayerColumn::CompatibleArchitectures),
                    Box::new(LayerColumn::VersionArn),
                ];

                let config = TableConfig {
                    items: layer_refs,
                    selected_index: app.lambda_state.layer_selected,
                    expanded_index: app.lambda_state.layer_expanded,
                    columns: &columns,
                    sort_column: "",
                    sort_direction: SortDirection::Asc,
                    title,
                    area: chunks_content[2],
                    get_expanded_content: Some(Box::new(|layer: &Layer| {
                        crate::ui::format_expansion_text(&[
                            ("Merge order", layer.merge_order.clone()),
                            ("Name", layer.name.clone()),
                            ("Layer version", layer.layer_version.clone()),
                            ("Compatible runtimes", layer.compatible_runtimes.clone()),
                            (
                                "Compatible architectures",
                                layer.compatible_architectures.clone(),
                            ),
                            ("Version ARN", layer.version_arn.clone()),
                        ])
                    })),
                    is_active: app.lambda_state.detail_tab == DetailTab::Code,
                };

                render_table(frame, config);
            }
        }
    } else if app.lambda_state.detail_tab == DetailTab::Monitor {
        if app.lambda_state.metrics_loading {
            let loading_block = Block::default()
                .title(" Monitoring ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded);
            let loading_text = Paragraph::new("Loading metrics...")
                .block(loading_block)
                .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(loading_text, chunks[2]);
            return;
        }

        render_lambda_monitoring_charts(frame, app, chunks[2]);
    } else if app.lambda_state.detail_tab == DetailTab::Configuration {
        // Configuration tab
        if let Some(func_name) = &app.lambda_state.current_function {
            if let Some(func) = app
                .lambda_state
                .table
                .items
                .iter()
                .find(|f| f.name == *func_name)
            {
                let config_lines = vec![
                    labeled_field("Description", &func.description),
                    labeled_field("Revision", &func.last_modified),
                    labeled_field("Memory", format_memory_mb(func.memory_mb)),
                    labeled_field("Ephemeral storage", format_memory_mb(512)),
                    labeled_field("Timeout", format_duration_seconds(func.timeout_seconds)),
                    labeled_field("SnapStart", "None"),
                ];

                let config_chunks = vertical(
                    [
                        Constraint::Length(block_height(&config_lines)),
                        Constraint::Min(0),
                    ],
                    chunks[2],
                );

                let config_block = Block::default()
                    .title(" General configuration ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default());

                let config_inner = config_block.inner(config_chunks[0]);
                frame.render_widget(config_block, config_chunks[0]);

                frame.render_widget(Paragraph::new(config_lines), config_inner);
            }
        }
    } else if app.lambda_state.detail_tab == DetailTab::Versions {
        // Versions tab
        let version_chunks = vertical(
            [
                Constraint::Length(3), // Filter
                Constraint::Min(0),    // Table
            ],
            chunks[2],
        );

        // Filter
        let page_size = app.lambda_state.version_table.page_size.value();
        let filtered_count: usize = app
            .lambda_state
            .version_table
            .items
            .iter()
            .filter(|v| {
                app.lambda_state.version_table.filter.is_empty()
                    || v.version
                        .to_lowercase()
                        .contains(&app.lambda_state.version_table.filter.to_lowercase())
                    || v.aliases
                        .to_lowercase()
                        .contains(&app.lambda_state.version_table.filter.to_lowercase())
                    || v.description
                        .to_lowercase()
                        .contains(&app.lambda_state.version_table.filter.to_lowercase())
            })
            .count();

        let total_pages = filtered_count.div_ceil(page_size);
        let current_page = app.lambda_state.version_table.selected / page_size;
        let pagination = render_pagination_text(current_page, total_pages);

        crate::ui::filter::render_simple_filter(
            frame,
            version_chunks[0],
            crate::ui::filter::SimpleFilterConfig {
                filter_text: &app.lambda_state.version_table.filter,
                placeholder: "Filter by attributes or search by keyword",
                pagination: &pagination,
                mode: app.mode,
                is_input_focused: app.lambda_state.version_input_focus == InputFocus::Filter,
                is_pagination_focused: app.lambda_state.version_input_focus
                    == InputFocus::Pagination,
            },
        );

        // Table
        let filtered: Vec<_> = app
            .lambda_state
            .version_table
            .items
            .iter()
            .filter(|v| {
                app.lambda_state.version_table.filter.is_empty()
                    || v.version
                        .to_lowercase()
                        .contains(&app.lambda_state.version_table.filter.to_lowercase())
                    || v.aliases
                        .to_lowercase()
                        .contains(&app.lambda_state.version_table.filter.to_lowercase())
                    || v.description
                        .to_lowercase()
                        .contains(&app.lambda_state.version_table.filter.to_lowercase())
            })
            .collect();

        let start_idx = current_page * page_size;
        let end_idx = (start_idx + page_size).min(filtered.len());
        let paginated: Vec<_> = filtered[start_idx..end_idx].to_vec();

        let title = format!(" Versions ({}) ", filtered.len());

        let mut columns: Vec<Box<dyn TableColumn<Version>>> = vec![];
        for col_name in &app.lambda_state.version_visible_column_ids {
            let column = match col_name.as_str() {
                "Version" => Some(VersionColumn::Version),
                "Aliases" => Some(VersionColumn::Aliases),
                "Description" => Some(VersionColumn::Description),
                "Last modified" => Some(VersionColumn::LastModified),
                "Architecture" => Some(VersionColumn::Architecture),
                _ => None,
            };
            if let Some(c) = column {
                columns.push(c.to_column());
            }
        }

        let expanded_index = if let Some(expanded) = app.lambda_state.version_table.expanded_item {
            if expanded >= start_idx && expanded < end_idx {
                Some(expanded - start_idx)
            } else {
                None
            }
        } else {
            None
        };

        let config = TableConfig {
            items: paginated,
            selected_index: app.lambda_state.version_table.selected % page_size,
            expanded_index,
            columns: &columns,
            sort_column: "Version",
            sort_direction: SortDirection::Desc,
            title,
            area: version_chunks[1],
            get_expanded_content: Some(Box::new(|ver: &crate::lambda::Version| {
                expanded_from_columns(&columns, ver)
            })),
            is_active: app.mode != Mode::FilterInput,
        };

        render_table(frame, config);
    } else if app.lambda_state.detail_tab == DetailTab::Aliases {
        // Aliases tab
        let alias_chunks = vertical(
            [
                Constraint::Length(3), // Filter
                Constraint::Min(0),    // Table
            ],
            chunks[2],
        );

        // Filter
        let page_size = app.lambda_state.alias_table.page_size.value();
        let filtered_count: usize = app
            .lambda_state
            .alias_table
            .items
            .iter()
            .filter(|a| {
                app.lambda_state.alias_table.filter.is_empty()
                    || a.name
                        .to_lowercase()
                        .contains(&app.lambda_state.alias_table.filter.to_lowercase())
                    || a.versions
                        .to_lowercase()
                        .contains(&app.lambda_state.alias_table.filter.to_lowercase())
                    || a.description
                        .to_lowercase()
                        .contains(&app.lambda_state.alias_table.filter.to_lowercase())
            })
            .count();

        let total_pages = filtered_count.div_ceil(page_size);
        let current_page = app.lambda_state.alias_table.selected / page_size;
        let pagination = render_pagination_text(current_page, total_pages);

        crate::ui::filter::render_simple_filter(
            frame,
            alias_chunks[0],
            crate::ui::filter::SimpleFilterConfig {
                filter_text: &app.lambda_state.alias_table.filter,
                placeholder: "Filter by attributes or search by keyword",
                pagination: &pagination,
                mode: app.mode,
                is_input_focused: app.lambda_state.alias_input_focus == InputFocus::Filter,
                is_pagination_focused: app.lambda_state.alias_input_focus == InputFocus::Pagination,
            },
        );

        // Table
        let filtered: Vec<_> = app
            .lambda_state
            .alias_table
            .items
            .iter()
            .filter(|a| {
                app.lambda_state.alias_table.filter.is_empty()
                    || a.name
                        .to_lowercase()
                        .contains(&app.lambda_state.alias_table.filter.to_lowercase())
                    || a.versions
                        .to_lowercase()
                        .contains(&app.lambda_state.alias_table.filter.to_lowercase())
                    || a.description
                        .to_lowercase()
                        .contains(&app.lambda_state.alias_table.filter.to_lowercase())
            })
            .collect();

        let start_idx = current_page * page_size;
        let end_idx = (start_idx + page_size).min(filtered.len());
        let paginated: Vec<_> = filtered[start_idx..end_idx].to_vec();

        let title = format!(" Aliases ({}) ", filtered.len());

        let mut columns: Vec<Box<dyn TableColumn<Alias>>> = vec![];
        for col_name in &app.lambda_state.alias_visible_column_ids {
            let column = match col_name.as_str() {
                "Name" => Some(AliasColumn::Name),
                "Versions" => Some(AliasColumn::Versions),
                "Description" => Some(AliasColumn::Description),
                _ => None,
            };
            if let Some(c) = column {
                columns.push(c.to_column());
            }
        }

        let expanded_index = if let Some(expanded) = app.lambda_state.alias_table.expanded_item {
            if expanded >= start_idx && expanded < end_idx {
                Some(expanded - start_idx)
            } else {
                None
            }
        } else {
            None
        };

        let config = TableConfig {
            items: paginated,
            selected_index: app.lambda_state.alias_table.selected % page_size,
            expanded_index,
            columns: &columns,
            sort_column: "Name",
            sort_direction: SortDirection::Asc,
            title,
            area: alias_chunks[1],
            get_expanded_content: Some(Box::new(|alias: &crate::lambda::Alias| {
                expanded_from_columns(&columns, alias)
            })),
            is_active: app.mode != Mode::FilterInput,
        };

        render_table(frame, config);
    } else {
        // Placeholder for other tabs
        let content = Paragraph::new(format!(
            "{} tab content (coming soon)",
            app.lambda_state.detail_tab.name()
        ))
        .block(crate::ui::rounded_block());
        frame.render_widget(content, chunks[2]);
    }
}

pub fn render_alias_detail(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    // Build overview lines first to calculate height
    let mut overview_lines = vec![];
    if let Some(func_name) = &app.lambda_state.current_function {
        if let Some(func) = app
            .lambda_state
            .table
            .items
            .iter()
            .find(|f| f.name == *func_name)
        {
            if let Some(alias_name) = &app.lambda_state.current_alias {
                if let Some(alias) = app
                    .lambda_state
                    .alias_table
                    .items
                    .iter()
                    .find(|a| a.name == *alias_name)
                {
                    overview_lines.push(labeled_field("Description", &alias.description));

                    // Parse versions
                    let versions_parts: Vec<&str> =
                        alias.versions.split(',').map(|s| s.trim()).collect();
                    if let Some(first_version) = versions_parts.first() {
                        overview_lines.push(labeled_field("Version", *first_version));
                    }
                    if versions_parts.len() > 1 {
                        if let Some(second_version) = versions_parts.get(1) {
                            overview_lines
                                .push(labeled_field("Additional version", *second_version));
                        }
                    }

                    overview_lines.push(labeled_field("Function ARN", &func.arn));

                    if let Some(app) = &func.application {
                        overview_lines.push(labeled_field("Application", app));
                    }

                    overview_lines.push(labeled_field("Function URL", "-"));
                }
            }
        }
    }

    // Build config lines to calculate height
    let mut config_lines = vec![];
    if let Some(_func_name) = &app.lambda_state.current_function {
        if let Some(alias_name) = &app.lambda_state.current_alias {
            if let Some(alias) = app
                .lambda_state
                .alias_table
                .items
                .iter()
                .find(|a| a.name == *alias_name)
            {
                config_lines.push(labeled_field("Name", &alias.name));
                config_lines.push(labeled_field("Description", &alias.description));

                // Parse versions
                let versions_parts: Vec<&str> =
                    alias.versions.split(',').map(|s| s.trim()).collect();
                if let Some(first_version) = versions_parts.first() {
                    config_lines.push(labeled_field("Version", *first_version));
                }
                if versions_parts.len() > 1 {
                    if let Some(second_version) = versions_parts.get(1) {
                        config_lines.push(labeled_field("Additional version", *second_version));
                    }
                }
            }
        }
    }

    let config_height = if config_lines.is_empty() {
        0
    } else {
        config_lines.len() as u16 + 2
    };

    let overview_height = overview_lines.len() as u16 + 2; // +2 for borders

    let chunks = vertical(
        [
            Constraint::Length(overview_height),
            Constraint::Length(config_height),
            Constraint::Min(0), // Empty space
        ],
        area,
    );

    // Function overview
    if let Some(func_name) = &app.lambda_state.current_function {
        if let Some(_func) = app
            .lambda_state
            .table
            .items
            .iter()
            .find(|f| f.name == *func_name)
        {
            if let Some(alias_name) = &app.lambda_state.current_alias {
                if let Some(_alias) = app
                    .lambda_state
                    .alias_table
                    .items
                    .iter()
                    .find(|a| a.name == *alias_name)
                {
                    let overview_block = Block::default()
                        .title(" Function overview ")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default());

                    let overview_inner = overview_block.inner(chunks[0]);
                    frame.render_widget(overview_block, chunks[0]);

                    frame.render_widget(Paragraph::new(overview_lines), overview_inner);
                }
            }
        }
    }

    // General configuration
    if !config_lines.is_empty() {
        let config_block = Block::default()
            .title(" General configuration ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        let config_inner = config_block.inner(chunks[1]);
        frame.render_widget(config_block, chunks[1]);
        frame.render_widget(Paragraph::new(config_lines), config_inner);
    }
}

pub fn render_version_detail(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    // Build overview lines first to calculate height
    let mut overview_lines = vec![];
    if let Some(func_name) = &app.lambda_state.current_function {
        if let Some(func) = app
            .lambda_state
            .table
            .items
            .iter()
            .find(|f| f.name == *func_name)
        {
            if let Some(version_num) = &app.lambda_state.current_version {
                let version_arn = format!("{}:{}", func.arn, version_num);

                overview_lines.push(labeled_field("Name", &func.name));

                if let Some(app) = &func.application {
                    overview_lines.push(labeled_field("Application", app));
                }

                overview_lines.extend(vec![
                    labeled_field("ARN", version_arn),
                    labeled_field("Version", version_num),
                ]);
            }
        }
    }

    let overview_height = if overview_lines.is_empty() {
        0
    } else {
        overview_lines.len() as u16 + 2
    };

    let chunks = vertical(
        [
            Constraint::Length(overview_height),
            Constraint::Length(1), // Tabs (Code, Configuration only)
            Constraint::Min(0),    // Content
        ],
        area,
    );

    // Function overview
    if !overview_lines.is_empty() {
        let overview_block = Block::default()
            .title(" Function overview ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default());

        let overview_inner = overview_block.inner(chunks[0]);
        frame.render_widget(overview_block, chunks[0]);
        frame.render_widget(Paragraph::new(overview_lines), overview_inner);
    }

    // Tabs - only Code, Monitor, and Configuration
    let tabs: Vec<(&str, VersionDetailTab)> = VersionDetailTab::ALL
        .iter()
        .map(|tab| (tab.name(), *tab))
        .collect();

    render_tabs(
        frame,
        chunks[1],
        &tabs,
        &app.lambda_state.version_detail_tab,
    );

    // Content area - reuse same rendering as function detail
    if app.lambda_state.detail_tab == DetailTab::Code {
        if let Some(func_name) = &app.lambda_state.current_function {
            if let Some(func) = app
                .lambda_state
                .table
                .items
                .iter()
                .find(|f| f.name == *func_name)
            {
                // Build lines first to calculate heights
                let code_lines = vec![
                    labeled_field("Package size", format_bytes(func.code_size)),
                    labeled_field("SHA256 hash", &func.code_sha256),
                    labeled_field("Last modified", &func.last_modified),
                ];

                let runtime_lines = vec![
                    labeled_field("Runtime", format_runtime(&func.runtime)),
                    labeled_field("Handler", ""),
                    labeled_field("Architecture", format_architecture(&func.architecture)),
                ];

                let chunks_content = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(block_height(&code_lines)),
                        Constraint::Length(block_height(&runtime_lines)),
                        Constraint::Min(0),
                    ])
                    .split(chunks[2]);

                // Code properties section
                let code_block = Block::default()
                    .title(" Code properties ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded);

                let code_inner = code_block.inner(chunks_content[0]);
                frame.render_widget(code_block, chunks_content[0]);

                frame.render_widget(Paragraph::new(code_lines), code_inner);

                // Runtime settings section
                let runtime_block = Block::default()
                    .title(" Runtime settings ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded);

                let runtime_inner = runtime_block.inner(chunks_content[1]);
                frame.render_widget(runtime_block, chunks_content[1]);

                frame.render_widget(Paragraph::new(runtime_lines), runtime_inner);

                // Layers section (empty table)
                let layers: Vec<Layer> = vec![];
                let layer_refs: Vec<&Layer> = layers.iter().collect();
                let title = format!(" Layers ({}) ", layer_refs.len());

                let columns: Vec<Box<dyn TableColumn<Layer>>> = vec![
                    Box::new(LayerColumn::MergeOrder),
                    Box::new(LayerColumn::Name),
                    Box::new(LayerColumn::LayerVersion),
                    Box::new(LayerColumn::CompatibleRuntimes),
                    Box::new(LayerColumn::CompatibleArchitectures),
                    Box::new(LayerColumn::VersionArn),
                ];

                let config = TableConfig {
                    items: layer_refs,
                    selected_index: 0,
                    expanded_index: None,
                    columns: &columns,
                    sort_column: "",
                    sort_direction: SortDirection::Asc,
                    title,
                    area: chunks_content[2],
                    get_expanded_content: Some(Box::new(|layer: &Layer| {
                        crate::ui::format_expansion_text(&[
                            ("Merge order", layer.merge_order.clone()),
                            ("Name", layer.name.clone()),
                            ("Layer version", layer.layer_version.clone()),
                            ("Compatible runtimes", layer.compatible_runtimes.clone()),
                            (
                                "Compatible architectures",
                                layer.compatible_architectures.clone(),
                            ),
                            ("Version ARN", layer.version_arn.clone()),
                        ])
                    })),
                    is_active: app.lambda_state.detail_tab == DetailTab::Code,
                };

                render_table(frame, config);
            }
        }
    } else if app.lambda_state.detail_tab == DetailTab::Monitor {
        // Monitor tab - render same charts as function detail
        if app.lambda_state.metrics_loading {
            let loading_block = Block::default()
                .title(" Monitor ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded);
            let loading_text = Paragraph::new("Loading metrics...")
                .block(loading_block)
                .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(loading_text, chunks[2]);
            return;
        }

        // Reuse the same monitoring rendering logic
        render_lambda_monitoring_charts(frame, app, chunks[2]);
    } else if app.lambda_state.detail_tab == DetailTab::Configuration {
        if let Some(func_name) = &app.lambda_state.current_function {
            if let Some(func) = app
                .lambda_state
                .table
                .items
                .iter()
                .find(|f| f.name == *func_name)
            {
                if let Some(version_num) = &app.lambda_state.current_version {
                    // Version Configuration: show config + aliases for this version
                    let config_lines = vec![
                        labeled_field("Description", &func.description),
                        labeled_field("Memory", format_memory_mb(func.memory_mb)),
                        labeled_field("Timeout", format_duration_seconds(func.timeout_seconds)),
                    ];

                    let chunks_content = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(block_height(&config_lines)),
                            Constraint::Length(3), // Filter
                            Constraint::Min(0),    // Aliases table
                        ])
                        .split(chunks[2]);

                    let config_block = Block::default()
                        .title(" General configuration ")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default());

                    let config_inner = config_block.inner(chunks_content[0]);
                    frame.render_widget(config_block, chunks_content[0]);

                    frame.render_widget(Paragraph::new(config_lines), config_inner);

                    // Filter for aliases
                    let page_size = app.lambda_state.alias_table.page_size.value();
                    let filtered_count: usize = app
                        .lambda_state
                        .alias_table
                        .items
                        .iter()
                        .filter(|a| {
                            a.versions.contains(version_num)
                                && (app.lambda_state.alias_table.filter.is_empty()
                                    || a.name.to_lowercase().contains(
                                        &app.lambda_state.alias_table.filter.to_lowercase(),
                                    )
                                    || a.versions.to_lowercase().contains(
                                        &app.lambda_state.alias_table.filter.to_lowercase(),
                                    )
                                    || a.description.to_lowercase().contains(
                                        &app.lambda_state.alias_table.filter.to_lowercase(),
                                    ))
                        })
                        .count();

                    let total_pages = filtered_count.div_ceil(page_size);
                    let current_page = app.lambda_state.alias_table.selected / page_size;
                    let pagination = render_pagination_text(current_page, total_pages);

                    crate::ui::filter::render_simple_filter(
                        frame,
                        chunks_content[1],
                        crate::ui::filter::SimpleFilterConfig {
                            filter_text: &app.lambda_state.alias_table.filter,
                            placeholder: "Filter by attributes or search by keyword",
                            pagination: &pagination,
                            mode: app.mode,
                            is_input_focused: app.lambda_state.alias_input_focus
                                == InputFocus::Filter,
                            is_pagination_focused: app.lambda_state.alias_input_focus
                                == InputFocus::Pagination,
                        },
                    );

                    // Aliases table - filter to show only aliases pointing to this version
                    let filtered: Vec<_> = app
                        .lambda_state
                        .alias_table
                        .items
                        .iter()
                        .filter(|a| {
                            a.versions.contains(version_num)
                                && (app.lambda_state.alias_table.filter.is_empty()
                                    || a.name.to_lowercase().contains(
                                        &app.lambda_state.alias_table.filter.to_lowercase(),
                                    )
                                    || a.versions.to_lowercase().contains(
                                        &app.lambda_state.alias_table.filter.to_lowercase(),
                                    )
                                    || a.description.to_lowercase().contains(
                                        &app.lambda_state.alias_table.filter.to_lowercase(),
                                    ))
                        })
                        .collect();

                    let start_idx = current_page * page_size;
                    let end_idx = (start_idx + page_size).min(filtered.len());
                    let paginated: Vec<_> = filtered[start_idx..end_idx].to_vec();

                    let title = format!(" Aliases ({}) ", filtered.len());

                    let mut columns: Vec<Box<dyn TableColumn<Alias>>> = vec![];
                    for col_name in &app.lambda_state.alias_visible_column_ids {
                        let column = match col_name.as_str() {
                            "Name" => Some(AliasColumn::Name),
                            "Versions" => Some(AliasColumn::Versions),
                            "Description" => Some(AliasColumn::Description),
                            _ => None,
                        };
                        if let Some(c) = column {
                            columns.push(c.to_column());
                        }
                    }

                    let expanded_index =
                        if let Some(expanded) = app.lambda_state.alias_table.expanded_item {
                            if expanded >= start_idx && expanded < end_idx {
                                Some(expanded - start_idx)
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                    let config = TableConfig {
                        items: paginated,
                        selected_index: app.lambda_state.alias_table.selected % page_size,
                        expanded_index,
                        columns: &columns,
                        sort_column: "Name",
                        sort_direction: SortDirection::Asc,
                        title,
                        area: chunks_content[2],
                        get_expanded_content: Some(Box::new(|alias: &crate::lambda::Alias| {
                            expanded_from_columns(&columns, alias)
                        })),
                        is_active: app.mode != Mode::FilterInput,
                    };

                    render_table(frame, config);
                }
            }
        }
    }
}

pub fn render_applications(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    if app.lambda_application_state.current_application.is_some() {
        render_application_detail(frame, app, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Filter with pagination
    let page_size = app.lambda_application_state.table.page_size.value();
    let filtered_count = filtered_lambda_applications(app).len();
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = app.lambda_application_state.table.selected / page_size;
    let pagination = render_pagination_text(current_page, total_pages);

    crate::ui::filter::render_simple_filter(
        frame,
        chunks[0],
        crate::ui::filter::SimpleFilterConfig {
            filter_text: &app.lambda_application_state.table.filter,
            placeholder: "Filter by attributes or search by keyword",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.lambda_application_state.input_focus == InputFocus::Filter,
            is_pagination_focused: app.lambda_application_state.input_focus
                == InputFocus::Pagination,
        },
    );

    // Table
    let filtered: Vec<_> = filtered_lambda_applications(app);
    let start_idx = current_page * page_size;
    let end_idx = (start_idx + page_size).min(filtered.len());
    let paginated: Vec<_> = filtered[start_idx..end_idx].to_vec();

    let title = format!(" Applications ({}) ", filtered.len());

    let mut columns: Vec<Box<dyn TableColumn<LambdaApplication>>> = vec![];
    for col_id in &app.lambda_application_visible_column_ids {
        if let Some(column) = crate::lambda::ApplicationColumn::from_id(col_id) {
            columns.push(Box::new(column));
        }
    }

    let expanded_index = if let Some(expanded) = app.lambda_application_state.table.expanded_item {
        if expanded >= start_idx && expanded < end_idx {
            Some(expanded - start_idx)
        } else {
            None
        }
    } else {
        None
    };

    let config = TableConfig {
        items: paginated,
        selected_index: app.lambda_application_state.table.selected % page_size,
        expanded_index,
        columns: &columns,
        sort_column: "Last modified",
        sort_direction: SortDirection::Desc,
        title,
        area: chunks[1],
        get_expanded_content: Some(Box::new(|app: &LambdaApplication| {
            expanded_from_columns(&columns, app)
        })),
        is_active: app.mode != Mode::FilterInput,
    };

    render_table(frame, config);
}

// Lambda-specific helper functions
pub fn filtered_lambda_functions(app: &App) -> Vec<&LambdaFunction> {
    if app.lambda_state.table.filter.is_empty() {
        app.lambda_state.table.items.iter().collect()
    } else {
        app.lambda_state
            .table
            .items
            .iter()
            .filter(|f| {
                f.name
                    .to_lowercase()
                    .contains(&app.lambda_state.table.filter.to_lowercase())
                    || f.description
                        .to_lowercase()
                        .contains(&app.lambda_state.table.filter.to_lowercase())
                    || f.runtime
                        .to_lowercase()
                        .contains(&app.lambda_state.table.filter.to_lowercase())
            })
            .collect()
    }
}

pub fn filtered_lambda_applications(app: &App) -> Vec<&LambdaApplication> {
    if app.lambda_application_state.table.filter.is_empty() {
        app.lambda_application_state.table.items.iter().collect()
    } else {
        app.lambda_application_state
            .table
            .items
            .iter()
            .filter(|a| {
                a.name
                    .to_lowercase()
                    .contains(&app.lambda_application_state.table.filter.to_lowercase())
                    || a.description
                        .to_lowercase()
                        .contains(&app.lambda_application_state.table.filter.to_lowercase())
                    || a.status
                        .to_lowercase()
                        .contains(&app.lambda_application_state.table.filter.to_lowercase())
            })
            .collect()
    }
}

pub async fn load_lambda_functions(app: &mut App) -> anyhow::Result<()> {
    let functions = app.lambda_client.list_functions().await?;

    let mut functions: Vec<LambdaFunction> = functions
        .into_iter()
        .map(|f| LambdaFunction {
            name: f.name,
            arn: f.arn,
            application: f.application,
            description: f.description,
            package_type: f.package_type,
            runtime: f.runtime,
            architecture: f.architecture,
            code_size: f.code_size,
            code_sha256: f.code_sha256,
            memory_mb: f.memory_mb,
            timeout_seconds: f.timeout_seconds,
            last_modified: f.last_modified,
            layers: f
                .layers
                .into_iter()
                .enumerate()
                .map(|(i, l)| {
                    let (name, version) = crate::lambda::parse_layer_arn(&l.arn);
                    Layer {
                        merge_order: (i + 1).to_string(),
                        name,
                        layer_version: version,
                        compatible_runtimes: "-".to_string(),
                        compatible_architectures: "-".to_string(),
                        version_arn: l.arn,
                    }
                })
                .collect(),
        })
        .collect();

    // Sort by last_modified DESC
    functions.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

    app.lambda_state.table.items = functions;

    Ok(())
}

pub async fn load_lambda_applications(app: &mut App) -> anyhow::Result<()> {
    let applications = app.lambda_client.list_applications().await?;
    let mut applications: Vec<LambdaApplication> = applications
        .into_iter()
        .map(|a| LambdaApplication {
            name: a.name,
            arn: a.arn,
            description: a.description,
            status: a.status,
            last_modified: a.last_modified,
        })
        .collect();
    applications.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
    app.lambda_application_state.table.items = applications;
    Ok(())
}

pub async fn load_lambda_versions(app: &mut App, function_name: &str) -> anyhow::Result<()> {
    let versions = app.lambda_client.list_versions(function_name).await?;
    let mut versions: Vec<Version> = versions
        .into_iter()
        .map(|v| Version {
            version: v.version,
            aliases: v.aliases,
            description: v.description,
            last_modified: v.last_modified,
            architecture: v.architecture,
        })
        .collect();

    // Sort by version DESC (numeric sort)
    versions.sort_by(|a, b| {
        let a_num = a.version.parse::<i32>().unwrap_or(0);
        let b_num = b.version.parse::<i32>().unwrap_or(0);
        b_num.cmp(&a_num)
    });

    app.lambda_state.version_table.items = versions;
    Ok(())
}

pub async fn load_lambda_aliases(app: &mut App, function_name: &str) -> anyhow::Result<()> {
    let aliases = app.lambda_client.list_aliases(function_name).await?;
    let mut aliases: Vec<Alias> = aliases
        .into_iter()
        .map(|a| Alias {
            name: a.name,
            versions: a.versions,
            description: a.description,
        })
        .collect();

    // Sort by name ASC
    aliases.sort_by(|a, b| a.name.cmp(&b.name));

    app.lambda_state.alias_table.items = aliases;
    Ok(())
}

pub async fn load_lambda_metrics(
    app: &mut App,
    function_name: &str,
    version: Option<&str>,
) -> anyhow::Result<()> {
    use rusticity_core::lambda::Statistic;

    // Build resource string if version is provided (e.g., "function_name:1")
    let resource = version.map(|v| format!("{}:{}", function_name, v));
    let resource_ref = resource.as_deref();

    let invocations = app
        .lambda_client
        .get_invocations_metric(function_name, resource_ref)
        .await?;
    app.lambda_state.metric_data_invocations = invocations.clone();

    let duration_min = app
        .lambda_client
        .get_duration_metric(function_name, Statistic::Minimum)
        .await?;
    app.lambda_state.metric_data_duration_min = duration_min;

    let duration_avg = app
        .lambda_client
        .get_duration_metric(function_name, Statistic::Average)
        .await?;
    app.lambda_state.metric_data_duration_avg = duration_avg;

    let duration_max = app
        .lambda_client
        .get_duration_metric(function_name, Statistic::Maximum)
        .await?;
    app.lambda_state.metric_data_duration_max = duration_max;

    let errors = app.lambda_client.get_errors_metric(function_name).await?;
    app.lambda_state.metric_data_errors = errors.clone();

    let mut success_rate = Vec::new();
    for (timestamp, error_count) in &errors {
        if let Some((_, invocation_count)) = invocations.iter().find(|(ts, _)| ts == timestamp) {
            let max_val = error_count.max(*invocation_count);
            if max_val > 0.0 {
                let rate = 100.0 - 100.0 * error_count / max_val;
                success_rate.push((*timestamp, rate));
            }
        }
    }
    app.lambda_state.metric_data_success_rate = success_rate;

    let throttles = app
        .lambda_client
        .get_throttles_metric(function_name)
        .await?;
    app.lambda_state.metric_data_throttles = throttles;

    let concurrent_executions = app
        .lambda_client
        .get_concurrent_executions_metric(function_name)
        .await?;
    app.lambda_state.metric_data_concurrent_executions = concurrent_executions;

    let recursive_invocations_dropped = app
        .lambda_client
        .get_recursive_invocations_dropped_metric(function_name)
        .await?;
    app.lambda_state.metric_data_recursive_invocations_dropped = recursive_invocations_dropped;

    let async_event_age_min = app
        .lambda_client
        .get_async_event_age_metric(function_name, Statistic::Minimum)
        .await?;
    app.lambda_state.metric_data_async_event_age_min = async_event_age_min;

    let async_event_age_avg = app
        .lambda_client
        .get_async_event_age_metric(function_name, Statistic::Average)
        .await?;
    app.lambda_state.metric_data_async_event_age_avg = async_event_age_avg;

    let async_event_age_max = app
        .lambda_client
        .get_async_event_age_metric(function_name, Statistic::Maximum)
        .await?;
    app.lambda_state.metric_data_async_event_age_max = async_event_age_max;

    let async_events_received = app
        .lambda_client
        .get_async_events_received_metric(function_name)
        .await?;
    app.lambda_state.metric_data_async_events_received = async_events_received;

    let async_events_dropped = app
        .lambda_client
        .get_async_events_dropped_metric(function_name)
        .await?;
    app.lambda_state.metric_data_async_events_dropped = async_events_dropped;

    let destination_delivery_failures = app
        .lambda_client
        .get_destination_delivery_failures_metric(function_name)
        .await?;
    app.lambda_state.metric_data_destination_delivery_failures = destination_delivery_failures;

    let dead_letter_errors = app
        .lambda_client
        .get_dead_letter_errors_metric(function_name)
        .await?;
    app.lambda_state.metric_data_dead_letter_errors = dead_letter_errors;

    let iterator_age = app
        .lambda_client
        .get_iterator_age_metric(function_name)
        .await?;
    app.lambda_state.metric_data_iterator_age = iterator_age;

    Ok(())
}

pub fn render_application_detail(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    let chunks = vertical(
        [
            Constraint::Length(1), // Application name
            Constraint::Length(1), // Tabs
            Constraint::Min(0),    // Content
        ],
        area,
    );

    // Application name
    if let Some(app_name) = &app.lambda_application_state.current_application {
        frame.render_widget(Paragraph::new(app_name.as_str()), chunks[0]);
    }

    // Tabs
    let tabs: Vec<(&str, ApplicationDetailTab)> = ApplicationDetailTab::ALL
        .iter()
        .map(|tab| (tab.name(), *tab))
        .collect();
    render_tabs(
        frame,
        chunks[1],
        &tabs,
        &app.lambda_application_state.detail_tab,
    );

    // Content
    if app.lambda_application_state.detail_tab == ApplicationDetailTab::Overview {
        let chunks_content = vertical(
            [
                Constraint::Length(3), // Filter
                Constraint::Min(0),    // Table
            ],
            chunks[2],
        );

        // Filter with pagination
        let page_size = app.lambda_application_state.resources.page_size.value();
        let filtered_count = app.lambda_application_state.resources.items.len();
        let total_pages = filtered_count.div_ceil(page_size);
        let current_page = app.lambda_application_state.resources.selected / page_size;
        let pagination = render_pagination_text(current_page, total_pages);

        crate::ui::filter::render_simple_filter(
            frame,
            chunks_content[0],
            crate::ui::filter::SimpleFilterConfig {
                filter_text: &app.lambda_application_state.resources.filter,
                placeholder: "Filter by attributes or search by keyword",
                pagination: &pagination,
                mode: app.mode,
                is_input_focused: app.lambda_application_state.resource_input_focus
                    == InputFocus::Filter,
                is_pagination_focused: app.lambda_application_state.resource_input_focus
                    == InputFocus::Pagination,
            },
        );

        // Resources table
        let title = format!(
            " Resources ({}) ",
            app.lambda_application_state.resources.items.len()
        );

        let columns: Vec<Box<dyn crate::ui::table::Column<Resource>>> = app
            .lambda_resource_visible_column_ids
            .iter()
            .filter_map(|col_id| {
                crate::lambda::ResourceColumn::from_id(col_id)
                    .map(|col| Box::new(col) as Box<dyn crate::ui::table::Column<Resource>>)
            })
            .collect();
        // let columns: Vec<Box<dyn TableColumn<Resource>>> = vec![
        //     Box::new(column!(name="Logical ID", width=30, type=Resource, field=logical_id)),
        //     Box::new(column!(name="Physical ID", width=40, type=Resource, field=physical_id)),
        //     Box::new(column!(name="Type", width=30, type=Resource, field=resource_type)),
        //     Box::new(column!(name="Last modified", width=27, type=Resource, field=last_modified)),
        // ];

        let start_idx = current_page * page_size;
        let end_idx = (start_idx + page_size).min(filtered_count);
        let paginated: Vec<&Resource> = app.lambda_application_state.resources.items
            [start_idx..end_idx]
            .iter()
            .collect();

        let config = TableConfig {
            items: paginated,
            selected_index: app.lambda_application_state.resources.selected,
            expanded_index: app.lambda_application_state.resources.expanded_item,
            columns: &columns,
            sort_column: "Logical ID",
            sort_direction: SortDirection::Asc,
            title,
            area: chunks_content[1],
            get_expanded_content: Some(Box::new(|res: &Resource| {
                crate::ui::table::plain_expanded_content(format!(
                    "Logical ID: {}\nPhysical ID: {}\nType: {}\nLast modified: {}",
                    res.logical_id, res.physical_id, res.resource_type, res.last_modified
                ))
            })),
            is_active: true,
        };

        render_table(frame, config);
    } else if app.lambda_application_state.detail_tab == ApplicationDetailTab::Deployments {
        let chunks_content = vertical(
            [
                Constraint::Length(3), // Filter
                Constraint::Min(0),    // Table
            ],
            chunks[2],
        );

        // Filter with pagination
        let page_size = app.lambda_application_state.deployments.page_size.value();
        let filtered_count = app.lambda_application_state.deployments.items.len();
        let total_pages = filtered_count.div_ceil(page_size);
        let current_page = app.lambda_application_state.deployments.selected / page_size;
        let pagination = render_pagination_text(current_page, total_pages);

        crate::ui::filter::render_simple_filter(
            frame,
            chunks_content[0],
            crate::ui::filter::SimpleFilterConfig {
                filter_text: &app.lambda_application_state.deployments.filter,
                placeholder: "Filter by attributes or search by keyword",
                pagination: &pagination,
                mode: app.mode,
                is_input_focused: app.lambda_application_state.deployment_input_focus
                    == InputFocus::Filter,
                is_pagination_focused: app.lambda_application_state.deployment_input_focus
                    == InputFocus::Pagination,
            },
        );

        // Table
        let title = format!(
            " Deployment history ({}) ",
            app.lambda_application_state.deployments.items.len()
        );

        use crate::lambda::DeploymentColumn;
        let columns: Vec<Box<dyn TableColumn<Deployment>>> = vec![
            Box::new(DeploymentColumn::Deployment),
            Box::new(DeploymentColumn::ResourceType),
            Box::new(DeploymentColumn::LastUpdated),
            Box::new(DeploymentColumn::Status),
        ];

        let start_idx = current_page * page_size;
        let end_idx = (start_idx + page_size).min(filtered_count);
        let paginated: Vec<&Deployment> = app.lambda_application_state.deployments.items
            [start_idx..end_idx]
            .iter()
            .collect();

        let config = TableConfig {
            items: paginated,
            selected_index: app.lambda_application_state.deployments.selected,
            expanded_index: app.lambda_application_state.deployments.expanded_item,
            columns: &columns,
            sort_column: "",
            sort_direction: SortDirection::Asc,
            title,
            area: chunks_content[1],
            get_expanded_content: Some(Box::new(|dep: &Deployment| {
                crate::ui::table::plain_expanded_content(format!(
                    "Deployment: {}\nResource type: {}\nLast updated: {}\nStatus: {}",
                    dep.deployment_id, dep.resource_type, dep.last_updated, dep.status
                ))
            })),
            is_active: true,
        };

        render_table(frame, config);
    }
}

fn render_lambda_monitoring_charts(frame: &mut Frame, app: &App, area: Rect) {
    use crate::ui::monitoring::{
        render_monitoring_tab, DualAxisChart, MetricChart, MultiDatasetChart,
    };

    // Calculate all labels (same logic as inline version)
    let invocations_sum: f64 = app
        .lambda_state
        .metric_data_invocations
        .iter()
        .map(|(_, v)| v)
        .sum();
    let invocations_label = format!("Invocations [sum: {:.0}]", invocations_sum);

    let duration_min: f64 = app
        .lambda_state
        .metric_data_duration_min
        .iter()
        .map(|(_, v)| v)
        .fold(f64::INFINITY, |a, &b| a.min(b));
    let duration_avg: f64 = if !app.lambda_state.metric_data_duration_avg.is_empty() {
        app.lambda_state
            .metric_data_duration_avg
            .iter()
            .map(|(_, v)| v)
            .sum::<f64>()
            / app.lambda_state.metric_data_duration_avg.len() as f64
    } else {
        0.0
    };
    let duration_max: f64 = app
        .lambda_state
        .metric_data_duration_max
        .iter()
        .map(|(_, v)| v)
        .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let duration_label = format!(
        "Minimum [{:.0}], Average [{:.0}], Maximum [{:.0}]",
        if duration_min.is_finite() {
            duration_min
        } else {
            0.0
        },
        duration_avg,
        if duration_max.is_finite() {
            duration_max
        } else {
            0.0
        }
    );

    let async_event_age_min: f64 = app
        .lambda_state
        .metric_data_async_event_age_min
        .iter()
        .map(|(_, v)| v)
        .fold(f64::INFINITY, |a, &b| a.min(b));
    let async_event_age_avg: f64 = if !app.lambda_state.metric_data_async_event_age_avg.is_empty() {
        app.lambda_state
            .metric_data_async_event_age_avg
            .iter()
            .map(|(_, v)| v)
            .sum::<f64>()
            / app.lambda_state.metric_data_async_event_age_avg.len() as f64
    } else {
        0.0
    };
    let async_event_age_max: f64 = app
        .lambda_state
        .metric_data_async_event_age_max
        .iter()
        .map(|(_, v)| v)
        .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let async_event_age_label = format!(
        "Minimum [{:.0}], Average [{:.0}], Maximum [{:.0}]",
        if async_event_age_min.is_finite() {
            async_event_age_min
        } else {
            0.0
        },
        async_event_age_avg,
        if async_event_age_max.is_finite() {
            async_event_age_max
        } else {
            0.0
        }
    );

    let async_events_received_sum: f64 = app
        .lambda_state
        .metric_data_async_events_received
        .iter()
        .map(|(_, v)| v)
        .sum();
    let async_events_dropped_sum: f64 = app
        .lambda_state
        .metric_data_async_events_dropped
        .iter()
        .map(|(_, v)| v)
        .sum();
    let async_events_label = format!(
        "Received [sum: {:.0}], Dropped [sum: {:.0}]",
        async_events_received_sum, async_events_dropped_sum
    );

    let destination_delivery_failures_sum: f64 = app
        .lambda_state
        .metric_data_destination_delivery_failures
        .iter()
        .map(|(_, v)| v)
        .sum();
    let dead_letter_errors_sum: f64 = app
        .lambda_state
        .metric_data_dead_letter_errors
        .iter()
        .map(|(_, v)| v)
        .sum();
    let async_delivery_failures_label = format!(
        "Destination delivery failures [sum: {:.0}], Dead letter queue failures [sum: {:.0}]",
        destination_delivery_failures_sum, dead_letter_errors_sum
    );

    let iterator_age_max: f64 = app
        .lambda_state
        .metric_data_iterator_age
        .iter()
        .map(|(_, v)| v)
        .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let iterator_age_label = format!(
        "Maximum [{}]",
        if iterator_age_max.is_finite() {
            format!("{:.0}", iterator_age_max)
        } else {
            "--".to_string()
        }
    );

    let error_max: f64 = app
        .lambda_state
        .metric_data_errors
        .iter()
        .map(|(_, v)| v)
        .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let success_rate_min: f64 = app
        .lambda_state
        .metric_data_success_rate
        .iter()
        .map(|(_, v)| v)
        .fold(f64::INFINITY, |a, &b| a.min(b));
    let error_label = format!(
        "Errors [max: {:.0}] and Success rate [min: {:.0}%]",
        if error_max.is_finite() {
            error_max
        } else {
            0.0
        },
        if success_rate_min.is_finite() {
            success_rate_min
        } else {
            0.0
        }
    );

    let throttles_max: f64 = app
        .lambda_state
        .metric_data_throttles
        .iter()
        .map(|(_, v)| v)
        .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let throttles_label = format!(
        "Throttles [max: {:.0}]",
        if throttles_max.is_finite() {
            throttles_max
        } else {
            0.0
        }
    );

    let concurrent_max: f64 = app
        .lambda_state
        .metric_data_concurrent_executions
        .iter()
        .map(|(_, v)| v)
        .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let concurrent_label = format!(
        "Concurrent executions [max: {}]",
        if concurrent_max.is_finite() {
            format!("{:.0}", concurrent_max)
        } else {
            "--".to_string()
        }
    );

    let recursive_sum: f64 = app
        .lambda_state
        .metric_data_recursive_invocations_dropped
        .iter()
        .map(|(_, v)| v)
        .sum();
    let recursive_label = format!(
        "Dropped [sum: {}]",
        if recursive_sum > 0.0 {
            format!("{:.0}", recursive_sum)
        } else {
            "--".to_string()
        }
    );

    render_monitoring_tab(
        frame,
        area,
        &[MetricChart {
            title: "Invocations",
            data: &app.lambda_state.metric_data_invocations,
            y_axis_label: "Count",
            x_axis_label: Some(invocations_label),
        }],
        &[MultiDatasetChart {
            title: "Duration",
            datasets: vec![
                ("Minimum", &app.lambda_state.metric_data_duration_min),
                ("Average", &app.lambda_state.metric_data_duration_avg),
                ("Maximum", &app.lambda_state.metric_data_duration_max),
            ],
            y_axis_label: "Milliseconds",
            y_axis_step: 1000,
            x_axis_label: Some(duration_label),
        }],
        &[DualAxisChart {
            title: "Error count and success rate",
            left_dataset: ("Errors", &app.lambda_state.metric_data_errors),
            right_dataset: ("Success rate", &app.lambda_state.metric_data_success_rate),
            left_y_label: "Count",
            right_y_label: "%",
            x_axis_label: Some(error_label),
        }],
        &[
            MetricChart {
                title: "Throttles",
                data: &app.lambda_state.metric_data_throttles,
                y_axis_label: "Count",
                x_axis_label: Some(throttles_label),
            },
            MetricChart {
                title: "Total concurrent executions",
                data: &app.lambda_state.metric_data_concurrent_executions,
                y_axis_label: "Count",
                x_axis_label: Some(concurrent_label),
            },
            MetricChart {
                title: "Recursive invocations",
                data: &app.lambda_state.metric_data_recursive_invocations_dropped,
                y_axis_label: "Count",
                x_axis_label: Some(recursive_label),
            },
            MetricChart {
                title: "Async event age",
                data: &app.lambda_state.metric_data_async_event_age_avg,
                y_axis_label: "Milliseconds",
                x_axis_label: Some(async_event_age_label),
            },
            MetricChart {
                title: "Async events",
                data: &app.lambda_state.metric_data_async_events_received,
                y_axis_label: "Count",
                x_axis_label: Some(async_events_label),
            },
            MetricChart {
                title: "Async delivery failures",
                data: &app.lambda_state.metric_data_destination_delivery_failures,
                y_axis_label: "Count",
                x_axis_label: Some(async_delivery_failures_label),
            },
            MetricChart {
                title: "Iterator age",
                data: &app.lambda_state.metric_data_iterator_age,
                y_axis_label: "Milliseconds",
                x_axis_label: Some(iterator_age_label),
            },
        ],
        app.lambda_state.monitoring_scroll,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detail_tab_monitoring_in_all() {
        let tabs = DetailTab::ALL;
        assert_eq!(tabs.len(), 5);
        assert_eq!(tabs[0], DetailTab::Code);
        assert_eq!(tabs[1], DetailTab::Monitor);
        assert_eq!(tabs[2], DetailTab::Configuration);
        assert_eq!(tabs[3], DetailTab::Aliases);
        assert_eq!(tabs[4], DetailTab::Versions);
    }

    #[test]
    fn test_detail_tab_monitoring_name() {
        assert_eq!(DetailTab::Monitor.name(), "Monitor");
    }

    #[test]
    fn test_detail_tab_monitoring_navigation() {
        use crate::common::CyclicEnum;
        let tab = DetailTab::Code;
        assert_eq!(tab.next(), DetailTab::Monitor);

        let tab = DetailTab::Monitor;
        assert_eq!(tab.next(), DetailTab::Configuration);
        assert_eq!(tab.prev(), DetailTab::Code);
    }

    #[test]
    fn test_state_monitoring_fields_initialized() {
        let state = State::new();
        assert_eq!(state.monitoring_scroll, 0);
        assert!(state.metric_data_invocations.is_empty());
        assert!(state.metric_data_duration_min.is_empty());
        assert!(state.metric_data_duration_avg.is_empty());
        assert!(state.metric_data_duration_max.is_empty());
        assert!(state.metric_data_errors.is_empty());
        assert!(state.metric_data_success_rate.is_empty());
        assert!(state.metric_data_throttles.is_empty());
        assert!(state.metric_data_concurrent_executions.is_empty());
        assert!(state.metric_data_recursive_invocations_dropped.is_empty());
    }

    #[test]
    fn test_state_monitoring_scroll() {
        let mut state = State::new();
        assert_eq!(state.monitoring_scroll, 0);

        state.monitoring_scroll = 1;
        assert_eq!(state.monitoring_scroll, 1);

        state.monitoring_scroll = 2;
        assert_eq!(state.monitoring_scroll, 2);
    }

    #[test]
    fn test_state_metric_data() {
        let mut state = State::new();
        state.metric_data_invocations = vec![(1700000000, 10.0), (1700000060, 15.0)];
        state.metric_data_duration_min = vec![(1700000000, 100.0), (1700000060, 150.0)];
        state.metric_data_duration_avg = vec![(1700000000, 200.0), (1700000060, 250.0)];
        state.metric_data_duration_max = vec![(1700000000, 300.0), (1700000060, 350.0)];
        state.metric_data_errors = vec![(1700000000, 1.0), (1700000060, 2.0)];
        state.metric_data_success_rate = vec![(1700000000, 90.0), (1700000060, 85.0)];
        state.metric_data_throttles = vec![(1700000000, 0.0), (1700000060, 1.0)];
        state.metric_data_concurrent_executions = vec![(1700000000, 5.0), (1700000060, 10.0)];
        state.metric_data_recursive_invocations_dropped =
            vec![(1700000000, 0.0), (1700000060, 0.0)];

        assert_eq!(state.metric_data_invocations.len(), 2);
        assert_eq!(state.metric_data_duration_min.len(), 2);
        assert_eq!(state.metric_data_duration_avg.len(), 2);
        assert_eq!(state.metric_data_duration_max.len(), 2);
        assert_eq!(state.metric_data_errors.len(), 2);
        assert_eq!(state.metric_data_success_rate.len(), 2);
        assert_eq!(state.metric_data_throttles.len(), 2);
        assert_eq!(state.metric_data_concurrent_executions.len(), 2);
        assert_eq!(state.metric_data_recursive_invocations_dropped.len(), 2);
    }

    #[test]
    fn test_invocations_sum_calculation() {
        let data = [(1700000000, 10.0), (1700000060, 15.0), (1700000120, 5.0)];
        let sum: f64 = data.iter().map(|(_, v)| v).sum();
        assert_eq!(sum, 30.0);
    }

    #[test]
    fn test_invocations_label_format() {
        let sum = 1234.5;
        let label = format!("Invocations [sum: {:.0}]", sum);
        assert_eq!(label, "Invocations [sum: 1234]");
    }

    #[test]
    fn test_invocations_sum_empty() {
        let data: Vec<(i64, f64)> = vec![];
        let sum: f64 = data.iter().map(|(_, v)| v).sum();
        assert_eq!(sum, 0.0);
    }

    #[test]
    fn test_duration_label_formatting() {
        let min = 100.5;
        let avg = 250.7;
        let max = 450.2;
        let label = format!(
            "Minimum [{:.0}], Average [{:.0}], Maximum [{:.0}]",
            min, avg, max
        );
        assert_eq!(label, "Minimum [100], Average [251], Maximum [450]");
    }

    #[test]
    fn test_duration_min_with_infinity() {
        let data: Vec<(i64, f64)> = vec![];
        let min: f64 = data
            .iter()
            .map(|(_, v)| v)
            .fold(f64::INFINITY, |a, &b| a.min(b));
        assert!(min.is_infinite());
        let result = if min.is_finite() { min } else { 0.0 };
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_duration_max_with_neg_infinity() {
        let data: Vec<(i64, f64)> = vec![];
        let max: f64 = data
            .iter()
            .map(|(_, v)| v)
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        assert!(max.is_infinite());
        let result = if max.is_finite() { max } else { 0.0 };
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_duration_avg_empty_data() {
        let data: Vec<(i64, f64)> = vec![];
        let avg: f64 = if !data.is_empty() {
            data.iter().map(|(_, v)| v).sum::<f64>() / data.len() as f64
        } else {
            0.0
        };
        assert_eq!(avg, 0.0);
    }

    #[test]
    fn test_duration_metrics_with_data() {
        let min_data = [(1700000000, 100.0), (1700000060, 90.0), (1700000120, 110.0)];
        let avg_data = [
            (1700000000, 200.0),
            (1700000060, 210.0),
            (1700000120, 190.0),
        ];
        let max_data = [
            (1700000000, 300.0),
            (1700000060, 320.0),
            (1700000120, 310.0),
        ];

        let min: f64 = min_data
            .iter()
            .map(|(_, v)| v)
            .fold(f64::INFINITY, |a, &b| a.min(b));
        let avg: f64 = avg_data.iter().map(|(_, v)| v).sum::<f64>() / avg_data.len() as f64;
        let max: f64 = max_data
            .iter()
            .map(|(_, v)| v)
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        assert_eq!(min, 90.0);
        assert_eq!(avg, 200.0);
        assert_eq!(max, 320.0);
    }

    #[test]
    fn test_success_rate_calculation() {
        let errors: f64 = 5.0;
        let invocations: f64 = 100.0;
        let max_val = errors.max(invocations);
        let success_rate = 100.0 - 100.0 * errors / max_val;
        assert_eq!(success_rate, 95.0);
    }

    #[test]
    fn test_success_rate_with_zero_invocations() {
        let errors: f64 = 0.0;
        let invocations: f64 = 0.0;
        let max_val = errors.max(invocations);
        assert_eq!(max_val, 0.0);
    }

    #[test]
    fn test_error_label_format() {
        let error_max = 10.0;
        let success_rate_min = 85.5;
        let label = format!(
            "Errors [max: {:.0}] and Success rate [min: {:.0}%]",
            error_max, success_rate_min
        );
        assert_eq!(label, "Errors [max: 10] and Success rate [min: 86%]");
    }

    #[test]
    fn test_load_lambda_metrics_builds_resource_string() {
        // Test that version parameter creates correct resource format
        let function_name = "test-function";
        let version = Some("1");
        let resource = version.map(|v| format!("{}:{}", function_name, v));
        assert_eq!(resource, Some("test-function:1".to_string()));

        // Test without version
        let version: Option<&str> = None;
        let resource = version.map(|v| format!("{}:{}", function_name, v));
        assert_eq!(resource, None);
    }

    #[test]
    fn test_detail_tab_next_version_tab() {
        assert_eq!(VersionDetailTab::Code.next(), VersionDetailTab::Monitor);
        assert_eq!(
            VersionDetailTab::Monitor.next(),
            VersionDetailTab::Configuration
        );
        assert_eq!(
            VersionDetailTab::Configuration.next(),
            VersionDetailTab::Code
        );
    }

    #[test]
    fn test_detail_tab_prev_version_tab() {
        assert_eq!(
            VersionDetailTab::Code.prev(),
            VersionDetailTab::Configuration
        );
        assert_eq!(
            VersionDetailTab::Configuration.prev(),
            VersionDetailTab::Monitor
        );
        assert_eq!(VersionDetailTab::Monitor.prev(), VersionDetailTab::Code);
    }
}
