use crate::app::{App, ViewMode};
use crate::common::{
    filter_by_field, filter_by_fields, format_duration_seconds, render_pagination_text, CyclicEnum,
    InputFocus, SortDirection,
};
use crate::iam::{
    GroupColumn, GroupUser, GroupUserColumn, IamGroup, IamRole, IamUser, LastAccessedService,
    Policy, PolicyColumn, RoleColumn, RoleTag, UserColumn, UserGroup, UserTag,
};
use crate::keymap::Mode;
use crate::table::TableState;
use crate::ui::filter::{render_simple_filter, SimpleFilterConfig};
use crate::ui::table::{
    expanded_from_columns, plain_expanded_content, render_table, Column, TableConfig,
};
use crate::ui::{
    block_height_for, calculate_dynamic_height, format_title, labeled_field,
    render_fields_with_dynamic_columns, render_json_highlighted, render_last_accessed_section,
    render_permissions_section, render_search_filter, render_summary, render_tabs,
    render_tags_section, titled_block, vertical,
};
use ratatui::{prelude::*, widgets::*};

pub const POLICY_TYPE_DROPDOWN: InputFocus = InputFocus::Dropdown("PolicyType");
pub const HISTORY_FILTER: InputFocus = InputFocus::Dropdown("HistoryFilter");
pub const POLICY_FILTER_CONTROLS: [InputFocus; 3] = [
    InputFocus::Filter,
    POLICY_TYPE_DROPDOWN,
    InputFocus::Pagination,
];

pub const ROLE_FILTER_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

pub const USER_SIMPLE_FILTER_CONTROLS: [InputFocus; 2] =
    [InputFocus::Filter, InputFocus::Pagination];

pub const USER_LAST_ACCESSED_FILTER_CONTROLS: [InputFocus; 3] =
    [InputFocus::Filter, HISTORY_FILTER, InputFocus::Pagination];

pub const GROUP_FILTER_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UserTab {
    Permissions,
    Groups,
    Tags,
    SecurityCredentials,
    LastAccessed,
}

impl CyclicEnum for UserTab {
    const ALL: &'static [Self] = &[
        Self::Permissions,
        Self::Groups,
        Self::Tags,
        Self::SecurityCredentials,
        Self::LastAccessed,
    ];
}

impl UserTab {
    pub fn name(&self) -> &'static str {
        match self {
            UserTab::Permissions => "Permissions",
            UserTab::Groups => "Groups",
            UserTab::Tags => "Tags",
            UserTab::SecurityCredentials => "Security Credentials",
            UserTab::LastAccessed => "Last Accessed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RoleTab {
    Permissions,
    TrustRelationships,
    Tags,
    LastAccessed,
    RevokeSessions,
}

impl CyclicEnum for RoleTab {
    const ALL: &'static [Self] = &[
        Self::Permissions,
        Self::TrustRelationships,
        Self::Tags,
        Self::LastAccessed,
        Self::RevokeSessions,
    ];
}

impl RoleTab {
    pub fn name(&self) -> &'static str {
        match self {
            RoleTab::Permissions => "Permissions",
            RoleTab::TrustRelationships => "Trust relationships",
            RoleTab::Tags => "Tags",
            RoleTab::LastAccessed => "Last Accessed",
            RoleTab::RevokeSessions => "Revoke sessions",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GroupTab {
    Users,
    Permissions,
    AccessAdvisor,
}

impl CyclicEnum for GroupTab {
    const ALL: &'static [Self] = &[Self::Users, Self::Permissions, Self::AccessAdvisor];
}

impl GroupTab {
    pub fn name(&self) -> &'static str {
        match self {
            GroupTab::Users => "Users",
            GroupTab::Permissions => "Permissions",
            GroupTab::AccessAdvisor => "Access Advisor",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AccessHistoryFilter {
    NoFilter,
    ServicesAccessed,
    ServicesNotAccessed,
}

impl CyclicEnum for AccessHistoryFilter {
    const ALL: &'static [Self] = &[
        Self::NoFilter,
        Self::ServicesAccessed,
        Self::ServicesNotAccessed,
    ];
}

impl AccessHistoryFilter {
    pub fn name(&self) -> &'static str {
        match self {
            AccessHistoryFilter::NoFilter => "No filter",
            AccessHistoryFilter::ServicesAccessed => "Services accessed",
            AccessHistoryFilter::ServicesNotAccessed => "Services not accessed",
        }
    }
}

pub struct State {
    pub users: TableState<IamUser>,
    pub current_user: Option<String>,
    pub user_tab: UserTab,
    pub user_input_focus: InputFocus,
    pub roles: TableState<IamRole>,
    pub current_role: Option<String>,
    pub role_tab: RoleTab,
    pub role_input_focus: InputFocus,
    pub groups: TableState<IamGroup>,
    pub current_group: Option<String>,
    pub group_tab: GroupTab,
    pub group_input_focus: InputFocus,
    pub policies: TableState<Policy>,
    pub policy_type_filter: String,
    pub policy_input_focus: InputFocus,
    pub current_policy: Option<String>,
    pub policy_document: String,
    pub policy_scroll: usize,
    pub trust_policy_document: String,
    pub trust_policy_scroll: usize,
    pub tags: TableState<RoleTag>,
    pub user_tags: TableState<UserTag>,
    pub user_group_memberships: TableState<UserGroup>,
    pub group_users: TableState<GroupUser>,
    pub last_accessed_services: TableState<LastAccessedService>,
    pub last_accessed_filter: String,
    pub last_accessed_history_filter: AccessHistoryFilter,
    pub last_accessed_input_focus: InputFocus,
    pub revoke_sessions_scroll: usize,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            users: TableState::new(),
            current_user: None,
            user_tab: UserTab::Permissions,
            user_input_focus: InputFocus::Filter,
            roles: TableState::new(),
            current_role: None,
            role_tab: RoleTab::Permissions,
            role_input_focus: InputFocus::Filter,
            groups: TableState::new(),
            current_group: None,
            group_tab: GroupTab::Users,
            group_input_focus: InputFocus::Filter,
            policies: TableState::new(),
            policy_type_filter: "All types".to_string(),
            policy_input_focus: InputFocus::Filter,
            current_policy: None,
            policy_document: String::new(),
            policy_scroll: 0,
            trust_policy_document: String::new(),
            trust_policy_scroll: 0,
            tags: TableState::new(),
            user_tags: TableState::new(),
            user_group_memberships: TableState::new(),
            group_users: TableState::new(),
            last_accessed_services: TableState::new(),
            last_accessed_filter: String::new(),
            last_accessed_history_filter: AccessHistoryFilter::NoFilter,
            last_accessed_input_focus: InputFocus::Filter,
            revoke_sessions_scroll: 0,
        }
    }
}

pub fn render_users(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    if app.iam_state.current_user.is_some() {
        render_user_detail(frame, app, area);
    } else {
        render_user_list(frame, app, area);
    }
}

pub fn render_user_list(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = vertical(
        [
            Constraint::Length(3), // Filter
            Constraint::Min(0),    // Table
        ],
        area,
    );

    // Filter
    let filtered_users = filtered_iam_users(app);
    let filtered_count = filtered_users.len();
    let page_size = app.iam_state.users.page_size.value();
    render_search_filter(
        frame,
        chunks[0],
        &app.iam_state.users.filter,
        app.mode == Mode::FilterInput,
        app.iam_state.users.selected,
        filtered_count,
        page_size,
    );

    // Table
    let scroll_offset = app.iam_state.users.scroll_offset;
    let page_users: Vec<_> = filtered_users
        .iter()
        .skip(scroll_offset)
        .take(page_size)
        .collect();

    let columns: Vec<Box<dyn Column<&IamUser>>> = app
        .iam_user_visible_column_ids
        .iter()
        .filter_map(|col_id| {
            UserColumn::from_id(col_id).map(|col| Box::new(col) as Box<dyn Column<&IamUser>>)
        })
        .collect();

    let expanded_index = app.iam_state.users.expanded_item.and_then(|idx| {
        if idx >= scroll_offset && idx < scroll_offset + page_size {
            Some(idx - scroll_offset)
        } else {
            None
        }
    });

    let config = TableConfig {
        items: page_users,
        selected_index: app.iam_state.users.selected - app.iam_state.users.scroll_offset,
        expanded_index,
        columns: &columns,
        sort_column: "User name",
        sort_direction: SortDirection::Asc,
        title: format_title(&format!("Users ({})", filtered_count)),
        area: chunks[1],
        get_expanded_content: Some(Box::new(|user: &&IamUser| {
            expanded_from_columns(&columns, user)
        })),
        is_active: app.mode != Mode::FilterInput,
    };

    render_table(frame, config);
}

pub fn render_roles(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    if app.view_mode == ViewMode::PolicyView {
        render_policy_view(frame, app, area);
    } else if app.iam_state.current_role.is_some() {
        render_role_detail(frame, app, area);
    } else {
        render_role_list(frame, app, area);
    }
}

pub fn render_user_groups(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    if app.iam_state.current_group.is_some() {
        render_group_detail(frame, app, area);
    } else {
        render_group_list(frame, app, area);
    }
}

pub fn render_group_detail(frame: &mut Frame, app: &App, area: Rect) {
    // Calculate summary height
    let summary_height = if app.iam_state.current_group.is_some() {
        block_height_for(3) // 3 fields
    } else {
        0
    };

    let chunks = vertical(
        [
            Constraint::Length(summary_height), // Summary
            Constraint::Length(1),              // Tabs
            Constraint::Min(0),                 // Content
        ],
        area,
    );

    if let Some(group_name) = &app.iam_state.current_group {
        // Summary section
        if let Some(group) = app
            .iam_state
            .groups
            .items
            .iter()
            .find(|g| g.group_name == *group_name)
        {
            render_summary(
                frame,
                chunks[0],
                " Summary ",
                &[
                    ("User group name: ", group.group_name.clone()),
                    ("Creation time: ", group.creation_time.clone()),
                    (
                        "ARN: ",
                        format!(
                            "arn:aws:iam::{}:group/{}",
                            app.config.account_id, group.group_name
                        ),
                    ),
                ],
            );
        }

        // Tabs
        let tabs: Vec<(&str, GroupTab)> =
            GroupTab::ALL.iter().map(|tab| (tab.name(), *tab)).collect();
        render_tabs(frame, chunks[1], &tabs, &app.iam_state.group_tab);

        // Content area based on selected tab
        match app.iam_state.group_tab {
            GroupTab::Users => {
                render_group_users_tab(frame, app, chunks[2]);
            }
            GroupTab::Permissions => {
                render_permissions_section(
                    frame,
                    chunks[2],
                    "You can attach up to 10 managed policies.",
                    |f, area| render_policies_table(f, app, area),
                );
            }
            GroupTab::AccessAdvisor => {
                render_group_access_advisor_tab(frame, app, chunks[2]);
            }
        }
    }
}

pub fn render_group_users_tab(frame: &mut Frame, app: &App, area: Rect) {
    render_group_users_table(frame, app, area);
}

pub fn render_group_users_table(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = vertical(
        [
            Constraint::Length(3), // Filter with pagination
            Constraint::Min(0),    // Table
        ],
        area,
    );

    let page_size = app.iam_state.group_users.page_size.value();
    let filtered_users: Vec<_> = app
        .iam_state
        .group_users
        .items
        .iter()
        .filter(|u| {
            if app.iam_state.group_users.filter.is_empty() {
                true
            } else {
                u.user_name
                    .to_lowercase()
                    .contains(&app.iam_state.group_users.filter.to_lowercase())
            }
        })
        .collect();

    let filtered_count = filtered_users.len();
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = app.iam_state.group_users.selected / page_size;
    let pagination = render_pagination_text(current_page, total_pages);

    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &app.iam_state.group_users.filter,
            placeholder: "Search",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: true,
            is_pagination_focused: false,
        },
    );

    let scroll_offset = app.iam_state.group_users.scroll_offset;
    let page_users: Vec<&GroupUser> = filtered_users
        .into_iter()
        .skip(scroll_offset)
        .take(page_size)
        .collect();

    let columns: Vec<Box<dyn Column<GroupUser>>> = vec![
        Box::new(GroupUserColumn::UserName),
        Box::new(GroupUserColumn::Groups),
        Box::new(GroupUserColumn::LastActivity),
        Box::new(GroupUserColumn::CreationTime),
    ];

    let expanded_index = app.iam_state.group_users.expanded_item.and_then(|idx| {
        if idx >= scroll_offset && idx < scroll_offset + page_size {
            Some(idx - scroll_offset)
        } else {
            None
        }
    });

    let config = TableConfig {
        items: page_users,
        selected_index: app.iam_state.group_users.selected - scroll_offset,
        expanded_index,
        columns: &columns,
        sort_column: "User name",
        sort_direction: SortDirection::Asc,
        title: format_title(&format!(
            "Users ({})",
            app.iam_state.group_users.items.len()
        )),
        area: chunks[1],
        is_active: app.mode != Mode::ColumnSelector,
        get_expanded_content: Some(Box::new(|user: &GroupUser| {
            plain_expanded_content(format!(
                "User name: {}\nGroups: {}\nLast activity: {}\nCreation time: {}",
                user.user_name, user.groups, user.last_activity, user.creation_time
            ))
        })),
    };

    render_table(frame, config);
}

pub fn render_group_access_advisor_tab(frame: &mut Frame, app: &App, area: Rect) {
    render_last_accessed_table(frame, app, area);
}

pub fn render_group_list(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = vertical(
        [
            Constraint::Length(3), // Filter
            Constraint::Min(0),    // Table
        ],
        area,
    );

    let page_size = app.iam_state.groups.page_size.value();
    let filtered_groups: Vec<_> = app
        .iam_state
        .groups
        .items
        .iter()
        .filter(|g| {
            if app.iam_state.groups.filter.is_empty() {
                true
            } else {
                g.group_name
                    .to_lowercase()
                    .contains(&app.iam_state.groups.filter.to_lowercase())
            }
        })
        .collect();

    let filtered_count = filtered_groups.len();
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = app.iam_state.groups.selected / page_size;
    let pagination = render_pagination_text(current_page, total_pages);

    use crate::ui::filter::{render_simple_filter, SimpleFilterConfig};
    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &app.iam_state.groups.filter,
            placeholder: "Search",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.iam_state.group_input_focus == InputFocus::Filter,
            is_pagination_focused: app.iam_state.group_input_focus == InputFocus::Pagination,
        },
    );

    let scroll_offset = app.iam_state.groups.scroll_offset;
    let page_groups: Vec<&IamGroup> = filtered_groups
        .into_iter()
        .skip(scroll_offset)
        .take(page_size)
        .collect();

    let mut columns: Vec<Box<dyn Column<IamGroup>>> = vec![];
    for col_name in &app.iam_group_visible_column_ids {
        let column = match col_name.as_str() {
            "Group name" => Some(GroupColumn::GroupName),
            "Path" => Some(GroupColumn::Path),
            "Users" => Some(GroupColumn::Users),
            "Permissions" => Some(GroupColumn::Permissions),
            "Creation time" => Some(GroupColumn::CreationTime),
            _ => None,
        };
        if let Some(c) = column {
            columns.push(Box::new(c));
        }
    }

    let expanded_index = app.iam_state.groups.expanded_item.and_then(|idx| {
        if idx >= scroll_offset && idx < scroll_offset + page_size {
            Some(idx - scroll_offset)
        } else {
            None
        }
    });

    let config = TableConfig {
        items: page_groups,
        selected_index: app.iam_state.groups.selected - scroll_offset,
        expanded_index,
        columns: &columns,
        sort_column: "Group name",
        sort_direction: SortDirection::Asc,
        title: format_title(&format!(
            "User groups ({})",
            app.iam_state.groups.items.len()
        )),
        area: chunks[1],
        is_active: app.mode != Mode::ColumnSelector,
        get_expanded_content: Some(Box::new(|group: &IamGroup| {
            expanded_from_columns(&columns, group)
        })),
    };

    render_table(frame, config);
}

pub fn render_role_list(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = vertical(
        [
            Constraint::Length(3), // Filter
            Constraint::Min(0),    // Table
        ],
        area,
    );

    // Filter with CFN pattern
    let page_size = app.iam_state.roles.page_size.value();
    let filtered_count = filtered_iam_roles(app).len();
    let total_pages = if filtered_count == 0 {
        1
    } else {
        filtered_count.div_ceil(page_size)
    };
    let current_page = if filtered_count == 0 {
        0
    } else {
        app.iam_state.roles.selected / page_size
    };
    let pagination = render_pagination_text(current_page, total_pages);

    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &app.iam_state.roles.filter,
            placeholder: "Search",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.iam_state.role_input_focus == InputFocus::Filter,
            is_pagination_focused: app.iam_state.role_input_focus == InputFocus::Pagination,
        },
    );

    // Table
    let scroll_offset = app.iam_state.roles.scroll_offset;
    let page_roles: Vec<_> = filtered_iam_roles(app)
        .into_iter()
        .skip(scroll_offset)
        .take(page_size)
        .collect();

    let mut columns: Vec<Box<dyn Column<IamRole>>> = vec![];
    for col_id in &app.iam_role_visible_column_ids {
        if let Some(col) = RoleColumn::from_id(col_id) {
            columns.push(Box::new(col));
        }
    }

    let expanded_index = app.iam_state.roles.expanded_item.and_then(|idx| {
        if idx >= scroll_offset && idx < scroll_offset + page_size {
            Some(idx - scroll_offset)
        } else {
            None
        }
    });

    let config = TableConfig {
        items: page_roles,
        selected_index: app.iam_state.roles.selected % page_size,
        expanded_index,
        columns: &columns,
        sort_column: "Role name",
        sort_direction: SortDirection::Asc,
        title: format_title(&format!("Roles ({})", filtered_count)),
        area: chunks[1],
        is_active: app.mode != Mode::ColumnSelector,
        get_expanded_content: Some(Box::new(|role: &IamRole| {
            expanded_from_columns(&columns, role)
        })),
    };

    render_table(frame, config);
}

pub fn render_role_detail(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    // Calculate summary height
    let summary_height = if app.iam_state.current_role.is_some() {
        block_height_for(5) // 5 fields
    } else {
        0
    };

    let chunks = vertical(
        [
            Constraint::Length(summary_height), // Summary
            Constraint::Length(1),              // Tabs
            Constraint::Min(0),                 // Content
        ],
        area,
    );

    // Summary section
    if let Some(role_name) = &app.iam_state.current_role {
        if let Some(role) = app
            .iam_state
            .roles
            .items
            .iter()
            .find(|r| r.role_name == *role_name)
        {
            let formatted_duration = role
                .max_session_duration
                .map(format_duration_seconds)
                .unwrap_or_default();

            render_summary(
                frame,
                chunks[0],
                " Summary ",
                &[
                    ("ARN: ", role.arn.clone()),
                    ("Trusted entities: ", role.trusted_entities.clone()),
                    ("Max session duration: ", formatted_duration),
                    ("Created: ", role.creation_time.clone()),
                    ("Description: ", role.description.clone()),
                ],
            );
        }
    }

    // Tabs
    render_tabs(
        frame,
        chunks[1],
        &RoleTab::ALL
            .iter()
            .map(|tab| (tab.name(), *tab))
            .collect::<Vec<_>>(),
        &app.iam_state.role_tab,
    );

    // Content based on selected tab
    match app.iam_state.role_tab {
        RoleTab::Permissions => {
            render_permissions_section(
                frame,
                chunks[2],
                "You can attach up to 10 managed policies.",
                |f, area| render_policies_table(f, app, area),
            );
        }
        RoleTab::TrustRelationships => {
            render_json_highlighted(
                frame,
                chunks[2],
                &app.iam_state.trust_policy_document,
                app.iam_state.trust_policy_scroll,
                " Trust Policy ",
                true,
            );
        }
        RoleTab::Tags => {
            render_tags_section(frame, chunks[2], |f, area| render_tags_table(f, app, area));
        }
        RoleTab::RevokeSessions => {
            let example_policy = r#"{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Deny",
            "Action": [
                "*"
            ],
            "Resource": [
                "*"
            ],
            "Condition": {
                "DateLessThan": {
                    "aws:TokenIssueTime": "[policy creation time]"
                }
            }
        }
    ]
}"#;

            render_json_highlighted(
                frame,
                chunks[2],
                example_policy,
                app.iam_state.revoke_sessions_scroll,
                " Example Policy ",
                true,
            );
        }
        RoleTab::LastAccessed => {
            render_last_accessed_section(
                frame,
                chunks[2],
                "Last accessed information shows the services that this role can access and when those services were last accessed. Review this data to remove unused permissions.",
                "IAM reports activity for services and management actions. Learn more about action last accessed information. To see actions, choose the appropriate service name from the list.",
                |f, area| render_last_accessed_table(f, app, area),
            );
        }
    }
}

pub fn render_user_detail(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    // Build summary lines first to calculate height
    let summary_lines = if let Some(user_name) = &app.iam_state.current_user {
        if let Some(user) = app
            .iam_state
            .users
            .items
            .iter()
            .find(|u| u.user_name == *user_name)
        {
            vec![
                labeled_field("ARN", &user.arn),
                labeled_field("Console access", &user.console_access),
                labeled_field("Access key", &user.access_key_id),
                labeled_field("Created", &user.creation_time),
                labeled_field("Last console sign-in", &user.console_last_sign_in),
            ]
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    // Calculate summary height
    let summary_height = if summary_lines.is_empty() {
        0
    } else {
        calculate_dynamic_height(&summary_lines, area.width.saturating_sub(4)) + 2
    };

    let chunks = vertical(
        [
            Constraint::Length(summary_height), // Summary
            Constraint::Length(1),              // Tabs
            Constraint::Min(0),                 // Content
        ],
        area,
    );

    // Summary section
    if !summary_lines.is_empty() {
        let summary_block = titled_block("Summary");

        let summary_inner = summary_block.inner(chunks[0]);
        frame.render_widget(summary_block, chunks[0]);

        render_fields_with_dynamic_columns(frame, summary_inner, summary_lines);
    }

    // Tabs
    render_tabs(
        frame,
        chunks[1],
        &UserTab::ALL
            .iter()
            .map(|tab| (tab.name(), *tab))
            .collect::<Vec<_>>(),
        &app.iam_state.user_tab,
    );

    // Content area - Permissions tab
    if app.iam_state.user_tab == UserTab::Permissions {
        render_permissions_tab(frame, app, chunks[2]);
    } else if app.iam_state.user_tab == UserTab::Groups {
        render_user_groups_tab(frame, app, chunks[2]);
    } else if app.iam_state.user_tab == UserTab::Tags {
        render_tags_section(frame, chunks[2], |f, area| {
            render_user_tags_table(f, app, area)
        });
    } else if app.iam_state.user_tab == UserTab::SecurityCredentials {
        let block = titled_block("Security Credentials");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let text = Paragraph::new("Security credentials information is not yet implemented.");
        frame.render_widget(text, inner);
    } else if app.iam_state.user_tab == UserTab::LastAccessed {
        render_user_last_accessed_tab(frame, app, chunks[2]);
    }
}

pub fn render_permissions_tab(frame: &mut Frame, app: &App, area: Rect) {
    render_permissions_section(
        frame,
        area,
        "Permissions are defined by policies attached to the user directly or through groups.",
        |f, area| render_policies_table(f, app, area),
    );
}

pub fn render_policy_view(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    render_json_highlighted(
        frame,
        area,
        &app.iam_state.policy_document,
        app.iam_state.policy_scroll,
        " Policy Document ",
        true,
    );
}

pub fn render_policies_table(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = vertical(
        [
            Constraint::Length(3), // Filter bar
            Constraint::Min(0),    // Table
        ],
        area,
    );

    // Filter policies
    let page_size = app.iam_state.policies.page_size.value();
    let filtered_policies: Vec<_> = app
        .iam_state
        .policies
        .items
        .iter()
        .filter(|p| {
            let matches_filter = if app.iam_state.policies.filter.is_empty() {
                true
            } else {
                p.policy_name
                    .to_lowercase()
                    .contains(&app.iam_state.policies.filter.to_lowercase())
            };
            let matches_type = if app.iam_state.policy_type_filter == "All types" {
                true
            } else {
                p.policy_type == app.iam_state.policy_type_filter
            };
            matches_filter && matches_type
        })
        .collect();

    let filtered_count = filtered_policies.len();
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = app.iam_state.policies.selected / page_size;
    let pagination = render_pagination_text(current_page, total_pages);
    let policy_type_text = format!("Type: {}", app.iam_state.policy_type_filter);

    use crate::ui::filter::{render_filter_bar, FilterConfig, FilterControl};
    render_filter_bar(
        frame,
        FilterConfig {
            filter_text: &app.iam_state.policies.filter,
            placeholder: "Search",
            mode: app.mode,
            is_input_focused: app.iam_state.policy_input_focus == InputFocus::Filter,
            controls: vec![
                FilterControl {
                    text: policy_type_text,
                    is_focused: app.iam_state.policy_input_focus == POLICY_TYPE_DROPDOWN,
                },
                FilterControl {
                    text: pagination.clone(),
                    is_focused: app.iam_state.policy_input_focus == InputFocus::Pagination,
                },
            ],
            area: chunks[0],
        },
    );

    // Table
    let scroll_offset = app.iam_state.policies.scroll_offset;
    let page_policies: Vec<&Policy> = filtered_policies
        .into_iter()
        .skip(scroll_offset)
        .take(page_size)
        .collect();

    // Define columns
    let mut columns: Vec<Box<dyn Column<Policy>>> = vec![];
    for col in &app.iam_policy_visible_column_ids {
        match col.as_str() {
            "Policy name" => columns.push(Box::new(PolicyColumn::PolicyName)),
            "Type" => columns.push(Box::new(PolicyColumn::Type)),
            "Attached via" => columns.push(Box::new(PolicyColumn::AttachedVia)),
            "Attached entities" => columns.push(Box::new(PolicyColumn::AttachedEntities)),
            "Description" => columns.push(Box::new(PolicyColumn::Description)),
            "Creation time" => columns.push(Box::new(PolicyColumn::CreationTime)),
            "Edited time" => columns.push(Box::new(PolicyColumn::EditedTime)),
            _ => {}
        }
    }

    let expanded_index = app.iam_state.policies.expanded_item.and_then(|idx| {
        if idx >= scroll_offset && idx < scroll_offset + page_size {
            Some(idx - scroll_offset)
        } else {
            None
        }
    });

    let config = TableConfig {
        items: page_policies,
        selected_index: app.iam_state.policies.selected - scroll_offset,
        expanded_index,
        columns: &columns,
        sort_column: "Policy name",
        sort_direction: SortDirection::Asc,
        title: format_title(&format!("Permissions policies ({})", filtered_count)),
        area: chunks[1],
        is_active: app.mode != Mode::ColumnSelector,
        get_expanded_content: Some(Box::new(|policy: &Policy| {
            expanded_from_columns(&columns, policy)
        })),
    };

    render_table(frame, config);

    // Render dropdown for policy type when focused (after table so it appears on top)
    if app.mode == Mode::FilterInput && app.iam_state.policy_input_focus == POLICY_TYPE_DROPDOWN {
        use crate::common::render_dropdown;
        let policy_types = ["All types", "AWS managed", "Customer managed"];
        let selected_idx = policy_types
            .iter()
            .position(|&t| t == app.iam_state.policy_type_filter)
            .unwrap_or(0);
        let controls_after = pagination.len() as u16 + 3;
        render_dropdown(
            frame,
            &policy_types,
            selected_idx,
            chunks[0],
            controls_after,
        );
    }
}

pub fn render_tags_table(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = vertical(
        [
            Constraint::Length(3), // Filter with pagination
            Constraint::Min(0),    // Table
        ],
        area,
    );

    // Filter
    let page_size = app.iam_state.tags.page_size.value();
    let filtered_tags: Vec<_> = app
        .iam_state
        .tags
        .items
        .iter()
        .filter(|t| {
            if app.iam_state.tags.filter.is_empty() {
                true
            } else {
                t.key
                    .to_lowercase()
                    .contains(&app.iam_state.tags.filter.to_lowercase())
                    || t.value
                        .to_lowercase()
                        .contains(&app.iam_state.tags.filter.to_lowercase())
            }
        })
        .collect();

    let filtered_count = filtered_tags.len();
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = app.iam_state.tags.selected / page_size;
    let pagination = render_pagination_text(current_page, total_pages);

    use crate::ui::filter::{render_simple_filter, SimpleFilterConfig};
    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &app.iam_state.tags.filter,
            placeholder: "Search",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.iam_state.role_input_focus == InputFocus::Filter,
            is_pagination_focused: app.iam_state.role_input_focus == InputFocus::Pagination,
        },
    );

    // Table using common render_table
    let scroll_offset = app.iam_state.tags.scroll_offset;
    let page_tags: Vec<&RoleTag> = filtered_tags
        .into_iter()
        .skip(scroll_offset)
        .take(page_size)
        .collect();

    use crate::iam::TagColumn;
    let columns: Vec<Box<dyn Column<RoleTag>>> =
        vec![Box::new(TagColumn::Key), Box::new(TagColumn::Value)];

    let expanded_index = app.iam_state.tags.expanded_item.and_then(|idx| {
        if idx >= scroll_offset && idx < scroll_offset + page_size {
            Some(idx - scroll_offset)
        } else {
            None
        }
    });

    let config = TableConfig {
        items: page_tags,
        selected_index: app.iam_state.tags.selected - scroll_offset,
        expanded_index,
        columns: &columns,
        sort_column: "",
        sort_direction: SortDirection::Asc,
        title: format_title(&format!("Tags ({})", app.iam_state.tags.items.len())),
        area: chunks[1],
        is_active: true,
        get_expanded_content: Some(Box::new(|tag: &RoleTag| {
            plain_expanded_content(format!("Key: {}\nValue: {}", tag.key, tag.value))
        })),
    };

    render_table(frame, config);
}

pub fn render_user_groups_tab(frame: &mut Frame, app: &App, area: Rect) {
    render_user_groups_table(frame, app, area);
}

pub fn render_user_groups_table(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = vertical(
        [
            Constraint::Length(3), // Filter with pagination
            Constraint::Min(0),    // Table
        ],
        area,
    );

    let page_size = app.iam_state.user_group_memberships.page_size.value();
    let filtered_groups: Vec<_> = app
        .iam_state
        .user_group_memberships
        .items
        .iter()
        .filter(|g| {
            if app.iam_state.user_group_memberships.filter.is_empty() {
                true
            } else {
                g.group_name
                    .to_lowercase()
                    .contains(&app.iam_state.user_group_memberships.filter.to_lowercase())
            }
        })
        .collect();

    let filtered_count = filtered_groups.len();
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = app.iam_state.user_group_memberships.selected / page_size;
    let pagination = render_pagination_text(current_page, total_pages);

    use crate::ui::filter::{render_simple_filter, SimpleFilterConfig};
    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &app.iam_state.user_group_memberships.filter,
            placeholder: "Search",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.iam_state.user_input_focus == InputFocus::Filter,
            is_pagination_focused: app.iam_state.user_input_focus == InputFocus::Pagination,
        },
    );

    let scroll_offset = app.iam_state.user_group_memberships.scroll_offset;
    let page_groups: Vec<&UserGroup> = filtered_groups
        .into_iter()
        .skip(scroll_offset)
        .take(page_size)
        .collect();

    use crate::iam::UserGroupColumn;
    let columns: Vec<Box<dyn Column<UserGroup>>> = vec![
        Box::new(UserGroupColumn::GroupName),
        Box::new(UserGroupColumn::AttachedPolicies),
    ];

    let expanded_index = app
        .iam_state
        .user_group_memberships
        .expanded_item
        .and_then(|idx| {
            if idx >= scroll_offset && idx < scroll_offset + page_size {
                Some(idx - scroll_offset)
            } else {
                None
            }
        });

    let config = TableConfig {
        items: page_groups,
        selected_index: app.iam_state.user_group_memberships.selected - scroll_offset,
        expanded_index,
        columns: &columns,
        sort_column: "",
        sort_direction: SortDirection::Asc,
        title: format_title(&format!(
            "User groups membership ({})",
            app.iam_state.user_group_memberships.items.len()
        )),
        area: chunks[1],
        is_active: true,
        get_expanded_content: Some(Box::new(|group: &UserGroup| {
            plain_expanded_content(format!(
                "Group: {}\nAttached policies: {}",
                group.group_name, group.attached_policies
            ))
        })),
    };

    render_table(frame, config);
}

pub fn render_user_last_accessed_tab(frame: &mut Frame, app: &App, area: Rect) {
    render_last_accessed_table(frame, app, area);
}

pub fn render_user_tags_table(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = vertical(
        [
            Constraint::Length(3), // Filter with pagination
            Constraint::Min(0),    // Table
        ],
        area,
    );

    // Filter
    let page_size = app.iam_state.user_tags.page_size.value();
    let filtered_tags: Vec<_> = app
        .iam_state
        .user_tags
        .items
        .iter()
        .filter(|t| {
            if app.iam_state.user_tags.filter.is_empty() {
                true
            } else {
                t.key
                    .to_lowercase()
                    .contains(&app.iam_state.user_tags.filter.to_lowercase())
                    || t.value
                        .to_lowercase()
                        .contains(&app.iam_state.user_tags.filter.to_lowercase())
            }
        })
        .collect();

    let filtered_count = filtered_tags.len();
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = app.iam_state.user_tags.selected / page_size;
    let pagination = render_pagination_text(current_page, total_pages);

    use crate::ui::filter::{render_simple_filter, SimpleFilterConfig};
    render_simple_filter(
        frame,
        chunks[0],
        SimpleFilterConfig {
            filter_text: &app.iam_state.user_tags.filter,
            placeholder: "Search",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: app.iam_state.user_input_focus == InputFocus::Filter,
            is_pagination_focused: app.iam_state.user_input_focus == InputFocus::Pagination,
        },
    );

    // Table using common render_table
    let scroll_offset = app.iam_state.user_tags.scroll_offset;
    let page_tags: Vec<&UserTag> = filtered_tags
        .into_iter()
        .skip(scroll_offset)
        .take(page_size)
        .collect();

    use crate::iam::TagColumn;
    let columns: Vec<Box<dyn Column<UserTag>>> =
        vec![Box::new(TagColumn::Key), Box::new(TagColumn::Value)];

    let expanded_index = app.iam_state.user_tags.expanded_item.and_then(|idx| {
        if idx >= scroll_offset && idx < scroll_offset + page_size {
            Some(idx - scroll_offset)
        } else {
            None
        }
    });

    let config = TableConfig {
        items: page_tags,
        selected_index: app.iam_state.user_tags.selected - scroll_offset,
        expanded_index,
        columns: &columns,
        sort_column: "",
        sort_direction: SortDirection::Asc,
        title: format_title(&format!("Tags ({})", app.iam_state.user_tags.items.len())),
        area: chunks[1],
        is_active: true,
        get_expanded_content: Some(Box::new(|tag: &UserTag| {
            plain_expanded_content(format!("Key: {}\nValue: {}", tag.key, tag.value))
        })),
    };

    render_table(frame, config);
}

pub fn render_last_accessed_table(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = vertical(
        [
            Constraint::Length(3), // Filter bar
            Constraint::Min(0),    // Table
        ],
        area,
    );

    // Filter services
    let page_size = app.iam_state.last_accessed_services.page_size.value();
    let filtered_services: Vec<_> = app
        .iam_state
        .last_accessed_services
        .items
        .iter()
        .filter(|s| {
            let matches_filter = if app.iam_state.last_accessed_filter.is_empty() {
                true
            } else {
                s.service
                    .to_lowercase()
                    .contains(&app.iam_state.last_accessed_filter.to_lowercase())
            };
            let matches_history = match app.iam_state.last_accessed_history_filter {
                AccessHistoryFilter::NoFilter => true,
                AccessHistoryFilter::ServicesAccessed => {
                    !s.last_accessed.is_empty() && s.last_accessed != "Not accessed"
                }
                AccessHistoryFilter::ServicesNotAccessed => {
                    s.last_accessed.is_empty() || s.last_accessed == "Not accessed"
                }
            };
            matches_filter && matches_history
        })
        .collect();

    let filtered_count = filtered_services.len();
    let total_pages = filtered_count.div_ceil(page_size);
    let current_page = app.iam_state.last_accessed_services.selected / page_size;
    let pagination = render_pagination_text(current_page, total_pages);

    let history_filter_text = format!(
        "Filter by access history: {}",
        app.iam_state.last_accessed_history_filter.name()
    );

    use crate::ui::filter::{render_filter_bar, FilterConfig, FilterControl};
    render_filter_bar(
        frame,
        FilterConfig {
            filter_text: &app.iam_state.last_accessed_filter,
            placeholder: "Search",
            mode: app.mode,
            is_input_focused: app.iam_state.last_accessed_input_focus == InputFocus::Filter,
            controls: vec![
                FilterControl {
                    text: history_filter_text,
                    is_focused: app.iam_state.last_accessed_input_focus == HISTORY_FILTER,
                },
                FilterControl {
                    text: pagination.clone(),
                    is_focused: app.iam_state.last_accessed_input_focus == InputFocus::Pagination,
                },
            ],
            area: chunks[0],
        },
    );

    // Table using common render_table
    let scroll_offset = app.iam_state.last_accessed_services.scroll_offset;
    let page_services: Vec<&LastAccessedService> = filtered_services
        .into_iter()
        .skip(scroll_offset)
        .take(page_size)
        .collect();

    use crate::iam::LastAccessedServiceColumn;
    let columns: Vec<Box<dyn Column<LastAccessedService>>> = vec![
        Box::new(LastAccessedServiceColumn::Service),
        Box::new(LastAccessedServiceColumn::PoliciesGranting),
        Box::new(LastAccessedServiceColumn::LastAccessed),
    ];

    let expanded_index = app
        .iam_state
        .last_accessed_services
        .expanded_item
        .and_then(|idx| {
            if idx >= scroll_offset && idx < scroll_offset + page_size {
                Some(idx - scroll_offset)
            } else {
                None
            }
        });

    let config = TableConfig {
        items: page_services,
        selected_index: app
            .iam_state
            .last_accessed_services
            .selected
            .saturating_sub(scroll_offset),
        expanded_index,
        columns: &columns,
        sort_column: "Last accessed",
        sort_direction: SortDirection::Desc,
        title: format_title(&format!(
            "Allowed services ({})",
            app.iam_state.last_accessed_services.items.len()
        )),
        area: chunks[1],
        is_active: true,
        get_expanded_content: Some(Box::new(|service: &LastAccessedService| {
            plain_expanded_content(format!(
                "Service: {}\nPolicies granting permissions: {}\nLast accessed: {}",
                service.service, service.policies_granting, service.last_accessed
            ))
        })),
    };

    render_table(frame, config);

    // Render dropdown for history filter when focused (after table so it appears on top)
    if app.mode == Mode::FilterInput && app.iam_state.last_accessed_input_focus == HISTORY_FILTER {
        use crate::common::render_dropdown;
        let filter_names: Vec<&str> = AccessHistoryFilter::ALL.iter().map(|f| f.name()).collect();
        let selected_idx = AccessHistoryFilter::ALL
            .iter()
            .position(|f| *f == app.iam_state.last_accessed_history_filter)
            .unwrap_or(0);
        let controls_after = pagination.len() as u16 + 3;
        render_dropdown(
            frame,
            &filter_names,
            selected_idx,
            chunks[0],
            controls_after,
        );
    }
}

// IAM-specific helper functions
pub fn filtered_iam_users(app: &App) -> Vec<&IamUser> {
    filter_by_field(
        &app.iam_state.users.items,
        &app.iam_state.users.filter,
        |u| &u.user_name,
    )
}

pub fn filtered_iam_roles(app: &App) -> Vec<&IamRole> {
    filter_by_field(
        &app.iam_state.roles.items,
        &app.iam_state.roles.filter,
        |r| &r.role_name,
    )
}

pub fn filtered_iam_policies(app: &App) -> Vec<&Policy> {
    app.iam_state
        .policies
        .items
        .iter()
        .filter(|p| {
            let matches_filter = if app.iam_state.policies.filter.is_empty() {
                true
            } else {
                p.policy_name
                    .to_lowercase()
                    .contains(&app.iam_state.policies.filter.to_lowercase())
            };
            let matches_type = if app.iam_state.policy_type_filter == "All types" {
                true
            } else {
                p.policy_type == app.iam_state.policy_type_filter
            };
            matches_filter && matches_type
        })
        .collect()
}

pub fn filtered_tags(app: &App) -> Vec<&RoleTag> {
    filter_by_fields(&app.iam_state.tags.items, &app.iam_state.tags.filter, |t| {
        vec![&t.key, &t.value]
    })
}

pub fn filtered_user_tags(app: &App) -> Vec<&UserTag> {
    filter_by_fields(
        &app.iam_state.user_tags.items,
        &app.iam_state.user_tags.filter,
        |t| vec![&t.key, &t.value],
    )
}

pub fn filtered_last_accessed(app: &App) -> Vec<&LastAccessedService> {
    app.iam_state
        .last_accessed_services
        .items
        .iter()
        .filter(|s| {
            let matches_filter = if app.iam_state.last_accessed_filter.is_empty() {
                true
            } else {
                s.service
                    .to_lowercase()
                    .contains(&app.iam_state.last_accessed_filter.to_lowercase())
            };
            let matches_history = match app.iam_state.last_accessed_history_filter {
                AccessHistoryFilter::NoFilter => true,
                AccessHistoryFilter::ServicesAccessed => {
                    !s.last_accessed.is_empty() && s.last_accessed != "Not accessed"
                }
                AccessHistoryFilter::ServicesNotAccessed => {
                    s.last_accessed.is_empty() || s.last_accessed == "Not accessed"
                }
            };
            matches_filter && matches_history
        })
        .collect()
}

pub async fn load_iam_users(app: &mut App) -> anyhow::Result<()> {
    let users = app
        .iam_client
        .list_users()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    let mut iam_users = Vec::new();
    for u in users {
        let user_name = u.user_name().to_string();

        let has_console = app
            .iam_client
            .get_login_profile(&user_name)
            .await
            .unwrap_or(false);
        let access_key_count = app
            .iam_client
            .list_access_keys(&user_name)
            .await
            .unwrap_or(0);
        let creation_time = {
            let dt = u.create_date();
            let timestamp = dt.secs();
            let datetime = chrono::DateTime::from_timestamp(timestamp, 0).unwrap_or_default();
            datetime.format("%Y-%m-%d %H:%M:%S (UTC)").to_string()
        };

        iam_users.push(IamUser {
            user_name,
            path: u.path().to_string(),
            groups: String::new(),
            last_activity: String::new(),
            mfa: String::new(),
            password_age: String::new(),
            console_last_sign_in: String::new(),
            access_key_id: access_key_count.to_string(),
            active_key_age: String::new(),
            access_key_last_used: String::new(),
            arn: u.arn().to_string(),
            creation_time,
            console_access: if has_console {
                "Enabled".to_string()
            } else {
                "Disabled".to_string()
            },
            signing_certs: String::new(),
        });
    }

    app.iam_state.users.items = iam_users;

    Ok(())
}

pub async fn load_iam_roles(app: &mut App) -> anyhow::Result<()> {
    let roles = app
        .iam_client
        .list_roles()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    let roles: Vec<IamRole> = roles
        .into_iter()
        .map(|r| {
            let trusted_entities = r
                .assume_role_policy_document()
                .and_then(|doc| {
                    let decoded = urlencoding::decode(doc).ok()?;
                    let policy: serde_json::Value = serde_json::from_str(&decoded).ok()?;
                    let statements = policy.get("Statement")?.as_array()?;

                    let mut entities = Vec::new();
                    for stmt in statements {
                        if let Some(principal) = stmt.get("Principal") {
                            if let Some(service) = principal.get("Service") {
                                if let Some(s) = service.as_str() {
                                    let clean = s.replace(".amazonaws.com", "");
                                    entities.push(format!("AWS Service: {}", clean));
                                } else if let Some(arr) = service.as_array() {
                                    for s in arr {
                                        if let Some(s) = s.as_str() {
                                            let clean = s.replace(".amazonaws.com", "");
                                            entities.push(format!("AWS Service: {}", clean));
                                        }
                                    }
                                }
                            }
                            if let Some(aws) = principal.get("AWS") {
                                if let Some(a) = aws.as_str() {
                                    if a.starts_with("arn:aws:iam::") {
                                        if let Some(account) = a.split(':').nth(4) {
                                            entities.push(format!("Account: {}", account));
                                        }
                                    } else {
                                        entities.push(format!("Account: {}", a));
                                    }
                                } else if let Some(arr) = aws.as_array() {
                                    for a in arr {
                                        if let Some(a) = a.as_str() {
                                            if a.starts_with("arn:aws:iam::") {
                                                if let Some(account) = a.split(':').nth(4) {
                                                    entities.push(format!("Account: {}", account));
                                                }
                                            } else {
                                                entities.push(format!("Account: {}", a));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Some(entities.join(", "))
                })
                .unwrap_or_default();

            let last_activity = r
                .role_last_used()
                .and_then(|last_used| {
                    last_used.last_used_date().map(|dt| {
                        let timestamp = dt.secs();
                        let datetime =
                            chrono::DateTime::from_timestamp(timestamp, 0).unwrap_or_default();
                        datetime.format("%Y-%m-%d %H:%M:%S (UTC)").to_string()
                    })
                })
                .or_else(|| {
                    r.role_last_used().and_then(|last_used| {
                        last_used
                            .region()
                            .map(|region| format!("Used in {}", region))
                    })
                })
                .unwrap_or_else(|| "-".to_string());

            IamRole {
                role_name: r.role_name().to_string(),
                path: r.path().to_string(),
                trusted_entities,
                last_activity,
                arn: r.arn().to_string(),
                creation_time: {
                    let dt = r.create_date();
                    let timestamp = dt.secs();
                    let datetime =
                        chrono::DateTime::from_timestamp(timestamp, 0).unwrap_or_default();
                    datetime.format("%Y-%m-%d %H:%M:%S (UTC)").to_string()
                },
                description: r.description().unwrap_or("").to_string(),
                max_session_duration: r.max_session_duration(),
            }
        })
        .collect();

    app.iam_state.roles.items = roles;

    Ok(())
}

pub async fn load_iam_user_groups(app: &mut App) -> anyhow::Result<()> {
    let groups = app
        .iam_client
        .list_groups()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    let mut iam_groups = Vec::new();
    for g in groups {
        let creation_time = {
            let dt = g.create_date();
            let timestamp = dt.secs();
            let datetime = chrono::DateTime::from_timestamp(timestamp, 0).unwrap_or_default();
            datetime.format("%Y-%m-%d %H:%M:%S (UTC)").to_string()
        };

        let group_name = g.group_name().to_string();
        let user_count = app.iam_client.get_group(&group_name).await.unwrap_or(0);

        iam_groups.push(IamGroup {
            group_name,
            path: g.path().to_string(),
            users: user_count.to_string(),
            permissions: "Defined".to_string(),
            creation_time,
        });
    }

    app.iam_state.groups.items = iam_groups;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_input_focus_next() {
        assert_eq!(
            InputFocus::Filter.next(&POLICY_FILTER_CONTROLS),
            POLICY_TYPE_DROPDOWN
        );
        assert_eq!(
            POLICY_TYPE_DROPDOWN.next(&POLICY_FILTER_CONTROLS),
            InputFocus::Pagination
        );
        assert_eq!(
            InputFocus::Pagination.next(&POLICY_FILTER_CONTROLS),
            InputFocus::Filter
        );
    }

    #[test]
    fn test_policy_input_focus_prev() {
        assert_eq!(
            InputFocus::Filter.prev(&POLICY_FILTER_CONTROLS),
            InputFocus::Pagination
        );
        assert_eq!(
            InputFocus::Pagination.prev(&POLICY_FILTER_CONTROLS),
            POLICY_TYPE_DROPDOWN
        );
        assert_eq!(
            POLICY_TYPE_DROPDOWN.prev(&POLICY_FILTER_CONTROLS),
            InputFocus::Filter
        );
    }

    #[test]
    fn test_role_input_focus_next() {
        assert_eq!(
            InputFocus::Filter.next(&ROLE_FILTER_CONTROLS),
            InputFocus::Pagination
        );
        assert_eq!(
            InputFocus::Pagination.next(&ROLE_FILTER_CONTROLS),
            InputFocus::Filter
        );
    }

    #[test]
    fn test_role_input_focus_prev() {
        assert_eq!(
            InputFocus::Filter.prev(&ROLE_FILTER_CONTROLS),
            InputFocus::Pagination
        );
        assert_eq!(
            InputFocus::Pagination.prev(&ROLE_FILTER_CONTROLS),
            InputFocus::Filter
        );
    }

    #[test]
    fn test_rounded_block_for_summary() {
        use ratatui::prelude::Rect;
        let block = titled_block("Summary");
        let area = Rect::new(0, 0, 60, 15);
        let inner = block.inner(area);
        assert_eq!(inner.width, 58);
        assert_eq!(inner.height, 13);
    }

    #[test]
    fn test_user_summary_uses_dynamic_height() {
        use crate::ui::{calculate_dynamic_height, labeled_field};
        // Verify user summary height accounts for column packing
        let summary_lines = vec![
            labeled_field("ARN", "arn:aws:iam::123456789012:user/test"),
            labeled_field("Console access", "Enabled"),
            labeled_field("Access key", "AKIA..."),
            labeled_field("Created", "2024-01-01"),
            labeled_field("Last console sign-in", "2024-12-11"),
        ];
        let width = 200;
        let height = calculate_dynamic_height(&summary_lines, width);
        // With 5 fields and wide width, should pack into 3 rows or less
        assert!(height <= 3, "Expected 3 rows or less, got {}", height);
    }
}
