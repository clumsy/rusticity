//! EC2 › Load balancers: the list, then one load balancer with its console
//! tabs. Listeners, the Resource map (listener, rule, target group and target
//! health), security groups, metrics, attributes and tags come from one detail
//! read; see `rusticity_core::elb`.
use super::{keys, DetailRead, TagRow, TAG_COLUMNS};
use crate::{
    app::{App, Service},
    async_read::{ReadScope, ReadStatus},
    common::CyclicEnum,
    console::{component, ConsoleUrl},
    keymap::{Action, Mode},
    list_read::ListRead,
    spec_table::{AnyTable, Spec, Table},
};
use rusticity_core::elb::{
    ElbClient, Listener, LoadBalancer, LoadBalancerDetail, ResourceRoute, SecurityGroup,
    ZoneMapping, MAX_CHARTS,
};

pub(crate) const LIST_COLUMNS: Spec<LoadBalancer> = &[
    ("Name", 24, |l| l.name.clone()),
    ("DNS name", 44, |l| l.dns_name.clone()),
    ("State", 12, |l| super::words(&l.state)),
    ("VPC ID", 22, |l| l.vpc_id.clone()),
    ("Availability Zones", 20, |l| match l.zones.len() {
        0 => String::new(),
        1 => "1 Availability Zone".into(),
        n => format!("{n} Availability Zones"),
    }),
    ("Type", 12, |l| l.lb_type.clone()),
    ("Date created", 26, |l| super::timestamp(&l.created_time)),
];

pub(crate) const LISTENER_COLUMNS: Spec<Listener> = &[
    ("Protocol:Port", 16, |l| {
        format!(
            "{}:{}",
            l.protocol,
            l.port.map(|p| p.to_string()).unwrap_or_default()
        )
    }),
    ("Default action", 50, |l| l.default_action.clone()),
    ("Rules", 10, |l| match l.rules {
        Some(1) => "1 rule".into(),
        Some(n) => format!("{n} rules"),
        None => String::new(),
    }),
    ("ARN", 70, |l| l.arn.clone()),
    ("Security policy", 30, |l| l.ssl_policy.clone()),
    ("Default SSL/TLS certificate", 40, |l| {
        l.default_certificate
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_string()
    }),
    ("mTLS", 12, |l| super::words(&l.mtls_mode)),
    ("Trust store", 30, |l| {
        rusticity_core::elb::name_from_arn(&l.trust_store_arn)
    }),
];

pub(crate) const MAPPING_COLUMNS: Spec<ZoneMapping> = &[
    ("Availability Zone", 20, |z| z.zone.clone()),
    ("Subnet", 26, |z| z.subnet_id.clone()),
    ("IPv4 address", 20, |z| z.ipv4.join(", ")),
    ("Private IPv4 address", 22, |z| z.private_ipv4.join(", ")),
    ("IPv6 address", 30, |z| z.ipv6.join(", ")),
];

fn count(n: Option<usize>) -> String {
    n.map(|n| n.to_string())
        .unwrap_or_else(|| "Unavailable".into())
}

pub(crate) const ROUTE_COLUMNS: Spec<ResourceRoute> = &[
    ("Listener", 16, |r| r.listener.clone()),
    ("Rule", 14, |r| r.rule.clone()),
    ("Target group", 32, |r| r.target_group.clone()),
    ("Targets", 10, |r| count(r.targets)),
    ("Healthy", 10, |r| count(r.healthy)),
    ("Unhealthy", 10, |r| count(r.unhealthy)),
];

pub(crate) const GROUP_COLUMNS: Spec<SecurityGroup> = &[
    ("Security group ID", 24, |g| g.id.clone()),
    ("Name", 32, |g| g.name.clone()),
    ("Description", 60, |g| g.description.clone()),
];

/// Console tabs, in console order, below the fixed Details pane.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Listeners,
    NetworkMapping,
    ResourceMap,
    Security,
    Monitoring,
    Attributes,
    Tags,
}

impl CyclicEnum for Tab {
    const ALL: &'static [Self] = &[
        Self::Listeners,
        Self::NetworkMapping,
        Self::ResourceMap,
        Self::Security,
        Self::Monitoring,
        Self::Attributes,
        Self::Tags,
    ];
}

impl Tab {
    /// Rules belong to Application Load Balancer listeners only.
    pub fn name(self, lb_type: &str) -> &'static str {
        match self {
            Self::Listeners if lb_type == "application" => "Listeners and rules",
            Self::Listeners => "Listeners",
            Self::NetworkMapping => "Network mapping",
            Self::ResourceMap => "Resource map",
            Self::Security => "Security",
            Self::Monitoring => "Monitoring",
            Self::Attributes => "Attributes",
            Self::Tags => "Tags",
        }
    }
}

/// The entered load balancer.
pub struct View {
    /// Identity snapshot of the entered row.
    pub source: LoadBalancer,
    pub tab: Tab,
    pub(crate) read: DetailRead<LoadBalancerDetail>,
    pub(crate) listeners: Table<Listener>,
    pub(crate) mappings: Table<ZoneMapping>,
    pub(crate) routes: Table<ResourceRoute>,
    pub(crate) groups: Table<SecurityGroup>,
    pub(crate) tags: Table<TagRow>,
    pub(crate) monitoring_scroll: usize,
}

impl View {
    fn new(source: LoadBalancer, scope: ReadScope) -> Self {
        Self {
            source,
            tab: Tab::default(),
            read: DetailRead::requested(scope),
            listeners: Table::new(LISTENER_COLUMNS),
            mappings: Table::new(MAPPING_COLUMNS),
            routes: Table::new(ROUTE_COLUMNS),
            groups: Table::new(GROUP_COLUMNS),
            tags: Table::new(TAG_COLUMNS),
            monitoring_scroll: 0,
        }
    }

    fn clear(&mut self) {
        self.listeners.clear();
        self.mappings.clear();
        self.routes.clear();
        self.groups.clear();
        self.tags.clear();
        self.monitoring_scroll = 0;
    }

    fn fill(&mut self) {
        let Some(d) = self.read.detail() else {
            return;
        };
        fn rows<T: Clone>(section: &Result<Vec<T>, String>) -> Vec<T> {
            section.clone().unwrap_or_default()
        }
        self.listeners.rows.items = rows(&d.listeners);
        self.mappings.rows.items = d.load_balancer.zones.clone();
        self.routes.rows.items = rows(&d.resource_map);
        self.groups.rows.items = rows(&d.security_groups);
        self.tags.rows.items = super::tag_rows(&rows(&d.tags));
    }

    /// The table that takes input on the current tab.
    pub(crate) fn table(&self) -> Option<&dyn AnyTable> {
        Some(match self.tab {
            Tab::Listeners => &self.listeners,
            Tab::NetworkMapping => &self.mappings,
            Tab::ResourceMap => &self.routes,
            Tab::Security => &self.groups,
            Tab::Tags => &self.tags,
            Tab::Monitoring | Tab::Attributes => return None,
        })
    }

    fn table_mut(&mut self) -> Option<&mut dyn AnyTable> {
        Some(match self.tab {
            Tab::Listeners => &mut self.listeners,
            Tab::NetworkMapping => &mut self.mappings,
            Tab::ResourceMap => &mut self.routes,
            Tab::Security => &mut self.groups,
            Tab::Tags => &mut self.tags,
            Tab::Monitoring | Tab::Attributes => return None,
        })
    }

    /// Why the current table is unavailable, after a successful read.
    pub(crate) fn section_error(&self) -> Option<&str> {
        let d = self.read.detail()?;
        match self.tab {
            Tab::Listeners => d.listeners.as_ref().err(),
            Tab::ResourceMap => d.resource_map.as_ref().err(),
            Tab::Security => d.security_groups.as_ref().err(),
            Tab::Tags => d.tags.as_ref().err(),
            _ => None,
        }
        .map(String::as_str)
    }
}

/// Embedded at `app.load_balancers_state`.
pub struct State {
    pub(crate) list: Table<LoadBalancer>,
    pub(crate) inventory: ListRead<LoadBalancer>,
    pub view: Option<View>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            list: Table::new(LIST_COLUMNS),
            inventory: ListRead::default(),
            view: None,
        }
    }
}

impl State {
    /// The status of the level on screen.
    pub(crate) fn load_status(&self) -> ReadStatus {
        match &self.view {
            Some(view) => view.read.status(),
            None => self.inventory.status(),
        }
    }
}

pub fn active(app: &App) -> bool {
    app.service_selected && app.current_service == Service::Ec2LoadBalancers
}

fn list_scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "elb:load-balancers")
}

fn view_scope(app: &App) -> Option<ReadScope> {
    let view = app.load_balancers_state.view.as_ref()?;
    Some(ReadScope::from_app(
        app,
        format!("elb:load-balancer:{}", view.source.arn),
    ))
}

/// Queue the list read; clears the rows at once.
pub fn request(app: &mut App) {
    let scope = list_scope(app);
    let state = &mut app.load_balancers_state;
    state.view = None;
    state.inventory.request(&mut state.list.rows, scope);
}

fn open(app: &mut App) {
    let state = &app.load_balancers_state;
    let rows = state.list.filtered();
    let Some(lb) = state.list.rows.get_selected(&rows).map(|l| (*l).clone()) else {
        return;
    };
    if lb.arn.is_empty() {
        return;
    }
    let scope = ReadScope::from_app(app, format!("elb:load-balancer:{}", lb.arn));
    app.load_balancers_state.view = Some(View::new(lb, scope));
    app.update_current_tab_breadcrumb();
}

fn close(app: &mut App) {
    app.load_balancers_state.view = None;
    app.update_current_tab_breadcrumb();
}

/// The ARN `y` copies: the entered load balancer, else the selected row.
pub(crate) fn selected_arn(app: &App) -> Option<String> {
    let state = &app.load_balancers_state;
    match &state.view {
        Some(view) => Some(view.source.arn.clone()),
        None => {
            let rows = state.list.filtered();
            state.list.rows.get_selected(&rows).map(|l| l.arn.clone())
        }
    }
}

fn refresh(app: &mut App) {
    match view_scope(app) {
        None => request(app),
        Some(scope) => {
            let view = app.load_balancers_state.view.as_mut().unwrap();
            if view.read.matches_scope(&scope) {
                view.clear();
                view.read.request(scope);
            } else {
                close(app);
            }
        }
    }
}

/// Esc: cancel a running read, else go up one level.
fn back(app: &mut App) -> bool {
    let state = &mut app.load_balancers_state;
    if let Some(view) = &mut state.view {
        if view.read.is_busy() {
            view.read.cancel();
            view.clear();
        } else {
            close(app);
        }
        return true;
    }
    if state.inventory.is_busy() {
        state.inventory.cancel(&mut state.list.rows);
        return true;
    }
    false
}

fn list_input(app: &mut App, action: &Action) -> bool {
    let selected = selected_arn(app);
    let (state, keys) = keys!(app, load_balancers_state, selected);
    crate::spec_table::input(&mut state.list, keys, action)
}

fn view_input(app: &mut App, action: &Action) -> bool {
    let selected = selected_arn(app);
    let (state, keys) = keys!(app, load_balancers_state, selected);
    let view = state.view.as_mut().unwrap();
    match view.tab {
        Tab::Listeners => crate::spec_table::input(&mut view.listeners, keys, action),
        Tab::NetworkMapping => crate::spec_table::input(&mut view.mappings, keys, action),
        Tab::ResourceMap => crate::spec_table::input(&mut view.routes, keys, action),
        Tab::Security => crate::spec_table::input(&mut view.groups, keys, action),
        Tab::Tags => crate::spec_table::input(&mut view.tags, keys, action),
        // Panes swallow table keys.
        Tab::Monitoring | Tab::Attributes => *keys.mode == Mode::Normal,
    }
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app)
        || !matches!(
            app.mode,
            Mode::Normal | Mode::FilterInput | Mode::ErrorModal
        )
    {
        return false;
    }
    if app.mode == Mode::ErrorModal {
        if super::retry_from_error(app, action) {
            refresh(app);
            return true;
        }
        return false;
    }
    if app.mode == Mode::Normal {
        match action {
            Action::Refresh => {
                refresh(app);
                return true;
            }
            Action::GoBack => return back(app),
            _ if super::global(action) => return false,
            _ => {}
        }
    }
    let Some(view) = app.load_balancers_state.view.as_mut() else {
        if app.mode == Mode::Normal && *action == Action::Select {
            open(app);
            return true;
        }
        if *action == Action::OpenColumnSelector {
            return false;
        }
        return list_input(app, action);
    };
    if app.mode == Mode::Normal {
        let tab = view.tab;
        match action {
            Action::NextDetailTab => {
                view.tab = tab.next();
                return true;
            }
            Action::PrevDetailTab => {
                view.tab = tab.prev();
                return true;
            }
            Action::Yank => {
                let arn = view.source.arn.clone();
                app.clipboard.request_text(&arn);
                return true;
            }
            Action::OpenColumnSelector => return view.table().is_none(),
            Action::NextItem | Action::PageDown | Action::ScrollDown if tab == Tab::Monitoring => {
                view.monitoring_scroll = (view.monitoring_scroll + 1).min(MAX_CHARTS - 1);
                return true;
            }
            Action::PrevItem | Action::PageUp | Action::ScrollUp if tab == Tab::Monitoring => {
                view.monitoring_scroll = view.monitoring_scroll.saturating_sub(1);
                return true;
            }
            _ => {}
        }
    }
    view_input(app, action)
}

/// Drive the read of the level on screen; called for the owning tab only.
pub async fn poll(app: &mut App) -> bool {
    let scope = list_scope(app);
    let config = app.config.clone();
    let state = &mut app.load_balancers_state;
    let changed = state
        .inventory
        .drive(
            &mut state.list.rows,
            &scope,
            "Load balancers read failed",
            || async move {
                ElbClient::new(config)
                    .list_load_balancers()
                    .await
                    .map_err(|e| format!("{e:#}"))
            },
        )
        .await;
    let Some(scope) = view_scope(app) else {
        return changed;
    };
    let config = app.config.clone();
    let view = app.load_balancers_state.view.as_mut().unwrap();
    if !view.read.matches_scope(&scope) {
        // AWS context or owner moved: drop the page with no late repaint.
        close(app);
        return true;
    }
    let arn = view.source.arn.clone();
    let (moved, arrived) = view
        .read
        .drive(&scope, "Load balancer read failed", || async move {
            ElbClient::new(config)
                .load_balancer_details(&arn)
                .await
                .map(Box::new)
                .map_err(|e| format!("{e:#}"))
        })
        .await;
    if arrived {
        view.clear();
        view.fill();
    }
    changed | moved
}

pub fn breadcrumb(app: &App) -> Vec<String> {
    let mut parts = vec!["EC2".to_string(), "Load balancers".to_string()];
    if let Some(view) = &app.load_balancers_state.view {
        parts.push(view.source.name.clone());
    }
    parts
}

/// The console page of the level on screen.
pub fn console_url(app: &App) -> String {
    let region = &app.config.region;
    let url = ConsoleUrl::regional(region, "/ec2/home").query("region", region);
    match &app.load_balancers_state.view {
        Some(view) => url
            .fragment(format!(
                "LoadBalancer:loadBalancerArn={}",
                component(&view.source.arn)
            ))
            .finish(),
        None => url.fragment("LoadBalancers:").finish(),
    }
}

/// The table on screen, for preferences and sort; none on a pane tab.
pub(crate) fn table(app: &App) -> Option<&dyn AnyTable> {
    let state = &app.load_balancers_state;
    match &state.view {
        Some(view) => view.table(),
        None => Some(&state.list),
    }
}

pub(crate) fn table_mut(app: &mut App) -> Option<&mut dyn AnyTable> {
    let state = &mut app.load_balancers_state;
    match &mut state.view {
        Some(view) => view.table_mut(),
        None => Some(&mut state.list),
    }
}
