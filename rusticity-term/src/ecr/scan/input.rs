//! Use the shared filter/pagination input model; only source selection is ECR-specific.
use super::*;
use crate::{
    common::InputFocus,
    inventory::{self, InventoryInput},
};

pub(super) const SOURCE: InputFocus = InputFocus::Dropdown("EcrReplicationSource");
const REPLICATION_CONTROLS: [InputFocus; 3] = [InputFocus::Filter, SOURCE, InputFocus::Pagination];
const TABLE_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

pub(super) fn handle_action(app: &mut App, action: &Action) -> bool {
    let state = &mut app.ecr_state.scan;
    let view = state.view.as_mut().unwrap();
    let selecting = view.selecting_source();
    let next = matches!(action, Action::NextFilterFocus)
        || (selecting && *action == Action::NextDetailTab);
    let prev = matches!(action, Action::PrevFilterFocus)
        || (selecting && *action == Action::PrevDetailTab);
    if (app.mode == Mode::FilterInput || selecting) && (next || prev) {
        let controls = if view.tab == DetailTab::Replication {
            &REPLICATION_CONTROLS[..]
        } else {
            &TABLE_CONTROLS[..]
        };
        let focus = view.data().focus;
        view.data_mut().focus = if next {
            focus.next(controls)
        } else {
            focus.prev(controls)
        };
        view.source_picker = if view.data().focus == SOURCE {
            Some(
                view.sources()
                    .iter()
                    .position(|s| *s == view.source_tag)
                    .unwrap_or(0),
            )
        } else {
            None
        };
        app.page_input.clear();
        app.mode = Mode::FilterInput;
        return true;
    }
    if let Some(selected) = view.source_picker {
        let sources = view.sources();
        match action {
            Action::NextItem => {
                view.source_picker = Some((selected + 1).min(sources.len().saturating_sub(1)))
            }
            Action::PrevItem => view.source_picker = Some(selected.saturating_sub(1)),
            Action::Select | Action::ApplyFilter => {
                if let Some(source) = sources.get(selected) {
                    let changed = view.source_tag != *source;
                    view.source_tag = source.clone();
                    view.source_picker = None;
                    view.data_mut().focus = InputFocus::Filter;
                    if changed {
                        state.activate(DetailTab::Replication, true);
                    }
                    app.page_input.clear();
                    app.mode = Mode::Normal;
                }
            }
            Action::GoBack | Action::CloseMenu => {
                view.source_picker = None;
                view.data_mut().focus = InputFocus::Filter;
                app.page_input.clear();
                app.mode = Mode::Normal;
            }
            Action::Quit | Action::ConfirmQuit | Action::CloseService => return false,
            _ => {}
        }
        return true;
    }
    if app.mode != Mode::FilterInput {
        return false;
    }
    let filtered_count = view.data().filtered(view.tab).len();
    let data = view.data_mut();
    if *action == Action::DeleteWord {
        if data.focus == InputFocus::Filter {
            data.table.filter_clear();
        } else {
            app.page_input.clear();
        }
        return true;
    }
    // Unhandled detail-filter keys must not reach the parent image-list filter.
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
