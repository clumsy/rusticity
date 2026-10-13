use crate::app::App;
use crate::common::SortDirection;
use crate::finder::Focus;
use crate::ui::table::{render_table, Column, TableConfig};
use crate::ui::{filter_area, format_title};
use ratatui::{
    prelude::*,
    widgets::{Clear, Paragraph},
};
use rusticity_core::finder::Resource;

pub const FILTER_PLACEHOLDER: &str =
    "Type at least 3 search characters; @s3, @dynamodb, @ec2, @rds, @iam, @lambda limit services";

enum FinderColumn {
    Service,
    Type,
    Id,
    Name,
    Region,
}
impl Column<Resource> for FinderColumn {
    fn name(&self) -> &str {
        match self {
            Self::Service => "Service",
            Self::Type => "Resource type",
            Self::Id => "ID",
            Self::Name => "Name",
            Self::Region => "Region",
        }
    }
    fn width(&self) -> u16 {
        match self {
            Self::Service => 16,
            Self::Type => 16,
            Self::Id => 48,
            Self::Name => 32,
            Self::Region => 14,
        }
    }
    fn render(&self, resource: &Resource) -> (String, Style) {
        let text = match self {
            Self::Service => resource.kind.service(),
            Self::Type => resource.kind.label(),
            Self::Id => &resource.id,
            Self::Name => &resource.name,
            Self::Region => &resource.region,
        };
        (text.into(), Style::default())
    }
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let popup = area.centered(Constraint::Percentage(90), Constraint::Percentage(80));
    frame.render_widget(Clear, popup);
    let finder = &app.resource_finder;
    let message = finder.message();
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(u16::from(!message.is_empty())),
    ])
    .split(popup);
    let cursor = Span::styled("█", Style::default().fg(Color::Green));
    let mut spans = Vec::new();
    if finder.input.is_empty() {
        if finder.focus == Focus::Filter {
            spans.push(cursor);
        }
        spans.push(Span::styled(
            FILTER_PLACEHOLDER,
            Style::default().fg(Color::DarkGray),
        ));
    } else {
        spans.push(Span::raw(&finder.input));
        if finder.focus == Focus::Filter {
            spans.push(cursor);
        }
    }
    frame.render_widget(filter_area(spans, finder.focus == Focus::Filter), chunks[0]);
    let columns: Vec<Box<dyn Column<Resource>>> = vec![
        Box::new(FinderColumn::Service),
        Box::new(FinderColumn::Type),
        Box::new(FinderColumn::Id),
        Box::new(FinderColumn::Name),
        Box::new(FinderColumn::Region),
    ];
    render_table(
        frame,
        TableConfig {
            items: finder.rows.iter().collect(),
            selected_index: finder.selected,
            expanded_index: None,
            columns: &columns,
            sort_column: "",
            sort_direction: SortDirection::Asc,
            title: format_title("AWS Resources"),
            area: chunks[1],
            get_expanded_content: None,
            is_active: finder.focus == Focus::Table,
        },
    );
    if !message.is_empty() {
        frame.render_widget(
            Paragraph::new(message).style(Style::default().fg(if finder.warnings.is_empty() {
                Color::DarkGray
            } else {
                Color::Yellow
            })),
            chunks[2],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{finder, keymap::Action};
    use ratatui::{backend::TestBackend, Terminal};

    fn app() -> App {
        let mut app = App::new_without_client("test".into(), Some("us-east-2".into()));
        finder::open(&mut app);
        app
    }

    fn draw(app: &App, width: u16, height: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| render(f, app, f.area())).unwrap();
        terminal.backend().buffer().clone()
    }

    fn row(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect()
    }

    #[test]
    fn finder_placeholder_is_gray_in_both_focus_states_and_never_input_data() {
        let mut app = app();
        let popup = Rect::new(0, 0, 180, 30)
            .centered(Constraint::Percentage(90), Constraint::Percentage(80));
        for focus in [Focus::Filter, Focus::Table] {
            app.resource_finder.focus = focus;
            let buffer = draw(&app, 180, 30);
            let text = row(&buffer, popup.y + 1);
            assert!(text.contains(FILTER_PLACEHOLDER));
            assert_eq!(text.contains('█'), focus == Focus::Filter);
            let start = popup.x + 1 + u16::from(focus == Focus::Filter);
            for offset in 0..FILTER_PLACEHOLDER.len() as u16 {
                assert_eq!(buffer[(start + offset, popup.y + 1)].fg, Color::DarkGray);
            }
            assert!(app.resource_finder.input.is_empty());
            assert!(app.resource_finder.message().is_empty());
            assert!(!app.resource_finder.searching);
            assert_eq!(
                buffer[(popup.x, popup.y)].fg == Color::Green,
                focus == Focus::Filter
            );
            assert_eq!(
                buffer[(popup.x, popup.y + 3)].fg == Color::Green,
                focus == Focus::Table
            );
        }
    }

    #[test]
    fn finder_placeholder_disappears_on_first_character_and_returns_when_cleared() {
        let mut app = app();
        let popup = Rect::new(0, 0, 180, 30)
            .centered(Constraint::Percentage(90), Constraint::Percentage(80));
        app.handle_action(Action::FilterInput('a'));
        let buffer = draw(&app, 180, 30);
        assert_eq!(app.resource_finder.input, "a");
        assert!(row(&buffer, popup.y + 1).contains("a█"));
        assert!(!row(&buffer, popup.y + 1).contains("Type at least"));
        assert_ne!(buffer[(popup.x + 1, popup.y + 1)].fg, Color::DarkGray);
        assert!(app.resource_finder.message().is_empty());
        app.handle_action(Action::FilterBackspace);
        assert!(row(&draw(&app, 180, 30), popup.y + 1).contains(FILTER_PLACEHOLDER));
        assert!(app.resource_finder.input.is_empty());
    }

    #[test]
    fn finder_idle_has_no_footer_rows_and_messages_use_just_one_row() {
        let mut app = app();
        let popup = Rect::new(0, 0, 180, 30)
            .centered(Constraint::Percentage(90), Constraint::Percentage(80));
        let buffer = draw(&app, 180, 30);
        assert_eq!(buffer[(popup.x, popup.bottom() - 1)].symbol(), "╰");
        let text: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert_eq!(text.matches("Type at least").count(), 1);
        assert!(!text.contains("Enter / Tab:"));
        assert!(!text.contains("Esc: cancel"));
        assert!(!text.contains("Ctrl+R:"));

        for c in "orders".chars() {
            app.handle_action(Action::FilterInput(c));
        }
        app.resource_finder.provider = "Indexed prefix search".into();
        app.resource_finder.searching = true;
        let buffer = draw(&app, 180, 30);
        assert_eq!(buffer[(popup.x, popup.bottom() - 2)].symbol(), "╰");
        assert!(row(&buffer, popup.bottom() - 1).contains("Indexed prefix search"));
        app.resource_finder.warnings = vec!["AccessDenied: partial results".into()];
        let buffer = draw(&app, 180, 30);
        assert_eq!(buffer[(popup.x, popup.bottom() - 2)].symbol(), "╰");
        assert!(row(&buffer, popup.bottom() - 1).contains("AccessDenied"));
        assert_eq!(buffer[(popup.x, popup.bottom() - 1)].fg, Color::Yellow);
    }

    #[test]
    fn finder_placeholder_clips_safely_in_small_viewports() {
        let mut app = app();
        for focus in [Focus::Filter, Focus::Table] {
            app.resource_finder.focus = focus;
            for (width, height) in [(1, 1), (2, 3), (12, 8), (40, 12), (80, 20)] {
                draw(&app, width, height);
            }
        }
        assert!(app.resource_finder.input.is_empty());
    }
}
