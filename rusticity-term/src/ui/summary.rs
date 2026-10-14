//! Content-sized, styled detail summaries using the shared field layout.
use super::{
    calculate_dynamic_height, labeled_field, render_fields_with_dynamic_columns, titled_block,
};
use ratatui::prelude::*;

pub trait SummaryField {
    fn summary_line(&self) -> Line<'_>;
}
impl SummaryField for Line<'_> {
    fn summary_line(&self) -> Line<'_> {
        self.clone()
    }
}
impl SummaryField for (&str, String) {
    fn summary_line(&self) -> Line<'_> {
        labeled_field(self.0, self.1.clone())
    }
}

pub struct Summary<'a> {
    title: &'a str,
    lines: Vec<Line<'a>>,
}
impl<'a> Summary<'a> {
    pub fn new(title: &'a str, lines: Vec<Line<'a>>) -> Self {
        Self { title, lines }
    }
    /// Outer height including exactly the block's two border rows.
    pub fn height(&self, width: u16) -> u16 {
        calculate_dynamic_height(&self.lines, width.saturating_sub(2)).saturating_add(2)
    }
    /// Fits content within the provided area and returns the area consumed.
    pub fn render(self, frame: &mut Frame, area: Rect) -> Rect {
        let area = Rect {
            height: self.height(area.width).min(area.height),
            ..area
        };
        self.render_fill(frame, area);
        area
    }
    /// Explicit opt-in for panes that intentionally fill their available space.
    pub fn render_fill(self, frame: &mut Frame, area: Rect) {
        let block = titled_block(self.title);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        render_fields_with_dynamic_columns(frame, inner, self.lines);
    }
}

pub fn render_summary<F: SummaryField>(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    fields: &[F],
) -> Rect {
    Summary::new(
        title,
        fields.iter().map(SummaryField::summary_line).collect(),
    )
    .render(frame, area)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn summary_fits_content_and_preserves_styles_and_unicode() {
        for width in [20, 45, 180] {
            let lines = vec![
                Line::from(vec![
                    Span::raw("State: "),
                    Span::styled("✅ Available", Style::default().fg(Color::Green)),
                ]),
                Line::raw("Name: 資源"),
                Line::raw("Owner: test"),
            ];
            let height = Summary::new("General", lines.clone()).height(width);
            let mut terminal = Terminal::new(TestBackend::new(width, 20)).unwrap();
            terminal
                .draw(|frame| {
                    let used = render_summary(frame, frame.area(), "General", &lines);
                    assert_eq!(used.height, height);
                })
                .unwrap();
            let buffer = terminal.backend().buffer();
            assert_eq!(buffer[(0, height - 1)].symbol(), "╰");
            assert_eq!(buffer[(0, height)].symbol(), " ");
            assert!(buffer
                .content()
                .iter()
                .any(|c| c.symbol().contains('✅') && c.fg == Color::Green));
        }
    }

    #[test]
    fn summary_accepts_plain_fields_and_clamps_tiny_areas() {
        let mut terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();
        terminal
            .draw(|frame| {
                let fields = [("Name", "example".to_string())];
                let used = render_summary(frame, Rect::new(3, 2, 40, 8), "Summary", &fields);
                assert_eq!(used, Rect::new(3, 2, 40, 3));
                for area in [Rect::new(0, 0, 0, 0), Rect::new(0, 0, 1, 1)] {
                    let used = render_summary(frame, area, "Tiny", &fields);
                    assert!(used.height <= area.height);
                }
            })
            .unwrap();
    }
}
