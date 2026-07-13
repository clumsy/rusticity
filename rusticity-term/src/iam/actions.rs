use crate::app::App;
use crate::common::{CyclicEnum, InputFocus, PageSize};
use crate::ui::iam::{
    filtered_iam_policies, filtered_iam_roles, filtered_iam_users, filtered_last_accessed,
    filtered_tags as filtered_iam_tags, filtered_user_tags, GroupTab, RoleTab, UserTab,
    GROUP_FILTER_CONTROLS, HISTORY_FILTER, POLICY_FILTER_CONTROLS, POLICY_TYPE_DROPDOWN,
    ROLE_FILTER_CONTROLS, USER_LAST_ACCESSED_FILTER_CONTROLS, USER_SIMPLE_FILTER_CONTROLS,
};

// ── Filter reset ──────────────────────────────────────────────────────────────

pub fn apply_filter_reset_users(app: &mut App) {
    if app.iam_state.current_user.is_some() {
        app.iam_state.user_tags.reset();
        app.iam_state.policies.reset();
    } else {
        app.iam_state.users.reset();
    }
}

pub fn apply_filter_reset_roles(app: &mut App) {
    if app.iam_state.current_role.is_some() {
        app.iam_state.tags.reset();
        app.iam_state.policies.reset();
    } else {
        app.iam_state.roles.reset();
    }
}

pub fn apply_filter_reset_groups(app: &mut App) {
    if app.iam_state.current_group.is_some() {
        app.iam_state.policies.reset();
        app.iam_state.group_users.reset();
    } else {
        app.iam_state.groups.reset();
    }
}

pub fn reset_on_service_select_roles(app: &mut App) {
    app.iam_state.roles.reset();
}

pub fn reset_on_service_select_users(app: &mut App) {
    app.iam_state.users.reset();
}

pub fn reset_on_service_select_groups(app: &mut App) {
    app.iam_state.groups.reset();
}

// ── Filter focus ──────────────────────────────────────────────────────────────

pub fn next_filter_focus_roles(app: &mut App) {
    if app.iam_state.current_role.is_some() {
        app.iam_state.policy_input_focus = app
            .iam_state
            .policy_input_focus
            .next(&POLICY_FILTER_CONTROLS);
    } else {
        app.iam_state.role_input_focus = app.iam_state.role_input_focus.next(&ROLE_FILTER_CONTROLS);
    }
}

pub fn prev_filter_focus_roles(app: &mut App) {
    if app.iam_state.current_role.is_some() {
        app.iam_state.policy_input_focus = app
            .iam_state
            .policy_input_focus
            .prev(&POLICY_FILTER_CONTROLS);
    } else {
        app.iam_state.role_input_focus = app.iam_state.role_input_focus.prev(&ROLE_FILTER_CONTROLS);
    }
}

pub fn next_filter_focus_users(app: &mut App) {
    if app.iam_state.user_tab == UserTab::Permissions {
        app.iam_state.policy_input_focus = app
            .iam_state
            .policy_input_focus
            .next(&POLICY_FILTER_CONTROLS);
    } else if app.iam_state.user_tab == UserTab::LastAccessed {
        app.iam_state.last_accessed_input_focus = app
            .iam_state
            .last_accessed_input_focus
            .next(&USER_LAST_ACCESSED_FILTER_CONTROLS);
    } else {
        app.iam_state.user_input_focus = app
            .iam_state
            .user_input_focus
            .next(&USER_SIMPLE_FILTER_CONTROLS);
    }
}

pub fn prev_filter_focus_users(app: &mut App) {
    if app.iam_state.user_tab == UserTab::Permissions {
        app.iam_state.policy_input_focus = app
            .iam_state
            .policy_input_focus
            .prev(&POLICY_FILTER_CONTROLS);
    } else if app.iam_state.user_tab == UserTab::LastAccessed {
        app.iam_state.last_accessed_input_focus = app
            .iam_state
            .last_accessed_input_focus
            .prev(&USER_LAST_ACCESSED_FILTER_CONTROLS);
    } else {
        app.iam_state.user_input_focus = app
            .iam_state
            .user_input_focus
            .prev(&USER_SIMPLE_FILTER_CONTROLS);
    }
}

pub fn next_filter_focus_groups(app: &mut App) {
    app.iam_state.group_input_focus = app.iam_state.group_input_focus.next(&GROUP_FILTER_CONTROLS);
}

pub fn prev_filter_focus_groups(app: &mut App) {
    app.iam_state.group_input_focus = app.iam_state.group_input_focus.prev(&GROUP_FILTER_CONTROLS);
}

pub fn is_pagination_focused_roles(app: &App) -> bool {
    app.iam_state.current_role.is_none() && app.iam_state.role_input_focus == InputFocus::Pagination
}

pub fn is_filter_focused_roles(app: &App) -> bool {
    app.iam_state.current_role.is_none() && app.iam_state.role_input_focus == InputFocus::Filter
}

pub fn is_filter_focused_policy_view(app: &App) -> bool {
    app.iam_state.policy_input_focus == InputFocus::Filter
}

pub fn is_pagination_focused_policy_view(app: &App) -> bool {
    app.iam_state.policy_input_focus == InputFocus::Pagination
}

// ── Navigation ────────────────────────────────────────────────────────────────

pub fn next_item_users(app: &mut App) {
    if app.iam_state.current_user.is_some() {
        if app.iam_state.user_tab == UserTab::Tags {
            let filtered = filtered_user_tags(app);
            if !filtered.is_empty() {
                app.iam_state.user_tags.next_item(filtered.len());
            }
        } else {
            let filtered = filtered_iam_policies(app);
            if !filtered.is_empty() {
                app.iam_state.policies.next_item(filtered.len());
            }
        }
    } else {
        let filtered = filtered_iam_users(app);
        if !filtered.is_empty() {
            app.iam_state.users.next_item(filtered.len());
        }
    }
}

pub fn next_item_roles(app: &mut App) {
    if app.iam_state.current_role.is_some() {
        match app.iam_state.role_tab {
            RoleTab::TrustRelationships => {
                let lines = app.iam_state.trust_policy_document.lines().count();
                let max_scroll = lines.saturating_sub(1);
                app.iam_state.trust_policy_scroll =
                    (app.iam_state.trust_policy_scroll + 1).min(max_scroll);
            }
            RoleTab::RevokeSessions => {
                app.iam_state.revoke_sessions_scroll =
                    (app.iam_state.revoke_sessions_scroll + 1).min(19);
            }
            RoleTab::Tags => {
                let filtered = filtered_iam_tags(app);
                if !filtered.is_empty() {
                    app.iam_state.tags.next_item(filtered.len());
                }
            }
            RoleTab::LastAccessed => {
                let filtered = filtered_last_accessed(app);
                if !filtered.is_empty() {
                    app.iam_state
                        .last_accessed_services
                        .next_item(filtered.len());
                }
            }
            _ => {
                let filtered = filtered_iam_policies(app);
                if !filtered.is_empty() {
                    app.iam_state.policies.next_item(filtered.len());
                }
            }
        }
    } else {
        let filtered = filtered_iam_roles(app);
        if !filtered.is_empty() {
            app.iam_state.roles.next_item(filtered.len());
        }
    }
}

pub fn next_item_groups(app: &mut App) {
    if app.iam_state.current_group.is_some() {
        match app.iam_state.group_tab {
            GroupTab::Users => {
                let filtered: Vec<_> = app
                    .iam_state
                    .group_users
                    .items
                    .iter()
                    .filter(|u| {
                        app.iam_state.group_users.filter.is_empty()
                            || u.user_name
                                .to_lowercase()
                                .contains(&app.iam_state.group_users.filter.to_lowercase())
                    })
                    .collect();
                if !filtered.is_empty() {
                    app.iam_state.group_users.next_item(filtered.len());
                }
            }
            GroupTab::Permissions => {
                let filtered = filtered_iam_policies(app);
                if !filtered.is_empty() {
                    app.iam_state.policies.next_item(filtered.len());
                }
            }
            GroupTab::AccessAdvisor => {
                let filtered = filtered_last_accessed(app);
                if !filtered.is_empty() {
                    app.iam_state
                        .last_accessed_services
                        .next_item(filtered.len());
                }
            }
        }
    } else {
        let filtered: Vec<_> = app
            .iam_state
            .groups
            .items
            .iter()
            .filter(|g| {
                app.iam_state.groups.filter.is_empty()
                    || g.group_name
                        .to_lowercase()
                        .contains(&app.iam_state.groups.filter.to_lowercase())
            })
            .collect();
        if !filtered.is_empty() {
            app.iam_state.groups.next_item(filtered.len());
        }
    }
}

pub fn prev_item_users(app: &mut App) {
    app.iam_state.users.prev_item();
}

pub fn prev_item_roles(app: &mut App) {
    if app.iam_state.current_role.is_some() {
        match app.iam_state.role_tab {
            RoleTab::TrustRelationships => {
                app.iam_state.trust_policy_scroll =
                    app.iam_state.trust_policy_scroll.saturating_sub(1);
            }
            RoleTab::RevokeSessions => {
                app.iam_state.revoke_sessions_scroll =
                    app.iam_state.revoke_sessions_scroll.saturating_sub(1);
            }
            RoleTab::Tags => app.iam_state.tags.prev_item(),
            RoleTab::LastAccessed => app.iam_state.last_accessed_services.prev_item(),
            _ => app.iam_state.policies.prev_item(),
        }
    } else {
        app.iam_state.roles.prev_item();
    }
}

pub fn prev_item_groups(app: &mut App) {
    if app.iam_state.current_group.is_some() {
        match app.iam_state.group_tab {
            GroupTab::Users => app.iam_state.group_users.prev_item(),
            GroupTab::Permissions => app.iam_state.policies.prev_item(),
            GroupTab::AccessAdvisor => app.iam_state.last_accessed_services.prev_item(),
        }
    } else {
        app.iam_state.groups.prev_item();
    }
}

pub fn page_down_filter_input_roles(app: &mut App) {
    let page_size = app.iam_state.roles.page_size.value();
    let filtered_count = filtered_iam_roles(app).len();
    app.iam_state.role_input_focus.handle_page_down(
        &mut app.iam_state.roles.selected,
        &mut app.iam_state.roles.scroll_offset,
        page_size,
        filtered_count,
    );
}

pub fn page_down_filter_input_policy_view(app: &mut App) {
    let page_size = app.iam_state.policies.page_size.value();
    let filtered_count = filtered_iam_policies(app).len();
    app.iam_state.policy_input_focus.handle_page_down(
        &mut app.iam_state.policies.selected,
        &mut app.iam_state.policies.scroll_offset,
        page_size,
        filtered_count,
    );
}

pub fn page_down_normal_users(app: &mut App) {
    let len = filtered_iam_users(app).len();
    crate::app::nav_page_down(&mut app.iam_state.users.selected, len, 10);
}

pub fn page_down_normal_roles(app: &mut App) {
    if app.iam_state.current_role.is_some() {
        let filtered = filtered_iam_policies(app);
        if !filtered.is_empty() {
            app.iam_state.policies.page_down(filtered.len());
        }
    } else {
        let filtered = filtered_iam_roles(app);
        app.iam_state.roles.page_down(filtered.len());
    }
}

pub fn page_down_normal_groups(app: &mut App) {
    if app.iam_state.current_group.is_some() {
        match app.iam_state.group_tab {
            GroupTab::Users => {
                let filtered: Vec<_> = app
                    .iam_state
                    .group_users
                    .items
                    .iter()
                    .filter(|u| {
                        app.iam_state.group_users.filter.is_empty()
                            || u.user_name
                                .to_lowercase()
                                .contains(&app.iam_state.group_users.filter.to_lowercase())
                    })
                    .collect();
                if !filtered.is_empty() {
                    app.iam_state.group_users.page_down(filtered.len());
                }
            }
            GroupTab::Permissions => {
                let filtered = filtered_iam_policies(app);
                if !filtered.is_empty() {
                    app.iam_state.policies.page_down(filtered.len());
                }
            }
            GroupTab::AccessAdvisor => {
                let filtered = filtered_last_accessed(app);
                if !filtered.is_empty() {
                    app.iam_state
                        .last_accessed_services
                        .page_down(filtered.len());
                }
            }
        }
    } else {
        let filtered: Vec<_> = app
            .iam_state
            .groups
            .items
            .iter()
            .filter(|g| {
                app.iam_state.groups.filter.is_empty()
                    || g.group_name
                        .to_lowercase()
                        .contains(&app.iam_state.groups.filter.to_lowercase())
            })
            .collect();
        if !filtered.is_empty() {
            app.iam_state.groups.page_down(filtered.len());
        }
    }
}

pub fn page_up_filter_input_roles(app: &mut App) {
    let page_size = app.iam_state.roles.page_size.value();
    app.iam_state.role_input_focus.handle_page_up(
        &mut app.iam_state.roles.selected,
        &mut app.iam_state.roles.scroll_offset,
        page_size,
    );
}

pub fn page_up_filter_input_policy_view(app: &mut App) {
    let page_size = app.iam_state.policies.page_size.value();
    app.iam_state.policy_input_focus.handle_page_up(
        &mut app.iam_state.policies.selected,
        &mut app.iam_state.policies.scroll_offset,
        page_size,
    );
}

pub fn page_up_normal_users(app: &mut App) {
    app.iam_state.users.page_up();
}

pub fn page_up_normal_roles(app: &mut App) {
    if app.iam_state.current_role.is_some() {
        app.iam_state.policies.page_up();
    } else {
        app.iam_state.roles.page_up();
    }
}

pub fn go_to_page_users(app: &mut App, page: usize) {
    let filtered_count = filtered_iam_users(app).len();
    app.iam_state.users.goto_page(page, filtered_count);
}

pub fn go_to_page_roles(app: &mut App, page: usize) {
    let filtered_count = filtered_iam_roles(app).len();
    app.iam_state.roles.goto_page(page, filtered_count);
}

pub fn scroll_up_policy_view(app: &mut App) {
    app.iam_state.policy_scroll = app.iam_state.policy_scroll.saturating_sub(10);
}

pub fn scroll_down_policy_view(app: &mut App) {
    let lines = app.iam_state.policy_document.lines().count();
    let max_scroll = lines.saturating_sub(1);
    app.iam_state.policy_scroll = (app.iam_state.policy_scroll + 10).min(max_scroll);
}

pub fn scroll_up_trust_policy(app: &mut App) {
    app.iam_state.trust_policy_scroll = app.iam_state.trust_policy_scroll.saturating_sub(10);
}

pub fn scroll_down_trust_policy(app: &mut App) {
    let lines = app.iam_state.trust_policy_document.lines().count();
    let max_scroll = lines.saturating_sub(1);
    app.iam_state.trust_policy_scroll = (app.iam_state.trust_policy_scroll + 10).min(max_scroll);
}

pub fn scroll_up_revoke_sessions(app: &mut App) {
    app.iam_state.revoke_sessions_scroll = app.iam_state.revoke_sessions_scroll.saturating_sub(10);
}

pub fn scroll_down_revoke_sessions(app: &mut App) {
    app.iam_state.revoke_sessions_scroll = (app.iam_state.revoke_sessions_scroll + 10).min(19);
}

pub fn scroll_down_policy_view_one(app: &mut App) {
    let lines = app.iam_state.policy_document.lines().count();
    let max_scroll = lines.saturating_sub(1);
    app.iam_state.policy_scroll = (app.iam_state.policy_scroll + 1).min(max_scroll);
}

pub fn scroll_up_policy_view_one(app: &mut App) {
    app.iam_state.policy_scroll = app.iam_state.policy_scroll.saturating_sub(1);
}

// ── FilterInput next_item (dropdown cycling) ──────────────────────────────────

pub fn next_item_filter_input_users(app: &mut App) {
    if app.iam_state.user_tab == UserTab::Permissions
        && app.iam_state.policy_input_focus == POLICY_TYPE_DROPDOWN
    {
        cycle_policy_type_next(app);
    } else if app.iam_state.user_tab == UserTab::LastAccessed
        && app.iam_state.last_accessed_input_focus == HISTORY_FILTER
    {
        app.iam_state.last_accessed_history_filter =
            app.iam_state.last_accessed_history_filter.next();
        app.iam_state.last_accessed_services.reset();
    }
}

pub fn prev_item_filter_input_users(app: &mut App) {
    if app.iam_state.user_tab == UserTab::Permissions
        && app.iam_state.policy_input_focus == POLICY_TYPE_DROPDOWN
    {
        cycle_policy_type_prev(app);
    } else if app.iam_state.user_tab == UserTab::LastAccessed
        && app.iam_state.last_accessed_input_focus == HISTORY_FILTER
    {
        app.iam_state.last_accessed_history_filter =
            app.iam_state.last_accessed_history_filter.prev();
        app.iam_state.last_accessed_services.reset();
    }
}

pub fn next_item_filter_input_roles(app: &mut App) {
    if app.iam_state.role_tab == RoleTab::Permissions
        && app.iam_state.policy_input_focus == POLICY_TYPE_DROPDOWN
    {
        cycle_policy_type_next(app);
    }
}

pub fn prev_item_filter_input_roles(app: &mut App) {
    if app.iam_state.role_tab == RoleTab::Permissions
        && app.iam_state.policy_input_focus == POLICY_TYPE_DROPDOWN
    {
        cycle_policy_type_prev(app);
    }
}

fn cycle_policy_type_next(app: &mut App) {
    let types = ["All types", "AWS managed", "Customer managed"];
    let current_idx = types
        .iter()
        .position(|&t| t == app.iam_state.policy_type_filter)
        .unwrap_or(0);
    app.iam_state.policy_type_filter = types[(current_idx + 1) % types.len()].to_string();
    app.iam_state.policies.reset();
}

fn cycle_policy_type_prev(app: &mut App) {
    let types = ["All types", "AWS managed", "Customer managed"];
    let current_idx = types
        .iter()
        .position(|&t| t == app.iam_state.policy_type_filter)
        .unwrap_or(0);
    let prev_idx = if current_idx == 0 {
        types.len() - 1
    } else {
        current_idx - 1
    };
    app.iam_state.policy_type_filter = types[prev_idx].to_string();
    app.iam_state.policies.reset();
}

// ── Expand / collapse ─────────────────────────────────────────────────────────

pub fn expand_row_users(app: &mut App) {
    if app.iam_state.current_user.is_some() {
        if app.iam_state.user_tab == UserTab::Tags {
            if app.iam_state.user_tags.expanded_item != Some(app.iam_state.user_tags.selected) {
                app.iam_state.user_tags.expanded_item = Some(app.iam_state.user_tags.selected);
            }
        } else if app.iam_state.policies.expanded_item != Some(app.iam_state.policies.selected) {
            app.iam_state.policies.toggle_expand();
        }
    } else if !app.iam_state.users.is_expanded() {
        app.iam_state.users.toggle_expand();
    }
}

pub fn expand_row_roles(app: &mut App) {
    if app.iam_state.current_role.is_some() {
        match app.iam_state.role_tab {
            RoleTab::Tags => {
                if !app.iam_state.tags.is_expanded() {
                    app.iam_state.tags.expand();
                }
            }
            RoleTab::LastAccessed => {
                if !app.iam_state.last_accessed_services.is_expanded() {
                    app.iam_state.last_accessed_services.expand();
                }
            }
            _ => {
                if !app.iam_state.policies.is_expanded() {
                    app.iam_state.policies.expand();
                }
            }
        }
    } else if !app.iam_state.roles.is_expanded() {
        app.iam_state.roles.expand();
    }
}

pub fn expand_row_groups(app: &mut App) {
    if app.iam_state.current_group.is_some() {
        match app.iam_state.group_tab {
            GroupTab::Users => {
                if !app.iam_state.group_users.is_expanded() {
                    app.iam_state.group_users.expand();
                }
            }
            GroupTab::Permissions => {
                if !app.iam_state.policies.is_expanded() {
                    app.iam_state.policies.expand();
                }
            }
            GroupTab::AccessAdvisor => {
                if !app.iam_state.last_accessed_services.is_expanded() {
                    app.iam_state.last_accessed_services.expand();
                }
            }
        }
    } else if !app.iam_state.groups.is_expanded() {
        app.iam_state.groups.expand();
    }
}

pub fn prev_pane_users(app: &mut App) {
    if app.iam_state.users.has_expanded_item() {
        app.iam_state.users.collapse();
    }
}

pub fn prev_pane_roles(app: &mut App) {
    if app.iam_state.current_role.is_some() {
        if app.iam_state.role_tab == RoleTab::Tags && app.iam_state.tags.has_expanded_item() {
            app.iam_state.tags.collapse();
        } else if app.iam_state.role_tab == RoleTab::LastAccessed
            && app.iam_state.last_accessed_services.expanded_item.is_some()
        {
            app.iam_state.last_accessed_services.collapse();
        } else if app.iam_state.policies.has_expanded_item() {
            app.iam_state.policies.collapse();
        }
    } else if app.iam_state.roles.has_expanded_item() {
        app.iam_state.roles.collapse();
    }
}

pub fn prev_pane_groups(app: &mut App) {
    if app.iam_state.current_group.is_some() {
        if app.iam_state.group_tab == GroupTab::Users
            && app.iam_state.group_users.has_expanded_item()
        {
            app.iam_state.group_users.collapse();
        } else if app.iam_state.group_tab == GroupTab::Permissions
            && app.iam_state.policies.has_expanded_item()
        {
            app.iam_state.policies.collapse();
        } else if app.iam_state.group_tab == GroupTab::AccessAdvisor
            && app.iam_state.last_accessed_services.expanded_item.is_some()
        {
            app.iam_state.last_accessed_services.collapse();
        }
    } else if app.iam_state.groups.has_expanded_item() {
        app.iam_state.groups.collapse();
    }
}

pub fn collapse_row_users(app: &mut App) {
    if app.iam_state.current_user.is_some() {
        match app.iam_state.user_tab {
            UserTab::Permissions => app.iam_state.policies.collapse(),
            UserTab::Groups => app.iam_state.user_group_memberships.collapse(),
            UserTab::Tags => app.iam_state.user_tags.collapse(),
            _ => {}
        }
    } else {
        app.iam_state.users.collapse();
    }
}

pub fn collapse_row_roles(app: &mut App) {
    if app.iam_state.current_role.is_some() {
        match app.iam_state.role_tab {
            RoleTab::Permissions => app.iam_state.policies.collapse(),
            RoleTab::Tags => app.iam_state.tags.collapse(),
            _ => {}
        }
    } else {
        app.iam_state.roles.collapse();
    }
}

pub fn collapse_row_groups(app: &mut App) {
    app.iam_state.groups.collapse();
}

// ── Go back ───────────────────────────────────────────────────────────────────

pub fn go_back_users(app: &mut App) {
    app.iam_state.current_user = None;
    app.iam_state.policies.items.clear();
    app.iam_state.policies.reset();
    app.update_current_tab_breadcrumb();
}

pub fn go_back_groups(app: &mut App) {
    app.iam_state.current_group = None;
    app.update_current_tab_breadcrumb();
}

pub fn go_back_roles(app: &mut App) {
    if app.view_mode == crate::app::ViewMode::PolicyView {
        app.view_mode = crate::app::ViewMode::Detail;
        app.iam_state.current_policy = None;
        app.iam_state.policy_document.clear();
        app.iam_state.policy_scroll = 0;
        app.update_current_tab_breadcrumb();
    } else if app.iam_state.current_role.is_some() {
        app.iam_state.current_role = None;
        app.iam_state.policies.items.clear();
        app.iam_state.policies.reset();
        app.update_current_tab_breadcrumb();
    }
}

// ── Detail tabs ───────────────────────────────────────────────────────────────

pub fn next_detail_tab_roles(app: &mut App) {
    app.iam_state.role_tab = app.iam_state.role_tab.next();
    if app.iam_state.role_tab == RoleTab::Tags {
        app.iam_state.tags.loading = true;
    }
}

pub fn prev_detail_tab_roles(app: &mut App) {
    app.iam_state.role_tab = app.iam_state.role_tab.prev();
}

pub fn next_detail_tab_users(app: &mut App) {
    app.iam_state.user_tab = app.iam_state.user_tab.next();
    if app.iam_state.user_tab == UserTab::Tags {
        app.iam_state.user_tags.loading = true;
    }
}

pub fn prev_detail_tab_users(app: &mut App) {
    app.iam_state.user_tab = app.iam_state.user_tab.prev();
}

pub fn next_detail_tab_groups(app: &mut App) {
    app.iam_state.group_tab = app.iam_state.group_tab.next();
}

pub fn prev_detail_tab_groups(app: &mut App) {
    app.iam_state.group_tab = app.iam_state.group_tab.prev();
}

// ── Select item ───────────────────────────────────────────────────────────────

pub fn select_item_users(app: &mut App) {
    if app.iam_state.current_user.is_some() {
        if app.iam_state.user_tab == UserTab::Permissions {
            let filtered = filtered_iam_policies(app);
            if let Some(policy) = app.iam_state.policies.get_selected(&filtered) {
                app.iam_state.current_policy = Some(policy.policy_name.clone());
                app.iam_state.policy_scroll = 0;
                app.view_mode = crate::app::ViewMode::PolicyView;
                app.iam_state.policies.loading = true;
                app.update_current_tab_breadcrumb();
            }
        }
    } else {
        let filtered = filtered_iam_users(app);
        if let Some(user) = app.iam_state.users.get_selected(&filtered) {
            app.iam_state.current_user = Some(user.user_name.clone());
            app.iam_state.user_tab = UserTab::Permissions;
            app.iam_state.policies.reset();
            app.update_current_tab_breadcrumb();
        }
    }
}

pub fn select_item_roles(app: &mut App) {
    if app.iam_state.current_role.is_some() {
        if app.iam_state.role_tab == RoleTab::Permissions {
            let filtered = filtered_iam_policies(app);
            if let Some(policy) = app.iam_state.policies.get_selected(&filtered) {
                app.iam_state.current_policy = Some(policy.policy_name.clone());
                app.iam_state.policy_scroll = 0;
                app.view_mode = crate::app::ViewMode::PolicyView;
                app.iam_state.policies.loading = true;
                app.update_current_tab_breadcrumb();
            }
        }
    } else {
        let filtered = filtered_iam_roles(app);
        if let Some(role) = app.iam_state.roles.get_selected(&filtered) {
            app.iam_state.current_role = Some(role.role_name.clone());
            app.iam_state.role_tab = RoleTab::Permissions;
            app.iam_state.policies.reset();
            app.update_current_tab_breadcrumb();
        }
    }
}

pub fn select_item_groups(app: &mut App) {
    if app.iam_state.current_group.is_none() {
        let filtered: Vec<_> = app
            .iam_state
            .groups
            .items
            .iter()
            .filter(|g| {
                app.iam_state.groups.filter.is_empty()
                    || g.group_name
                        .to_lowercase()
                        .contains(&app.iam_state.groups.filter.to_lowercase())
            })
            .collect();
        if let Some(group) = app.iam_state.groups.get_selected(&filtered) {
            app.iam_state.current_group = Some(group.group_name.clone());
            app.update_current_tab_breadcrumb();
        }
    }
}

// ── Yank ──────────────────────────────────────────────────────────────────────

pub fn yank_users(app: &App) {
    use crate::app::copy_to_clipboard;
    if app.iam_state.current_user.is_some() {
        if let Some(user_name) = &app.iam_state.current_user {
            if let Some(user) = app
                .iam_state
                .users
                .items
                .iter()
                .find(|u| u.user_name == *user_name)
            {
                copy_to_clipboard(&user.arn);
            }
        }
    } else {
        let filtered = filtered_iam_users(app);
        if let Some(user) = app.iam_state.users.get_selected(&filtered) {
            copy_to_clipboard(&user.arn);
        }
    }
}

pub fn yank_roles(app: &App) {
    use crate::app::copy_to_clipboard;
    if app.iam_state.current_role.is_some() {
        if let Some(role_name) = &app.iam_state.current_role {
            if let Some(role) = app
                .iam_state
                .roles
                .items
                .iter()
                .find(|r| r.role_name == *role_name)
            {
                copy_to_clipboard(&role.arn);
            }
        }
    } else {
        let filtered = filtered_iam_roles(app);
        if let Some(role) = app.iam_state.roles.get_selected(&filtered) {
            copy_to_clipboard(&role.arn);
        }
    }
}

pub fn yank_groups(app: &App) {
    use crate::app::copy_to_clipboard;
    use crate::iam;
    if let Some(group_name) = &app.iam_state.current_group {
        let arn = iam::format_arn(&app.config.account_id, "group", group_name);
        copy_to_clipboard(&arn);
    } else {
        let filtered: Vec<_> = app
            .iam_state
            .groups
            .items
            .iter()
            .filter(|g| {
                app.iam_state.groups.filter.is_empty()
                    || g.group_name
                        .to_lowercase()
                        .contains(&app.iam_state.groups.filter.to_lowercase())
            })
            .collect();
        if let Some(group) = app.iam_state.groups.get_selected(&filtered) {
            let arn = iam::format_arn(&app.config.account_id, "group", &group.group_name);
            copy_to_clipboard(&arn);
        }
    }
}

// ── Column preferences ────────────────────────────────────────────────────────

pub fn toggle_column_users(app: &mut App) {
    use crate::app::{toggle_iam_page_size_only, toggle_iam_preference_static};
    let idx = app.column_selector_index;
    if app.iam_state.current_user.is_some() {
        match app.iam_state.user_tab {
            UserTab::Permissions => {
                if idx > 0 && idx <= app.iam_policy_column_ids.len() {
                    if let Some(col) = app.iam_policy_column_ids.get(idx - 1) {
                        if let Some(pos) = app
                            .iam_policy_visible_column_ids
                            .iter()
                            .position(|c| c == col)
                        {
                            app.iam_policy_visible_column_ids.remove(pos);
                        } else {
                            app.iam_policy_visible_column_ids.push(col.clone());
                        }
                    }
                } else if idx == app.iam_policy_column_ids.len() + 3 {
                    app.iam_state.policies.page_size = PageSize::Ten;
                } else if idx == app.iam_policy_column_ids.len() + 4 {
                    app.iam_state.policies.page_size = PageSize::TwentyFive;
                } else if idx == app.iam_policy_column_ids.len() + 5 {
                    app.iam_state.policies.page_size = PageSize::Fifty;
                }
            }
            UserTab::Groups => toggle_iam_page_size_only(
                idx,
                5,
                &mut app.iam_state.user_group_memberships.page_size,
            ),
            UserTab::Tags => {
                toggle_iam_page_size_only(idx, 5, &mut app.iam_state.user_tags.page_size)
            }
            UserTab::LastAccessed => toggle_iam_page_size_only(
                idx,
                6,
                &mut app.iam_state.last_accessed_services.page_size,
            ),
            _ => {}
        }
    } else {
        toggle_iam_preference_static(
            idx,
            &app.iam_user_column_ids.clone(),
            &mut app.iam_user_visible_column_ids,
            &mut app.iam_state.users.page_size,
        );
    }
}

pub fn toggle_column_roles(app: &mut App) {
    use crate::app::{
        toggle_iam_page_size_only, toggle_iam_preference, toggle_iam_preference_static,
    };
    let idx = app.column_selector_index;
    if app.iam_state.current_role.is_some() {
        match app.iam_state.role_tab {
            RoleTab::Permissions => toggle_iam_preference(
                idx,
                &app.iam_policy_column_ids.clone(),
                &mut app.iam_policy_visible_column_ids,
                &mut app.iam_state.policies.page_size,
            ),
            RoleTab::LastAccessed => toggle_iam_page_size_only(
                idx,
                6,
                &mut app.iam_state.last_accessed_services.page_size,
            ),
            _ => {}
        }
    } else {
        toggle_iam_preference_static(
            idx,
            &app.iam_role_column_ids.clone(),
            &mut app.iam_role_visible_column_ids,
            &mut app.iam_state.roles.page_size,
        );
    }
}

pub fn toggle_column_groups(app: &mut App) {
    use crate::app::toggle_iam_preference;
    toggle_iam_preference(
        app.column_selector_index,
        &app.iam_group_column_ids.clone(),
        &mut app.iam_group_visible_column_ids,
        &mut app.iam_state.groups.page_size,
    );
}

// ── Prefs cycling ─────────────────────────────────────────────────────────────

pub fn next_preferences_users(app: &mut App) {
    if app.iam_state.current_user.is_some() {
        match app.iam_state.user_tab {
            UserTab::Permissions => {
                let page_size_idx = app.iam_policy_column_ids.len() + 2;
                if app.column_selector_index < page_size_idx {
                    app.column_selector_index = page_size_idx;
                } else {
                    app.column_selector_index = 0;
                }
            }
            UserTab::Groups | UserTab::Tags => {
                if app.column_selector_index < 4 {
                    app.column_selector_index = 4;
                } else {
                    app.column_selector_index = 0;
                }
            }
            UserTab::LastAccessed => {
                if app.column_selector_index < 5 {
                    app.column_selector_index = 5;
                } else {
                    app.column_selector_index = 0;
                }
            }
            _ => {}
        }
    } else {
        let page_size_idx = app.iam_user_column_ids.len() + 2;
        if app.column_selector_index < page_size_idx {
            app.column_selector_index = page_size_idx;
        } else {
            app.column_selector_index = 0;
        }
    }
}

pub fn prev_preferences_users(app: &mut App) {
    if app.iam_state.current_user.is_some() {
        match app.iam_state.user_tab {
            UserTab::Permissions => {
                let page_size_idx = app.iam_policy_column_ids.len() + 2;
                if app.column_selector_index >= page_size_idx {
                    app.column_selector_index = 0;
                } else {
                    app.column_selector_index = page_size_idx;
                }
            }
            UserTab::Groups | UserTab::Tags => {
                if app.column_selector_index >= 4 {
                    app.column_selector_index = 0;
                } else {
                    app.column_selector_index = 4;
                }
            }
            UserTab::LastAccessed => {
                if app.column_selector_index >= 5 {
                    app.column_selector_index = 0;
                } else {
                    app.column_selector_index = 5;
                }
            }
            _ => {}
        }
    } else {
        let page_size_idx = app.iam_user_column_ids.len() + 2;
        if app.column_selector_index >= page_size_idx {
            app.column_selector_index = 0;
        } else {
            app.column_selector_index = page_size_idx;
        }
    }
}

pub fn next_preferences_roles(app: &mut App) {
    if app.iam_state.current_role.is_some() {
        match app.iam_state.role_tab {
            RoleTab::Permissions => {
                let page_size_idx = app.iam_policy_column_ids.len() + 2;
                if app.column_selector_index < page_size_idx {
                    app.column_selector_index = page_size_idx;
                } else {
                    app.column_selector_index = 0;
                }
            }
            RoleTab::Tags => {
                if app.column_selector_index < 4 {
                    app.column_selector_index = 4;
                } else {
                    app.column_selector_index = 0;
                }
            }
            RoleTab::LastAccessed => {
                if app.column_selector_index < 5 {
                    app.column_selector_index = 5;
                } else {
                    app.column_selector_index = 0;
                }
            }
            _ => {}
        }
    } else {
        let page_size_idx = app.iam_role_column_ids.len() + 2;
        if app.column_selector_index < page_size_idx {
            app.column_selector_index = page_size_idx;
        } else {
            app.column_selector_index = 0;
        }
    }
}

pub fn prev_preferences_roles(app: &mut App) {
    if app.iam_state.current_role.is_some() {
        match app.iam_state.role_tab {
            RoleTab::Permissions => {
                let page_size_idx = app.iam_policy_column_ids.len() + 2;
                if app.column_selector_index >= page_size_idx {
                    app.column_selector_index = 0;
                } else {
                    app.column_selector_index = page_size_idx;
                }
            }
            RoleTab::Tags => {
                if app.column_selector_index >= 4 {
                    app.column_selector_index = 0;
                } else {
                    app.column_selector_index = 4;
                }
            }
            RoleTab::LastAccessed => {
                if app.column_selector_index >= 5 {
                    app.column_selector_index = 0;
                } else {
                    app.column_selector_index = 5;
                }
            }
            _ => {}
        }
    } else {
        let page_size_idx = app.iam_role_column_ids.len() + 2;
        if app.column_selector_index >= page_size_idx {
            app.column_selector_index = 0;
        } else {
            app.column_selector_index = page_size_idx;
        }
    }
}

pub fn next_preferences_groups(app: &mut App) {
    let page_size_idx = app.iam_group_column_ids.len() + 2;
    if app.column_selector_index < page_size_idx {
        app.column_selector_index = page_size_idx;
    } else {
        app.column_selector_index = 0;
    }
}

pub fn prev_preferences_groups(app: &mut App) {
    let page_size_idx = app.iam_group_column_ids.len() + 2;
    if app.column_selector_index >= page_size_idx {
        app.column_selector_index = 0;
    } else {
        app.column_selector_index = page_size_idx;
    }
}

pub fn column_selector_max_users(app: &App) -> usize {
    if app.iam_state.current_user.is_some() {
        app.iam_policy_column_ids.len() + 5
    } else {
        app.iam_user_column_ids.len() + 5
    }
}

pub fn column_count_users(app: &App) -> usize {
    if app.iam_state.current_user.is_some() {
        app.iam_policy_column_ids.len()
    } else {
        app.iam_user_column_ids.len()
    }
}

pub fn column_selector_max_roles(app: &App) -> usize {
    if app.iam_state.current_role.is_some() {
        app.iam_policy_column_ids.len() + 5
    } else {
        app.iam_role_column_ids.len() + 5
    }
}

pub fn column_count_roles(app: &App) -> usize {
    if app.iam_state.current_role.is_some() {
        app.iam_policy_column_ids.len()
    } else {
        app.iam_role_column_ids.len()
    }
}

pub fn column_selector_max_groups(app: &App) -> usize {
    app.iam_group_column_ids.len() + 5
}

pub fn column_count_groups(app: &App) -> usize {
    app.iam_group_column_ids.len()
}

// ── Breadcrumb / console URL ──────────────────────────────────────────────────

pub fn breadcrumb_users() -> Vec<String> {
    vec!["IAM".to_string(), "Users".to_string()]
}

pub fn breadcrumb_roles(app: &App) -> Vec<String> {
    let mut parts = vec!["IAM".to_string(), "Roles".to_string()];
    if let Some(role_name) = &app.iam_state.current_role {
        parts.push(role_name.clone());
        if let Some(policy_name) = &app.iam_state.current_policy {
            parts.push(policy_name.clone());
        }
    }
    parts
}

pub fn breadcrumb_groups(app: &App) -> Vec<String> {
    let mut parts = vec!["IAM".to_string(), "User Groups".to_string()];
    if let Some(group_name) = &app.iam_state.current_group {
        parts.push(group_name.clone());
    }
    parts
}

pub fn console_url_users(app: &App) -> String {
    use crate::iam;
    if let Some(user_name) = &app.iam_state.current_user {
        let section = match app.iam_state.user_tab {
            UserTab::Permissions => "permissions",
            UserTab::Groups => "groups",
            UserTab::Tags => "tags",
            UserTab::SecurityCredentials => "security_credentials",
            UserTab::LastAccessed => "access_advisor",
        };
        iam::console_url_user_detail(&app.config.region, user_name, section)
    } else {
        iam::console_url_users(&app.config.region)
    }
}

pub fn console_url_roles(app: &App) -> String {
    use crate::iam;
    if let Some(policy_name) = &app.iam_state.current_policy {
        if let Some(role_name) = &app.iam_state.current_role {
            return iam::console_url_role_policy(&app.config.region, role_name, policy_name);
        }
    }
    if let Some(role_name) = &app.iam_state.current_role {
        let section = match app.iam_state.role_tab {
            RoleTab::Permissions => "permissions",
            RoleTab::TrustRelationships => "trust_relationships",
            RoleTab::Tags => "tags",
            RoleTab::LastAccessed => "access_advisor",
            RoleTab::RevokeSessions => "revoke_sessions",
        };
        iam::console_url_role_detail(&app.config.region, role_name, section)
    } else {
        iam::console_url_roles(&app.config.region)
    }
}

pub fn console_url_groups(app: &App) -> String {
    use crate::iam;
    iam::console_url_groups(&app.config.region)
}
