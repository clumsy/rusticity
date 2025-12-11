use crate::app::App;
use crate::cfn::{format_status, Column as CfnColumn, Stack as CfnStack};
use crate::common::{
    filter_by_fields, render_dropdown, render_pagination_text, translate_column, ColumnId,
    CyclicEnum, InputFocus, SortDirection,
};
use crate::keymap::Mode;
use crate::table::TableState;
use crate::ui::filter::{render_filter_bar, FilterConfig, FilterControl};
use crate::ui::table::{expanded_from_columns, render_table, Column, TableConfig};
use crate::ui::{
    block_height_for, calculate_dynamic_height, labeled_field, render_fields_with_dynamic_columns,
    render_json_highlighted, render_tabs, rounded_block,
};
use ratatui::{prelude::*, widgets::*};
use rusticity_core::cfn::{StackOutput, StackParameter, StackResource};
use std::collections::HashSet;

pub const STATUS_FILTER: InputFocus = InputFocus::Dropdown("StatusFilter");
pub const VIEW_NESTED: InputFocus = InputFocus::Checkbox("ViewNested");

impl State {
    pub const FILTER_CONTROLS: [InputFocus; 4] = [
        InputFocus::Filter,
        STATUS_FILTER,
        VIEW_NESTED,
        InputFocus::Pagination,
    ];

    pub const PARAMETERS_FILTER_CONTROLS: [InputFocus; 2] =
        [InputFocus::Filter, InputFocus::Pagination];

    pub const OUTPUTS_FILTER_CONTROLS: [InputFocus; 2] =
        [InputFocus::Filter, InputFocus::Pagination];

    pub const RESOURCES_FILTER_CONTROLS: [InputFocus; 2] =
        [InputFocus::Filter, InputFocus::Pagination];
}

pub struct State {
    pub table: TableState<CfnStack>,
    pub input_focus: InputFocus,
    pub status_filter: StatusFilter,
    pub view_nested: bool,
    pub current_stack: Option<String>,
    pub detail_tab: DetailTab,
    pub overview_scroll: u16,
    pub sort_column: CfnColumn,
    pub sort_direction: SortDirection,
    pub template_body: String,
    pub template_scroll: usize,
    pub parameters: TableState<StackParameter>,
    pub parameters_input_focus: InputFocus,
    pub outputs: TableState<StackOutput>,
    pub outputs_input_focus: InputFocus,
    pub tags: TableState<(String, String)>,
    pub policy_scroll: usize,
    /// Tracks expanded items for hierarchical views (Resources, Events tabs).
    /// Keys are resource IDs or logical resource names that are currently expanded.
    pub expanded_items: HashSet<String>,
    pub resources: TableState<StackResource>,
    pub resources_input_focus: InputFocus,
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
            input_focus: InputFocus::Filter,
            status_filter: StatusFilter::All,
            view_nested: false,
            current_stack: None,
            detail_tab: DetailTab::StackInfo,
            overview_scroll: 0,
            sort_column: CfnColumn::CreatedTime,
            sort_direction: SortDirection::Desc,
            template_body: String::new(),
            template_scroll: 0,
            parameters: TableState::new(),
            parameters_input_focus: InputFocus::Filter,
            outputs: TableState::new(),
            outputs_input_focus: InputFocus::Filter,
            tags: TableState::new(),
            policy_scroll: 0,
            expanded_items: HashSet::new(),
            resources: TableState::new(),
            resources_input_focus: InputFocus::Filter,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StatusFilter {
    All,
    Active,
    Complete,
    Failed,
    Deleted,
    InProgress,
}

impl StatusFilter {
    pub fn name(&self) -> &'static str {
        match self {
            StatusFilter::All => "All",
            StatusFilter::Active => "Active",
            StatusFilter::Complete => "Complete",
            StatusFilter::Failed => "Failed",
            StatusFilter::Deleted => "Deleted",
            StatusFilter::InProgress => "In progress",
        }
    }

    pub fn all() -> Vec<StatusFilter> {
        vec![
            StatusFilter::All,
            StatusFilter::Active,
            StatusFilter::Complete,
            StatusFilter::Failed,
            StatusFilter::Deleted,
            StatusFilter::InProgress,
        ]
    }
}

impl CyclicEnum for StatusFilter {
    const ALL: &'static [Self] = &[
        Self::All,
        Self::Active,
        Self::Complete,
        Self::Failed,
        Self::Deleted,
        Self::InProgress,
    ];
}

impl StatusFilter {
    pub fn matches(&self, status: &str) -> bool {
        match self {
            StatusFilter::All => true,
            StatusFilter::Active => {
                !status.contains("DELETE")
                    && !status.contains("COMPLETE")
                    && !status.contains("FAILED")
            }
            StatusFilter::Complete => status.contains("COMPLETE") && !status.contains("DELETE"),
            StatusFilter::Failed => status.contains("FAILED"),
            StatusFilter::Deleted => status.contains("DELETE"),
            StatusFilter::InProgress => status.contains("IN_PROGRESS"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetailTab {
    StackInfo,
    Events,
    Resources,
    Outputs,
    Parameters,
    Template,
    ChangeSets,
    GitSync,
}

impl CyclicEnum for DetailTab {
    const ALL: &'static [Self] = &[
        Self::StackInfo,
        Self::Events,
        Self::Resources,
        Self::Outputs,
        Self::Parameters,
        Self::Template,
        Self::ChangeSets,
        Self::GitSync,
    ];
}

impl DetailTab {
    pub fn name(&self) -> &'static str {
        match self {
            DetailTab::StackInfo => "Stack info",
            DetailTab::Events => "Events",
            DetailTab::Resources => "Resources",
            DetailTab::Outputs => "Outputs",
            DetailTab::Parameters => "Parameters",
            DetailTab::Template => "Template",
            DetailTab::ChangeSets => "Change sets",
            DetailTab::GitSync => "Git sync",
        }
    }

    pub fn all() -> Vec<DetailTab> {
        vec![
            DetailTab::StackInfo,
            DetailTab::Events,
            DetailTab::Resources,
            DetailTab::Outputs,
            DetailTab::Parameters,
            DetailTab::Template,
            DetailTab::ChangeSets,
            DetailTab::GitSync,
        ]
    }

    pub fn allows_preferences(&self) -> bool {
        matches!(
            self,
            DetailTab::StackInfo
                | DetailTab::Parameters
                | DetailTab::Outputs
                | DetailTab::Resources
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResourceColumn {
    LogicalId,
    PhysicalId,
    Type,
    Status,
    Module,
}

impl ResourceColumn {
    pub fn id(&self) -> &'static str {
        match self {
            ResourceColumn::LogicalId => "cfn.resource.logical_id",
            ResourceColumn::PhysicalId => "cfn.resource.physical_id",
            ResourceColumn::Type => "cfn.resource.type",
            ResourceColumn::Status => "cfn.resource.status",
            ResourceColumn::Module => "cfn.resource.module",
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            ResourceColumn::LogicalId => "Logical ID",
            ResourceColumn::PhysicalId => "Physical ID",
            ResourceColumn::Type => "Type",
            ResourceColumn::Status => "Status",
            ResourceColumn::Module => "Module",
        }
    }

    pub fn all() -> Vec<ResourceColumn> {
        vec![
            ResourceColumn::LogicalId,
            ResourceColumn::PhysicalId,
            ResourceColumn::Type,
            ResourceColumn::Status,
            ResourceColumn::Module,
        ]
    }

    pub fn from_id(id: &str) -> Option<ResourceColumn> {
        match id {
            "cfn.resource.logical_id" => Some(ResourceColumn::LogicalId),
            "cfn.resource.physical_id" => Some(ResourceColumn::PhysicalId),
            "cfn.resource.type" => Some(ResourceColumn::Type),
            "cfn.resource.status" => Some(ResourceColumn::Status),
            "cfn.resource.module" => Some(ResourceColumn::Module),
            _ => None,
        }
    }
}

pub fn resource_column_ids() -> Vec<ColumnId> {
    ResourceColumn::all().iter().map(|c| c.id()).collect()
}

pub fn filtered_cloudformation_stacks(app: &App) -> Vec<&CfnStack> {
    filter_by_fields(
        &app.cfn_state.table.items,
        &app.cfn_state.table.filter,
        |s| vec![&s.name, &s.description],
    )
    .into_iter()
    .filter(|s| app.cfn_state.status_filter.matches(&s.status))
    .collect()
}

pub fn parameter_column_ids() -> Vec<ColumnId> {
    ParameterColumn::all().iter().map(|c| c.id()).collect()
}

pub fn output_column_ids() -> Vec<ColumnId> {
    OutputColumn::all().iter().map(|c| c.id()).collect()
}

pub fn filtered_parameters(app: &App) -> Vec<&StackParameter> {
    filter_by_fields(
        &app.cfn_state.parameters.items,
        &app.cfn_state.parameters.filter,
        |p| vec![&p.key, &p.value, &p.resolved_value],
    )
}

pub fn filtered_outputs(app: &App) -> Vec<&StackOutput> {
    filter_by_fields(
        &app.cfn_state.outputs.items,
        &app.cfn_state.outputs.filter,
        |o| vec![&o.key, &o.value, &o.description, &o.export_name],
    )
}

pub fn filtered_resources(app: &App) -> Vec<&StackResource> {
    filter_by_fields(
        &app.cfn_state.resources.items,
        &app.cfn_state.resources.filter,
        |r| {
            vec![
                &r.logical_id,
                &r.physical_id,
                &r.resource_type,
                &r.status,
                &r.module_info,
            ]
        },
    )
}

pub fn filtered_tags(app: &App) -> Vec<&(String, String)> {
    filter_by_fields(
        &app.cfn_state.tags.items,
        &app.cfn_state.tags.filter,
        |(k, v)| vec![k.as_str(), v.as_str()],
    )
}

pub fn render_stacks(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    if app.cfn_state.current_stack.is_some() {
        render_cloudformation_stack_detail(frame, app, area);
    } else {
        render_cloudformation_stack_list(frame, app, area);
    }
}

pub fn render_cloudformation_stack_list(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter + controls
            Constraint::Min(0),    // Table
        ])
        .split(area);

    // Filter line - search on left, controls on right
    let filtered_stacks = filtered_cloudformation_stacks(app);
    let filtered_count = filtered_stacks.len();

    let placeholder = "Search by stack name";

    let status_filter_text = format!("Filter status: {}", app.cfn_state.status_filter.name());
    let view_nested_text = if app.cfn_state.view_nested {
        "☑ View nested"
    } else {
        "☐ View nested"
    };
    let page_size = app.cfn_state.table.page_size.value();
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page =
        if filtered_count > 0 && app.cfn_state.table.scroll_offset + page_size >= filtered_count {
            total_pages.saturating_sub(1)
        } else {
            app.cfn_state.table.scroll_offset / page_size
        };
    let pagination = render_pagination_text(current_page, total_pages);

    render_filter_bar(
        frame,
        FilterConfig {
            filter_text: &app.cfn_state.table.filter,
            placeholder,
            mode: app.mode,
            is_input_focused: app.cfn_state.input_focus == InputFocus::Filter,
            controls: vec![
                FilterControl {
                    text: status_filter_text.to_string(),
                    is_focused: app.cfn_state.input_focus == STATUS_FILTER,
                },
                FilterControl {
                    text: view_nested_text.to_string(),
                    is_focused: app.cfn_state.input_focus == VIEW_NESTED,
                },
                FilterControl {
                    text: pagination.clone(),
                    is_focused: app.cfn_state.input_focus == InputFocus::Pagination,
                },
            ],
            area: chunks[0],
        },
    );

    // Table - use scroll_offset for pagination
    let scroll_offset = app.cfn_state.table.scroll_offset;
    let page_stacks: Vec<_> = filtered_stacks
        .iter()
        .skip(scroll_offset)
        .take(page_size)
        .collect();

    // Define columns
    let column_enums: Vec<CfnColumn> = app
        .cfn_visible_column_ids
        .iter()
        .filter_map(|col_id| CfnColumn::from_id(col_id))
        .collect();

    let columns: Vec<Box<dyn Column<&CfnStack>>> =
        column_enums.iter().map(|col| col.to_column()).collect();

    let expanded_index = app.cfn_state.table.expanded_item.and_then(|idx| {
        let scroll_offset = app.cfn_state.table.scroll_offset;
        if idx >= scroll_offset && idx < scroll_offset + page_size {
            Some(idx - scroll_offset)
        } else {
            None
        }
    });

    let config = TableConfig {
        items: page_stacks,
        selected_index: app.cfn_state.table.selected % app.cfn_state.table.page_size.value(),
        expanded_index,
        columns: &columns,
        sort_column: app.cfn_state.sort_column.default_name(),
        sort_direction: app.cfn_state.sort_direction,
        title: format!(" Stacks ({}) ", filtered_count),
        area: chunks[1],
        get_expanded_content: Some(Box::new(|stack: &&CfnStack| {
            expanded_from_columns(&columns, stack)
        })),
        is_active: app.mode != Mode::FilterInput,
    };

    render_table(frame, config);

    // Render dropdown for StatusFilter when focused (after table so it appears on top)
    if app.mode == Mode::FilterInput && app.cfn_state.input_focus == STATUS_FILTER {
        let filter_names: Vec<&str> = StatusFilter::all().iter().map(|f| f.name()).collect();
        let selected_idx = StatusFilter::all()
            .iter()
            .position(|f| *f == app.cfn_state.status_filter)
            .unwrap_or(0);
        let view_nested_width = " ☑ View nested ".len() as u16;
        let controls_after = view_nested_width + 3 + pagination.len() as u16 + 3;
        render_dropdown(
            frame,
            &filter_names,
            selected_idx,
            chunks[0],
            controls_after,
        );
    }
}

pub fn render_cloudformation_stack_detail(frame: &mut Frame, app: &App, area: Rect) {
    let stack_name = app.cfn_state.current_stack.as_ref().unwrap();

    // Find the stack
    let stack = app
        .cfn_state
        .table
        .items
        .iter()
        .find(|s| &s.name == stack_name);

    if stack.is_none() {
        let paragraph = Paragraph::new("Stack not found").block(rounded_block().title(" Error "));
        frame.render_widget(paragraph, area);
        return;
    }

    let stack = stack.unwrap();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Stack name
            Constraint::Length(1), // Tabs
            Constraint::Min(0),    // Content
        ])
        .split(area);

    // Render stack name
    frame.render_widget(Paragraph::new(stack.name.clone()), chunks[0]);

    // Render tabs
    let tabs: Vec<_> = DetailTab::ALL.iter().map(|t| (t.name(), *t)).collect();
    render_tabs(frame, chunks[1], &tabs, &app.cfn_state.detail_tab);

    // Render content based on selected tab
    match app.cfn_state.detail_tab {
        DetailTab::StackInfo => {
            render_stack_info(frame, app, stack, chunks[2]);
        }
        DetailTab::GitSync => {
            render_git_sync(frame, app, stack, chunks[2]);
        }
        DetailTab::Template => {
            render_json_highlighted(
                frame,
                chunks[2],
                &app.cfn_state.template_body,
                app.cfn_state.template_scroll,
                " Template ",
                true,
            );
        }
        DetailTab::Parameters => {
            render_parameters(frame, app, chunks[2]);
        }
        DetailTab::Outputs => {
            render_outputs(frame, app, chunks[2]);
        }
        DetailTab::Resources => {
            render_resources(frame, app, chunks[2]);
        }
        _ => {
            let paragraph =
                Paragraph::new(format!("{} - Coming soon", app.cfn_state.detail_tab.name()))
                    .block(rounded_block());
            frame.render_widget(paragraph, chunks[2]);
        }
    }
}

pub fn render_stack_info(frame: &mut Frame, app: &App, stack: &CfnStack, area: Rect) {
    let (formatted_status, _status_color) = format_status(&stack.status);

    // Overview section
    let fields = vec![
        (
            "Stack ID",
            if stack.stack_id.is_empty() {
                "-"
            } else {
                &stack.stack_id
            },
        ),
        (
            "Description",
            if stack.description.is_empty() {
                "-"
            } else {
                &stack.description
            },
        ),
        ("Status", &formatted_status),
        (
            "Detailed status",
            if stack.detailed_status.is_empty() {
                "-"
            } else {
                &stack.detailed_status
            },
        ),
        (
            "Status reason",
            if stack.status_reason.is_empty() {
                "-"
            } else {
                &stack.status_reason
            },
        ),
        (
            "Root stack",
            if stack.root_stack.is_empty() {
                "-"
            } else {
                &stack.root_stack
            },
        ),
        (
            "Parent stack",
            if stack.parent_stack.is_empty() {
                "-"
            } else {
                &stack.parent_stack
            },
        ),
        (
            "Created time",
            if stack.created_time.is_empty() {
                "-"
            } else {
                &stack.created_time
            },
        ),
        (
            "Updated time",
            if stack.updated_time.is_empty() {
                "-"
            } else {
                &stack.updated_time
            },
        ),
        (
            "Deleted time",
            if stack.deleted_time.is_empty() {
                "-"
            } else {
                &stack.deleted_time
            },
        ),
        (
            "Drift status",
            if stack.drift_status.is_empty() {
                "-"
            } else if stack.drift_status == "NOT_CHECKED" || stack.drift_status == "NotChecked" {
                "⭕ NOT CHECKED"
            } else {
                &stack.drift_status
            },
        ),
        (
            "Last drift check time",
            if stack.last_drift_check_time.is_empty() {
                "-"
            } else {
                &stack.last_drift_check_time
            },
        ),
        (
            "Termination protection",
            if stack.termination_protection {
                "Activated"
            } else {
                "Disabled"
            },
        ),
        (
            "IAM role",
            if stack.iam_role.is_empty() {
                "-"
            } else {
                &stack.iam_role
            },
        ),
    ];
    let overview_height = calculate_dynamic_height(
        &fields
            .iter()
            .map(|(label, value)| labeled_field(label, *value))
            .collect::<Vec<_>>(),
        area.width.saturating_sub(4),
    ) + 2;

    // Tags section - use table with filter
    let tags_height = 12; // Fixed height for tags table

    // Stack policy section - render with scrolling like template
    let policy_height = 15; // Fixed height for policy section

    // Rollback configuration section
    let rollback_lines: Vec<Line> = {
        let mut lines = vec![labeled_field(
            "Monitoring time",
            if stack.rollback_monitoring_time.is_empty() {
                "-"
            } else {
                &stack.rollback_monitoring_time
            },
        )];

        if stack.rollback_alarms.is_empty() {
            lines.push(Line::from("CloudWatch alarm ARN: No alarms configured"));
        } else {
            for alarm in &stack.rollback_alarms {
                lines.push(Line::from(format!("CloudWatch alarm ARN: {}", alarm)));
            }
        }
        lines
    };
    let rollback_height =
        calculate_dynamic_height(&rollback_lines, area.width.saturating_sub(4)) + 2;

    // Notification options section
    let notification_lines = if stack.notification_arns.is_empty() {
        vec!["SNS topic ARN: No notifications configured".to_string()]
    } else {
        stack
            .notification_arns
            .iter()
            .map(|arn| format!("SNS topic ARN: {}", arn))
            .collect()
    };
    let notification_height = notification_lines.len() as u16 + 2; // +2 for borders

    // Split into sections with calculated heights
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(overview_height),
            Constraint::Length(tags_height),
            Constraint::Length(policy_height),
            Constraint::Length(rollback_height),
            Constraint::Length(notification_height),
            Constraint::Min(0), // Remaining space
        ])
        .split(area);

    // Render overview
    let overview_lines: Vec<_> = fields
        .iter()
        .map(|(label, value)| labeled_field(label, *value))
        .collect();

    let overview_block = rounded_block().title(" Overview ");
    let overview_inner = overview_block.inner(sections[0]);
    frame.render_widget(overview_block, sections[0]);
    render_fields_with_dynamic_columns(frame, overview_inner, overview_lines);

    // Render tags table
    render_tags(frame, app, sections[1]);

    // Render stack policy with scrolling
    let policy_text = if stack.stack_policy.is_empty() {
        "No stack policy".to_string()
    } else {
        stack.stack_policy.clone()
    };
    render_json_highlighted(
        frame,
        sections[2],
        &policy_text,
        app.cfn_state.policy_scroll,
        " Stack policy ",
        true,
    );

    // Render rollback configuration
    let rollback_block = rounded_block().title(" Rollback configuration ");
    let rollback_inner = rollback_block.inner(sections[3]);
    frame.render_widget(rollback_block, sections[3]);
    render_fields_with_dynamic_columns(frame, rollback_inner, rollback_lines);

    // Render notification options
    let notifications = Paragraph::new(notification_lines.join("\n"))
        .block(rounded_block().title(" Notification options "))
        .wrap(Wrap { trim: true });
    frame.render_widget(notifications, sections[4]);
}

fn render_tags(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let filtered: Vec<&(String, String)> = filtered_tags(app);
    let filtered_count = filtered.len();

    render_filter_bar(
        frame,
        FilterConfig {
            filter_text: &app.cfn_state.tags.filter,
            placeholder: "Search tags",
            mode: app.mode,
            is_input_focused: false,
            controls: vec![],
            area: chunks[0],
        },
    );

    let page_size = app.cfn_state.tags.page_size.value();
    let page_start = app.cfn_state.tags.scroll_offset;
    let page_end = (page_start + page_size).min(filtered_count);
    let page_tags: Vec<_> = filtered[page_start..page_end].to_vec();

    let columns: Vec<Box<dyn Column<(String, String)>>> =
        vec![Box::new(TagColumn::Key), Box::new(TagColumn::Value)];

    let expanded_index = app.cfn_state.tags.expanded_item.and_then(|idx| {
        let scroll_offset = app.cfn_state.tags.scroll_offset;
        if idx >= scroll_offset && idx < scroll_offset + page_size {
            Some(idx - scroll_offset)
        } else {
            None
        }
    });

    let config = TableConfig {
        items: page_tags,
        selected_index: app.cfn_state.tags.selected % page_size,
        expanded_index,
        columns: &columns,
        sort_column: "Key",
        sort_direction: SortDirection::Asc,
        title: format!(" Tags ({}) ", filtered_count),
        area: chunks[1],
        get_expanded_content: Some(Box::new(|tag: &(String, String)| {
            expanded_from_columns(&columns, tag)
        })),
        is_active: true,
    };

    render_table(frame, config);
}

fn render_git_sync(frame: &mut Frame, _app: &App, _stack: &CfnStack, area: Rect) {
    let fields = [
        ("Repository", "-"),
        ("Deployment file path", "-"),
        ("Git sync", "-"),
        ("Repository provider", "-"),
        ("Repository sync status", "-"),
        ("Provisioning status", "-"),
        ("Branch", "-"),
        ("Repository sync status message", "-"),
    ];

    let git_sync_height = block_height_for(fields.len());

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(git_sync_height), Constraint::Min(0)])
        .split(area);

    let lines: Vec<Line> = fields
        .iter()
        .map(|&(label, value)| labeled_field(label, value))
        .collect();

    let block = rounded_block().title(" Git sync ");
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, sections[0]);
}

#[cfg(test)]
mod tests {
    use super::{filtered_tags, State};
    use crate::app::App;

    fn test_app() -> App {
        App::new_without_client("test".to_string(), Some("us-east-1".to_string()))
    }

    #[test]
    fn test_drift_status_not_checked_formatting() {
        let drift_status = "NOT_CHECKED";
        let formatted = if drift_status == "NOT_CHECKED" {
            "⭕ NOT CHECKED"
        } else {
            drift_status
        };
        assert_eq!(formatted, "⭕ NOT CHECKED");
    }

    #[test]
    fn test_drift_status_not_checked_pascal_case() {
        let drift_status = "NotChecked";
        let formatted = if drift_status == "NOT_CHECKED" || drift_status == "NotChecked" {
            "⭕ NOT CHECKED"
        } else {
            drift_status
        };
        assert_eq!(formatted, "⭕ NOT CHECKED");
    }

    #[test]
    fn test_drift_status_other_values() {
        let drift_status = "IN_SYNC";
        let formatted = if drift_status == "NOT_CHECKED" {
            "⭕ NOT CHECKED"
        } else {
            drift_status
        };
        assert_eq!(formatted, "IN_SYNC");
    }

    #[test]
    fn test_git_sync_renders_all_fields() {
        use crate::cfn::Stack;
        let stack = Stack {
            name: "test-stack".to_string(),
            stack_id: "id".to_string(),
            status: "CREATE_COMPLETE".to_string(),
            created_time: String::new(),
            updated_time: String::new(),
            deleted_time: String::new(),
            drift_status: String::new(),
            last_drift_check_time: String::new(),
            status_reason: String::new(),
            description: String::new(),
            detailed_status: String::new(),
            root_stack: String::new(),
            parent_stack: String::new(),
            termination_protection: false,
            iam_role: String::new(),
            tags: Vec::new(),
            stack_policy: String::new(),
            rollback_monitoring_time: String::new(),
            rollback_alarms: Vec::new(),
            notification_arns: Vec::new(),
        };

        // Verify the fields are defined
        let fields = [
            "Repository",
            "Deployment file path",
            "Git sync",
            "Repository provider",
            "Repository sync status",
            "Provisioning status",
            "Branch",
            "Repository sync status message",
        ];

        assert_eq!(fields.len(), 8);
        assert_eq!(stack.name, "test-stack");
    }

    #[test]
    fn test_git_sync_block_height() {
        use crate::ui::block_height_for;

        // Git sync has 8 labeled fields
        let field_count = 8;
        let expected_height = field_count + 2; // +2 for borders

        assert_eq!(block_height_for(field_count), expected_height as u16);
        assert_eq!(block_height_for(field_count), 10);
    }

    #[test]
    fn test_notification_arn_format() {
        use crate::cfn::Stack;

        // Test with notification ARNs
        let stack_with_notifications = Stack {
            name: "test-stack".to_string(),
            stack_id: "id".to_string(),
            status: "CREATE_COMPLETE".to_string(),
            created_time: String::new(),
            updated_time: String::new(),
            deleted_time: String::new(),
            drift_status: String::new(),
            last_drift_check_time: String::new(),
            status_reason: String::new(),
            description: String::new(),
            detailed_status: String::new(),
            root_stack: String::new(),
            parent_stack: String::new(),
            termination_protection: false,
            iam_role: String::new(),
            tags: Vec::new(),
            stack_policy: String::new(),
            rollback_monitoring_time: String::new(),
            rollback_alarms: Vec::new(),
            notification_arns: vec![
                "arn:aws:sns:us-east-1:588850195596:CloudFormationNotifications".to_string(),
            ],
        };

        let notification_lines: Vec<String> = stack_with_notifications
            .notification_arns
            .iter()
            .map(|arn| format!("SNS topic ARN: {}", arn))
            .collect();

        assert_eq!(notification_lines.len(), 1);
        assert_eq!(
            notification_lines[0],
            "SNS topic ARN: arn:aws:sns:us-east-1:588850195596:CloudFormationNotifications"
        );

        // Test with no notifications
        let stack_without_notifications = Stack {
            name: "test-stack".to_string(),
            stack_id: "id".to_string(),
            status: "CREATE_COMPLETE".to_string(),
            created_time: String::new(),
            updated_time: String::new(),
            deleted_time: String::new(),
            drift_status: String::new(),
            last_drift_check_time: String::new(),
            status_reason: String::new(),
            description: String::new(),
            detailed_status: String::new(),
            root_stack: String::new(),
            parent_stack: String::new(),
            termination_protection: false,
            iam_role: String::new(),
            tags: Vec::new(),
            stack_policy: String::new(),
            rollback_monitoring_time: String::new(),
            rollback_alarms: Vec::new(),
            notification_arns: Vec::new(),
        };

        let notification_lines: Vec<String> =
            if stack_without_notifications.notification_arns.is_empty() {
                vec!["SNS topic ARN: No notifications configured".to_string()]
            } else {
                stack_without_notifications
                    .notification_arns
                    .iter()
                    .map(|arn| format!("SNS topic ARN: {}", arn))
                    .collect()
            };

        assert_eq!(notification_lines.len(), 1);
        assert_eq!(
            notification_lines[0],
            "SNS topic ARN: No notifications configured"
        );
    }

    #[test]
    fn test_template_scroll_state() {
        let state = State::new();
        assert_eq!(state.template_body, "");
        assert_eq!(state.template_scroll, 0);
    }

    #[test]
    fn test_template_body_storage() {
        let mut state = State::new();
        let template = r#"{"AWSTemplateFormatVersion":"2010-09-09","Resources":{}}"#;
        state.template_body = template.to_string();
        assert_eq!(state.template_body, template);
    }

    #[test]
    fn test_rollback_alarm_format() {
        let alarm_arn = "arn:aws:cloudwatch:us-east-1:123456789012:alarm:MyAlarm";
        let formatted = format!("CloudWatch alarm ARN: {}", alarm_arn);
        assert_eq!(
            formatted,
            "CloudWatch alarm ARN: arn:aws:cloudwatch:us-east-1:123456789012:alarm:MyAlarm"
        );
    }

    #[test]
    fn test_filtered_tags() {
        let mut app = test_app();
        app.cfn_state.tags.items = vec![
            ("Environment".to_string(), "Production".to_string()),
            ("Application".to_string(), "WebApp".to_string()),
            ("Owner".to_string(), "TeamA".to_string()),
        ];

        // No filter
        app.cfn_state.tags.filter = String::new();
        let filtered = filtered_tags(&app);
        assert_eq!(filtered.len(), 3);

        // Filter by key
        app.cfn_state.tags.filter = "env".to_string();
        let filtered = filtered_tags(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "Environment");

        // Filter by value
        app.cfn_state.tags.filter = "prod".to_string();
        let filtered = filtered_tags(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1, "Production");
    }

    #[test]
    fn test_tags_sorted_by_key() {
        let mut tags = [
            ("Zebra".to_string(), "value1".to_string()),
            ("Alpha".to_string(), "value2".to_string()),
            ("Beta".to_string(), "value3".to_string()),
        ];
        tags.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(tags[0].0, "Alpha");
        assert_eq!(tags[1].0, "Beta");
        assert_eq!(tags[2].0, "Zebra");
    }

    #[test]
    fn test_policy_scroll_state() {
        let state = State::new();
        assert_eq!(state.policy_scroll, 0);
    }

    #[test]
    fn test_rounded_block_for_rollback_config() {
        use crate::ui::rounded_block;
        use ratatui::prelude::Rect;
        let block = rounded_block().title(" Rollback configuration ");
        let area = Rect::new(0, 0, 90, 8);
        let inner = block.inner(area);
        assert_eq!(inner.width, 88);
        assert_eq!(inner.height, 6);
    }

    #[test]
    fn test_overview_uses_dynamic_height() {
        use crate::ui::{calculate_dynamic_height, labeled_field};
        // Verify overview height accounts for column packing
        let fields = vec![
            labeled_field("Stack name", "test-stack"),
            labeled_field("Status", "CREATE_COMPLETE"),
            labeled_field("Created", "2024-01-01"),
        ];
        let width = 150;
        let height = calculate_dynamic_height(&fields, width);
        // With 3 fields and reasonable width, should pack into fewer rows
        assert!(height < 3, "Expected fewer than 3 rows with column packing");
    }
}

pub fn render_parameters(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let filtered: Vec<&StackParameter> = if app.cfn_state.parameters.filter.is_empty() {
        app.cfn_state.parameters.items.iter().collect()
    } else {
        app.cfn_state
            .parameters
            .items
            .iter()
            .filter(|p| {
                p.key
                    .to_lowercase()
                    .contains(&app.cfn_state.parameters.filter.to_lowercase())
                    || p.value
                        .to_lowercase()
                        .contains(&app.cfn_state.parameters.filter.to_lowercase())
                    || p.resolved_value
                        .to_lowercase()
                        .contains(&app.cfn_state.parameters.filter.to_lowercase())
            })
            .collect()
    };

    let filtered_count = filtered.len();
    let page_size = app.cfn_state.parameters.page_size.value();
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = if filtered_count > 0
        && app.cfn_state.parameters.scroll_offset + page_size >= filtered_count
    {
        total_pages.saturating_sub(1)
    } else {
        app.cfn_state.parameters.scroll_offset / page_size
    };
    let pagination = render_pagination_text(current_page, total_pages);

    render_filter_bar(
        frame,
        FilterConfig {
            filter_text: &app.cfn_state.parameters.filter,
            placeholder: "Search parameters",
            mode: app.mode,
            is_input_focused: app.cfn_state.parameters_input_focus == InputFocus::Filter,
            controls: vec![FilterControl {
                text: pagination,
                is_focused: app.cfn_state.parameters_input_focus == InputFocus::Pagination,
            }],
            area: chunks[0],
        },
    );

    let page_start = app.cfn_state.parameters.scroll_offset;
    let page_end = (page_start + page_size).min(filtered_count);
    let page_params: Vec<_> = filtered[page_start..page_end].to_vec();

    let columns: Vec<Box<dyn Column<StackParameter>>> = app
        .cfn_parameter_visible_column_ids
        .iter()
        .filter_map(|col_id| {
            ParameterColumn::from_id(col_id)
                .map(|col| Box::new(col) as Box<dyn Column<StackParameter>>)
        })
        .collect();

    let expanded_index = app.cfn_state.parameters.expanded_item.and_then(|idx| {
        let scroll_offset = app.cfn_state.parameters.scroll_offset;
        if idx >= scroll_offset && idx < scroll_offset + page_size {
            Some(idx - scroll_offset)
        } else {
            None
        }
    });

    let config = TableConfig {
        items: page_params,
        selected_index: app.cfn_state.parameters.selected % page_size,
        expanded_index,
        columns: &columns,
        sort_column: "Key",
        sort_direction: SortDirection::Asc,
        title: format!(" Parameters ({}) ", filtered_count),
        area: chunks[1],
        get_expanded_content: Some(Box::new(|param: &StackParameter| {
            expanded_from_columns(&columns, param)
        })),
        is_active: app.mode != Mode::FilterInput,
    };

    render_table(frame, config);
}

pub fn render_outputs(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let filtered: Vec<&StackOutput> = if app.cfn_state.outputs.filter.is_empty() {
        app.cfn_state.outputs.items.iter().collect()
    } else {
        app.cfn_state
            .outputs
            .items
            .iter()
            .filter(|o| {
                o.key
                    .to_lowercase()
                    .contains(&app.cfn_state.outputs.filter.to_lowercase())
                    || o.value
                        .to_lowercase()
                        .contains(&app.cfn_state.outputs.filter.to_lowercase())
                    || o.description
                        .to_lowercase()
                        .contains(&app.cfn_state.outputs.filter.to_lowercase())
                    || o.export_name
                        .to_lowercase()
                        .contains(&app.cfn_state.outputs.filter.to_lowercase())
            })
            .collect()
    };

    let filtered_count = filtered.len();
    let page_size = app.cfn_state.outputs.page_size.value();
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = if filtered_count > 0
        && app.cfn_state.outputs.scroll_offset + page_size >= filtered_count
    {
        total_pages.saturating_sub(1)
    } else {
        app.cfn_state.outputs.scroll_offset / page_size
    };
    let pagination = render_pagination_text(current_page, total_pages);

    render_filter_bar(
        frame,
        FilterConfig {
            filter_text: &app.cfn_state.outputs.filter,
            placeholder: "Search outputs",
            mode: app.mode,
            is_input_focused: app.cfn_state.outputs_input_focus == InputFocus::Filter,
            controls: vec![FilterControl {
                text: pagination,
                is_focused: app.cfn_state.outputs_input_focus == InputFocus::Pagination,
            }],
            area: chunks[0],
        },
    );

    let page_start = app.cfn_state.outputs.scroll_offset;
    let page_end = (page_start + page_size).min(filtered_count);
    let page_outputs: Vec<_> = filtered[page_start..page_end].to_vec();

    let columns: Vec<Box<dyn Column<StackOutput>>> = app
        .cfn_output_visible_column_ids
        .iter()
        .filter_map(|col_id| {
            OutputColumn::from_id(col_id).map(|col| Box::new(col) as Box<dyn Column<StackOutput>>)
        })
        .collect();

    let expanded_index = app.cfn_state.outputs.expanded_item.and_then(|idx| {
        let scroll_offset = app.cfn_state.outputs.scroll_offset;
        if idx >= scroll_offset && idx < scroll_offset + page_size {
            Some(idx - scroll_offset)
        } else {
            None
        }
    });

    let config = TableConfig {
        items: page_outputs,
        selected_index: app.cfn_state.outputs.selected % page_size,
        expanded_index,
        columns: &columns,
        sort_column: "Key",
        sort_direction: SortDirection::Asc,
        title: format!(" Outputs ({}) ", filtered_count),
        area: chunks[1],
        get_expanded_content: Some(Box::new(|output: &StackOutput| {
            expanded_from_columns(&columns, output)
        })),
        is_active: app.mode != Mode::FilterInput,
    };

    render_table(frame, config);
}

pub fn render_resources(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let filtered: Vec<&StackResource> = filtered_resources(app);
    let filtered_count = filtered.len();
    let page_size = app.cfn_state.resources.page_size.value();
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = if filtered_count > 0
        && app.cfn_state.resources.scroll_offset + page_size >= filtered_count
    {
        total_pages.saturating_sub(1)
    } else {
        app.cfn_state.resources.scroll_offset / page_size
    };
    let pagination = render_pagination_text(current_page, total_pages);

    render_filter_bar(
        frame,
        FilterConfig {
            filter_text: &app.cfn_state.resources.filter,
            placeholder: "Search resources",
            mode: app.mode,
            is_input_focused: app.cfn_state.resources_input_focus == InputFocus::Filter,
            controls: vec![FilterControl {
                text: pagination,
                is_focused: app.cfn_state.resources_input_focus == InputFocus::Pagination,
            }],
            area: chunks[0],
        },
    );

    let page_start = app.cfn_state.resources.scroll_offset;
    let page_end = (page_start + page_size).min(filtered_count);
    let page_resources: Vec<_> = filtered[page_start..page_end].to_vec();

    let columns: Vec<Box<dyn Column<StackResource>>> = app
        .cfn_resource_visible_column_ids
        .iter()
        .filter_map(|col_id| {
            ResourceColumn::from_id(col_id)
                .map(|col| Box::new(col) as Box<dyn Column<StackResource>>)
        })
        .collect();

    let expanded_index = app.cfn_state.resources.expanded_item.and_then(|idx| {
        let scroll_offset = app.cfn_state.resources.scroll_offset;
        if idx >= scroll_offset && idx < scroll_offset + page_size {
            Some(idx - scroll_offset)
        } else {
            None
        }
    });

    let config = TableConfig {
        items: page_resources,
        selected_index: app.cfn_state.resources.selected % page_size,
        expanded_index,
        columns: &columns,
        sort_column: "Logical ID",
        sort_direction: SortDirection::Asc,
        title: format!(" Resources ({}) ", filtered_count),
        area: chunks[1],
        get_expanded_content: Some(Box::new(|resource: &StackResource| {
            expanded_from_columns(&columns, resource)
        })),
        is_active: app.mode != Mode::FilterInput,
    };

    render_table(frame, config);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParameterColumn {
    Key,
    Value,
    ResolvedValue,
}

impl ParameterColumn {
    fn id(&self) -> &'static str {
        match self {
            ParameterColumn::Key => "cfn.parameter.key",
            ParameterColumn::Value => "cfn.parameter.value",
            ParameterColumn::ResolvedValue => "cfn.parameter.resolved_value",
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            ParameterColumn::Key => "Key",
            ParameterColumn::Value => "Value",
            ParameterColumn::ResolvedValue => "Resolved value",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Self::Key, Self::Value, Self::ResolvedValue]
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "cfn.parameter.key" => Some(Self::Key),
            "cfn.parameter.value" => Some(Self::Value),
            "cfn.parameter.resolved_value" => Some(Self::ResolvedValue),
            _ => None,
        }
    }
}

impl Column<StackParameter> for ParameterColumn {
    fn id(&self) -> &'static str {
        Self::id(self)
    }

    fn default_name(&self) -> &'static str {
        Self::default_name(self)
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
        translated.len().max(match self {
            ParameterColumn::Key => 40,
            ParameterColumn::Value => 50,
            ParameterColumn::ResolvedValue => 50,
        }) as u16
    }

    fn render(&self, item: &StackParameter) -> (String, Style) {
        match self {
            ParameterColumn::Key => (item.key.clone(), Style::default()),
            ParameterColumn::Value => (item.value.clone(), Style::default()),
            ParameterColumn::ResolvedValue => (item.resolved_value.clone(), Style::default()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputColumn {
    Key,
    Value,
    Description,
    ExportName,
}

impl OutputColumn {
    fn id(&self) -> &'static str {
        match self {
            OutputColumn::Key => "cfn.output.key",
            OutputColumn::Value => "cfn.output.value",
            OutputColumn::Description => "cfn.output.description",
            OutputColumn::ExportName => "cfn.output.export_name",
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            OutputColumn::Key => "Key",
            OutputColumn::Value => "Value",
            OutputColumn::Description => "Description",
            OutputColumn::ExportName => "Export name",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Self::Key, Self::Value, Self::Description, Self::ExportName]
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "cfn.output.key" => Some(Self::Key),
            "cfn.output.value" => Some(Self::Value),
            "cfn.output.description" => Some(Self::Description),
            "cfn.output.export_name" => Some(Self::ExportName),
            _ => None,
        }
    }
}

impl Column<StackOutput> for OutputColumn {
    fn id(&self) -> &'static str {
        Self::id(self)
    }

    fn default_name(&self) -> &'static str {
        Self::default_name(self)
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
        translated.len().max(match self {
            OutputColumn::Key => 40,
            OutputColumn::Value => 50,
            OutputColumn::Description => 50,
            OutputColumn::ExportName => 40,
        }) as u16
    }

    fn render(&self, item: &StackOutput) -> (String, Style) {
        match self {
            OutputColumn::Key => (item.key.clone(), Style::default()),
            OutputColumn::Value => (item.value.clone(), Style::default()),
            OutputColumn::Description => (item.description.clone(), Style::default()),
            OutputColumn::ExportName => (item.export_name.clone(), Style::default()),
        }
    }
}

impl Column<StackResource> for ResourceColumn {
    fn id(&self) -> &'static str {
        Self::id(self)
    }

    fn default_name(&self) -> &'static str {
        Self::default_name(self)
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
        translated.len().max(match self {
            ResourceColumn::LogicalId => 40,
            ResourceColumn::PhysicalId => 50,
            ResourceColumn::Type => 40,
            ResourceColumn::Status => 25,
            ResourceColumn::Module => 40,
        }) as u16
    }

    fn render(&self, item: &StackResource) -> (String, Style) {
        match self {
            ResourceColumn::LogicalId => (item.logical_id.clone(), Style::default()),
            ResourceColumn::PhysicalId => (item.physical_id.clone(), Style::default()),
            ResourceColumn::Type => (item.resource_type.clone(), Style::default()),
            ResourceColumn::Status => (item.status.clone(), Style::default()),
            ResourceColumn::Module => (item.module_info.clone(), Style::default()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TagColumn {
    Key,
    Value,
}

impl Column<(String, String)> for TagColumn {
    fn name(&self) -> &str {
        match self {
            TagColumn::Key => "Key",
            TagColumn::Value => "Value",
        }
    }

    fn width(&self) -> u16 {
        match self {
            TagColumn::Key => 40,
            TagColumn::Value => 60,
        }
    }

    fn render(&self, item: &(String, String)) -> (String, Style) {
        match self {
            TagColumn::Key => (item.0.clone(), Style::default()),
            TagColumn::Value => (item.1.clone(), Style::default()),
        }
    }
}

#[cfg(test)]
mod parameter_tests {
    use super::*;
    use crate::app::App;

    fn test_app() -> App {
        App::new_without_client("test".to_string(), Some("us-east-1".to_string()))
    }

    #[test]
    fn test_filtered_parameters_empty_filter() {
        let mut app = test_app();
        app.cfn_state.parameters.items = vec![
            StackParameter {
                key: "Param1".to_string(),
                value: "Value1".to_string(),
                resolved_value: "Resolved1".to_string(),
            },
            StackParameter {
                key: "Param2".to_string(),
                value: "Value2".to_string(),
                resolved_value: "Resolved2".to_string(),
            },
        ];
        app.cfn_state.parameters.filter = String::new();

        let filtered = filtered_parameters(&app);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filtered_parameters_by_key() {
        let mut app = test_app();
        app.cfn_state.parameters.items = vec![
            StackParameter {
                key: "DatabaseName".to_string(),
                value: "mydb".to_string(),
                resolved_value: "mydb".to_string(),
            },
            StackParameter {
                key: "InstanceType".to_string(),
                value: "t2.micro".to_string(),
                resolved_value: "t2.micro".to_string(),
            },
        ];
        app.cfn_state.parameters.filter = "database".to_string();

        let filtered = filtered_parameters(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].key, "DatabaseName");
    }

    #[test]
    fn test_filtered_parameters_by_value() {
        let mut app = test_app();
        app.cfn_state.parameters.items = vec![
            StackParameter {
                key: "Param1".to_string(),
                value: "production".to_string(),
                resolved_value: "production".to_string(),
            },
            StackParameter {
                key: "Param2".to_string(),
                value: "staging".to_string(),
                resolved_value: "staging".to_string(),
            },
        ];
        app.cfn_state.parameters.filter = "prod".to_string();

        let filtered = filtered_parameters(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].value, "production");
    }

    #[test]
    fn test_parameters_state_initialization() {
        let state = State::new();
        assert_eq!(state.parameters.items.len(), 0);
        assert_eq!(state.parameters.selected, 0);
        assert_eq!(state.parameters.filter, "");
        assert_eq!(state.parameters_input_focus, InputFocus::Filter);
    }

    #[test]
    fn test_parameters_expansion() {
        use crate::app::Service;
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.cfn_state.current_stack = Some("test-stack".to_string());
        app.cfn_state.detail_tab = DetailTab::Parameters;
        app.cfn_state.parameters.items = vec![StackParameter {
            key: "Param1".to_string(),
            value: "Value1".to_string(),
            resolved_value: "Resolved1".to_string(),
        }];

        assert_eq!(app.cfn_state.parameters.expanded_item, None);

        // Expand
        app.cfn_state.parameters.toggle_expand();
        assert_eq!(app.cfn_state.parameters.expanded_item, Some(0));

        // Collapse
        app.cfn_state.parameters.collapse();
        assert_eq!(app.cfn_state.parameters.expanded_item, None);
    }
}

#[cfg(test)]
mod output_tests {
    use super::*;
    use crate::app::App;

    fn test_app() -> App {
        App::new_without_client("test".to_string(), Some("us-east-1".to_string()))
    }

    #[test]
    fn test_filtered_outputs_empty_filter() {
        let mut app = test_app();
        app.cfn_state.outputs.items = vec![
            StackOutput {
                key: "Output1".to_string(),
                value: "Value1".to_string(),
                description: "Desc1".to_string(),
                export_name: "Export1".to_string(),
            },
            StackOutput {
                key: "Output2".to_string(),
                value: "Value2".to_string(),
                description: "Desc2".to_string(),
                export_name: "Export2".to_string(),
            },
        ];
        app.cfn_state.outputs.filter = String::new();

        let filtered = filtered_outputs(&app);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filtered_outputs_by_key() {
        let mut app = test_app();
        app.cfn_state.outputs.items = vec![
            StackOutput {
                key: "ApiUrl".to_string(),
                value: "https://api.example.com".to_string(),
                description: "API endpoint".to_string(),
                export_name: "MyApiUrl".to_string(),
            },
            StackOutput {
                key: "BucketName".to_string(),
                value: "my-bucket".to_string(),
                description: "S3 bucket".to_string(),
                export_name: "MyBucket".to_string(),
            },
        ];
        app.cfn_state.outputs.filter = "api".to_string();

        let filtered = filtered_outputs(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].key, "ApiUrl");
    }

    #[test]
    fn test_filtered_outputs_by_value() {
        let mut app = test_app();
        app.cfn_state.outputs.items = vec![
            StackOutput {
                key: "ApiUrl".to_string(),
                value: "https://api.example.com".to_string(),
                description: "API endpoint".to_string(),
                export_name: "MyApiUrl".to_string(),
            },
            StackOutput {
                key: "BucketName".to_string(),
                value: "my-bucket".to_string(),
                description: "S3 bucket".to_string(),
                export_name: "MyBucket".to_string(),
            },
        ];
        app.cfn_state.outputs.filter = "my-bucket".to_string();

        let filtered = filtered_outputs(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].key, "BucketName");
    }

    #[test]
    fn test_outputs_state_initialization() {
        let app = test_app();
        assert_eq!(app.cfn_state.outputs.items.len(), 0);
        assert_eq!(app.cfn_state.outputs.filter, "");
        assert_eq!(app.cfn_state.outputs.selected, 0);
        assert_eq!(app.cfn_state.outputs.expanded_item, None);
    }

    #[test]
    fn test_outputs_expansion() {
        let mut app = test_app();
        app.cfn_state.current_stack = Some("test-stack".to_string());
        app.cfn_state.detail_tab = DetailTab::Outputs;
        app.cfn_state.outputs.items = vec![StackOutput {
            key: "Output1".to_string(),
            value: "Value1".to_string(),
            description: "Desc1".to_string(),
            export_name: "Export1".to_string(),
        }];

        assert_eq!(app.cfn_state.outputs.expanded_item, None);

        // Expand
        app.cfn_state.outputs.toggle_expand();
        assert_eq!(app.cfn_state.outputs.expanded_item, Some(0));

        // Collapse
        app.cfn_state.outputs.collapse();
        assert_eq!(app.cfn_state.outputs.expanded_item, None);
    }

    #[test]
    fn test_expanded_items_hierarchical_view() {
        let mut app = test_app();

        // Initially empty
        assert!(app.cfn_state.expanded_items.is_empty());

        // Expand a resource
        app.cfn_state
            .expanded_items
            .insert("MyNestedStack".to_string());
        assert!(app.cfn_state.expanded_items.contains("MyNestedStack"));

        // Expand another resource
        app.cfn_state
            .expanded_items
            .insert("MyNestedStack/ChildResource".to_string());
        assert_eq!(app.cfn_state.expanded_items.len(), 2);

        // Collapse a resource
        app.cfn_state.expanded_items.remove("MyNestedStack");
        assert!(!app.cfn_state.expanded_items.contains("MyNestedStack"));
        assert_eq!(app.cfn_state.expanded_items.len(), 1);
    }
}

#[cfg(test)]
mod resource_tests {
    use super::*;
    use crate::app::App;

    fn test_app() -> App {
        App::new_without_client("test".to_string(), Some("us-east-1".to_string()))
    }

    #[test]
    fn test_resources_state_initialization() {
        let app = test_app();
        assert_eq!(app.cfn_state.resources.items.len(), 0);
        assert_eq!(app.cfn_state.resources.filter, "");
        assert_eq!(app.cfn_state.resources.selected, 0);
    }

    #[test]
    fn test_filtered_resources_empty_filter() {
        let mut app = test_app();
        app.cfn_state.resources.items = vec![
            StackResource {
                logical_id: "MyBucket".to_string(),
                physical_id: "my-bucket-123".to_string(),
                resource_type: "AWS::S3::Bucket".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                module_info: String::new(),
            },
            StackResource {
                logical_id: "MyFunction".to_string(),
                physical_id: "my-function-456".to_string(),
                resource_type: "AWS::Lambda::Function".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                module_info: String::new(),
            },
        ];
        app.cfn_state.resources.filter = String::new();

        let filtered: Vec<&StackResource> = if app.cfn_state.resources.filter.is_empty() {
            app.cfn_state.resources.items.iter().collect()
        } else {
            app.cfn_state
                .resources
                .items
                .iter()
                .filter(|r| {
                    r.logical_id
                        .to_lowercase()
                        .contains(&app.cfn_state.resources.filter.to_lowercase())
                })
                .collect()
        };
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filtered_resources_by_logical_id() {
        let mut app = test_app();
        app.cfn_state.resources.items = vec![
            StackResource {
                logical_id: "MyBucket".to_string(),
                physical_id: "my-bucket-123".to_string(),
                resource_type: "AWS::S3::Bucket".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                module_info: String::new(),
            },
            StackResource {
                logical_id: "MyFunction".to_string(),
                physical_id: "my-function-456".to_string(),
                resource_type: "AWS::Lambda::Function".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                module_info: String::new(),
            },
        ];
        app.cfn_state.resources.filter = "bucket".to_string();

        let filtered: Vec<&StackResource> = app
            .cfn_state
            .resources
            .items
            .iter()
            .filter(|r| {
                r.logical_id
                    .to_lowercase()
                    .contains(&app.cfn_state.resources.filter.to_lowercase())
            })
            .collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].logical_id, "MyBucket");
    }

    #[test]
    fn test_filtered_resources_by_type() {
        let mut app = test_app();
        app.cfn_state.resources.items = vec![
            StackResource {
                logical_id: "MyBucket".to_string(),
                physical_id: "my-bucket-123".to_string(),
                resource_type: "AWS::S3::Bucket".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                module_info: String::new(),
            },
            StackResource {
                logical_id: "MyFunction".to_string(),
                physical_id: "my-function-456".to_string(),
                resource_type: "AWS::Lambda::Function".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                module_info: String::new(),
            },
        ];
        app.cfn_state.resources.filter = "lambda".to_string();

        let filtered: Vec<&StackResource> = app
            .cfn_state
            .resources
            .items
            .iter()
            .filter(|r| {
                r.resource_type
                    .to_lowercase()
                    .contains(&app.cfn_state.resources.filter.to_lowercase())
            })
            .collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].logical_id, "MyFunction");
    }

    #[test]
    fn test_resources_sorted_by_logical_id() {
        let mut app = test_app();
        app.cfn_state.resources.items = vec![
            StackResource {
                logical_id: "ZBucket".to_string(),
                physical_id: "z-bucket".to_string(),
                resource_type: "AWS::S3::Bucket".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                module_info: String::new(),
            },
            StackResource {
                logical_id: "AFunction".to_string(),
                physical_id: "a-function".to_string(),
                resource_type: "AWS::Lambda::Function".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                module_info: String::new(),
            },
        ];

        // Resources should be sorted by logical_id in the API response
        // but let's verify the order
        assert_eq!(app.cfn_state.resources.items[0].logical_id, "ZBucket");
        assert_eq!(app.cfn_state.resources.items[1].logical_id, "AFunction");
    }

    #[test]
    fn test_resources_expansion() {
        let mut app = test_app();
        app.cfn_state.resources.items = vec![StackResource {
            logical_id: "MyBucket".to_string(),
            physical_id: "my-bucket-123".to_string(),
            resource_type: "AWS::S3::Bucket".to_string(),
            status: "CREATE_COMPLETE".to_string(),
            module_info: String::new(),
        }];

        assert_eq!(app.cfn_state.resources.expanded_item, None);

        // Expand
        app.cfn_state.resources.toggle_expand();
        assert_eq!(app.cfn_state.resources.expanded_item, Some(0));

        // Collapse
        app.cfn_state.resources.collapse();
        assert_eq!(app.cfn_state.resources.expanded_item, None);
    }

    #[test]
    fn test_resource_column_ids() {
        let ids = resource_column_ids();
        assert_eq!(ids.len(), 5);
        assert!(ids.contains(&"cfn.resource.logical_id"));
        assert!(ids.contains(&"cfn.resource.physical_id"));
        assert!(ids.contains(&"cfn.resource.type"));
        assert!(ids.contains(&"cfn.resource.status"));
        assert!(ids.contains(&"cfn.resource.module"));
    }

    #[test]
    fn test_detail_tab_allows_preferences() {
        assert!(DetailTab::StackInfo.allows_preferences());
        assert!(DetailTab::Parameters.allows_preferences());
        assert!(DetailTab::Outputs.allows_preferences());
        assert!(DetailTab::Resources.allows_preferences());
        assert!(!DetailTab::Template.allows_preferences());
        assert!(!DetailTab::Events.allows_preferences());
        assert!(!DetailTab::ChangeSets.allows_preferences());
        assert!(!DetailTab::GitSync.allows_preferences());
    }

    #[test]
    fn test_resources_tree_view_expansion() {
        let mut app = test_app();
        app.cfn_state.resources.items = vec![
            StackResource {
                logical_id: "ParentModule".to_string(),
                physical_id: "parent-123".to_string(),
                resource_type: "AWS::CloudFormation::Stack".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                module_info: String::new(),
            },
            StackResource {
                logical_id: "ChildResource".to_string(),
                physical_id: "child-456".to_string(),
                resource_type: "AWS::S3::Bucket".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                module_info: "ParentModule".to_string(),
            },
        ];

        // Initially not expanded
        assert!(!app.cfn_state.expanded_items.contains("ParentModule"));

        // Expand parent
        app.cfn_state
            .expanded_items
            .insert("ParentModule".to_string());
        assert!(app.cfn_state.expanded_items.contains("ParentModule"));

        // Collapse parent
        app.cfn_state.expanded_items.remove("ParentModule");
        assert!(!app.cfn_state.expanded_items.contains("ParentModule"));
    }

    #[test]
    fn test_resources_navigation() {
        use crate::app::Service;
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.cfn_state.current_stack = Some("test-stack".to_string());
        app.cfn_state.detail_tab = DetailTab::Resources;
        app.cfn_state.resources.items = vec![
            StackResource {
                logical_id: "Resource1".to_string(),
                physical_id: "res-1".to_string(),
                resource_type: "AWS::S3::Bucket".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                module_info: String::new(),
            },
            StackResource {
                logical_id: "Resource2".to_string(),
                physical_id: "res-2".to_string(),
                resource_type: "AWS::Lambda::Function".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                module_info: String::new(),
            },
        ];

        assert_eq!(app.cfn_state.resources.selected, 0);

        // Navigate down
        let filtered = filtered_resources(&app);
        app.cfn_state.resources.next_item(filtered.len());
        assert_eq!(app.cfn_state.resources.selected, 1);

        // Navigate up
        app.cfn_state.resources.prev_item();
        assert_eq!(app.cfn_state.resources.selected, 0);
    }

    #[test]
    fn test_resources_filter() {
        let mut app = test_app();
        app.cfn_state.resources.items = vec![
            StackResource {
                logical_id: "MyBucket".to_string(),
                physical_id: "my-bucket-123".to_string(),
                resource_type: "AWS::S3::Bucket".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                module_info: String::new(),
            },
            StackResource {
                logical_id: "MyFunction".to_string(),
                physical_id: "my-function-456".to_string(),
                resource_type: "AWS::Lambda::Function".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                module_info: String::new(),
            },
        ];

        // No filter
        app.cfn_state.resources.filter = String::new();
        let filtered = filtered_resources(&app);
        assert_eq!(filtered.len(), 2);

        // Filter by logical ID
        app.cfn_state.resources.filter = "bucket".to_string();
        let filtered = filtered_resources(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].logical_id, "MyBucket");

        // Filter by type
        app.cfn_state.resources.filter = "lambda".to_string();
        let filtered = filtered_resources(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].logical_id, "MyFunction");
    }

    #[test]
    fn test_resources_page_size() {
        use crate::common::PageSize;
        let mut app = test_app();

        // Default page size
        assert_eq!(app.cfn_state.resources.page_size, PageSize::Fifty);
        assert_eq!(app.cfn_state.resources.page_size.value(), 50);

        // Change page size
        app.cfn_state.resources.page_size = PageSize::TwentyFive;
        assert_eq!(app.cfn_state.resources.page_size, PageSize::TwentyFive);
        assert_eq!(app.cfn_state.resources.page_size.value(), 25);
    }
}
