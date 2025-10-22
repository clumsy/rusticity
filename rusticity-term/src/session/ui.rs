use super::Session;
use crate::common::SortDirection;

pub fn render_session_picker(
    frame: &mut ratatui::Frame,
    app: &crate::app::App,
    area: ratatui::prelude::Rect,
    centered_rect: fn(u16, u16, ratatui::prelude::Rect) -> ratatui::prelude::Rect,
) {
    use crate::ui::table::{render_table, Column as TableColumn, TableConfig};
    use ratatui::{prelude::*, widgets::*};

    let popup_area = centered_rect(80, 70, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(popup_area);

    let cursor = "█";
    let filter = Paragraph::new(Line::from(vec![
        Span::raw(&app.session_filter),
        Span::styled(cursor, Style::default().fg(Color::Green)),
    ]))
    .block(
        Block::default()
            .title(" 🔍 ")
            .borders(Borders::ALL)
            .border_style(crate::ui::active_border()),
    )
    .style(Style::default());

    frame.render_widget(Clear, popup_area);
    frame.render_widget(filter, chunks[0]);

    struct SessionTimestampColumn;
    impl TableColumn<Session> for SessionTimestampColumn {
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
    impl TableColumn<Session> for SessionProfileColumn {
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
    impl TableColumn<Session> for SessionRegionColumn {
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
    impl TableColumn<Session> for SessionAccountColumn {
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
    impl TableColumn<Session> for SessionTabsColumn {
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

    let columns: Vec<Box<dyn TableColumn<Session>>> = vec![
        Box::new(SessionTimestampColumn),
        Box::new(SessionProfileColumn),
        Box::new(SessionRegionColumn),
        Box::new(SessionAccountColumn),
        Box::new(SessionTabsColumn),
    ];

    let filtered = app.get_filtered_sessions();
    let config = TableConfig {
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

    render_table(frame, config);
}
