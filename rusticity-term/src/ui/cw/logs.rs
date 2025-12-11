// CloudWatch Logs UI rendering and state
use crate::app::App;
use crate::common::{
    format_bytes, format_timestamp, render_pagination_text, render_vertical_scrollbar, CyclicEnum,
    InputFocus, SortDirection,
};
use crate::cw::insights::{DateRangeType, TimeUnit};
use crate::cw::logs::{EventColumn, LogGroupColumn, StreamColumn};
use crate::keymap::Mode;
use crate::ui::table::{expanded_from_columns, render_table, Column as TableColumn, TableConfig};
use crate::ui::{
    calculate_dynamic_height, filter_area, get_cursor, labeled_field,
    render_fields_with_dynamic_columns, render_tabs,
};
use ratatui::{prelude::*, widgets::*};
use rusticity_core::{LogEvent, LogGroup, LogStream};

// State
pub struct CloudWatchLogGroupsState {
    pub log_groups: crate::table::TableState<LogGroup>,
    pub log_streams: Vec<LogStream>,
    pub log_events: Vec<LogEvent>,
    pub selected_stream: usize,
    pub selected_event: usize,
    pub loading: bool,
    pub loading_message: String,
    pub detail_tab: DetailTab,
    pub stream_filter: String,
    pub exact_match: bool,
    pub show_expired: bool,
    pub filter_mode: bool,
    pub input_focus: InputFocus,
    pub stream_page: usize,
    pub stream_sort: StreamSort,
    pub stream_sort_desc: bool,
    pub event_filter: String,
    pub event_scroll_offset: usize,
    pub event_horizontal_scroll: usize,
    pub has_older_events: bool,
    pub event_input_focus: EventFilterFocus,
    pub stream_page_size: usize,
    pub stream_current_page: usize,
    pub expanded_event: Option<usize>,
    pub expanded_stream: Option<usize>,
    pub next_backward_token: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub date_range_type: DateRangeType,
    pub relative_amount: String,
    pub relative_unit: TimeUnit,
}

impl CloudWatchLogGroupsState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for CloudWatchLogGroupsState {
    fn default() -> Self {
        Self {
            log_groups: crate::table::TableState::new(),
            log_streams: Vec::new(),
            log_events: Vec::new(),
            selected_stream: 0,
            selected_event: 0,
            loading: false,
            loading_message: String::new(),
            detail_tab: DetailTab::LogStreams,
            stream_filter: String::new(),
            exact_match: false,
            show_expired: false,
            filter_mode: false,
            input_focus: InputFocus::Filter,
            stream_page: 0,
            stream_sort: StreamSort::LastEventTime,
            stream_sort_desc: true,
            event_filter: String::new(),
            event_scroll_offset: 0,
            event_horizontal_scroll: 0,
            has_older_events: true,
            event_input_focus: EventFilterFocus::Filter,
            stream_page_size: 20,
            stream_current_page: 0,
            expanded_event: None,
            expanded_stream: None,
            next_backward_token: None,
            start_time: None,
            end_time: None,
            date_range_type: DateRangeType::Relative,
            relative_amount: String::new(),
            relative_unit: TimeUnit::Hours,
        }
    }
}

// Enums
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StreamSort {
    Name,
    CreationTime,
    LastEventTime,
}

pub const FILTER_CONTROLS: [InputFocus; 4] = [
    InputFocus::Filter,
    InputFocus::Checkbox("ExactMatch"),
    InputFocus::Checkbox("ShowExpired"),
    InputFocus::Pagination,
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventFilterFocus {
    Filter,
    DateRange,
}

impl EventFilterFocus {
    const ALL: [EventFilterFocus; 2] = [EventFilterFocus::Filter, EventFilterFocus::DateRange];

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|&f| f == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|&f| f == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetailTab {
    LogStreams,
    Tags,
    AnomalyDetection,
    MetricFilter,
    SubscriptionFilters,
    ContributorInsights,
    DataProtection,
    FieldIndexes,
    Transformer,
}

impl CyclicEnum for DetailTab {
    const ALL: &'static [Self] = &[Self::LogStreams];
}

impl DetailTab {
    pub fn name(&self) -> &'static str {
        match self {
            DetailTab::LogStreams => "Log streams",
            DetailTab::Tags => "Tags",
            DetailTab::AnomalyDetection => "Anomaly detection",
            DetailTab::MetricFilter => "Metric filter",
            DetailTab::SubscriptionFilters => "Subscription filters",
            DetailTab::ContributorInsights => "Contributor insights",
            DetailTab::DataProtection => "Data protection",
            DetailTab::FieldIndexes => "Field indexes",
            DetailTab::Transformer => "Transformer",
        }
    }

    pub fn all() -> Vec<DetailTab> {
        vec![DetailTab::LogStreams]
    }
}

// Helper functions

pub fn selected_log_group(app: &App) -> Option<&LogGroup> {
    app.log_groups_state
        .log_groups
        .items
        .get(app.log_groups_state.log_groups.selected)
}

pub fn filtered_log_groups(app: &App) -> Vec<&LogGroup> {
    if app.log_groups_state.log_groups.filter.is_empty() {
        app.log_groups_state.log_groups.items.iter().collect()
    } else {
        app.log_groups_state
            .log_groups
            .items
            .iter()
            .filter(|group| {
                if app.log_groups_state.exact_match {
                    group.name == app.log_groups_state.log_groups.filter
                } else {
                    group.name.contains(&app.log_groups_state.log_groups.filter)
                }
            })
            .collect()
    }
}

pub fn filtered_log_streams(app: &App) -> Vec<&LogStream> {
    let mut streams: Vec<&LogStream> = if app.log_groups_state.stream_filter.is_empty() {
        app.log_groups_state.log_streams.iter().collect()
    } else {
        app.log_groups_state
            .log_streams
            .iter()
            .filter(|stream| {
                if app.log_groups_state.exact_match {
                    stream.name == app.log_groups_state.stream_filter
                } else {
                    stream.name.contains(&app.log_groups_state.stream_filter)
                }
            })
            .collect()
    };

    streams.sort_by(|a, b| {
        let cmp = match app.log_groups_state.stream_sort {
            StreamSort::Name => a.name.cmp(&b.name),
            StreamSort::CreationTime => a.creation_time.cmp(&b.creation_time),
            StreamSort::LastEventTime => a.last_event_time.cmp(&b.last_event_time),
        };
        if app.log_groups_state.stream_sort_desc {
            cmp.reverse()
        } else {
            cmp
        }
    });

    streams
}

pub fn filtered_log_events(app: &App) -> Vec<&LogEvent> {
    if app.log_groups_state.event_filter.is_empty() {
        app.log_groups_state.log_events.iter().collect()
    } else {
        app.log_groups_state
            .log_events
            .iter()
            .filter(|event| event.message.contains(&app.log_groups_state.event_filter))
            .collect()
    }
}

pub fn render_groups_list(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter
            Constraint::Min(0),    // Table
        ])
        .split(area);

    let placeholder = "Filter loaded log groups or try prefix search";
    let filtered_groups = filtered_log_groups(app);
    let filtered_count = filtered_groups.len();
    let page_size = app.log_groups_state.log_groups.page_size.value();
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = app.log_groups_state.log_groups.selected / page_size;
    let pagination = render_pagination_text(current_page, total_pages);

    crate::ui::filter::render_simple_filter(
        frame,
        chunks[0],
        crate::ui::filter::SimpleFilterConfig {
            filter_text: &app.log_groups_state.log_groups.filter,
            placeholder,
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.log_groups_state.input_focus == InputFocus::Filter,
            is_pagination_focused: app.log_groups_state.input_focus == InputFocus::Pagination,
        },
    );

    let scroll_offset = app.log_groups_state.log_groups.scroll_offset;
    let start_idx = scroll_offset;
    let end_idx = (start_idx + page_size).min(filtered_groups.len());
    let paginated: Vec<&LogGroup> = filtered_groups[start_idx..end_idx].to_vec();

    let mut columns: Vec<Box<dyn TableColumn<LogGroup>>> = vec![];

    for col_id in &app.cw_log_group_visible_column_ids {
        let Some(col) = LogGroupColumn::from_id(col_id) else {
            continue;
        };
        columns.push(Box::new(col));
    }

    let expanded_index = app
        .log_groups_state
        .log_groups
        .expanded_item
        .and_then(|idx| {
            if idx >= start_idx && idx < end_idx {
                Some(idx - start_idx)
            } else {
                None
            }
        });

    let config = TableConfig {
        items: paginated,
        selected_index: app.log_groups_state.log_groups.selected % page_size,
        expanded_index,
        columns: &columns,
        sort_column: "",
        sort_direction: SortDirection::Asc,
        title: format!(" Log groups ({}) ", filtered_count),
        area: chunks[1],
        get_expanded_content: Some(Box::new(|group: &LogGroup| {
            expanded_from_columns(&columns, group)
        })),
        is_active: app.mode != Mode::FilterInput,
    };

    render_table(frame, config);
}

pub fn render_group_detail(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    let is_active = !matches!(
        app.mode,
        Mode::SpaceMenu
            | Mode::ServicePicker
            | Mode::ColumnSelector
            | Mode::ErrorModal
            | Mode::HelpModal
            | Mode::RegionPicker
            | Mode::CalendarPicker
            | Mode::TabPicker
    );
    let border_style = if is_active {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let detail_height = if let Some(group) = selected_log_group(app) {
        let arn = format!(
            "arn:aws:logs:{}:{}:log-group:{}:*",
            app.config.region, app.config.account_id, group.name
        );
        let creation_time = group
            .creation_time
            .map(|t| format_timestamp(&t))
            .unwrap_or_else(|| "-".to_string());
        let stored_bytes = format_bytes(group.stored_bytes.unwrap_or(0));

        let lines = vec![
            labeled_field("Log class", "Standard"),
            labeled_field("Retention", "Never expire"),
            labeled_field("Stored bytes", stored_bytes),
            labeled_field("Creation time", creation_time),
            labeled_field("ARN", arn),
            Line::from(vec![
                Span::styled(
                    "Metric filters: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw("0"),
            ]),
            Line::from(vec![
                Span::styled(
                    "Subscription filters: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw("0"),
            ]),
            Line::from(vec![
                Span::styled(
                    "KMS key ID: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw("-"),
            ]),
        ];

        calculate_dynamic_height(&lines, area.width.saturating_sub(2)) + 2
    } else {
        3
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(detail_height),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    if let Some(group) = selected_log_group(app) {
        let detail_block = Block::default()
            .title(" Log group details ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default());
        let inner = detail_block.inner(chunks[0]);
        frame.render_widget(detail_block, chunks[0]);
        frame.render_widget(Clear, inner);

        let arn = format!(
            "arn:aws:logs:{}:{}:log-group:{}:*",
            app.config.region, app.config.account_id, group.name
        );
        let creation_time = group
            .creation_time
            .map(|t| format_timestamp(&t))
            .unwrap_or_else(|| "-".to_string());
        let stored_bytes = format_bytes(group.stored_bytes.unwrap_or(0));

        let lines = vec![
            labeled_field("Log class", "Standard"),
            labeled_field("Retention", "Never expire"),
            labeled_field("Stored bytes", stored_bytes),
            labeled_field("Creation time", creation_time),
            labeled_field("ARN", arn),
            Line::from(vec![
                Span::styled(
                    "Metric filters: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw("0"),
            ]),
            Line::from(vec![
                Span::styled(
                    "Subscription filters: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw("0"),
            ]),
            Line::from(vec![
                Span::styled(
                    "KMS key ID: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw("-"),
            ]),
        ];

        render_fields_with_dynamic_columns(frame, inner, lines);
    }

    render_tab_menu(frame, app, chunks[1]);

    match app.log_groups_state.detail_tab {
        DetailTab::LogStreams => render_log_streams_table(frame, app, chunks[2], border_style),
        _ => render_tab_placeholder(frame, app, chunks[2], border_style),
    }
}

fn render_tab_menu(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);
    let all_tabs = DetailTab::all();
    let tabs: Vec<(&str, DetailTab)> = all_tabs.iter().map(|tab| (tab.name(), *tab)).collect();
    render_tabs(frame, area, &tabs, &app.log_groups_state.detail_tab);
}

fn render_tab_placeholder(frame: &mut Frame, app: &App, area: Rect, border_style: Style) {
    frame.render_widget(Clear, area);
    let text = format!("{} - Coming soon", app.log_groups_state.detail_tab.name());
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style),
        )
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(paragraph, area);
}
fn render_log_streams_table(frame: &mut Frame, app: &App, area: Rect, border_style: Style) {
    frame.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let placeholder = "Filter loaded log streams or try prefix search";

    let exact_match_text = if app.log_groups_state.exact_match {
        "☑ Exact match"
    } else {
        "☐ Exact match"
    };
    let show_expired_text = if app.log_groups_state.show_expired {
        "☑ Show expired"
    } else {
        "☐ Show expired"
    };

    let filtered_streams = filtered_log_streams(app);
    let count = filtered_streams.len();
    let page_size = 20;
    let total_pages = count.div_ceil(page_size);
    let current_page = app.log_groups_state.selected_stream / page_size;
    let pagination = render_pagination_text(current_page, total_pages);

    crate::ui::filter::render_filter_bar(
        frame,
        crate::ui::filter::FilterConfig {
            filter_text: &app.log_groups_state.stream_filter,
            placeholder,
            mode: app.mode,
            is_input_focused: app.log_groups_state.input_focus == InputFocus::Filter,
            controls: vec![
                crate::ui::filter::FilterControl {
                    text: exact_match_text.to_string(),
                    is_focused: app.log_groups_state.input_focus
                        == InputFocus::Checkbox("ExactMatch"),
                },
                crate::ui::filter::FilterControl {
                    text: show_expired_text.to_string(),
                    is_focused: app.log_groups_state.input_focus
                        == InputFocus::Checkbox("ShowExpired"),
                },
                crate::ui::filter::FilterControl {
                    text: pagination,
                    is_focused: app.log_groups_state.input_focus == InputFocus::Pagination,
                },
            ],
            area: chunks[0],
        },
    );

    let columns: Vec<Box<dyn TableColumn<LogStream>>> = app
        .cw_log_stream_visible_column_ids
        .iter()
        .filter_map(|col_id| {
            StreamColumn::from_id(col_id)
                .map(|col| Box::new(col) as Box<dyn TableColumn<LogStream>>)
        })
        .collect();

    let count_display = if count >= 100 {
        "100+".to_string()
    } else {
        count.to_string()
    };

    let sort_column = match app.log_groups_state.stream_sort {
        StreamSort::Name => "Log stream",
        StreamSort::CreationTime => "Creation time",
        StreamSort::LastEventTime => "Last event time",
    };

    let sort_direction = if app.log_groups_state.stream_sort_desc {
        SortDirection::Desc
    } else {
        SortDirection::Asc
    };

    let config = TableConfig {
        items: filtered_streams,
        selected_index: app.log_groups_state.selected_stream,
        expanded_index: app.log_groups_state.expanded_stream,
        columns: &columns,
        sort_column,
        sort_direction,
        title: format!(" Log streams ({}) ", count_display),
        area: chunks[1],
        get_expanded_content: Some(Box::new(|stream: &LogStream| {
            expanded_from_columns(&columns, stream)
        })),
        is_active: border_style.fg == Some(Color::Green)
            && app.log_groups_state.input_focus != InputFocus::Filter,
    };

    render_table(frame, config);
}

pub fn render_events(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    let is_active = !matches!(
        app.mode,
        Mode::SpaceMenu
            | Mode::ServicePicker
            | Mode::ColumnSelector
            | Mode::ErrorModal
            | Mode::HelpModal
            | Mode::RegionPicker
            | Mode::CalendarPicker
            | Mode::TabPicker
    );
    let border_style = if is_active {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Filter and date range
    let filter_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    let cursor = get_cursor(
        app.mode == Mode::EventFilterInput
            && app.log_groups_state.event_input_focus == EventFilterFocus::Filter,
    );
    let filter_text =
        if app.log_groups_state.event_filter.is_empty() && app.mode != Mode::EventFilterInput {
            vec![
                Span::styled(
                    "Filter events - press ⏎ to search",
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(cursor, Style::default().fg(Color::Yellow)),
            ]
        } else {
            vec![
                Span::raw(&app.log_groups_state.event_filter),
                Span::styled(cursor, Style::default().fg(Color::Yellow)),
            ]
        };

    let is_filter_active = app.mode == Mode::EventFilterInput
        && app.log_groups_state.event_input_focus == EventFilterFocus::Filter;
    let filter = filter_area(filter_text, is_filter_active);

    let date_border_style = if app.mode == Mode::EventFilterInput
        && app.log_groups_state.event_input_focus == EventFilterFocus::DateRange
    {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let date_range_text = vec![
        Span::raw(format!(
            "Last [{}] <{}>",
            if app.log_groups_state.relative_amount.is_empty() {
                "_"
            } else {
                &app.log_groups_state.relative_amount
            },
            app.log_groups_state.relative_unit.name()
        )),
        Span::styled(cursor, Style::default().fg(Color::Yellow)),
    ];

    let date_range = Paragraph::new(Line::from(date_range_text)).block(
        Block::default()
            .title(" Date range ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(date_border_style),
    );

    frame.render_widget(filter, filter_chunks[0]);
    frame.render_widget(date_range, filter_chunks[1]);

    // Events table with banner
    let table_area = chunks[1];

    let header_cells = app
        .cw_log_event_visible_column_ids
        .iter()
        .enumerate()
        .filter_map(|(i, col_id)| {
            EventColumn::from_id(col_id).map(|col| {
                let name = if i > 0 {
                    format!("⋮ {}", col.name())
                } else {
                    col.name().to_string()
                };
                Cell::from(name).style(Style::default().add_modifier(Modifier::BOLD))
            })
        })
        .collect::<Vec<_>>();
    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::White).fg(Color::Black))
        .height(1);

    let visible_events: Vec<_> = filtered_log_events(app).into_iter().collect();

    // Add banner as first row if there are older events
    let mut all_rows: Vec<Row> = Vec::new();

    if app.log_groups_state.has_older_events {
        let banner_cells = vec![
            Cell::from(""),
            Cell::from("There are older events to load. Scroll up to load more.")
                .style(Style::default().fg(Color::Yellow)),
        ];
        all_rows.push(Row::new(banner_cells).height(1));
    }

    // Calculate available width for message column
    let table_width = table_area.width.saturating_sub(4) as usize; // borders + spacing
    let fixed_width: usize = app
        .cw_log_event_visible_column_ids
        .iter()
        .filter_map(|col_id| EventColumn::from_id(col_id))
        .filter(|col| col.width() > 0)
        .map(|col| col.width() as usize + 1) // +1 for spacing
        .sum();
    let message_max_width = table_width.saturating_sub(fixed_width).saturating_sub(3); // -3 for highlight symbol

    let mut table_row_to_event_idx = Vec::new();
    let event_rows = visible_events.iter().enumerate().flat_map(|(idx, event)| {
        let is_expanded = app.log_groups_state.expanded_event == Some(idx);
        let is_selected = idx == app.log_groups_state.event_scroll_offset;

        let mut rows = Vec::new();

        // Main row with columns - always show first line or truncated message
        let mut cells: Vec<Cell> = Vec::new();
        for (i, col_id) in app.cw_log_event_visible_column_ids.iter().enumerate() {
            let Some(col) = EventColumn::from_id(col_id) else {
                continue;
            };
            let content = match col {
                EventColumn::Timestamp => {
                    let timestamp_str = format_timestamp(&event.timestamp);
                    crate::ui::table::format_expandable_with_selection(
                        &timestamp_str,
                        is_expanded,
                        is_selected,
                    )
                }
                EventColumn::Message => {
                    let msg = event
                        .message
                        .lines()
                        .next()
                        .unwrap_or("")
                        .replace('\t', " ");
                    if msg.len() > message_max_width {
                        format!("{}…", &msg[..message_max_width.saturating_sub(1)])
                    } else {
                        msg
                    }
                }
                EventColumn::IngestionTime => "-".to_string(),
                EventColumn::EventId => "-".to_string(),
                EventColumn::LogStreamName => "-".to_string(),
            };

            let cell_content = if i > 0 {
                format!("⋮ {}", content)
            } else {
                content
            };

            cells.push(Cell::from(cell_content));
        }
        table_row_to_event_idx.push(idx);
        rows.push(Row::new(cells).height(1));

        // If expanded, add empty rows to reserve space for overlay
        if is_expanded {
            // Calculate wrapped line count
            let max_width = (table_area.width.saturating_sub(3)) as usize;
            let mut line_count = 0;

            for col_id in &app.cw_log_event_visible_column_ids {
                let Some(col) = EventColumn::from_id(col_id) else {
                    continue;
                };
                let value = match col {
                    EventColumn::Timestamp => format_timestamp(&event.timestamp),
                    EventColumn::Message => event.message.replace('\t', "    "),
                    _ => "-".to_string(),
                };
                let full_line = format!("{}: {}", col.name(), value);
                line_count += full_line.len().div_ceil(max_width);
            }

            for _ in 0..line_count {
                // Empty row to reserve space - will be covered by overlay
                table_row_to_event_idx.push(idx);
                rows.push(Row::new(vec![Cell::from("")]).height(1));
            }
        }

        rows
    });

    all_rows.extend(event_rows);

    let banner_offset = if app.log_groups_state.has_older_events {
        1
    } else {
        0
    };
    let mut table_state_index = banner_offset;
    for (i, &event_idx) in table_row_to_event_idx.iter().enumerate() {
        if event_idx == app.log_groups_state.event_scroll_offset {
            table_state_index = banner_offset + i;
            break;
        }
    }

    let widths: Vec<Constraint> = app
        .cw_log_event_visible_column_ids
        .iter()
        .filter_map(|col_id| {
            EventColumn::from_id(col_id).map(|col| {
                if col.width() == 0 {
                    Constraint::Min(0)
                } else {
                    Constraint::Length(col.width())
                }
            })
        })
        .collect();

    let table = Table::new(all_rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(format!(" Log events ({}) ", visible_events.len()))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style),
        )
        .column_spacing(1)
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("");

    let mut state = TableState::default();
    state.select(Some(table_state_index));

    frame.render_stateful_widget(table, table_area, &mut state);

    // Render expanded content as overlay
    if let Some(expanded_idx) = app.log_groups_state.expanded_event {
        if let Some(event) = visible_events.get(expanded_idx) {
            // Find row position
            let mut row_y = 0;
            for (i, &event_idx) in table_row_to_event_idx.iter().enumerate() {
                if event_idx == expanded_idx {
                    row_y = i;
                    break;
                }
            }

            let banner_offset = if app.log_groups_state.has_older_events {
                1
            } else {
                0
            };

            // Build content with column names
            let mut lines = Vec::new();
            let max_width = table_area.width.saturating_sub(3) as usize;

            for col_id in &app.cw_log_event_visible_column_ids {
                let Some(col) = EventColumn::from_id(col_id) else {
                    continue;
                };
                let value = match col {
                    EventColumn::Timestamp => format_timestamp(&event.timestamp),
                    EventColumn::Message => event.message.replace('\t', "    "),
                    EventColumn::IngestionTime => "-".to_string(),
                    EventColumn::EventId => "-".to_string(),
                    EventColumn::LogStreamName => "-".to_string(),
                };
                let col_name = format!("{}: ", col.name());
                let full_line = format!("{}{}", col_name, value);

                // Wrap long lines, marking first line
                if full_line.len() <= max_width {
                    lines.push((full_line, true)); // true = first line with column name
                } else {
                    // First chunk includes column name
                    let first_chunk_len = max_width.min(full_line.len());
                    lines.push((full_line[..first_chunk_len].to_string(), true));

                    // Remaining chunks are continuation
                    let mut remaining = &full_line[first_chunk_len..];
                    while !remaining.is_empty() {
                        let take = max_width.min(remaining.len());
                        lines.push((remaining[..take].to_string(), false)); // false = continuation
                        remaining = &remaining[take..];
                    }
                }
            }

            // Render each line as overlay
            // Clear entire expanded area once
            let start_y = table_area.y + 2 + banner_offset as u16 + row_y as u16 + 1;
            let max_y = table_area.y + table_area.height - 1;

            // Only render if start_y is within bounds
            if start_y < max_y {
                let available_height = (max_y - start_y) as usize;
                let visible_lines = lines.len().min(available_height);

                if visible_lines > 0 {
                    let clear_area = Rect {
                        x: table_area.x + 1,
                        y: start_y,
                        width: table_area.width.saturating_sub(3),
                        height: visible_lines as u16,
                    };
                    frame.render_widget(Clear, clear_area);
                }

                for (line_idx, (line, is_first)) in lines.iter().enumerate() {
                    let y = start_y + line_idx as u16;
                    if y >= max_y {
                        break;
                    }

                    let line_area = Rect {
                        x: table_area.x + 1,
                        y,
                        width: table_area.width.saturating_sub(3), // Leave room for scrollbar
                        height: 1,
                    };

                    // Add expansion indicator on the left
                    let is_last_line = line_idx == lines.len() - 1;
                    let indicator = if is_last_line {
                        "╰ "
                    } else if *is_first {
                        "├ "
                    } else {
                        "│ "
                    };

                    // Bold column name only on first line of each field
                    let spans = if *is_first {
                        if let Some(colon_pos) = line.find(": ") {
                            let col_name = &line[..colon_pos + 2];
                            let rest = &line[colon_pos + 2..];
                            vec![
                                Span::raw(indicator),
                                Span::styled(
                                    col_name.to_string(),
                                    Style::default().add_modifier(Modifier::BOLD),
                                ),
                                Span::raw(rest.to_string()),
                            ]
                        } else {
                            vec![Span::raw(indicator), Span::raw(line.clone())]
                        }
                    } else {
                        // Continuation line - no bold
                        vec![Span::raw(indicator), Span::raw(line.clone())]
                    };

                    let paragraph = Paragraph::new(Line::from(spans));
                    frame.render_widget(paragraph, line_area);
                }
            }
        }
    }

    // Render scrollbar
    let event_count = app.log_groups_state.log_events.len();
    if event_count > 0 {
        render_vertical_scrollbar(
            frame,
            table_area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            event_count,
            app.log_groups_state.event_scroll_offset,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_focus_enum_cycling() {
        use InputFocus;
        assert_eq!(
            InputFocus::Filter.next(&FILTER_CONTROLS),
            InputFocus::Checkbox("ExactMatch")
        );
        assert_eq!(
            InputFocus::Checkbox("ExactMatch").next(&FILTER_CONTROLS),
            InputFocus::Checkbox("ShowExpired")
        );
        assert_eq!(
            InputFocus::Checkbox("ShowExpired").next(&FILTER_CONTROLS),
            InputFocus::Pagination
        );
        assert_eq!(
            InputFocus::Pagination.next(&FILTER_CONTROLS),
            InputFocus::Filter
        );

        assert_eq!(
            InputFocus::Filter.prev(&FILTER_CONTROLS),
            InputFocus::Pagination
        );
        assert_eq!(
            InputFocus::Pagination.prev(&FILTER_CONTROLS),
            InputFocus::Checkbox("ShowExpired")
        );
        assert_eq!(
            InputFocus::Checkbox("ShowExpired").prev(&FILTER_CONTROLS),
            InputFocus::Checkbox("ExactMatch")
        );
        assert_eq!(
            InputFocus::Checkbox("ExactMatch").prev(&FILTER_CONTROLS),
            InputFocus::Filter
        );
    }
}
