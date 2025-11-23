use super::Session;
use crate::common::SortDirection;
use crate::ui::filter_area;
use crate::ui::table::{self};
use ratatui::prelude::{Color, Constraint, Direction, Layout, Span, Style};
use ratatui::widgets::Clear;

pub fn render_session_picker(
    frame: &mut ratatui::Frame,
    app: &crate::app::App,
    area: ratatui::prelude::Rect,
    centered_rect: fn(u16, u16, ratatui::prelude::Rect) -> ratatui::prelude::Rect,
) {
    let popup_area = centered_rect(80, 70, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(popup_area);

    let cursor = "█";
    let filter_text = vec![
        Span::raw(&app.session_filter),
        Span::styled(cursor, Style::default().fg(Color::Green)),
    ];
    let filter = filter_area(filter_text, true);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(filter, chunks[0]);

    struct SessionTimestampColumn;
    impl table::Column<Session> for SessionTimestampColumn {
        fn name(&self) -> &str {
            "Timestamp"
        }
        fn width(&self) -> u16 {
            25
        }
        fn render(&self, item: &Session) -> (String, Style) {
            (item.timestamp.clone(), Style::default())
        }
    }

    struct SessionProfileColumn;
    impl table::Column<Session> for SessionProfileColumn {
        fn name(&self) -> &str {
            "Profile"
        }
        fn width(&self) -> u16 {
            25
        }
        fn render(&self, item: &Session) -> (String, Style) {
            (item.profile.clone(), Style::default())
        }
    }

    struct SessionRegionColumn;
    impl table::Column<Session> for SessionRegionColumn {
        fn name(&self) -> &str {
            "Region"
        }
        fn width(&self) -> u16 {
            15
        }
        fn render(&self, item: &Session) -> (String, Style) {
            (item.region.clone(), Style::default())
        }
    }

    struct SessionAccountColumn;
    impl table::Column<Session> for SessionAccountColumn {
        fn name(&self) -> &str {
            "Account"
        }
        fn width(&self) -> u16 {
            15
        }
        fn render(&self, item: &Session) -> (String, Style) {
            (item.account_id.clone(), Style::default())
        }
    }

    struct SessionTabsColumn;
    impl table::Column<Session> for SessionTabsColumn {
        fn name(&self) -> &str {
            "Tabs"
        }
        fn width(&self) -> u16 {
            8
        }
        fn render(&self, item: &Session) -> (String, Style) {
            (item.tabs.len().to_string(), Style::default())
        }
    }

    let columns: Vec<Box<dyn table::Column<Session>>> = vec![
        Box::new(SessionTimestampColumn),
        Box::new(SessionProfileColumn),
        Box::new(SessionRegionColumn),
        Box::new(SessionAccountColumn),
        Box::new(SessionTabsColumn),
    ];

    let filtered = app.get_filtered_sessions();
    let config = table::TableConfig {
        items: filtered,
        selected_index: app.session_picker_selected,
        expanded_index: None,
        columns: &columns,
        sort_column: "Timestamp",
        sort_direction: SortDirection::Desc,
        title: " Sessions ".to_string(),
        area: chunks[1],
        get_expanded_content: None,
        is_active: true,
    };

    table::render_table(frame, config);
}
