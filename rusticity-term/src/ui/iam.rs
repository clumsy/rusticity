use crate::app::{App, ViewMode};
use crate::common::CyclicEnum;
use crate::common::{render_pagination_text, InputFocus, SortDirection};
use crate::iam::{
    GroupUser, IamGroup, IamRole, IamUser, LastAccessedService, Policy, RoleTag, UserGroup, UserTag,
};
use crate::keymap::Mode;
use crate::table::TableState;
use crate::ui::table::Column;
use crate::ui::{
    active_border, filter_area, get_cursor, labeled_field, render_json_highlighted,
    render_last_accessed_section, render_permissions_section, render_tags_section, vertical,
};
use ratatui::{prelude::*, widgets::*};

pub const POLICY_TYPE_DROPDOWN: InputFocus = InputFocus::Dropdown("PolicyType");
pub const POLICY_FILTER_CONTROLS: [InputFocus; 3] = [
    InputFocus::Filter,
    POLICY_TYPE_DROPDOWN,
    InputFocus::Pagination,
];

pub const ROLE_FILTER_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

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
    pub roles: TableState<IamRole>,
    pub current_role: Option<String>,
    pub role_tab: RoleTab,
    pub role_input_focus: InputFocus,
    pub groups: TableState<IamGroup>,
    pub current_group: Option<String>,
    pub group_tab: GroupTab,
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
            roles: TableState::new(),
            current_role: None,
            role_tab: RoleTab::Permissions,
            role_input_focus: InputFocus::Filter,
            groups: TableState::new(),
            current_group: None,
            group_tab: GroupTab::Users,
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
            Constraint::Length(1), // Description
            Constraint::Length(3), // Filter
            Constraint::Min(0),    // Table
        ],
        area,
    );

    // Description
    let desc = Paragraph::new("An IAM user is an identity with long-term credentials that is used to interact with AWS in an account.")
        .style(Style::default().fg(Color::White));
    frame.render_widget(desc, chunks[0]);

    // Filter
    let filtered_users = crate::ui::iam::filtered_iam_users(app);
    let filtered_count = filtered_users.len();
    let page_size = app.iam_state.users.page_size.value();
    crate::ui::render_search_filter(
        frame,
        chunks[1],
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

    use crate::iam::UserColumn;
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

    let config = crate::ui::table::TableConfig {
        items: page_users,
        selected_index: app.iam_state.users.selected - app.iam_state.users.scroll_offset,
        expanded_index,
        columns: &columns,
        sort_column: "User name",
        sort_direction: SortDirection::Asc,
        title: format!(" Users ({}) ", filtered_count),
        area: chunks[2],
        get_expanded_content: Some(Box::new(|user: &&crate::iam::IamUser| {
            crate::ui::table::expanded_from_columns(&columns, user)
        })),
        is_active: app.mode != Mode::FilterInput,
    };

    crate::ui::table::render_table(frame, config);
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
    let chunks = vertical(
        [
            Constraint::Length(1), // Group name
            Constraint::Length(5), // Summary (3 lines + 2 borders)
            Constraint::Length(1), // Tabs
            Constraint::Min(0),    // Content
        ],
        area,
    );

    if let Some(group_name) = &app.iam_state.current_group {
        let label = Paragraph::new(group_name.as_str()).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(label, chunks[0]);

        // Summary section
        if let Some(group) = app
            .iam_state
            .groups
            .items
            .iter()
            .find(|g| g.group_name == *group_name)
        {
            crate::ui::render_summary(
                frame,
                chunks[1],
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
        crate::ui::render_tabs(
            frame,
            chunks[2],
            &[
                ("Users", GroupTab::Users),
                ("Permissions", GroupTab::Permissions),
                ("Access Advisor", GroupTab::AccessAdvisor),
            ],
            &app.iam_state.group_tab,
        );

        // Content area based on selected tab
        match app.iam_state.group_tab {
            GroupTab::Users => {
                render_group_users_tab(frame, app, chunks[3]);
            }
            GroupTab::Permissions => {
                render_permissions_section(
                    frame,
                    chunks[3],
                    "You can attach up to 10 managed policies.",
                    |f, area| render_policies_table(f, app, area),
                );
            }
            GroupTab::AccessAdvisor => {
                render_group_access_advisor_tab(frame, app, chunks[3]);
            }
        }
    }
}

pub fn render_group_users_tab(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = vertical(
        [
            Constraint::Length(1), // Description
            Constraint::Min(0),    // Table
        ],
        area,
    );

    frame.render_widget(
        Paragraph::new("An IAM user is an entity that you create in AWS to represent the person or application that uses it to interact with AWS."),
        chunks[0],
    );

    render_group_users_table(frame, app, chunks[1]);
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

    crate::ui::filter::render_simple_filter(
        frame,
        chunks[0],
        crate::ui::filter::SimpleFilterConfig {
            filter_text: &app.iam_state.group_users.filter,
            placeholder: "Search",
            pagination: &pagination,
            mode: app.mode,
            is_input_focused: true,
            is_pagination_focused: false,
        },
    );

    let scroll_offset = app.iam_state.group_users.scroll_offset;
    let page_users: Vec<&crate::iam::GroupUser> = filtered_users
        .into_iter()
        .skip(scroll_offset)
        .take(page_size)
        .collect();

    use crate::iam::GroupUserColumn;
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

    let config = crate::ui::table::TableConfig {
        items: page_users,
        selected_index: app.iam_state.group_users.selected - scroll_offset,
        expanded_index,
        columns: &columns,
        sort_column: "User name",
        sort_direction: SortDirection::Asc,
        title: format!(" Users ({}) ", app.iam_state.group_users.items.len()),
        area: chunks[1],
        is_active: app.mode != Mode::ColumnSelector,
        get_expanded_content: Some(Box::new(|user: &crate::iam::GroupUser| {
            crate::ui::table::plain_expanded_content(format!(
                "User name: {}\nGroups: {}\nLast activity: {}\nCreation time: {}",
                user.user_name, user.groups, user.last_activity, user.creation_time
            ))
        })),
    };

    crate::ui::table::render_table(frame, config);
}

pub fn render_group_access_advisor_tab(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = vertical(
        [
            Constraint::Length(1), // Description
            Constraint::Min(0),    // Table with filters
        ],
        area,
    );

    frame.render_widget(
        Paragraph::new(
            "IAM reports activity for services and management actions. Learn more about action last accessed information. To see actions, choose the appropriate service name from the list."
        ),
        chunks[0],
    );

    render_last_accessed_table(frame, app, chunks[1]);
}

pub fn render_group_list(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = vertical(
        [
            Constraint::Length(1), // Description
            Constraint::Length(3), // Filter
            Constraint::Min(0),    // Table
        ],
        area,
    );

    let desc = Paragraph::new("A user group is a collection of IAM users. Use groups to specify permissions for a collection of users.")
        .style(Style::default().fg(Color::White));
    frame.render_widget(desc, chunks[0]);

    let cursor = get_cursor(app.mode == Mode::FilterInput);
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

    let filter_width = (chunks[1].width as usize).saturating_sub(4);
    let pagination_len = pagination.len();
    let available_space = filter_width.saturating_sub(pagination_len + 1);

    let mut first_line_spans = vec![];
    if app.iam_state.groups.filter.is_empty() && app.mode != Mode::FilterInput {
        first_line_spans.push(Span::styled("Search", Style::default().fg(Color::DarkGray)));
    } else {
        first_line_spans.push(Span::raw(&app.iam_state.groups.filter));
    }
    if app.mode == Mode::FilterInput {
        first_line_spans.push(Span::raw(cursor));
    }

    let content_len = if app.iam_state.groups.filter.is_empty() && app.mode != Mode::FilterInput {
        6
    } else {
        app.iam_state.groups.filter.len() + cursor.len()
    };

    if content_len < available_space {
        first_line_spans.push(Span::raw(
            " ".repeat(available_space.saturating_sub(content_len)),
        ));
    }
    first_line_spans.push(Span::styled(
        pagination,
        Style::default().fg(Color::DarkGray),
    ));

    let filter = filter_area(first_line_spans, app.mode == Mode::FilterInput);
    frame.render_widget(filter, chunks[1]);

    let scroll_offset = app.iam_state.groups.scroll_offset;
    let page_groups: Vec<&crate::iam::IamGroup> = filtered_groups
        .into_iter()
        .skip(scroll_offset)
        .take(page_size)
        .collect();

    use crate::iam::GroupColumn;
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

    let config = crate::ui::table::TableConfig {
        items: page_groups,
        selected_index: app.iam_state.groups.selected - scroll_offset,
        expanded_index,
        columns: &columns,
        sort_column: "Group name",
        sort_direction: SortDirection::Asc,
        title: format!(" User groups ({}) ", app.iam_state.groups.items.len()),
        area: chunks[2],
        is_active: app.mode != Mode::ColumnSelector,
        get_expanded_content: Some(Box::new(|group: &crate::iam::IamGroup| {
            crate::ui::table::expanded_from_columns(&columns, group)
        })),
    };

    crate::ui::table::render_table(frame, config);
}

pub fn render_role_list(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = vertical(
        [
            Constraint::Length(1), // Description
            Constraint::Length(3), // Filter
            Constraint::Min(0),    // Table
        ],
        area,
    );

    let desc = Paragraph::new("An IAM role is an identity you can create that has specific permissions with credentials that are valid for short durations. Roles can be assumed by entities that you trust.")
        .style(Style::default().fg(Color::White));
    frame.render_widget(desc, chunks[0]);

    // Filter with CFN pattern
    let page_size = app.iam_state.roles.page_size.value();
    let filtered_count = crate::ui::iam::filtered_iam_roles(app).len();
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

    crate::ui::filter::render_simple_filter(
        frame,
        chunks[1],
        crate::ui::filter::SimpleFilterConfig {
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

    use crate::iam::RoleColumn;
    let mut columns: Vec<Box<dyn Column<IamRole>>> = vec![];
    for col in &app.iam_role_visible_column_ids {
        let column = match col.as_str() {
            "Role name" => Some(RoleColumn::RoleName),
            "Path" => Some(RoleColumn::Path),
            "Description" => Some(RoleColumn::Description),
            "Trusted entities" => Some(RoleColumn::TrustedEntities),
            "Creation time" => Some(RoleColumn::CreationTime),
            "ARN" => Some(RoleColumn::Arn),
            "Max CLI/API session" => Some(RoleColumn::MaxSessionDuration),
            "Last activity" => Some(RoleColumn::LastActivity),
            _ => None,
        };
        if let Some(c) = column {
            columns.push(Box::new(c));
        }
    }

    let expanded_index = app.iam_state.roles.expanded_item.and_then(|idx| {
        if idx >= scroll_offset && idx < scroll_offset + page_size {
            Some(idx - scroll_offset)
        } else {
            None
        }
    });

    let config = crate::ui::table::TableConfig {
        items: page_roles,
        selected_index: app.iam_state.roles.selected % page_size,
        expanded_index,
        columns: &columns,
        sort_column: "Role name",
        sort_direction: SortDirection::Asc,
        title: format!(" Roles ({}) ", filtered_count),
        area: chunks[2],
        is_active: app.mode != Mode::ColumnSelector,
        get_expanded_content: Some(Box::new(|role: &crate::iam::IamRole| {
            crate::ui::table::expanded_from_columns(&columns, role)
        })),
    };

    crate::ui::table::render_table(frame, config);
}

pub fn render_role_detail(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    let chunks = vertical(
        [
            Constraint::Length(1), // Role name
            Constraint::Length(7), // Summary (5 lines + 2 borders)
            Constraint::Length(1), // Tabs
            Constraint::Min(0),    // Content
        ],
        area,
    );

    // Role name label
    if let Some(role_name) = &app.iam_state.current_role {
        let label = Paragraph::new(role_name.as_str()).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(label, chunks[0]);
    }

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
                .map(|d| crate::ui::format_duration(d as u64))
                .unwrap_or_default();

            crate::ui::render_summary(
                frame,
                chunks[1],
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
    crate::ui::render_tabs(
        frame,
        chunks[2],
        &[
            (RoleTab::Permissions.name(), RoleTab::Permissions),
            (
                RoleTab::TrustRelationships.name(),
                RoleTab::TrustRelationships,
            ),
            (RoleTab::Tags.name(), RoleTab::Tags),
            (RoleTab::LastAccessed.name(), RoleTab::LastAccessed),
            (RoleTab::RevokeSessions.name(), RoleTab::RevokeSessions),
        ],
        &app.iam_state.role_tab,
    );

    // Content based on selected tab
    match app.iam_state.role_tab {
        RoleTab::Permissions => {
            render_permissions_section(
                frame,
                chunks[3],
                "You can attach up to 10 managed policies.",
                |f, area| render_policies_table(f, app, area),
            );
        }
        RoleTab::TrustRelationships => {
            let chunks_inner = vertical(
                [
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ],
                chunks[3],
            );

            frame.render_widget(
                Paragraph::new("Trusted entities").style(Style::default().fg(Color::Cyan).bold()),
                chunks_inner[0],
            );

            frame.render_widget(
                Paragraph::new("Entities that can assume this role under specified conditions."),
                chunks_inner[1],
            );

            render_json_highlighted(
                frame,
                chunks_inner[2],
                &app.iam_state.trust_policy_document,
                app.iam_state.trust_policy_scroll,
                " Trust Policy ",
            );
        }
        RoleTab::Tags => {
            render_tags_section(frame, chunks[3], |f, area| render_tags_table(f, app, area));
        }
        RoleTab::RevokeSessions => {
            let chunks_inner = vertical(
                [
                    Constraint::Length(1),
                    Constraint::Length(2),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ],
                chunks[3],
            );

            frame.render_widget(
                Paragraph::new("Revoke all active sessions")
                    .style(Style::default().fg(Color::Cyan).bold()),
                chunks_inner[0],
            );

            frame.render_widget(
                Paragraph::new(
                    "If you choose Revoke active sessions, IAM attaches an inline policy named AWSRevokeOlderSessions to this role. This policy denies access to all currently active sessions for this role. You can continue to create new sessions based on this role. If you need to undo this action later, you can remove the inline policy."
                ).wrap(ratatui::widgets::Wrap { trim: true }),
                chunks_inner[1],
            );

            frame.render_widget(
                Paragraph::new("Here is an example of the AWSRevokeOlderSessions policy that is created after you choose Revoke active sessions:"),
                chunks_inner[2],
            );

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
                chunks_inner[3],
                example_policy,
                app.iam_state.revoke_sessions_scroll,
                " Example Policy ",
            );
        }
        RoleTab::LastAccessed => {
            render_last_accessed_section(
                frame,
                chunks[3],
                "Last accessed information shows the services that this role can access and when those services were last accessed. Review this data to remove unused permissions.",
                "IAM reports activity for services and management actions. Learn more about action last accessed information. To see actions, choose the appropriate service name from the list.",
                |f, area| render_last_accessed_table(f, app, area),
            );
        }
    }
}

pub fn render_user_detail(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);

    let chunks = vertical(
        [
            Constraint::Length(1), // User name
            Constraint::Length(7), // Summary (5 lines + 2 borders)
            Constraint::Length(1), // Tabs
            Constraint::Min(0),    // Content
        ],
        area,
    );

    // User name label
    if let Some(user_name) = &app.iam_state.current_user {
        let label = Paragraph::new(user_name.as_str()).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(label, chunks[0]);
    }

    // Summary section
    if let Some(user_name) = &app.iam_state.current_user {
        if let Some(user) = app
            .iam_state
            .users
            .items
            .iter()
            .find(|u| u.user_name == *user_name)
        {
            let summary_block = Block::default()
                .title(" Summary ")
                .borders(Borders::ALL)
                .border_type(BorderType::Plain)
                .border_style(active_border());

            let summary_inner = summary_block.inner(chunks[1]);
            frame.render_widget(summary_block, chunks[1]);

            let summary_lines = vec![
                labeled_field("ARN", &user.arn),
                labeled_field("Console access", &user.console_access),
                labeled_field("Access key", &user.access_key_id),
                labeled_field("Created", &user.creation_time),
                labeled_field("Last console sign-in", &user.console_last_sign_in),
            ];

            let summary_paragraph = Paragraph::new(summary_lines);
            frame.render_widget(summary_paragraph, summary_inner);
        }
    }

    // Tabs
    crate::ui::render_tabs(
        frame,
        chunks[2],
        &[
            ("Permissions", UserTab::Permissions),
            ("Groups", UserTab::Groups),
            ("Tags", UserTab::Tags),
            ("Security Credentials", UserTab::SecurityCredentials),
            ("Last Accessed", UserTab::LastAccessed),
        ],
        &app.iam_state.user_tab,
    );

    // Content area - Permissions tab
    if app.iam_state.user_tab == UserTab::Permissions {
        render_permissions_tab(frame, app, chunks[3]);
    } else if app.iam_state.user_tab == UserTab::Groups {
        render_user_groups_tab(frame, app, chunks[3]);
    } else if app.iam_state.user_tab == UserTab::Tags {
        render_tags_section(frame, chunks[3], |f, area| {
            render_user_tags_table(f, app, area)
        });
    } else if app.iam_state.user_tab == UserTab::LastAccessed {
        render_user_last_accessed_tab(frame, app, chunks[3]);
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

    let chunks = vertical([Constraint::Length(1), Constraint::Min(0)], area);

    let policy_name = app.iam_state.current_policy.as_deref().unwrap_or("");
    frame.render_widget(
        Paragraph::new(policy_name).style(Style::default().fg(Color::Cyan).bold()),
        chunks[0],
    );

    render_json_highlighted(
        frame,
        chunks[1],
        &app.iam_state.policy_document,
        app.iam_state.policy_scroll,
        " Policy Document ",
    );
}

pub fn render_policies_table(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = vertical(
        [
            Constraint::Length(3), // Filter with dropdown and pagination
            Constraint::Min(0),    // Table
        ],
        area,
    );

    // Filter
    let cursor = get_cursor(app.mode == Mode::FilterInput);
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
    let dropdown = format!("Type: {}", app.iam_state.policy_type_filter);

    let filter_width = (chunks[0].width as usize).saturating_sub(4);
    let right_content = format!("{} ⋮ {}", dropdown, pagination);
    let right_len = right_content.len();
    let available_space = filter_width.saturating_sub(right_len + 1);

    let mut first_line_spans = vec![];
    if app.iam_state.policies.filter.is_empty() && app.mode != Mode::FilterInput {
        first_line_spans.push(Span::styled("Search", Style::default().fg(Color::DarkGray)));
    } else {
        let display_text = if app.iam_state.policies.filter.len() > available_space {
            format!(
                "...{}",
                &app.iam_state.policies.filter
                    [app.iam_state.policies.filter.len() - available_space + 3..]
            )
        } else {
            app.iam_state.policies.filter.clone()
        };
        first_line_spans.push(Span::raw(display_text));
    }
    if app.mode == Mode::FilterInput {
        first_line_spans.push(Span::raw(cursor));
    }

    first_line_spans.push(Span::raw(
        " ".repeat(
            available_space.saturating_sub(
                first_line_spans
                    .iter()
                    .map(|s| s.content.len())
                    .sum::<usize>(),
            ),
        ),
    ));
    first_line_spans.push(Span::styled(
        right_content,
        Style::default().fg(Color::DarkGray),
    ));

    let filter = filter_area(first_line_spans, app.mode == Mode::FilterInput);
    frame.render_widget(filter, chunks[0]);

    // Table
    let scroll_offset = app.iam_state.policies.scroll_offset;
    let page_policies: Vec<&crate::iam::Policy> = filtered_policies
        .into_iter()
        .skip(scroll_offset)
        .take(page_size)
        .collect();

    // Define columns
    use crate::iam::PolicyColumn;
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

    let config = crate::ui::table::TableConfig {
        items: page_policies,
        selected_index: app.iam_state.policies.selected - scroll_offset,
        expanded_index,
        columns: &columns,
        sort_column: "Policy name",
        sort_direction: SortDirection::Asc,
        title: format!(" Permissions policies ({}) ", filtered_count),
        area: chunks[1],
        is_active: app.mode != Mode::ColumnSelector,
        get_expanded_content: Some(Box::new(|policy: &crate::iam::Policy| {
            crate::ui::table::expanded_from_columns(&columns, policy)
        })),
    };

    crate::ui::table::render_table(frame, config);
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
    let cursor = get_cursor(app.mode == Mode::FilterInput);
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

    let filter_width = (chunks[0].width as usize).saturating_sub(4);
    let pagination_len = pagination.len();
    let available_space = filter_width.saturating_sub(pagination_len + 1);

    let mut first_line_spans = vec![];
    if app.iam_state.tags.filter.is_empty() && app.mode != Mode::FilterInput {
        first_line_spans.push(Span::styled("Search", Style::default().fg(Color::DarkGray)));
    } else {
        first_line_spans.push(Span::raw(&app.iam_state.tags.filter));
    }
    if app.mode == Mode::FilterInput {
        first_line_spans.push(Span::raw(cursor));
    }

    let content_len = if app.iam_state.tags.filter.is_empty() && app.mode != Mode::FilterInput {
        6
    } else {
        app.iam_state.tags.filter.len() + cursor.len()
    };

    if content_len < available_space {
        first_line_spans.push(Span::raw(
            " ".repeat(available_space.saturating_sub(content_len)),
        ));
    }
    first_line_spans.push(Span::styled(
        pagination,
        Style::default().fg(Color::DarkGray),
    ));

    let filter = filter_area(first_line_spans, app.mode == Mode::FilterInput);
    frame.render_widget(filter, chunks[0]);

    // Table using common render_table
    let scroll_offset = app.iam_state.tags.scroll_offset;
    let page_tags: Vec<&crate::iam::RoleTag> = filtered_tags
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

    let config = crate::ui::table::TableConfig {
        items: page_tags,
        selected_index: app.iam_state.tags.selected - scroll_offset,
        expanded_index,
        columns: &columns,
        sort_column: "",
        sort_direction: SortDirection::Asc,
        title: format!(" Tags ({}) ", app.iam_state.tags.items.len()),
        area: chunks[1],
        is_active: true,
        get_expanded_content: Some(Box::new(|tag: &crate::iam::RoleTag| {
            crate::ui::table::plain_expanded_content(format!(
                "Key: {}\nValue: {}",
                tag.key, tag.value
            ))
        })),
    };

    crate::ui::table::render_table(frame, config);
}

pub fn render_user_groups_tab(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = vertical(
        [
            Constraint::Length(1), // Description
            Constraint::Min(0),    // Table
        ],
        area,
    );

    let desc = Paragraph::new(
        "A user group is a collection of IAM users. Use groups to specify permissions for a collection of users. A user can be a member of up to 10 groups at a time.",
    )
    .style(Style::default().fg(Color::White));
    frame.render_widget(desc, chunks[0]);

    render_user_groups_table(frame, app, chunks[1]);
}

pub fn render_user_groups_table(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = vertical(
        [
            Constraint::Length(3), // Filter with pagination
            Constraint::Min(0),    // Table
        ],
        area,
    );

    let cursor = get_cursor(app.mode == Mode::FilterInput);
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

    let filter_width = (chunks[0].width as usize).saturating_sub(4);
    let pagination_len = pagination.len();
    let available_space = filter_width.saturating_sub(pagination_len + 1);

    let mut first_line_spans = vec![];
    if app.iam_state.user_group_memberships.filter.is_empty() && app.mode != Mode::FilterInput {
        first_line_spans.push(Span::styled("Search", Style::default().fg(Color::DarkGray)));
    } else {
        first_line_spans.push(Span::raw(&app.iam_state.user_group_memberships.filter));
    }
    if app.mode == Mode::FilterInput {
        first_line_spans.push(Span::raw(cursor));
    }

    let content_len = if app.iam_state.user_group_memberships.filter.is_empty()
        && app.mode != Mode::FilterInput
    {
        6
    } else {
        app.iam_state.user_group_memberships.filter.len() + cursor.len()
    };

    if content_len < available_space {
        first_line_spans.push(Span::raw(
            " ".repeat(available_space.saturating_sub(content_len)),
        ));
    }
    first_line_spans.push(Span::styled(
        pagination,
        Style::default().fg(Color::DarkGray),
    ));

    let filter = filter_area(first_line_spans, app.mode == Mode::FilterInput);
    frame.render_widget(filter, chunks[0]);

    let scroll_offset = app.iam_state.user_group_memberships.scroll_offset;
    let page_groups: Vec<&crate::iam::UserGroup> = filtered_groups
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

    let config = crate::ui::table::TableConfig {
        items: page_groups,
        selected_index: app.iam_state.user_group_memberships.selected - scroll_offset,
        expanded_index,
        columns: &columns,
        sort_column: "",
        sort_direction: SortDirection::Asc,
        title: format!(
            " User groups membership ({}) ",
            app.iam_state.user_group_memberships.items.len()
        ),
        area: chunks[1],
        is_active: true,
        get_expanded_content: Some(Box::new(|group: &crate::iam::UserGroup| {
            crate::ui::table::plain_expanded_content(format!(
                "Group: {}\nAttached policies: {}",
                group.group_name, group.attached_policies
            ))
        })),
    };

    crate::ui::table::render_table(frame, config);
}

pub fn render_user_last_accessed_tab(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = vertical(
        [
            Constraint::Length(1), // Description
            Constraint::Length(1), // Note above table
            Constraint::Min(0),    // Table with filters
        ],
        area,
    );

    frame.render_widget(
        Paragraph::new(
            "Last accessed information shows the services that this user can access and when those services were last accessed. Review this data to remove unused permissions."
        ),
        chunks[0],
    );

    frame.render_widget(
        Paragraph::new(
            "IAM reports activity for services and management actions. Learn more about action last accessed information. To see actions, choose the appropriate service name from the list."
        ),
        chunks[1],
    );

    render_last_accessed_table(frame, app, chunks[2]);
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
    let cursor = get_cursor(app.mode == Mode::FilterInput);
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

    let filter_width = (chunks[0].width as usize).saturating_sub(4);
    let pagination_len = pagination.len();
    let available_space = filter_width.saturating_sub(pagination_len + 1);

    let mut first_line_spans = vec![];
    if app.iam_state.user_tags.filter.is_empty() && app.mode != Mode::FilterInput {
        first_line_spans.push(Span::styled("Search", Style::default().fg(Color::DarkGray)));
    } else {
        first_line_spans.push(Span::raw(&app.iam_state.user_tags.filter));
    }
    if app.mode == Mode::FilterInput {
        first_line_spans.push(Span::raw(cursor));
    }

    let content_len = if app.iam_state.user_tags.filter.is_empty() && app.mode != Mode::FilterInput
    {
        6
    } else {
        app.iam_state.user_tags.filter.len() + cursor.len()
    };

    if content_len < available_space {
        first_line_spans.push(Span::raw(
            " ".repeat(available_space.saturating_sub(content_len)),
        ));
    }
    first_line_spans.push(Span::styled(
        pagination,
        Style::default().fg(Color::DarkGray),
    ));

    let filter = filter_area(first_line_spans, app.mode == Mode::FilterInput);
    frame.render_widget(filter, chunks[0]);

    // Table using common render_table
    let scroll_offset = app.iam_state.user_tags.scroll_offset;
    let page_tags: Vec<&crate::iam::UserTag> = filtered_tags
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

    let config = crate::ui::table::TableConfig {
        items: page_tags,
        selected_index: app.iam_state.user_tags.selected - scroll_offset,
        expanded_index,
        columns: &columns,
        sort_column: "",
        sort_direction: SortDirection::Asc,
        title: format!(" Tags ({}) ", app.iam_state.user_tags.items.len()),
        area: chunks[1],
        is_active: true,
        get_expanded_content: Some(Box::new(|tag: &crate::iam::UserTag| {
            crate::ui::table::plain_expanded_content(format!(
                "Key: {}\nValue: {}",
                tag.key, tag.value
            ))
        })),
    };

    crate::ui::table::render_table(frame, config);
}

pub fn render_last_accessed_table(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = vertical(
        [
            Constraint::Length(3), // Filter with dropdown and pagination
            Constraint::Min(0),    // Table
        ],
        area,
    );

    // Filter
    let cursor = get_cursor(app.mode == Mode::FilterInput);
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
    let dropdown = format!(
        "Filter by services access history: {}",
        app.iam_state.last_accessed_history_filter.name()
    );

    let filter_width = (chunks[0].width as usize).saturating_sub(4);
    let right_content = format!("{} ⋮ {}", dropdown, pagination);
    let right_len = right_content.len();
    let available_space = filter_width.saturating_sub(right_len + 1);

    let mut first_line_spans = vec![];
    if app.iam_state.last_accessed_filter.is_empty() && app.mode != Mode::FilterInput {
        first_line_spans.push(Span::styled("Search", Style::default().fg(Color::DarkGray)));
    } else {
        let display_text = if app.iam_state.last_accessed_filter.len() > available_space {
            format!(
                "...{}",
                &app.iam_state.last_accessed_filter
                    [app.iam_state.last_accessed_filter.len() - available_space + 3..]
            )
        } else {
            app.iam_state.last_accessed_filter.clone()
        };
        first_line_spans.push(Span::raw(display_text));
    }
    if app.mode == Mode::FilterInput {
        first_line_spans.push(Span::raw(cursor));
    }

    first_line_spans.push(Span::raw(
        " ".repeat(
            available_space.saturating_sub(
                first_line_spans
                    .iter()
                    .map(|s| s.content.len())
                    .sum::<usize>(),
            ),
        ),
    ));
    first_line_spans.push(Span::styled(
        right_content,
        Style::default().fg(Color::DarkGray),
    ));
    let filter = filter_area(first_line_spans, app.mode == Mode::FilterInput);
    frame.render_widget(filter, chunks[0]);

    // Table using common render_table
    let scroll_offset = app.iam_state.last_accessed_services.scroll_offset;
    let page_services: Vec<&crate::iam::LastAccessedService> = filtered_services
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

    let config = crate::ui::table::TableConfig {
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
        title: format!(
            " Allowed services ({}) ",
            app.iam_state.last_accessed_services.items.len()
        ),
        area: chunks[1],
        is_active: true,
        get_expanded_content: Some(Box::new(|service: &crate::iam::LastAccessedService| {
            crate::ui::table::plain_expanded_content(format!(
                "Service: {}\nPolicies granting permissions: {}\nLast accessed: {}",
                service.service, service.policies_granting, service.last_accessed
            ))
        })),
    };

    crate::ui::table::render_table(frame, config);
}

// IAM-specific helper functions
pub fn filtered_iam_users(app: &App) -> Vec<&crate::iam::IamUser> {
    if app.iam_state.users.filter.is_empty() {
        app.iam_state.users.items.iter().collect()
    } else {
        app.iam_state
            .users
            .items
            .iter()
            .filter(|u| {
                u.user_name
                    .to_lowercase()
                    .contains(&app.iam_state.users.filter.to_lowercase())
            })
            .collect()
    }
}

pub fn filtered_iam_roles(app: &App) -> Vec<&crate::iam::IamRole> {
    if app.iam_state.roles.filter.is_empty() {
        app.iam_state.roles.items.iter().collect()
    } else {
        app.iam_state
            .roles
            .items
            .iter()
            .filter(|r| {
                r.role_name
                    .to_lowercase()
                    .contains(&app.iam_state.roles.filter.to_lowercase())
            })
            .collect()
    }
}

pub fn filtered_iam_policies(app: &App) -> Vec<&crate::iam::Policy> {
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

pub fn filtered_tags(app: &App) -> Vec<&crate::iam::RoleTag> {
    if app.iam_state.tags.filter.is_empty() {
        app.iam_state.tags.items.iter().collect()
    } else {
        app.iam_state
            .tags
            .items
            .iter()
            .filter(|t| {
                t.key
                    .to_lowercase()
                    .contains(&app.iam_state.tags.filter.to_lowercase())
                    || t.value
                        .to_lowercase()
                        .contains(&app.iam_state.tags.filter.to_lowercase())
            })
            .collect()
    }
}

pub fn filtered_user_tags(app: &App) -> Vec<&crate::iam::UserTag> {
    if app.iam_state.user_tags.filter.is_empty() {
        app.iam_state.user_tags.items.iter().collect()
    } else {
        app.iam_state
            .user_tags
            .items
            .iter()
            .filter(|t| {
                t.key
                    .to_lowercase()
                    .contains(&app.iam_state.user_tags.filter.to_lowercase())
                    || t.value
                        .to_lowercase()
                        .contains(&app.iam_state.user_tags.filter.to_lowercase())
            })
            .collect()
    }
}

pub fn filtered_last_accessed(app: &App) -> Vec<&crate::iam::LastAccessedService> {
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
                crate::ui::iam::AccessHistoryFilter::NoFilter => true,
                crate::ui::iam::AccessHistoryFilter::ServicesAccessed => {
                    !s.last_accessed.is_empty() && s.last_accessed != "Not accessed"
                }
                crate::ui::iam::AccessHistoryFilter::ServicesNotAccessed => {
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

        iam_users.push(crate::iam::IamUser {
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

    let roles: Vec<crate::iam::IamRole> = roles
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

            crate::iam::IamRole {
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

        iam_groups.push(crate::iam::IamGroup {
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
}
