use crate::app::App;
use crate::common::{CyclicEnum, InputFocus, PageSize};
use crate::ui::ec2::{
    filtered_ec2_instances, filtered_tags, DetailTab as Ec2DetailTab, FILTER_CONTROLS,
    STATE_FILTER as EC2_STATE_FILTER,
};
use crate::ui::monitoring::MonitoringState;

// ── Filter ────────────────────────────────────────────────────────────────────

pub fn apply_filter_reset(app: &mut App) {
    app.ec2_state.table.reset();
}

pub fn get_active_filter_mut(app: &mut App) -> Option<&mut String> {
    if app.ec2_state.current_instance.is_some() && app.ec2_state.detail_tab == Ec2DetailTab::Tags {
        Some(&mut app.ec2_state.tags.filter)
    } else {
        Some(&mut app.ec2_state.table.filter)
    }
}

pub fn is_pagination_focused(app: &App) -> bool {
    app.ec2_state.input_focus == InputFocus::Pagination
}

pub fn next_filter_focus(app: &mut App) {
    app.ec2_state.input_focus = app.ec2_state.input_focus.next(&FILTER_CONTROLS);
}

pub fn prev_filter_focus(app: &mut App) {
    app.ec2_state.input_focus = app.ec2_state.input_focus.prev(&FILTER_CONTROLS);
}

pub fn filter_char_push(app: &mut App, c: char) {
    if app.ec2_state.input_focus == InputFocus::Filter {
        app.ec2_state.tags.filter.push(c);
        app.ec2_state.tags.selected = 0;
    }
}

pub fn filter_char_pop(app: &mut App) {
    if app.ec2_state.input_focus == InputFocus::Filter {
        app.ec2_state.tags.filter.pop();
        app.ec2_state.tags.selected = 0;
    }
}

pub fn toggle_state_filter_next(app: &mut App) {
    if app.ec2_state.input_focus == EC2_STATE_FILTER {
        app.ec2_state.state_filter = app.ec2_state.state_filter.next();
        app.ec2_state.table.reset();
    }
}

pub fn toggle_state_filter_prev(app: &mut App) {
    if app.ec2_state.input_focus == EC2_STATE_FILTER {
        app.ec2_state.state_filter = app.ec2_state.state_filter.prev();
        app.ec2_state.table.reset();
    }
}

pub fn toggle_filter_checkbox(app: &mut App) {
    if app.ec2_state.input_focus == EC2_STATE_FILTER {
        app.ec2_state.state_filter = app.ec2_state.state_filter.next();
        app.ec2_state.table.reset();
    }
}

// ── Navigation ────────────────────────────────────────────────────────────────

pub fn next_item(app: &mut App) {
    if app.ec2_state.current_instance.is_some() && app.ec2_state.detail_tab == Ec2DetailTab::Tags {
        let filtered = filtered_tags(app);
        if !filtered.is_empty() {
            app.ec2_state.tags.next_item(filtered.len());
        }
    } else {
        let filtered = filtered_ec2_instances(app);
        if !filtered.is_empty() {
            app.ec2_state.table.next_item(filtered.len());
        }
    }
}

pub fn prev_item(app: &mut App) {
    if app.ec2_state.current_instance.is_some() && app.ec2_state.detail_tab == Ec2DetailTab::Tags {
        app.ec2_state.tags.prev_item();
    } else {
        app.ec2_state.table.prev_item();
    }
}

pub fn page_down_normal(app: &mut App) {
    let filtered = filtered_ec2_instances(app);
    if !filtered.is_empty() {
        app.ec2_state.table.page_down(filtered.len());
    }
}

pub fn page_up_normal(app: &mut App) {
    if app.ec2_state.current_instance.is_some()
        && app.ec2_state.detail_tab == Ec2DetailTab::Monitoring
        && !app.ec2_state.is_metrics_loading()
    {
        app.ec2_state
            .set_monitoring_scroll(app.ec2_state.monitoring_scroll().saturating_sub(1));
    } else {
        app.ec2_state.table.page_up();
    }
}

pub fn scroll_up(app: &mut App) {
    if app.ec2_state.current_instance.is_some()
        && app.ec2_state.detail_tab == Ec2DetailTab::Monitoring
        && !app.ec2_state.is_metrics_loading()
    {
        app.ec2_state
            .set_monitoring_scroll(app.ec2_state.monitoring_scroll().saturating_sub(1));
    }
}

pub fn scroll_down(app: &mut App) {
    if app.ec2_state.current_instance.is_some()
        && app.ec2_state.detail_tab == Ec2DetailTab::Monitoring
        && !app.ec2_state.is_metrics_loading()
    {
        app.ec2_state
            .set_monitoring_scroll((app.ec2_state.monitoring_scroll() + 1).min(5));
    }
}

pub fn expand_row(app: &mut App) {
    if app.ec2_state.current_instance.is_some() && app.ec2_state.detail_tab == Ec2DetailTab::Tags {
        app.ec2_state.tags.toggle_expand();
    } else if !app.ec2_state.table.is_expanded() {
        app.ec2_state.table.toggle_expand();
    }
}

pub fn prev_pane(app: &mut App) {
    app.ec2_state.table.collapse();
}

pub fn collapse_row(app: &mut App) {
    if app.ec2_state.current_instance.is_some() && app.ec2_state.detail_tab == Ec2DetailTab::Tags {
        app.ec2_state.tags.collapse();
    } else {
        app.ec2_state.table.collapse();
    }
}

// ── Actions ───────────────────────────────────────────────────────────────────

pub fn select_item(app: &mut App) {
    if app.ec2_state.current_instance.is_none() {
        let filtered = filtered_ec2_instances(app);
        if let Some(instance) = app.ec2_state.table.get_selected(&filtered) {
            app.ec2_state.current_instance = Some(instance.instance_id.clone());
            app.view_mode = crate::app::ViewMode::Detail;
            app.update_current_tab_breadcrumb();
        }
    }
}

pub fn go_back(app: &mut App) {
    app.ec2_state.current_instance = None;
    app.view_mode = crate::app::ViewMode::List;
    app.update_current_tab_breadcrumb();
}

pub fn next_detail_tab(app: &mut App) {
    app.ec2_state.detail_tab = app.ec2_state.detail_tab.next();
    if app.ec2_state.detail_tab == Ec2DetailTab::Tags {
        app.ec2_state.tags.loading = true;
    } else if app.ec2_state.detail_tab == Ec2DetailTab::Monitoring {
        app.ec2_state.set_metrics_loading(true);
        app.ec2_state.set_monitoring_scroll(0);
        app.ec2_state.clear_metrics();
    }
}

pub fn prev_detail_tab(app: &mut App) {
    app.ec2_state.detail_tab = app.ec2_state.detail_tab.prev();
    if app.ec2_state.detail_tab == Ec2DetailTab::Tags {
        app.ec2_state.tags.loading = true;
    } else if app.ec2_state.detail_tab == Ec2DetailTab::Monitoring {
        app.ec2_state.set_metrics_loading(true);
        app.ec2_state.set_monitoring_scroll(0);
        app.ec2_state.clear_metrics();
    }
}

pub fn block_column_selector(app: &App) -> bool {
    app.ec2_state.table.expanded_item.is_some() && app.ec2_state.detail_tab != Ec2DetailTab::Tags
}

// ── Column preferences ────────────────────────────────────────────────────────

pub fn toggle_column(app: &mut App) {
    let idx = app.column_selector_index;
    if app.ec2_state.current_instance.is_some() && app.ec2_state.detail_tab == Ec2DetailTab::Tags {
        if idx > 0 && idx <= app.ec2_state.tag_column_ids.len() {
            if let Some(col) = app.ec2_state.tag_column_ids.get(idx - 1) {
                if let Some(pos) = app
                    .ec2_state
                    .tag_visible_column_ids
                    .iter()
                    .position(|c| c == col)
                {
                    if app.ec2_state.tag_visible_column_ids.len() > 1 {
                        app.ec2_state.tag_visible_column_ids.remove(pos);
                    }
                } else {
                    app.ec2_state.tag_visible_column_ids.push(col.clone());
                }
            }
        } else if idx == app.ec2_state.tag_column_ids.len() + 3 {
            app.ec2_state.tags.page_size = PageSize::Ten;
        } else if idx == app.ec2_state.tag_column_ids.len() + 4 {
            app.ec2_state.tags.page_size = PageSize::TwentyFive;
        } else if idx == app.ec2_state.tag_column_ids.len() + 5 {
            app.ec2_state.tags.page_size = PageSize::Fifty;
        } else if idx == app.ec2_state.tag_column_ids.len() + 6 {
            app.ec2_state.tags.page_size = PageSize::OneHundred;
        }
    } else {
        if idx > 0 && idx <= app.ec2_column_ids.len() {
            if let Some(col) = app.ec2_column_ids.get(idx - 1) {
                if let Some(pos) = app.ec2_visible_column_ids.iter().position(|c| c == col) {
                    if app.ec2_visible_column_ids.len() > 1 {
                        app.ec2_visible_column_ids.remove(pos);
                    }
                } else {
                    app.ec2_visible_column_ids.push(*col);
                }
            }
        } else if idx == app.ec2_column_ids.len() + 3 {
            app.ec2_state.table.page_size = PageSize::Ten;
        } else if idx == app.ec2_column_ids.len() + 4 {
            app.ec2_state.table.page_size = PageSize::TwentyFive;
        } else if idx == app.ec2_column_ids.len() + 5 {
            app.ec2_state.table.page_size = PageSize::Fifty;
        } else if idx == app.ec2_column_ids.len() + 6 {
            app.ec2_state.table.page_size = PageSize::OneHundred;
        }
    }
}

pub fn next_preferences(app: &mut App) {
    let page_size_idx = app.ec2_column_ids.len() + 2;
    if app.column_selector_index < page_size_idx {
        app.column_selector_index = page_size_idx;
    } else {
        app.column_selector_index = 0;
    }
}

pub fn column_selector_max(app: &App) -> usize {
    if app.ec2_state.current_instance.is_some() && app.ec2_state.detail_tab == Ec2DetailTab::Tags {
        app.ec2_state.tag_column_ids.len() + 6
    } else {
        app.ec2_column_ids.len() + 6
    }
}

pub fn column_count(app: &App) -> usize {
    if app.ec2_state.current_instance.is_some() && app.ec2_state.detail_tab == Ec2DetailTab::Tags {
        app.ec2_state.tag_column_ids.len()
    } else {
        app.ec2_column_ids.len()
    }
}

// ── Breadcrumb / console URL ──────────────────────────────────────────────────

pub fn breadcrumb(app: &App) -> Vec<String> {
    let mut parts = vec!["EC2".to_string()];
    if let Some(id) = &app.ec2_state.current_instance {
        parts.push(id.clone());
    } else {
        parts.push("Instances".to_string());
    }
    parts
}

pub fn console_url(app: &App) -> String {
    if let Some(instance_id) = &app.ec2_state.current_instance {
        format!(
            "https://{}.console.aws.amazon.com/ec2/home?region={}#InstanceDetails:instanceId={}",
            app.config.region, app.config.region, instance_id
        )
    } else {
        format!(
            "https://{}.console.aws.amazon.com/ec2/home?region={}#Instances:",
            app.config.region, app.config.region
        )
    }
}
