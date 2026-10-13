use crate::app::{copy_to_clipboard, App, Service};
use crate::common::InputFocus;
use crate::keymap::{Action, Mode};
use crate::rds;
use crate::ui::rds::{filtered_instances, FILTER_CONTROLS};

pub fn console_url(app: &App) -> String {
    let instances = filtered_instances(app);
    match app.rds_state.instances.get_selected(&instances) {
        Some(instance) if !instance.identifier.is_empty() => {
            rds::console_url_instance(&app.config.region, &instance.identifier)
        }
        _ => rds::console_url_instances(&app.config.region),
    }
}

pub fn selected_arn(app: &App) -> Option<String> {
    let instances = filtered_instances(app);
    app.rds_state
        .instances
        .get_selected(&instances)
        .map(|instance| {
            if instance.arn.is_empty() {
                instance.identifier.clone()
            } else {
                instance.arn.clone()
            }
        })
}

pub fn go_to_page(app: &mut App, page: usize) {
    if page == 0 {
        return;
    }
    let count = filtered_instances(app).len();
    // Clamp before delegating, including extremely large typed page numbers.
    let pages = app.rds_state.instances.total_pages(count).max(1);
    app.rds_state.instances.goto_page(page.min(pages), count);
    app.rds_state.instances.collapse();
}

/// Handle only this service's view-specific input, before legacy CloudWatch
/// ViewMode branches. Global pickers, menus, Preferences, and shortcuts stay shared.
pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !app.service_selected || app.current_service != Service::RdsInstances {
        return false;
    }
    if app.mode == Mode::ErrorModal && *action == Action::RetryLoad {
        app.error_message = None;
        app.error_scroll = 0;
        app.mode = Mode::Normal;
        app.rds_state.instances.loading = true;
        return true;
    }
    if app.mode == Mode::Normal {
        match action {
            Action::NextItem => {
                let count = filtered_instances(app).len();
                app.rds_state.instances.next_item(count);
            }
            Action::PrevItem => app.rds_state.instances.prev_item(),
            Action::PageDown | Action::ScrollDown => {
                let count = filtered_instances(app).len();
                app.rds_state.instances.page_down(count);
                app.rds_state.instances.collapse();
            }
            Action::PageUp | Action::ScrollUp => {
                app.rds_state.instances.page_up();
                app.rds_state.instances.collapse();
            }
            Action::Select | Action::ExpandRow | Action::NextPane => {
                if app
                    .rds_state
                    .instances
                    .get_selected(&filtered_instances(app))
                    .is_some()
                {
                    app.rds_state.instances.toggle_expand();
                }
            }
            Action::CollapseRow | Action::PrevPane | Action::GoBack => {
                app.rds_state.instances.collapse()
            }
            Action::StartFilter => {
                app.mode = Mode::FilterInput;
                app.rds_state.input_focus = InputFocus::Filter;
            }
            Action::Refresh => app.rds_state.instances.loading = true,
            Action::Yank => {
                if let Some(arn) = selected_arn(app) {
                    copy_to_clipboard(&arn);
                }
            }
            // No detail tabs, sort toggles, or server-side filter switches yet.
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
    if app.mode == Mode::FilterInput {
        let count = filtered_instances(app).len();
        let state = &mut app.rds_state;
        match action {
            Action::NextFilterFocus => state.input_focus = state.input_focus.next(&FILTER_CONTROLS),
            Action::PrevFilterFocus => state.input_focus = state.input_focus.prev(&FILTER_CONTROLS),
            Action::PageDown | Action::PageUp => {
                if state.input_focus == InputFocus::Pagination {
                    if *action == Action::PageDown {
                        state.input_focus.handle_page_down(
                            &mut state.instances.selected,
                            &mut state.instances.scroll_offset,
                            state.instances.page_size.value(),
                            count,
                        );
                    } else {
                        state.input_focus.handle_page_up(
                            &mut state.instances.selected,
                            &mut state.instances.scroll_offset,
                            state.instances.page_size.value(),
                        );
                    }
                    state.instances.collapse();
                }
            }
            Action::NextItem | Action::PrevItem => {} // No dropdowns in this filter bar.
            Action::FilterInput(c) if state.input_focus == InputFocus::Filter => {
                state.instances.filter_push(*c)
            }
            Action::FilterBackspace if state.input_focus == InputFocus::Filter => {
                state.instances.filter_pop()
            }
            Action::ToggleFilterCheckbox if state.input_focus == InputFocus::Filter => {
                state.instances.filter_push(' ')
            }
            Action::FilterInput(c) if c.is_ascii_digit() => app.page_input.push(*c),
            Action::FilterBackspace => {
                app.page_input.pop();
            }
            Action::FilterInput(_) | Action::ToggleFilterCheckbox => {}
            Action::ApplyFilter => {
                if let Ok(page) = app.page_input.parse() {
                    go_to_page(app, page);
                }
                app.page_input.clear();
                app.mode = Mode::Normal;
            }
            Action::CloseMenu => app.mode = Mode::Normal,
            _ => return false,
        }
        return true;
    }
    false
}
