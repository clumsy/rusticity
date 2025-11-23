use chrono::{DateTime, Utc};
use ratatui::{prelude::*, widgets::*};

use crate::ui::{filter_area, styles};

// Width for UTC timestamp format: "YYYY-MM-DD HH:MM:SS (UTC)"
pub const UTC_TIMESTAMP_WIDTH: u16 = 27;

pub fn format_timestamp(dt: &DateTime<Utc>) -> String {
    format!("{} (UTC)", dt.format("%Y-%m-%d %H:%M:%S"))
}

pub fn format_optional_timestamp(dt: Option<DateTime<Utc>>) -> String {
    dt.map(|t| format_timestamp(&t))
        .unwrap_or_else(|| "-".to_string())
}

pub fn format_iso_timestamp(iso_string: &str) -> String {
    if iso_string.is_empty() {
        return "-".to_string();
    }

    // Parse ISO 8601 format (e.g., "2024-01-01T12:30:45.123Z")
    if let Ok(dt) = DateTime::parse_from_rfc3339(iso_string) {
        format_timestamp(&dt.with_timezone(&Utc))
    } else {
        iso_string.to_string()
    }
}

pub fn format_unix_timestamp(unix_string: &str) -> String {
    if unix_string.is_empty() {
        return "-".to_string();
    }

    if let Ok(timestamp) = unix_string.parse::<i64>() {
        if let Some(dt) = DateTime::from_timestamp(timestamp, 0) {
            format_timestamp(&dt)
        } else {
            unix_string.to_string()
        }
    } else {
        unix_string.to_string()
    }
}

pub trait ColumnTrait {
    fn name(&self) -> &'static str;

    // Future: column type for formatting
    fn column_type(&self) -> ColumnType {
        ColumnType::String
    }

    // Future: translation key for i18n
    fn translation_key(&self) -> Option<&'static str> {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColumnType {
    String,
    Number,
    DateTime,
    Boolean,
}

#[macro_export]
macro_rules! column {
    (name=$name:expr, width=$width:expr, type=$item_type:ty, render_styled=$render:expr) => {{
        struct __Column;
        impl $crate::ui::table::Column<$item_type> for __Column {
            fn name(&self) -> &str {
                $name
            }
            fn width(&self) -> u16 {
                ($width).max($name.len() as u16)
            }
            fn render(&self, item: &$item_type) -> (String, Style) {
                $render(item)
            }
        }
        __Column
    }};
    (name=$name:expr, type=$item_type:ty, render_styled=$render:expr) => {{
        column!(name=$name, width=0, type=$item_type, render_styled=$render)
    }};
    (name=$name:expr, width=$width:expr, type=$item_type:ty, render_expanded=$render:expr) => {{
        struct __Column;
        impl $crate::ui::table::Column<$item_type> for __Column {
            fn name(&self) -> &str {
                $name
            }
            fn width(&self) -> u16 {
                ($width).max($name.len() as u16)
            }
            fn render(&self, item: &$item_type) -> (String, Style) {
                ($render(item), Style::default())
            }
        }
        __Column
    }};
    (name=$name:expr, type=$item_type:ty, render_expanded=$render:expr) => {{
        column!(name=$name, width=0, type=$item_type, render_expanded=$render)
    }};
    (name=$name:expr, width=$width:expr, type=$item_type:ty, render=$render:expr) => {{
        struct __Column;
        impl $crate::ui::table::Column<$item_type> for __Column {
            fn name(&self) -> &str {
                $name
            }
            fn width(&self) -> u16 {
                ($width).max($name.len() as u16)
            }
            fn render(&self, item: &$item_type) -> (String, Style) {
                ($render(item), Style::default())
            }
        }
        __Column
    }};
    (name=$name:expr, type=$item_type:ty, render=$render:expr) => {{
        column!(name=$name, width=0, type=$item_type, render=$render)
    }};
    (name=$name:expr, width=$width:expr, type=$item_type:ty, field=$field:ident) => {{
        struct __Column;
        impl $crate::ui::table::Column<$item_type> for __Column {
            fn name(&self) -> &str {
                $name
            }
            fn width(&self) -> u16 {
                ($width).max($name.len() as u16)
            }
            fn render(&self, item: &$item_type) -> (String, Style) {
                (item.$field.clone(), Style::default())
            }
        }
        __Column
    }};
    (name=$name:expr, type=$item_type:ty, field=$field:ident) => {{
        column!(name=$name, width=0, type=$item_type, field=$field)
    }};
    ($name:expr, $width:expr, $item_type:ty, $field:ident) => {{
        struct __Column;
        impl $crate::ui::table::Column<$item_type> for __Column {
            fn name(&self) -> &str {
                $name
            }
            fn width(&self) -> u16 {
                ($width).max($name.len() as u16)
            }
            fn render(&self, item: &$item_type) -> (String, Style) {
                (item.$field.clone(), Style::default())
            }
        }
        __Column
    }};
}

pub fn format_bytes(bytes: i64) -> String {
    const KB: i64 = 1000;
    const MB: i64 = KB * 1000;
    const GB: i64 = MB * 1000;
    const TB: i64 = GB * 1000;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn format_memory_mb(mb: i32) -> String {
    if mb >= 1024 {
        format!("{} GB", mb / 1024)
    } else {
        format!("{} MB", mb)
    }
}

pub fn format_duration_seconds(seconds: i32) -> String {
    let minutes = seconds / 60;
    let secs = seconds % 60;
    if minutes > 0 {
        format!("{}min {}sec", minutes, secs)
    } else {
        format!("{}sec", secs)
    }
}

pub fn border_style(is_active: bool) -> Style {
    if is_active {
        styles::active_border()
    } else {
        Style::default()
    }
}

pub fn render_scrollbar(frame: &mut Frame, area: Rect, total: usize, position: usize) {
    if total == 0 {
        return;
    }
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    let mut state = ScrollbarState::new(total).position(position);
    frame.render_stateful_widget(scrollbar, area, &mut state);
}

pub fn render_vertical_scrollbar(frame: &mut Frame, area: Rect, total: usize, position: usize) {
    render_scrollbar(frame, area, total, position);
}

pub fn render_horizontal_scrollbar(frame: &mut Frame, area: Rect, position: usize, total: usize) {
    let scrollbar = Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
        .begin_symbol(Some("◀"))
        .end_symbol(Some("▶"));
    let mut state = ScrollbarState::new(total).position(position);
    frame.render_stateful_widget(scrollbar, area, &mut state);
}

pub fn render_pagination(current: usize, total: usize) -> String {
    if total == 0 {
        return "[1]".to_string();
    }
    if total <= 10 {
        return (0..total)
            .map(|i| {
                if i == current {
                    format!("[{}]", i + 1)
                } else {
                    format!("{}", i + 1)
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
    }
    let start = current.saturating_sub(4);
    let end = (start + 9).min(total);
    let start = if end == total {
        total.saturating_sub(9)
    } else {
        start
    };
    (start..end)
        .map(|i| {
            if i == current {
                format!("[{}]", i + 1)
            } else {
                format!("{}", i + 1)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn render_pagination_text(current: usize, total: usize) -> String {
    render_pagination(current, total)
}

pub struct FilterConfig<'a> {
    pub text: &'a str,
    pub placeholder: &'a str,
    pub is_active: bool,
    pub right_content: Vec<(&'a str, &'a str)>,
    pub area: Rect,
}

pub struct FilterAreaConfig<'a> {
    pub filter_text: &'a str,
    pub placeholder: &'a str,
    pub mode: crate::keymap::Mode,
    pub input_focus: FilterFocusType,
    pub controls: Vec<FilterControl>,
    pub area: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

impl SortDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum InputFocus {
    #[default]
    Filter,
    Pagination,
    Dropdown(&'static str),
    Checkbox(&'static str),
}

impl InputFocus {
    pub fn next(&self, controls: &[InputFocus]) -> Self {
        if controls.is_empty() {
            return *self;
        }
        let idx = controls.iter().position(|f| f == self).unwrap_or(0);
        controls[(idx + 1) % controls.len()]
    }

    pub fn prev(&self, controls: &[InputFocus]) -> Self {
        if controls.is_empty() {
            return *self;
        }
        let idx = controls.iter().position(|f| f == self).unwrap_or(0);
        controls[(idx + controls.len() - 1) % controls.len()]
    }

    /// Navigate to next page when pagination is focused
    pub fn handle_page_down(
        &self,
        selected: &mut usize,
        scroll_offset: &mut usize,
        page_size: usize,
        filtered_count: usize,
    ) {
        if *self == InputFocus::Pagination {
            let max_offset = filtered_count.saturating_sub(page_size);
            *selected = (*selected + page_size).min(max_offset);
            *scroll_offset = *selected;
        }
    }

    /// Navigate to previous page when pagination is focused
    pub fn handle_page_up(
        &self,
        selected: &mut usize,
        scroll_offset: &mut usize,
        page_size: usize,
    ) {
        if *self == InputFocus::Pagination {
            *selected = selected.saturating_sub(page_size);
            *scroll_offset = *selected;
        }
    }
}

pub trait CyclicEnum: Copy + PartialEq + Sized + 'static {
    const ALL: &'static [Self];

    fn next(&self) -> Self {
        let idx = Self::ALL.iter().position(|x| x == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    fn prev(&self) -> Self {
        let idx = Self::ALL.iter().position(|x| x == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(PartialEq)]
pub enum FilterFocusType {
    Input,
    Control(usize),
}

pub struct FilterControl {
    pub text: String,
    pub is_focused: bool,
    pub style: ratatui::style::Style,
}

pub fn render_filter_area(frame: &mut Frame, config: FilterAreaConfig) {
    use crate::keymap::Mode;
    use crate::ui::get_cursor;
    use ratatui::prelude::*;

    let cursor = get_cursor(
        config.mode == Mode::FilterInput && config.input_focus == FilterFocusType::Input,
    );
    let filter_width = config.area.width.saturating_sub(4) as usize;

    // Calculate controls text
    let controls_text: String = config
        .controls
        .iter()
        .map(|c| c.text.as_str())
        .collect::<Vec<_>>()
        .join(" ⋮ ");
    let controls_len = controls_text.len();

    let placeholder_len = config.placeholder.len();
    let content_len =
        if config.filter_text.is_empty() && config.mode != Mode::FilterInput {
            placeholder_len
        } else {
            config.filter_text.len()
        } + if config.mode == Mode::FilterInput && config.input_focus == FilterFocusType::Input {
            cursor.len()
        } else {
            0
        };

    let available_space = filter_width.saturating_sub(controls_len + 1);

    let mut line_spans = vec![];
    if config.filter_text.is_empty() {
        if config.mode == Mode::FilterInput {
            line_spans.push(Span::raw(""));
        } else {
            line_spans.push(Span::styled(
                config.placeholder,
                Style::default().fg(Color::DarkGray),
            ));
        }
    } else {
        line_spans.push(Span::raw(config.filter_text));
    }

    if config.mode == Mode::FilterInput && config.input_focus == FilterFocusType::Input {
        line_spans.push(Span::styled(cursor, Style::default().fg(Color::Yellow)));
    }

    if content_len < available_space {
        line_spans.push(Span::raw(" ".repeat(available_space - content_len)));
    }

    if config.mode == Mode::FilterInput {
        for control in &config.controls {
            line_spans.push(Span::raw(" ⋮ "));
            line_spans.push(Span::styled(&control.text, control.style));
        }
    } else {
        line_spans.push(Span::styled(
            format!(" ⋮ {}", controls_text),
            Style::default(),
        ));
    }

    let filter = filter_area(line_spans, config.mode == Mode::FilterInput);
    frame.render_widget(filter, config.area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_format_timestamp() {
        let dt = Utc.with_ymd_and_hms(2025, 11, 12, 14, 30, 45).unwrap();
        assert_eq!(format_timestamp(&dt), "2025-11-12 14:30:45 (UTC)");
    }

    #[test]
    fn test_format_optional_timestamp_some() {
        let dt = Utc.with_ymd_and_hms(2025, 11, 12, 14, 30, 45).unwrap();
        assert_eq!(
            format_optional_timestamp(Some(dt)),
            "2025-11-12 14:30:45 (UTC)"
        );
    }

    #[test]
    fn test_format_optional_timestamp_none() {
        assert_eq!(format_optional_timestamp(None), "-");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1500), "1.50 KB");
        assert_eq!(format_bytes(1_500_000), "1.50 MB");
        assert_eq!(format_bytes(1_500_000_000), "1.50 GB");
        assert_eq!(format_bytes(1_500_000_000_000), "1.50 TB");
    }

    #[test]
    fn test_render_pagination_single_page() {
        assert_eq!(render_pagination(0, 1), "[1]");
    }

    #[test]
    fn test_render_pagination_two_pages() {
        assert_eq!(render_pagination(0, 2), "[1] 2");
        assert_eq!(render_pagination(1, 2), "1 [2]");
    }

    #[test]
    fn test_render_pagination_ten_pages() {
        assert_eq!(render_pagination(0, 10), "[1] 2 3 4 5 6 7 8 9 10");
        assert_eq!(render_pagination(5, 10), "1 2 3 4 5 [6] 7 8 9 10");
        assert_eq!(render_pagination(9, 10), "1 2 3 4 5 6 7 8 9 [10]");
    }

    #[test]
    fn test_format_memory_mb() {
        assert_eq!(format_memory_mb(128), "128 MB");
        assert_eq!(format_memory_mb(512), "512 MB");
        assert_eq!(format_memory_mb(1024), "1 GB");
        assert_eq!(format_memory_mb(2048), "2 GB");
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration_seconds(30), "30sec");
        assert_eq!(format_duration_seconds(40), "40sec");
        assert_eq!(format_duration_seconds(60), "1min 0sec");
        assert_eq!(format_duration_seconds(90), "1min 30sec");
        assert_eq!(format_duration_seconds(900), "15min 0sec");
    }

    #[test]
    fn test_render_pagination_many_pages() {
        assert_eq!(render_pagination(0, 20), "[1] 2 3 4 5 6 7 8 9");
        assert_eq!(render_pagination(5, 20), "2 3 4 5 [6] 7 8 9 10");
        assert_eq!(render_pagination(15, 20), "12 13 14 15 [16] 17 18 19 20");
        assert_eq!(render_pagination(19, 20), "12 13 14 15 16 17 18 19 [20]");
    }

    #[test]
    fn test_render_pagination_zero_total() {
        assert_eq!(render_pagination(0, 0), "[1]");
    }
}

pub fn render_filter(frame: &mut Frame, config: FilterConfig) {
    let cursor = if config.is_active { "█" } else { "" };
    let content = if config.text.is_empty() && !config.is_active {
        config.placeholder
    } else {
        config.text
    };

    let right_text = config
        .right_content
        .iter()
        .map(|(k, v)| format!("{}: {}", k, v))
        .collect::<Vec<_>>()
        .join(" ⋮ ");

    let width = (config.area.width as usize).saturating_sub(4);
    let right_len = right_text.len();
    let content_len = content.len() + if config.is_active { cursor.len() } else { 0 };
    let available = width.saturating_sub(right_len + 3);

    let display = if content_len > available {
        &content[content_len.saturating_sub(available)..]
    } else {
        content
    };

    let style = if config.is_active {
        styles::yellow()
    } else {
        styles::placeholder()
    };

    let mut spans = vec![Span::styled(display, style)];
    if config.is_active {
        spans.push(Span::styled(cursor, styles::cursor()));
    }

    let padding = " ".repeat(
        width
            .saturating_sub(display.len())
            .saturating_sub(if config.is_active { cursor.len() } else { 0 })
            .saturating_sub(right_len)
            .saturating_sub(3),
    );

    spans.push(Span::raw(padding));
    spans.push(Span::styled(format!(" {}", right_text), styles::cyan()));

    frame.render_widget(
        Paragraph::new(Line::from(spans)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style(config.is_active)),
        ),
        config.area,
    );
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageSize {
    Ten,
    TwentyFive,
    Fifty,
    OneHundred,
}
