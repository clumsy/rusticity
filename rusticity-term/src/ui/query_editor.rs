use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use super::styles;

pub struct QueryEditorConfig<'a> {
    pub query_text: &'a str,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub is_active: bool,
    pub title: &'a str,
    pub area: Rect,
}

pub fn render_query_editor(frame: &mut Frame, config: QueryEditorConfig) {
    let (border_style, border_type) = if config.is_active {
        (styles::active_border(), BorderType::Double)
    } else {
        (Style::default(), BorderType::Plain)
    };

    let lines: Vec<Line> = config
        .query_text
        .lines()
        .enumerate()
        .map(|(line_idx, line_text)| {
            if config.is_active && line_idx == config.cursor_line {
                let before = &line_text[..config.cursor_col.min(line_text.len())];
                let cursor_char = line_text
                    .chars()
                    .nth(config.cursor_col)
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| " ".to_string());
                let after =
                    &line_text[(config.cursor_col + cursor_char.len()).min(line_text.len())..];

                Line::from(vec![
                    Span::raw(before),
                    Span::styled(cursor_char, styles::bg_white().fg(Color::Black)),
                    Span::raw(after),
                ])
            } else {
                Line::from(line_text.to_string())
            }
        })
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(config.title)
                .borders(Borders::ALL)
                .border_style(border_style)
                .border_type(border_type),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, config.area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_editor_config_creation() {
        let config = QueryEditorConfig {
            query_text: "SELECT * FROM logs",
            cursor_line: 0,
            cursor_col: 5,
            is_active: true,
            title: "Query",
            area: Rect::new(0, 0, 80, 10),
        };

        assert_eq!(config.query_text, "SELECT * FROM logs");
        assert_eq!(config.cursor_line, 0);
        assert_eq!(config.cursor_col, 5);
        assert!(config.is_active);
    }

    #[test]
    fn test_query_editor_multiline() {
        assert_eq!(
            "fields @timestamp\n| sort @timestamp desc\n| limit 100"
                .lines()
                .count(),
            3
        );
    }
}
