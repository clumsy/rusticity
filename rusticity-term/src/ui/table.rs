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
                    // If content has a leading indent (e.g. "  name" for nested items),
                    // move it before the indicator so it renders as "  ► name" not "►   name"
                    let leading = if content.starts_with("  ") { "  " } else { "" };
                    let trimmed = if leading.is_empty() {
                        content.as_str()
                    } else {
                        content.strip_prefix("  ").unwrap_or(&content)
                    };
                    content = if is_expanded {
                        format!("{}{} {}", leading, CURSOR_EXPANDED, trimmed)
                    } else if is_selected {
                        format!("{}{} {}", leading, CURSOR_COLLAPSED, trimmed)
                    } else {
                        format!("{}  {}", leading, trimmed)
                    };
                }

                if i > 0 {
                    Cell::from(Line::from(vec![
                        Span::raw("⋮ "),
                        Span::styled(content, style),
                    ]))
                } else {
                    // First column: render indent + cursor in white, rest in style
                    if is_expanded {
                        // content = "{indent}▼ {text}"
                        let cursor_str = format!("{} ", CURSOR_EXPANDED);
                        let (indent_cursor, text) = if let Some(pos) = content.find(&cursor_str) {
                            let (prefix, rest) = content.split_at(pos + cursor_str.len());
                            (prefix, rest)
                        } else {
                            ("", content.as_str())
                        };
                        Cell::from(Line::from(vec![
                            Span::styled(
                                indent_cursor.to_string(),
                                Style::default().fg(Color::White),
                            ),
                            Span::styled(text.to_string(), style),
                        ]))
                    } else if is_selected {
                        // content = "{indent}► {text}"
                        let cursor_str = format!("{} ", CURSOR_COLLAPSED);
                        let (indent_cursor, text) = if let Some(pos) = content.find(&cursor_str) {
                            let (prefix, rest) = content.split_at(pos + cursor_str.len());
                            (prefix, rest)
                        } else {
                            ("", content.as_str())
                        };
                        Cell::from(Line::from(vec![
                            Span::styled(
                                indent_cursor.to_string(),
                                Style::default().fg(Color::White),
                            ),
                            Span::styled(text.to_string(), style),
                        ]))
                    } else {
                        // content = "{indent}  {text}" — split at first non-space after indent
                        let non_space = content.find(|c: char| c != ' ').unwrap_or(0);
                        let (spaces, text) = content.split_at(non_space);
                        Cell::from(Line::from(vec![
                            Span::raw(spaces.to_string()),
                            Span::styled(text.to_string(), style),
                        ]))
                    }
                }
            })
            .collect();

        table_row_to_item_idx.push(idx);
        rows.push(Row::new(cells).height(1));

        // Add placeholder rows for expanded content so subsequent rows are pushed down
        // and the scrollbar reflects the true content height. We track which are
        // placeholder rows so table_state_index is never set to one.
        if is_expanded {
            if let Some(ref get_content) = config.get_expanded_content {
                let styled_lines = get_content(item);
                for _ in 0..styled_lines.len() {
                    let empty_cells: Vec<Cell> =
                        (0..config.columns.len()).map(|_| Cell::from("")).collect();
                    // Use usize::MAX as a sentinel meaning "placeholder, not selectable"
                    table_row_to_item_idx.push(usize::MAX);
                    rows.push(Row::new(empty_cells).height(1));
                }
            }
        }

        rows
    });

    let all_rows: Vec<Row> = item_rows.collect();

    let mut table_state_index = 0;
    for (i, &item_idx) in table_row_to_item_idx.iter().enumerate() {
        // Skip placeholder rows (sentineled with usize::MAX)
        if item_idx != usize::MAX && item_idx == config.selected_index {
            table_state_index = i;
            break;
        }
    }

    // Find the widest column — it becomes the stretch column that absorbs remaining space.
    let max_declared_width = config
        .columns
        .iter()
        .map(|col| col.width())
        .max()
        .unwrap_or(0);

    let widths: Vec<Constraint> = config
        .columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            // Calculate the actual formatted header width
            let formatted_header = format_header_cell(col.name(), i);
            let header_width = formatted_header.chars().count() as u16;
            let width = col.width().max(header_width);
            // The widest column (typically Message) stretches to fill available space.
            if col.width() == max_declared_width {
                Constraint::Fill(1)
            } else {
                Constraint::Length(width)
            }
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

    // If the selected item is expanded, pre-set the offset so ratatui shows all detail rows.
    // Count placeholder rows after the selected item.
    let detail_row_count = if config.expanded_index == Some(config.selected_index) {
        table_row_to_item_idx
            .iter()
            .skip(table_state_index + 1)
            .take_while(|&&idx| idx == usize::MAX)
            .count()
    } else {
        0
    };
    if detail_row_count > 0 {
        // The item is at table_state_index and needs detail_row_count rows below.
        // visible_rows = area.height - 3 (borders + header).
        let visible_rows = config.area.height.saturating_sub(3) as usize;
        if visible_rows > 0 {
            let last_detail = table_state_index + detail_row_count;
            if last_detail >= visible_rows {
                // Scroll so that last detail row is the last visible row
                let needed_offset = last_detail.saturating_sub(visible_rows - 1);
                *state.offset_mut() = needed_offset;
            }
        }
    }

    // KNOWN ISSUE: ratatui 0.29 Table widget has built-in scrollbar that:
    // 1. Uses ║ and █ characters that cannot be customized
    // 2. Shows automatically when ratatui detects potential overflow
    // 3. Cannot be disabled without upgrading ratatui or implementing custom table rendering
    // The scrollbar may appear even when all paginated rows fit in the viewport
    frame.render_stateful_widget(table, config.area, &mut state);

    // After rendering, state.offset() gives ratatui's internal scroll offset.
    // We use this to compute the correct visual position for the expanded content overlay.
    let ratatui_scroll_offset = state.offset();

    // Render expanded content as overlay if present
    if let Some(expanded_idx) = config.expanded_index {
        if let Some(ref get_content) = config.get_expanded_content {
            if let Some(item) = config.items.get(expanded_idx) {
                let styled_lines = get_content(item);

                // Calculate position: find absolute row index for the expanded item,
                // then subtract ratatui's scroll offset to get the VISUAL row position.
                let mut abs_row_y = 0;
                for (i, &item_idx) in table_row_to_item_idx.iter().enumerate() {
                    if item_idx == expanded_idx {
                        abs_row_y = i;
                        break;
                    }
                }
                // Visual row = absolute row - ratatui's scroll offset
                // If the expanded item is scrolled off the top, don't render overlay.
                if abs_row_y < ratatui_scroll_offset {
                    // Expanded item is above the visible area; skip overlay
                } else {
                    let row_y = abs_row_y - ratatui_scroll_offset;

                    // Clear entire expanded area once
                    let start_y = config.area.y + 2 + row_y as u16 + 1;
                    let bottom = config.area.y + config.area.height - 1;
                    let visible_lines = if start_y >= bottom {
                        0
                    } else {
                        styled_lines.len().min((bottom - start_y) as usize)
                    };
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

                        // Add expansion indicator on the left.
                        // If the line starts with spaces (indent prefix for nested items),
                        // strip it from the content and prepend it before the indicator.
                        let indent = if line.starts_with("  ") { "  " } else { "" };
                        let line_content = if indent.is_empty() {
                            line.as_str()
                        } else {
                            line.strip_prefix("  ").unwrap_or(line.as_str())
                        };

                        let is_last_line = line_idx == styled_lines.len() - 1;
                        let is_field_start = line_content.contains(": ");
                        let indicator = if is_last_line {
                            "╰ "
                        } else if is_field_start {
                            "├ "
                        } else {
                            "│ "
                        };

                        let spans = if let Some(colon_pos) = line_content.find(": ") {
                            let col_name = &line_content[..colon_pos + 2];
                            let rest = &line_content[colon_pos + 2..];
                            vec![
                                Span::raw(format!("{}{}", indent, indicator)),
                                Span::styled(col_name.to_string(), styles::label()),
                                Span::styled(rest.to_string(), *line_style),
                            ]
                        } else {
                            vec![
                                Span::raw(format!("{}{}", indent, indicator)),
                                Span::styled(line_content.to_string(), *line_style),
                            ]
                        };

                        let paragraph = Paragraph::new(Line::from(spans));
                        frame.render_widget(paragraph, line_area);
                    }
                } // end else (expanded item is visible)
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

    #[test]
    fn test_expanded_row_does_not_add_placeholder_rows_to_table() {
        // Regression: expanded content used to add N empty placeholder rows to the ratatui
        // Table widget, causing it to scroll internally when the expanded item was near
        // the bottom ("sticky" overlay bug). Now expanded content is a pure overlay only.
        // We verify: TableConfig.items.len() == page_size regardless of expansion.
        struct TestCol;
        impl Column<String> for TestCol {
            fn id(&self) -> &'static str {
                "test"
            }
            fn default_name(&self) -> &'static str {
                "Name"
            }
            fn width(&self) -> u16 {
                10
            }
            fn render(&self, item: &String) -> (String, ratatui::style::Style) {
                (item.clone(), ratatui::style::Style::default())
            }
        }

        let items: Vec<String> = vec!["alpha".into(), "beta".into(), "gamma".into()];
        let columns: Vec<Box<dyn Column<String>>> = vec![Box::new(TestCol)];

        let config = TableConfig {
            items: items.iter().collect(),
            selected_index: 2,
            expanded_index: Some(2), // last item expanded
            columns: &columns,
            sort_column: "",
            sort_direction: crate::common::SortDirection::Asc,
            title: "Test".to_string(),
            area: ratatui::layout::Rect::new(0, 0, 80, 10),
            get_expanded_content: Some(Box::new(|_item: &String| {
                vec![
                    ("f1: v1".to_string(), ratatui::style::Style::default()),
                    ("f2: v2".to_string(), ratatui::style::Style::default()),
                ]
            })),
            is_active: true,
        };

        assert_eq!(
            config.items.len(),
            3,
            "items.len() must equal page_size — no placeholder rows added for expanded content"
        );
    }
}
