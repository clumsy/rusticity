use super::store::Bookmark;
use crate::{app::App, common::SortDirection, ui};
use ratatui::{prelude::*, widgets::Clear};

#[derive(Clone, Copy)]
enum Column {
    Resource,
    Kind,
    Profile,
    Region,
    Account,
}

impl ui::table::Column<Bookmark> for Column {
    fn name(&self) -> std::borrow::Cow<'_, str> {
        match self {
            Self::Resource => "Resource",
            Self::Kind => "Type",
            Self::Profile => "Profile",
            Self::Region => "Region",
            Self::Account => "Account",
        }
        .into()
    }
    fn width(&self) -> u16 {
        match self {
            Self::Resource => 40,
            Self::Kind => 18,
            Self::Profile => 22,
            Self::Region => 18,
            Self::Account => 16,
        }
    }
    fn render(&self, item: &Bookmark) -> (String, Style) {
        let value = match self {
            Self::Resource => {
                if item.resource.name.is_empty() {
                    &item.resource.id
                } else {
                    &item.resource.name
                }
            }
            Self::Kind => {
                return (
                    format!(
                        "{}: {}",
                        item.resource.kind.service(),
                        item.resource.kind.label()
                    ),
                    Style::default(),
                )
            }
            Self::Profile => &item.profile,
            Self::Region => &item.region,
            Self::Account => &item.account_id,
        };
        (value.to_string(), Style::default())
    }
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let popup = ui::centered_rect(90, 70, area);
    frame.render_widget(Clear, popup);
    let parts = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(popup);
    let state = &app.bookmarks;
    let mut spans = vec![Span::raw(&state.filter)];
    if state.filtering {
        spans.push(Span::styled("█", Style::default().fg(Color::Green)));
    }
    frame.render_widget(ui::filter_area(spans, state.filtering), parts[0]);
    let rows = state.filtered();
    let title = if state.opening {
        "Bookmarks — opening with saved profile / Region…".to_string()
    } else {
        format!(
            "Bookmarks ({}) — Enter opens with saved profile / Region",
            rows.len()
        )
    };
    let columns: Vec<Box<dyn ui::table::Column<Bookmark>>> = [
        Column::Resource,
        Column::Kind,
        Column::Profile,
        Column::Region,
        Column::Account,
    ]
    .into_iter()
    .map(|c| Box::new(c) as Box<dyn ui::table::Column<Bookmark>>)
    .collect();
    let empty = rows.is_empty();
    ui::table::render_table(
        frame,
        ui::table::TableConfig {
            items: rows,
            selected_index: state.selected,
            expanded_index: state.expanded,
            columns: &columns,
            sort_column: "",
            sort_direction: SortDirection::Asc,
            title,
            area: parts[1],
            get_expanded_content: Some(Box::new(|entry| {
                vec![
                    (format!("ID: {}", entry.resource.id), Style::default()),
                    (
                        format!(
                            "ARN: {}",
                            if entry.resource.arn.is_empty() {
                                "-"
                            } else {
                                &entry.resource.arn
                            }
                        ),
                        Style::default(),
                    ),
                ]
            })),
            is_active: !state.filtering,
        },
    );
    if empty && parts[1].width > 2 && parts[1].height > 3 {
        let message = if state.rows.is_empty() {
            "No bookmarks yet. Select a resource or finder result and press Ctrl+B."
        } else {
            "No matching bookmarks. Clear or change the filter."
        };
        frame.render_widget(
            ratatui::widgets::Paragraph::new(message).wrap(ratatui::widgets::Wrap { trim: false }),
            Rect {
                x: parts[1].x + 1,
                y: parts[1].y + 2,
                width: parts[1].width - 2,
                height: parts[1].height - 3,
            },
        );
    }
}
