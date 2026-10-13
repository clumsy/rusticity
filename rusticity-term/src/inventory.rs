//! Shared input handling for flat, read-only inventories (RDS, VPC).
//! Service adapters provide state and the same filtered count used by rendering.
use crate::app::copy_to_clipboard;
use crate::common::InputFocus;
use crate::keymap::{Action, Mode};
use crate::table::TableState;

const FILTER_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

pub struct InventoryInput<'a, T> {
    pub table: &'a mut TableState<T>,
    pub focus: &'a mut InputFocus,
    pub mode: &'a mut Mode,
    pub page_input: &'a mut String,
    pub error_message: &'a mut Option<String>,
    pub error_scroll: &'a mut usize,
    pub filtered_count: usize,
    pub selected_arn: Option<String>,
}

pub fn go_to_page<T>(table: &mut TableState<T>, page: usize, count: usize) {
    if page == 0 {
        return;
    }
    let pages = table.total_pages(count).max(1);
    table.goto_page(page.min(pages), count);
    table.collapse();
}

pub fn handle_action<T>(input: InventoryInput<'_, T>, action: &Action) -> bool {
    let InventoryInput {
        table,
        focus,
        mode,
        page_input,
        error_message,
        error_scroll,
        filtered_count,
        selected_arn,
    } = input;
    if *mode == Mode::ErrorModal && *action == Action::RetryLoad {
        *error_message = None;
        *error_scroll = 0;
        *mode = Mode::Normal;
        table.loading = true;
        return true;
    }
    if *mode == Mode::Normal {
        match action {
            Action::NextItem => table.next_item(filtered_count),
            Action::PrevItem => table.prev_item(),
            Action::PageDown | Action::ScrollDown => {
                table.page_down(filtered_count);
                table.collapse();
            }
            Action::PageUp | Action::ScrollUp => {
                table.page_up();
                table.collapse();
            }
            Action::Select | Action::ExpandRow | Action::NextPane => {
                if table.selected < filtered_count {
                    table.toggle_expand();
                }
            }
            Action::CollapseRow | Action::PrevPane | Action::GoBack => table.collapse(),
            Action::StartFilter => {
                *mode = Mode::FilterInput;
                *focus = InputFocus::Filter;
            }
            Action::Refresh => table.loading = true,
            Action::Yank => {
                if let Some(arn) = selected_arn {
                    copy_to_clipboard(&arn);
                }
            }
            // Flat inventory views do not implement tabs or additional sort/filter switches.
            Action::NextDetailTab
            | Action::PrevDetailTab
            | Action::CycleSortColumn
            | Action::ToggleSortDirection
            | Action::ToggleExactMatch
            | Action::ToggleShowExpired => {}
            _ => return false,
        }
        return true;
    }
    if *mode == Mode::FilterInput {
        match action {
            Action::NextFilterFocus => *focus = focus.next(&FILTER_CONTROLS),
            Action::PrevFilterFocus => *focus = focus.prev(&FILTER_CONTROLS),
            Action::PageDown | Action::PageUp => {
                if *focus == InputFocus::Pagination {
                    if *action == Action::PageDown {
                        focus.handle_page_down(
                            &mut table.selected,
                            &mut table.scroll_offset,
                            table.page_size.value(),
                            filtered_count,
                        );
                    } else {
                        focus.handle_page_up(
                            &mut table.selected,
                            &mut table.scroll_offset,
                            table.page_size.value(),
                        );
                    }
                    table.collapse();
                }
            }
            Action::NextItem | Action::PrevItem => {}
            Action::FilterInput(c) if *focus == InputFocus::Filter => table.filter_push(*c),
            Action::FilterBackspace if *focus == InputFocus::Filter => table.filter_pop(),
            Action::ToggleFilterCheckbox if *focus == InputFocus::Filter => table.filter_push(' '),
            Action::FilterInput(c) if c.is_ascii_digit() => page_input.push(*c),
            Action::FilterBackspace => {
                page_input.pop();
            }
            Action::FilterInput(_) | Action::ToggleFilterCheckbox => {}
            Action::ApplyFilter => {
                if let Ok(page) = page_input.parse() {
                    go_to_page(table, page, filtered_count);
                }
                page_input.clear();
                *mode = Mode::Normal;
            }
            Action::CloseMenu => *mode = Mode::Normal,
            _ => return false,
        }
        return true;
    }
    false
}
