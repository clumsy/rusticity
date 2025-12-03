use crate::keymap::Mode;
use crate::ui::{get_cursor, SEARCH_ICON};
use ratatui::{prelude::*, widgets::*};

pub struct FilterControl {
    pub text: String,
    pub is_focused: bool,
}

pub struct FilterConfig<'a> {
    pub filter_text: &'a str,
    pub placeholder: &'a str,
    pub mode: Mode,
    pub is_input_focused: bool,
    pub controls: Vec<FilterControl>,
    pub area: Rect,
}

/// Renders a filter bar with flush-right controls (like top bar datetime/profile)
pub fn render_filter_bar(frame: &mut Frame, config: FilterConfig) {
    let cursor = get_cursor(config.mode == Mode::FilterInput && config.is_input_focused);

    // Calculate actual controls width based on mode
    let controls_width = if config.mode == Mode::FilterInput {
        // In FilterInput mode: " ⋮ " + control1 + " ⋮ " + control2 + ...
        let separators_width = 3 + (config.controls.len().saturating_sub(1) * 3); // " ⋮ " = 3 chars
        let controls_text_width: usize = config.controls.iter().map(|c| c.text.len()).sum();
        (separators_width + controls_text_width) as u16
    } else {
        // In Normal mode: " ⋮ " + joined controls
        let controls_text: String = config
            .controls
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>()
            .join(" ⋮ ");
        (3 + controls_text.len()) as u16
    };

    // Split the filter area into left (input) and right (controls) like status bar
    let inner_area = Rect {
        x: config.area.x + 2,
        y: config.area.y + 1,
        width: config.area.width.saturating_sub(4),
        height: 1,
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(controls_width)])
        .split(inner_area);

    // Render border
    frame.render_widget(
        Block::default()
            .title(SEARCH_ICON)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if config.mode == Mode::FilterInput {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            }),
        config.area,
    );

    // Left side: filter input
    let mut input_spans = vec![];
    if config.filter_text.is_empty() {
        if config.mode == Mode::FilterInput {
            input_spans.push(Span::raw(""));
        } else {
            input_spans.push(Span::styled(
                config.placeholder,
                Style::default().fg(Color::DarkGray),
            ));
        }
    } else {
        input_spans.push(Span::raw(config.filter_text));
    }

    if config.mode == Mode::FilterInput && config.is_input_focused {
        input_spans.push(Span::styled(cursor, Style::default().fg(Color::Yellow)));
    }

    frame.render_widget(
        Paragraph::new(Line::from(input_spans))
            .alignment(Alignment::Left)
            .style(Style::default()),
        chunks[0],
    );

    // Right side: controls (flush right)
    let mut control_spans = vec![Span::raw(" ⋮ ")];
    if config.mode == Mode::FilterInput {
        let highlight = Style::default().fg(Color::Yellow);
        for (i, control) in config.controls.iter().enumerate() {
            if i > 0 {
                control_spans.push(Span::raw(" ⋮ "));
            }
            control_spans.push(Span::styled(
                &control.text,
                if control.is_focused {
                    highlight
                } else {
                    Style::default()
                },
            ));
        }
    } else {
        let controls_text: String = config
            .controls
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>()
            .join(" ⋮ ");
        control_spans.push(Span::styled(controls_text, Style::default()));
    }

    frame.render_widget(
        Paragraph::new(Line::from(control_spans))
            .alignment(Alignment::Right)
            .style(Style::default()),
        chunks[1],
    );
}

/// Helper to create a simple pagination-only filter
pub struct SimpleFilterConfig<'a> {
    pub filter_text: &'a str,
    pub placeholder: &'a str,
    pub pagination: &'a str,
    pub mode: Mode,
    pub is_input_focused: bool,
    pub is_pagination_focused: bool,
}

pub fn render_simple_filter(frame: &mut Frame, area: Rect, config: SimpleFilterConfig) {
    render_filter_bar(
        frame,
        FilterConfig {
            filter_text: config.filter_text,
            placeholder: config.placeholder,
            mode: config.mode,
            is_input_focused: config.is_input_focused,
            controls: vec![FilterControl {
                text: config.pagination.to_string(),
                is_focused: config.is_pagination_focused,
            }],
            area,
        },
    );
}
