use crate::app::App;
use crate::common::{format_timestamp, render_horizontal_scrollbar, render_vertical_scrollbar};
use crate::keymap::Mode;
use crate::ui::vertical;
use ratatui::{prelude::*, widgets::*};

pub struct State {
    pub query_language: QueryLanguage,
    pub query_text: String,
    pub query_cursor_line: usize,
    pub query_cursor_col: usize,
    pub log_group_search: String,
    pub selected_log_groups: Vec<String>,
    pub log_group_matches: Vec<String>,
    pub show_dropdown: bool,
    pub dropdown_selected: usize,
    pub insights_start_time: Option<i64>,
    pub insights_end_time: Option<i64>,
    pub insights_date_range_type: DateRangeType,
    pub insights_relative_amount: String,
    pub insights_relative_unit: TimeUnit,
    pub insights_focus: InsightsFocus,
    pub query_completed: bool,
    pub query_results: Vec<Vec<(String, String)>>,
    pub results_selected: usize,
    pub expanded_result: Option<usize>,
    pub results_horizontal_scroll: usize,
    pub results_vertical_scroll: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            query_language: QueryLanguage::LogsInsightsQL,
            query_text: String::from("fields @timestamp, @message, @logStream, @log\n| sort @timestamp desc\n| limit 10000"),
            query_cursor_line: 0,
            query_cursor_col: 0,
            log_group_search: String::new(),
            selected_log_groups: Vec::new(),
            log_group_matches: Vec::new(),
            show_dropdown: false,
            dropdown_selected: 0,
            insights_start_time: None,
            insights_end_time: None,
            insights_date_range_type: DateRangeType::Relative,
            insights_relative_amount: "1".to_string(),
            insights_relative_unit: TimeUnit::Hours,
            insights_focus: InsightsFocus::Query,
            query_completed: false,
            query_results: Vec::new(),
            results_selected: 0,
            expanded_result: None,
            results_horizontal_scroll: 0,
            results_vertical_scroll: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QueryLanguage {
    LogsInsightsQL,
    PPL,
    SQL,
}

impl QueryLanguage {
    pub fn name(&self) -> &'static str {
        match self {
            QueryLanguage::LogsInsightsQL => "Logs Insights QL",
            QueryLanguage::PPL => "PPL",
            QueryLanguage::SQL => "SQL",
        }
    }

    pub fn next(&self) -> QueryLanguage {
        match self {
            QueryLanguage::LogsInsightsQL => QueryLanguage::PPL,
            QueryLanguage::PPL => QueryLanguage::SQL,
            QueryLanguage::SQL => QueryLanguage::LogsInsightsQL,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InsightsFocus {
    QueryLanguage,
    DatePicker,
    LogGroupSearch,
    Query,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DateRangeType {
    Relative,
    Absolute,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeUnit {
    Minutes,
    Hours,
    Days,
    Weeks,
}

impl TimeUnit {
    pub fn name(&self) -> &'static str {
        match self {
            TimeUnit::Minutes => "minutes",
            TimeUnit::Hours => "hours",
            TimeUnit::Days => "days",
            TimeUnit::Weeks => "weeks",
        }
    }

    pub fn next(&self) -> TimeUnit {
        match self {
            TimeUnit::Minutes => TimeUnit::Hours,
            TimeUnit::Hours => TimeUnit::Days,
            TimeUnit::Days => TimeUnit::Weeks,
            TimeUnit::Weeks => TimeUnit::Minutes,
        }
    }
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    // Calculate query text area height
    let query_lines = app
        .insights_state
        .insights
        .query_text
        .lines()
        .count()
        .max(1);
    let query_height = (query_lines + 1).max(3) as u16;
    let input_pane_height = 3 + 3 + query_height + 2 + 2; // rows + borders

    // Split into input pane and results pane
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(input_pane_height), Constraint::Min(0)])
        .split(area);

    // Render input pane
    render_input_pane(frame, app, main_chunks[0], query_height);

    // Render results pane
    render_results_pane(frame, app, main_chunks[1]);
}

fn render_input_pane(frame: &mut Frame, app: &App, area: Rect, query_height: u16) {
    let is_active = app.mode == Mode::InsightsInput
        && !matches!(
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

    let block = Block::default()
        .title(" Logs Insights ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = vertical(
        [
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(query_height + 2),
        ],
        inner,
    );

    // Row 1: Query Language selector (left) and Date Picker (right)
    let row1_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[0]);

    // Query Language selector
    let ql_focused = app.mode == Mode::InsightsInput
        && app.insights_state.insights.insights_focus == InsightsFocus::QueryLanguage;
    let ql_style = if ql_focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let ql_block = Block::default()
        .borders(Borders::ALL)
        .border_style(ql_style);
    let ql_text = format!(" {} ", app.insights_state.insights.query_language.name());
    let ql_para = Paragraph::new(ql_text).block(ql_block);
    frame.render_widget(ql_para, row1_chunks[0]);

    // Date Picker
    let date_focused = app.mode == Mode::InsightsInput
        && app.insights_state.insights.insights_focus == InsightsFocus::DatePicker;
    let date_style = if date_focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let date_block = Block::default()
        .borders(Borders::ALL)
        .border_style(date_style);
    let date_text = format!(
        " Last {} {} ",
        app.insights_state.insights.insights_relative_amount,
        app.insights_state.insights.insights_relative_unit.name()
    );
    let date_para = Paragraph::new(date_text).block(date_block);
    frame.render_widget(date_para, row1_chunks[1]);

    // Row 2: "Select log groups by" combo and Selection criteria input
    let row2_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[1]);

    // Combo box (static for now)
    let combo_block = Block::default().borders(Borders::ALL);
    let combo_text = " Log group name ";
    let combo_para = Paragraph::new(combo_text).block(combo_block);
    frame.render_widget(combo_para, row2_chunks[0]);

    // Log group search input
    let search_focused = app.mode == Mode::InsightsInput
        && app.insights_state.insights.insights_focus == InsightsFocus::LogGroupSearch;
    let search_style = if search_focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_style(search_style);

    let search_text = if !app.insights_state.insights.show_dropdown
        && !app.insights_state.insights.selected_log_groups.is_empty()
    {
        let count = app.insights_state.insights.selected_log_groups.len();
        format!(
            " {} log group{} selected",
            count,
            if count == 1 { "" } else { "s" }
        )
    } else if app.insights_state.insights.log_group_search.is_empty() {
        " Select up to 50 groups".to_string()
    } else {
        format!(" {} ", app.insights_state.insights.log_group_search)
    };

    let search_para = Paragraph::new(search_text)
        .style(
            if app.insights_state.insights.log_group_search.is_empty()
                && app.insights_state.insights.selected_log_groups.is_empty()
            {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            },
        )
        .block(search_block);
    frame.render_widget(search_para, row2_chunks[1]);

    // Row 3: Query editor with line numbers
    let query_focused = app.mode == Mode::InsightsInput
        && app.insights_state.insights.insights_focus == InsightsFocus::Query;
    let query_style = if query_focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let query_block = Block::default()
        .borders(Borders::ALL)
        .border_style(query_style);

    let query_inner = query_block.inner(chunks[2]);
    frame.render_widget(query_block, chunks[2]);

    // Split for line numbers and query text
    let query_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(query_inner);

    // Line numbers - count actual lines, add 1 if text ends with newline
    let num_lines = if app.insights_state.insights.query_text.is_empty() {
        1
    } else {
        let base_lines = app.insights_state.insights.query_text.lines().count();
        if app.insights_state.insights.query_text.ends_with('\n') {
            base_lines + 1
        } else {
            base_lines
        }
    };
    let line_numbers: Vec<String> = (1..=num_lines).map(|i| format!("{:>3} ", i)).collect();
    let line_num_text = line_numbers.join("\n");
    let line_num_para = Paragraph::new(line_num_text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(line_num_para, query_chunks[0]);

    // Query text with syntax highlighting
    let query_lines = highlight_insights_query(&app.insights_state.insights.query_text);
    let query_para = Paragraph::new(query_lines);
    frame.render_widget(query_para, query_chunks[1]);

    // Render dropdown if active
    if app.mode == Mode::InsightsInput
        && app.insights_state.insights.show_dropdown
        && !app.insights_state.insights.log_group_matches.is_empty()
    {
        let row2_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(chunks[1]);

        let dropdown_height =
            (app.insights_state.insights.log_group_matches.len() as u16).min(10) + 2;
        let dropdown_area = Rect {
            x: row2_chunks[1].x,
            y: row2_chunks[1].y + row2_chunks[1].height,
            width: row2_chunks[1].width,
            height: dropdown_height.min(
                area.height
                    .saturating_sub(row2_chunks[1].y + row2_chunks[1].height),
            ),
        };

        let items: Vec<ListItem> = app
            .insights_state
            .insights
            .log_group_matches
            .iter()
            .map(|name| {
                let is_selected = app
                    .insights_state
                    .insights
                    .selected_log_groups
                    .contains(name);
                let checkbox = if is_selected { "☑" } else { "☐" };
                let text = format!("{} {}", checkbox, name);
                ListItem::new(text)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        let mut state = ListState::default();
        state.select(Some(app.insights_state.insights.dropdown_selected));

        frame.render_widget(Clear, dropdown_area);
        frame.render_stateful_widget(list, dropdown_area, &mut state);
    }
}

fn render_results_pane(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    let is_active = app.mode == Mode::Normal
        && !matches!(
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

    let results_block = Block::default()
        .title(format!(
            " Logs ({}) ",
            app.insights_state.insights.query_results.len()
        ))
        .borders(Borders::ALL)
        .border_style(border_style);

    let results_inner = results_block.inner(area);
    frame.render_widget(results_block, area);

    // Show loading message if executing
    if app.log_groups_state.loading {
        let loading_text = "Executing query...";
        let loading = Paragraph::new(loading_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Yellow));

        let centered_area = Rect {
            x: results_inner.x,
            y: results_inner.y + results_inner.height / 3,
            width: results_inner.width,
            height: 1,
        };
        frame.render_widget(loading, centered_area);
    } else if app.insights_state.insights.query_results.is_empty() {
        let display_text = if app.insights_state.insights.query_completed {
            "No results found\n\nTry adjusting your query or time range"
        } else {
            "No results\nRun a query to see related events"
        };

        let no_results = Paragraph::new(display_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));

        let centered_area = Rect {
            x: results_inner.x,
            y: results_inner.y + results_inner.height / 3,
            width: results_inner.width,
            height: 2,
        };
        frame.render_widget(no_results, centered_area);
    } else {
        // Display results as table with inline expansion
        let num_cols = app
            .insights_state
            .insights
            .query_results
            .first()
            .map(|r| r.len())
            .unwrap_or(0);
        let scroll_offset = app.insights_state.insights.results_horizontal_scroll;

        let mut all_rows = Vec::new();
        let mut row_to_result_idx = Vec::new();

        for (idx, result_row) in app.insights_state.insights.query_results.iter().enumerate() {
            let is_expanded = app.insights_state.insights.expanded_result == Some(idx);
            let is_selected = idx == app.insights_state.insights.results_selected;

            // Main row
            let cells: Vec<Cell> = result_row
                .iter()
                .enumerate()
                .skip(scroll_offset)
                .map(|(i, (field, value))| {
                    let formatted_value = if field == "@timestamp" {
                        format_timestamp_value(value)
                    } else {
                        value.replace('\t', " ")
                    };

                    let cell_content = if i > scroll_offset {
                        format!("⋮ {}", formatted_value)
                    } else if i == scroll_offset {
                        // First visible column gets arrow indicator
                        crate::ui::table::format_expandable_with_selection(
                            &formatted_value,
                            is_expanded,
                            is_selected,
                        )
                    } else {
                        formatted_value
                    };

                    Cell::from(cell_content)
                })
                .collect();

            row_to_result_idx.push(idx);
            all_rows.push(Row::new(cells).height(1));

            // If expanded, add empty rows for space
            if is_expanded {
                for (field, value) in result_row.iter() {
                    let formatted_value = if field == "@timestamp" {
                        format_timestamp_value(value)
                    } else {
                        value.replace('\t', " ")
                    };
                    let _detail_text = format!("  {}: {}", field, formatted_value);

                    // Add empty row
                    let mut detail_cells = Vec::new();
                    for _ in 0..result_row.iter().skip(scroll_offset).count() {
                        detail_cells.push(Cell::from(""));
                    }
                    row_to_result_idx.push(idx);
                    all_rows.push(Row::new(detail_cells).height(1));
                }
            }
        }

        let (headers, widths) =
            if let Some(first_row) = app.insights_state.insights.query_results.first() {
                let headers: Vec<Cell> = first_row
                    .iter()
                    .enumerate()
                    .skip(scroll_offset)
                    .map(|(i, (field, _))| {
                        let name = if i > scroll_offset {
                            format!("⋮ {}", field)
                        } else {
                            field.to_string()
                        };
                        Cell::from(name).style(
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )
                    })
                    .collect();

                let visible_cols: Vec<_> = first_row.iter().skip(scroll_offset).collect();
                let widths: Vec<Constraint> = visible_cols
                    .iter()
                    .enumerate()
                    .map(|(i, (field, _))| {
                        if i == visible_cols.len() - 1 {
                            // Last column takes all remaining width
                            Constraint::Min(0)
                        } else if field == "@timestamp" {
                            Constraint::Length(28)
                        } else {
                            Constraint::Length(50)
                        }
                    })
                    .collect();

                (headers, widths)
            } else {
                (vec![], vec![])
            };

        let header = Row::new(headers).style(Style::default().bg(Color::White).fg(Color::Black));

        let table = Table::new(all_rows, widths)
            .header(header)
            .column_spacing(1)
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("");

        let mut state = TableState::default();
        let table_idx = row_to_result_idx
            .iter()
            .position(|&i| i == app.insights_state.insights.results_selected)
            .unwrap_or(0);
        state.select(Some(table_idx));

        frame.render_stateful_widget(table, results_inner, &mut state);

        // Render expanded content as overlay
        if let Some(expanded_idx) = app.insights_state.insights.expanded_result {
            if let Some(result_row) = app.insights_state.insights.query_results.get(expanded_idx) {
                // Find row position
                let mut row_y = 0;
                for (i, &idx) in row_to_result_idx.iter().enumerate() {
                    if idx == expanded_idx {
                        row_y = i;
                        break;
                    }
                }

                // Render each field as overlay
                for (line_offset, (field, value)) in result_row.iter().enumerate() {
                    let formatted_value = if field == "@timestamp" {
                        format_timestamp_value(value)
                    } else {
                        value.replace('\t', " ")
                    };
                    let detail_text = format!("  {}: {}", field, formatted_value);

                    let y = results_inner.y + 1 + row_y as u16 + 1 + line_offset as u16; // +1 for header, +1 for main row
                    if y >= results_inner.y + results_inner.height {
                        break;
                    }

                    let line_area = Rect {
                        x: results_inner.x,
                        y,
                        width: results_inner.width,
                        height: 1,
                    };

                    let paragraph = Paragraph::new(detail_text);
                    frame.render_widget(paragraph, line_area);
                }
            }
        }

        render_vertical_scrollbar(
            frame,
            results_inner,
            app.insights_state.insights.query_results.len(),
            app.insights_state.insights.results_selected,
        );

        if app.insights_state.insights.results_horizontal_scroll > 0 {
            let h_scrollbar_area = Rect {
                x: results_inner.x,
                y: results_inner.y + results_inner.height - 1,
                width: results_inner.width,
                height: 1,
            };
            render_horizontal_scrollbar(
                frame,
                h_scrollbar_area,
                app.insights_state.insights.results_horizontal_scroll,
                num_cols.saturating_sub(1).max(1),
            );
        }
    }
}

fn format_timestamp_value(value: &str) -> String {
    // Try to parse as milliseconds timestamp
    if let Ok(millis) = value.parse::<i64>() {
        use chrono::DateTime;
        if let Some(dt) =
            DateTime::from_timestamp(millis / 1000, ((millis % 1000) * 1_000_000) as u32)
        {
            return format_timestamp(&dt);
        }
    }
    // If parsing fails, return original value
    value.to_string()
}

fn highlight_insights_query(query: &str) -> Vec<Line<'_>> {
    const KEYWORDS: &[&str] = &[
        "fields",
        "filter",
        "stats",
        "sort",
        "limit",
        "parse",
        "display",
        "dedup",
        "by",
        "as",
        "asc",
        "desc",
        "in",
        "like",
        "and",
        "or",
        "not",
        "count",
        "sum",
        "avg",
        "min",
        "max",
        "stddev",
        "pct",
        "earliest",
        "latest",
        "sortsFirst",
        "sortsLast",
        "concat",
        "strlen",
        "trim",
        "ltrim",
        "rtrim",
        "tolower",
        "toupper",
        "substr",
        "replace",
        "strcontains",
        "isempty",
        "isblank",
        "ispresent",
        "abs",
        "ceil",
        "floor",
        "greatest",
        "least",
        "log",
        "sqrt",
        "bin",
        "dateceil",
        "datefloor",
        "fromMillis",
        "toMillis",
    ];

    query
        .lines()
        .map(|line| {
            let mut spans = Vec::new();
            let mut current = String::new();
            let chars = line.chars().peekable();

            for ch in chars {
                if ch.is_whitespace() || ch == '|' || ch == ',' || ch == '(' || ch == ')' {
                    if !current.is_empty() {
                        let is_keyword = KEYWORDS.contains(&current.to_lowercase().as_str());
                        let is_at_field = current.starts_with('@');

                        let style = if is_keyword {
                            Style::default().fg(Color::Blue)
                        } else if is_at_field {
                            Style::default().add_modifier(Modifier::ITALIC)
                        } else {
                            Style::default()
                        };

                        spans.push(Span::styled(current.clone(), style));
                        current.clear();
                    }
                    spans.push(Span::raw(ch.to_string()));
                } else {
                    current.push(ch);
                }
            }

            if !current.is_empty() {
                let is_keyword = KEYWORDS.contains(&current.to_lowercase().as_str());
                let is_at_field = current.starts_with('@');

                let style = if is_keyword {
                    Style::default().fg(Color::Blue)
                } else if is_at_field {
                    Style::default().add_modifier(Modifier::ITALIC)
                } else {
                    Style::default()
                };

                spans.push(Span::styled(current, style));
            }

            Line::from(spans)
        })
        .collect()
}
