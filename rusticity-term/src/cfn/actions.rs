use crate::app::App;
use crate::common::{CyclicEnum, InputFocus, PageSize};
use crate::ui::cfn::{
    filtered_change_sets, filtered_cloudformation_stacks, filtered_events, filtered_outputs,
    filtered_parameters, filtered_resources, DetailTab as CfnDetailTab,
    EventsView as CfnEventsView, State as CfnStateConstants, STATUS_FILTER, VIEW_NESTED,
};

// ── Filter / reset ────────────────────────────────────────────────────────────

pub fn apply_filter_reset(app: &mut App) {
    if app.cfn_state.current_stack.is_some() && app.cfn_state.detail_tab == CfnDetailTab::Parameters
    {
        app.cfn_state.parameters.reset();
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Outputs
    {
        app.cfn_state.outputs.reset();
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Events
    {
        app.cfn_state.events.reset();
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::ChangeSets
    {
        app.cfn_state.change_sets.reset();
    } else {
        app.cfn_state.table.reset();
    }
}

pub fn reset_on_service_select(app: &mut App) {
    app.cfn_state.table.reset();
}

pub fn is_pagination_focused(app: &App) -> bool {
    app.cfn_state.input_focus == InputFocus::Pagination
}

pub fn filter_char_push(app: &mut App, c: char) {
    if app.cfn_state.current_stack.is_some() && app.cfn_state.detail_tab == CfnDetailTab::Parameters
    {
        if app.cfn_state.parameters_input_focus == InputFocus::Filter {
            app.cfn_state.parameters.filter.push(c);
            app.cfn_state.parameters.selected = 0;
        }
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Outputs
    {
        if app.cfn_state.outputs_input_focus == InputFocus::Filter {
            app.cfn_state.outputs.filter.push(c);
            app.cfn_state.outputs.selected = 0;
        }
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Resources
    {
        if app.cfn_state.resources_input_focus == InputFocus::Filter {
            app.cfn_state.resources.filter.push(c);
            app.cfn_state.resources.selected = 0;
        }
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Events
    {
        if app.cfn_state.events_input_focus == InputFocus::Filter {
            app.cfn_state.events.filter.push(c);
            app.cfn_state.events.selected = 0;
        }
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::ChangeSets
    {
        if app.cfn_state.change_sets_input_focus == InputFocus::Filter {
            app.cfn_state.change_sets.filter.push(c);
            app.cfn_state.change_sets.selected = 0;
        }
    } else if app.cfn_state.input_focus == InputFocus::Filter {
        app.cfn_state.table.filter.push(c);
        app.cfn_state.table.selected = 0;
    }
}

pub fn filter_char_pop(app: &mut App) {
    if app.cfn_state.current_stack.is_some() && app.cfn_state.detail_tab == CfnDetailTab::Parameters
    {
        if app.cfn_state.parameters_input_focus == InputFocus::Filter {
            app.cfn_state.parameters.filter.pop();
            app.cfn_state.parameters.selected = 0;
        }
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Outputs
    {
        if app.cfn_state.outputs_input_focus == InputFocus::Filter {
            app.cfn_state.outputs.filter.pop();
            app.cfn_state.outputs.selected = 0;
        }
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Resources
    {
        if app.cfn_state.resources_input_focus == InputFocus::Filter {
            app.cfn_state.resources.filter.pop();
            app.cfn_state.resources.selected = 0;
        }
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Events
    {
        if app.cfn_state.events_input_focus == InputFocus::Filter {
            app.cfn_state.events.filter.pop();
            app.cfn_state.events.selected = 0;
        }
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::ChangeSets
    {
        if app.cfn_state.change_sets_input_focus == InputFocus::Filter {
            app.cfn_state.change_sets.filter.pop();
            app.cfn_state.change_sets.selected = 0;
        }
    } else if app.cfn_state.input_focus == InputFocus::Filter {
        app.cfn_state.table.filter.pop();
        app.cfn_state.table.selected = 0;
    }
}

pub fn start_filter(app: &mut App) {
    if app.cfn_state.current_stack.is_some() && app.cfn_state.detail_tab == CfnDetailTab::Parameters
    {
        app.cfn_state.parameters_input_focus = InputFocus::Filter;
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Outputs
    {
        app.cfn_state.outputs_input_focus = InputFocus::Filter;
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Events
    {
        app.cfn_state.events_input_focus = InputFocus::Filter;
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::ChangeSets
    {
        app.cfn_state.change_sets_input_focus = InputFocus::Filter;
    } else {
        app.cfn_state.input_focus = InputFocus::Filter;
    }
}

pub fn next_filter_focus(app: &mut App) {
    if app.cfn_state.current_stack.is_some() && app.cfn_state.detail_tab == CfnDetailTab::Parameters
    {
        app.cfn_state.parameters_input_focus = app
            .cfn_state
            .parameters_input_focus
            .next(&CfnStateConstants::PARAMETERS_FILTER_CONTROLS);
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Outputs
    {
        app.cfn_state.outputs_input_focus = app
            .cfn_state
            .outputs_input_focus
            .next(&CfnStateConstants::OUTPUTS_FILTER_CONTROLS);
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Resources
    {
        app.cfn_state.resources_input_focus = app
            .cfn_state
            .resources_input_focus
            .next(&CfnStateConstants::RESOURCES_FILTER_CONTROLS);
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Events
    {
        app.cfn_state.events_input_focus = app
            .cfn_state
            .events_input_focus
            .next(&CfnStateConstants::EVENTS_FILTER_CONTROLS);
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::ChangeSets
    {
        app.cfn_state.change_sets_input_focus = app
            .cfn_state
            .change_sets_input_focus
            .next(&CfnStateConstants::CHANGE_SETS_FILTER_CONTROLS);
    } else {
        app.cfn_state.input_focus = app
            .cfn_state
            .input_focus
            .next(&CfnStateConstants::FILTER_CONTROLS);
    }
}

pub fn prev_filter_focus(app: &mut App) {
    if app.cfn_state.current_stack.is_some() && app.cfn_state.detail_tab == CfnDetailTab::Parameters
    {
        app.cfn_state.parameters_input_focus = app
            .cfn_state
            .parameters_input_focus
            .prev(&CfnStateConstants::PARAMETERS_FILTER_CONTROLS);
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Outputs
    {
        app.cfn_state.outputs_input_focus = app
            .cfn_state
            .outputs_input_focus
            .prev(&CfnStateConstants::OUTPUTS_FILTER_CONTROLS);
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Resources
    {
        app.cfn_state.resources_input_focus = app
            .cfn_state
            .resources_input_focus
            .prev(&CfnStateConstants::RESOURCES_FILTER_CONTROLS);
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Events
    {
        app.cfn_state.events_input_focus = app
            .cfn_state
            .events_input_focus
            .prev(&CfnStateConstants::EVENTS_FILTER_CONTROLS);
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::ChangeSets
    {
        app.cfn_state.change_sets_input_focus = app
            .cfn_state
            .change_sets_input_focus
            .prev(&CfnStateConstants::CHANGE_SETS_FILTER_CONTROLS);
    } else {
        app.cfn_state.input_focus = app
            .cfn_state
            .input_focus
            .prev(&CfnStateConstants::FILTER_CONTROLS);
    }
}

pub fn toggle_filter_checkbox(app: &mut App) {
    match app.cfn_state.input_focus {
        STATUS_FILTER => {
            app.cfn_state.status_filter = app.cfn_state.status_filter.next();
            app.cfn_state.table.reset();
        }
        VIEW_NESTED => {
            app.cfn_state.view_nested = !app.cfn_state.view_nested;
            // Clear expanded nested stack state
            app.cfn_state.expanded_items.clear();
            // Set loading=true to trigger a reload with the new nested setting.
            app.cfn_state.table.reset();
            app.cfn_state.table.loading = true;
        }
        _ => {}
    }
}

pub fn block_column_selector(app: &App) -> bool {
    app.cfn_state.current_stack.is_some()
        && (app.cfn_state.detail_tab == CfnDetailTab::Template
            || app.cfn_state.detail_tab == CfnDetailTab::GitSync)
}

// ── Navigation ────────────────────────────────────────────────────────────────

pub fn next_item(app: &mut App) {
    if app.cfn_state.current_stack.is_some() && app.cfn_state.detail_tab == CfnDetailTab::Template {
        scroll_down_template(app);
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Parameters
    {
        let filtered = filtered_parameters(app);
        app.cfn_state.parameters.next_item(filtered.len());
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Outputs
    {
        let filtered = filtered_outputs(app);
        app.cfn_state.outputs.next_item(filtered.len());
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Resources
    {
        let filtered = filtered_resources(app);
        app.cfn_state.resources.next_item(filtered.len());
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Events
    {
        let filtered = filtered_events(app);
        app.cfn_state.events.next_item(filtered.len());
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::ChangeSets
    {
        let filtered = filtered_change_sets(app);
        app.cfn_state.change_sets.next_item(filtered.len());
    } else {
        let filtered = filtered_cloudformation_stacks(app);
        // Compute extra rows from expanded content that appear before the NEXT selected item.
        let next_selected =
            (app.cfn_state.table.selected + 1).min(filtered.len().saturating_sub(1));
        let extra_rows = if let Some(exp_idx) = app.cfn_state.table.expanded_item {
            let page_size = app.cfn_state.table.page_size.value();
            let scroll = app.cfn_state.table.scroll_offset;
            // The expanded item must be on the current page and strictly before next_selected
            if exp_idx >= scroll && exp_idx < scroll + page_size && exp_idx < next_selected {
                app.cfn_visible_column_ids.len()
            } else {
                0
            }
        } else {
            0
        };
        app.cfn_state
            .table
            .next_item_with_expansion(filtered.len(), extra_rows);
    }
}

pub fn next_item_filter_input(app: &mut App) {
    if app.cfn_state.input_focus == STATUS_FILTER {
        app.cfn_state.status_filter = app.cfn_state.status_filter.next();
        app.cfn_state.table.reset();
    }
}

pub fn prev_item(app: &mut App) {
    if app.cfn_state.current_stack.is_some() && app.cfn_state.detail_tab == CfnDetailTab::Template {
        scroll_up_template(app);
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Parameters
    {
        app.cfn_state.parameters.prev_item();
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Outputs
    {
        app.cfn_state.outputs.prev_item();
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Resources
    {
        app.cfn_state.resources.prev_item();
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Events
    {
        app.cfn_state.events.prev_item();
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::ChangeSets
    {
        app.cfn_state.change_sets.prev_item();
    } else {
        app.cfn_state.table.prev_item();
    }
}

pub fn prev_item_filter_input(app: &mut App) {
    if app.cfn_state.input_focus == STATUS_FILTER {
        app.cfn_state.status_filter = app.cfn_state.status_filter.prev();
        app.cfn_state.table.reset();
    }
}

pub fn page_down_filter_input(app: &mut App) {
    if app.cfn_state.current_stack.is_some() && app.cfn_state.detail_tab == CfnDetailTab::Parameters
    {
        let page_size = app.cfn_state.parameters.page_size.value();
        let filtered_count = filtered_parameters(app).len();
        app.cfn_state.parameters_input_focus.handle_page_down(
            &mut app.cfn_state.parameters.selected,
            &mut app.cfn_state.parameters.scroll_offset,
            page_size,
            filtered_count,
        );
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Outputs
    {
        let page_size = app.cfn_state.outputs.page_size.value();
        let filtered_count = filtered_outputs(app).len();
        app.cfn_state.outputs_input_focus.handle_page_down(
            &mut app.cfn_state.outputs.selected,
            &mut app.cfn_state.outputs.scroll_offset,
            page_size,
            filtered_count,
        );
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Resources
    {
        let page_size = app.cfn_state.resources.page_size.value();
        let filtered_count = filtered_resources(app).len();
        app.cfn_state.resources_input_focus.handle_page_down(
            &mut app.cfn_state.resources.selected,
            &mut app.cfn_state.resources.scroll_offset,
            page_size,
            filtered_count,
        );
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Events
    {
        let page_size = app.cfn_state.events.page_size.value();
        let filtered_count = filtered_events(app).len();
        app.cfn_state.events_input_focus.handle_page_down(
            &mut app.cfn_state.events.selected,
            &mut app.cfn_state.events.scroll_offset,
            page_size,
            filtered_count,
        );
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::ChangeSets
    {
        let page_size = app.cfn_state.change_sets.page_size.value();
        let filtered_count = filtered_change_sets(app).len();
        app.cfn_state.change_sets_input_focus.handle_page_down(
            &mut app.cfn_state.change_sets.selected,
            &mut app.cfn_state.change_sets.scroll_offset,
            page_size,
            filtered_count,
        );
    } else {
        let page_size = app.cfn_state.table.page_size.value();
        let filtered_count = filtered_cloudformation_stacks(app).len();
        app.cfn_state.input_focus.handle_page_down(
            &mut app.cfn_state.table.selected,
            &mut app.cfn_state.table.scroll_offset,
            page_size,
            filtered_count,
        );
    }
}

pub fn page_down_normal(app: &mut App) {
    if app.cfn_state.current_stack.is_some() && app.cfn_state.detail_tab == CfnDetailTab::Parameters
    {
        let filtered = filtered_parameters(app);
        app.cfn_state.parameters.page_down(filtered.len());
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Outputs
    {
        let filtered = filtered_outputs(app);
        app.cfn_state.outputs.page_down(filtered.len());
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Resources
    {
        let filtered = filtered_resources(app);
        app.cfn_state.resources.page_down(filtered.len());
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Events
    {
        let filtered = filtered_events(app);
        app.cfn_state.events.page_down(filtered.len());
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::ChangeSets
    {
        let filtered = filtered_change_sets(app);
        app.cfn_state.change_sets.page_down(filtered.len());
    } else {
        let filtered = filtered_cloudformation_stacks(app);
        app.cfn_state.table.page_down(filtered.len());
    }
}

pub fn page_up_filter_input(app: &mut App) {
    if app.cfn_state.current_stack.is_some() && app.cfn_state.detail_tab == CfnDetailTab::Parameters
    {
        let page_size = app.cfn_state.parameters.page_size.value();
        app.cfn_state.parameters_input_focus.handle_page_up(
            &mut app.cfn_state.parameters.selected,
            &mut app.cfn_state.parameters.scroll_offset,
            page_size,
        );
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Outputs
    {
        let page_size = app.cfn_state.outputs.page_size.value();
        app.cfn_state.outputs_input_focus.handle_page_up(
            &mut app.cfn_state.outputs.selected,
            &mut app.cfn_state.outputs.scroll_offset,
            page_size,
        );
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Resources
    {
        let page_size = app.cfn_state.resources.page_size.value();
        app.cfn_state.resources_input_focus.handle_page_up(
            &mut app.cfn_state.resources.selected,
            &mut app.cfn_state.resources.scroll_offset,
            page_size,
        );
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Events
    {
        let page_size = app.cfn_state.events.page_size.value();
        app.cfn_state.events_input_focus.handle_page_up(
            &mut app.cfn_state.events.selected,
            &mut app.cfn_state.events.scroll_offset,
            page_size,
        );
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::ChangeSets
    {
        let page_size = app.cfn_state.change_sets.page_size.value();
        app.cfn_state.change_sets_input_focus.handle_page_up(
            &mut app.cfn_state.change_sets.selected,
            &mut app.cfn_state.change_sets.scroll_offset,
            page_size,
        );
    } else {
        let page_size = app.cfn_state.table.page_size.value();
        app.cfn_state.input_focus.handle_page_up(
            &mut app.cfn_state.table.selected,
            &mut app.cfn_state.table.scroll_offset,
            page_size,
        );
    }
}

pub fn page_up_normal(app: &mut App) {
    if app.cfn_state.current_stack.is_some() && app.cfn_state.detail_tab == CfnDetailTab::Parameters
    {
        app.cfn_state.parameters.page_up();
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Outputs
    {
        app.cfn_state.outputs.page_up();
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Resources
    {
        app.cfn_state.resources.page_up();
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Events
    {
        app.cfn_state.events.page_up();
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::ChangeSets
    {
        app.cfn_state.change_sets.page_up();
    } else {
        app.cfn_state.table.page_up();
    }
}

pub fn scroll_up_template(app: &mut App) {
    app.cfn_state.template_scroll = app.cfn_state.template_scroll.saturating_sub(1);
}

pub fn scroll_down_template(app: &mut App) {
    let lines = app.cfn_state.template_body.lines().count();
    let max_scroll = lines.saturating_sub(1);
    app.cfn_state.template_scroll = (app.cfn_state.template_scroll + 1).min(max_scroll);
}

pub fn scroll_up_template_fast(app: &mut App) {
    app.cfn_state.template_scroll = app.cfn_state.template_scroll.saturating_sub(10);
}

pub fn scroll_down_template_fast(app: &mut App) {
    let lines = app.cfn_state.template_body.lines().count();
    let max_scroll = lines.saturating_sub(1);
    app.cfn_state.template_scroll = (app.cfn_state.template_scroll + 10).min(max_scroll);
}

pub fn expand_row(app: &mut App) {
    if app.cfn_state.current_stack.is_none() {
        app.cfn_state.table.toggle_expand();
        // If just expanded, scroll up to ensure detail rows are visible
        if app.cfn_state.table.expanded_item.is_some() {
            let detail_rows = app.cfn_visible_column_ids.len();
            let page_size = app.cfn_state.table.page_size.value();
            app.cfn_state
                .table
                .ensure_expansion_visible(detail_rows, page_size);
        }
    } else if app.cfn_state.detail_tab == CfnDetailTab::Parameters {
        app.cfn_state.parameters.toggle_expand();
    } else if app.cfn_state.detail_tab == CfnDetailTab::Outputs {
        app.cfn_state.outputs.toggle_expand();
    } else if app.cfn_state.detail_tab == CfnDetailTab::Resources {
        app.cfn_state.resources.toggle_expand();
    } else if app.cfn_state.detail_tab == CfnDetailTab::Events {
        app.cfn_state.events.toggle_expand();
    } else if app.cfn_state.detail_tab == CfnDetailTab::ChangeSets {
        app.cfn_state.change_sets.toggle_expand();
    }
}

pub fn prev_pane(app: &mut App) {
    if app.cfn_state.current_stack.is_none() {
        app.cfn_state.table.collapse();
    } else if app.cfn_state.detail_tab == CfnDetailTab::Parameters {
        app.cfn_state.parameters.collapse();
    } else if app.cfn_state.detail_tab == CfnDetailTab::Outputs {
        app.cfn_state.outputs.collapse();
    } else if app.cfn_state.detail_tab == CfnDetailTab::Resources {
        app.cfn_state.resources.collapse();
    } else if app.cfn_state.detail_tab == CfnDetailTab::Events {
        app.cfn_state.events.collapse();
    } else if app.cfn_state.detail_tab == CfnDetailTab::ChangeSets {
        app.cfn_state.change_sets.collapse();
    }
}

pub fn collapse_row(app: &mut App) {
    if app.cfn_state.current_stack.is_some() {
        match app.cfn_state.detail_tab {
            CfnDetailTab::Resources => app.cfn_state.resources.collapse(),
            CfnDetailTab::Parameters => app.cfn_state.parameters.collapse(),
            CfnDetailTab::Outputs => app.cfn_state.outputs.collapse(),
            CfnDetailTab::Events => app.cfn_state.events.collapse(),
            CfnDetailTab::ChangeSets => app.cfn_state.change_sets.collapse(),
            _ => {}
        }
    } else {
        app.cfn_state.table.collapse();
    }
}

pub fn go_to_page(app: &mut App, page: usize) {
    let filtered_count = filtered_cloudformation_stacks(app).len();
    app.cfn_state.table.goto_page(page, filtered_count);
}

// ── Actions ───────────────────────────────────────────────────────────────────

pub fn select_item(app: &mut App) {
    if app.cfn_state.current_stack.is_none() {
        let filtered = filtered_cloudformation_stacks(app);
        if let Some(stack) = app.cfn_state.table.get_selected(&filtered) {
            let stack_name = stack.name.clone();
            let mut tags = stack.tags.clone();
            tags.sort_by(|a, b| a.0.cmp(&b.0));
            app.cfn_state.current_stack = Some(stack_name);
            app.cfn_state.tags.items = tags;
            app.cfn_state.tags.reset();
            app.cfn_state.table.loading = true;
            app.update_current_tab_breadcrumb();
        }
    }
}

pub fn go_back(app: &mut App) {
    app.cfn_state.current_stack = None;
    app.update_current_tab_breadcrumb();
}

pub fn next_detail_tab(app: &mut App) {
    app.cfn_state.detail_tab = app.cfn_state.detail_tab.next();
}

pub fn prev_detail_tab(app: &mut App) {
    app.cfn_state.detail_tab = app.cfn_state.detail_tab.prev();
}

/// Toggle between Table and Timeline view on the Events tab (bound to Tab key).
pub fn toggle_events_view(app: &mut App) {
    app.cfn_state.events_view = match app.cfn_state.events_view {
        CfnEventsView::Table => CfnEventsView::Timeline,
        CfnEventsView::Timeline => CfnEventsView::Table,
    };
}

pub fn yank(app: &App) {
    use crate::app::copy_to_clipboard;
    if let Some(stack_name) = &app.cfn_state.current_stack {
        // On Template tab — copy the full template body
        if app.cfn_state.detail_tab == CfnDetailTab::Template {
            if !app.cfn_state.template_body.is_empty() {
                copy_to_clipboard(&app.cfn_state.template_body);
            }
            return;
        }
        if let Some(stack) = app
            .cfn_state
            .table
            .items
            .iter()
            .find(|s| &s.name == stack_name)
        {
            copy_to_clipboard(&stack.stack_id);
        }
    } else {
        let filtered = filtered_cloudformation_stacks(app);
        if let Some(stack) = app.cfn_state.table.get_selected(&filtered) {
            copy_to_clipboard(&stack.stack_id);
        }
    }
}

// ── Column preferences ────────────────────────────────────────────────────────

pub fn toggle_column(app: &mut App) {
    let idx = app.column_selector_index;
    if app.cfn_state.current_stack.is_some() && app.cfn_state.detail_tab == CfnDetailTab::StackInfo
    {
        // Tags: page size only
        if idx == 4 {
            app.cfn_state.tags.page_size = PageSize::Ten;
        } else if idx == 5 {
            app.cfn_state.tags.page_size = PageSize::TwentyFive;
        } else if idx == 6 {
            app.cfn_state.tags.page_size = PageSize::Fifty;
        } else if idx == 7 {
            app.cfn_state.tags.page_size = PageSize::OneHundred;
        }
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Parameters
    {
        if idx > 0 && idx <= app.cfn_parameter_column_ids.len() {
            if let Some(col) = app.cfn_parameter_column_ids.get(idx - 1) {
                if let Some(pos) = app
                    .cfn_parameter_visible_column_ids
                    .iter()
                    .position(|c| c == col)
                {
                    if app.cfn_parameter_visible_column_ids.len() > 1 {
                        app.cfn_parameter_visible_column_ids.remove(pos);
                    }
                } else {
                    app.cfn_parameter_visible_column_ids.push(col);
                }
            }
        } else if idx == app.cfn_parameter_column_ids.len() + 3 {
            app.cfn_state.parameters.page_size = PageSize::Ten;
        } else if idx == app.cfn_parameter_column_ids.len() + 4 {
            app.cfn_state.parameters.page_size = PageSize::TwentyFive;
        } else if idx == app.cfn_parameter_column_ids.len() + 5 {
            app.cfn_state.parameters.page_size = PageSize::Fifty;
        } else if idx == app.cfn_parameter_column_ids.len() + 6 {
            app.cfn_state.parameters.page_size = PageSize::OneHundred;
        }
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Outputs
    {
        if idx > 0 && idx <= app.cfn_output_column_ids.len() {
            if let Some(col) = app.cfn_output_column_ids.get(idx - 1) {
                if let Some(pos) = app
                    .cfn_output_visible_column_ids
                    .iter()
                    .position(|c| c == col)
                {
                    if app.cfn_output_visible_column_ids.len() > 1 {
                        app.cfn_output_visible_column_ids.remove(pos);
                    }
                } else {
                    app.cfn_output_visible_column_ids.push(col);
                }
            }
        } else if idx == app.cfn_output_column_ids.len() + 3 {
            app.cfn_state.outputs.page_size = PageSize::Ten;
        } else if idx == app.cfn_output_column_ids.len() + 4 {
            app.cfn_state.outputs.page_size = PageSize::TwentyFive;
        } else if idx == app.cfn_output_column_ids.len() + 5 {
            app.cfn_state.outputs.page_size = PageSize::Fifty;
        } else if idx == app.cfn_output_column_ids.len() + 6 {
            app.cfn_state.outputs.page_size = PageSize::OneHundred;
        }
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Resources
    {
        if idx > 0 && idx <= app.cfn_resource_column_ids.len() {
            if let Some(col) = app.cfn_resource_column_ids.get(idx - 1) {
                if let Some(pos) = app
                    .cfn_resource_visible_column_ids
                    .iter()
                    .position(|c| c == col)
                {
                    if app.cfn_resource_visible_column_ids.len() > 1 {
                        app.cfn_resource_visible_column_ids.remove(pos);
                    }
                } else {
                    app.cfn_resource_visible_column_ids.push(col);
                }
            }
        } else if idx == app.cfn_resource_column_ids.len() + 3 {
            app.cfn_state.resources.page_size = PageSize::Ten;
        } else if idx == app.cfn_resource_column_ids.len() + 4 {
            app.cfn_state.resources.page_size = PageSize::TwentyFive;
        } else if idx == app.cfn_resource_column_ids.len() + 5 {
            app.cfn_state.resources.page_size = PageSize::Fifty;
        } else if idx == app.cfn_resource_column_ids.len() + 6 {
            app.cfn_state.resources.page_size = PageSize::OneHundred;
        }
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::Events
    {
        if idx > 0 && idx <= app.cfn_event_column_ids.len() {
            if let Some(col) = app.cfn_event_column_ids.get(idx - 1) {
                if let Some(pos) = app
                    .cfn_event_visible_column_ids
                    .iter()
                    .position(|c| c == col)
                {
                    if app.cfn_event_visible_column_ids.len() > 1 {
                        app.cfn_event_visible_column_ids.remove(pos);
                    }
                } else {
                    app.cfn_event_visible_column_ids.push(col);
                }
            }
        } else if idx == app.cfn_event_column_ids.len() + 3 {
            app.cfn_state.events.page_size = PageSize::Ten;
        } else if idx == app.cfn_event_column_ids.len() + 4 {
            app.cfn_state.events.page_size = PageSize::TwentyFive;
        } else if idx == app.cfn_event_column_ids.len() + 5 {
            app.cfn_state.events.page_size = PageSize::Fifty;
        } else if idx == app.cfn_event_column_ids.len() + 6 {
            app.cfn_state.events.page_size = PageSize::OneHundred;
        }
    } else if app.cfn_state.current_stack.is_some()
        && app.cfn_state.detail_tab == CfnDetailTab::ChangeSets
    {
        if idx > 0 && idx <= app.cfn_change_set_column_ids.len() {
            if let Some(col) = app.cfn_change_set_column_ids.get(idx - 1) {
                if let Some(pos) = app
                    .cfn_change_set_visible_column_ids
                    .iter()
                    .position(|c| c == col)
                {
                    if app.cfn_change_set_visible_column_ids.len() > 1 {
                        app.cfn_change_set_visible_column_ids.remove(pos);
                    }
                } else {
                    app.cfn_change_set_visible_column_ids.push(col);
                }
            }
        } else if idx == app.cfn_change_set_column_ids.len() + 3 {
            app.cfn_state.change_sets.page_size = PageSize::Ten;
        } else if idx == app.cfn_change_set_column_ids.len() + 4 {
            app.cfn_state.change_sets.page_size = PageSize::TwentyFive;
        } else if idx == app.cfn_change_set_column_ids.len() + 5 {
            app.cfn_state.change_sets.page_size = PageSize::Fifty;
        } else if idx == app.cfn_change_set_column_ids.len() + 6 {
            app.cfn_state.change_sets.page_size = PageSize::OneHundred;
        }
    } else if app.cfn_state.current_stack.is_none() {
        if idx > 0 && idx <= app.cfn_column_ids.len() {
            if let Some(col) = app.cfn_column_ids.get(idx - 1) {
                if let Some(pos) = app.cfn_visible_column_ids.iter().position(|c| c == col) {
                    if app.cfn_visible_column_ids.len() > 1 {
                        app.cfn_visible_column_ids.remove(pos);
                    }
                } else {
                    app.cfn_visible_column_ids.push(*col);
                }
            }
        } else if idx == app.cfn_column_ids.len() + 3 {
            app.cfn_state.table.page_size = PageSize::Ten;
        } else if idx == app.cfn_column_ids.len() + 4 {
            app.cfn_state.table.page_size = PageSize::TwentyFive;
        } else if idx == app.cfn_column_ids.len() + 5 {
            app.cfn_state.table.page_size = PageSize::Fifty;
        } else if idx == app.cfn_column_ids.len() + 6 {
            app.cfn_state.table.page_size = PageSize::OneHundred;
        }
    }
}

pub fn next_preferences(app: &mut App) {
    let cols = column_count(app);
    let page_size_idx = cols + 2;
    if app.column_selector_index < page_size_idx {
        app.column_selector_index = page_size_idx;
    } else {
        app.column_selector_index = 0;
    }
}

pub fn prev_preferences(app: &mut App) {
    let cols = column_count(app);
    let page_size_idx = cols + 2;
    if app.column_selector_index >= page_size_idx {
        app.column_selector_index = 0;
    } else {
        app.column_selector_index = page_size_idx;
    }
}

pub fn column_selector_max(app: &App) -> usize {
    column_count(app) + 6
}

pub fn column_count(app: &App) -> usize {
    if app.cfn_state.current_stack.is_some() {
        match app.cfn_state.detail_tab {
            CfnDetailTab::Parameters => app.cfn_parameter_column_ids.len(),
            CfnDetailTab::Outputs => app.cfn_output_column_ids.len(),
            CfnDetailTab::Resources => app.cfn_resource_column_ids.len(),
            CfnDetailTab::Events => app.cfn_event_column_ids.len(),
            CfnDetailTab::ChangeSets => app.cfn_change_set_column_ids.len(),
            _ => 0,
        }
    } else {
        app.cfn_column_ids.len()
    }
}

// ── Breadcrumb / console URL ──────────────────────────────────────────────────

pub fn breadcrumb(app: &App) -> Vec<String> {
    let mut parts = vec!["CloudFormation".to_string()];
    if let Some(stack_name) = &app.cfn_state.current_stack {
        parts.push(stack_name.clone());
    } else {
        parts.push("Stacks".to_string());
    }
    parts
}

pub fn console_url(app: &App) -> String {
    use crate::cfn;
    if let Some(stack_name) = &app.cfn_state.current_stack {
        if let Some(stack) = app
            .cfn_state
            .table
            .items
            .iter()
            .find(|s| &s.name == stack_name)
        {
            return cfn::console_url_stack_detail_with_tab(
                &app.config.region,
                &stack.stack_id,
                &app.cfn_state.detail_tab,
            );
        }
    }
    cfn::console_url_stacks(&app.config.region)
}
