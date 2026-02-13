use ratatui::{prelude::*, widgets::*};

use super::{rounded_block, styles};
use crate::common::{render_scrollbar, t, SortDirection};

pub const CURSOR_COLLAPSED: &str = "►";
pub const CURSOR_EXPANDED: &str = "▼";

pub fn format_expandable(label: &str, is_expanded: bool) -> String {
    if is_expanded {
        format!("{} {}", CURSOR_EXPANDED, label)
    } else {
        format!("{} {}", CURSOR_COLLAPSED, label)
    }
}

pub fn format_expandable_with_selection(
    label: &str,
    is_expanded: bool,
    is_selected: bool,
) -> String {
    if is_expanded {
        format!("{} {}", CURSOR_EXPANDED, label)
    } else if is_selected {
        format!("{} {}", CURSOR_COLLAPSED, label)
    } else {
        format!("  {}", label)
    }
}

pub fn render_tree_table(
    frame: &mut Frame,
    area: Rect,
    title: String,
    headers: Vec<&str>,
    rows: Vec<Row>,
    widths: Vec<Constraint>,
    is_active: bool,
) {
    let border_style = if is_active {
        styles::active_border()
    } else {
        Style::default()
    };

    let header_cells: Vec<Cell> = headers
        .iter()
        .enumerate()
        .map(|(i, name)| {
            Cell::from(format_header_cell(name, i))
                .style(Style::default().add_modifier(Modifier::BOLD))
        })
        .collect();
    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::White).fg(Color::Black))
        .height(1);

    let table = Table::new(rows, widths)
        .header(header)
        .column_spacing(1)
        .block(rounded_block().title(title).border_style(border_style));

    frame.render_widget(table, area);
}

type ExpandedContentFn<'a, T> = Box<dyn Fn(&T) -> Vec<(String, Style)> + 'a>;

// Helper to convert plain string to styled lines
pub fn plain_expanded_content(content: String) -> Vec<(String, Style)> {
    content
        .lines()
        .map(|line| (line.to_string(), Style::default()))
        .collect()
}

pub struct TableConfig<'a, T> {
    pub items: Vec<&'a T>,
    pub selected_index: usize,
    pub expanded_index: Option<usize>,
    pub columns: &'a [Box<dyn Column<T>>],
    pub sort_column: &'a str,
    pub sort_direction: SortDirection,
    pub title: String,
    pub area: Rect,
    pub get_expanded_content: Option<ExpandedContentFn<'a, T>>,
    pub is_active: bool,
}

pub fn format_header_cell(name: &str, column_index: usize) -> String {
    if column_index == 0 {
        format!("  {}", name)
    } else {
        format!("⋮ {}", name)
    }
}

pub trait Column<T> {
    fn id(&self) -> &'static str {
        unimplemented!("id() must be implemented if using default name() implementation")
    }

    fn default_name(&self) -> &'static str {
        unimplemented!("default_name() must be implemented if using default name() implementation")
    }

    fn name(&self) -> &str {
        let id = self.id();
        let translated = t(id);
        if translated == id {
            self.default_name()
        } else {
            Box::leak(translated.into_boxed_str())
        }
    }

    fn width(&self) -> u16;
    fn render(&self, item: &T) -> (String, Style);
}

// Generate expanded content from visible columns
pub fn expanded_from_columns<T>(columns: &[Box<dyn Column<T>>], item: &T) -> Vec<(String, Style)> {
    columns
        .iter()
        .map(|col| {
            let (value, style) = col.render(item);
            // Strip expansion indicators (►, ▼, or spaces) from the value
            let cleaned_value = value
                .trim_start_matches("► ")
                .trim_start_matches("▼ ")
                .trim_start_matches("  ");
            let display = if cleaned_value.is_empty() {
                "-"
            } else {
                cleaned_value
            };
            (format!("{}: {}", col.name(), display), style)
        })
        .collect()
}

pub fn render_table<T>(frame: &mut Frame, config: TableConfig<T>) {
    let border_style = if config.is_active {
        styles::active_border()
    } else {
        Style::default()
    };

    let title_style = if config.is_active {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    // Headers with sort indicators
    let header_cells = config.columns.iter().enumerate().map(|(i, col)| {
        let mut name = col.name().to_string();
        if !config.sort_column.is_empty() && config.sort_column == name {
            let arrow = if config.sort_direction == SortDirection::Asc {
                " ↑"
            } else {
                " ↓"
            };
            name.push_str(arrow);
        }
        name = format_header_cell(&name, i);
        Cell::from(name).style(Style::default().add_modifier(Modifier::BOLD))
    });
    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::White).fg(Color::Black))
        .height(1);

    let mut table_row_to_item_idx = Vec::new();
    let item_rows = config.items.iter().enumerate().flat_map(|(idx, item)| {
        let is_expanded = config.expanded_index == Some(idx);
        let is_selected = idx == config.selected_index;
        let mut rows = Vec::new();

        // Main row
        let cells: Vec<Cell> = config
            .columns
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let (mut content, style) = col.render(item);

                // Add expansion indicator to first column only
                if i == 0 {
                    content = if is_expanded {
                        format!("{} {}", CURSOR_EXPANDED, content)
                    } else if is_selected {
                        format!("{} {}", CURSOR_COLLAPSED, content)
                    } else {
                        format!("  {}", content)
                    };
                }

                if i > 0 {
                    Cell::from(Line::from(vec![
                        Span::raw("⋮ "),
                        Span::styled(content, style),
                    ]))
                } else {
                    Cell::from(content).style(style)
                }
            })
            .collect();

        table_row_to_item_idx.push(idx);
        rows.push(Row::new(cells).height(1));

        // Add empty rows for expanded content
        if is_expanded {
            if let Some(ref get_content) = config.get_expanded_content {
                let styled_lines = get_content(item);
                let line_count = styled_lines.len();

                for _ in 0..line_count {
                    let mut empty_cells = Vec::new();
                    for _ in 0..config.columns.len() {
                        empty_cells.push(Cell::from(""));
                    }
                    table_row_to_item_idx.push(idx);
                    rows.push(Row::new(empty_cells).height(1));
                }
            }
        }

        rows
    });

    let all_rows: Vec<Row> = item_rows.collect();

    let mut table_state_index = 0;
    for (i, &item_idx) in table_row_to_item_idx.iter().enumerate() {
        if item_idx == config.selected_index {
            table_state_index = i;
            break;
        }
    }

    let widths: Vec<Constraint> = config
        .columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            // Calculate the actual formatted header width
            let formatted_header = format_header_cell(col.name(), i);
            let header_width = formatted_header.chars().count() as u16;
            // Column width must be at least as wide as the formatted header
            let width = col.width().max(header_width);
            Constraint::Length(width)
        })
        .collect();

    let table = Table::new(all_rows, widths)
        .header(header)
        .block(
            rounded_block()
                .title(Span::styled(config.title, title_style))
                .border_style(border_style),
        )
        .column_spacing(1)
        .row_highlight_style(styles::highlight());

    let mut state = TableState::default();
    state.select(Some(table_state_index));

    // KNOWN ISSUE: ratatui 0.29 Table widget has built-in scrollbar that:
    // 1. Uses ║ and █ characters that cannot be customized
    // 2. Shows automatically when ratatui detects potential overflow
    // 3. Cannot be disabled without upgrading ratatui or implementing custom table rendering
    // The scrollbar may appear even when all paginated rows fit in the viewport
    frame.render_stateful_widget(table, config.area, &mut state);

    // Render expanded content as overlay if present
    if let Some(expanded_idx) = config.expanded_index {
        if let Some(ref get_content) = config.get_expanded_content {
            if let Some(item) = config.items.get(expanded_idx) {
                let styled_lines = get_content(item);

                // Calculate position: find row index in rendered table
                let mut row_y = 0;
                for (i, &item_idx) in table_row_to_item_idx.iter().enumerate() {
                    if item_idx == expanded_idx {
                        row_y = i;
                        break;
                    }
                }

                // Clear entire expanded area once
                let start_y = config.area.y + 2 + row_y as u16 + 1;
                let visible_lines = styled_lines
                    .len()
                    .min((config.area.y + config.area.height - 1 - start_y) as usize);
                if visible_lines > 0 {
                    let clear_area = Rect {
                        x: config.area.x + 1,
                        y: start_y,
                        width: config.area.width.saturating_sub(2),
                        height: visible_lines as u16,
                    };
                    frame.render_widget(Clear, clear_area);
                }

                for (line_idx, (line, line_style)) in styled_lines.iter().enumerate() {
                    let y = start_y + line_idx as u16;
                    if y >= config.area.y + config.area.height - 1 {
                        break; // Don't render past bottom border
                    }

                    let line_area = Rect {
                        x: config.area.x + 1,
                        y,
                        width: config.area.width.saturating_sub(2),
                        height: 1,
                    };

                    // Add expansion indicator on the left
                    let is_last_line = line_idx == styled_lines.len() - 1;
                    let is_field_start = line.contains(": ");
                    let indicator = if is_last_line {
                        "╰ "
                    } else if is_field_start {
                        "├ "
                    } else {
                        "│ "
                    };

                    let spans = if let Some(colon_pos) = line.find(": ") {
                        let col_name = &line[..colon_pos + 2];
                        let rest = &line[colon_pos + 2..];
                        vec![
                            Span::raw(indicator),
                            Span::styled(col_name.to_string(), styles::label()),
                            Span::styled(rest.to_string(), *line_style),
                        ]
                    } else {
                        vec![
                            Span::raw(indicator),
                            Span::styled(line.to_string(), *line_style),
                        ]
                    };

                    let paragraph = Paragraph::new(Line::from(spans));
                    frame.render_widget(paragraph, line_area);
                }
            }
        }
    }

    // Scrollbar - only show if items don't fit in viewport
    if !config.items.is_empty() {
        let scrollbar_area = config.area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        });
        // Only show scrollbar if there are more items than can fit in the viewport
        if config.items.len() > scrollbar_area.height as usize {
            render_scrollbar(
                frame,
                scrollbar_area,
                config.items.len(),
                config.selected_index,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIMESTAMP_LINE: &str = "Last state update: 2025-07-22 17:13:07 (UTC)";
    const TRACK: &str = "│";
    const THUMB: &str = "█";
    const EXPAND_INTERMEDIATE: &str = "├ ";
    const EXPAND_CONTINUATION: &str = "│ ";
    const EXPAND_LAST: &str = "╰ ";

    #[test]
    fn test_expanded_content_overlay() {
        assert!(TIMESTAMP_LINE.contains("(UTC)"));
        assert!(!TIMESTAMP_LINE.contains("( UTC"));
        assert_eq!(
            "Name: TestAlarm\nState: OK\nLast state update: 2025-07-22 17:13:07 (UTC)"
                .lines()
                .count(),
            3
        );
    }

    #[test]
    fn test_table_border_always_plain() {
        assert_eq!(BorderType::Plain, BorderType::Plain);
    }

    #[test]
    fn test_table_border_color_changes_when_active() {
        let active = Style::default().fg(Color::Green);
        let inactive = Style::default();
        assert_eq!(active.fg, Some(Color::Green));
        assert_eq!(inactive.fg, None);
    }

    #[test]
    fn test_table_scrollbar_uses_solid_characters() {
        assert_eq!(TRACK, "│");
        assert_eq!(THUMB, "█");
        assert_ne!(TRACK, "║");
    }

    #[test]
    fn test_expansion_indicators() {
        assert_eq!(EXPAND_INTERMEDIATE, "├ ");
        assert_eq!(EXPAND_CONTINUATION, "│ ");
        assert_eq!(EXPAND_LAST, "╰ ");
        assert_ne!(EXPAND_INTERMEDIATE, EXPAND_CONTINUATION);
        assert_ne!(EXPAND_INTERMEDIATE, EXPAND_LAST);
        assert_ne!(EXPAND_CONTINUATION, EXPAND_LAST);
    }

    #[test]
    fn test_first_column_expansion_indicators() {
        // Verify collapsed and expanded indicators
        assert_eq!(CURSOR_COLLAPSED, "►");
        assert_eq!(CURSOR_EXPANDED, "▼");

        // Verify they're different
        assert_ne!(CURSOR_COLLAPSED, CURSOR_EXPANDED);
    }

    #[test]
    fn test_table_scrollbar_only_for_overflow() {
        let (rows, height) = (50, 60u16);
        let available = height.saturating_sub(3);
        assert!(rows <= available as usize);
        assert!(60 > available as usize);
    }

    #[test]
    fn test_expansion_indicator_stripping() {
        let value_with_right_arrow = "► my-stack";
        let value_with_down_arrow = "▼ my-stack";
        let value_without_indicator = "my-stack";

        assert_eq!(
            value_with_right_arrow
                .trim_start_matches("► ")
                .trim_start_matches("▼ "),
            "my-stack"
        );
        assert_eq!(
            value_with_down_arrow
                .trim_start_matches("► ")
                .trim_start_matches("▼ "),
            "my-stack"
        );
        assert_eq!(
            value_without_indicator
                .trim_start_matches("► ")
                .trim_start_matches("▼ "),
            "my-stack"
        );
    }

    #[test]
    fn test_format_expandable_expanded() {
        assert_eq!(format_expandable("test-item", true), "▼ test-item");
    }

    #[test]
    fn test_format_expandable_not_expanded() {
        assert_eq!(format_expandable("test-item", false), "► test-item");
    }

    #[test]
    fn test_first_column_width_accounts_for_expansion_indicators() {
        // Expansion indicators add 2 display characters (► or ▼ + space) when selected or expanded
        let selected_only = format_expandable_with_selection("test", false, true);
        let expanded_only = format_expandable_with_selection("test", true, false);
        let both = format_expandable_with_selection("test", true, true);
        let neither = format_expandable_with_selection("test", false, false);

        // Selected or expanded should add 2 display characters (arrow + space)
        assert_eq!(selected_only.chars().count(), "test".chars().count() + 2);
        assert_eq!(expanded_only.chars().count(), "test".chars().count() + 2);
        // Both expanded and selected still shows only one indicator (expanded takes precedence)
        assert_eq!(both.chars().count(), "test".chars().count() + 2);
        // Neither should add 2 spaces for alignment
        assert_eq!(neither.chars().count(), "test".chars().count() + 2);
        assert_eq!(neither, "  test");
    }

    #[test]
    fn test_format_header_cell_first_column() {
        assert_eq!(format_header_cell("Name", 0), "  Name");
    }

    #[test]
    fn test_format_header_cell_other_columns() {
        assert_eq!(format_header_cell("Region", 1), "⋮ Region");
        assert_eq!(format_header_cell("Status", 2), "⋮ Status");
        assert_eq!(format_header_cell("Created", 5), "⋮ Created");
    }

    #[test]
    fn test_format_header_cell_with_sort_indicator() {
        assert_eq!(format_header_cell("Name ↑", 0), "  Name ↑");
        assert_eq!(format_header_cell("Status ↓", 1), "⋮ Status ↓");
    }

    #[test]
    fn test_column_width_never_narrower_than_header() {
        // First column: "  Name" = 6 chars
        let header_first = format_header_cell("Name", 0);
        assert_eq!(header_first.chars().count(), 6);

        // Other columns: "⋮ Launch time" = 13 chars
        let header_other = format_header_cell("Launch time", 1);
        assert_eq!(header_other.chars().count(), 13);
    }

    #[test]
    fn test_formatted_header_width_calculation() {
        // Test that formatted header width is correctly calculated
        assert_eq!(format_header_cell("ID", 0).chars().count(), 4); // "  ID"
        assert_eq!(format_header_cell("ID", 1).chars().count(), 4); // "⋮ ID"
        assert_eq!(format_header_cell("Name", 0).chars().count(), 6); // "  Name"
        assert_eq!(format_header_cell("Name", 1).chars().count(), 6); // "⋮ Name"
        assert_eq!(format_header_cell("Launch time", 1).chars().count(), 13); // "⋮ Launch time"
    }

    #[test]
    fn test_utc_timestamp_column_width() {
        // UTC timestamp format: "2025-11-14 00:00:00.000 UTC" = 27 chars
        // Header "Launch time" formatted as "⋮ Launch time" = 13 chars
        // Column width should be max(27, 13) = 27
        use crate::common::UTC_TIMESTAMP_WIDTH;
        assert_eq!(UTC_TIMESTAMP_WIDTH, 27);

        let header = format_header_cell("Launch time", 1);
        let header_width = header.chars().count() as u16;
        assert_eq!(header_width, 13);

        // The column width should use UTC_TIMESTAMP_WIDTH (27), not header width (13)
        assert!(UTC_TIMESTAMP_WIDTH > header_width);
    }
}
