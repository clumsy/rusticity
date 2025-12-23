use super::Session;
use crate::common::SortDirection;
use crate::ui::filter_area;
use crate::ui::format_title;
use crate::ui::table::{self};
use ratatui::prelude::{Color, Constraint, Direction, Layout, Span, Style};
use ratatui::widgets::Clear;

enum SessionColumn {
    Timestamp,
    Profile,
    Region,
    Account,
    Tabs,
}

impl table::Column<Session> for SessionColumn {
    fn name(&self) -> &str {
        match self {
            Self::Timestamp => "Timestamp",
            Self::Profile => "Profile",
            Self::Region => "Region",
            Self::Account => "Account",
            Self::Tabs => "Tabs",
        }
    }

    fn width(&self) -> u16 {
        match self {
            Self::Timestamp => 25,
            Self::Profile => 25,
            Self::Region => 15,
            Self::Account => 15,
            Self::Tabs => 8,
        }
    }

    fn render(&self, item: &Session) -> (String, Style) {
        let text = match self {
            Self::Timestamp => item.timestamp.clone(),
            Self::Profile => item.profile.clone(),
            Self::Region => item.region.clone(),
            Self::Account => item.account_id.clone(),
            Self::Tabs => item.tabs.len().to_string(),
        };
        (text, Style::default())
    }
}

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

    let columns: Vec<Box<dyn table::Column<Session>>> = vec![
        Box::new(SessionColumn::Timestamp),
        Box::new(SessionColumn::Profile),
        Box::new(SessionColumn::Region),
        Box::new(SessionColumn::Account),
        Box::new(SessionColumn::Tabs),
    ];

    let filtered = app.get_filtered_sessions();
    let config = table::TableConfig {
        items: filtered,
        selected_index: app.session_picker_selected,
        expanded_index: None,
        columns: &columns,
        sort_column: "Timestamp",
        sort_direction: SortDirection::Desc,
        title: format_title("Sessions"),
        area: chunks[1],
        get_expanded_content: None,
        is_active: true,
    };

    table::render_table(frame, config);
}
