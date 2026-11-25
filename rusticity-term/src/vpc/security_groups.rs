//! VPC › Security groups: the read-only list with inbound and outbound rule
//! counts. No detail view; Enter expands a row.
use super::list;
use crate::{
    app::{App, Service},
    async_read::ReadScope,
    elb::keys,
    keymap::Action,
    spec_table::{Spec, Texts},
};
use ratatui::prelude::*;
use rusticity_core::{vpc::SecurityGroup, VpcClient};

/// `1 Permission entry`, `3 Permission entries`.
fn entries(count: usize) -> String {
    if count == 1 {
        "1 Permission entry".into()
    } else {
        format!("{count} Permission entries")
    }
}

/// The console columns, in console order. Name is not one; the filter
/// matches it, as in the VPC list.
pub(crate) const COLUMNS: Spec<SecurityGroup> = &[
    ("Security group ID", |g| g.group_id.clone()),
    ("Security group name", |g| g.group_name.clone()),
    ("VPC ID", |g| g.vpc_id.clone()),
    ("Description", |g| g.description.clone()),
    ("Owner", |g| g.owner_id.clone()),
    ("Inbound rules count", |g| entries(g.inbound_rules)),
    ("Outbound rules count", |g| entries(g.outbound_rules)),
];

/// Embedded at `app.security_groups_state`.
pub type State = list::State<SecurityGroup>;

impl Default for State {
    fn default() -> Self {
        let all: Vec<&'static str> = COLUMNS.iter().map(|(id, ..)| *id).collect();
        list::State::new(
            list::table(COLUMNS, &all, "Security group ID").searching(|g| g.name.clone()),
        )
    }
}

pub fn active(app: &App) -> bool {
    app.service_selected && app.current_service == Service::VpcSecurityGroups
}

pub(crate) fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "inventory:vpc-security-groups")
}

/// Input-handler safe: clears the rows at once and queues the read.
pub fn request(app: &mut App) {
    let scope = scope(app);
    app.security_groups_state.request(scope);
}

/// Tests: the rows in the table count as a fresh list read.
#[cfg(test)]
pub(crate) fn mark_loaded(app: &mut App) {
    let scope = scope(app);
    app.security_groups_state.inventory.mark_loaded(scope);
}

/// The ARN `y` copies.
pub(crate) fn selected_arn(app: &App) -> Option<String> {
    app.security_groups_state
        .selected()
        .map(|g| g.arn.clone())
        .filter(|arn| !arn.is_empty())
}

fn state(app: &mut App) -> &mut State {
    &mut app.security_groups_state
}

fn input(app: &mut App, action: &Action) -> bool {
    let selected = selected_arn(app);
    let (state, keys) = keys!(app, security_groups_state, selected);
    crate::spec_table::input(&mut state.list, keys, action)
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    active(app)
        && list::handle_action(
            app,
            list::Service {
                state,
                request,
                input,
            },
            action,
        )
}

/// Drive the list read; called for the owning tab only.
pub async fn poll(app: &mut App) -> bool {
    let scope = scope(app);
    let read = VpcClient::new(app.aws.config.clone());
    app.security_groups_state
        .drive(&scope, "Security groups read", async move {
            read.list_security_groups().await
        })
        .await
}

pub fn breadcrumb() -> Vec<String> {
    vec!["VPC".into(), "Security groups".into()]
}

pub fn console_url(app: &App) -> String {
    super::console_url_list(&app.aws.config.region, "SecurityGroups:")
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let state = &app.security_groups_state;
    crate::spec_table::render(
        frame,
        app,
        &state.list,
        Texts {
            title: "Security groups",
            placeholder: "Filter security groups",
            no_items: "No security groups in this Region.",
        },
        state.inventory.status(),
        state.inventory.error(),
        list::style,
        area,
    );
}
