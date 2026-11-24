//! VPC › Subnets: the read-only list, with the route table, network ACL and
//! network border group of each subnet. No detail view; Enter expands a row.
use super::list::{self, yes_no};
use crate::{
    app::{App, Service},
    async_read::ReadScope,
    elb::keys,
    keymap::Action,
    spec_table::{Spec, Texts},
};
use ratatui::prelude::*;
use rusticity_core::{vpc::Subnet, VpcClient};

/// The console columns, in console order. Name is not one; the filter
/// matches it, as in the VPC list.
pub(crate) const COLUMNS: Spec<Subnet> = &[
    ("Subnet ID", 26, |s| s.subnet_id.clone()),
    ("ARN", 70, |s| s.arn.clone()),
    ("State", 14, |s| super::state_label(&s.state).0),
    ("VPC", 23, |s| s.vpc_id.clone()),
    ("Block Public Access", 22, |s| {
        super::block_public_access_label(&s.block_public_access).0
    }),
    ("Automated internet firewall connectivity type", 16, |s| {
        s.firewall_connectivity.clone()
    }),
    ("Outpost ID", 20, |s| s.outpost_id.clone()),
    ("IPv4 CIDR", 18, |s| s.ipv4_cidr.clone()),
    ("IPv6 CIDR", 24, |s| s.ipv6_cidrs.clone()),
    ("IPv6 CIDR association ID", 32, |s| {
        s.ipv6_association_ids.clone()
    }),
    ("Available IPv4 addresses", 10, |s| {
        s.available_ipv4.map(|n| n.to_string()).unwrap_or_default()
    }),
    ("Availability Zone", 12, |s| s.availability_zone.clone()),
    ("Network border group", 14, |s| {
        s.network_border_group.clone()
    }),
    ("Route table", 24, |s| s.route_table.clone()),
    ("Network ACL", 24, |s| s.network_acl.clone()),
    ("Default subnet", 8, |s| yes_no(s.default_for_az)),
    ("Auto-assign public IPv4 address", 8, |s| {
        yes_no(s.map_public_ip)
    }),
    ("Auto-assign customer-owned IPv4 address", 8, |s| {
        yes_no(s.map_customer_owned_ip)
    }),
    ("Customer-owned IPv4 pool", 20, |s| {
        s.customer_owned_ipv4_pool.clone()
    }),
    ("Auto-assign IPv6 address", 8, |s| yes_no(s.assign_ipv6)),
    ("Owner ID", 14, |s| s.owner_id.clone()),
    ("Subnet ARN", 70, |s| s.arn.clone()),
    ("Enable DNS64", 10, |s| yes_no(s.dns64)),
];

/// Shown at first; the other columns are in preferences.
const VISIBLE: &[&str] = &[
    "Subnet ID",
    "State",
    "VPC",
    "IPv4 CIDR",
    "IPv6 CIDR",
    "Available IPv4 addresses",
    "Availability Zone",
    "Route table",
    "Network ACL",
    "Default subnet",
];

/// Embedded at `app.subnets_state`.
pub type State = list::State<Subnet>;

impl Default for State {
    fn default() -> Self {
        list::State::new(list::table(COLUMNS, VISIBLE, "Subnet ID").searching(|s| s.name.clone()))
    }
}

pub fn active(app: &App) -> bool {
    app.service_selected && app.current_service == Service::VpcSubnets
}

pub(crate) fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "inventory:vpc-subnets")
}

/// Input-handler safe: clears the rows at once and queues the read.
pub fn request(app: &mut App) {
    let scope = scope(app);
    app.subnets_state.request(scope);
}

/// Tests: the rows in the table count as a fresh list read.
#[cfg(test)]
pub(crate) fn mark_loaded(app: &mut App) {
    let scope = scope(app);
    app.subnets_state.inventory.mark_loaded(scope);
}

/// The ARN `y` copies.
pub(crate) fn selected_arn(app: &App) -> Option<String> {
    app.subnets_state
        .selected()
        .map(|s| s.arn.clone())
        .filter(|arn| !arn.is_empty())
}

fn state(app: &mut App) -> &mut State {
    &mut app.subnets_state
}

fn input(app: &mut App, action: &Action) -> bool {
    let selected = selected_arn(app);
    let (state, keys) = keys!(app, subnets_state, selected);
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
    app.subnets_state
        .drive(
            &scope,
            "Subnets read",
            async move { read.list_subnets().await },
        )
        .await
}

pub fn breadcrumb() -> Vec<String> {
    vec!["VPC".into(), "Subnets".into()]
}

pub fn console_url(app: &App) -> String {
    super::console_url_list(&app.aws.config.region, "subnets:")
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let state = &app.subnets_state;
    crate::spec_table::render(
        frame,
        app,
        &state.list,
        Texts {
            title: "Subnets",
            placeholder: "Filter subnets",
            no_items: "No subnets in this Region.",
        },
        state.inventory.status(),
        state.inventory.error(),
        list::style,
        area,
    );
}
