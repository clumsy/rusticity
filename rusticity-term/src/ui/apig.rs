use crate::apig::api::{self, RestApi};
use crate::apig::resource::Resource as ApigResource;
use crate::apig::route::Route;
use crate::app::App;
use crate::common::{
    filter_by_field, render_pagination_text, CyclicEnum, InputFocus, SortDirection,
};
use crate::keymap::Mode;
use crate::table::TableState;
use crate::ui::filter::{render_simple_filter, SimpleFilterConfig};
use crate::ui::render_tabs;
use crate::ui::table::{expanded_from_columns, render_table, Column as TableColumn, TableConfig};
use crate::ui::tree::TreeItem;
use ratatui::prelude::*;
use ratatui::widgets::Row;
use std::collections::{HashMap, HashSet};

pub const FILTER_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ApiDetailTab {
    Routes,
    // Authorization,
    // Integrations,
    // Cors,
    // Reimport,
    // Export,
    // Stages,
    // Metrics,
    // Logging,
    // Throttling,
}

impl CyclicEnum for ApiDetailTab {
    const ALL: &'static [Self] = &[
        Self::Routes,
        // Self::Authorization,
        // Self::Integrations,
        // Self::Cors,
        // Self::Reimport,
        // Self::Export,
        // Self::Stages,
        // Self::Metrics,
        // Self::Logging,
        // Self::Throttling,
    ];

    fn next(&self) -> Self {
        match self {
            Self::Routes => Self::Routes,
            // Self::Authorization => Self::Integrations,
            // Self::Integrations => Self::Cors,
            // Self::Cors => Self::Reimport,
            // Self::Reimport => Self::Export,
            // Self::Export => Self::Stages,
            // Self::Stages => Self::Metrics,
            // Self::Metrics => Self::Logging,
            // Self::Logging => Self::Throttling,
            // Self::Throttling => Self::Routes,
        }
    }

    fn prev(&self) -> Self {
        match self {
            Self::Routes => Self::Routes,
            // Self::Authorization => Self::Routes,
            // Self::Integrations => Self::Authorization,
            // Self::Cors => Self::Integrations,
            // Self::Reimport => Self::Cors,
            // Self::Export => Self::Reimport,
            // Self::Stages => Self::Export,
            // Self::Metrics => Self::Stages,
            // Self::Logging => Self::Metrics,
            // Self::Throttling => Self::Logging,
        }
    }
}

impl ApiDetailTab {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Routes => "Routes",
            // Self::Authorization => "Authorization",
            // Self::Integrations => "Integrations",
            // Self::Cors => "CORS",
            // Self::Reimport => "Reimport",
            // Self::Export => "Export",
            // Self::Stages => "Stages",
            // Self::Metrics => "Metrics",
            // Self::Logging => "Logging",
            // Self::Throttling => "Throttling",
        }
    }

    pub fn as_str_for_api(&self, protocol_type: &str) -> &'static str {
        if protocol_type.to_uppercase() == "REST" {
            match self {
                Self::Routes => "Resources",
                // Self::Authorization => "Authorizers",
                // Self::Integrations => "Gateway Responses",
                // Self::Cors => "Models",
                // Self::Reimport => "Resource Policy",
                // Self::Export => "Documentation",
                // Self::Stages => "Stages",
                // Self::Metrics => "Dashboard",
                // Self::Logging => "Settings",
                // Self::Throttling => "API Keys",
            }
        } else {
            self.as_str()
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Routes,
            // Self::Authorization,
            // Self::Integrations,
            // Self::Cors,
            // Self::Reimport,
            // Self::Export,
            // Self::Stages,
            // Self::Metrics,
            // Self::Logging,
            // Self::Throttling,
        ]
    }
}

pub struct State {
    pub apis: TableState<RestApi>,
    pub input_focus: InputFocus,
    pub current_api: Option<RestApi>,
    pub detail_tab: ApiDetailTab,
    pub routes: TableState<Route>,
    pub expanded_routes: HashSet<String>,
    pub route_children: HashMap<String, Vec<Route>>,
    pub resources: TableState<ApigResource>,
    pub expanded_resources: HashSet<String>,
    pub resource_children: HashMap<String, Vec<ApigResource>>,
    pub route_filter: String,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            apis: TableState::new(),
            input_focus: InputFocus::Filter,
            current_api: None,
            detail_tab: ApiDetailTab::Routes,
            routes: TableState::new(),
            expanded_routes: HashSet::new(),
            route_children: HashMap::new(),
            resources: TableState::new(),
            expanded_resources: HashSet::new(),
            resource_children: HashMap::new(),
            route_filter: String::new(),
        }
    }
}

pub fn filtered_apis(app: &App) -> Vec<&RestApi> {
    filter_by_field(
        &app.apig_state.apis.items,
        &app.apig_state.apis.filter,
        |a| &a.name,
    )
}

pub fn render_apis(frame: &mut Frame, app: &App, area: Rect) {
    // Check if we're in detail view
    if app.apig_state.current_api.is_some() {
        render_api_detail(frame, app, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter
            Constraint::Min(0),    // Table
        ])
        .split(area);

    // Calculate pagination
    let filtered: Vec<_> = filtered_apis(app);
    let filtered_count = filtered.len();

    let page_size = app.apig_state.apis.page_size.value();
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = app.apig_state.apis.selected / page_size;
    let pagination = render_pagination_text(current_page, total_pages);

    // Filter
    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &app.apig_state.apis.filter,
            placeholder: "Find APIs",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.apig_state.input_focus == InputFocus::Filter,
            is_pagination_focused: app.apig_state.input_focus == InputFocus::Pagination,
        },
    );

    // Apply pagination
    let start_idx = current_page * page_size;
    let end_idx = (start_idx + page_size).min(filtered.len());
    let paginated: Vec<_> = filtered[start_idx..end_idx].to_vec();

    let title = format!("APIs ({}) ", filtered_count);

    // Define columns
    let columns: Vec<Box<dyn TableColumn<RestApi>>> = app
        .apig_api_visible_column_ids
        .iter()
        .filter_map(|col_id| {
            api::Column::from_id(col_id).map(|col| Box::new(col) as Box<dyn TableColumn<RestApi>>)
        })
        .collect();

    let expanded_index = app.apig_state.apis.expanded_item.and_then(|idx| {
        if idx >= start_idx && idx < end_idx {
            Some(idx - start_idx)
        } else {
            None
        }
    });

    let config = TableConfig {
        items: paginated,
        selected_index: app.apig_state.apis.selected % page_size,
        expanded_index,
        columns: &columns,
        sort_column: "Name",
        sort_direction: SortDirection::Asc,
        title,
        area: chunks[1],
        get_expanded_content: Some(Box::new(|api: &RestApi| {
            expanded_from_columns(&columns, api)
        })),
        is_active: app.mode != Mode::FilterInput,
    };

    render_table(frame, config);
}

fn render_api_detail(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(api) = &app.apig_state.current_api {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Tabs
                Constraint::Min(0),    // Content
            ])
            .split(area);

        // Render tabs with API-type-specific names
        let tabs: Vec<(&str, ApiDetailTab)> = ApiDetailTab::all()
            .iter()
            .map(|t| (t.as_str_for_api(&api.protocol_type), *t))
            .collect();
        render_tabs(frame, chunks[0], &tabs, &app.apig_state.detail_tab);

        // Render tab content
        let inner = chunks[1];

        // Format status with color
        let (_status_text, _status_color) = crate::apig::format_status(&api.status);

        // Show API details based on selected tab
        match app.apig_state.detail_tab {
            ApiDetailTab::Routes => {
                // Split for filter and content
                let route_chunks = Layout::default()
                    .direction(ratatui::layout::Direction::Vertical)
                    .constraints([
                        Constraint::Length(3), // Filter
                        Constraint::Min(0),    // Content
                    ])
                    .split(inner);

                // Render filter
                render_simple_filter(
                    frame,
                    route_chunks[0],
                    SimpleFilterConfig {
                        filter_text: &app.apig_state.route_filter,
                        placeholder: "Search",
                        pagination: "",
                        mode: app.mode,
                        is_input_focused: app.mode == Mode::FilterInput
                            && app.apig_state.current_api.is_some()
                            && app.apig_state.detail_tab == ApiDetailTab::Routes,
                        is_pagination_focused: false,
                    },
                );

                let content_area = route_chunks[1];

                // Check if this is a REST API (v1) - they have resources, not routes
                if api.protocol_type.to_uppercase() == "REST" {
                    if app.apig_state.resources.loading {
                        let loading = ratatui::widgets::Paragraph::new("Loading resources...")
                            .style(Style::default().fg(Color::Yellow));
                        frame.render_widget(loading, content_area);
                    } else if app.apig_state.resources.items.is_empty() {
                        let empty = ratatui::widgets::Paragraph::new("No resources found")
                            .style(Style::default().fg(Color::Yellow));
                        frame.render_widget(empty, content_area);
                    } else {
                        use crate::ui::tree::TreeRenderer;

                        // Apply filter
                        let (filtered_items, filtered_children) = filter_tree_items(
                            &app.apig_state.resources.items,
                            &app.apig_state.resource_children,
                            &app.apig_state.route_filter,
                        );

                        let renderer = TreeRenderer::<ApigResource>::new(
                            &filtered_items,
                            &app.apig_state.expanded_resources,
                            &filtered_children,
                            app.apig_state.resources.selected,
                            0,
                        );

                        use crate::apig::resource::Column as ResourceColumn;
                        use crate::ui::table::Column as TableColumn;

                        // Get visible columns - always include first column (Resource)
                        let mut visible_columns: Vec<ResourceColumn> = vec![ResourceColumn::Path];
                        visible_columns.extend(
                            app.apig_resource_visible_column_ids
                                .iter()
                                .filter_map(|id| ResourceColumn::from_id(id))
                                .filter(|col| *col != ResourceColumn::Path),
                        );

                        let tree_rows = renderer.render(|resource, tree_prefix| {
                            visible_columns
                                .iter()
                                .enumerate()
                                .map(|(i, col)| {
                                    if i == 0 {
                                        let (text, _) = col.render(resource);
                                        ratatui::widgets::Cell::from(format!(
                                            "{}{}",
                                            tree_prefix, text
                                        ))
                                    } else {
                                        let (text, _) = col.render(resource);
                                        ratatui::widgets::Cell::from(text)
                                    }
                                })
                                .collect()
                        });

                        let headers: Vec<&str> =
                            visible_columns.iter().map(|col| col.name()).collect();

                        let widths: Vec<Constraint> = visible_columns
                            .iter()
                            .map(|col| Constraint::Length(col.width()))
                            .collect();

                        let rows: Vec<Row> = tree_rows
                            .into_iter()
                            .map(|(cells, style)| Row::new(cells).style(style))
                            .collect();

                        crate::ui::table::render_tree_table(
                            frame,
                            content_area,
                            format!("{} - Resources ", api.name),
                            headers,
                            rows,
                            widths,
                            app.mode != Mode::FilterInput,
                        );
                    }
                } else {
                    // Render routes tree for HTTP/WebSocket APIs
                    if app.apig_state.routes.loading {
                        let message = vec![Line::from(""), Line::from("Loading routes...")];
                        let paragraph = ratatui::widgets::Paragraph::new(message)
                            .style(Style::default().fg(Color::Yellow));
                        frame.render_widget(paragraph, content_area);
                    } else if app.apig_state.routes.items.is_empty() {
                        let message = vec![
                            Line::from(""),
                            Line::from("No routes found for this API."),
                            Line::from(""),
                            Line::from(format!("API ID: {}", api.id)),
                            Line::from(format!("Protocol: {}", api.protocol_type)),
                        ];
                        let paragraph = ratatui::widgets::Paragraph::new(message)
                            .style(Style::default().fg(Color::Yellow));
                        frame.render_widget(paragraph, content_area);
                    } else {
                        use crate::ui::tree::TreeRenderer;
                        use ratatui::widgets::Row;

                        // Apply filter
                        let (filtered_items, filtered_children) = filter_tree_items(
                            &app.apig_state.routes.items,
                            &app.apig_state.route_children,
                            &app.apig_state.route_filter,
                        );

                        let renderer = TreeRenderer::new(
                            &filtered_items,
                            &app.apig_state.expanded_routes,
                            &filtered_children,
                            app.apig_state.routes.selected,
                            0,
                        );

                        use crate::apig::route::Column as RouteColumn;
                        use crate::ui::table::Column as TableColumn;

                        // Get visible columns - always include first column (Route)
                        let mut visible_columns: Vec<RouteColumn> = vec![RouteColumn::RouteKey];
                        visible_columns.extend(
                            app.apig_route_visible_column_ids
                                .iter()
                                .filter_map(|id| RouteColumn::from_id(id))
                                .filter(|col| *col != RouteColumn::RouteKey), // Skip if already added
                        );

                        let tree_rows = renderer.render(|route, tree_prefix| {
                            visible_columns
                                .iter()
                                .enumerate()
                                .map(|(i, col)| {
                                    if i == 0 {
                                        // First column gets tree prefix
                                        let (text, _) = col.render(route);
                                        ratatui::widgets::Cell::from(format!(
                                            "{}{}",
                                            tree_prefix, text
                                        ))
                                    } else {
                                        let (text, _) = col.render(route);
                                        ratatui::widgets::Cell::from(text)
                                    }
                                })
                                .collect()
                        });

                        let headers: Vec<&str> =
                            visible_columns.iter().map(|col| col.name()).collect();

                        let widths: Vec<Constraint> = visible_columns
                            .iter()
                            .map(|col| Constraint::Length(col.width()))
                            .collect();

                        let rows: Vec<Row> = tree_rows
                            .into_iter()
                            .map(|(cells, style)| Row::new(cells).style(style))
                            .collect();

                        crate::ui::table::render_tree_table(
                            frame,
                            content_area,
                            format!("{} - Routes ", api.name),
                            headers,
                            rows,
                            widths,
                            app.mode != Mode::FilterInput,
                        );
                    }
                }
            }
        }
    }
}

pub async fn load_routes(app: &mut App, api_id: &str) -> anyhow::Result<()> {
    let client = rusticity_core::apig::ApiGatewayClient::new(app.config.clone());
    let routes = client.list_routes(api_id).await?;

    let route_items: Vec<Route> = routes
        .into_iter()
        .map(|r| Route {
            route_id: r.route_id.clone(),
            route_key: r.route_key.clone(),
            target: r.target,
            authorization_type: r.authorization_type,
            api_key_required: r.api_key_required,
            display_name: r.route_key,
            arn: r.arn,
        })
        .collect();

    // Build hierarchy from paths
    let (root_routes, children_map) = build_route_hierarchy(route_items);

    app.apig_state.routes.items = root_routes;
    app.apig_state.route_children = children_map;

    Ok(())
}

/// Build hierarchical structure from flat route list based on path segments
pub fn build_route_hierarchy(routes: Vec<Route>) -> (Vec<Route>, HashMap<String, Vec<Route>>) {
    let mut all_routes: HashMap<String, Route> = HashMap::new();
    let mut children_map: HashMap<String, Vec<Route>> = HashMap::new();

    // First pass: collect all actual routes and split method from path
    for route in routes {
        let route_key = &route.route_key;

        // Check if route has HTTP method prefix
        if route_key.contains(' ') {
            let parts: Vec<&str> = route_key.split_whitespace().collect();
            if parts.len() == 2 {
                let method = parts[0];
                let path = parts[1];

                // Store the path node (without method)
                if !all_routes.contains_key(path) {
                    let display = if path == "/" {
                        "/".to_string()
                    } else {
                        path.rsplit('/')
                            .next()
                            .map(|s| format!("/{}", s))
                            .unwrap_or(path.to_string())
                    };

                    all_routes.insert(
                        path.to_string(),
                        Route {
                            route_id: String::new(), // Virtual parent has no ID
                            route_key: path.to_string(),
                            target: String::new(),
                            authorization_type: String::new(),
                            api_key_required: false,
                            display_name: display,
                            arn: String::new(), // Virtual parent has no ARN
                        },
                    );
                }

                // Create method child node
                let method_route = Route {
                    route_id: route.route_id.clone(),
                    route_key: method.to_string(),
                    target: route.target.clone(),
                    authorization_type: route.authorization_type.clone(),
                    api_key_required: route.api_key_required,
                    display_name: method.to_string(),
                    arn: route.arn.clone(),
                };

                children_map
                    .entry(path.to_string())
                    .or_default()
                    .push(method_route);
                continue;
            }
        }

        // Regular route without method prefix - set display_name
        let route_key_clone = route_key.clone();
        let mut route_with_display = route;
        if route_with_display.display_name.is_empty() {
            route_with_display.display_name = if route_key_clone == "/" {
                "/".to_string()
            } else if route_key_clone.starts_with('/') {
                route_key_clone
                    .rsplit('/')
                    .next()
                    .map(|s| format!("/{}", s))
                    .unwrap_or(route_key_clone.clone())
            } else {
                route_key_clone.clone()
            };
        }
        all_routes.insert(route_key_clone, route_with_display);
    }

    // Second pass: create all virtual parent nodes
    let route_keys: Vec<String> = all_routes.keys().cloned().collect();

    for route_key in &route_keys {
        // Special routes (WebSocket) have no parents
        if route_key.starts_with('$') {
            continue;
        }

        // Extract path from route key (may include HTTP method like "GET /path")
        let path = if route_key.contains(' ') {
            // Format: "GET /path" - extract path part
            route_key.split_whitespace().nth(1).unwrap_or(route_key)
        } else {
            // Format: "/path"
            route_key.as_str()
        };

        // Split path into segments
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        // Create all parent levels
        for i in 1..segments.len() {
            let parent_segments = &segments[..i];
            let parent_path = format!("/{}", parent_segments.join("/"));

            // Create virtual parent if it doesn't exist
            if !all_routes.contains_key(&parent_path) {
                let display = format!("/{}", segments[i - 1]);
                all_routes.insert(
                    parent_path.clone(),
                    Route {
                        route_id: String::new(), // Virtual parent has no ID
                        route_key: parent_path.clone(),
                        target: String::new(),
                        authorization_type: String::new(),
                        api_key_required: false,
                        display_name: display,
                        arn: String::new(),
                    },
                );
            }
        }
    }

    // Third pass: build parent-child relationships
    let all_route_keys: Vec<String> = all_routes.keys().cloned().collect();

    for route_key in &all_route_keys {
        // Special routes (WebSocket) have no parents
        if route_key.starts_with('$') {
            continue;
        }

        // Extract path from route key
        let path = if route_key.contains(' ') {
            route_key.split_whitespace().nth(1).unwrap_or(route_key)
        } else {
            route_key.as_str()
        };

        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        if segments.len() > 1 {
            // Parent is all segments except last
            let parent_segments = &segments[..segments.len() - 1];
            let parent_path = format!("/{}", parent_segments.join("/"));

            // Add this route as child of parent
            if let Some(route) = all_routes.get(route_key) {
                children_map
                    .entry(parent_path)
                    .or_default()
                    .push(route.clone());
            }
        }
    }

    // Collect root routes (single segment or WebSocket)
    let mut root_routes = Vec::new();
    for (route_key, route) in &all_routes {
        if route_key.starts_with('$') {
            root_routes.push(route.clone());
            continue;
        }

        let path = if route_key.contains(' ') {
            route_key.split_whitespace().nth(1).unwrap_or(route_key)
        } else {
            route_key.as_str()
        };

        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if segments.len() == 1 {
            root_routes.push(route.clone());
        }
    }

    (root_routes, children_map)
}

pub async fn load_resources(app: &mut App, api_id: &str) -> anyhow::Result<()> {
    let client = rusticity_core::apig::ApiGatewayClient::new(app.config.clone());
    let resources = client.list_resources(api_id).await?;

    let resource_items: Vec<ApigResource> = resources
        .into_iter()
        .map(|r| ApigResource {
            id: r.id,
            path: r.path,
            parent_id: r.parent_id,
            methods: r.methods,
            display_name: r.display_name,
            arn: r.arn,
        })
        .collect();

    let (root_resources, children_map) = build_resource_hierarchy(resource_items);

    app.apig_state.resources.items = root_resources;
    app.apig_state.resource_children = children_map;

    Ok(())
}

pub fn build_resource_hierarchy(
    mut resources: Vec<ApigResource>,
) -> (Vec<ApigResource>, HashMap<String, Vec<ApigResource>>) {
    let mut children_map: HashMap<String, Vec<ApigResource>> = HashMap::new();

    // Mark resources that have children
    let parent_ids: std::collections::HashSet<String> = resources
        .iter()
        .filter_map(|r| r.parent_id.clone())
        .collect();

    for resource in &mut resources {
        if parent_ids.contains(&resource.id) && resource.methods.is_empty() {
            resource.methods.push("_has_children".to_string());
        }
    }

    let mut root_resources = Vec::new();

    for resource in resources {
        // Create method children for this resource (skip marker)
        let mut method_children = Vec::new();
        for method in &resource.methods {
            if method != "_has_children" {
                method_children.push(ApigResource {
                    id: format!("{}#{}", resource.id, method),
                    path: method.clone(),
                    parent_id: Some(resource.id.clone()),
                    methods: vec![],
                    display_name: method.clone(),
                    arn: String::new(), // Method children don't have ARNs
                });
            }
        }

        if !method_children.is_empty() {
            children_map.insert(resource.id.clone(), method_children);
        }

        // Add resource to parent or root
        if let Some(parent_id) = &resource.parent_id {
            children_map
                .entry(parent_id.clone())
                .or_default()
                .push(resource);
        } else {
            root_resources.push(resource);
        }
    }

    (root_resources, children_map)
}

pub fn filter_tree_items<T: TreeItem + Clone>(
    items: &[T],
    children_map: &HashMap<String, Vec<T>>,
    filter: &str,
) -> (Vec<T>, HashMap<String, Vec<T>>) {
    if filter.is_empty() {
        return (items.to_vec(), children_map.clone());
    }

    let filter_lower = filter.to_lowercase();
    let mut filtered_items = Vec::new();
    let mut filtered_children = HashMap::new();

    for item in items {
        if matches_filter(item, children_map, &filter_lower, &mut filtered_children) {
            filtered_items.push(item.clone());
        }
    }

    (filtered_items, filtered_children)
}

fn matches_filter<T: TreeItem + Clone>(
    item: &T,
    children_map: &HashMap<String, Vec<T>>,
    filter: &str,
    filtered_children: &mut HashMap<String, Vec<T>>,
) -> bool {
    // Check if item itself matches
    if item.display_name().to_lowercase().contains(filter) {
        // Include all children
        if let Some(children) = children_map.get(item.id()) {
            filtered_children.insert(item.id().to_string(), children.clone());
        }
        return true;
    }

    // Check if any child matches
    if let Some(children) = children_map.get(item.id()) {
        let mut matching_children = Vec::new();
        for child in children {
            if matches_filter(child, children_map, filter, filtered_children) {
                matching_children.push(child.clone());
            }
        }

        if !matching_children.is_empty() {
            filtered_children.insert(item.id().to_string(), matching_children);
            return true;
        }
    }

    false
}
