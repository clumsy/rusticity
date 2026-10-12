//! Ratatui presentation of the shared Preferences rows; popup layout stays in ui/mod.rs.
use crate::preferences::{PreferenceRow, Preferences};
use ratatui::{prelude::*, widgets::ListItem};

pub(super) fn render_rows(prefs: &Preferences) -> (Vec<ListItem<'static>>, &'static str, usize) {
    let items: Vec<_> = prefs
        .rows()
        .iter()
        .map(|row| match row {
            PreferenceRow::Section(label) => super::render_section_header(label).0,
            PreferenceRow::Separator => ListItem::new(""),
            PreferenceRow::Column(column) if column.locked || !prefs.is_enabled() => {
                let toggle = if column.visible { "◼⬜" } else { "⬜◼" };
                ListItem::new(Span::styled(
                    format!("{} {}", toggle, column.label),
                    Style::default().fg(Color::DarkGray),
                ))
            }
            PreferenceRow::Column(column) => {
                super::render_column_toggle_string(&column.label, column.visible).0
            }
            PreferenceRow::PageSize {
                label, selected, ..
            } if prefs.is_enabled() => super::render_radio_item(label, *selected, false).0,
            PreferenceRow::PageSize {
                label, selected, ..
            } => {
                let (radio, _) = super::render_radio(*selected);
                ListItem::new(Span::styled(
                    format!("{} {}", radio, label),
                    Style::default().fg(Color::DarkGray),
                ))
            }
        })
        .collect();
    let width = items.iter().map(ListItem::width).max().unwrap_or(0);
    (items, " Preferences ", width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::PageSize;
    use crate::preferences::PreferenceColumn;
    use ratatui::widgets::{List, Widget};

    #[test]
    fn rendered_rows_preserve_model_order_labels_and_locked_style() {
        let prefs = Preferences::table(
            vec![
                PreferenceColumn {
                    id: "fixed".into(),
                    label: "Alias".into(),
                    visible: true,
                    locked: true,
                },
                PreferenceColumn {
                    id: "dynamic".into(),
                    label: "属性列".into(),
                    visible: false,
                    locked: false,
                },
            ],
            PageSize::TwentyFive,
        );
        let (items, title, width) = render_rows(&prefs);
        assert_eq!(items.len(), prefs.rows().len());
        assert_eq!(title, " Preferences ");
        assert_eq!(width, items.iter().map(ListItem::width).max().unwrap());
        let area = Rect::new(0, 0, 40, items.len() as u16);
        let mut buffer = Buffer::empty(area);
        Widget::render(List::new(items), area, &mut buffer);
        for (index, row) in prefs.rows().iter().enumerate() {
            let text = (0..area.width)
                .map(|x| buffer[(x, index as u16)].symbol())
                .collect::<String>();
            match row {
                PreferenceRow::Section(label) => assert!(text.contains(label)),
                PreferenceRow::Column(column) => {
                    // Wide glyphs have blank continuation cells in the buffer.
                    assert!(text
                        .replace(' ', "")
                        .contains(&column.label.replace(' ', "")));
                }
                PreferenceRow::Separator => assert!(text.trim().is_empty()),
                PreferenceRow::PageSize { label, .. } => assert!(text.contains(label)),
            }
        }
        assert_eq!(buffer[(0, 1)].fg, Color::DarkGray);
    }
}
