//! Shared adapter for column schemas whose translated label is an owned String.
use super::Column;
use crate::common::{translate_column, ColumnId};
use ratatui::style::Style;
use std::borrow::Cow;

/// Resolve the label once per column construction and borrow it on subsequent
/// header/expanded-row reads. Dropping the column also drops its label.
/// Services retain ownership of cell formatting and compact-sizing policy.
pub struct OwnedColumn<F> {
    id: ColumnId,
    default_name: &'static str,
    label: String,
    width: u16,
    fit_content: bool,
    render: F,
}

impl<F> OwnedColumn<F> {
    pub fn new(
        id: ColumnId,
        default_name: &'static str,
        width: u16,
        fit_content: bool,
        render: F,
    ) -> Self {
        let label = translate_column(id, default_name);
        let width = width.max(label.chars().count().min(u16::MAX as usize) as u16);
        Self {
            id,
            default_name,
            label,
            width,
            fit_content,
            render,
        }
    }
}

impl<T, F: Fn(&T) -> (String, Style)> Column<T> for OwnedColumn<F> {
    fn id(&self) -> &'static str {
        self.id
    }
    fn default_name(&self) -> &'static str {
        self.default_name
    }
    fn name(&self) -> Cow<'_, str> {
        self.label.as_str().into()
    }
    fn width(&self) -> u16 {
        self.width
    }
    fn fit_content(&self) -> bool {
        self.fit_content
    }
    fn render(&self, item: &T) -> (String, Style) {
        (self.render)(item)
    }
}
