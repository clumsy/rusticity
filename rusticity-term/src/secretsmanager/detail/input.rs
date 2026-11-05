//! Filter and pagination input for the secret detail tables, using the same
//! InventoryInput model as every other service.
use super::*;
use crate::inventory::{self, InventoryInput};

const TABLE_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

pub(super) fn handle_action(app: &mut App, action: &Action) -> bool {
    let view = app.secretsmanager_state.detail.view.as_mut().unwrap();
    let next = matches!(action, Action::NextFilterFocus);
    let prev = matches!(action, Action::PrevFilterFocus);
    if app.mode == Mode::FilterInput && (next || prev) {
        let focus = view.data().focus;
        view.data_mut().focus = if next {
            focus.next(&TABLE_CONTROLS)
        } else {
            focus.prev(&TABLE_CONTROLS)
        };
        app.page_input.clear();
        return true;
    }
    if app.mode != Mode::FilterInput {
        return false;
    }
    let filtered_count = view.data().filtered().len();
    let data = view.data_mut();
    if *action == Action::DeleteWord {
        if data.focus == InputFocus::Filter {
            data.table.filter_clear();
        } else {
            app.page_input.clear();
        }
        return true;
    }
    // Unhandled detail-filter keys must not reach the inventory filter.
    inventory::handle_action(
        InventoryInput {
            table: &mut data.table,
            focus: &mut data.focus,
            mode: &mut app.mode,
            page_input: &mut app.page_input,
            error_message: &mut app.error_message,
            error_scroll: &mut app.error_scroll,
            filtered_count,
            selected_arn: None,
            clipboard: &app.clipboard,
        },
        action,
    );
    true
}
