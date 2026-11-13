//! EC2 › Target groups: the list, then one target group with its console
//! tabs. Targets with their health, metrics, attributes and tags come from
//! one detail read; see `rusticity_core::elb`.
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
    name_from_arn, ElbClient, Target, TargetGroup, TargetGroupDetail, MAX_CHARTS,
};

/// `web, api`, or `None associated`.
pub(crate) fn load_balancer_names(arns: &[String]) -> String {
    if arns.is_empty() {
        return "None associated".into();
    }
    arns.iter()
        .map(|a| name_from_arn(a))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) const LIST_COLUMNS: Spec<TargetGroup> = &[
    ("Name", 32, |t| t.name.clone()),
    ("ARN", 70, |t| t.arn.clone()),
    ("Port", 8, |t| {
        t.port.map(|p| p.to_string()).unwrap_or_default()
    }),
    ("Protocol", 10, |t| t.protocol.clone()),
    ("Target type", 16, |t| {
        super::target_type_name(&t.target_type)
    }),
    ("Load balancer", 32, |t| {
        load_balancer_names(&t.load_balancer_arns)
    }),
    ("VPC ID", 24, |t| t.vpc_id.clone()),
];

fn port(t: &Target) -> String {
    t.port.map(|p| p.to_string()).unwrap_or_default()
}

fn details(t: &Target) -> String {
    if t.description.is_empty() {
        t.reason.clone()
    } else {
        t.description.clone()
    }
}

fn health(t: &Target) -> String {
    super::health_name(&t.state)
}

fn overridden(t: &Target) -> String {
    super::words(&t.administrative_override)
}

fn anomaly(t: &Target) -> String {
    super::words(&t.anomaly)
}

/// Targets columns by target type, as the console heads them.
const INSTANCE_TARGETS: Spec<Target> = &[
    ("Instance ID", 22, |t| t.id.clone()),
    ("Name", 28, |t| t.name.clone()),
    ("Port", 8, port),
    ("Zone", 14, |t| t.zone.clone()),
    ("Health status", 22, health),
    ("Health status details", 50, details),
    ("Administrative override", 24, overridden),
    ("Anomaly detection result", 24, anomaly),
];
const IP_TARGETS: Spec<Target> = &[
    ("IP address", 40, |t| t.id.clone()),
    ("Port", 8, port),
    ("Zone", 14, |t| t.zone.clone()),
    ("Health status", 22, health),
    ("Health status details", 50, details),
    ("Administrative override", 24, overridden),
    ("Anomaly detection result", 24, anomaly),
];
const LAMBDA_TARGETS: Spec<Target> = &[
    ("Lambda function", 70, |t| t.id.clone()),
    ("Health status", 22, health),
    ("Health status details", 50, details),
];
const ALB_TARGETS: Spec<Target> = &[
    ("Application Load Balancer", 70, |t| t.id.clone()),
    ("Port", 8, port),
    ("Health status", 22, health),
    ("Health status details", 50, details),
];

pub(crate) fn target_columns(target_type: &str) -> Spec<Target> {
    match target_type {
        "ip" => IP_TARGETS,
        "lambda" => LAMBDA_TARGETS,
        "alb" => ALB_TARGETS,
        _ => INSTANCE_TARGETS,
    }
}

/// Console tabs, in console order, below the fixed Details pane.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Targets,
    Monitoring,
    HealthChecks,
    Attributes,
    Tags,
}

impl CyclicEnum for Tab {
    const ALL: &'static [Self] = &[
        Self::Targets,
        Self::Monitoring,
        Self::HealthChecks,
        Self::Attributes,
        Self::Tags,
    ];
}

impl Tab {
    pub fn name(self) -> &'static str {
        match self {
            Self::Targets => "Targets",
            Self::Monitoring => "Monitoring",
            Self::HealthChecks => "Health checks",
            Self::Attributes => "Attributes",
            Self::Tags => "Tags",
        }
    }
}

/// The entered target group.
pub struct View {
    /// Identity snapshot of the entered row.
    pub source: TargetGroup,
    pub tab: Tab,
    pub(crate) read: DetailRead<TargetGroupDetail>,
    pub(crate) targets: Table<Target>,
    pub(crate) tags: Table<TagRow>,
    pub(crate) monitoring_scroll: usize,
}

impl View {
    fn new(source: TargetGroup, scope: ReadScope) -> Self {
        let targets = Table::new(target_columns(&source.target_type));
        Self {
            source,
            tab: Tab::default(),
            read: DetailRead::requested(scope),
            targets,
            tags: Table::new(TAG_COLUMNS),
            monitoring_scroll: 0,
        }
    }

    fn clear(&mut self) {
        self.targets.clear();
        self.tags.clear();
        self.monitoring_scroll = 0;
    }

    fn fill(&mut self) {
        let Some(d) = self.read.detail() else {
            return;
        };
        self.targets.rows.items = d.targets.clone().unwrap_or_default();
        self.tags.rows.items = super::tag_rows(d.tags.as_deref().unwrap_or_default());
    }

    pub(crate) fn table(&self) -> Option<&dyn AnyTable> {
        Some(match self.tab {
            Tab::Targets => &self.targets,
            Tab::Tags => &self.tags,
            _ => return None,
        })
    }

    fn table_mut(&mut self) -> Option<&mut dyn AnyTable> {
        Some(match self.tab {
            Tab::Targets => &mut self.targets,
            Tab::Tags => &mut self.tags,
            _ => return None,
        })
    }

    /// Why the current table is unavailable, after a successful read.
    pub(crate) fn section_error(&self) -> Option<&str> {
        let d = self.read.detail()?;
        match self.tab {
            Tab::Targets => d.targets.as_ref().err(),
            Tab::Tags => d.tags.as_ref().err(),
            _ => None,
        }
        .map(String::as_str)
    }
}

/// Embedded at `app.target_groups_state`.
pub struct State {
    pub(crate) list: Table<TargetGroup>,
    pub(crate) inventory: ListRead<TargetGroup>,
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
    app.service_selected && app.current_service == Service::Ec2TargetGroups
}

fn list_scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "elb:target-groups")
}

fn view_scope(app: &App) -> Option<ReadScope> {
    let view = app.target_groups_state.view.as_ref()?;
    Some(ReadScope::from_app(
        app,
        format!("elb:target-group:{}", view.source.arn),
    ))
}

/// Queue the list read; clears the rows at once.
pub fn request(app: &mut App) {
    let scope = list_scope(app);
    let state = &mut app.target_groups_state;
    state.view = None;
    state.inventory.request(&mut state.list.rows, scope);
}

fn open(app: &mut App) {
    let state = &app.target_groups_state;
    let rows = state.list.filtered();
    let Some(tg) = state.list.rows.get_selected(&rows).map(|t| (*t).clone()) else {
        return;
    };
    if tg.arn.is_empty() {
        return;
    }
    let scope = ReadScope::from_app(app, format!("elb:target-group:{}", tg.arn));
    app.target_groups_state.view = Some(View::new(tg, scope));
    app.update_current_tab_breadcrumb();
}

fn close(app: &mut App) {
    app.target_groups_state.view = None;
    app.update_current_tab_breadcrumb();
}

/// The ARN `y` copies: the entered target group, else the selected row.
pub(crate) fn selected_arn(app: &App) -> Option<String> {
    let state = &app.target_groups_state;
    match &state.view {
        Some(view) => Some(view.source.arn.clone()),
        None => {
            let rows = state.list.filtered();
            state.list.rows.get_selected(&rows).map(|t| t.arn.clone())
        }
    }
}

fn refresh(app: &mut App) {
    match view_scope(app) {
        None => request(app),
        Some(scope) => {
            let view = app.target_groups_state.view.as_mut().unwrap();
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
    let state = &mut app.target_groups_state;
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
    let (state, keys) = keys!(app, target_groups_state, selected);
    crate::spec_table::input(&mut state.list, keys, action)
}

fn view_input(app: &mut App, action: &Action) -> bool {
    let selected = selected_arn(app);
    let (state, keys) = keys!(app, target_groups_state, selected);
    let view = state.view.as_mut().unwrap();
    match view.tab {
        Tab::Targets => crate::spec_table::input(&mut view.targets, keys, action),
        Tab::Tags => crate::spec_table::input(&mut view.tags, keys, action),
        // Panes swallow table keys.
        _ => *keys.mode == Mode::Normal,
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
    let Some(view) = app.target_groups_state.view.as_mut() else {
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
    let state = &mut app.target_groups_state;
    let changed = state
        .inventory
        .drive(
            &mut state.list.rows,
            &scope,
            "Target groups read failed",
            || async move {
                ElbClient::new(config)
                    .list_target_groups()
                    .await
                    .map_err(|e| format!("{e:#}"))
            },
        )
        .await;
    let Some(scope) = view_scope(app) else {
        return changed;
    };
    let config = app.config.clone();
    let view = app.target_groups_state.view.as_mut().unwrap();
    if !view.read.matches_scope(&scope) {
        // AWS context or owner moved: drop the page with no late repaint.
        close(app);
        return true;
    }
    let arn = view.source.arn.clone();
    let (moved, arrived) = view
        .read
        .drive(&scope, "Target group read failed", || async move {
            ElbClient::new(config)
                .target_group_details(&arn)
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
    let mut parts = vec!["EC2".to_string(), "Target groups".to_string()];
    if let Some(view) = &app.target_groups_state.view {
        parts.push(view.source.name.clone());
    }
    parts
}

/// The console page of the level on screen.
pub fn console_url(app: &App) -> String {
    let region = &app.config.region;
    let url = ConsoleUrl::regional(region, "/ec2/home").query("region", region);
    match &app.target_groups_state.view {
        Some(view) => url
            .fragment(format!(
                "TargetGroup:targetGroupArn={}",
                component(&view.source.arn)
            ))
            .finish(),
        None => url.fragment("TargetGroups:").finish(),
    }
}

/// The table on screen, for preferences and sort; none on a pane tab.
pub(crate) fn table(app: &App) -> Option<&dyn AnyTable> {
    let state = &app.target_groups_state;
    match &state.view {
        Some(view) => view.table(),
        None => Some(&state.list),
    }
}

pub(crate) fn table_mut(app: &mut App) -> Option<&mut dyn AnyTable> {
    let state = &mut app.target_groups_state;
    match &mut state.view {
        Some(view) => view.table_mut(),
        None => Some(&mut state.list),
    }
}
