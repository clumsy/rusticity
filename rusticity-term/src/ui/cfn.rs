use crate::app::App;
use crate::cfn::{Column as CfnColumn, Stack as CfnStack};
use crate::common::CyclicEnum;
use crate::common::{render_pagination_text, InputFocus, SortDirection};
use crate::keymap::Mode;
use crate::table::TableState;
use crate::ui::labeled_field;
use ratatui::{prelude::*, widgets::*};

pub const STATUS_FILTER: InputFocus = InputFocus::Dropdown("StatusFilter");
pub const VIEW_NESTED: InputFocus = InputFocus::Checkbox("ViewNested");

impl State {
    pub const FILTER_CONTROLS: [InputFocus; 4] = [
        InputFocus::Filter,
        STATUS_FILTER,
        VIEW_NESTED,
        InputFocus::Pagination,
    ];
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

    pub fn next(&self) -> Self {
        match self {
            StatusFilter::All => StatusFilter::Active,
            StatusFilter::Active => StatusFilter::Complete,
            StatusFilter::Complete => StatusFilter::Failed,
            StatusFilter::Failed => StatusFilter::Deleted,
            StatusFilter::Deleted => StatusFilter::InProgress,
            StatusFilter::InProgress => StatusFilter::All,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            StatusFilter::All => StatusFilter::InProgress,
            StatusFilter::Active => StatusFilter::All,
            StatusFilter::Complete => StatusFilter::Active,
            StatusFilter::Failed => StatusFilter::Complete,
            StatusFilter::Deleted => StatusFilter::Failed,
            StatusFilter::InProgress => StatusFilter::Deleted,
        }
    }

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
        // Self::Events,
        // Self::Resources,
        // Self::Outputs,
        // Self::Parameters,
        // Self::Template,
        // Self::ChangeSets,
        // Self::GitSync,
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
}

pub fn filtered_cloudformation_stacks(app: &App) -> Vec<&crate::cfn::Stack> {
    let filtered: Vec<&crate::cfn::Stack> = if app.cfn_state.table.filter.is_empty() {
        app.cfn_state.table.items.iter().collect()
    } else {
        app.cfn_state
            .table
            .items
            .iter()
            .filter(|s| {
                s.name
                    .to_lowercase()
                    .contains(&app.cfn_state.table.filter.to_lowercase())
                    || s.description
                        .to_lowercase()
                        .contains(&app.cfn_state.table.filter.to_lowercase())
            })
            .collect()
    };

    filtered
        .into_iter()
        .filter(|s| app.cfn_state.status_filter.matches(&s.status))
        .collect()
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

    crate::ui::filter::render_filter_bar(
        frame,
        crate::ui::filter::FilterConfig {
            filter_text: &app.cfn_state.table.filter,
            placeholder,
            mode: app.mode,
            is_input_focused: app.cfn_state.input_focus == InputFocus::Filter,
            controls: vec![
                crate::ui::filter::FilterControl {
                    text: status_filter_text.to_string(),
                    is_focused: app.cfn_state.input_focus == STATUS_FILTER,
                },
                crate::ui::filter::FilterControl {
                    text: view_nested_text.to_string(),
                    is_focused: app.cfn_state.input_focus == VIEW_NESTED,
                },
                crate::ui::filter::FilterControl {
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
    let columns: Vec<Box<dyn crate::ui::table::Column<&crate::cfn::Stack>>> = app
        .visible_cfn_columns
        .iter()
        .map(|col| col.to_column())
        .collect();

    let expanded_index = app.cfn_state.table.expanded_item.and_then(|idx| {
        let scroll_offset = app.cfn_state.table.scroll_offset;
        if idx >= scroll_offset && idx < scroll_offset + page_size {
            Some(idx - scroll_offset)
        } else {
            None
        }
    });

    let config = crate::ui::table::TableConfig {
        items: page_stacks,
        selected_index: app.cfn_state.table.selected % app.cfn_state.table.page_size.value(),
        expanded_index,
        columns: &columns,
        sort_column: app.cfn_state.sort_column.name(),
        sort_direction: app.cfn_state.sort_direction,
        title: format!(" Stacks ({}) ", filtered_count),
        area: chunks[1],
        get_expanded_content: Some(Box::new(|stack: &&crate::cfn::Stack| {
            crate::ui::table::expanded_from_columns(&columns, stack)
        })),
        is_active: app.mode != Mode::FilterInput,
    };

    crate::ui::table::render_table(frame, config);

    // Render dropdown for StatusFilter when focused (after table so it appears on top)
    if app.mode == Mode::FilterInput && app.cfn_state.input_focus == STATUS_FILTER {
        // Find the longest filter name for consistent width
        let max_filter_width = StatusFilter::all()
            .iter()
            .map(|f| f.name().len())
            .max()
            .unwrap_or(10) as u16
            + 4; // +4 for padding and borders

        let dropdown_items: Vec<ListItem> = StatusFilter::all()
            .iter()
            .map(|filter| {
                let style = if *filter == app.cfn_state.status_filter {
                    Style::default().fg(Color::Yellow).bold()
                } else {
                    Style::default()
                };
                ListItem::new(format!(" {} ", filter.name())).style(style)
            })
            .collect();

        let dropdown_height = dropdown_items.len() as u16 + 2;

        // Calculate position based on actual control positions
        let view_nested_width = " ☑ View nested ".len() as u16;
        let pagination_width = pagination.len() as u16;

        let dropdown_width = max_filter_width;
        let dropdown_x = chunks[0]
            .x
            .saturating_add(chunks[0].width)
            .saturating_sub(view_nested_width + 3 + pagination_width + 3 + dropdown_width);

        let dropdown_area = Rect {
            x: dropdown_x,
            y: chunks[0].y + chunks[0].height,
            width: dropdown_width,
            height: dropdown_height.min(10),
        };

        frame.render_widget(
            List::new(dropdown_items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow)),
                )
                .style(Style::default().bg(Color::Black)),
            dropdown_area,
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
        let paragraph = Paragraph::new("Stack not found")
            .block(Block::default().borders(Borders::ALL).title(" Error "));
        frame.render_widget(paragraph, area);
        return;
    }

    let stack = stack.unwrap();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Stack name
            Constraint::Min(0),    // Content
        ])
        .split(area);

    // Render stack name
    frame.render_widget(Paragraph::new(stack.name.clone()), chunks[0]);

    // Render content based on selected tab
    match app.cfn_state.detail_tab {
        DetailTab::StackInfo => {
            render_stack_info(frame, app, stack, chunks[1]);
        }
        _ => unimplemented!(),
    }
}

pub fn render_stack_info(frame: &mut Frame, _app: &App, stack: &crate::cfn::Stack, area: Rect) {
    let (formatted_status, _status_color) = crate::cfn::format_status(&stack.status);

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
    let overview_height = fields.len() as u16 + 2; // +2 for borders

    // Tags section
    let tags_lines = if stack.tags.is_empty() {
        vec![
            "Stack-level tags will apply to all supported resources in your stack.".to_string(),
            "You can add up to 50 unique tags for each stack.".to_string(),
            String::new(),
            "No tags defined".to_string(),
        ]
    } else {
        let mut lines = vec!["Key                          Value".to_string()];
        for (key, value) in &stack.tags {
            lines.push(format!("{}  {}", key, value));
        }
        lines
    };
    let tags_height = tags_lines.len() as u16 + 2; // +2 for borders

    // Stack policy section
    let policy_lines = if stack.stack_policy.is_empty() {
        vec![
            "Defines the resources that you want to protect from unintentional".to_string(),
            "updates during a stack update.".to_string(),
            String::new(),
            "No stack policy".to_string(),
            "  There is no stack policy defined".to_string(),
        ]
    } else {
        vec![stack.stack_policy.clone()]
    };
    let policy_height = policy_lines.len() as u16 + 2; // +2 for borders

    // Rollback configuration section
    let rollback_lines = if stack.rollback_alarms.is_empty() {
        vec![
            "Specifies alarms for CloudFormation to monitor when creating and".to_string(),
            "updating the stack. If the operation breaches an alarm threshold,".to_string(),
            "CloudFormation rolls it back.".to_string(),
            String::new(),
            "Monitoring time".to_string(),
            format!(
                "  {}",
                if stack.rollback_monitoring_time.is_empty() {
                    "-"
                } else {
                    &stack.rollback_monitoring_time
                }
            ),
        ]
    } else {
        let mut lines = vec![
            "Monitoring time".to_string(),
            format!(
                "  {}",
                if stack.rollback_monitoring_time.is_empty() {
                    "-"
                } else {
                    &stack.rollback_monitoring_time
                }
            ),
            String::new(),
            "CloudWatch alarm ARN".to_string(),
        ];
        for alarm in &stack.rollback_alarms {
            lines.push(format!("  {}", alarm));
        }
        lines
    };
    let rollback_height = rollback_lines.len() as u16 + 2; // +2 for borders

    // Notification options section
    let notification_lines = if stack.notification_arns.is_empty() {
        vec![
            "Specifies where notifications about stack actions will be sent.".to_string(),
            String::new(),
            "SNS topic ARN".to_string(),
            "  No notifications configured".to_string(),
        ]
    } else {
        let mut lines = vec![
            "Specifies where notifications about stack actions will be sent.".to_string(),
            String::new(),
            "SNS topic ARN".to_string(),
        ];
        for arn in &stack.notification_arns {
            lines.push(format!("  {}", arn));
        }
        lines
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
    let overview = Paragraph::new(overview_lines)
        .block(Block::default().borders(Borders::ALL).title(" Overview "))
        .wrap(Wrap { trim: true });
    frame.render_widget(overview, sections[0]);

    // Render tags
    let tags = Paragraph::new(tags_lines.join("\n"))
        .block(Block::default().borders(Borders::ALL).title(" Tags "))
        .wrap(Wrap { trim: true });
    frame.render_widget(tags, sections[1]);

    // Render stack policy
    let policy = Paragraph::new(policy_lines.join("\n"))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Stack policy "),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(policy, sections[2]);

    // Render rollback configuration
    let rollback = Paragraph::new(rollback_lines.join("\n"))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Rollback configuration "),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(rollback, sections[3]);

    // Render notification options
    let notifications = Paragraph::new(notification_lines.join("\n"))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Notification options "),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(notifications, sections[4]);
}
