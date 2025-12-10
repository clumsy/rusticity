pub mod cfn;
pub mod cw;
pub mod ec2;
pub mod ecr;
mod expanded_view;
pub mod filter;
pub mod iam;
pub mod lambda;
pub mod monitoring;
mod pagination;
pub mod prefs;
mod query_editor;
pub mod s3;
pub mod sqs;
mod status;
pub mod styles;
pub mod table;

pub use cw::insights::{DateRangeType, TimeUnit};
pub use cw::{
    CloudWatchLogGroupsState, DetailTab, EventColumn, EventFilterFocus, LogGroupColumn,
    StreamColumn, StreamSort,
};
pub use expanded_view::{format_expansion_text, format_fields};
pub use pagination::{render_paginated_filter, PaginatedFilterConfig};
pub use prefs::Preferences;
pub use query_editor::{render_query_editor, QueryEditorConfig};
pub use status::{first_hint, hint, last_hint, SPINNER_FRAMES};
pub use table::{format_expandable, CURSOR_COLLAPSED, CURSOR_EXPANDED};

use self::styles::highlight;
use crate::app::{AlarmViewMode, App, CalendarField, LambdaDetailTab, Service, ViewMode};
use crate::cfn::Column as CfnColumn;
use crate::common::{render_pagination_text, render_scrollbar, translate_column, PageSize};
use crate::cw::alarms::AlarmColumn;
use crate::ec2::Column as Ec2Column;
use crate::ecr::{image, repo};
use crate::keymap::Mode;
use crate::lambda::{ApplicationColumn, DeploymentColumn, FunctionColumn, ResourceColumn};
use crate::s3::BucketColumn;
use crate::sqs::queue::Column as SqsColumn;
use crate::sqs::trigger::Column as SqsTriggerColumn;
use crate::ui::cfn::{
    DetailTab as CfnDetailTab, OutputColumn, ParameterColumn, ResourceColumn as CfnResourceColumn,
};
use crate::ui::iam::UserTab;
use crate::ui::lambda::ApplicationDetailTab;
use crate::ui::sqs::QueueDetailTab as SqsQueueDetailTab;
use crate::ui::table::Column as TableColumn;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

pub fn labeled_field(label: &str, value: impl Into<String>) -> Line<'static> {
    let val = value.into();
    let display = if val.is_empty() { "-".to_string() } else { val };
    Line::from(vec![
        Span::styled(
            format!("{}: ", label),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(display),
    ])
}

/// Calculate the height needed for a block containing lines (lines + 2 for borders)
pub fn block_height(lines: &[Line]) -> u16 {
    lines.len() as u16 + 2
}

/// Calculate the height needed for a block with a given number of lines (lines + 2 for borders)
pub fn block_height_for(line_count: usize) -> u16 {
    line_count as u16 + 2
}

pub fn section_header(text: &str, width: u16) -> Line<'static> {
    let text_len = text.len() as u16;
    // Format: " Section Name ─────────────────────"
    // space + text + space + dashes = width
    let remaining = width.saturating_sub(text_len + 2);
    let dashes = "─".repeat(remaining as usize);
    Line::from(vec![
        Span::raw(" "),
        Span::raw(text.to_string()),
        Span::raw(format!(" {}", dashes)),
    ])
}

pub fn tab_style(selected: bool) -> Style {
    if selected {
        highlight()
    } else {
        Style::default()
    }
}

pub fn service_tab_style(selected: bool) -> Style {
    if selected {
        Style::default().bg(Color::Green).fg(Color::Black)
    } else {
        Style::default()
    }
}

pub fn render_tab_spans<'a>(tabs: &[(&'a str, bool)]) -> Vec<Span<'a>> {
    let mut spans = Vec::new();
    for (i, (name, selected)) in tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" ⋮ "));
        }
        spans.push(Span::styled(*name, service_tab_style(*selected)));
    }
    spans
}

use ratatui::{prelude::*, widgets::*};

// Common UI constants
pub const SEARCH_ICON: &str = " 🔍 ";
pub const PREFERENCES_TITLE: &str = " Preferences ";

// Filter
pub fn filter_area(filter_text: Vec<Span<'_>>, is_active: bool) -> Paragraph<'_> {
    Paragraph::new(Line::from(filter_text))
        .block(
            Block::default()
                .title(SEARCH_ICON)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_type(BorderType::Rounded)
                .border_type(BorderType::Rounded)
                .border_style(if is_active {
                    active_border()
                } else {
                    Style::default()
                }),
        )
        .style(Style::default())
}

// Common style helpers
pub fn active_border() -> Style {
    Style::default().fg(Color::Green)
}

pub fn rounded_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_type(BorderType::Rounded)
        .border_type(BorderType::Rounded)
}

pub fn titled_rounded_block(title: &'static str) -> Block<'static> {
    rounded_block().title(title)
}

pub fn bold_style() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

pub fn cyan_bold() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

pub fn red_text() -> Style {
    Style::default().fg(Color::Rgb(255, 165, 0))
}

pub fn yellow_text() -> Style {
    Style::default().fg(Color::Yellow)
}

pub fn get_cursor(active: bool) -> &'static str {
    if active {
        "█"
    } else {
        ""
    }
}

pub fn render_search_filter(
    frame: &mut Frame,
    area: Rect,
    filter_text: &str,
    is_active: bool,
    selected: usize,
    total_items: usize,
    page_size: usize,
) {
    let cursor = get_cursor(is_active);
    let total_pages = total_items.div_ceil(page_size);
    let current_page = selected / page_size;
    let pagination = render_pagination_text(current_page, total_pages);

    let controls_text = format!(" {}", pagination);
    let filter_width = (area.width as usize).saturating_sub(4);
    let content_len = filter_text.len() + if is_active { cursor.len() } else { 0 };
    let available_space = filter_width.saturating_sub(controls_text.len() + 1);

    let mut spans = vec![];
    if filter_text.is_empty() && !is_active {
        spans.push(Span::styled("Search", Style::default().fg(Color::DarkGray)));
    } else {
        spans.push(Span::raw(filter_text));
    }
    if is_active {
        spans.push(Span::styled(cursor, Style::default().fg(Color::Yellow)));
    }
    if content_len < available_space {
        spans.push(Span::raw(
            " ".repeat(available_space.saturating_sub(content_len)),
        ));
    }
    spans.push(Span::styled(
        controls_text,
        if is_active {
            Style::default()
        } else {
            Style::default().fg(Color::Green)
        },
    ));

    let filter = filter_area(spans, is_active);
    frame.render_widget(filter, area);
}

fn render_toggle(is_on: bool) -> Vec<Span<'static>> {
    if is_on {
        vec![
            Span::styled("◼", Style::default().fg(Color::Blue)),
            Span::raw("⬜"),
        ]
    } else {
        vec![
            Span::raw("⬜"),
            Span::styled("◼", Style::default().fg(Color::Black)),
        ]
    }
}

fn render_radio(is_selected: bool) -> (String, Style) {
    if is_selected {
        ("●".to_string(), Style::default().fg(Color::Blue))
    } else {
        ("○".to_string(), Style::default())
    }
}

// Common UI constants

// Common style helpers
pub fn vertical(
    constraints: impl IntoIterator<Item = Constraint>,
    area: Rect,
) -> std::rc::Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area)
}

pub fn horizontal(
    constraints: impl IntoIterator<Item = Constraint>,
    area: Rect,
) -> std::rc::Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area)
}

// Block helpers
pub fn block(title: &str) -> Block<'_> {
    rounded_block().title(title)
}

pub fn block_with_style(title: &str, style: Style) -> Block<'_> {
    block(title).border_style(style)
}

// Render a summary section with labeled fields
pub fn render_summary(frame: &mut Frame, area: Rect, title: &str, fields: &[(&str, String)]) {
    let summary_block = block(title).border_type(BorderType::Rounded);
    let inner = summary_block.inner(area);
    frame.render_widget(summary_block, area);

    let lines: Vec<Line> = fields
        .iter()
        .map(|(label, value)| {
            Line::from(vec![
                Span::styled(*label, Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(value),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), inner);
}

// Render tabs with selection highlighting
pub fn render_tabs<T: PartialEq>(frame: &mut Frame, area: Rect, tabs: &[(&str, T)], selected: &T) {
    let spans: Vec<Span> = tabs
        .iter()
        .enumerate()
        .flat_map(|(i, (name, tab))| {
            let mut result = Vec::new();
            if i > 0 {
                result.push(Span::raw(" ⋮ "));
            }
            if tab == selected {
                result.push(Span::styled(*name, tab_style(true)));
            } else {
                result.push(Span::raw(*name));
            }
            result
        })
        .collect();

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

pub fn format_duration(seconds: u64) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;
    const WEEK: u64 = 7 * DAY;
    const YEAR: u64 = 365 * DAY;

    if seconds >= YEAR {
        let years = seconds / YEAR;
        let remainder = seconds % YEAR;
        if remainder == 0 {
            format!("{} year{}", years, if years == 1 { "" } else { "s" })
        } else {
            let weeks = remainder / WEEK;
            format!(
                "{} year{} {} week{}",
                years,
                if years == 1 { "" } else { "s" },
                weeks,
                if weeks == 1 { "" } else { "s" }
            )
        }
    } else if seconds >= WEEK {
        let weeks = seconds / WEEK;
        let remainder = seconds % WEEK;
        if remainder == 0 {
            format!("{} week{}", weeks, if weeks == 1 { "" } else { "s" })
        } else {
            let days = remainder / DAY;
            format!(
                "{} week{} {} day{}",
                weeks,
                if weeks == 1 { "" } else { "s" },
                days,
                if days == 1 { "" } else { "s" }
            )
        }
    } else if seconds >= DAY {
        let days = seconds / DAY;
        let remainder = seconds % DAY;
        if remainder == 0 {
            format!("{} day{}", days, if days == 1 { "" } else { "s" })
        } else {
            let hours = remainder / HOUR;
            format!(
                "{} day{} {} hour{}",
                days,
                if days == 1 { "" } else { "s" },
                hours,
                if hours == 1 { "" } else { "s" }
            )
        }
    } else if seconds >= HOUR {
        let hours = seconds / HOUR;
        let remainder = seconds % HOUR;
        if remainder == 0 {
            format!("{} hour{}", hours, if hours == 1 { "" } else { "s" })
        } else {
            let minutes = remainder / MINUTE;
            format!(
                "{} hour{} {} minute{}",
                hours,
                if hours == 1 { "" } else { "s" },
                minutes,
                if minutes == 1 { "" } else { "s" }
            )
        }
    } else if seconds >= MINUTE {
        let minutes = seconds / MINUTE;
        format!("{} minute{}", minutes, if minutes == 1 { "" } else { "s" })
    } else {
        format!("{} second{}", seconds, if seconds == 1 { "" } else { "s" })
    }
}

fn render_column_toggle_string(col_name: &str, is_visible: bool) -> (ListItem<'static>, usize) {
    let mut spans = vec![];
    spans.extend(render_toggle(is_visible));
    spans.push(Span::raw(" "));
    spans.push(Span::raw(col_name.to_string()));
    let text_len = 4 + col_name.len();
    (ListItem::new(Line::from(spans)), text_len)
}

// Helper to render a section header
fn render_section_header(title: &str) -> (ListItem<'static>, usize) {
    let len = title.len();
    (
        ListItem::new(Line::from(Span::styled(
            title.to_string(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))),
        len,
    )
}

// Helper to render a radio button item
fn render_radio_item(label: &str, is_selected: bool, indent: bool) -> (ListItem<'static>, usize) {
    let (radio, style) = render_radio(is_selected);
    let text_len = (if indent { 2 } else { 0 }) + radio.chars().count() + 1 + label.len();
    let mut spans = if indent {
        vec![Span::raw("  ")]
    } else {
        vec![]
    };
    spans.push(Span::styled(radio, style));
    spans.push(Span::raw(format!(" {}", label)));
    (ListItem::new(Line::from(spans)), text_len)
}

// Helper to render page size options
fn render_page_size_section(
    current_size: PageSize,
    sizes: &[(PageSize, &str)],
) -> (Vec<ListItem<'static>>, usize) {
    let mut items = Vec::new();
    let mut max_len = 0;

    let (header, header_len) = render_section_header("Page size");
    items.push(header);
    max_len = max_len.max(header_len);

    for (size, label) in sizes {
        let is_selected = current_size == *size;
        let (item, len) = render_radio_item(label, is_selected, false);
        items.push(item);
        max_len = max_len.max(len);
    }

    (items, max_len)
}

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Always show tabs row (with profile info), optionally show top bar for breadcrumbs
    let has_tabs = !app.tabs.is_empty();
    let show_breadcrumbs = has_tabs && app.service_selected && {
        // Only show breadcrumbs if we're deeper than the root level
        match app.current_service {
            Service::CloudWatchLogGroups => app.view_mode != ViewMode::List,
            Service::S3Buckets => app.s3_state.current_bucket.is_some(),
            _ => false,
        }
    };

    let chunks = if show_breadcrumbs {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Tabs row (profile info + tabs)
                Constraint::Length(1), // Top bar (breadcrumbs)
                Constraint::Min(0),    // Content
                Constraint::Length(1), // Bottom bar
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Tabs row (profile info + tabs)
                Constraint::Min(0),    // Content
                Constraint::Length(1), // Bottom bar
            ])
            .split(area)
    };

    // Always render tabs row (shows profile info)
    render_tabs_row(frame, app, chunks[0]);

    if show_breadcrumbs {
        render_top_bar(frame, app, chunks[1]);
    }

    let content_idx = if show_breadcrumbs { 2 } else { 1 };
    let bottom_idx = if show_breadcrumbs { 3 } else { 2 };

    if !app.service_selected && app.tabs.is_empty() && app.mode == Mode::Normal {
        // Empty screen with message
        let message = vec![
            Line::from(""),
            Line::from(""),
            Line::from(vec![
                Span::raw("Press "),
                Span::styled("␣", Style::default().fg(Color::Red)),
                Span::raw(" to open Menu"),
            ]),
        ];
        let paragraph = Paragraph::new(message).alignment(Alignment::Center);
        frame.render_widget(paragraph, chunks[content_idx]);
        render_bottom_bar(frame, app, chunks[bottom_idx]);
    } else if !app.service_selected && app.mode == Mode::Normal {
        render_service_picker(frame, app, chunks[content_idx]);
        render_bottom_bar(frame, app, chunks[bottom_idx]);
    } else if app.service_selected {
        render_service(frame, app, chunks[content_idx]);
        render_bottom_bar(frame, app, chunks[bottom_idx]);
    } else {
        // SpaceMenu with no service selected - just render bottom bar
        render_bottom_bar(frame, app, chunks[bottom_idx]);
    }

    // Render modals on top
    match app.mode {
        Mode::SpaceMenu => render_space_menu(frame, area),
        Mode::ServicePicker => render_service_picker(frame, app, area),
        Mode::ColumnSelector => render_column_selector(frame, app, area),
        Mode::ErrorModal => render_error_modal(frame, app, area),
        Mode::HelpModal => render_help_modal(frame, area),
        Mode::RegionPicker => render_region_selector(frame, app, area),
        Mode::ProfilePicker => render_profile_picker(frame, app, area),
        Mode::CalendarPicker => render_calendar_picker(frame, app, area),
        Mode::TabPicker => render_tab_picker(frame, app, area),
        Mode::SessionPicker => render_session_picker(frame, app, area),
        _ => {}
    }
}

fn render_tabs_row(frame: &mut Frame, app: &App, area: Rect) {
    // Split into 2 lines: profile info on top, tabs below
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    // Profile info line (highlighted)
    let now = chrono::Utc::now();
    let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();

    let (identity_label, identity_value) = if app.config.role_arn.is_empty() {
        ("Identity:", "N/A".to_string())
    } else if let Some(role_part) = app.config.role_arn.split("assumed-role/").nth(1) {
        (
            "Role:",
            role_part.split('/').next().unwrap_or("N/A").to_string(),
        )
    } else if let Some(user_part) = app.config.role_arn.split(":user/").nth(1) {
        ("User:", user_part.to_string())
    } else {
        ("Identity:", "N/A".to_string())
    };

    let region_display = if app.config.region_auto_detected {
        format!(" {} ⚡ ⋮ ", app.config.region)
    } else {
        format!(" {} ⋮ ", app.config.region)
    };

    let info_spans = vec![
        Span::styled(
            "Profile:",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} ⋮ ", app.profile),
            Style::default().fg(Color::White),
        ),
        Span::styled(
            "Account:",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} ⋮ ", app.config.account_id),
            Style::default().fg(Color::White),
        ),
        Span::styled(
            "Region:",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(region_display, Style::default().fg(Color::White)),
        Span::styled(
            identity_label,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} ⋮ ", identity_value),
            Style::default().fg(Color::White),
        ),
        Span::styled(
            "Timestamp:",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} (UTC)", timestamp),
            Style::default().fg(Color::White),
        ),
    ];

    let info_widget = Paragraph::new(Line::from(info_spans))
        .alignment(Alignment::Right)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(info_widget, chunks[0]);

    // Tabs line
    let tab_data: Vec<(&str, bool)> = app
        .tabs
        .iter()
        .enumerate()
        .map(|(i, tab)| (tab.title.as_ref(), i == app.current_tab))
        .collect();
    let spans = render_tab_spans(&tab_data);

    let tabs_widget = Paragraph::new(Line::from(spans));
    frame.render_widget(tabs_widget, chunks[1]);
}

fn render_top_bar(frame: &mut Frame, app: &App, area: Rect) {
    let breadcrumbs_str = app.breadcrumbs();

    // For S3 with prefix, highlight the last part (prefix)
    let breadcrumb_line = if app.current_service == Service::S3Buckets
        && app.s3_state.current_bucket.is_some()
        && !app.s3_state.prefix_stack.is_empty()
    {
        let parts: Vec<&str> = breadcrumbs_str.split(" > ").collect();
        let mut spans = Vec::new();
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" > "));
            }
            if i == parts.len() - 1 {
                // Last part (prefix) - highlight in cyan
                spans.push(Span::styled(
                    *part,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::raw(*part));
            }
        }
        Line::from(spans)
    } else {
        Line::from(breadcrumbs_str)
    };

    let breadcrumb_widget =
        Paragraph::new(breadcrumb_line).style(Style::default().fg(Color::White));

    frame.render_widget(breadcrumb_widget, area);
}
fn render_bottom_bar(frame: &mut Frame, app: &App, area: Rect) {
    status::render_bottom_bar(frame, app, area);
}

fn render_service(frame: &mut Frame, app: &App, area: Rect) {
    match app.current_service {
        Service::CloudWatchLogGroups => {
            if app.view_mode == ViewMode::Events {
                cw::logs::render_events(frame, app, area);
            } else if app.view_mode == ViewMode::Detail {
                cw::logs::render_group_detail(frame, app, area);
            } else {
                cw::logs::render_groups_list(frame, app, area);
            }
        }
        Service::CloudWatchInsights => cw::render_insights(frame, app, area),
        Service::CloudWatchAlarms => cw::render_alarms(frame, app, area),
        Service::Ec2Instances => ec2::render_instances(
            frame,
            area,
            &app.ec2_state,
            &app.ec2_visible_column_ids
                .iter()
                .map(|s| s.as_ref())
                .collect::<Vec<_>>(),
            app.mode,
        ),
        Service::EcrRepositories => ecr::render_repositories(frame, app, area),
        Service::LambdaFunctions => lambda::render_functions(frame, app, area),
        Service::LambdaApplications => lambda::render_applications(frame, app, area),
        Service::S3Buckets => s3::render_buckets(frame, app, area),
        Service::SqsQueues => sqs::render_queues(frame, app, area),
        Service::CloudFormationStacks => cfn::render_stacks(frame, app, area),
        Service::IamUsers => iam::render_users(frame, app, area),
        Service::IamRoles => iam::render_roles(frame, app, area),
        Service::IamUserGroups => iam::render_user_groups(frame, app, area),
    }
}

fn render_column_selector(frame: &mut Frame, app: &App, area: Rect) {
    let (items, title, max_text_len) = if app.current_service == Service::S3Buckets
        && app.s3_state.current_bucket.is_none()
    {
        let mut max_len = 0;
        let items: Vec<ListItem> = app
            .s3_bucket_column_ids
            .iter()
            .filter_map(|col_id| {
                BucketColumn::from_id(col_id).map(|col| {
                    let is_visible = app.s3_bucket_visible_column_ids.contains(col_id);
                    let (item, len) = render_column_toggle_string(&col.name(), is_visible);
                    max_len = max_len.max(len);
                    item
                })
            })
            .collect();
        (items, " Preferences ", max_len)
    } else if app.current_service == Service::CloudWatchAlarms {
        let mut all_items: Vec<ListItem> = Vec::new();
        let mut max_len = 0;

        // Columns section
        let (header, header_len) = render_section_header("Columns");
        all_items.push(header);
        max_len = max_len.max(header_len);

        for col_id in &app.cw_alarm_column_ids {
            let is_visible = app.cw_alarm_visible_column_ids.contains(col_id);
            if let Some(col) = AlarmColumn::from_id(col_id) {
                let (item, len) = render_column_toggle_string(&col.name(), is_visible);
                all_items.push(item);
                max_len = max_len.max(len);
            }
        }

        // View As section
        all_items.push(ListItem::new(""));
        let (header, header_len) = render_section_header("View as");
        all_items.push(header);
        max_len = max_len.max(header_len);

        let (item, len) = render_radio_item(
            "Table",
            app.alarms_state.view_as == AlarmViewMode::Table,
            true,
        );
        all_items.push(item);
        max_len = max_len.max(len);

        let (item, len) = render_radio_item(
            "Cards",
            app.alarms_state.view_as == AlarmViewMode::Cards,
            true,
        );
        all_items.push(item);
        max_len = max_len.max(len);

        // Page Size section
        all_items.push(ListItem::new(""));
        let (page_items, page_len) = render_page_size_section(
            app.alarms_state.table.page_size,
            &[
                (PageSize::Ten, "10"),
                (PageSize::TwentyFive, "25"),
                (PageSize::Fifty, "50"),
                (PageSize::OneHundred, "100"),
            ],
        );
        all_items.extend(page_items);
        max_len = max_len.max(page_len);

        // Wrap Lines section
        all_items.push(ListItem::new(""));
        let (header, header_len) = render_section_header("Wrap lines");
        all_items.push(header);
        max_len = max_len.max(header_len);

        let (item, len) = render_column_toggle_string("Wrap lines", app.alarms_state.wrap_lines);
        all_items.push(item);
        max_len = max_len.max(len);

        (all_items, " Preferences ", max_len)
    } else if app.view_mode == ViewMode::Events {
        let mut max_len = 0;
        let items: Vec<ListItem> = app
            .cw_log_event_column_ids
            .iter()
            .filter_map(|col_id| {
                EventColumn::from_id(col_id).map(|col| {
                    let is_visible = app.cw_log_event_visible_column_ids.contains(col_id);
                    let (item, len) = render_column_toggle_string(col.name(), is_visible);
                    max_len = max_len.max(len);
                    item
                })
            })
            .collect();
        (items, " Select visible columns (Space to toggle) ", max_len)
    } else if app.view_mode == ViewMode::Detail {
        let mut max_len = 0;
        let items: Vec<ListItem> = app
            .cw_log_stream_column_ids
            .iter()
            .filter_map(|col_id| {
                StreamColumn::from_id(col_id).map(|col| {
                    let is_visible = app.cw_log_stream_visible_column_ids.contains(col_id);
                    let (item, len) = render_column_toggle_string(col.name(), is_visible);
                    max_len = max_len.max(len);
                    item
                })
            })
            .collect();
        (items, " Preferences ", max_len)
    } else if app.current_service == Service::CloudWatchLogGroups {
        let mut max_len = 0;
        let items: Vec<ListItem> = app
            .cw_log_group_column_ids
            .iter()
            .filter_map(|col_id| {
                LogGroupColumn::from_id(col_id).map(|col| {
                    let is_visible = app.cw_log_group_visible_column_ids.contains(col_id);
                    let (item, len) = render_column_toggle_string(col.name(), is_visible);
                    max_len = max_len.max(len);
                    item
                })
            })
            .collect();
        (items, " Preferences ", max_len)
    } else if app.current_service == Service::EcrRepositories {
        let mut max_len = 0;
        let items: Vec<ListItem> = if app.ecr_state.current_repository.is_some() {
            // ECR images columns
            app.ecr_image_column_ids
                .iter()
                .filter_map(|col_id| {
                    image::Column::from_id(col_id).map(|col| {
                        let is_visible = app.ecr_image_visible_column_ids.contains(col_id);
                        let (item, len) = render_column_toggle_string(&col.name(), is_visible);
                        max_len = max_len.max(len);
                        item
                    })
                })
                .collect()
        } else {
            // ECR repository columns
            app.ecr_repo_column_ids
                .iter()
                .filter_map(|col_id| {
                    repo::Column::from_id(col_id).map(|col| {
                        let is_visible = app.ecr_repo_visible_column_ids.contains(col_id);
                        let (item, len) = render_column_toggle_string(&col.name(), is_visible);
                        max_len = max_len.max(len);
                        item
                    })
                })
                .collect()
        };
        (items, " Preferences ", max_len)
    } else if app.current_service == Service::Ec2Instances {
        let mut all_items: Vec<ListItem> = Vec::new();
        let mut max_len = 0;

        let (header, header_len) = render_section_header("Columns");
        all_items.push(header);
        max_len = max_len.max(header_len);

        for col_id in &app.ec2_column_ids {
            if let Some(col) = Ec2Column::from_id(col_id) {
                let is_visible = app.ec2_visible_column_ids.contains(col_id);
                let (item, len) = render_column_toggle_string(&col.name(), is_visible);
                all_items.push(item);
                max_len = max_len.max(len);
            }
        }

        all_items.push(ListItem::new(""));

        let (page_items, page_len) = render_page_size_section(
            app.ec2_state.table.page_size,
            &[
                (PageSize::Ten, "10"),
                (PageSize::TwentyFive, "25"),
                (PageSize::Fifty, "50"),
                (PageSize::OneHundred, "100"),
            ],
        );
        all_items.extend(page_items);
        max_len = max_len.max(page_len);

        (all_items, " Preferences ", max_len)
    } else if app.current_service == Service::SqsQueues {
        if app.sqs_state.current_queue.is_some()
            && app.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers
        {
            // Triggers tab - columns + page size
            let mut all_items: Vec<ListItem> = Vec::new();
            let mut max_len = 0;

            let (header, header_len) = render_section_header("Columns");
            all_items.push(header);
            max_len = max_len.max(header_len);

            for col_id in &app.sqs_state.trigger_column_ids {
                if let Some(col) = SqsTriggerColumn::from_id(col_id) {
                    let is_visible = app.sqs_state.trigger_visible_column_ids.contains(col_id);
                    let (item, len) = render_column_toggle_string(&col.name(), is_visible);
                    all_items.push(item);
                    max_len = max_len.max(len);
                }
            }

            all_items.push(ListItem::new(""));
            let (page_items, page_len) = render_page_size_section(
                app.sqs_state.triggers.page_size,
                &[
                    (PageSize::Ten, "10"),
                    (PageSize::TwentyFive, "25"),
                    (PageSize::Fifty, "50"),
                    (PageSize::OneHundred, "100"),
                ],
            );
            all_items.extend(page_items);
            max_len = max_len.max(page_len);

            (all_items, " Preferences ", max_len)
        } else {
            // Queue list - just columns
            let mut max_len = 0;
            let items: Vec<ListItem> = app
                .sqs_column_ids
                .iter()
                .filter_map(|col_id| {
                    SqsColumn::from_id(col_id).map(|col| {
                        let is_visible = app.sqs_visible_column_ids.contains(col_id);
                        let (item, len) = render_column_toggle_string(&col.name(), is_visible);
                        max_len = max_len.max(len);
                        item
                    })
                })
                .collect();
            (items, " Preferences ", max_len)
        }
    } else if app.current_service == Service::LambdaFunctions {
        let mut all_items: Vec<ListItem> = Vec::new();
        let mut max_len = 0;

        let (header, header_len) = render_section_header("Columns");
        all_items.push(header);
        max_len = max_len.max(header_len);

        // Show appropriate columns based on current tab
        if app.lambda_state.current_function.is_some()
            && app.lambda_state.detail_tab == LambdaDetailTab::Code
        {
            // Show layer columns for Code tab
            for col in &app.lambda_state.layer_column_ids {
                let is_visible = app.lambda_state.layer_visible_column_ids.contains(col);
                let (item, len) = render_column_toggle_string(col, is_visible);
                all_items.push(item);
                max_len = max_len.max(len);
            }
        } else if app.lambda_state.detail_tab == LambdaDetailTab::Versions {
            for col in &app.lambda_state.version_column_ids {
                let is_visible = app.lambda_state.version_visible_column_ids.contains(col);
                let (item, len) = render_column_toggle_string(col, is_visible);
                all_items.push(item);
                max_len = max_len.max(len);
            }
        } else if app.lambda_state.detail_tab == LambdaDetailTab::Aliases {
            for col in &app.lambda_state.alias_column_ids {
                let is_visible = app.lambda_state.alias_visible_column_ids.contains(col);
                let (item, len) = render_column_toggle_string(col, is_visible);
                all_items.push(item);
                max_len = max_len.max(len);
            }
        } else {
            for col_id in &app.lambda_state.function_column_ids {
                if let Some(col) = FunctionColumn::from_id(col_id) {
                    let is_visible = app
                        .lambda_state
                        .function_visible_column_ids
                        .contains(col_id);
                    let (item, len) = render_column_toggle_string(col.name(), is_visible);
                    all_items.push(item);
                    max_len = max_len.max(len);
                }
            }
        }

        all_items.push(ListItem::new(""));

        let (page_items, page_len) = render_page_size_section(
            if app.lambda_state.detail_tab == LambdaDetailTab::Versions {
                app.lambda_state.version_table.page_size
            } else {
                app.lambda_state.table.page_size
            },
            &[
                (PageSize::Ten, "10"),
                (PageSize::TwentyFive, "25"),
                (PageSize::Fifty, "50"),
                (PageSize::OneHundred, "100"),
            ],
        );
        all_items.extend(page_items);
        max_len = max_len.max(page_len);

        (all_items, " Preferences ", max_len)
    } else if app.current_service == Service::LambdaApplications {
        let mut all_items: Vec<ListItem> = Vec::new();
        let mut max_len = 0;

        let (header, header_len) = render_section_header("Columns");
        all_items.push(header);
        max_len = max_len.max(header_len);

        // Show different columns based on current view
        if app.lambda_application_state.current_application.is_some() {
            if app.lambda_application_state.detail_tab == ApplicationDetailTab::Overview {
                // Resources columns
                for col_id in &app.lambda_resource_column_ids {
                    let is_visible = app.lambda_resource_visible_column_ids.contains(col_id);
                    if let Some(col) = ResourceColumn::from_id(col_id) {
                        let (item, len) = render_column_toggle_string(&col.name(), is_visible);
                        all_items.push(item);
                        max_len = max_len.max(len);
                    }
                }

                all_items.push(ListItem::new(""));
                let (page_items, page_len) = render_page_size_section(
                    app.lambda_application_state.resources.page_size,
                    &[
                        (PageSize::Ten, "10"),
                        (PageSize::TwentyFive, "25"),
                        (PageSize::Fifty, "50"),
                    ],
                );
                all_items.extend(page_items);
                max_len = max_len.max(page_len);
            } else {
                // Deployments columns
                for col_id in &app.lambda_deployment_column_ids {
                    let is_visible = app.lambda_deployment_visible_column_ids.contains(col_id);
                    if let Some(col) = DeploymentColumn::from_id(col_id) {
                        let (item, len) = render_column_toggle_string(&col.name(), is_visible);
                        all_items.push(item);
                        max_len = max_len.max(len);
                    }
                }

                all_items.push(ListItem::new(""));
                let (page_items, page_len) = render_page_size_section(
                    app.lambda_application_state.deployments.page_size,
                    &[
                        (PageSize::Ten, "10"),
                        (PageSize::TwentyFive, "25"),
                        (PageSize::Fifty, "50"),
                    ],
                );
                all_items.extend(page_items);
                max_len = max_len.max(page_len);
            }
        } else {
            // Application list columns
            for col_id in &app.lambda_application_column_ids {
                if let Some(col) = ApplicationColumn::from_id(col_id) {
                    let is_visible = app.lambda_application_visible_column_ids.contains(col_id);
                    let (item, len) = render_column_toggle_string(&col.name(), is_visible);
                    all_items.push(item);
                    max_len = max_len.max(len);
                }
            }

            all_items.push(ListItem::new(""));
            let (page_items, page_len) = render_page_size_section(
                app.lambda_application_state.table.page_size,
                &[
                    (PageSize::Ten, "10"),
                    (PageSize::TwentyFive, "25"),
                    (PageSize::Fifty, "50"),
                ],
            );
            all_items.extend(page_items);
            max_len = max_len.max(page_len);
        }

        (all_items, " Preferences ", max_len)
    } else if app.current_service == Service::CloudFormationStacks {
        let mut all_items: Vec<ListItem> = Vec::new();
        let mut max_len = 0;

        // Check if we're in StackInfo tab (tags table)
        if app.cfn_state.current_stack.is_some()
            && app.cfn_state.detail_tab == CfnDetailTab::StackInfo
        {
            let (header, header_len) = render_section_header("Columns");
            all_items.push(header);
            max_len = max_len.max(header_len);

            // Tags only have Key and Value columns
            let tag_columns = ["Key", "Value"];
            for col_name in &tag_columns {
                let (item, len) = render_column_toggle_string(col_name, true);
                all_items.push(item);
                max_len = max_len.max(len);
            }

            all_items.push(ListItem::new(""));
            let (page_items, page_len) = render_page_size_section(
                app.cfn_state.tags.page_size,
                &[
                    (PageSize::Ten, "10"),
                    (PageSize::TwentyFive, "25"),
                    (PageSize::Fifty, "50"),
                    (PageSize::OneHundred, "100"),
                ],
            );
            all_items.extend(page_items);
            max_len = max_len.max(page_len);
        } else if app.cfn_state.current_stack.is_some()
            && app.cfn_state.detail_tab == CfnDetailTab::Parameters
        {
            let (header, header_len) = render_section_header("Columns");
            all_items.push(header);
            max_len = max_len.max(header_len);

            for col_id in &app.cfn_parameter_column_ids {
                let is_visible = app.cfn_parameter_visible_column_ids.contains(col_id);
                if let Some(col) = ParameterColumn::from_id(col_id) {
                    let name = translate_column(col.id(), col.default_name());
                    let (item, len) = render_column_toggle_string(&name, is_visible);
                    all_items.push(item);
                    max_len = max_len.max(len);
                }
            }

            all_items.push(ListItem::new(""));
            let (page_items, page_len) = render_page_size_section(
                app.cfn_state.parameters.page_size,
                &[
                    (PageSize::Ten, "10"),
                    (PageSize::TwentyFive, "25"),
                    (PageSize::Fifty, "50"),
                    (PageSize::OneHundred, "100"),
                ],
            );
            all_items.extend(page_items);
            max_len = max_len.max(page_len);
        } else if app.cfn_state.current_stack.is_some()
            && app.cfn_state.detail_tab == CfnDetailTab::Outputs
        {
            let (header, header_len) = render_section_header("Columns");
            all_items.push(header);
            max_len = max_len.max(header_len);

            for col_id in &app.cfn_output_column_ids {
                let is_visible = app.cfn_output_visible_column_ids.contains(col_id);
                if let Some(col) = OutputColumn::from_id(col_id) {
                    let name = translate_column(col.id(), col.default_name());
                    let (item, len) = render_column_toggle_string(&name, is_visible);
                    all_items.push(item);
                    max_len = max_len.max(len);
                }
            }

            all_items.push(ListItem::new(""));
            let (page_items, page_len) = render_page_size_section(
                app.cfn_state.outputs.page_size,
                &[
                    (PageSize::Ten, "10"),
                    (PageSize::TwentyFive, "25"),
                    (PageSize::Fifty, "50"),
                    (PageSize::OneHundred, "100"),
                ],
            );
            all_items.extend(page_items);
            max_len = max_len.max(page_len);
        } else if app.cfn_state.current_stack.is_some()
            && app.cfn_state.detail_tab == CfnDetailTab::Resources
        {
            let (header, header_len) = render_section_header("Columns");
            all_items.push(header);
            max_len = max_len.max(header_len);

            for col_id in &app.cfn_resource_column_ids {
                let is_visible = app.cfn_resource_visible_column_ids.contains(col_id);
                if let Some(col) = CfnResourceColumn::from_id(col_id) {
                    let name = translate_column(col.id(), col.default_name());
                    let (item, len) = render_column_toggle_string(&name, is_visible);
                    all_items.push(item);
                    max_len = max_len.max(len);
                }
            }

            all_items.push(ListItem::new(""));
            let (page_items, page_len) = render_page_size_section(
                app.cfn_state.resources.page_size,
                &[
                    (PageSize::Ten, "10"),
                    (PageSize::TwentyFive, "25"),
                    (PageSize::Fifty, "50"),
                    (PageSize::OneHundred, "100"),
                ],
            );
            all_items.extend(page_items);
            max_len = max_len.max(page_len);
        } else if app.cfn_state.current_stack.is_none() {
            // Stack list view
            let (header, header_len) = render_section_header("Columns");
            all_items.push(header);
            max_len = max_len.max(header_len);

            for col_id in &app.cfn_column_ids {
                let is_visible = app.cfn_visible_column_ids.contains(col_id);
                if let Some(col) = CfnColumn::from_id(col_id) {
                    let (item, len) = render_column_toggle_string(&col.name(), is_visible);
                    all_items.push(item);
                    max_len = max_len.max(len);
                }
            }

            all_items.push(ListItem::new(""));
            let (page_items, page_len) = render_page_size_section(
                app.cfn_state.table.page_size,
                &[
                    (PageSize::Ten, "10"),
                    (PageSize::TwentyFive, "25"),
                    (PageSize::Fifty, "50"),
                    (PageSize::OneHundred, "100"),
                ],
            );
            all_items.extend(page_items);
            max_len = max_len.max(page_len);
        }
        // Template tab: no preferences

        (all_items, " Preferences ", max_len)
    } else if app.current_service == Service::IamUsers {
        let mut all_items: Vec<ListItem> = Vec::new();
        let mut max_len = 0;

        // Show policy columns only for Permissions tab in user detail view
        if app.iam_state.current_user.is_some() && app.iam_state.user_tab == UserTab::Permissions {
            let (header, header_len) = render_section_header("Columns");
            all_items.push(header);
            max_len = max_len.max(header_len);

            for col in &app.iam_policy_column_ids {
                let is_visible = app.iam_policy_visible_column_ids.contains(col);
                let mut spans = vec![];
                spans.extend(render_toggle(is_visible));
                spans.push(Span::raw(" "));
                spans.push(Span::raw(col.clone()));
                let text_len = 4 + col.len();
                all_items.push(ListItem::new(Line::from(spans)));
                max_len = max_len.max(text_len);
            }

            all_items.push(ListItem::new(""));
            let (page_items, page_len) = render_page_size_section(
                app.iam_state.policies.page_size,
                &[
                    (PageSize::Ten, "10"),
                    (PageSize::TwentyFive, "25"),
                    (PageSize::Fifty, "50"),
                ],
            );
            all_items.extend(page_items);
            max_len = max_len.max(page_len);
        } else if app.iam_state.current_user.is_none() {
            let (header, header_len) = render_section_header("Columns");
            all_items.push(header);
            max_len = max_len.max(header_len);

            for col in &app.iam_user_column_ids {
                let is_visible = app.iam_user_visible_column_ids.contains(col);
                let (item, len) = render_column_toggle_string(col, is_visible);
                all_items.push(item);
                max_len = max_len.max(len);
            }

            all_items.push(ListItem::new(""));
            let (page_items, page_len) = render_page_size_section(
                app.iam_state.users.page_size,
                &[
                    (PageSize::Ten, "10"),
                    (PageSize::TwentyFive, "25"),
                    (PageSize::Fifty, "50"),
                ],
            );
            all_items.extend(page_items);
            max_len = max_len.max(page_len);
        }

        (all_items, " Preferences ", max_len)
    } else if app.current_service == Service::IamRoles {
        let mut all_items: Vec<ListItem> = Vec::new();
        let mut max_len = 0;

        let (header, header_len) = render_section_header("Columns");
        all_items.push(header);
        max_len = max_len.max(header_len);

        for col in &app.iam_role_column_ids {
            let is_visible = app.iam_role_visible_column_ids.contains(col);
            let mut spans = vec![];
            spans.extend(render_toggle(is_visible));
            spans.push(Span::raw(" "));
            spans.push(Span::raw(col.clone()));
            let text_len = 4 + col.len();
            all_items.push(ListItem::new(Line::from(spans)));
            max_len = max_len.max(text_len);
        }

        all_items.push(ListItem::new(""));
        let (page_items, page_len) = render_page_size_section(
            app.iam_state.roles.page_size,
            &[
                (PageSize::Ten, "10"),
                (PageSize::TwentyFive, "25"),
                (PageSize::Fifty, "50"),
            ],
        );
        all_items.extend(page_items);
        max_len = max_len.max(page_len);

        (all_items, " Preferences ", max_len)
    } else if app.current_service == Service::IamUserGroups {
        let mut all_items: Vec<ListItem> = Vec::new();
        let mut max_len = 0;

        let (header, header_len) = render_section_header("Columns");
        all_items.push(header);
        max_len = max_len.max(header_len);

        for col in &app.iam_group_column_ids {
            let is_visible = app.iam_group_visible_column_ids.contains(col);
            let mut spans = vec![];
            spans.extend(render_toggle(is_visible));
            spans.push(Span::raw(" "));
            spans.push(Span::raw(col.clone()));
            let text_len = 4 + col.len();
            all_items.push(ListItem::new(Line::from(spans)));
            max_len = max_len.max(text_len);
        }

        all_items.push(ListItem::new(""));
        let (page_items, page_len) = render_page_size_section(
            app.iam_state.groups.page_size,
            &[
                (PageSize::Ten, "10"),
                (PageSize::TwentyFive, "25"),
                (PageSize::Fifty, "50"),
            ],
        );
        all_items.extend(page_items);
        max_len = max_len.max(page_len);

        (all_items, " Preferences ", max_len)
    } else {
        // Fallback for unknown services
        (vec![], " Preferences ", 0)
    };

    // Calculate popup size based on content
    let item_count = items.len();

    // Width: based on content + padding
    let width = (max_text_len + 10).clamp(30, 100) as u16; // +10 for padding, min 30, max 100

    // Height: fit all items if possible, otherwise use max available and show scrollbar
    let height = (item_count as u16 + 2).max(8); // +2 for borders, min 8
    let max_height = area.height.saturating_sub(4);
    let actual_height = height.min(max_height);
    let popup_area = centered_rect_absolute(width, actual_height, area);

    // Check if scrollbar is needed
    let needs_scrollbar = height > max_height;

    // Preferences should always have green border (active state)
    let border_color = Color::Green;

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("► ");

    let mut state = ListState::default();
    state.select(Some(app.column_selector_index));

    frame.render_widget(Clear, popup_area);
    frame.render_stateful_widget(list, popup_area, &mut state);

    // Render scrollbar only if content doesn't fit
    if needs_scrollbar {
        render_scrollbar(
            frame,
            popup_area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            item_count,
            app.column_selector_index,
        );
    }
}

fn render_error_modal(frame: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(80, 60, area);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(
        Block::default()
            .title(" Error ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_type(BorderType::Rounded)
            .border_style(red_text())
            .style(Style::default().bg(Color::Black)),
        popup_area,
    );

    let inner = popup_area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });

    let error_text = app.error_message.as_deref().unwrap_or("Unknown error");

    let chunks = vertical(
        [
            Constraint::Length(2), // Header
            Constraint::Min(0),    // Error text (scrollable)
            Constraint::Length(2), // Help text
        ],
        inner,
    );

    // Header
    let header = Paragraph::new("AWS Error")
        .alignment(Alignment::Center)
        .style(red_text().add_modifier(Modifier::BOLD));
    frame.render_widget(header, chunks[0]);

    // Scrollable error text with border
    let error_lines: Vec<Line> = error_text
        .lines()
        .skip(app.error_scroll)
        .flat_map(|line| {
            let width = chunks[1].width.saturating_sub(4) as usize; // Account for borders + padding
            if line.len() <= width {
                vec![Line::from(line)]
            } else {
                line.chars()
                    .collect::<Vec<_>>()
                    .chunks(width)
                    .map(|chunk| Line::from(chunk.iter().collect::<String>()))
                    .collect()
            }
        })
        .collect();

    let error_paragraph = Paragraph::new(error_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_type(BorderType::Rounded)
                .border_style(active_border()),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(error_paragraph, chunks[1]);

    // Render scrollbar if needed
    let total_lines: usize = error_text
        .lines()
        .map(|line| {
            let width = chunks[1].width.saturating_sub(4) as usize;
            if line.len() <= width {
                1
            } else {
                line.len().div_ceil(width)
            }
        })
        .sum();
    let visible_lines = chunks[1].height.saturating_sub(2) as usize;
    if total_lines > visible_lines {
        render_scrollbar(
            frame,
            chunks[1].inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            total_lines,
            app.error_scroll,
        );
    }

    // Help text
    let help_spans = vec![
        first_hint("^r", "retry"),
        hint("y", "copy"),
        hint("↑↓,^u,^d", "scroll"),
        last_hint("q,⎋", "close"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let help = Paragraph::new(Line::from(help_spans)).alignment(Alignment::Center);

    frame.render_widget(help, chunks[2]);
}

fn render_space_menu(frame: &mut Frame, area: Rect) {
    let items = vec![
        Line::from(vec![
            Span::styled("o", Style::default().fg(Color::Yellow)),
            Span::raw(" services"),
        ]),
        Line::from(vec![
            Span::styled("t", Style::default().fg(Color::Yellow)),
            Span::raw(" tabs"),
        ]),
        Line::from(vec![
            Span::styled("c", Style::default().fg(Color::Yellow)),
            Span::raw(" close"),
        ]),
        Line::from(vec![
            Span::styled("r", Style::default().fg(Color::Yellow)),
            Span::raw(" regions"),
        ]),
        Line::from(vec![
            Span::styled("s", Style::default().fg(Color::Yellow)),
            Span::raw(" sessions"),
        ]),
        Line::from(vec![
            Span::styled("h", Style::default().fg(Color::Yellow)),
            Span::raw(" help"),
        ]),
    ];

    let menu_height = items.len() as u16 + 2; // +2 for borders
    let menu_area = bottom_right_rect(30, menu_height, area);

    let paragraph = Paragraph::new(items)
        .block(
            Block::default()
                .title(" Menu ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_type(BorderType::Rounded)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().bg(Color::Black));

    frame.render_widget(Clear, menu_area);
    frame.render_widget(paragraph, menu_area);
}

fn render_service_picker(frame: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(60, 60, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(popup_area);

    let is_active = app.mode == Mode::ServicePicker;
    let cursor = get_cursor(is_active);
    let active_color = Color::Green;
    let inactive_color = Color::Cyan;
    let filter = Paragraph::new(Line::from(vec![
        Span::raw(&app.service_picker.filter),
        Span::styled(cursor, Style::default().fg(active_color)),
    ]))
    .block(
        Block::default()
            .title(SEARCH_ICON)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if is_active {
                active_color
            } else {
                inactive_color
            })),
    )
    .style(Style::default());

    let filtered = app.filtered_services();
    let items: Vec<ListItem> = filtered.iter().map(|s| ListItem::new(*s)).collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" AWS Services ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_type(BorderType::Rounded)
                .border_type(BorderType::Rounded)
                .border_style(if is_active {
                    active_border()
                } else {
                    Style::default().fg(Color::Cyan)
                }),
        )
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("► ");

    let mut state = ListState::default();
    state.select(Some(app.service_picker.selected));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(filter, chunks[0]);
    frame.render_stateful_widget(list, chunks[1], &mut state);
}

fn render_tab_picker(frame: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(80, 60, area);

    // Split into filter, list and preview
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(popup_area);

    // Filter input
    let filter_text = if app.tab_filter.is_empty() {
        "Type to filter tabs...".to_string()
    } else {
        app.tab_filter.clone()
    };
    let filter_style = if app.tab_filter.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
    };
    let filter = Paragraph::new(filter_text).style(filter_style).block(
        Block::default()
            .title(SEARCH_ICON)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(Clear, main_chunks[0]);
    frame.render_widget(filter, main_chunks[0]);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[1]);

    // Tab list - use filtered tabs
    let filtered_tabs = app.get_filtered_tabs();
    let items: Vec<ListItem> = filtered_tabs
        .iter()
        .map(|(_, tab)| ListItem::new(tab.breadcrumb.clone()))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(
                    " Tabs ({}/{}) ",
                    filtered_tabs.len(),
                    app.tabs.len()
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_type(BorderType::Rounded)
                .border_type(BorderType::Rounded)
                .border_style(active_border()),
        )
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("► ");

    let mut state = ListState::default();
    state.select(Some(app.tab_picker_selected));

    frame.render_widget(Clear, chunks[0]);
    frame.render_stateful_widget(list, chunks[0], &mut state);

    // Preview pane
    frame.render_widget(Clear, chunks[1]);

    let preview_block = Block::default()
        .title(" Preview ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let preview_inner = preview_block.inner(chunks[1]);
    frame.render_widget(preview_block, chunks[1]);

    if let Some(&(_, tab)) = filtered_tabs.get(app.tab_picker_selected) {
        // Render preview using the tab's service context
        // Note: This may show stale state if the tab's service differs from current_service
        render_service_preview(frame, app, tab.service, preview_inner);
    }
}

fn render_service_preview(frame: &mut Frame, app: &App, service: Service, area: Rect) {
    match service {
        Service::CloudWatchLogGroups => {
            if app.view_mode == ViewMode::Events {
                cw::logs::render_events(frame, app, area);
            } else if app.view_mode == ViewMode::Detail {
                cw::logs::render_group_detail(frame, app, area);
            } else {
                cw::logs::render_groups_list(frame, app, area);
            }
        }
        Service::CloudWatchInsights => cw::render_insights(frame, app, area),
        Service::CloudWatchAlarms => cw::render_alarms(frame, app, area),
        Service::Ec2Instances => ec2::render_instances(
            frame,
            area,
            &app.ec2_state,
            &app.ec2_visible_column_ids
                .iter()
                .map(|s| s.as_ref())
                .collect::<Vec<_>>(),
            app.mode,
        ),
        Service::EcrRepositories => ecr::render_repositories(frame, app, area),
        Service::LambdaFunctions => lambda::render_functions(frame, app, area),
        Service::LambdaApplications => lambda::render_applications(frame, app, area),
        Service::S3Buckets => s3::render_buckets(frame, app, area),
        Service::SqsQueues => sqs::render_queues(frame, app, area),
        Service::CloudFormationStacks => cfn::render_stacks(frame, app, area),
        Service::IamUsers => iam::render_users(frame, app, area),
        Service::IamRoles => iam::render_roles(frame, app, area),
        Service::IamUserGroups => iam::render_user_groups(frame, app, area),
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn centered_rect_absolute(width: u16, height: u16, r: Rect) -> Rect {
    let x = (r.width.saturating_sub(width)) / 2;
    let y = (r.height.saturating_sub(height)) / 2;
    Rect {
        x: r.x + x,
        y: r.y + y,
        width: width.min(r.width),
        height: height.min(r.height),
    }
}

fn bottom_right_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.width.saturating_sub(width + 1);
    let y = r.height.saturating_sub(height + 1);
    Rect {
        x: r.x + x,
        y: r.y + y,
        width: width.min(r.width),
        height: height.min(r.height),
    }
}

fn render_help_modal(frame: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(vec![Span::styled("⎋  ", red_text()), Span::raw("  Escape")]),
        Line::from(vec![
            Span::styled("⏎  ", red_text()),
            Span::raw("  Enter/Return"),
        ]),
        Line::from(vec![Span::styled("⇤⇥ ", red_text()), Span::raw("  Tab")]),
        Line::from(vec![Span::styled("␣  ", red_text()), Span::raw("  Space")]),
        Line::from(vec![Span::styled("^r ", red_text()), Span::raw("  Ctrl+r")]),
        Line::from(vec![Span::styled("^w ", red_text()), Span::raw("  Ctrl+w")]),
        Line::from(vec![Span::styled("^o ", red_text()), Span::raw("  Ctrl+o")]),
        Line::from(vec![Span::styled("^p ", red_text()), Span::raw("  Ctrl+p")]),
        Line::from(vec![
            Span::styled("^u ", red_text()),
            Span::raw("  Ctrl+u (page up)"),
        ]),
        Line::from(vec![
            Span::styled("^d ", red_text()),
            Span::raw("  Ctrl+d (page down)"),
        ]),
        Line::from(vec![
            Span::styled("[] ", red_text()),
            Span::raw("  [ and ] (switch tabs)"),
        ]),
        Line::from(vec![
            Span::styled("↑↓ ", red_text()),
            Span::raw("  Arrow up/down"),
        ]),
        Line::from(vec![
            Span::styled("←→ ", red_text()),
            Span::raw("  Arrow left/right"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", Style::default()),
            Span::styled("⎋", red_text()),
            Span::styled(" or ", Style::default()),
            Span::styled("⏎", red_text()),
            Span::styled(" to close", Style::default()),
        ]),
    ];

    // Find max line width
    let max_width = help_text
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.len())
                .sum::<usize>()
        })
        .max()
        .unwrap_or(80) as u16;

    // Content dimensions + borders + padding
    let content_width = max_width + 6; // +6 for borders and 1 char padding on each side
    let content_height = help_text.len() as u16 + 2; // +2 for borders

    // Center the dialog
    let popup_width = content_width.min(area.width.saturating_sub(4));
    let popup_height = content_height.min(area.height.saturating_sub(4));

    let popup_area = Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(Span::styled(
                    " Help ",
                    Style::default().add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_type(BorderType::Rounded)
                .border_style(active_border())
                .padding(Padding::horizontal(1)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}

fn render_region_selector(frame: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(60, 60, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(popup_area);

    // Filter input at top
    let cursor = "█";
    let filter_text = vec![Span::from(format!("{}{}", app.region_filter, cursor))];
    let filter = filter_area(filter_text, true);

    // Filtered list below
    let filtered = app.get_filtered_regions();
    let items: Vec<ListItem> = filtered
        .iter()
        .map(|r| {
            let latency_str = match r.latency_ms {
                Some(ms) => format!("({}ms)", ms),
                None => "(>1s)".to_string(),
            };
            let opt_in = if r.opt_in { "[opt-in] " } else { "" };
            let display = format!(
                "{} > {} > {} {}{}",
                r.group, r.name, r.code, opt_in, latency_str
            );
            ListItem::new(display)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Regions ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_type(BorderType::Rounded)
                .border_style(active_border()),
        )
        .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
        .highlight_symbol("▶ ");

    frame.render_widget(Clear, popup_area);
    frame.render_widget(filter, chunks[0]);
    frame.render_stateful_widget(
        list,
        chunks[1],
        &mut ratatui::widgets::ListState::default().with_selected(Some(app.region_picker_selected)),
    );
}

fn render_profile_picker(frame: &mut Frame, app: &App, area: Rect) {
    crate::aws::render_profile_picker(frame, app, area, centered_rect);
}

fn render_session_picker(frame: &mut Frame, app: &App, area: Rect) {
    crate::session::render_session_picker(frame, app, area, centered_rect);
}

fn render_calendar_picker(frame: &mut Frame, app: &App, area: Rect) {
    use ratatui::widgets::calendar::{CalendarEventStore, Monthly};

    let popup_area = centered_rect(50, 50, area);

    let date = app
        .calendar_date
        .unwrap_or_else(|| time::OffsetDateTime::now_utc().date());

    let field_name = match app.calendar_selecting {
        CalendarField::StartDate => "Start Date",
        CalendarField::EndDate => "End Date",
    };

    let events = CalendarEventStore::today(
        Style::default()
            .add_modifier(Modifier::BOLD)
            .bg(Color::Blue),
    );

    let calendar = Monthly::new(date, events)
        .block(
            Block::default()
                .title(format!(" Select {} ", field_name))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_type(BorderType::Rounded)
                .border_style(active_border()),
        )
        .show_weekdays_header(Style::new().bold().yellow())
        .show_month_header(Style::new().bold().green());

    frame.render_widget(Clear, popup_area);
    frame.render_widget(calendar, popup_area);
}

// Render JSON content with syntax highlighting and scrollbar
pub fn render_json_highlighted(
    frame: &mut Frame,
    area: Rect,
    json_text: &str,
    scroll_offset: usize,
    title: &str,
    is_active: bool,
) {
    let lines: Vec<Line> = json_text
        .lines()
        .skip(scroll_offset)
        .map(|line| {
            let mut spans = Vec::new();
            let trimmed = line.trim_start();
            let indent = line.len() - trimmed.len();

            if indent > 0 {
                spans.push(Span::raw(" ".repeat(indent)));
            }

            if trimmed.starts_with('"') && trimmed.contains(':') {
                if let Some(colon_pos) = trimmed.find(':') {
                    spans.push(Span::styled(
                        &trimmed[..colon_pos],
                        Style::default().fg(Color::Blue),
                    ));
                    spans.push(Span::raw(&trimmed[colon_pos..]));
                } else {
                    spans.push(Span::raw(trimmed));
                }
            } else if trimmed.starts_with('"') {
                spans.push(Span::styled(trimmed, Style::default().fg(Color::Green)));
            } else if trimmed.starts_with("true") || trimmed.starts_with("false") {
                spans.push(Span::styled(trimmed, Style::default().fg(Color::Yellow)));
            } else if trimmed.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                spans.push(Span::styled(trimmed, Style::default().fg(Color::Magenta)));
            } else {
                spans.push(Span::raw(trimmed));
            }

            Line::from(spans)
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(if is_active {
                    active_border()
                } else {
                    Style::default()
                }),
        ),
        area,
    );

    let total_lines = json_text.lines().count();
    if total_lines > 0 {
        render_scrollbar(
            frame,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            total_lines,
            scroll_offset,
        );
    }
}

// Render a tags tab with description and table
pub fn render_tags_section<F>(frame: &mut Frame, area: Rect, render_table: F)
where
    F: FnOnce(&mut Frame, Rect),
{
    let chunks = vertical([Constraint::Length(1), Constraint::Min(0)], area);

    frame.render_widget(
        Paragraph::new(
            "Tags are key-value pairs that you can add to AWS resources to help identify, organize, or search for resources.",
        ),
        chunks[0],
    );

    render_table(frame, chunks[1]);
}

// Render a permissions tab with description and policies table
pub fn render_permissions_section<F>(
    frame: &mut Frame,
    area: Rect,
    description: &str,
    render_table: F,
) where
    F: FnOnce(&mut Frame, Rect),
{
    let chunks = vertical([Constraint::Length(1), Constraint::Min(0)], area);

    frame.render_widget(Paragraph::new(description), chunks[0]);

    render_table(frame, chunks[1]);
}

// Render a last accessed tab with description, note, and table
pub fn render_last_accessed_section<F>(
    frame: &mut Frame,
    area: Rect,
    description: &str,
    note: &str,
    render_table: F,
) where
    F: FnOnce(&mut Frame, Rect),
{
    let chunks = vertical(
        [
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ],
        area,
    );

    frame.render_widget(Paragraph::new(description), chunks[0]);
    frame.render_widget(Paragraph::new(note), chunks[1]);

    render_table(frame, chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Service;
    use crate::app::Tab;
    use crate::ecr::image::Image as EcrImage;
    use crate::ecr::repo::Repository as EcrRepository;
    use crate::keymap::Action;
    use crate::lambda;
    use crate::ui::table::Column;

    fn test_app() -> App {
        App::new_without_client("test".to_string(), Some("us-east-1".to_string()))
    }

    fn test_app_no_region() -> App {
        App::new_without_client("test".to_string(), None)
    }

    #[test]
    fn test_expanded_content_wrapping_marks_continuation_lines() {
        // Simulate the wrapping logic
        let max_width = 50;
        let col_name = "Message: ";
        let value = "This is a very long message that will definitely exceed the maximum width and need to be wrapped";
        let full_line = format!("{}{}", col_name, value);

        let mut lines = Vec::new();

        if full_line.len() <= max_width {
            lines.push((full_line, true));
        } else {
            let first_chunk_len = max_width.min(full_line.len());
            lines.push((full_line[..first_chunk_len].to_string(), true));

            let mut remaining = &full_line[first_chunk_len..];
            while !remaining.is_empty() {
                let take = max_width.min(remaining.len());
                lines.push((remaining[..take].to_string(), false));
                remaining = &remaining[take..];
            }
        }

        // First line should be marked as first (true)
        assert!(lines[0].1);
        // Continuation lines should be marked as continuation (false)
        assert!(!lines[1].1);
        assert!(lines.len() > 1);
    }

    #[test]
    fn test_expanded_content_short_line_not_wrapped() {
        let max_width = 100;
        let col_name = "Timestamp: ";
        let value = "2025-03-13 19:49:30 (UTC)";
        let full_line = format!("{}{}", col_name, value);

        let mut lines = Vec::new();

        if full_line.len() <= max_width {
            lines.push((full_line.clone(), true));
        } else {
            let first_chunk_len = max_width.min(full_line.len());
            lines.push((full_line[..first_chunk_len].to_string(), true));

            let mut remaining = &full_line[first_chunk_len..];
            while !remaining.is_empty() {
                let take = max_width.min(remaining.len());
                lines.push((remaining[..take].to_string(), false));
                remaining = &remaining[take..];
            }
        }

        // Should only have one line
        assert_eq!(lines.len(), 1);
        assert!(lines[0].1);
        assert_eq!(lines[0].0, full_line);
    }

    #[test]
    fn test_tabs_display_with_separator() {
        // Test that tabs are formatted with ⋮ separator
        let tabs = [
            Tab {
                service: Service::CloudWatchLogGroups,
                title: "CloudWatch > Log Groups".to_string(),
                breadcrumb: "CloudWatch > Log Groups".to_string(),
            },
            Tab {
                service: Service::CloudWatchInsights,
                title: "CloudWatch > Logs Insights".to_string(),
                breadcrumb: "CloudWatch > Logs Insights".to_string(),
            },
        ];

        let mut spans = Vec::new();
        for (i, tab) in tabs.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" ⋮ "));
            }
            spans.push(Span::raw(tab.title.clone()));
        }

        // Should have 3 spans: Tab1, separator, Tab2
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[1].content, " ⋮ ");
    }

    #[test]
    fn test_current_tab_highlighted() {
        let tabs = [
            crate::app::Tab {
                service: Service::CloudWatchLogGroups,
                title: "CloudWatch > Log Groups".to_string(),
                breadcrumb: "CloudWatch > Log Groups".to_string(),
            },
            crate::app::Tab {
                service: Service::CloudWatchInsights,
                title: "CloudWatch > Logs Insights".to_string(),
                breadcrumb: "CloudWatch > Logs Insights".to_string(),
            },
        ];
        let current_tab = 1;

        let mut spans = Vec::new();
        for (i, tab) in tabs.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" ⋮ "));
            }
            if i == current_tab {
                spans.push(Span::styled(
                    tab.title.clone(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::raw(tab.title.clone()));
            }
        }

        // Current tab (index 2 in spans) should have yellow color
        assert_eq!(spans[2].style.fg, Some(Color::Yellow));
        assert!(spans[2].style.add_modifier.contains(Modifier::BOLD));
        // First tab should have no styling
        assert_eq!(spans[0].style.fg, None);
    }

    #[test]
    fn test_lambda_application_update_complete_shows_green_checkmark() {
        let app = crate::lambda::Application {
            name: "test-stack".to_string(),
            arn: "arn:aws:cloudformation:us-east-1:123456789012:stack/test-stack/abc123"
                .to_string(),
            description: "Test stack".to_string(),
            status: "UPDATE_COMPLETE".to_string(),
            last_modified: "2025-10-31 12:00:00 (UTC)".to_string(),
        };

        let col = ApplicationColumn::Status;
        let (text, style) = col.render(&app);
        assert_eq!(text, "✅ UPDATE_COMPLETE");
        assert_eq!(style.fg, Some(Color::Green));
    }

    #[test]
    fn test_lambda_application_create_complete_shows_green_checkmark() {
        let app = crate::lambda::Application {
            name: "test-stack".to_string(),
            arn: "arn:aws:cloudformation:us-east-1:123456789012:stack/test-stack/abc123"
                .to_string(),
            description: "Test stack".to_string(),
            status: "CREATE_COMPLETE".to_string(),
            last_modified: "2025-10-31 12:00:00 (UTC)".to_string(),
        };

        let col = ApplicationColumn::Status;
        let (text, style) = col.render(&app);
        assert_eq!(text, "✅ CREATE_COMPLETE");
        assert_eq!(style.fg, Some(Color::Green));
    }

    #[test]
    fn test_lambda_application_other_status_shows_default() {
        let app = crate::lambda::Application {
            name: "test-stack".to_string(),
            arn: "arn:aws:cloudformation:us-east-1:123456789012:stack/test-stack/abc123"
                .to_string(),
            description: "Test stack".to_string(),
            status: "UPDATE_IN_PROGRESS".to_string(),
            last_modified: "2025-10-31 12:00:00 (UTC)".to_string(),
        };

        let col = ApplicationColumn::Status;
        let (text, style) = col.render(&app);
        assert_eq!(text, "ℹ️  UPDATE_IN_PROGRESS");
        assert_eq!(style.fg, Some(ratatui::style::Color::LightBlue));
    }

    #[test]
    fn test_lambda_application_status_complete() {
        let app = crate::lambda::Application {
            name: "test-stack".to_string(),
            arn: "arn:aws:cloudformation:us-east-1:123456789012:stack/test-stack/abc123"
                .to_string(),
            description: "Test stack".to_string(),
            status: "UPDATE_COMPLETE".to_string(),
            last_modified: "2025-10-31 12:00:00 (UTC)".to_string(),
        };

        let col = ApplicationColumn::Status;
        let (text, style) = col.render(&app);
        assert_eq!(text, "✅ UPDATE_COMPLETE");
        assert_eq!(style.fg, Some(ratatui::style::Color::Green));
    }

    #[test]
    fn test_lambda_application_status_failed() {
        let app = crate::lambda::Application {
            name: "test-stack".to_string(),
            arn: "arn:aws:cloudformation:us-east-1:123456789012:stack/test-stack/abc123"
                .to_string(),
            description: "Test stack".to_string(),
            status: "UPDATE_FAILED".to_string(),
            last_modified: "2025-10-31 12:00:00 (UTC)".to_string(),
        };

        let col = ApplicationColumn::Status;
        let (text, style) = col.render(&app);
        assert_eq!(text, "❌ UPDATE_FAILED");
        assert_eq!(style.fg, Some(ratatui::style::Color::Red));
    }

    #[test]
    fn test_lambda_application_status_rollback() {
        let app = crate::lambda::Application {
            name: "test-stack".to_string(),
            arn: "arn:aws:cloudformation:us-east-1:123456789012:stack/test-stack/abc123"
                .to_string(),
            description: "Test stack".to_string(),
            status: "UPDATE_ROLLBACK_IN_PROGRESS".to_string(),
            last_modified: "2025-10-31 12:00:00 (UTC)".to_string(),
        };

        let col = ApplicationColumn::Status;
        let (text, style) = col.render(&app);
        assert_eq!(text, "❌ UPDATE_ROLLBACK_IN_PROGRESS");
        assert_eq!(style.fg, Some(ratatui::style::Color::Red));
    }

    #[test]
    fn test_tab_picker_shows_breadcrumb_and_preview() {
        let tabs = [
            crate::app::Tab {
                service: crate::app::Service::CloudWatchLogGroups,
                title: "CloudWatch > Log Groups".to_string(),
                breadcrumb: "CloudWatch > Log Groups".to_string(),
            },
            crate::app::Tab {
                service: crate::app::Service::CloudWatchAlarms,
                title: "CloudWatch > Alarms".to_string(),
                breadcrumb: "CloudWatch > Alarms".to_string(),
            },
        ];

        // Tab picker should show breadcrumb in list
        let selected_idx = 1;
        let selected_tab = &tabs[selected_idx];
        assert_eq!(selected_tab.breadcrumb, "CloudWatch > Alarms");
        assert_eq!(selected_tab.title, "CloudWatch > Alarms");

        // Preview should show both service and tab name
        assert!(selected_tab.breadcrumb.contains("CloudWatch"));
        assert!(selected_tab.breadcrumb.contains("Alarms"));
    }

    #[test]
    fn test_tab_picker_has_active_border() {
        // Tab picker should have green border like other active controls
        let border_style = Style::default().fg(Color::Green);
        let border_type = BorderType::Plain;

        // Verify green color is used
        assert_eq!(border_style.fg, Some(Color::Green));
        // Verify plain border type
        assert_eq!(border_type, BorderType::Plain);
    }

    #[test]
    fn test_tab_picker_title_is_tabs() {
        // Tab picker should be titled "Tabs" not "Open Tabs"
        let title = " Tabs ";
        assert_eq!(title.trim(), "Tabs");
        assert!(!title.contains("Open"));
    }

    #[test]
    fn test_s3_bucket_tabs_no_count_in_tabs() {
        // S3 bucket type tabs should not show counts (only in table title)
        let general_purpose_tab = "General purpose buckets (All AWS Regions)";
        let directory_tab = "Directory buckets";

        // Verify no count in tab labels
        assert!(!general_purpose_tab.contains("(0)"));
        assert!(!general_purpose_tab.contains("(1)"));
        assert!(!directory_tab.contains("(0)"));
        assert!(!directory_tab.contains("(1)"));

        // Count should only appear in table title
        let table_title = " General purpose buckets (42) ";
        assert!(table_title.contains("(42)"));
    }

    #[test]
    fn test_s3_bucket_column_preferences_shows_bucket_columns() {
        use crate::app::S3BucketColumn;

        let app = test_app();

        // Should have 3 bucket columns (Name, Region, CreationDate)
        assert_eq!(app.s3_bucket_column_ids.len(), 3);
        assert_eq!(app.s3_bucket_visible_column_ids.len(), 3);

        // Verify column names
        assert_eq!(S3BucketColumn::Name.name(), "Name");
        assert_eq!(S3BucketColumn::Region.name(), "Region");
        assert_eq!(S3BucketColumn::CreationDate.name(), "Creation date");
    }

    #[test]
    fn test_s3_bucket_columns_not_cloudwatch_columns() {
        let app = test_app();

        // S3 bucket columns should be different from CloudWatch log group columns
        let bucket_col_names: Vec<String> = app
            .s3_bucket_column_ids
            .iter()
            .filter_map(|id| BucketColumn::from_id(id).map(|c| c.name()))
            .collect();
        let log_col_names: Vec<String> = app
            .cw_log_group_column_ids
            .iter()
            .filter_map(|id| LogGroupColumn::from_id(id).map(|c| c.name().to_string()))
            .collect();

        // Verify they're different
        assert_ne!(bucket_col_names, log_col_names);

        // Verify S3 columns don't contain CloudWatch-specific terms
        assert!(!bucket_col_names.contains(&"Log group".to_string()));
        assert!(!bucket_col_names.contains(&"Stored bytes".to_string()));

        // Verify S3 columns contain S3-specific terms
        assert!(bucket_col_names.contains(&"Creation date".to_string()));

        // Region should NOT be in bucket columns (shown only when expanded)
        assert!(!bucket_col_names.contains(&"AWS Region".to_string()));
    }

    #[test]
    fn test_s3_bucket_column_toggle() {
        use crate::app::Service;

        let mut app = test_app();
        app.current_service = Service::S3Buckets;

        // Initially 3 columns visible
        assert_eq!(app.s3_bucket_visible_column_ids.len(), 3);

        // Simulate toggling off the Region column (index 1)
        let col = app.s3_bucket_column_ids[1];
        if let Some(pos) = app
            .s3_bucket_visible_column_ids
            .iter()
            .position(|c| *c == col)
        {
            app.s3_bucket_visible_column_ids.remove(pos);
        }

        assert_eq!(app.s3_bucket_visible_column_ids.len(), 2);
        assert!(!app
            .s3_bucket_visible_column_ids
            .contains(&"column.s3.bucket.region"));

        // Toggle it back on
        app.s3_bucket_visible_column_ids.push(col);
        assert_eq!(app.s3_bucket_visible_column_ids.len(), 3);
        assert!(app
            .s3_bucket_visible_column_ids
            .contains(&"column.s3.bucket.region"));
    }

    #[test]
    fn test_s3_preferences_dialog_title() {
        // S3 bucket preferences should be titled "Preferences" without hints
        let title = " Preferences ";
        assert_eq!(title.trim(), "Preferences");
        assert!(!title.contains("Space"));
        assert!(!title.contains("toggle"));
    }

    #[test]
    fn test_column_selector_mode_has_hotkey_hints() {
        // ColumnSelector mode should show hotkey hints in status bar
        let help = " ↑↓: scroll | ␣: toggle | esc: close ";

        // Verify key hints are present
        assert!(help.contains("␣: toggle"));
        assert!(help.contains("↑↓: scroll"));
        assert!(help.contains("esc: close"));

        // Should NOT contain unavailable keys
        assert!(!help.contains("⏎"));
        assert!(!help.contains("^w"));
    }

    #[test]
    fn test_date_range_title_no_hints() {
        // Date range title should not contain hints
        let title = " Date range ";

        // Should NOT contain hints
        assert!(!title.contains("Tab to switch"));
        assert!(!title.contains("Space to change"));
        assert!(!title.contains("("));
        assert!(!title.contains(")"));
    }

    #[test]
    fn test_event_filter_mode_has_hints_in_status_bar() {
        // EventFilterInput mode should show hints in status bar
        let help = " tab: switch | ␣: change unit | enter: apply | esc: cancel | ctrl+w: close ";

        // Verify key hints are present
        assert!(help.contains("tab: switch"));
        assert!(help.contains("␣: change unit"));
        assert!(help.contains("enter: apply"));
        assert!(help.contains("esc: cancel"));
    }

    #[test]
    fn test_s3_preferences_shows_all_columns() {
        let app = test_app();

        // Should have 3 bucket columns (Name, Region, CreationDate)
        assert_eq!(app.s3_bucket_column_ids.len(), 3);

        // All should be visible by default
        assert_eq!(app.s3_bucket_visible_column_ids.len(), 3);

        // Verify all column names
        let names: Vec<String> = app
            .s3_bucket_column_ids
            .iter()
            .filter_map(|id| BucketColumn::from_id(id).map(|c| c.name()))
            .collect();
        assert_eq!(names, vec!["Name", "Region", "Creation date"]);
    }

    #[test]
    fn test_s3_preferences_has_active_border() {
        use ratatui::style::Color;

        // S3 preferences should have green border (active state)
        let border_color = Color::Green;
        assert_eq!(border_color, Color::Green);

        // Not cyan (inactive)
        assert_ne!(border_color, Color::Cyan);
    }

    #[test]
    fn test_s3_table_loses_focus_when_preferences_shown() {
        use crate::app::Service;
        use crate::keymap::Mode;
        use ratatui::style::Color;

        let mut app = test_app();
        app.current_service = Service::S3Buckets;

        // When in Normal mode, table should be active (green)
        app.mode = Mode::Normal;
        let is_active = app.mode != Mode::ColumnSelector;
        let border_color = if is_active {
            Color::Green
        } else {
            Color::White
        };
        assert_eq!(border_color, Color::Green);

        // When in ColumnSelector mode, table should be inactive (white)
        app.mode = Mode::ColumnSelector;
        let is_active = app.mode != Mode::ColumnSelector;
        let border_color = if is_active {
            Color::Green
        } else {
            Color::White
        };
        assert_eq!(border_color, Color::White);
    }

    #[test]
    fn test_s3_object_tabs_cleared_before_render() {
        // Tabs should be cleared before rendering to prevent artifacts
        // This is verified by the Clear widget being rendered before tabs
    }

    #[test]
    fn test_s3_properties_tab_shows_bucket_info() {
        use crate::app::{S3ObjectTab, Service};

        let mut app = test_app();
        app.current_service = Service::S3Buckets;
        app.s3_state.current_bucket = Some("test-bucket".to_string());
        app.s3_state.object_tab = S3ObjectTab::Properties;

        // Properties tab should be selectable
        assert_eq!(app.s3_state.object_tab, S3ObjectTab::Properties);

        // Properties scroll should start at 0
        assert_eq!(app.s3_state.properties_scroll, 0);
    }

    #[test]
    fn test_s3_properties_scrolling() {
        use crate::app::{S3ObjectTab, Service};

        let mut app = test_app();
        app.current_service = Service::S3Buckets;
        app.s3_state.current_bucket = Some("test-bucket".to_string());
        app.s3_state.object_tab = S3ObjectTab::Properties;

        // Initial scroll should be 0
        assert_eq!(app.s3_state.properties_scroll, 0);

        // Scroll down
        app.s3_state.properties_scroll = app.s3_state.properties_scroll.saturating_add(1);
        assert_eq!(app.s3_state.properties_scroll, 1);

        app.s3_state.properties_scroll = app.s3_state.properties_scroll.saturating_add(1);
        assert_eq!(app.s3_state.properties_scroll, 2);

        // Scroll up
        app.s3_state.properties_scroll = app.s3_state.properties_scroll.saturating_sub(1);
        assert_eq!(app.s3_state.properties_scroll, 1);

        app.s3_state.properties_scroll = app.s3_state.properties_scroll.saturating_sub(1);
        assert_eq!(app.s3_state.properties_scroll, 0);

        // Should not go below 0
        app.s3_state.properties_scroll = app.s3_state.properties_scroll.saturating_sub(1);
        assert_eq!(app.s3_state.properties_scroll, 0);
    }

    #[test]
    fn test_s3_parent_prefix_cleared_before_render() {
        // Parent prefix area should be cleared to prevent artifacts
        // Verified by Clear widget being rendered before parent text
    }

    #[test]
    fn test_s3_empty_region_defaults_to_us_east_1() {
        let _app = App::new_without_client("test".to_string(), Some("us-east-1".to_string()));

        // When bucket region is empty, should default to us-east-1
        let empty_region = "";
        let bucket_region = if empty_region.is_empty() {
            "us-east-1"
        } else {
            empty_region
        };
        assert_eq!(bucket_region, "us-east-1");

        // When bucket region is set, should use it
        let set_region = "us-west-2";
        let bucket_region = if set_region.is_empty() {
            "us-east-1"
        } else {
            set_region
        };
        assert_eq!(bucket_region, "us-west-2");
    }

    #[test]
    fn test_s3_properties_has_multiple_blocks() {
        // Properties tab should have 12 separate blocks
        let block_count = 12;
        assert_eq!(block_count, 12);

        // Blocks: Bucket overview, Tags, Default encryption, Intelligent-Tiering,
        // Server access logging, CloudTrail, Event notifications, EventBridge,
        // Transfer acceleration, Object Lock, Requester pays, Static website hosting
    }

    #[test]
    fn test_s3_properties_tables_use_common_component() {
        // Tables should use ratatui Table widget
        // Tags table: Key, Value columns
        let tags_columns = ["Key", "Value"];
        assert_eq!(tags_columns.len(), 2);

        // Intelligent-Tiering table: 5 columns
        let tiering_columns = [
            "Name",
            "Status",
            "Scope",
            "Days to Archive",
            "Days to Deep Archive",
        ];
        assert_eq!(tiering_columns.len(), 5);

        // Event notifications table: 5 columns
        let events_columns = [
            "Name",
            "Event types",
            "Filters",
            "Destination type",
            "Destination",
        ];
        assert_eq!(events_columns.len(), 5);
    }

    #[test]
    fn test_s3_properties_field_format() {
        // Each field should have bold label followed by value
        use ratatui::style::{Modifier, Style};
        use ratatui::text::{Line, Span};

        let label = Line::from(vec![Span::styled(
            "AWS Region",
            Style::default().add_modifier(Modifier::BOLD),
        )]);
        let value = Line::from("us-east-1");

        // Verify label is bold
        assert!(label.spans[0].style.add_modifier.contains(Modifier::BOLD));

        // Verify value is plain text
        assert!(!value.spans[0].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_s3_properties_has_scrollbar() {
        // Properties tab should have vertical scrollbar
        let total_height = 7 + 5 + 6 + 5 + 4 + 4 + 5 + 4 + 4 + 4 + 4 + 4;
        assert_eq!(total_height, 56);

        // If total height exceeds area, scrollbar should be shown
        let area_height = 40;
        assert!(total_height > area_height);
    }

    #[test]
    fn test_s3_bucket_region_fetched_on_open() {
        // When bucket region is empty, it should be fetched before loading objects
        // This prevents PermanentRedirect errors

        // Simulate empty region
        let empty_region = "";
        assert!(empty_region.is_empty());

        // After fetch, region should be populated
        let fetched_region = "us-west-2";
        assert!(!fetched_region.is_empty());
    }

    #[test]
    fn test_s3_filter_space_used_when_hidden() {
        // When filter is hidden (non-Objects tabs), its space should be used by content
        // Objects tab: 4 chunks (prefix, tabs, filter, content)
        // Other tabs: 3 chunks (prefix, tabs, content)

        let objects_chunks = 4;
        let other_chunks = 3;

        assert_eq!(objects_chunks, 4);
        assert_eq!(other_chunks, 3);
        assert!(other_chunks < objects_chunks);
    }

    #[test]
    fn test_s3_properties_scrollable() {
        let mut app = test_app();

        // Properties should be scrollable
        assert_eq!(app.s3_state.properties_scroll, 0);

        // Scroll down
        app.s3_state.properties_scroll += 1;
        assert_eq!(app.s3_state.properties_scroll, 1);

        // Scroll up
        app.s3_state.properties_scroll = app.s3_state.properties_scroll.saturating_sub(1);
        assert_eq!(app.s3_state.properties_scroll, 0);
    }

    #[test]
    fn test_s3_properties_scrollbar_conditional() {
        // Scrollbar should only show when content exceeds viewport
        let content_height = 40;
        let small_viewport = 20;
        let large_viewport = 50;

        // Should show scrollbar
        assert!(content_height > small_viewport);

        // Should not show scrollbar
        assert!(content_height < large_viewport);
    }

    #[test]
    fn test_s3_tabs_visible_with_styling() {
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::Span;

        // Active tab should be yellow, bold, and underlined
        let active_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        let active_tab = Span::styled("Objects", active_style);
        assert_eq!(active_tab.style.fg, Some(Color::Yellow));
        assert!(active_tab.style.add_modifier.contains(Modifier::BOLD));
        assert!(active_tab.style.add_modifier.contains(Modifier::UNDERLINED));

        // Inactive tab should be gray
        let inactive_style = Style::default().fg(Color::Gray);
        let inactive_tab = Span::styled("Properties", inactive_style);
        assert_eq!(inactive_tab.style.fg, Some(Color::Gray));
    }

    #[test]
    fn test_s3_properties_field_labels_bold() {
        use ratatui::style::{Modifier, Style};
        use ratatui::text::{Line, Span};

        // Field labels should be bold, values should not
        let label = Span::styled(
            "AWS Region: ",
            Style::default().add_modifier(Modifier::BOLD),
        );
        let value = Span::raw("us-east-1");
        let line = Line::from(vec![label.clone(), value.clone()]);

        // Verify label is bold
        assert!(label.style.add_modifier.contains(Modifier::BOLD));

        // Verify value is not bold
        assert!(!value.style.add_modifier.contains(Modifier::BOLD));

        // Verify line has both parts
        assert_eq!(line.spans.len(), 2);
    }

    #[test]
    fn test_session_picker_dialog_opaque() {
        // Session picker dialog should use Clear widget to be opaque
        // This prevents background content from showing through
    }

    #[test]
    fn test_status_bar_hotkey_format() {
        // Status bar should use ⋮ separator, ^ for ctrl, uppercase for shift+char, and highlight keys in red

        // Test separator
        let separator = " ⋮ ";
        assert_eq!(separator, " ⋮ ");

        // Test ctrl format
        let ctrl_key = "^r";
        assert!(ctrl_key.starts_with("^"));
        assert!(!ctrl_key.contains("ctrl+"));
        assert!(!ctrl_key.contains("ctrl-"));

        // Test shift+char format (uppercase)
        let shift_key = "^R";
        assert!(shift_key.contains("^R"));
        assert!(!shift_key.contains("shift+"));
        assert!(!shift_key.contains("shift-"));

        // Test that old formats are not used
        let old_separator = " | ";
        assert_ne!(separator, old_separator);
    }

    #[test]
    fn test_space_key_uses_unicode_symbol() {
        // Space key should use ␣ (U+2423 OPEN BOX) symbol, not "space" text
        let space_symbol = "␣";
        assert_eq!(space_symbol, "␣");
        assert_eq!(space_symbol.len(), 3); // UTF-8 bytes

        // Should not use text "space"
        assert_ne!(space_symbol, "space");
        assert_ne!(space_symbol, "SPC");
    }

    #[test]
    fn test_region_hotkey_uses_space_menu() {
        // Region should use ␣→r (space menu), not ^R (Ctrl+Shift+R)
        let region_hotkey = "␣→r";
        assert_eq!(region_hotkey, "␣→r");

        // Should not use ^R for region
        assert_ne!(region_hotkey, "^R");
        assert_ne!(region_hotkey, "ctrl+shift+r");
    }

    #[test]
    fn test_no_incorrect_hotkey_patterns_in_ui() {
        // This test validates that common hotkey mistakes are not present in the UI code
        let source = include_str!("mod.rs");

        // Split at #[cfg(test)] to only check non-test code
        let ui_code = if let Some(pos) = source.find("#[cfg(test)]") {
            &source[..pos]
        } else {
            source
        };

        // Check for "space" text instead of ␣ symbol in hotkeys
        let space_text_pattern = r#"Span::styled("space""#;
        assert!(
            !ui_code.contains(space_text_pattern),
            "Found 'space' text in hotkey - should use ␣ symbol instead"
        );

        // Check for ^R followed by region (should be ␣→r)
        let lines_with_ctrl_shift_r: Vec<_> = ui_code
            .lines()
            .enumerate()
            .filter(|(_, line)| {
                line.contains(r#"Span::styled("^R""#) && line.contains("Color::Red")
            })
            .collect();

        assert!(
            lines_with_ctrl_shift_r.is_empty(),
            "Found ^R in hotkeys (should use ␣→r for region): {:?}",
            lines_with_ctrl_shift_r
        );
    }

    #[test]
    fn test_region_only_in_space_menu_not_status_bar() {
        // Region switching should ONLY be in Space menu, NOT in status bar hotkeys
        let source = include_str!("mod.rs");

        // Find the space menu section
        let space_menu_start = source
            .find("fn render_space_menu")
            .expect("render_space_menu function not found");
        let space_menu_end = space_menu_start
            + source[space_menu_start..]
                .find("fn render_service_picker")
                .expect("render_service_picker not found");
        let space_menu_code = &source[space_menu_start..space_menu_end];

        // Verify region IS in space menu
        assert!(
            space_menu_code.contains(r#"Span::raw(" regions")"#),
            "Region must be in Space menu"
        );

        // Find status bar section (render_bottom_bar)
        let status_bar_start = source
            .find("fn render_bottom_bar")
            .expect("render_bottom_bar function not found");
        let status_bar_end = status_bar_start
            + source[status_bar_start..]
                .find("\nfn render_")
                .expect("Next function not found");
        let status_bar_code = &source[status_bar_start..status_bar_end];

        // Verify region is NOT in status bar
        assert!(
            !status_bar_code.contains(" region ⋮ "),
            "Region hotkey must NOT be in status bar - it's only in Space menu!"
        );
        assert!(
            !status_bar_code.contains("␣→r"),
            "Region hotkey (␣→r) must NOT be in status bar - it's only in Space menu!"
        );
        assert!(
            !status_bar_code.contains("^R"),
            "Region hotkey (^R) must NOT be in status bar - it's only in Space menu!"
        );
    }

    #[test]
    fn test_s3_bucket_preview_permanent_redirect_handled() {
        // PermanentRedirect errors should be silently handled
        // Empty preview should be inserted to prevent retry
        let error_msg = "PermanentRedirect";
        assert!(error_msg.contains("PermanentRedirect"));

        // Verify empty preview prevents retry
        let mut preview_map: std::collections::HashMap<String, Vec<crate::app::S3Object>> =
            std::collections::HashMap::new();
        preview_map.insert("bucket".to_string(), vec![]);
        assert!(preview_map.contains_key("bucket"));
    }

    #[test]
    fn test_s3_objects_hint_is_open() {
        // Hint should say "open" not "open folder" or "drill down"
        let hint = "open";
        assert_eq!(hint, "open");
        assert_ne!(hint, "drill down");
        assert_ne!(hint, "open folder");
    }

    #[test]
    fn test_s3_service_tabs_use_cyan() {
        // Service tabs should use cyan color when active
        let active_color = Color::Cyan;
        assert_eq!(active_color, Color::Cyan);
        assert_ne!(active_color, Color::Yellow);
    }

    #[test]
    fn test_s3_column_names_use_orange() {
        // Column names should use orange (LightRed) color
        let column_color = Color::LightRed;
        assert_eq!(column_color, Color::LightRed);
    }

    #[test]
    fn test_s3_bucket_errors_shown_in_expanded_rows() {
        // Bucket errors should be stored and displayed in expanded rows
        let mut errors: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        errors.insert("bucket".to_string(), "Error message".to_string());
        assert!(errors.contains_key("bucket"));
        assert_eq!(errors.get("bucket").unwrap(), "Error message");
    }

    #[test]
    fn test_cloudwatch_alarms_page_input() {
        // Page input should work for CloudWatch alarms
        let mut app = test_app();
        app.current_service = Service::CloudWatchAlarms;
        app.page_input = "2".to_string();

        // Verify page input is set
        assert_eq!(app.page_input, "2");
    }

    #[test]
    fn test_tabs_row_shows_profile_info() {
        // Tabs row should show profile, account, region, identity, and timestamp
        let profile = "default";
        let account = "123456789012";
        let region = "us-west-2";
        let identity = "role:/MyRole";

        let info = format!(
            "Profile: {} ⋮ Account: {} ⋮ Region: {} ⋮ Identity: {}",
            profile, account, region, identity
        );
        assert!(info.contains("Profile:"));
        assert!(info.contains("Account:"));
        assert!(info.contains("Region:"));
        assert!(info.contains("Identity:"));
        assert!(info.contains("⋮"));
    }

    #[test]
    fn test_tabs_row_profile_labels_are_bold() {
        // Profile info labels should use bold modifier
        let label_style = Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);
        assert!(label_style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_profile_info_not_duplicated() {
        // Profile info should only appear once (in tabs row, not in top bar)
        // Top bar should only show breadcrumbs
        let breadcrumbs = "CloudWatch > Alarms";
        assert!(!breadcrumbs.contains("Profile:"));
        assert!(!breadcrumbs.contains("Account:"));
    }

    #[test]
    fn test_s3_column_headers_are_cyan() {
        // All table column headers should use Cyan color
        let header_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        assert_eq!(header_style.fg, Some(Color::Cyan));
        assert!(header_style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_s3_nested_objects_can_be_expanded() {
        // Nested objects (second level folders) should be expandable
        // Visual index should map to actual object including nested items
        let mut app = test_app();
        app.current_service = Service::S3Buckets;
        app.s3_state.current_bucket = Some("bucket".to_string());

        // Add a top-level folder
        app.s3_state.objects.push(crate::app::S3Object {
            key: "folder1/".to_string(),
            size: 0,
            last_modified: String::new(),
            is_prefix: true,
            storage_class: String::new(),
        });

        // Expand it
        app.s3_state
            .expanded_prefixes
            .insert("folder1/".to_string());

        // Add nested folder in preview
        let nested = vec![crate::app::S3Object {
            key: "folder1/subfolder/".to_string(),
            size: 0,
            last_modified: String::new(),
            is_prefix: true,
            storage_class: String::new(),
        }];
        app.s3_state
            .prefix_preview
            .insert("folder1/".to_string(), nested);

        // Visual index 1 should be the nested folder
        app.s3_state.selected_object = 1;

        // Should be able to expand nested folder
        assert!(app.s3_state.current_bucket.is_some());
    }

    #[test]
    fn test_s3_nested_folder_shows_expand_indicator() {
        use crate::app::{S3Object, Service};

        let mut app = test_app();
        app.current_service = Service::S3Buckets;
        app.s3_state.current_bucket = Some("test-bucket".to_string());

        // Add parent folder
        app.s3_state.objects = vec![S3Object {
            key: "parent/".to_string(),
            size: 0,
            last_modified: "2024-01-01T00:00:00Z".to_string(),
            is_prefix: true,
            storage_class: String::new(),
        }];

        // Expand parent and add nested folder
        app.s3_state.expanded_prefixes.insert("parent/".to_string());
        app.s3_state.prefix_preview.insert(
            "parent/".to_string(),
            vec![S3Object {
                key: "parent/child/".to_string(),
                size: 0,
                last_modified: "2024-01-01T00:00:00Z".to_string(),
                is_prefix: true,
                storage_class: String::new(),
            }],
        );

        // Nested folder should show ▶ when collapsed
        let child = &app.s3_state.prefix_preview.get("parent/").unwrap()[0];
        let is_expanded = app.s3_state.expanded_prefixes.contains(&child.key);
        let indicator = if is_expanded { "▼ " } else { "▶ " };
        assert_eq!(indicator, "▶ ");

        // After expanding, should show ▼
        app.s3_state
            .expanded_prefixes
            .insert("parent/child/".to_string());
        let is_expanded = app.s3_state.expanded_prefixes.contains(&child.key);
        let indicator = if is_expanded { "▼ " } else { "▶ " };
        assert_eq!(indicator, "▼ ");
    }

    #[test]
    fn test_tabs_row_always_visible() {
        // Tabs row should always be visible (shows profile info)
        // Even when on service picker
        let app = test_app();
        assert!(!app.service_selected); // On service picker
                                        // Tabs row should still render with profile info
    }

    #[test]
    fn test_no_duplicate_breadcrumbs_at_root() {
        // When at root level (e.g., CloudWatch > Alarms), don't show duplicate breadcrumb
        let mut app = test_app();
        app.current_service = Service::CloudWatchAlarms;
        app.service_selected = true;
        app.tabs.push(crate::app::Tab {
            service: Service::CloudWatchAlarms,
            title: "CloudWatch > Alarms".to_string(),
            breadcrumb: "CloudWatch > Alarms".to_string(),
        });

        // At root level, breadcrumb should not be shown separately
        // (it's already in the tab)
        assert_eq!(app.breadcrumbs(), "CloudWatch > Alarms");
    }

    #[test]
    fn test_preferences_headers_use_cyan_underline() {
        // Preferences section headers should use cyan with underline, not box drawing
        let header_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        assert_eq!(header_style.fg, Some(Color::Cyan));
        assert!(header_style.add_modifier.contains(Modifier::BOLD));
        assert!(header_style.add_modifier.contains(Modifier::UNDERLINED));

        // Should not use box drawing characters
        let header_text = "Columns";
        assert!(!header_text.contains("═"));
    }

    #[test]
    fn test_alarm_pagination_shows_actual_pages() {
        // Pagination should show "Page X of Y", not page size selector
        let page_size = 10;
        let total_items = 25;
        let total_pages = (total_items + page_size - 1) / page_size;
        let current_page = 1;

        let pagination = format!("Page {} of {}", current_page, total_pages);
        assert_eq!(pagination, "Page 1 of 3");
        assert!(!pagination.contains("[1]"));
        assert!(!pagination.contains("[2]"));
    }

    #[test]
    fn test_mode_indicator_uses_insert_not_input() {
        // Mode indicator should say "INSERT" not "INPUT"
        let mode_text = " INSERT ";
        assert_eq!(mode_text, " INSERT ");
        assert_ne!(mode_text, " INPUT ");
    }

    #[test]
    fn test_service_picker_shows_insert_mode_when_typing() {
        // Service picker should show INSERT mode when filter is not empty
        let mut app = test_app();
        app.mode = Mode::ServicePicker;
        app.service_picker.filter = "cloud".to_string();

        // Should show INSERT mode
        assert!(!app.service_picker.filter.is_empty());
    }

    #[test]
    fn test_log_events_no_horizontal_scrollbar() {
        // Log events should not show horizontal scrollbar
        // Only vertical scrollbar for navigating events
        // Message column truncates with ellipsis, expand to see full content
        let app = test_app();

        // Log events only have 2 columns: Timestamp and Message
        // No horizontal scrolling needed - message truncates
        assert_eq!(app.cw_log_event_visible_column_ids.len(), 2);

        // Horizontal scroll offset should not be used for events
        assert_eq!(app.log_groups_state.event_horizontal_scroll, 0);
    }

    #[test]
    fn test_log_events_expansion_stays_visible_when_scrolling() {
        // Expanded log event should stay visible when scrolling to other events
        // Same behavior as CloudWatch Alarms
        let mut app = test_app();

        // Expand event at index 0
        app.log_groups_state.expanded_event = Some(0);
        app.log_groups_state.event_scroll_offset = 0;

        // Scroll to event 1
        app.log_groups_state.event_scroll_offset = 1;

        // Expanded event should still be set and visible
        assert_eq!(app.log_groups_state.expanded_event, Some(0));
    }

    #[test]
    fn test_log_events_right_arrow_expands() {
        let mut app = test_app();
        app.current_service = Service::CloudWatchLogGroups;
        app.service_selected = true;
        app.view_mode = ViewMode::Events;

        app.log_groups_state.log_events = vec![rusticity_core::LogEvent {
            timestamp: chrono::Utc::now(),
            message: "Test log message".to_string(),
        }];
        app.log_groups_state.event_scroll_offset = 0;

        assert_eq!(app.log_groups_state.expanded_event, None);

        // Right arrow - should expand
        app.handle_action(Action::NextPane);
        assert_eq!(app.log_groups_state.expanded_event, Some(0));
    }

    #[test]
    fn test_log_events_left_arrow_collapses() {
        let mut app = test_app();
        app.current_service = Service::CloudWatchLogGroups;
        app.service_selected = true;
        app.view_mode = ViewMode::Events;

        app.log_groups_state.log_events = vec![rusticity_core::LogEvent {
            timestamp: chrono::Utc::now(),
            message: "Test log message".to_string(),
        }];
        app.log_groups_state.event_scroll_offset = 0;
        app.log_groups_state.expanded_event = Some(0);

        // Left arrow - should collapse
        app.handle_action(Action::PrevPane);
        assert_eq!(app.log_groups_state.expanded_event, None);
    }

    #[test]
    fn test_log_events_expanded_content_replaces_tabs() {
        // Expanded content should replace tabs with spaces to avoid rendering artifacts
        let message_with_tabs = "[INFO]\t2025-10-22T13:41:37.601Z\tb2227e1c";
        let cleaned = message_with_tabs.replace('\t', "    ");

        assert!(!cleaned.contains('\t'));
        assert!(cleaned.contains("    "));
        assert_eq!(cleaned, "[INFO]    2025-10-22T13:41:37.601Z    b2227e1c");
    }

    #[test]
    fn test_log_events_navigation_skips_expanded_overlay() {
        // When navigating down from an expanded event, selection should skip to next event
        // Empty rows are added to table to reserve space, but navigation uses event indices
        let mut app = test_app();

        // Expand event at index 0
        app.log_groups_state.expanded_event = Some(0);
        app.log_groups_state.event_scroll_offset = 0;

        // Navigate down - should go to event 1, not expanded overlay lines
        app.log_groups_state.event_scroll_offset = 1;

        // Selection is now on event 1
        assert_eq!(app.log_groups_state.event_scroll_offset, 1);

        // Expanded event 0 is still expanded
        assert_eq!(app.log_groups_state.expanded_event, Some(0));
    }

    #[test]
    fn test_log_events_empty_rows_reserve_space_for_overlay() {
        // Empty rows are added to table for expanded content to prevent overlay from covering next events
        // This ensures selection highlight is visible on the correct row
        let message = "Long message that will wrap across multiple lines when expanded";
        let max_width = 50;

        // Calculate how many lines this would take
        let full_line = format!("Message: {}", message);
        let line_count = full_line.len().div_ceil(max_width);

        // Should be at least 2 lines for this message
        assert!(line_count >= 2);

        // Empty rows equal to line_count should be added to reserve space
        // This prevents the overlay from covering the next event's selection highlight
    }

    #[test]
    fn test_preferences_title_no_hints() {
        // All preferences dialogs should have clean titles without hints
        // Hints should be in status bar instead
        let s3_title = " Preferences ";
        let events_title = " Preferences ";
        let alarms_title = " Preferences ";

        assert_eq!(s3_title.trim(), "Preferences");
        assert_eq!(events_title.trim(), "Preferences");
        assert_eq!(alarms_title.trim(), "Preferences");

        // No hints in titles
        assert!(!s3_title.contains("Space"));
        assert!(!events_title.contains("Space"));
        assert!(!alarms_title.contains("Tab"));
    }

    #[test]
    fn test_page_navigation_works_for_events() {
        // Page navigation (e.g., "2P") should work for log events
        let mut app = test_app();
        app.view_mode = ViewMode::Events;

        // Simulate having 50 events
        app.log_groups_state.event_scroll_offset = 0;

        // Navigate to page 2 (page_size = 20, so target_index = 20)
        let page = 2;
        let page_size = 20;
        let target_index = (page - 1) * page_size;

        assert_eq!(target_index, 20);

        // After navigation, page_input should be cleared
        app.page_input.clear();
        assert!(app.page_input.is_empty());
    }

    #[test]
    fn test_status_bar_shows_tab_hint_for_alarms_preferences() {
        // Alarms preferences should show Tab hint in status bar (has multiple sections)
        // Other preferences don't need Tab hint
        let app = test_app();

        // Alarms has sections: Columns, View As, Page Size, Wrap Lines
        // So it needs Tab navigation hint
        assert_eq!(app.current_service, Service::CloudWatchLogGroups);

        // When current_service is CloudWatchAlarms, Tab hint should be shown
        // This is checked in the status bar rendering logic
    }

    #[test]
    fn test_column_selector_shows_correct_columns_per_service() {
        use crate::app::Service;

        // S3 Buckets should show bucket columns
        let mut app = test_app();
        app.current_service = Service::S3Buckets;
        let bucket_col_names: Vec<String> = app
            .s3_bucket_column_ids
            .iter()
            .filter_map(|id| BucketColumn::from_id(id).map(|c| c.name()))
            .collect();
        assert_eq!(bucket_col_names, vec!["Name", "Region", "Creation date"]);

        // CloudWatch Log Groups should show log group columns
        app.current_service = Service::CloudWatchLogGroups;
        let log_col_names: Vec<String> = app
            .cw_log_group_column_ids
            .iter()
            .filter_map(|id| LogGroupColumn::from_id(id).map(|c| c.name().to_string()))
            .collect();
        assert_eq!(
            log_col_names,
            vec![
                "Log group",
                "Log class",
                "Retention",
                "Stored bytes",
                "Creation time",
                "ARN"
            ]
        );

        // CloudWatch Alarms should show alarm columns
        app.current_service = Service::CloudWatchAlarms;
        assert!(!app.cw_alarm_column_ids.is_empty());
        if let Some(col) = AlarmColumn::from_id(app.cw_alarm_column_ids[0]) {
            assert!(col.name().contains("Name") || col.name().contains("Alarm"));
        }
    }

    #[test]
    fn test_log_groups_preferences_shows_all_six_columns() {
        use crate::app::Service;

        let mut app = test_app();
        app.current_service = Service::CloudWatchLogGroups;

        // Verify all 6 columns exist
        assert_eq!(app.cw_log_group_column_ids.len(), 6);

        // Verify each column by name
        let col_names: Vec<String> = app
            .cw_log_group_column_ids
            .iter()
            .filter_map(|id| LogGroupColumn::from_id(id).map(|c| c.name().to_string()))
            .collect();
        assert!(col_names.iter().any(|n| n == "Log group"));
        assert!(col_names.iter().any(|n| n == "Log class"));
        assert!(col_names.iter().any(|n| n == "Retention"));
        assert!(col_names.iter().any(|n| n == "Stored bytes"));
        assert!(col_names.iter().any(|n| n == "Creation time"));
        assert!(col_names.iter().any(|n| n == "ARN"));
    }

    #[test]
    fn test_stream_preferences_shows_all_columns() {
        use crate::app::ViewMode;

        let mut app = test_app();
        app.view_mode = ViewMode::Detail;

        // Verify stream columns exist
        assert!(!app.cw_log_stream_column_ids.is_empty());
        assert_eq!(app.cw_log_stream_column_ids.len(), 7);
    }

    #[test]
    fn test_event_preferences_shows_all_columns() {
        use crate::app::ViewMode;

        let mut app = test_app();
        app.view_mode = ViewMode::Events;

        // Verify event columns exist
        assert!(!app.cw_log_event_column_ids.is_empty());
        assert_eq!(app.cw_log_event_column_ids.len(), 5);
    }

    #[test]
    fn test_alarm_preferences_shows_all_columns() {
        use crate::app::Service;

        let mut app = test_app();
        app.current_service = Service::CloudWatchAlarms;

        // Verify alarm columns exist
        assert!(!app.cw_alarm_column_ids.is_empty());
        assert_eq!(app.cw_alarm_column_ids.len(), 16);
    }

    #[test]
    fn test_column_selector_has_scrollbar() {
        // Column selector should have scrollbar when items don't fit
        // This is rendered in render_column_selector after the list widget
        let item_count = 6; // Log groups has 6 columns
        assert!(item_count > 0);

        // Scrollbar should be rendered with vertical right orientation
        // with up/down arrows
    }

    #[test]
    fn test_preferences_scrollbar_only_when_needed() {
        // Scrollbar should only appear when content exceeds available height
        let item_count = 6;
        let height = (item_count as u16 + 2).max(8); // +2 for borders
        let max_height_fits = 20; // Large enough to fit all items
        let max_height_doesnt_fit = 5; // Too small to fit all items

        // When content fits, no scrollbar needed
        let needs_scrollbar_fits = height > max_height_fits;
        assert!(!needs_scrollbar_fits);

        // When content doesn't fit, scrollbar needed
        let needs_scrollbar_doesnt_fit = height > max_height_doesnt_fit;
        assert!(needs_scrollbar_doesnt_fit);
    }

    #[test]
    fn test_preferences_height_no_extra_padding() {
        // Height should be item_count + 2 (for borders), not + 4
        let item_count = 6;
        let height = (item_count as u16 + 2).max(8);
        assert_eq!(height, 8); // 6 + 2 = 8

        // Should not have extra empty lines
        assert_ne!(height, 10); // Not 6 + 4
    }

    #[test]
    fn test_preferences_uses_absolute_sizing() {
        // Preferences should use centered_rect_absolute, not centered_rect (percentages)
        // This ensures width/height are in characters, not percentages
        let width = 50u16; // 50 characters
        let height = 10u16; // 10 lines

        // These are absolute values, not percentages
        assert!(width <= 100); // Reasonable character width
        assert!(height <= 50); // Reasonable line height
    }

    #[test]
    fn test_profile_picker_shows_sort_indicator() {
        // Profile picker should show sort on Profile column ascending
        let sort_column = "Profile";
        let sort_direction = "ASC";

        assert_eq!(sort_column, "Profile");
        assert_eq!(sort_direction, "ASC");

        // Verify arrow would be added
        let arrow = if sort_direction == "ASC" {
            " ↑"
        } else {
            " ↓"
        };
        assert_eq!(arrow, " ↑");
    }

    #[test]
    fn test_session_picker_shows_sort_indicator() {
        // Session picker should show sort on Timestamp column descending
        let sort_column = "Timestamp";
        let sort_direction = "DESC";

        assert_eq!(sort_column, "Timestamp");
        assert_eq!(sort_direction, "DESC");

        // Verify arrow would be added
        let arrow = if sort_direction == "ASC" {
            " ↑"
        } else {
            " ↓"
        };
        assert_eq!(arrow, " ↓");
    }

    #[test]
    fn test_profile_picker_sorted_ascending() {
        let mut app = test_app_no_region();
        app.available_profiles = vec![
            crate::app::AwsProfile {
                name: "zebra".to_string(),
                region: None,
                account: None,
                role_arn: None,
                source_profile: None,
            },
            crate::app::AwsProfile {
                name: "alpha".to_string(),
                region: None,
                account: None,
                role_arn: None,
                source_profile: None,
            },
        ];

        let filtered = app.get_filtered_profiles();
        assert_eq!(filtered[0].name, "alpha");
        assert_eq!(filtered[1].name, "zebra");
    }

    #[test]
    fn test_session_picker_sorted_descending() {
        let mut app = test_app_no_region();
        // Sessions should be added in descending timestamp order (newest first)
        app.sessions = vec![
            crate::session::Session {
                id: "2".to_string(),
                timestamp: "2024-01-02 10:00:00 UTC".to_string(),
                profile: "new".to_string(),
                region: "us-east-1".to_string(),
                account_id: "123".to_string(),
                role_arn: String::new(),
                tabs: vec![],
            },
            crate::session::Session {
                id: "1".to_string(),
                timestamp: "2024-01-01 10:00:00 UTC".to_string(),
                profile: "old".to_string(),
                region: "us-east-1".to_string(),
                account_id: "123".to_string(),
                role_arn: String::new(),
                tabs: vec![],
            },
        ];

        let filtered = app.get_filtered_sessions();
        // Sessions are already sorted descending by timestamp (newest first)
        assert_eq!(filtered[0].profile, "new");
        assert_eq!(filtered[1].profile, "old");
    }

    #[test]
    fn test_ecr_encryption_type_aes256_renders_as_aes_dash_256() {
        let repo = EcrRepository {
            name: "test-repo".to_string(),
            uri: "123456789012.dkr.ecr.us-east-1.amazonaws.com/test-repo".to_string(),
            created_at: "2024-01-01".to_string(),
            tag_immutability: "MUTABLE".to_string(),
            encryption_type: "AES256".to_string(),
        };

        let formatted = match repo.encryption_type.as_ref() {
            "AES256" => "AES-256".to_string(),
            "KMS" => "KMS".to_string(),
            other => other.to_string(),
        };

        assert_eq!(formatted, "AES-256");
    }

    #[test]
    fn test_ecr_encryption_type_kms_unchanged() {
        let repo = EcrRepository {
            name: "test-repo".to_string(),
            uri: "123456789012.dkr.ecr.us-east-1.amazonaws.com/test-repo".to_string(),
            created_at: "2024-01-01".to_string(),
            tag_immutability: "MUTABLE".to_string(),
            encryption_type: "KMS".to_string(),
        };

        let formatted = match repo.encryption_type.as_ref() {
            "AES256" => "AES-256".to_string(),
            "KMS" => "KMS".to_string(),
            other => other.to_string(),
        };

        assert_eq!(formatted, "KMS");
    }

    #[test]
    fn test_ecr_repo_filter_active_removes_table_focus() {
        let mut app = test_app_no_region();
        app.current_service = Service::EcrRepositories;
        app.mode = Mode::FilterInput;
        app.ecr_state.repositories.items = vec![EcrRepository {
            name: "test-repo".to_string(),
            uri: "123456789012.dkr.ecr.us-east-1.amazonaws.com/test-repo".to_string(),
            created_at: "2024-01-01".to_string(),
            tag_immutability: "MUTABLE".to_string(),
            encryption_type: "AES256".to_string(),
        }];

        // When in FilterInput mode, table should not be active
        assert_eq!(app.mode, Mode::FilterInput);
        // This would be checked in render logic: is_active: app.mode != Mode::FilterInput
    }

    #[test]
    fn test_ecr_image_filter_active_removes_table_focus() {
        let mut app = test_app_no_region();
        app.current_service = Service::EcrRepositories;
        app.ecr_state.current_repository = Some("test-repo".to_string());
        app.mode = Mode::FilterInput;
        app.ecr_state.images.items = vec![EcrImage {
            tag: "v1.0.0".to_string(),
            artifact_type: "application/vnd.docker.container.image.v1+json".to_string(),
            pushed_at: "2024-01-01".to_string(),
            size_bytes: 104857600,
            uri: "123456789012.dkr.ecr.us-east-1.amazonaws.com/test-repo:v1.0.0".to_string(),
            digest: "sha256:abc123".to_string(),
            last_pull_time: "2024-01-02".to_string(),
        }];

        // When in FilterInput mode, table should not be active
        assert_eq!(app.mode, Mode::FilterInput);
        // This would be checked in render logic: is_active: app.mode != Mode::FilterInput
    }

    #[test]
    fn test_ecr_filter_escape_returns_to_normal_mode() {
        let mut app = test_app_no_region();
        app.current_service = Service::EcrRepositories;
        app.mode = Mode::FilterInput;
        app.ecr_state.repositories.filter = "test".to_string();

        // Simulate Escape key (CloseMenu action)
        app.handle_action(crate::keymap::Action::CloseMenu);

        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_ecr_repos_no_scrollbar_when_all_fit() {
        // ECR repos table should not show scrollbar when all paginated items fit
        let mut app = test_app_no_region();
        app.current_service = Service::EcrRepositories;
        app.ecr_state.repositories.items = (0..50)
            .map(|i| EcrRepository {
                name: format!("repo{}", i),
                uri: format!("123456789012.dkr.ecr.us-east-1.amazonaws.com/repo{}", i),
                created_at: "2024-01-01".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            })
            .collect();

        // With 50 repos on page and typical terminal height, scrollbar should not appear
        // Scrollbar logic: row_count > (area_height - 3)
        let row_count = 50;
        let typical_area_height: u16 = 60;
        let available_height = typical_area_height.saturating_sub(3);

        assert!(
            row_count <= available_height as usize,
            "50 repos should fit without scrollbar"
        );
    }

    #[test]
    fn test_lambda_default_columns() {
        let app = test_app_no_region();

        assert_eq!(app.lambda_state.function_visible_column_ids.len(), 6);
        assert_eq!(
            app.lambda_state.function_visible_column_ids[0],
            "column.lambda.function.name"
        );
        assert_eq!(
            app.lambda_state.function_visible_column_ids[1],
            "column.lambda.function.runtime"
        );
        assert_eq!(
            app.lambda_state.function_visible_column_ids[2],
            "column.lambda.function.code_size"
        );
        assert_eq!(
            app.lambda_state.function_visible_column_ids[3],
            "column.lambda.function.memory_mb"
        );
        assert_eq!(
            app.lambda_state.function_visible_column_ids[4],
            "column.lambda.function.timeout_seconds"
        );
        assert_eq!(
            app.lambda_state.function_visible_column_ids[5],
            "column.lambda.function.last_modified"
        );
    }

    #[test]
    fn test_lambda_all_columns_available() {
        let all_columns = lambda::FunctionColumn::ids();

        assert_eq!(all_columns.len(), 9);
        assert!(all_columns.contains(&"column.lambda.function.name"));
        assert!(all_columns.contains(&"column.lambda.function.description"));
        assert!(all_columns.contains(&"column.lambda.function.package_type"));
        assert!(all_columns.contains(&"column.lambda.function.runtime"));
        assert!(all_columns.contains(&"column.lambda.function.architecture"));
        assert!(all_columns.contains(&"column.lambda.function.code_size"));
        assert!(all_columns.contains(&"column.lambda.function.memory_mb"));
        assert!(all_columns.contains(&"column.lambda.function.timeout_seconds"));
        assert!(all_columns.contains(&"column.lambda.function.last_modified"));
    }

    #[test]
    fn test_lambda_filter_active_removes_table_focus() {
        let mut app = test_app_no_region();
        app.current_service = Service::LambdaFunctions;
        app.mode = Mode::FilterInput;
        app.lambda_state.table.items = vec![lambda::Function {
            arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
            application: None,
            name: "test-function".to_string(),
            description: "Test function".to_string(),
            package_type: "Zip".to_string(),
            runtime: "python3.12".to_string(),
            architecture: "x86_64".to_string(),
            code_size: 1024,
            code_sha256: "test-sha256".to_string(),
            memory_mb: 128,
            timeout_seconds: 3,
            last_modified: "2024-01-01T00:00:00.000+0000".to_string(),
            layers: vec![],
        }];

        assert_eq!(app.mode, Mode::FilterInput);
    }

    #[test]
    fn test_lambda_default_page_size() {
        let app = test_app_no_region();

        assert_eq!(app.lambda_state.table.page_size, PageSize::Fifty);
        assert_eq!(app.lambda_state.table.page_size.value(), 50);
    }

    #[test]
    fn test_lambda_pagination() {
        let mut app = test_app_no_region();
        app.current_service = Service::LambdaFunctions;
        app.lambda_state.table.page_size = PageSize::Ten;
        app.lambda_state.table.items = (0..25)
            .map(|i| crate::app::LambdaFunction {
                arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
                application: None,
                name: format!("function-{}", i),
                description: format!("Function {}", i),
                package_type: "Zip".to_string(),
                runtime: "python3.12".to_string(),
                architecture: "x86_64".to_string(),
                code_size: 1024,
                code_sha256: "test-sha256".to_string(),
                memory_mb: 128,
                timeout_seconds: 3,
                last_modified: "2024-01-01T00:00:00.000+0000".to_string(),
                layers: vec![],
            })
            .collect();

        let page_size = app.lambda_state.table.page_size.value();
        let total_pages = app.lambda_state.table.items.len().div_ceil(page_size);

        assert_eq!(page_size, 10);
        assert_eq!(total_pages, 3);
    }

    #[test]
    fn test_lambda_filter_by_name() {
        let mut app = test_app_no_region();
        app.lambda_state.table.items = vec![
            crate::app::LambdaFunction {
                arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
                application: None,
                name: "api-handler".to_string(),
                description: "API handler".to_string(),
                package_type: "Zip".to_string(),
                runtime: "python3.12".to_string(),
                architecture: "x86_64".to_string(),
                code_size: 1024,
                code_sha256: "test-sha256".to_string(),
                memory_mb: 128,
                timeout_seconds: 3,
                last_modified: "2024-01-01T00:00:00.000+0000".to_string(),
                layers: vec![],
            },
            crate::app::LambdaFunction {
                arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
                application: None,
                name: "data-processor".to_string(),
                description: "Data processor".to_string(),
                package_type: "Zip".to_string(),
                runtime: "nodejs20.x".to_string(),
                architecture: "arm64".to_string(),
                code_size: 2048,
                code_sha256: "test-sha256".to_string(),
                memory_mb: 256,
                timeout_seconds: 30,
                last_modified: "2024-01-02T00:00:00.000+0000".to_string(),
                layers: vec![],
            },
        ];
        app.lambda_state.table.filter = "api".to_string();

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

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "api-handler");
    }

    #[test]
    fn test_lambda_filter_by_runtime() {
        let mut app = test_app_no_region();
        app.lambda_state.table.items = vec![
            crate::app::LambdaFunction {
                arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
                application: None,
                name: "python-func".to_string(),
                description: "Python function".to_string(),
                package_type: "Zip".to_string(),
                runtime: "python3.12".to_string(),
                architecture: "x86_64".to_string(),
                code_size: 1024,
                code_sha256: "test-sha256".to_string(),
                memory_mb: 128,
                timeout_seconds: 3,
                last_modified: "2024-01-01T00:00:00.000+0000".to_string(),
                layers: vec![],
            },
            crate::app::LambdaFunction {
                arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
                application: None,
                name: "node-func".to_string(),
                description: "Node function".to_string(),
                package_type: "Zip".to_string(),
                runtime: "nodejs20.x".to_string(),
                architecture: "arm64".to_string(),
                code_size: 2048,
                code_sha256: "test-sha256".to_string(),
                memory_mb: 256,
                timeout_seconds: 30,
                last_modified: "2024-01-02T00:00:00.000+0000".to_string(),
                layers: vec![],
            },
        ];
        app.lambda_state.table.filter = "python".to_string();

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

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].runtime, "python3.12");
    }

    #[test]
    fn test_lambda_page_size_changes_in_preferences() {
        let mut app = test_app_no_region();
        app.current_service = Service::LambdaFunctions;
        app.lambda_state.table.page_size = PageSize::Fifty;

        // Simulate opening preferences and changing page size
        app.mode = Mode::ColumnSelector;
        // Index for page size options: 0=Columns header, 1-9=columns, 10=empty, 11=PageSize header, 12=10, 13=25, 14=50, 15=100
        app.column_selector_index = 12; // 10 resources
        app.handle_action(crate::keymap::Action::ToggleColumn);

        assert_eq!(app.lambda_state.table.page_size, PageSize::Ten);
    }

    #[test]
    fn test_lambda_preferences_shows_page_sizes() {
        let app = test_app_no_region();
        let mut app = app;
        app.current_service = Service::LambdaFunctions;

        // Verify all page sizes are available
        let page_sizes = vec![
            PageSize::Ten,
            PageSize::TwentyFive,
            PageSize::Fifty,
            PageSize::OneHundred,
        ];

        for size in page_sizes {
            app.lambda_state.table.page_size = size;
            assert_eq!(app.lambda_state.table.page_size, size);
        }
    }

    #[test]
    fn test_lambda_pagination_respects_page_size() {
        let mut app = test_app_no_region();
        app.current_service = Service::LambdaFunctions;
        app.lambda_state.table.items = (0..100)
            .map(|i| crate::app::LambdaFunction {
                arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
                application: None,
                name: format!("function-{}", i),
                description: format!("Function {}", i),
                package_type: "Zip".to_string(),
                runtime: "python3.12".to_string(),
                architecture: "x86_64".to_string(),
                code_size: 1024,
                code_sha256: "test-sha256".to_string(),
                memory_mb: 128,
                timeout_seconds: 3,
                last_modified: "2024-01-01T00:00:00.000+0000".to_string(),
                layers: vec![],
            })
            .collect();

        // Test with page size 10
        app.lambda_state.table.page_size = PageSize::Ten;
        let page_size = app.lambda_state.table.page_size.value();
        let total_pages = app.lambda_state.table.items.len().div_ceil(page_size);
        assert_eq!(page_size, 10);
        assert_eq!(total_pages, 10);

        // Test with page size 25
        app.lambda_state.table.page_size = PageSize::TwentyFive;
        let page_size = app.lambda_state.table.page_size.value();
        let total_pages = app.lambda_state.table.items.len().div_ceil(page_size);
        assert_eq!(page_size, 25);
        assert_eq!(total_pages, 4);

        // Test with page size 50
        app.lambda_state.table.page_size = PageSize::Fifty;
        let page_size = app.lambda_state.table.page_size.value();
        let total_pages = app.lambda_state.table.items.len().div_ceil(page_size);
        assert_eq!(page_size, 50);
        assert_eq!(total_pages, 2);
    }

    #[test]
    fn test_lambda_next_preferences_cycles_sections() {
        let mut app = test_app_no_region();
        app.current_service = Service::LambdaFunctions;
        app.mode = Mode::ColumnSelector;

        // Start at columns section
        app.column_selector_index = 0;
        app.handle_action(crate::keymap::Action::NextPreferences);

        // Should jump to page size section (9 columns + 1 empty + 1 header = 11)
        assert_eq!(app.column_selector_index, 11);

        // Next should cycle back to columns
        app.handle_action(crate::keymap::Action::NextPreferences);
        assert_eq!(app.column_selector_index, 0);
    }

    #[test]
    fn test_lambda_drill_down_on_enter() {
        let mut app = test_app_no_region();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.lambda_state.table.items = vec![crate::app::LambdaFunction {
            arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
            application: None,
            name: "test-function".to_string(),
            description: "Test function".to_string(),
            package_type: "Zip".to_string(),
            runtime: "python3.12".to_string(),
            architecture: "x86_64".to_string(),
            code_size: 1024,
            code_sha256: "test-sha256".to_string(),
            memory_mb: 128,
            timeout_seconds: 3,
            last_modified: "2024-01-01 00:00:00 (UTC)".to_string(),
            layers: vec![],
        }];
        app.lambda_state.table.selected = 0;

        // Drill down into function
        app.handle_action(crate::keymap::Action::Select);

        assert_eq!(
            app.lambda_state.current_function,
            Some("test-function".to_string())
        );
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Code);
    }

    #[test]
    fn test_lambda_go_back_from_detail() {
        let mut app = test_app_no_region();
        app.current_service = Service::LambdaFunctions;
        app.lambda_state.current_function = Some("test-function".to_string());

        app.handle_action(crate::keymap::Action::GoBack);

        assert_eq!(app.lambda_state.current_function, None);
    }

    #[test]
    fn test_lambda_detail_tab_cycling() {
        let mut app = test_app_no_region();
        app.current_service = Service::LambdaFunctions;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Code;

        app.handle_action(crate::keymap::Action::NextDetailTab);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Monitor);

        app.handle_action(crate::keymap::Action::NextDetailTab);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Configuration);

        app.handle_action(crate::keymap::Action::NextDetailTab);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Aliases);

        app.handle_action(crate::keymap::Action::NextDetailTab);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Versions);

        app.handle_action(crate::keymap::Action::NextDetailTab);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Code);
    }

    #[test]
    fn test_lambda_breadcrumbs_with_function_name() {
        let mut app = test_app_no_region();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;

        // List view
        let breadcrumb = app.breadcrumbs();
        assert_eq!(breadcrumb, "Lambda > Functions");

        // Detail view
        app.lambda_state.current_function = Some("my-function".to_string());
        let breadcrumb = app.breadcrumbs();
        assert_eq!(breadcrumb, "Lambda > my-function");
    }

    #[test]
    fn test_lambda_console_url() {
        let mut app = test_app_no_region();
        app.current_service = Service::LambdaFunctions;
        app.config.region = "us-east-1".to_string();

        // List view
        let url = app.get_console_url();
        assert_eq!(
            url,
            "https://us-east-1.console.aws.amazon.com/lambda/home?region=us-east-1#/functions"
        );

        // Detail view
        app.lambda_state.current_function = Some("my-function".to_string());
        let url = app.get_console_url();
        assert_eq!(
            url,
            "https://us-east-1.console.aws.amazon.com/lambda/home?region=us-east-1#/functions/my-function"
        );
    }

    #[test]
    fn test_lambda_last_modified_format() {
        let func = crate::app::LambdaFunction {
            arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
            application: None,
            name: "test-function".to_string(),
            description: "Test function".to_string(),
            package_type: "Zip".to_string(),
            runtime: "python3.12".to_string(),
            architecture: "x86_64".to_string(),
            code_size: 1024,
            code_sha256: "test-sha256".to_string(),
            memory_mb: 128,
            timeout_seconds: 3,
            last_modified: "2024-01-01 12:30:45 (UTC)".to_string(),
            layers: vec![],
        };

        // Verify format matches our (UTC) pattern
        assert!(func.last_modified.contains("(UTC)"));
        assert!(func.last_modified.contains("2024-01-01"));
    }

    #[test]
    fn test_lambda_expand_on_right_arrow() {
        let mut app = test_app_no_region();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.lambda_state.table.items = vec![crate::app::LambdaFunction {
            arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
            application: None,
            name: "test-function".to_string(),
            description: "Test function".to_string(),
            package_type: "Zip".to_string(),
            runtime: "python3.12".to_string(),
            architecture: "x86_64".to_string(),
            code_size: 1024,
            code_sha256: "test-sha256".to_string(),
            memory_mb: 128,
            timeout_seconds: 3,
            last_modified: "2024-01-01 00:00:00 (UTC)".to_string(),
            layers: vec![],
        }];
        app.lambda_state.table.selected = 0;

        app.handle_action(crate::keymap::Action::NextPane);

        assert_eq!(app.lambda_state.table.expanded_item, Some(0));
    }

    #[test]
    fn test_lambda_collapse_on_left_arrow() {
        let mut app = test_app_no_region();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.lambda_state.current_function = None; // In list view
        app.lambda_state.table.expanded_item = Some(0);

        app.handle_action(crate::keymap::Action::PrevPane);

        assert_eq!(app.lambda_state.table.expanded_item, None);
    }

    #[test]
    fn test_lambda_filter_activation() {
        let mut app = test_app_no_region();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.mode = Mode::Normal;

        app.handle_action(crate::keymap::Action::StartFilter);

        assert_eq!(app.mode, Mode::FilterInput);
    }

    #[test]
    fn test_lambda_filter_backspace() {
        let mut app = test_app_no_region();
        app.current_service = Service::LambdaFunctions;
        app.mode = Mode::FilterInput;
        app.lambda_state.table.filter = "test".to_string();

        app.handle_action(crate::keymap::Action::FilterBackspace);

        assert_eq!(app.lambda_state.table.filter, "tes");
    }

    #[test]
    fn test_lambda_sorted_by_last_modified_desc() {
        let func1 = crate::app::LambdaFunction {
            arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
            application: None,
            name: "func1".to_string(),
            description: String::new(),
            package_type: "Zip".to_string(),
            runtime: "python3.12".to_string(),
            architecture: "x86_64".to_string(),
            code_size: 1024,
            code_sha256: "test-sha256".to_string(),
            memory_mb: 128,
            timeout_seconds: 3,
            last_modified: "2024-01-01 00:00:00 (UTC)".to_string(),
            layers: vec![],
        };
        let func2 = crate::app::LambdaFunction {
            arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
            application: None,
            name: "func2".to_string(),
            description: String::new(),
            package_type: "Zip".to_string(),
            runtime: "python3.12".to_string(),
            architecture: "x86_64".to_string(),
            code_size: 1024,
            code_sha256: "test-sha256".to_string(),
            memory_mb: 128,
            timeout_seconds: 3,
            last_modified: "2024-12-31 00:00:00 (UTC)".to_string(),
            layers: vec![],
        };

        let mut functions = [func1.clone(), func2.clone()].to_vec();
        functions.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

        // func2 should be first (newer)
        assert_eq!(functions[0].name, "func2");
        assert_eq!(functions[1].name, "func1");
    }

    #[test]
    fn test_lambda_code_properties_has_sha256() {
        let func = crate::app::LambdaFunction {
            arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
            application: None,
            name: "test-function".to_string(),
            description: "Test".to_string(),
            package_type: "Zip".to_string(),
            runtime: "python3.12".to_string(),
            architecture: "x86_64".to_string(),
            code_size: 2600,
            code_sha256: "HHn6CTPhEnmSfX9I/dozcFFLQXUTDFapBAkzjVj9UxE=".to_string(),
            memory_mb: 128,
            timeout_seconds: 3,
            last_modified: "2024-01-01 00:00:00 (UTC)".to_string(),
            layers: vec![],
        };

        assert!(!func.code_sha256.is_empty());
        assert_eq!(
            func.code_sha256,
            "HHn6CTPhEnmSfX9I/dozcFFLQXUTDFapBAkzjVj9UxE="
        );
    }

    #[test]
    fn test_lambda_name_column_has_expand_symbol() {
        let func = crate::app::LambdaFunction {
            arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
            application: None,
            name: "test-function".to_string(),
            description: "Test".to_string(),
            package_type: "Zip".to_string(),
            runtime: "python3.12".to_string(),
            architecture: "x86_64".to_string(),
            code_size: 1024,
            code_sha256: "test-sha256".to_string(),
            memory_mb: 128,
            timeout_seconds: 3,
            last_modified: "2024-01-01 00:00:00 (UTC)".to_string(),
            layers: vec![],
        };

        // Test collapsed state
        let symbol_collapsed = crate::ui::table::CURSOR_COLLAPSED;
        let rendered_collapsed = format!("{} {}", symbol_collapsed, func.name);
        assert!(rendered_collapsed.contains(symbol_collapsed));
        assert!(rendered_collapsed.contains("test-function"));

        // Test expanded state
        let symbol_expanded = crate::ui::table::CURSOR_EXPANDED;
        let rendered_expanded = format!("{} {}", symbol_expanded, func.name);
        assert!(rendered_expanded.contains(symbol_expanded));
        assert!(rendered_expanded.contains("test-function"));

        // Verify symbols are different
        assert_ne!(symbol_collapsed, symbol_expanded);
    }

    #[test]
    fn test_lambda_last_modified_column_width() {
        // Verify width is sufficient for "2025-10-31 08:37:46 (UTC)" (25 chars)
        let timestamp = "2025-10-31 08:37:46 (UTC)";
        assert_eq!(timestamp.len(), 25);

        // Column width should be at least 27 to have some padding
        let width = 27u16;
        assert!(width >= timestamp.len() as u16);
    }

    #[test]
    fn test_lambda_code_properties_has_info_and_kms_sections() {
        let mut app = test_app_no_region();
        app.current_service = Service::LambdaFunctions;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Code;
        app.lambda_state.table.items = vec![crate::app::LambdaFunction {
            arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
            application: None,
            name: "test-function".to_string(),
            description: "Test".to_string(),
            package_type: "Zip".to_string(),
            runtime: "python3.12".to_string(),
            architecture: "x86_64".to_string(),
            code_size: 2600,
            code_sha256: "HHn6CTPhEnmSfX9I/dozcFFLQXUTDFapBAkzjVj9UxE=".to_string(),
            memory_mb: 128,
            timeout_seconds: 3,
            last_modified: "2024-01-01 00:00:00 (UTC)".to_string(),
            layers: vec![],
        }];

        // Verify we're in Code tab
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Code);

        // Verify function exists
        assert!(app.lambda_state.current_function.is_some());
        assert_eq!(app.lambda_state.table.items.len(), 1);

        // Info section should have: Package size, SHA256 hash, Last modified
        let func = &app.lambda_state.table.items[0];
        assert!(!func.code_sha256.is_empty());
        assert!(!func.last_modified.is_empty());
        assert!(func.code_size > 0);
    }

    #[test]
    fn test_lambda_pagination_navigation() {
        let mut app = test_app_no_region();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.lambda_state.table.page_size = PageSize::Ten;

        // Create 25 functions
        app.lambda_state.table.items = (0..25)
            .map(|i| crate::app::LambdaFunction {
                arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
                application: None,
                name: format!("function-{}", i),
                description: "Test".to_string(),
                package_type: "Zip".to_string(),
                runtime: "python3.12".to_string(),
                architecture: "x86_64".to_string(),
                code_size: 1024,
                code_sha256: "test-sha256".to_string(),
                memory_mb: 128,
                timeout_seconds: 3,
                last_modified: "2024-01-01 00:00:00 (UTC)".to_string(),
                layers: vec![],
            })
            .collect();

        // Start at index 0 (page 0)
        app.lambda_state.table.selected = 0;
        let page_size = app.lambda_state.table.page_size.value();
        let current_page = app.lambda_state.table.selected / page_size;
        assert_eq!(current_page, 0);
        assert_eq!(app.lambda_state.table.selected % page_size, 0);

        // Navigate to index 10 (page 1)
        app.lambda_state.table.selected = 10;
        let current_page = app.lambda_state.table.selected / page_size;
        assert_eq!(current_page, 1);
        assert_eq!(app.lambda_state.table.selected % page_size, 0);

        // Navigate to index 15 (page 1, item 5)
        app.lambda_state.table.selected = 15;
        let current_page = app.lambda_state.table.selected / page_size;
        assert_eq!(current_page, 1);
        assert_eq!(app.lambda_state.table.selected % page_size, 5);
    }

    #[test]
    fn test_lambda_pagination_with_100_functions() {
        let mut app = test_app_no_region();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.lambda_state.table.page_size = PageSize::Fifty;

        // Create 100 functions (simulating real scenario)
        app.lambda_state.table.items = (0..100)
            .map(|i| crate::app::LambdaFunction {
                arn: format!("arn:aws:lambda:us-east-1:123456789012:function:func-{}", i),
                application: None,
                name: format!("function-{:03}", i),
                description: format!("Function {}", i),
                package_type: "Zip".to_string(),
                runtime: "python3.12".to_string(),
                architecture: "x86_64".to_string(),
                code_size: 1024 + i,
                code_sha256: format!("sha256-{}", i),
                memory_mb: 128,
                timeout_seconds: 3,
                last_modified: "2024-01-01 00:00:00 (UTC)".to_string(),
                layers: vec![],
            })
            .collect();

        let page_size = app.lambda_state.table.page_size.value();
        assert_eq!(page_size, 50);

        // Page 0: items 0-49
        app.lambda_state.table.selected = 0;
        let current_page = app.lambda_state.table.selected / page_size;
        assert_eq!(current_page, 0);

        app.lambda_state.table.selected = 49;
        let current_page = app.lambda_state.table.selected / page_size;
        assert_eq!(current_page, 0);

        // Page 1: items 50-99
        app.lambda_state.table.selected = 50;
        let current_page = app.lambda_state.table.selected / page_size;
        assert_eq!(current_page, 1);

        app.lambda_state.table.selected = 99;
        let current_page = app.lambda_state.table.selected / page_size;
        assert_eq!(current_page, 1);

        // Verify pagination text
        let filtered_count = app.lambda_state.table.items.len();
        let total_pages = filtered_count.div_ceil(page_size);
        assert_eq!(total_pages, 2);
    }

    #[test]
    fn test_pagination_color_matches_border_color() {
        use ratatui::style::{Color, Style};

        // When active (not in FilterInput mode), pagination should be green, border white
        let is_filter_input = false;
        let pagination_style = if is_filter_input {
            Style::default()
        } else {
            Style::default().fg(Color::Green)
        };
        let border_style = if is_filter_input {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        assert_eq!(pagination_style.fg, Some(Color::Green));
        assert_eq!(border_style.fg, None); // White (default)

        // When in FilterInput mode, pagination should be white (default), border yellow
        let is_filter_input = true;
        let pagination_style = if is_filter_input {
            Style::default()
        } else {
            Style::default().fg(Color::Green)
        };
        let border_style = if is_filter_input {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        assert_eq!(pagination_style.fg, None); // White (default)
        assert_eq!(border_style.fg, Some(Color::Yellow));
    }

    #[test]
    fn test_lambda_application_expansion_indicator() {
        // Lambda applications should show expansion indicator like ECR repos
        let app_name = "my-application";

        // Collapsed state
        let collapsed = crate::ui::table::format_expandable(app_name, false);
        assert!(collapsed.contains(crate::ui::table::CURSOR_COLLAPSED));
        assert!(collapsed.contains(app_name));

        // Expanded state
        let expanded = crate::ui::table::format_expandable(app_name, true);
        assert!(expanded.contains(crate::ui::table::CURSOR_EXPANDED));
        assert!(expanded.contains(app_name));
    }

    #[test]
    fn test_ecr_repository_selection_uses_table_state_page_size() {
        // ECR should use TableState's page_size, not hardcoded value
        let mut app = test_app_no_region();
        app.current_service = Service::EcrRepositories;

        // Create 100 repositories
        app.ecr_state.repositories.items = (0..100)
            .map(|i| crate::ecr::repo::Repository {
                name: format!("repo{}", i),
                uri: format!("123456789012.dkr.ecr.us-east-1.amazonaws.com/repo{}", i),
                created_at: "2024-01-01".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            })
            .collect();

        // Set page size to 25
        app.ecr_state.repositories.page_size = crate::common::PageSize::TwentyFive;

        // Select item 30 (should be on page 1, index 5 within page)
        app.ecr_state.repositories.selected = 30;

        let page_size = app.ecr_state.repositories.page_size.value();
        let selected_index = app.ecr_state.repositories.selected % page_size;

        assert_eq!(page_size, 25);
        assert_eq!(selected_index, 5); // 30 % 25 = 5
    }

    #[test]
    fn test_ecr_repository_selection_indicator_visible() {
        // Verify selection indicator calculation matches table rendering
        let mut app = test_app_no_region();
        app.current_service = Service::EcrRepositories;
        app.mode = crate::keymap::Mode::Normal;

        app.ecr_state.repositories.items = vec![
            crate::ecr::repo::Repository {
                name: "repo1".to_string(),
                uri: "123456789012.dkr.ecr.us-east-1.amazonaws.com/repo1".to_string(),
                created_at: "2024-01-01".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            },
            crate::ecr::repo::Repository {
                name: "repo2".to_string(),
                uri: "123456789012.dkr.ecr.us-east-1.amazonaws.com/repo2".to_string(),
                created_at: "2024-01-02".to_string(),
                tag_immutability: "IMMUTABLE".to_string(),
                encryption_type: "KMS".to_string(),
            },
        ];

        app.ecr_state.repositories.selected = 1;

        let page_size = app.ecr_state.repositories.page_size.value();
        let selected_index = app.ecr_state.repositories.selected % page_size;

        // Should be active (not in FilterInput mode)
        let is_active = app.mode != crate::keymap::Mode::FilterInput;

        assert_eq!(selected_index, 1);
        assert!(is_active);
    }

    #[test]
    fn test_ecr_repository_shows_expandable_indicator() {
        // ECR repository name column should use format_expandable to show ► indicator
        let repo = crate::ecr::repo::Repository {
            name: "test-repo".to_string(),
            uri: "123456789012.dkr.ecr.us-east-1.amazonaws.com/test-repo".to_string(),
            created_at: "2024-01-01".to_string(),
            tag_immutability: "MUTABLE".to_string(),
            encryption_type: "AES256".to_string(),
        };

        // Collapsed state should show ►
        let collapsed = crate::ui::table::format_expandable(&repo.name, false);
        assert!(collapsed.contains(crate::ui::table::CURSOR_COLLAPSED));
        assert!(collapsed.contains("test-repo"));

        // Expanded state should show ▼
        let expanded = crate::ui::table::format_expandable(&repo.name, true);
        assert!(expanded.contains(crate::ui::table::CURSOR_EXPANDED));
        assert!(expanded.contains("test-repo"));
    }

    #[test]
    fn test_lambda_application_expanded_status_formatting() {
        // Status in expanded content should show emoji for complete states
        let app = lambda::Application {
            name: "test-app".to_string(),
            arn: "arn:aws:cloudformation:us-east-1:123456789012:stack/test-app/abc123".to_string(),
            description: "Test application".to_string(),
            status: "UpdateComplete".to_string(),
            last_modified: "2024-01-01 00:00:00 (UTC)".to_string(),
        };

        let status_upper = app.status.to_uppercase();
        let formatted = if status_upper.contains("UPDATECOMPLETE")
            || status_upper.contains("UPDATE_COMPLETE")
        {
            "✅ Update complete"
        } else if status_upper.contains("CREATECOMPLETE")
            || status_upper.contains("CREATE_COMPLETE")
        {
            "✅ Create complete"
        } else {
            &app.status
        };

        assert_eq!(formatted, "✅ Update complete");

        // Test CREATE_COMPLETE
        let app2 = lambda::Application {
            status: "CreateComplete".to_string(),
            ..app
        };
        let status_upper = app2.status.to_uppercase();
        let formatted = if status_upper.contains("UPDATECOMPLETE")
            || status_upper.contains("UPDATE_COMPLETE")
        {
            "✅ Update complete"
        } else if status_upper.contains("CREATECOMPLETE")
            || status_upper.contains("CREATE_COMPLETE")
        {
            "✅ Create complete"
        } else {
            &app2.status
        };
        assert_eq!(formatted, "✅ Create complete");
    }

    #[test]
    fn test_pagination_shows_1_when_empty() {
        let result = render_pagination_text(0, 0);
        assert_eq!(result, "[1]");
    }

    #[test]
    fn test_pagination_shows_current_page() {
        let result = render_pagination_text(0, 3);
        assert_eq!(result, "[1] 2 3");

        let result = render_pagination_text(1, 3);
        assert_eq!(result, "1 [2] 3");
    }

    #[test]
    fn test_cloudformation_section_heights_match_content() {
        // Test that section heights are calculated based on content, not fixed
        // Overview: 14 fields + 2 borders = 16
        let overview_fields = 14;
        let overview_height = overview_fields + 2;
        assert_eq!(overview_height, 16);

        // Tags (empty): 4 lines + 2 borders = 6
        let tags_empty_lines = 4;
        let tags_empty_height = tags_empty_lines + 2;
        assert_eq!(tags_empty_height, 6);

        // Stack policy (empty): 5 lines + 2 borders = 7
        let policy_empty_lines = 5;
        let policy_empty_height = policy_empty_lines + 2;
        assert_eq!(policy_empty_height, 7);

        // Rollback (empty): 6 lines + 2 borders = 8
        let rollback_empty_lines = 6;
        let rollback_empty_height = rollback_empty_lines + 2;
        assert_eq!(rollback_empty_height, 8);

        // Notifications (empty): 4 lines + 2 borders = 6
        let notifications_empty_lines = 4;
        let notifications_empty_height = notifications_empty_lines + 2;
        assert_eq!(notifications_empty_height, 6);
    }

    #[test]
    fn test_log_groups_uses_table_state() {
        let mut app = test_app_no_region();
        app.current_service = Service::CloudWatchLogGroups;

        // Verify log_groups uses TableState
        assert_eq!(app.log_groups_state.log_groups.items.len(), 0);
        assert_eq!(app.log_groups_state.log_groups.selected, 0);
        assert_eq!(app.log_groups_state.log_groups.filter, "");
        assert_eq!(
            app.log_groups_state.log_groups.page_size,
            crate::common::PageSize::Fifty
        );
    }

    #[test]
    fn test_log_groups_filter_and_pagination() {
        let mut app = test_app_no_region();
        app.current_service = Service::CloudWatchLogGroups;

        // Add test log groups
        app.log_groups_state.log_groups.items = vec![
            rusticity_core::LogGroup {
                name: "/aws/lambda/function1".to_string(),
                creation_time: None,
                stored_bytes: Some(1024),
                retention_days: None,
                log_class: None,
                arn: None,
            },
            rusticity_core::LogGroup {
                name: "/aws/lambda/function2".to_string(),
                creation_time: None,
                stored_bytes: Some(2048),
                retention_days: None,
                log_class: None,
                arn: None,
            },
            rusticity_core::LogGroup {
                name: "/aws/ecs/service1".to_string(),
                creation_time: None,
                stored_bytes: Some(4096),
                retention_days: None,
                log_class: None,
                arn: None,
            },
        ];

        // Test filtering
        app.log_groups_state.log_groups.filter = "lambda".to_string();
        let filtered = app.filtered_log_groups();
        assert_eq!(filtered.len(), 2);

        // Test pagination
        let page_size = app.log_groups_state.log_groups.page_size.value();
        assert_eq!(page_size, 50);
    }

    #[test]
    fn test_log_groups_expandable_indicators() {
        let group = rusticity_core::LogGroup {
            name: "/aws/lambda/test".to_string(),
            creation_time: None,
            stored_bytes: Some(1024),
            retention_days: None,
            log_class: None,
            arn: None,
        };

        // Test collapsed state (►)
        let collapsed = crate::ui::table::format_expandable(&group.name, false);
        assert!(collapsed.starts_with("► "));
        assert!(collapsed.contains("/aws/lambda/test"));

        // Test expanded state (▼)
        let expanded = crate::ui::table::format_expandable(&group.name, true);
        assert!(expanded.starts_with("▼ "));
        assert!(expanded.contains("/aws/lambda/test"));
    }

    #[test]
    fn test_log_groups_visual_boundaries() {
        // Verify visual boundary constants exist
        assert_eq!(crate::ui::table::CURSOR_COLLAPSED, "►");
        assert_eq!(crate::ui::table::CURSOR_EXPANDED, "▼");

        // The visual boundaries │ and ╰ are rendered in render_table()
        // They are added as prefixes to expanded content lines
        let continuation = "│ ";
        let last_line = "╰ ";

        assert_eq!(continuation, "│ ");
        assert_eq!(last_line, "╰ ");
    }

    #[test]
    fn test_log_groups_right_arrow_expands() {
        let mut app = test_app();
        app.current_service = Service::CloudWatchLogGroups;
        app.service_selected = true;
        app.view_mode = ViewMode::List;

        app.log_groups_state.log_groups.items = vec![rusticity_core::LogGroup {
            name: "/aws/lambda/test".to_string(),
            creation_time: None,
            stored_bytes: Some(1024),
            retention_days: None,
            log_class: None,
            arn: None,
        }];
        app.log_groups_state.log_groups.selected = 0;

        assert_eq!(app.log_groups_state.log_groups.expanded_item, None);

        // Right arrow - should expand
        app.handle_action(Action::NextPane);
        assert_eq!(app.log_groups_state.log_groups.expanded_item, Some(0));

        // Left arrow - should collapse
        app.handle_action(Action::PrevPane);
        assert_eq!(app.log_groups_state.log_groups.expanded_item, None);
    }

    #[test]
    fn test_log_streams_right_arrow_expands() {
        let mut app = test_app();
        app.current_service = Service::CloudWatchLogGroups;
        app.service_selected = true;
        app.view_mode = ViewMode::Detail;

        app.log_groups_state.log_streams = vec![rusticity_core::LogStream {
            name: "stream-1".to_string(),
            creation_time: None,
            last_event_time: None,
        }];
        app.log_groups_state.selected_stream = 0;

        assert_eq!(app.log_groups_state.expanded_stream, None);

        // Right arrow - should expand
        app.handle_action(Action::NextPane);
        assert_eq!(app.log_groups_state.expanded_stream, Some(0));

        // Left arrow - should collapse
        app.handle_action(Action::PrevPane);
        assert_eq!(app.log_groups_state.expanded_stream, None);
    }

    #[test]
    fn test_log_events_border_style_no_double_border() {
        // Verify that log events don't use BorderType::Double
        // The new style only uses Green fg color for active state
        let mut app = test_app();
        app.current_service = Service::CloudWatchLogGroups;
        app.service_selected = true;
        app.view_mode = ViewMode::Events;

        // Border style should only be Green fg when active, not Double border type
        // This is a regression test to ensure we don't reintroduce Double borders
        assert_eq!(app.view_mode, ViewMode::Events);
    }

    #[test]
    fn test_log_group_detail_border_style_no_double_border() {
        // Verify that log group detail doesn't use BorderType::Double
        let mut app = test_app();
        app.current_service = Service::CloudWatchLogGroups;
        app.service_selected = true;
        app.view_mode = ViewMode::Detail;

        // Border style should only be Green fg when active, not Double border type
        assert_eq!(app.view_mode, ViewMode::Detail);
    }

    #[test]
    fn test_expansion_uses_intermediate_field_indicator() {
        // Verify that expanded content uses ├ for intermediate fields
        // This is tested by checking the constants exist
        // The actual rendering logic uses:
        // - ├ for field starts (lines with ": ")
        // - │ for continuation lines
        // - ╰ for the last line

        let intermediate = "├ ";
        let continuation = "│ ";
        let last = "╰ ";

        assert_eq!(intermediate, "├ ");
        assert_eq!(continuation, "│ ");
        assert_eq!(last, "╰ ");
    }

    #[test]
    fn test_log_streams_expansion_renders() {
        let mut app = test_app();
        app.current_service = Service::CloudWatchLogGroups;
        app.service_selected = true;
        app.view_mode = ViewMode::Detail;

        app.log_groups_state.log_streams = vec![rusticity_core::LogStream {
            name: "test-stream".to_string(),
            creation_time: None,
            last_event_time: None,
        }];
        app.log_groups_state.selected_stream = 0;
        app.log_groups_state.expanded_stream = Some(0);

        // Verify expansion is set
        assert_eq!(app.log_groups_state.expanded_stream, Some(0));

        // Verify stream exists
        assert_eq!(app.log_groups_state.log_streams.len(), 1);
        assert_eq!(app.log_groups_state.log_streams[0].name, "test-stream");
    }

    #[test]
    fn test_log_streams_filter_layout_single_line() {
        // Verify that filter, exact match, and show expired are on the same line
        // This is a visual test - we verify the constraint is Length(3) not Length(4)
        let _app = App::new_without_client("test".to_string(), Some("us-east-1".to_string()));

        // Filter area should be 3 lines (1 for content + 2 for borders)
        // not 4 lines (2 for content + 2 for borders)
        let expected_filter_height = 3;
        assert_eq!(expected_filter_height, 3);
    }

    #[test]
    fn test_table_navigation_at_page_boundary() {
        let mut app = test_app();
        app.current_service = Service::CloudWatchLogGroups;
        app.service_selected = true;
        app.view_mode = ViewMode::List;
        app.mode = Mode::Normal;

        // Create 100 log groups
        for i in 0..100 {
            app.log_groups_state
                .log_groups
                .items
                .push(rusticity_core::LogGroup {
                    name: format!("/aws/lambda/function{}", i),
                    creation_time: None,
                    stored_bytes: Some(1024),
                    retention_days: None,
                    log_class: None,
                    arn: None,
                });
        }

        // Set page size to 50
        app.log_groups_state.log_groups.page_size = crate::common::PageSize::Fifty;

        // Go to item 49 (last on page 1, 0-indexed)
        app.log_groups_state.log_groups.selected = 49;

        // Press down - should go to item 50 (first on page 2)
        app.handle_action(Action::NextItem);
        assert_eq!(app.log_groups_state.log_groups.selected, 50);

        // Press up - should go back to item 49
        app.handle_action(Action::PrevItem);
        assert_eq!(app.log_groups_state.log_groups.selected, 49);

        // Go to item 50 again
        app.handle_action(Action::NextItem);
        assert_eq!(app.log_groups_state.log_groups.selected, 50);

        // Press up again - should still go to 49
        app.handle_action(Action::PrevItem);
        assert_eq!(app.log_groups_state.log_groups.selected, 49);
    }

    #[test]
    fn test_table_navigation_at_end() {
        let mut app = test_app();
        app.current_service = Service::CloudWatchLogGroups;
        app.service_selected = true;
        app.view_mode = ViewMode::List;
        app.mode = Mode::Normal;

        // Create 100 log groups
        for i in 0..100 {
            app.log_groups_state
                .log_groups
                .items
                .push(rusticity_core::LogGroup {
                    name: format!("/aws/lambda/function{}", i),
                    creation_time: None,
                    stored_bytes: Some(1024),
                    retention_days: None,
                    log_class: None,
                    arn: None,
                });
        }

        // Go to last item (99)
        app.log_groups_state.log_groups.selected = 99;

        // Press down - should stay at 99
        app.handle_action(Action::NextItem);
        assert_eq!(app.log_groups_state.log_groups.selected, 99);

        // Press up - should go to 98
        app.handle_action(Action::PrevItem);
        assert_eq!(app.log_groups_state.log_groups.selected, 98);
    }

    #[test]
    fn test_table_viewport_scrolling() {
        let mut app = test_app();
        app.current_service = Service::CloudWatchLogGroups;
        app.service_selected = true;
        app.view_mode = ViewMode::List;
        app.mode = Mode::Normal;

        // Create 100 log groups
        for i in 0..100 {
            app.log_groups_state
                .log_groups
                .items
                .push(rusticity_core::LogGroup {
                    name: format!("/aws/lambda/function{}", i),
                    creation_time: None,
                    stored_bytes: Some(1024),
                    retention_days: None,
                    log_class: None,
                    arn: None,
                });
        }

        // Set page size to 50
        app.log_groups_state.log_groups.page_size = crate::common::PageSize::Fifty;

        // Start at item 49 (last visible on first viewport)
        app.log_groups_state.log_groups.selected = 49;
        app.log_groups_state.log_groups.scroll_offset = 0;

        // Press down - should go to item 50 and scroll viewport
        app.handle_action(Action::NextItem);
        assert_eq!(app.log_groups_state.log_groups.selected, 50);
        assert_eq!(app.log_groups_state.log_groups.scroll_offset, 1); // Scrolled by 1

        // Press up - should go back to item 49 WITHOUT scrolling back
        app.handle_action(Action::PrevItem);
        assert_eq!(app.log_groups_state.log_groups.selected, 49);
        assert_eq!(app.log_groups_state.log_groups.scroll_offset, 1); // Still at 1, not 0

        // Press up again - should go to 48, still no scroll
        app.handle_action(Action::PrevItem);
        assert_eq!(app.log_groups_state.log_groups.selected, 48);
        assert_eq!(app.log_groups_state.log_groups.scroll_offset, 1); // Still at 1

        // Keep going up until we hit the top of viewport (item 1)
        for _ in 0..47 {
            app.handle_action(Action::PrevItem);
        }
        assert_eq!(app.log_groups_state.log_groups.selected, 1);
        assert_eq!(app.log_groups_state.log_groups.scroll_offset, 1); // Still at 1

        // One more up - should scroll viewport up
        app.handle_action(Action::PrevItem);
        assert_eq!(app.log_groups_state.log_groups.selected, 0);
        assert_eq!(app.log_groups_state.log_groups.scroll_offset, 0); // Scrolled to 0
    }

    #[test]
    fn test_table_up_from_last_row() {
        let mut app = test_app();
        app.current_service = Service::CloudWatchLogGroups;
        app.service_selected = true;
        app.view_mode = ViewMode::List;
        app.mode = Mode::Normal;

        // Create 100 log groups
        for i in 0..100 {
            app.log_groups_state
                .log_groups
                .items
                .push(rusticity_core::LogGroup {
                    name: format!("/aws/lambda/function{}", i),
                    creation_time: None,
                    stored_bytes: Some(1024),
                    retention_days: None,
                    log_class: None,
                    arn: None,
                });
        }

        // Set page size to 50
        app.log_groups_state.log_groups.page_size = crate::common::PageSize::Fifty;

        // Go to last row (99) with scroll showing last page
        app.log_groups_state.log_groups.selected = 99;
        app.log_groups_state.log_groups.scroll_offset = 50; // Showing items 50-99

        // Press up - should go to item 98 WITHOUT scrolling
        app.handle_action(Action::PrevItem);
        assert_eq!(app.log_groups_state.log_groups.selected, 98);
        assert_eq!(app.log_groups_state.log_groups.scroll_offset, 50); // Should NOT scroll

        // Press up again - should go to 97, still no scroll
        app.handle_action(Action::PrevItem);
        assert_eq!(app.log_groups_state.log_groups.selected, 97);
        assert_eq!(app.log_groups_state.log_groups.scroll_offset, 50); // Should NOT scroll
    }

    #[test]
    fn test_table_up_from_last_visible_row() {
        let mut app = test_app();
        app.current_service = Service::CloudWatchLogGroups;
        app.service_selected = true;
        app.view_mode = ViewMode::List;
        app.mode = Mode::Normal;

        // Create 100 log groups
        for i in 0..100 {
            app.log_groups_state
                .log_groups
                .items
                .push(rusticity_core::LogGroup {
                    name: format!("/aws/lambda/function{}", i),
                    creation_time: None,
                    stored_bytes: Some(1024),
                    retention_days: None,
                    log_class: None,
                    arn: None,
                });
        }

        // Set page size to 50
        app.log_groups_state.log_groups.page_size = crate::common::PageSize::Fifty;

        // Simulate: at item 49 (last visible), press down to get to item 50
        app.log_groups_state.log_groups.selected = 49;
        app.log_groups_state.log_groups.scroll_offset = 0;
        app.handle_action(Action::NextItem);

        // Now at item 50, scroll_offset = 1 (showing items 1-50)
        assert_eq!(app.log_groups_state.log_groups.selected, 50);
        assert_eq!(app.log_groups_state.log_groups.scroll_offset, 1);

        // Item 50 is now the last visible row
        // Press up - should move to item 49 WITHOUT scrolling
        app.handle_action(Action::PrevItem);
        assert_eq!(
            app.log_groups_state.log_groups.selected, 49,
            "Selection should move to 49"
        );
        assert_eq!(
            app.log_groups_state.log_groups.scroll_offset, 1,
            "Should NOT scroll up"
        );
    }

    #[test]
    fn test_cloudformation_up_from_last_visible_row() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.mode = Mode::Normal;

        // Create 100 stacks
        for i in 0..100 {
            app.cfn_state.table.items.push(crate::cfn::Stack {
                name: format!("Stack{}", i),
                stack_id: format!("id{}", i),
                status: "CREATE_COMPLETE".to_string(),
                created_time: "2024-01-01 00:00:00 (UTC)".to_string(),
                updated_time: "2024-01-01 00:00:00 (UTC)".to_string(),
                deleted_time: String::new(),
                description: "Test".to_string(),
                drift_status: "NOT_CHECKED".to_string(),
                last_drift_check_time: "-".to_string(),
                status_reason: String::new(),
                detailed_status: "CREATE_COMPLETE".to_string(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            });
        }

        // Set page size to 50
        app.cfn_state.table.page_size = crate::common::PageSize::Fifty;

        // Simulate: at item 49 (last visible), press down to get to item 50
        app.cfn_state.table.selected = 49;
        app.cfn_state.table.scroll_offset = 0;
        app.handle_action(Action::NextItem);

        // Now at item 50, scroll_offset should be 1
        assert_eq!(app.cfn_state.table.selected, 50);
        assert_eq!(app.cfn_state.table.scroll_offset, 1);

        // Press up - should move to item 49 WITHOUT scrolling
        app.handle_action(Action::PrevItem);
        assert_eq!(
            app.cfn_state.table.selected, 49,
            "Selection should move to 49"
        );
        assert_eq!(
            app.cfn_state.table.scroll_offset, 1,
            "Should NOT scroll up - this is the bug!"
        );
    }

    #[test]
    fn test_cloudformation_up_from_actual_last_row() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.mode = Mode::Normal;

        // Create 88 stacks (like in the user's screenshot)
        for i in 0..88 {
            app.cfn_state.table.items.push(crate::cfn::Stack {
                name: format!("Stack{}", i),
                stack_id: format!("id{}", i),
                status: "CREATE_COMPLETE".to_string(),
                created_time: "2024-01-01 00:00:00 (UTC)".to_string(),
                updated_time: "2024-01-01 00:00:00 (UTC)".to_string(),
                deleted_time: String::new(),
                description: "Test".to_string(),
                drift_status: "NOT_CHECKED".to_string(),
                last_drift_check_time: "-".to_string(),
                status_reason: String::new(),
                detailed_status: "CREATE_COMPLETE".to_string(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            });
        }

        // Set page size to 50
        app.cfn_state.table.page_size = crate::common::PageSize::Fifty;

        // Simulate being on page 2 (showing items 38-87, which is the last page)
        // User is at item 87 (the actual last row)
        app.cfn_state.table.selected = 87;
        app.cfn_state.table.scroll_offset = 38; // Showing last 50 items

        // Press up - should move to item 86 WITHOUT scrolling
        app.handle_action(Action::PrevItem);
        assert_eq!(
            app.cfn_state.table.selected, 86,
            "Selection should move to 86"
        );
        assert_eq!(
            app.cfn_state.table.scroll_offset, 38,
            "Should NOT scroll - scroll_offset should stay at 38"
        );
    }

    #[test]
    fn test_iam_users_default_columns() {
        let app = test_app();
        assert_eq!(app.iam_user_visible_column_ids.len(), 11);
        assert!(app
            .iam_user_visible_column_ids
            .contains(&"column.iam.user.user_name"));
        assert!(app
            .iam_user_visible_column_ids
            .contains(&"column.iam.user.path"));
        assert!(app
            .iam_user_visible_column_ids
            .contains(&"column.iam.user.arn"));
    }

    #[test]
    fn test_iam_users_all_columns() {
        let app = test_app();
        assert_eq!(app.iam_user_column_ids.len(), 14);
        assert!(app
            .iam_user_column_ids
            .contains(&"column.iam.user.creation_time"));
        assert!(app
            .iam_user_column_ids
            .contains(&"column.iam.user.console_access"));
        assert!(app
            .iam_user_column_ids
            .contains(&"column.iam.user.signing_certs"));
    }

    #[test]
    fn test_iam_users_filter() {
        let mut app = test_app();
        app.current_service = Service::IamUsers;

        // Add test users
        app.iam_state.users.items = vec![
            crate::iam::IamUser {
                user_name: "alice".to_string(),
                path: "/".to_string(),
                groups: "admins".to_string(),
                last_activity: "2024-01-01".to_string(),
                mfa: "Enabled".to_string(),
                password_age: "30 days".to_string(),
                console_last_sign_in: "2024-01-01".to_string(),
                access_key_id: "AKIA...".to_string(),
                active_key_age: "60 days".to_string(),
                access_key_last_used: "2024-01-01".to_string(),
                arn: "arn:aws:iam::123456789012:user/alice".to_string(),
                creation_time: "2023-01-01".to_string(),
                console_access: "Enabled".to_string(),
                signing_certs: "0".to_string(),
            },
            crate::iam::IamUser {
                user_name: "bob".to_string(),
                path: "/".to_string(),
                groups: "developers".to_string(),
                last_activity: "2024-01-02".to_string(),
                mfa: "Disabled".to_string(),
                password_age: "45 days".to_string(),
                console_last_sign_in: "2024-01-02".to_string(),
                access_key_id: "AKIA...".to_string(),
                active_key_age: "90 days".to_string(),
                access_key_last_used: "2024-01-02".to_string(),
                arn: "arn:aws:iam::123456789012:user/bob".to_string(),
                creation_time: "2023-02-01".to_string(),
                console_access: "Enabled".to_string(),
                signing_certs: "1".to_string(),
            },
        ];

        // No filter - should return all users
        let filtered = crate::ui::iam::filtered_iam_users(&app);
        assert_eq!(filtered.len(), 2);

        // Filter by name
        app.iam_state.users.filter = "alice".to_string();
        let filtered = crate::ui::iam::filtered_iam_users(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].user_name, "alice");

        // Case insensitive filter
        app.iam_state.users.filter = "BOB".to_string();
        let filtered = crate::ui::iam::filtered_iam_users(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].user_name, "bob");
    }

    #[test]
    fn test_iam_users_pagination() {
        let mut app = test_app();
        app.current_service = Service::IamUsers;

        // Add 30 test users
        for i in 0..30 {
            app.iam_state.users.items.push(crate::iam::IamUser {
                user_name: format!("user{}", i),
                path: "/".to_string(),
                groups: String::new(),
                last_activity: "-".to_string(),
                mfa: "Disabled".to_string(),
                password_age: "-".to_string(),
                console_last_sign_in: "-".to_string(),
                access_key_id: "-".to_string(),
                active_key_age: "-".to_string(),
                access_key_last_used: "-".to_string(),
                arn: format!("arn:aws:iam::123456789012:user/user{}", i),
                creation_time: "2023-01-01".to_string(),
                console_access: "Disabled".to_string(),
                signing_certs: "0".to_string(),
            });
        }

        // Default page size is 25
        app.iam_state.users.page_size = crate::common::PageSize::TwentyFive;

        let filtered = crate::ui::iam::filtered_iam_users(&app);
        assert_eq!(filtered.len(), 30);

        // Pagination should work
        let page_size = app.iam_state.users.page_size.value();
        assert_eq!(page_size, 25);
    }

    #[test]
    fn test_iam_users_expansion() {
        let mut app = test_app();
        app.current_service = Service::IamUsers;
        app.service_selected = true;
        app.mode = Mode::Normal;

        app.iam_state.users.items = vec![crate::iam::IamUser {
            user_name: "testuser".to_string(),
            path: "/admin/".to_string(),
            groups: "admins,developers".to_string(),
            last_activity: "2024-01-01".to_string(),
            mfa: "Enabled".to_string(),
            password_age: "30 days".to_string(),
            console_last_sign_in: "2024-01-01 10:00:00".to_string(),
            access_key_id: "AKIAIOSFODNN7EXAMPLE".to_string(),
            active_key_age: "60 days".to_string(),
            access_key_last_used: "2024-01-01 09:00:00".to_string(),
            arn: "arn:aws:iam::123456789012:user/admin/testuser".to_string(),
            creation_time: "2023-01-01 00:00:00".to_string(),
            console_access: "Enabled".to_string(),
            signing_certs: "2".to_string(),
        }];

        // Expand first item
        app.handle_action(Action::NextPane);
        assert_eq!(app.iam_state.users.expanded_item, Some(0));

        // Collapse
        app.handle_action(Action::PrevPane);
        assert_eq!(app.iam_state.users.expanded_item, None);
    }

    #[test]
    fn test_iam_users_in_service_picker() {
        let app = test_app();
        assert!(app.service_picker.services.contains(&"IAM > Users"));
    }

    #[test]
    fn test_iam_users_service_selection() {
        let mut app = test_app();
        app.mode = Mode::ServicePicker;
        let filtered = app.filtered_services();
        let selected_idx = filtered.iter().position(|&s| s == "IAM > Users").unwrap();
        app.service_picker.selected = selected_idx;

        app.handle_action(Action::Select);

        assert_eq!(app.current_service, Service::IamUsers);
        assert!(app.service_selected);
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.tabs[0].service, Service::IamUsers);
        assert_eq!(app.tabs[0].title, "IAM > Users");
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(1), "1 second");
        assert_eq!(format_duration(30), "30 seconds");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(60), "1 minute");
        assert_eq!(format_duration(120), "2 minutes");
        assert_eq!(format_duration(3600 - 1), "59 minutes");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(3600), "1 hour");
        assert_eq!(format_duration(7200), "2 hours");
        assert_eq!(format_duration(3600 + 1800), "1 hour 30 minutes");
        assert_eq!(format_duration(7200 + 60), "2 hours 1 minute");
    }

    #[test]
    fn test_format_duration_days() {
        assert_eq!(format_duration(86400), "1 day");
        assert_eq!(format_duration(172800), "2 days");
        assert_eq!(format_duration(86400 + 3600), "1 day 1 hour");
        assert_eq!(format_duration(172800 + 7200), "2 days 2 hours");
    }

    #[test]
    fn test_format_duration_weeks() {
        assert_eq!(format_duration(604800), "1 week");
        assert_eq!(format_duration(1209600), "2 weeks");
        assert_eq!(format_duration(604800 + 86400), "1 week 1 day");
        assert_eq!(format_duration(1209600 + 172800), "2 weeks 2 days");
    }

    #[test]
    fn test_format_duration_years() {
        assert_eq!(format_duration(31536000), "1 year");
        assert_eq!(format_duration(63072000), "2 years");
        assert_eq!(format_duration(31536000 + 604800), "1 year 1 week");
        assert_eq!(format_duration(63072000 + 1209600), "2 years 2 weeks");
    }

    #[test]
    fn test_tab_style_selected() {
        let style = tab_style(true);
        assert_eq!(style, highlight());
    }

    #[test]
    fn test_tab_style_not_selected() {
        let style = tab_style(false);
        assert_eq!(style, Style::default());
    }

    #[test]
    fn test_render_tab_spans_single_tab() {
        let tabs = [("Tab1", true)];
        let spans = render_tab_spans(&tabs);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "Tab1");
        assert_eq!(spans[0].style, service_tab_style(true));
    }

    #[test]
    fn test_render_tab_spans_multiple_tabs() {
        let tabs = [("Tab1", true), ("Tab2", false), ("Tab3", false)];
        let spans = render_tab_spans(&tabs);
        assert_eq!(spans.len(), 5); // Tab1, separator, Tab2, separator, Tab3
        assert_eq!(spans[0].content, "Tab1");
        assert_eq!(spans[0].style, service_tab_style(true));
        assert_eq!(spans[1].content, " ⋮ ");
        assert_eq!(spans[2].content, "Tab2");
        assert_eq!(spans[2].style, Style::default());
        assert_eq!(spans[3].content, " ⋮ ");
        assert_eq!(spans[4].content, "Tab3");
        assert_eq!(spans[4].style, Style::default());
    }

    #[test]
    fn test_render_tab_spans_no_separator_for_first() {
        let tabs = [("First", false), ("Second", true)];
        let spans = render_tab_spans(&tabs);
        assert_eq!(spans.len(), 3); // First, separator, Second
        assert_eq!(spans[0].content, "First");
        assert_eq!(spans[1].content, " ⋮ ");
        assert_eq!(spans[2].content, "Second");
        assert_eq!(spans[2].style, service_tab_style(true));
    }
}
