use crate::app::App;
use crate::column;
use crate::common::CyclicEnum;
use crate::common::{
    format_bytes, format_duration_seconds, format_memory_mb, render_pagination_text, InputFocus,
    SortDirection,
};
use crate::keymap::Mode;
use crate::lambda::{
    format_architecture, format_runtime, Alias, AliasColumn, Application as LambdaApplication,
    Column as LambdaColumn, Deployment, Function as LambdaFunction, Layer, LayerColumn, Resource,
    Version, VersionColumn,
};
use crate::table::TableState;
use crate::ui::table::{expanded_from_columns, render_table, Column as TableColumn, TableConfig};
use crate::ui::{labeled_field, render_tabs, section_header, vertical};
use ratatui::{prelude::*, widgets::*};

pub const FILTER_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

pub struct State {
    pub table: TableState<LambdaFunction>,
    pub current_function: Option<String>,
    pub current_version: Option<String>,
    pub current_alias: Option<String>,
    pub detail_tab: DetailTab,
    pub visible_columns: Vec<String>,
    pub all_columns: Vec<String>,
    pub version_table: TableState<Version>,
    pub visible_version_columns: Vec<String>,
    pub all_version_columns: Vec<String>,
    pub alias_table: TableState<Alias>,
    pub visible_alias_columns: Vec<String>,
    pub all_alias_columns: Vec<String>,
    pub visible_layer_columns: Vec<String>,
    pub all_layer_columns: Vec<String>,
    pub input_focus: InputFocus,
    pub version_input_focus: InputFocus,
    pub alias_input_focus: InputFocus,
    pub layer_selected: usize,
    pub layer_expanded: Option<usize>,
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
            visible_columns: LambdaColumn::visible(),
            all_columns: LambdaColumn::all(),
            version_table: TableState::new(),
            visible_version_columns: VersionColumn::all()
                .iter()
                .map(|c| c.name().to_string())
                .collect(),
            all_version_columns: VersionColumn::all()
                .iter()
                .map(|c| c.name().to_string())
                .collect(),
            alias_table: TableState::new(),
            visible_alias_columns: AliasColumn::all()
                .iter()
                .map(|c| c.name().to_string())
                .collect(),
            all_alias_columns: AliasColumn::all()
                .iter()
                .map(|c| c.name().to_string())
                .collect(),
            visible_layer_columns: LayerColumn::all()
                .iter()
                .map(|c| c.name().to_string())
                .collect(),
            all_layer_columns: LayerColumn::all()
                .iter()
                .map(|c| c.name().to_string())
                .collect(),
            input_focus: InputFocus::Filter,
            version_input_focus: InputFocus::Filter,
            alias_input_focus: InputFocus::Filter,
            layer_selected: 0,
            layer_expanded: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetailTab {
    Code,
    Configuration,
    Aliases,
    Versions,
}

impl CyclicEnum for DetailTab {
    const ALL: &'static [Self] = &[
        Self::Code,
        Self::Configuration,
        Self::Aliases,
        Self::Versions,
    ];
}

impl DetailTab {
    pub fn name(&self) -> &'static str {
        match self {
            DetailTab::Code => "Code",
            DetailTab::Configuration => "Configuration",
            DetailTab::Aliases => "Aliases",
            DetailTab::Versions => "Versions",
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
    pub fn name(&self) -> &str {
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
    for col_id in &app.lambda_state.visible_columns {
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
            .border_type(BorderType::Plain)
            .border_style(Style::default());

        let overview_inner = overview_block.inner(chunks[0]);
        frame.render_widget(overview_block, chunks[0]);
        frame.render_widget(Paragraph::new(overview_lines), overview_inner);
    }

    // Tabs
    let tabs = [
        ("Code", DetailTab::Code),
        ("Configuration", DetailTab::Configuration),
        ("Aliases", DetailTab::Aliases),
        ("Versions", DetailTab::Versions),
    ];

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
                let chunks_content = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(10), // Code properties (includes KMS)
                        Constraint::Length(8),  // Runtime settings (includes Runtime management)
                        Constraint::Min(0),     // Layers
                    ])
                    .split(chunks[2]);

                // Code properties section
                let code_block = Block::default()
                    .title(" Code properties ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Plain);

                let code_inner = code_block.inner(chunks_content[0]);
                frame.render_widget(code_block, chunks_content[0]);

                let code_lines = vec![
                    labeled_field("Package size", format_bytes(func.code_size)),
                    labeled_field("SHA256 hash", &func.code_sha256),
                    labeled_field("Last modified", &func.last_modified),
                    section_header("Encryption with AWS KMS customer managed KMS key", code_inner.width),
                    Line::from(Span::styled(
                        "To edit customer managed key encryption, you must upload a new .zip deployment package.",
                        Style::default().fg(Color::DarkGray),
                    )),
                    labeled_field("AWS KMS key ARN", ""),
                    labeled_field("Key alias", ""),
                    labeled_field("Status", ""),
                ];

                frame.render_widget(Paragraph::new(code_lines), code_inner);

                // Runtime settings section
                let runtime_block = Block::default()
                    .title(" Runtime settings ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Plain);

                let runtime_inner = runtime_block.inner(chunks_content[1]);
                frame.render_widget(runtime_block, chunks_content[1]);

                let runtime_lines = vec![
                    labeled_field("Runtime", format_runtime(&func.runtime)),
                    labeled_field("Handler", ""),
                    labeled_field("Architecture", format_architecture(&func.architecture)),
                    section_header("Runtime management configuration", runtime_inner.width),
                    labeled_field("Runtime version ARN", ""),
                    labeled_field("Update runtime version", "Auto"),
                ];

                frame.render_widget(Paragraph::new(runtime_lines), runtime_inner);

                // Layers section
                #[derive(Clone)]
                struct Layer {
                    merge_order: String,
                    name: String,
                    layer_version: String,
                    compatible_runtimes: String,
                    compatible_architectures: String,
                    version_arn: String,
                }

                let layers: Vec<Layer> = func
                    .layers
                    .iter()
                    .enumerate()
                    .map(|(i, l)| {
                        let parts: Vec<&str> = l.arn.split(':').collect();
                        let name = parts.get(6).unwrap_or(&"").to_string();
                        let version = parts.get(7).unwrap_or(&"").to_string();
                        Layer {
                            merge_order: (i + 1).to_string(),
                            name,
                            layer_version: version,
                            compatible_runtimes: "-".to_string(),
                            compatible_architectures: "-".to_string(),
                            version_arn: l.arn.clone(),
                        }
                    })
                    .collect();
                let layer_refs: Vec<&Layer> = layers.iter().collect();
                let title = format!(" Layers ({}) ", layer_refs.len());

                let columns: Vec<Box<dyn TableColumn<Layer>>> = vec![
                    Box::new(column!(name="Merge order", width=12, type=Layer, field=merge_order)),
                    Box::new(column!(name="Name", width=20, type=Layer, field=name)),
                    Box::new(
                        column!(name="Layer version", width=14, type=Layer, field=layer_version),
                    ),
                    Box::new(
                        column!(name="Compatible runtimes", width=20, type=Layer, field=compatible_runtimes),
                    ),
                    Box::new(
                        column!(name="Compatible architectures", width=26, type=Layer, field=compatible_architectures),
                    ),
                    Box::new(column!(name="Version ARN", width=40, type=Layer, field=version_arn)),
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
                let config_block = Block::default()
                    .title(" General configuration ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Plain)
                    .border_style(Style::default());

                let config_inner = config_block.inner(chunks[2]);
                frame.render_widget(config_block, chunks[2]);

                let config_lines = vec![
                    labeled_field("Description", &func.description),
                    labeled_field("Revision", &func.last_modified),
                    labeled_field("Memory", format_memory_mb(func.memory_mb)),
                    labeled_field("Ephemeral storage", format_memory_mb(512)),
                    labeled_field("Timeout", format_duration_seconds(func.timeout_seconds)),
                    labeled_field("SnapStart", "None"),
                ];

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
        for col_name in &app.lambda_state.visible_version_columns {
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
        for col_name in &app.lambda_state.visible_alias_columns {
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
        .block(Block::default().borders(Borders::ALL));
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
                        .border_type(BorderType::Plain)
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
            .border_type(BorderType::Plain);

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
            .border_type(BorderType::Plain)
            .border_style(Style::default());

        let overview_inner = overview_block.inner(chunks[0]);
        frame.render_widget(overview_block, chunks[0]);
        frame.render_widget(Paragraph::new(overview_lines), overview_inner);
    }

    // Tabs - only Code and Configuration
    let tabs = [
        ("Code", DetailTab::Code),
        ("Configuration", DetailTab::Configuration),
    ];

    render_tabs(frame, chunks[1], &tabs, &app.lambda_state.detail_tab);

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
                let chunks_content = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(5),
                        Constraint::Length(5),
                        Constraint::Min(0),
                    ])
                    .split(chunks[2]);

                // Code properties section
                let code_block = Block::default()
                    .title(" Code properties ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Plain);

                let code_inner = code_block.inner(chunks_content[0]);
                frame.render_widget(code_block, chunks_content[0]);

                let code_lines = vec![
                    labeled_field("Package size", format_bytes(func.code_size)),
                    labeled_field("SHA256 hash", &func.code_sha256),
                    labeled_field("Last modified", &func.last_modified),
                ];

                frame.render_widget(Paragraph::new(code_lines), code_inner);

                // Runtime settings section
                let runtime_block = Block::default()
                    .title(" Runtime settings ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Plain);

                let runtime_inner = runtime_block.inner(chunks_content[1]);
                frame.render_widget(runtime_block, chunks_content[1]);

                let runtime_lines = vec![
                    labeled_field("Runtime", format_runtime(&func.runtime)),
                    labeled_field("Handler", ""),
                    labeled_field("Architecture", format_architecture(&func.architecture)),
                ];

                frame.render_widget(Paragraph::new(runtime_lines), runtime_inner);

                // Layers section (empty table)
                #[derive(Clone)]
                struct Layer {
                    merge_order: String,
                    name: String,
                    layer_version: String,
                    compatible_runtimes: String,
                    compatible_architectures: String,
                    version_arn: String,
                }

                let layers: Vec<Layer> = vec![];
                let layer_refs: Vec<&Layer> = layers.iter().collect();
                let title = format!(" Layers ({}) ", layer_refs.len());

                let columns: Vec<Box<dyn TableColumn<Layer>>> = vec![
                    Box::new(column!(name="Merge order", width=12, type=Layer, field=merge_order)),
                    Box::new(column!(name="Name", width=20, type=Layer, field=name)),
                    Box::new(
                        column!(name="Layer version", width=14, type=Layer, field=layer_version),
                    ),
                    Box::new(
                        column!(name="Compatible runtimes", width=20, type=Layer, field=compatible_runtimes),
                    ),
                    Box::new(
                        column!(name="Compatible architectures", width=26, type=Layer, field=compatible_architectures),
                    ),
                    Box::new(column!(name="Version ARN", width=40, type=Layer, field=version_arn)),
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
                    let chunks_content = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(5),
                            Constraint::Length(3), // Filter
                            Constraint::Min(0),    // Aliases table
                        ])
                        .split(chunks[2]);

                    let config_block = Block::default()
                        .title(" General configuration ")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Plain)
                        .border_style(Style::default());

                    let config_inner = config_block.inner(chunks_content[0]);
                    frame.render_widget(config_block, chunks_content[0]);

                    let config_lines = vec![
                        labeled_field("Description", &func.description),
                        labeled_field("Memory", format_memory_mb(func.memory_mb)),
                        labeled_field("Timeout", format_duration_seconds(func.timeout_seconds)),
                    ];

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
                    for col_name in &app.lambda_state.visible_alias_columns {
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
    for col_id in &app.visible_lambda_application_columns {
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
                .map(|l| Layer {
                    arn: l.arn,
                    code_size: l.code_size,
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
    let tabs = [
        ("Overview", ApplicationDetailTab::Overview),
        ("Deployments", ApplicationDetailTab::Deployments),
    ];
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

        let columns: Vec<Box<dyn TableColumn<Resource>>> = vec![
            Box::new(column!(name="Logical ID", width=30, type=Resource, field=logical_id)),
            Box::new(column!(name="Physical ID", width=40, type=Resource, field=physical_id)),
            Box::new(column!(name="Type", width=30, type=Resource, field=resource_type)),
            Box::new(column!(name="Last modified", width=27, type=Resource, field=last_modified)),
        ];

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
            DeploymentColumn::Deployment.as_table_column(),
            DeploymentColumn::ResourceType.as_table_column(),
            DeploymentColumn::LastUpdated.as_table_column(),
            DeploymentColumn::Status.as_table_column(),
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
