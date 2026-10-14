//! Preferences rows are the single source of truth for rendering and input.
//! This model has no dependency on App or Ratatui; service bindings live separately.
use crate::common::PageSize;
use std::borrow::Cow;

pub(crate) mod bindings;
mod panels;

pub const PAGE_SIZE_OPTIONS: &[(PageSize, &str)] = &[
    (PageSize::Ten, "10"),
    (PageSize::TwentyFive, "25"),
    (PageSize::Fifty, "50"),
    (PageSize::OneHundred, "100"),
];

#[derive(Debug, Clone, PartialEq)]
pub struct PreferenceColumn {
    pub id: Cow<'static, str>,
    pub label: String,
    pub visible: bool,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PreferenceRow {
    Section(&'static str),
    Column(PreferenceColumn),
    Separator,
    Radio {
        group: &'static str,
        value: &'static str,
        label: &'static str,
        selected: bool,
    },
    Toggle {
        id: &'static str,
        label: &'static str,
        enabled: bool,
    },
    PageSize {
        value: PageSize,
        label: &'static str,
        selected: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PreferenceAction {
    SetColumnVisibility {
        id: Cow<'static, str>,
        visible: bool,
    },
    SetPageSize(PageSize),
    SetRadio {
        group: &'static str,
        value: &'static str,
    },
    SetToggle {
        id: &'static str,
        enabled: bool,
    },
}

#[derive(Debug)]
pub struct Preferences {
    rows: Vec<PreferenceRow>,
    enabled: bool,
}

impl Preferences {
    pub fn table(columns: Vec<PreferenceColumn>, page_size: PageSize) -> Self {
        Self::table_with_page_sizes(columns, page_size, PAGE_SIZE_OPTIONS)
    }

    pub fn table_with_page_sizes(
        columns: Vec<PreferenceColumn>,
        page_size: PageSize,
        options: &[(PageSize, &'static str)],
    ) -> Self {
        Self::columns_only(columns, true).with_page_sizes(Some(page_size), options)
    }

    pub fn columns_only(columns: Vec<PreferenceColumn>, header: bool) -> Self {
        let mut rows = Vec::new();
        if header {
            rows.push(PreferenceRow::Section("Columns"));
        }
        rows.extend(columns.into_iter().map(PreferenceRow::Column));
        Self {
            rows,
            enabled: true,
        }
    }

    fn section(&mut self, title: &'static str) {
        if !self.rows.is_empty() {
            self.rows.push(PreferenceRow::Separator);
        }
        self.rows.push(PreferenceRow::Section(title));
    }

    pub fn with_page_sizes(
        mut self,
        current: Option<PageSize>,
        options: &[(PageSize, &'static str)],
    ) -> Self {
        self.section("Page size");
        self.rows.extend(
            options
                .iter()
                .map(|&(value, label)| PreferenceRow::PageSize {
                    value,
                    label,
                    selected: current == Some(value),
                }),
        );
        self
    }

    pub fn radio_group(
        mut self,
        group: &'static str,
        title: &'static str,
        choices: &[(&'static str, &'static str)],
        selected: &str,
    ) -> Self {
        self.section(title);
        self.rows
            .extend(choices.iter().map(|&(value, label)| PreferenceRow::Radio {
                group,
                value,
                label,
                selected: selected == value,
            }));
        self
    }

    pub fn toggle(
        mut self,
        id: &'static str,
        title: &'static str,
        label: &'static str,
        enabled: bool,
    ) -> Self {
        self.section(title);
        self.rows.push(PreferenceRow::Toggle { id, label, enabled });
        self
    }

    /// Preserve a non-table tab's fallback display without presenting working controls.
    pub fn read_only(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    pub fn rows(&self) -> &[PreferenceRow] {
        &self.rows
    }
    pub fn max_index(&self) -> usize {
        self.rows.len().saturating_sub(1)
    }
    pub fn columns(&self) -> impl Iterator<Item = &PreferenceColumn> {
        self.rows.iter().filter_map(|row| match row {
            PreferenceRow::Column(column) => Some(column),
            _ => None,
        })
    }
    pub fn column_count(&self) -> usize {
        self.columns().count()
    }
    pub fn is_separator(&self, index: usize) -> bool {
        matches!(self.rows.get(index), Some(PreferenceRow::Separator))
    }

    /// Headers, separators, locked columns, and stale indices have no action.
    /// Count only columns in this model, not stale IDs from a previous result set.
    pub fn action(&self, index: usize) -> Option<PreferenceAction> {
        if !self.enabled {
            return None;
        }
        match self.rows.get(index)? {
            PreferenceRow::Column(column) if !column.locked => {
                if column.visible && self.columns().filter(|c| c.visible).count() <= 1 {
                    return None;
                }
                Some(PreferenceAction::SetColumnVisibility {
                    id: column.id.clone(),
                    visible: !column.visible,
                })
            }
            PreferenceRow::PageSize { value, .. } => Some(PreferenceAction::SetPageSize(*value)),
            PreferenceRow::Radio { group, value, .. } => {
                Some(PreferenceAction::SetRadio { group, value })
            }
            PreferenceRow::Toggle { id, enabled, .. } => Some(PreferenceAction::SetToggle {
                id,
                enabled: !enabled,
            }),
            _ => None,
        }
    }

    /// Up/down saturates at the ends and skips separators, including stale cursors.
    pub fn move_row(&self, index: usize, forward: bool) -> usize {
        let index = index.min(self.max_index());
        if forward {
            (index.saturating_add(1)..self.rows.len())
                .find(|&i| !self.is_separator(i))
                .unwrap_or(index)
        } else {
            (0..index)
                .rev()
                .find(|&i| !self.is_separator(i))
                .unwrap_or(index)
        }
    }

    /// Tab/BackTab cycles section headers, not individual options.
    pub fn move_section(&self, index: usize, forward: bool) -> usize {
        let sections: Vec<_> = self
            .rows
            .iter()
            .enumerate()
            .filter_map(|(i, row)| matches!(row, PreferenceRow::Section(_)).then_some(i))
            .collect();
        if sections.is_empty() {
            return 0;
        }
        let current = sections.iter().rposition(|&i| i <= index).unwrap_or(0);
        let next = if forward {
            (current + 1) % sections.len()
        } else {
            (current + sections.len() - 1) % sections.len()
        };
        sections[next]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(n: usize) -> Preferences {
        Preferences::table(
            (0..n)
                .map(|i| PreferenceColumn {
                    id: format!("attribute-{i}").into(),
                    label: format!("Attribute {i}"),
                    visible: true,
                    locked: false,
                })
                .collect(),
            PageSize::Fifty,
        )
    }

    #[test]
    fn every_rendered_row_has_matching_bounds_and_action() {
        for n in [0, 1, 2, 18, 100] {
            let prefs = model(n);
            assert_eq!(prefs.column_count(), n);
            assert_eq!(prefs.max_index(), prefs.rows().len() - 1);
            assert_eq!(prefs.action(0), None);
            assert!(prefs.is_separator(n + 1));
            assert_eq!(prefs.action(n + 1), None);
            assert_eq!(prefs.action(n + 2), None);
            for (offset, &(size, _)) in PAGE_SIZE_OPTIONS.iter().enumerate() {
                assert_eq!(
                    prefs.action(n + 3 + offset),
                    Some(PreferenceAction::SetPageSize(size))
                );
            }
            assert_eq!(prefs.action(prefs.rows().len()), None);
            assert_eq!(prefs.action(usize::MAX), None);
        }
    }

    #[test]
    fn navigation_uses_rows_and_sections() {
        let prefs = model(2);
        assert_eq!(prefs.move_row(2, true), 4);
        assert_eq!(prefs.move_row(4, false), 2);
        assert_eq!(prefs.move_row(0, false), 0);
        assert_eq!(prefs.move_row(usize::MAX, true), prefs.max_index());
        for index in 0..=prefs.max_index() {
            let expected = if index < 4 { 4 } else { 0 };
            assert_eq!(prefs.move_section(index, true), expected);
            assert_eq!(prefs.move_section(index, false), expected);
        }
    }

    #[test]
    fn locked_and_last_visible_columns_cannot_be_hidden() {
        let mut prefs = model(2);
        if let PreferenceRow::Column(column) = &mut prefs.rows[1] {
            column.locked = true;
        }
        assert_eq!(prefs.action(1), None);
        assert!(matches!(
            prefs.action(2),
            Some(PreferenceAction::SetColumnVisibility { visible: false, .. })
        ));
        assert_eq!(model(1).action(1), None);
        if let PreferenceRow::Column(column) = &mut prefs.rows[2] {
            column.visible = false;
        }
        assert!(matches!(
            prefs.action(2),
            Some(PreferenceAction::SetColumnVisibility { visible: true, .. })
        ));
        let prefs = prefs.read_only();
        assert!((0..prefs.rows.len()).all(|i| prefs.action(i).is_none()));
    }
}
