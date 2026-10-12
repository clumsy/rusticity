use crate::common::PageSize;

/// Which in-memory window an existing table renders. This does not fetch data
/// or interpret a service's continuation token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableWindow {
    /// A page aligned to the current selection (most service tables).
    SelectedPage,
    /// A sliding window starting at scroll_offset (e.g. CloudWatch log groups).
    ScrollOffset,
}

/// Render-ready rows and metadata from one filtered, ordered result set.
///
/// The vector owns references, not resources, and reuses the caller's allocation.
/// Indices refer to this visible window; counts refer to the full result set.
#[derive(Debug)]
pub struct PreparedTableView<'a, T> {
    pub items: Vec<&'a T>,
    pub total_items: usize,
    pub current_page: usize,
    pub total_pages: usize,
    /// Preserves the renderer's saturating selection-offset convention. A stale
    /// selection may be outside items; preparation never changes navigation state.
    pub selected_index: usize,
    pub expanded_index: Option<usize>,
}

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
    pub next_token: Option<String>,
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
            next_token: None,
        }
    }

    pub fn filtered<F>(&self, predicate: F) -> Vec<&T>
    where
        F: Fn(&T) -> bool,
    {
        self.items.iter().filter(|item| predicate(item)).collect()
    }

    /// Prepare an already filtered and sorted result set for rendering.
    ///
    /// Keep service-specific filtering/sorting in the caller so rendering and
    /// actions continue to use the same ordering. Only in-memory windowing lives
    /// here: no loading, cursor advancement, selection reset, or resource clones.
    /// Stale positions produce an empty window instead of panicking.
    pub fn prepare_view<'a>(
        &self,
        mut items: Vec<&'a T>,
        window: TableWindow,
    ) -> PreparedTableView<'a, T> {
        let total_items = items.len();
        let page_size = self.page_size.value();
        let start = match window {
            TableWindow::SelectedPage => self.selected / page_size * page_size,
            TableWindow::ScrollOffset => self.scroll_offset,
        };
        let range = self.window_range(start, total_items);
        let expanded_index = self
            .expanded_item
            .filter(|idx| range.contains(idx))
            .map(|idx| idx - start);

        // Truncate before draining so only visible references need to be shifted.
        // No second page-sized Vec allocation (unlike slice.to_vec()).
        items.truncate(range.end);
        items.drain(..range.start);

        PreparedTableView {
            items,
            total_items,
            current_page: start / page_size,
            total_pages: self.total_pages(total_items),
            selected_index: self.selected.saturating_sub(start),
            expanded_index,
        }
    }

    fn window_range(&self, start: usize, total_items: usize) -> std::ops::Range<usize> {
        start.min(total_items)
            ..start
                .saturating_add(self.page_size.value())
                .min(total_items)
    }

    pub fn paginate<'a>(&self, filtered: &'a [&'a T]) -> &'a [&'a T] {
        &filtered[self.window_range(self.scroll_offset, filtered.len())]
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

                // Adjust scroll_offset if selection goes below viewport.
                // Uses effective_page_size = page_size - extra rows from expanded content.
                let page_size = self.page_size.value();
                if self.selected >= self.scroll_offset + page_size {
                    self.scroll_offset = self.selected - page_size + 1;
                }
            }
        }
    }

    /// Like `next_item` but accounts for `extra_rows_before_selected` occupied by
    /// expanded content that appears BEFORE the selected item in the viewport.
    /// Ensures scroll_offset advances so the selected item stays visible.
    pub fn next_item_with_expansion(&mut self, max: usize, extra_rows_before_selected: usize) {
        if max > 0 {
            let new_selected = (self.selected + 1).min(max - 1);
            if new_selected != self.selected {
                self.selected = new_selected;

                let page_size = self.page_size.value();

                // Standard scroll: ensure selected is within [scroll_offset, scroll_offset+page_size)
                if self.selected >= self.scroll_offset + page_size {
                    self.scroll_offset = self.selected - page_size + 1;
                }

                // Extra scroll: if expanded content above pushes selected below viewport
                let selected_visual_row =
                    self.selected.saturating_sub(self.scroll_offset) + extra_rows_before_selected;
                if selected_visual_row >= page_size {
                    let overflow = selected_visual_row.saturating_sub(page_size - 1);
                    self.scroll_offset += overflow;
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

    /// After expanding an item, scroll up if the detail rows would go below the viewport.
    /// `detail_rows` = number of detail lines shown for the expanded item.
    /// `viewport_height` = visible rows in the table (area.height - 3 for borders+header).
    pub fn ensure_expansion_visible(&mut self, detail_rows: usize, viewport_height: usize) {
        if self.expanded_item != Some(self.selected) || detail_rows == 0 || viewport_height == 0 {
            return;
        }
        let page_size = self.page_size.value();
        // Visual row of selected item (0-indexed within current page)
        let selected_visual = self.selected.saturating_sub(self.scroll_offset);
        // After expansion, detail rows appear at selected_visual+1 .. selected_visual+detail_rows+1
        let last_detail_row = selected_visual + detail_rows;
        // Use min of page_size and viewport_height as effective display height
        let effective_height = page_size.min(viewport_height);
        if last_detail_row >= effective_height {
            // Need to scroll up so last detail row is visible
            let overflow = last_detail_row.saturating_sub(effective_height - 1);
            self.scroll_offset = self.scroll_offset.saturating_add(overflow);
        }
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

    /// Change the page size and return to the first page. Keeps the filter,
    /// so the visible list starts from the top rather than jumping to a sparse
    /// last page when the selection was near the end.
    pub fn set_page_size(&mut self, page_size: PageSize) {
        self.page_size = page_size;
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

    #[test]
    fn test_next_item_with_expansion_scrolls_when_selection_goes_below_viewport() {
        // Regression: when an item is expanded with N detail rows, navigating past
        // the last visible item should scroll — not leave cursor below viewport.
        //
        // Setup: page_size=10, 20 items, expansion adds 4 detail rows before selected.
        // selected=5 at visual row 5+4=9 (last visible). Navigate to 6: visual=6+4=10 >= 10 → scroll.

        let mut state = TableState::<i32>::new();
        state.page_size = crate::common::PageSize::Ten; // page_size = 10
        state.items = (0..20).collect();
        state.scroll_offset = 0;
        state.selected = 5;

        // Navigate: extra_rows=4 before new selected=6 (expanded item is before 6)
        state.next_item_with_expansion(20, 4);

        assert_eq!(state.selected, 6, "selected must advance to 6");
        assert!(
            state.scroll_offset > 0,
            "scroll_offset must advance when expansion pushes selected off viewport; got {}",
            state.scroll_offset
        );
        let in_viewport =
            state.selected >= state.scroll_offset && state.selected < state.scroll_offset + 10;
        assert!(
            in_viewport,
            "selected={} must be within [{}, {})",
            state.selected,
            state.scroll_offset,
            state.scroll_offset + 10
        );
    }
}

#[cfg(test)]
mod prepared_view_tests {
    use super::*;

    #[test]
    fn preparation_reuses_allocation_and_keeps_filtered_sorted_order() {
        // Deliberately not Clone: preparing a view must only move references.
        #[derive(Debug)]
        struct Item(usize);
        let state = TableState {
            items: (0..30).map(Item).collect(),
            selected: 12,
            expanded_item: Some(11),
            page_size: PageSize::Ten,
            ..TableState::new()
        };
        let mut filtered = state.filtered(|item| item.0 % 2 == 0);
        filtered.sort_by_key(|item| std::cmp::Reverse(item.0));
        let allocation = filtered.as_ptr();
        let capacity = filtered.capacity();
        let view = state.prepare_view(filtered, TableWindow::SelectedPage);
        assert_eq!(view.items.as_ptr(), allocation);
        assert_eq!(view.items.capacity(), capacity);
        assert_eq!(
            view.items.iter().map(|item| item.0).collect::<Vec<_>>(),
            [8, 6, 4, 2, 0]
        );
        assert_eq!(
            (view.total_items, view.current_page, view.total_pages),
            (15, 1, 2)
        );
        assert_eq!(view.selected_index, 2);
        assert_eq!(view.expanded_index, Some(1));
    }

    #[test]
    fn window_modes_preserve_page_vs_sliding_scroll_behavior() {
        let state = TableState {
            items: (0..25).collect::<Vec<_>>(),
            selected: 14,
            scroll_offset: 7,
            expanded_item: Some(9),
            page_size: PageSize::Ten,
            ..TableState::new()
        };
        let page = state.prepare_view(state.items.iter().collect(), TableWindow::SelectedPage);
        let scroll = state.prepare_view(state.items.iter().collect(), TableWindow::ScrollOffset);
        assert_eq!(
            page.items,
            (10..20).collect::<Vec<_>>().iter().collect::<Vec<_>>()
        );
        assert_eq!(
            scroll.items,
            (7..17).collect::<Vec<_>>().iter().collect::<Vec<_>>()
        );
        assert_eq!(
            (page.current_page, page.selected_index, page.expanded_index),
            (1, 4, None)
        );
        assert_eq!(
            (
                scroll.current_page,
                scroll.selected_index,
                scroll.expanded_index
            ),
            (0, 7, Some(2))
        );
        assert_eq!(
            state.paginate(&state.items.iter().collect::<Vec<_>>()),
            scroll.items
        );
    }

    #[test]
    fn expansion_is_relative_and_only_present_inside_the_window() {
        let mut state = TableState {
            items: (0..25).collect::<Vec<_>>(),
            selected: 12,
            page_size: PageSize::Ten,
            ..TableState::new()
        };
        for (expanded, expected) in [
            (None, None),
            (Some(9), None),
            (Some(10), Some(0)),
            (Some(19), Some(9)),
            (Some(20), None),
            (Some(usize::MAX), None),
        ] {
            state.expanded_item = expanded;
            let view = state.prepare_view(state.items.iter().collect(), TableWindow::SelectedPage);
            assert_eq!(view.expanded_index, expected);
        }
    }

    #[test]
    fn empty_and_stale_positions_are_safe_and_do_not_mutate_state() {
        for window in [TableWindow::SelectedPage, TableWindow::ScrollOffset] {
            for len in [0, 3, 10] {
                let state = TableState {
                    items: (0..len).collect::<Vec<_>>(),
                    selected: usize::MAX,
                    scroll_offset: usize::MAX,
                    expanded_item: Some(usize::MAX),
                    filter: "unchanged".into(),
                    next_token: Some("server-cursor".into()),
                    page_size: PageSize::Ten,
                    ..TableState::new()
                };
                let view = state.prepare_view(state.items.iter().collect(), window);
                assert!(view.items.is_empty());
                assert_eq!(view.expanded_index, None);
                assert_eq!(view.total_items, len);
                assert_eq!(view.total_pages, len.div_ceil(10));
                assert!(state
                    .paginate(&state.items.iter().collect::<Vec<_>>())
                    .is_empty());
                assert_eq!(state.selected, usize::MAX);
                assert_eq!(state.scroll_offset, usize::MAX);
                assert_eq!(state.expanded_item, Some(usize::MAX));
                assert_eq!(state.filter, "unchanged");
                assert_eq!(state.next_token.as_deref(), Some("server-cursor"));
            }
        }
    }

    #[test]
    fn prepared_views_match_existing_arithmetic_at_all_page_sizes() {
        for page_size in [
            PageSize::Ten,
            PageSize::TwentyFive,
            PageSize::Fifty,
            PageSize::OneHundred,
        ] {
            let size = page_size.value();
            for len in [0, 1, size - 1, size, size + 1, 2 * size + 3] {
                let mut state = TableState {
                    items: (0..len).collect::<Vec<_>>(),
                    page_size,
                    ..TableState::new()
                };
                for selected in 0..len.max(1) {
                    state.selected = selected;
                    state.scroll_offset = selected.saturating_sub(3);
                    for window in [TableWindow::SelectedPage, TableWindow::ScrollOffset] {
                        let start = match window {
                            TableWindow::SelectedPage => selected / size * size,
                            TableWindow::ScrollOffset => state.scroll_offset,
                        };
                        let end = (start + size).min(len);
                        let view = state.prepare_view(state.items.iter().collect(), window);
                        assert_eq!(
                            view.items,
                            state.items[start..end].iter().collect::<Vec<_>>()
                        );
                        assert_eq!(view.selected_index, selected.saturating_sub(start));
                        assert_eq!(view.current_page, start / size);
                        assert_eq!(view.total_pages, len.div_ceil(size));
                        assert!(view.items.len() <= size);
                    }
                }
            }
        }
    }

    #[test]
    fn scrolling_selection_before_window_keeps_saturating_convention() {
        let state = TableState {
            items: (0..20).collect::<Vec<_>>(),
            selected: 3,
            scroll_offset: 7,
            page_size: PageSize::Ten,
            ..TableState::new()
        };
        let view = state.prepare_view(state.items.iter().collect(), TableWindow::ScrollOffset);
        assert_eq!(view.items[0], &7);
        assert_eq!(view.selected_index, 0);
    }
}
