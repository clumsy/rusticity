use crate::common::PageSize;

/// Generic table state for list-based services
#[derive(Debug, Clone)]
pub struct TableState<T> {
    pub items: Vec<T>,
    pub selected: usize,
    pub loading: bool,
    pub filter: String,
    pub page_size: PageSize,
    pub expanded_item: Option<usize>,
    pub scroll_offset: usize,
}

impl<T> Default for TableState<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TableState<T> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            selected: 0,
            loading: false,
            filter: String::new(),
            page_size: PageSize::Fifty,
            expanded_item: None,
            scroll_offset: 0,
        }
    }

    pub fn filtered<F>(&self, predicate: F) -> Vec<&T>
    where
        F: Fn(&T) -> bool,
    {
        self.items.iter().filter(|item| predicate(item)).collect()
    }

    pub fn paginate<'a>(&self, filtered: &'a [&'a T]) -> &'a [&'a T] {
        let page_size = self.page_size.value();
        let end_idx = (self.scroll_offset + page_size).min(filtered.len());
        &filtered[self.scroll_offset..end_idx]
    }

    pub fn current_page(&self, _total_items: usize) -> usize {
        self.scroll_offset / self.page_size.value()
    }

    pub fn total_pages(&self, total_items: usize) -> usize {
        total_items.div_ceil(self.page_size.value())
    }

    pub fn next_item(&mut self, max: usize) {
        if max > 0 {
            let new_selected = (self.selected + 1).min(max - 1);
            if new_selected != self.selected {
                self.selected = new_selected;

                // Adjust scroll_offset if selection goes below viewport
                let page_size = self.page_size.value();
                if self.selected >= self.scroll_offset + page_size {
                    self.scroll_offset = self.selected - page_size + 1;
                }
            }
        }
    }

    pub fn prev_item(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;

            // Adjust scroll_offset if selection goes above viewport
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
        }
    }

    pub fn page_down(&mut self, max: usize) {
        if max > 0 {
            let page_size = self.page_size.value();
            self.selected = (self.selected + 10).min(max - 1);

            // Snap scroll_offset to page boundary
            let current_page = self.selected / page_size;
            self.scroll_offset = current_page * page_size;
        }
    }

    pub fn page_up(&mut self) {
        let page_size = self.page_size.value();
        self.selected = self.selected.saturating_sub(10);

        // Snap scroll_offset to page boundary
        let current_page = self.selected / page_size;
        self.scroll_offset = current_page * page_size;
    }

    pub fn snap_to_page(&mut self) {
        let page_size = self.page_size.value();
        let current_page = self.selected / page_size;
        self.scroll_offset = current_page * page_size;
    }

    pub fn toggle_expand(&mut self) {
        self.expanded_item = if self.expanded_item == Some(self.selected) {
            None
        } else {
            Some(self.selected)
        };
    }

    pub fn collapse(&mut self) {
        self.expanded_item = None;
    }

    pub fn expand(&mut self) {
        self.expanded_item = Some(self.selected);
    }

    pub fn is_expanded(&self) -> bool {
        self.expanded_item == Some(self.selected)
    }

    pub fn has_expanded_item(&self) -> bool {
        self.expanded_item.is_some()
    }

    pub fn goto_page(&mut self, page: usize, total_items: usize) {
        let page_size = self.page_size.value();
        let target = (page - 1) * page_size;
        let max = total_items.saturating_sub(1);
        self.selected = target.min(max);
        self.scroll_offset = target.min(total_items.saturating_sub(page_size));
    }

    pub fn reset(&mut self) {
        self.selected = 0;
        self.scroll_offset = 0;
        self.expanded_item = None;
    }

    pub fn get_selected<'a>(&self, filtered: &'a [&'a T]) -> Option<&'a T> {
        filtered.get(self.selected).copied()
    }

    /// Push a character to the filter and reset selection
    pub fn filter_push(&mut self, c: char) {
        self.filter.push(c);
        self.reset();
    }

    /// Pop a character from the filter and reset selection
    pub fn filter_pop(&mut self) {
        self.filter.pop();
        self.reset();
    }

    /// Clear the filter and reset selection
    pub fn filter_clear(&mut self) {
        self.filter.clear();
        self.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_state_default() {
        let state: TableState<String> = TableState::new();
        assert_eq!(state.selected, 0);
        assert!(!state.loading);
        assert_eq!(state.filter, "");
        assert_eq!(state.page_size, PageSize::Fifty);
        assert_eq!(state.expanded_item, None);
    }

    #[test]
    fn test_filtered() {
        let mut state = TableState::new();
        state.items = vec![
            "apple".to_string(),
            "banana".to_string(),
            "apricot".to_string(),
        ];

        let filtered = state.filtered(|item| item.starts_with('a'));
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_paginate() {
        let state = TableState::<String> {
            page_size: PageSize::Ten,
            selected: 0,
            ..TableState::new()
        };

        let items: Vec<String> = (0..25).map(|i| i.to_string()).collect();
        let refs: Vec<&String> = items.iter().collect();

        let page = state.paginate(&refs);
        assert_eq!(page.len(), 10);
    }

    #[test]
    fn test_navigation() {
        let mut state = TableState::<String>::new();

        state.next_item(10);
        assert_eq!(state.selected, 1);

        state.prev_item();
        assert_eq!(state.selected, 0);

        state.page_down(100);
        assert_eq!(state.selected, 10);

        state.page_up();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn test_expand_toggle() {
        let mut state = TableState::<String>::new();

        assert!(!state.is_expanded());

        state.toggle_expand();
        assert!(state.is_expanded());

        state.toggle_expand();
        assert!(!state.is_expanded());

        state.toggle_expand();
        state.collapse();
        assert!(!state.is_expanded());
    }
}
