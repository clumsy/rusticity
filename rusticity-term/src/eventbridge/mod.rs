//! EventBridge: event buses, then the rules on the entered bus, then one rule
//! with its Event pattern (or Event schedule), Targets, Monitoring and Tags.
//!
//! Each level owns one cancelable read scoped to the tab, the AWS context and
//! the resource. Entering, refreshing or leaving a level clears it at once;
//! nothing older is shown during a reload or after a failed one. Read-only.
use crate::{
    app::{App, Service},
    async_read::{AsyncRead, ReadScope, ReadStatus},
    common::{CyclicEnum, InputFocus},
    console::{component, ConsoleUrl},
    inventory::{self, InventoryInput},
    keymap::{Action, Mode},
    table::TableState,
};
use rusticity_core::eventbridge::{EventTarget, RuleDetail, RULE_METRICS};
use rusticity_core::{EventBridgeClient, EventBus, EventRule};

pub mod view;
pub use view::render;

/// A column: (id and header, width, cell).
pub(crate) type Spec<T> = &'static [(&'static str, u16, fn(&T) -> String)];

/// Console columns. Status and ID belong to Custom Event Bus (V2) buses,
/// which ListEventBuses does not return, so they stay empty for these rows.
pub(crate) const BUS_COLUMNS: Spec<EventBus> = &[
    ("Name", 32, |b| b.name.clone()),
    ("Type", 28, |b| b.bus_type().into()),
    ("Status", 12, |_| String::new()),
    ("ID", 24, |_| String::new()),
    ("Amazon Resource Name (ARN)", 70, |b| b.arn.clone()),
    ("Schema discovery", 18, |b| {
        view::discovery(&b.schema_discovery)
    }),
    ("Description", 40, |b| b.description.clone()),
    ("Created on", 28, |b| view::timestamp(&b.created)),
    ("Last modified", 28, |b| view::timestamp(&b.last_modified)),
    ("Owner account", 16, |b| b.owner_account().into()),
];

pub(crate) const RULE_COLUMNS: Spec<EventRule> = &[
    ("Name", 36, |r| r.name.clone()),
    ("Status", 14, |r| view::state(&r.state)),
    ("Type", 12, |r| r.rule_type().into()),
    ("Description", 40, |r| r.description.clone()),
    ("Managed by", 24, |r| r.managed_by.clone()),
    ("ARN", 70, |r| r.arn.clone()),
];

pub(crate) const TARGET_COLUMNS: Spec<EventTarget> = &[
    ("Target ID", 24, |t| t.id.clone()),
    ("Type", 28, |t| view::target_type(&t.arn)),
    ("ARN", 70, |t| t.arn.clone()),
    ("Input", 40, |t| t.input.clone()),
    ("Role", 60, |t| t.role_arn.clone()),
];

pub(crate) const TAG_COLUMNS: Spec<TagRow> = &[
    ("Key", 32, |t| t.key.clone()),
    ("Value", 60, |t| t.value.clone()),
];

#[derive(Clone, Debug)]
pub struct TagRow {
    pub key: String,
    pub value: String,
}

/// One table with its filter focus and visible columns.
pub(crate) struct Table<T: 'static> {
    pub(crate) rows: TableState<T>,
    pub(crate) visible: Vec<crate::common::ColumnId>,
    pub(crate) focus: InputFocus,
    spec: Spec<T>,
}

impl<T: 'static> Table<T> {
    fn new(spec: Spec<T>) -> Self {
        Self {
            rows: TableState::new(),
            visible: spec.iter().map(|(id, ..)| *id).collect(),
            focus: InputFocus::Filter,
            spec,
        }
    }

    pub(crate) fn spec(&self) -> Spec<T> {
        self.spec
    }

    pub(crate) fn filtered(&self) -> Vec<&T> {
        let filter = self.rows.filter.to_lowercase();
        self.rows
            .items
            .iter()
            .filter(|row| {
                filter.is_empty()
                    || self
                        .spec
                        .iter()
                        .any(|(_, _, cell)| cell(row).to_lowercase().contains(&filter))
            })
            .collect()
    }

    fn clear(&mut self) {
        self.rows.items.clear();
        self.rows.reset();
    }
}

/// Rule tabs, in console order. The first is "Event schedule" for a
/// scheduled rule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Pattern,
    Targets,
    Monitoring,
    Tags,
}

impl CyclicEnum for Tab {
    const ALL: &'static [Self] = &[Self::Pattern, Self::Targets, Self::Monitoring, Self::Tags];
}

impl Tab {
    pub fn name(self, scheduled: bool) -> &'static str {
        match self {
            Self::Pattern if scheduled => "Event schedule",
            Self::Pattern => "Event pattern",
            Self::Targets => "Targets",
            Self::Monitoring => "Monitoring",
            Self::Tags => "Tags",
        }
    }
}

/// The entered rule.
pub struct RuleView {
    /// Identity snapshot of the entered row.
    pub source: EventRule,
    pub tab: Tab,
    detail: Option<Box<RuleDetail>>,
    error: Option<String>,
    read: AsyncRead<Box<RuleDetail>>,
    pub(crate) targets: Table<EventTarget>,
    pub(crate) tags: Table<TagRow>,
    pub(crate) pattern_scroll: usize,
    pub(crate) monitoring_scroll: usize,
}

impl RuleView {
    pub(crate) fn detail(&self) -> Option<&RuleDetail> {
        self.detail.as_deref()
    }

    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub(crate) fn status(&self) -> ReadStatus {
        if self.read.is_busy() {
            self.read.status()
        } else if self.error.is_some() {
            ReadStatus::Failed
        } else if self.detail.is_some() {
            ReadStatus::Ready
        } else {
            ReadStatus::Idle
        }
    }

    fn clear(&mut self) {
        self.detail = None;
        self.error = None;
        self.targets.clear();
        self.tags.clear();
        self.pattern_scroll = 0;
        self.monitoring_scroll = 0;
    }

    pub(crate) fn scheduled(&self) -> bool {
        !self.source.schedule_expression.is_empty()
    }
}

/// The entered event bus.
pub struct BusView {
    pub bus: EventBus,
    pub(crate) rules: Table<EventRule>,
    error: Option<String>,
    read: AsyncRead<Vec<EventRule>>,
    pub rule: Option<RuleView>,
}

impl BusView {
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub(crate) fn status(&self) -> ReadStatus {
        list_status(&self.read, &self.error)
    }
}

fn list_status<T>(read: &AsyncRead<T>, error: &Option<String>) -> ReadStatus {
    if read.is_busy() {
        read.status()
    } else if error.is_some() {
        ReadStatus::Failed
    } else {
        read.status()
    }
}

pub struct State {
    pub(crate) buses: Table<EventBus>,
    error: Option<String>,
    read: AsyncRead<Vec<EventBus>>,
    pub bus: Option<BusView>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            buses: Table::new(BUS_COLUMNS),
            error: None,
            read: AsyncRead::default(),
            bus: None,
        }
    }
}

impl State {
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub(crate) fn status(&self) -> ReadStatus {
        list_status(&self.read, &self.error)
    }

    /// The status of the level on screen.
    pub(crate) fn load_status(&self) -> ReadStatus {
        match &self.bus {
            Some(BusView {
                rule: Some(rule), ..
            }) => rule.status(),
            Some(bus) => bus.status(),
            None => self.status(),
        }
    }

    pub(crate) fn rule(&self) -> Option<&RuleView> {
        self.bus.as_ref()?.rule.as_ref()
    }
}

/// Which table takes input on the current level.
pub(crate) enum Level {
    Buses,
    Rules,
    Rule(Tab),
}

pub(crate) fn level(app: &App) -> Level {
    match &app.eventbridge_state.bus {
        Some(BusView {
            rule: Some(rule), ..
        }) => Level::Rule(rule.tab),
        Some(_) => Level::Rules,
        None => Level::Buses,
    }
}

pub fn active(app: &App) -> bool {
    app.service_selected && app.current_service == Service::EventBridgeEventBuses
}

fn buses_scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "eventbridge:buses")
}

fn rules_scope(app: &App) -> Option<ReadScope> {
    let bus = app.eventbridge_state.bus.as_ref()?;
    Some(ReadScope::from_app(
        app,
        format!("eventbridge:rules:{}", bus.bus.arn),
    ))
}

fn rule_scope(app: &App) -> Option<ReadScope> {
    let rule = app.eventbridge_state.rule()?;
    Some(ReadScope::from_app(
        app,
        format!("eventbridge:rule:{}", rule.source.arn),
    ))
}

/// Queue the bus inventory read; clears the rows at once.
pub fn request(app: &mut App) {
    let scope = buses_scope(app);
    let state = &mut app.eventbridge_state;
    state.buses.clear();
    state.error = None;
    state.bus = None;
    state.read.request(scope);
}

fn open_bus(app: &mut App) {
    let state = &app.eventbridge_state;
    let rows = state.buses.filtered();
    let Some(bus) = state.buses.rows.get_selected(&rows).cloned() else {
        return;
    };
    if bus.name.is_empty() {
        return;
    }
    let scope = ReadScope::from_app(app, format!("eventbridge:rules:{}", bus.arn));
    app.eventbridge_state.bus = Some(BusView {
        bus,
        rules: Table::new(RULE_COLUMNS),
        error: None,
        read: AsyncRead::requested(scope),
        rule: None,
    });
    app.update_current_tab_breadcrumb();
}

fn open_rule(app: &mut App) {
    let Some(bus) = app.eventbridge_state.bus.as_ref() else {
        return;
    };
    let rows = bus.rules.filtered();
    let Some(rule) = bus.rules.rows.get_selected(&rows).cloned() else {
        return;
    };
    if rule.arn.is_empty() {
        return;
    }
    let scope = ReadScope::from_app(app, format!("eventbridge:rule:{}", rule.arn));
    app.eventbridge_state.bus.as_mut().unwrap().rule = Some(RuleView {
        source: rule,
        tab: Tab::Pattern,
        detail: None,
        error: None,
        read: AsyncRead::requested(scope),
        targets: Table::new(TARGET_COLUMNS),
        tags: Table::new(TAG_COLUMNS),
        pattern_scroll: 0,
        monitoring_scroll: 0,
    });
    app.update_current_tab_breadcrumb();
}

/// The ARN of the row or resource Yank copies.
pub(crate) fn selected_arn(app: &App) -> Option<String> {
    let state = &app.eventbridge_state;
    match &state.bus {
        Some(BusView {
            rule: Some(rule), ..
        }) => Some(rule.source.arn.clone()),
        Some(bus) => {
            let rows = bus.rules.filtered();
            bus.rules.rows.get_selected(&rows).map(|r| r.arn.clone())
        }
        None => {
            let rows = state.buses.filtered();
            state.buses.rows.get_selected(&rows).map(|b| b.arn.clone())
        }
    }
}

const CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

/// Run the shared table input on the table `pick` selects.
fn table_input<T: 'static>(
    app: &mut App,
    pick: fn(&mut State) -> Option<&mut Table<T>>,
    action: &Action,
) -> bool {
    let arn = selected_arn(app);
    let App {
        eventbridge_state,
        mode,
        page_input,
        error_message,
        error_scroll,
        clipboard,
        ..
    } = app;
    let Some(table) = pick(eventbridge_state) else {
        return false;
    };
    if *mode == Mode::FilterInput
        && matches!(action, Action::NextFilterFocus | Action::PrevFilterFocus)
    {
        table.focus = if *action == Action::NextFilterFocus {
            table.focus.next(&CONTROLS)
        } else {
            table.focus.prev(&CONTROLS)
        };
        page_input.clear();
        return true;
    }
    if *mode == Mode::FilterInput && *action == Action::DeleteWord {
        if table.focus == InputFocus::Filter {
            table.rows.filter_clear();
        } else {
            page_input.clear();
        }
        return true;
    }
    if *mode == Mode::Normal && *action == Action::StartFilter {
        table.focus = InputFocus::Filter;
        page_input.clear();
        *mode = Mode::FilterInput;
        return true;
    }
    let filtered_count = table.filtered().len();
    inventory::handle_action(
        InventoryInput {
            table: &mut table.rows,
            focus: &mut table.focus,
            mode,
            page_input,
            error_message,
            error_scroll,
            filtered_count,
            selected_arn: arn,
            clipboard,
        },
        action,
    )
}

fn buses_table(state: &mut State) -> Option<&mut Table<EventBus>> {
    Some(&mut state.buses)
}

fn rules_table(state: &mut State) -> Option<&mut Table<EventRule>> {
    state.bus.as_mut().map(|b| &mut b.rules)
}

fn targets_table(state: &mut State) -> Option<&mut Table<EventTarget>> {
    state.bus.as_mut()?.rule.as_mut().map(|r| &mut r.targets)
}

fn tags_table(state: &mut State) -> Option<&mut Table<TagRow>> {
    state.bus.as_mut()?.rule.as_mut().map(|r| &mut r.tags)
}

/// Keys that leave this service alone.
fn global(action: &Action) -> bool {
    matches!(
        action,
        Action::Quit
            | Action::ConfirmQuit
            | Action::CloseService
            | Action::NextTab
            | Action::PrevTab
            | Action::OpenSpaceMenu
            | Action::ShowHelp
            | Action::CopyToClipboard
            | Action::CopyCliCommand
            | Action::OpenCliOutput
            | Action::OpenRelatedResources
            | Action::OpenResourceFinder
            | Action::OpenInBrowser
            | Action::ToggleBookmark
            | Action::OpenBookmarkPicker
            | Action::OpenInConsole
            | Action::OpenColumnSelector
    )
}

fn refresh(app: &mut App) {
    match level(app) {
        Level::Buses => request(app),
        Level::Rules => {
            let scope = rules_scope(app).unwrap();
            let bus = app.eventbridge_state.bus.as_mut().unwrap();
            bus.rules.clear();
            bus.error = None;
            bus.read.request(scope);
        }
        Level::Rule(_) => {
            let scope = rule_scope(app).unwrap();
            let rule = app
                .eventbridge_state
                .bus
                .as_mut()
                .unwrap()
                .rule
                .as_mut()
                .unwrap();
            rule.clear();
            rule.read.request(scope);
        }
    }
}

/// Esc: cancel a running read, else go up one level.
fn back(app: &mut App) -> bool {
    let state = &mut app.eventbridge_state;
    match &mut state.bus {
        Some(bus) if bus.rule.is_some() => bus.rule = None,
        Some(bus) if bus.read.is_busy() => {
            bus.read.cancel();
            bus.rules.clear();
            bus.error = Some("Cancelled — Ctrl+R to retry".into());
            return true;
        }
        Some(_) => state.bus = None,
        None if state.read.is_busy() => {
            state.read.cancel();
            state.buses.clear();
            state.error = Some("Cancelled — Ctrl+R to retry".into());
            return true;
        }
        None => return false,
    }
    app.update_current_tab_breadcrumb();
    true
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
        if *action == Action::RetryLoad {
            app.error_message = None;
            app.error_scroll = 0;
            app.mode = Mode::Normal;
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
            _ if global(action) => return false,
            _ => {}
        }
    }
    match level(app) {
        Level::Buses => {
            if app.mode == Mode::Normal && *action == Action::Select {
                open_bus(app);
                return true;
            }
            table_input(app, buses_table, action)
        }
        Level::Rules => {
            if app.mode == Mode::Normal && *action == Action::Select {
                open_rule(app);
                return true;
            }
            table_input(app, rules_table, action)
        }
        Level::Rule(tab) => {
            if app.mode == Mode::Normal {
                let rule = app
                    .eventbridge_state
                    .bus
                    .as_mut()
                    .unwrap()
                    .rule
                    .as_mut()
                    .unwrap();
                match action {
                    Action::NextDetailTab => {
                        rule.tab = tab.next();
                        return true;
                    }
                    Action::PrevDetailTab => {
                        rule.tab = tab.prev();
                        return true;
                    }
                    Action::Yank => {
                        let arn = rule.source.arn.clone();
                        app.clipboard.request_text(&arn);
                        return true;
                    }
                    Action::NextItem if tab == Tab::Pattern => {
                        rule.pattern_scroll += 1;
                        return true;
                    }
                    Action::PrevItem if tab == Tab::Pattern => {
                        rule.pattern_scroll = rule.pattern_scroll.saturating_sub(1);
                        return true;
                    }
                    Action::NextItem if tab == Tab::Monitoring => {
                        rule.monitoring_scroll =
                            (rule.monitoring_scroll + 1).min(RULE_METRICS.len() - 1);
                        return true;
                    }
                    Action::PrevItem if tab == Tab::Monitoring => {
                        rule.monitoring_scroll = rule.monitoring_scroll.saturating_sub(1);
                        return true;
                    }
                    _ => {}
                }
            }
            match tab {
                Tab::Targets => table_input(app, targets_table, action),
                Tab::Tags => table_input(app, tags_table, action),
                // Summary panes swallow table keys.
                _ => app.mode == Mode::Normal,
            }
        }
    }
}

/// Drive the read of the level on screen; called for the owning tab only.
pub async fn poll(app: &mut App) -> bool {
    let mut changed = false;
    let buses = buses_scope(app);
    let state = &mut app.eventbridge_state;
    if state.read.invalidate_if_changed(&buses) {
        // AWS context moved: drop every level with no late repaint.
        state.buses.clear();
        state.error = None;
        state.bus = None;
        app.update_current_tab_breadcrumb();
        return true;
    }
    if state.read.is_pending() {
        let config = app.config.clone();
        changed |= app.eventbridge_state.read.start(async move {
            EventBridgeClient::new(config)
                .list_event_buses()
                .await
                .map_err(|e| view::clean(&format!("{e:#}")))
        });
    }
    if let Some(result) = app.eventbridge_state.read.take_ready(&buses).await {
        let state = &mut app.eventbridge_state;
        state.buses.clear();
        match result {
            Ok(rows) => {
                state.buses.rows.items = rows;
                state.error = None;
            }
            Err(error) => {
                state.error = Some(view::clean(&error.into_message("Event buses read failed")))
            }
        }
        changed = true;
    }
    if let Some(scope) = rules_scope(app) {
        let bus = app.eventbridge_state.bus.as_mut().unwrap();
        if bus.read.is_pending() {
            let config = app.config.clone();
            let name = bus.bus.name.clone();
            changed |= bus.read.start(async move {
                EventBridgeClient::new(config)
                    .list_rules(&name)
                    .await
                    .map_err(|e| view::clean(&format!("{e:#}")))
            });
        }
        if let Some(result) = bus.read.take_ready(&scope).await {
            bus.rules.clear();
            match result {
                Ok(rows) => {
                    bus.rules.rows.items = rows;
                    bus.error = None;
                }
                Err(error) => {
                    bus.error = Some(view::clean(&error.into_message("Rules read failed")))
                }
            }
            changed = true;
        }
    }
    if let Some(scope) = rule_scope(app) {
        let config = app.config.clone();
        let rule = app
            .eventbridge_state
            .bus
            .as_mut()
            .unwrap()
            .rule
            .as_mut()
            .unwrap();
        if rule.read.is_pending() {
            let arn = rule.source.arn.clone();
            changed |= rule.read.start(async move {
                EventBridgeClient::new(config)
                    .rule_details(&arn)
                    .await
                    .map(Box::new)
                    .map_err(|e| view::clean(&format!("{e:#}")))
            });
        }
        if let Some(result) = rule.read.take_ready(&scope).await {
            rule.clear();
            match result {
                Ok(detail) => {
                    rule.targets.rows.items = detail.targets.clone().unwrap_or_default();
                    rule.tags.rows.items = detail
                        .tags
                        .clone()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(key, value)| TagRow { key, value })
                        .collect();
                    rule.detail = Some(detail);
                }
                Err(error) => {
                    rule.error = Some(view::clean(&error.into_message("Rule read failed")))
                }
            }
            changed = true;
        }
    }
    changed
}

pub fn breadcrumb(app: &App) -> Vec<String> {
    let mut parts = vec!["EventBridge".to_string(), "Event buses".to_string()];
    if let Some(bus) = &app.eventbridge_state.bus {
        parts.push(bus.bus.name.clone());
        if let Some(rule) = &bus.rule {
            parts.push(rule.source.name.clone());
        }
    }
    parts
}

/// The console page of the level on screen.
pub fn console_url(app: &App) -> String {
    let region = &app.config.region;
    let url = ConsoleUrl::regional(region, "/events/home").query("region", region);
    match &app.eventbridge_state.bus {
        Some(BusView {
            bus,
            rule: Some(rule),
            ..
        }) => url
            .fragment(format!(
                "/eventbus/{}/rules/{}",
                component(&bus.name),
                component(&rule.source.name)
            ))
            .finish(),
        Some(bus) => url
            .fragment(format!("/rules?eventbus={}", component(&bus.bus.name)))
            .finish(),
        None => url.fragment("/eventbuses").finish(),
    }
}

/// Columns of the table on screen, for preferences; none on a summary tab.
pub(crate) fn columns(app: &App) -> (Vec<&'static str>, Vec<&'static str>) {
    fn ids<T: 'static>(t: &Table<T>) -> (Vec<&'static str>, Vec<&'static str>) {
        (
            t.spec.iter().map(|(id, ..)| *id).collect(),
            t.visible.clone(),
        )
    }
    let state = &app.eventbridge_state;
    match (&state.bus, level(app)) {
        (_, Level::Buses) => ids(&state.buses),
        (Some(bus), Level::Rules) => ids(&bus.rules),
        (Some(bus), Level::Rule(Tab::Targets)) => ids(&bus.rule.as_ref().unwrap().targets),
        (Some(bus), Level::Rule(Tab::Tags)) => ids(&bus.rule.as_ref().unwrap().tags),
        _ => (Vec::new(), Vec::new()),
    }
}

/// The visible-column list and rows of the table on screen; the bus table on a
/// summary tab, so page size preferences always have a target.
pub(crate) fn visible_mut(app: &mut App) -> &mut Vec<&'static str> {
    let tab = match level(app) {
        Level::Rule(tab) => Some(tab),
        _ => None,
    };
    let state = &mut app.eventbridge_state;
    match (state.bus.as_mut(), tab) {
        (Some(bus), Some(Tab::Targets)) => &mut bus.rule.as_mut().unwrap().targets.visible,
        (Some(bus), Some(Tab::Tags)) => &mut bus.rule.as_mut().unwrap().tags.visible,
        (Some(bus), None) => &mut bus.rules.visible,
        _ => &mut state.buses.visible,
    }
}

pub(crate) fn page_size(app: &App) -> crate::common::PageSize {
    let state = &app.eventbridge_state;
    match (&state.bus, level(app)) {
        (Some(bus), Level::Rule(Tab::Targets)) => bus.rule.as_ref().unwrap().targets.rows.page_size,
        (Some(bus), Level::Rule(Tab::Tags)) => bus.rule.as_ref().unwrap().tags.rows.page_size,
        (Some(bus), _) => bus.rules.rows.page_size,
        _ => state.buses.rows.page_size,
    }
}

pub(crate) fn set_page_size(app: &mut App, size: crate::common::PageSize) {
    let tab = match level(app) {
        Level::Rule(tab) => Some(tab),
        _ => None,
    };
    let state = &mut app.eventbridge_state;
    match (state.bus.as_mut(), tab) {
        (Some(bus), Some(Tab::Targets)) => {
            bus.rule.as_mut().unwrap().targets.rows.set_page_size(size)
        }
        (Some(bus), Some(Tab::Tags)) => bus.rule.as_mut().unwrap().tags.rows.set_page_size(size),
        (Some(bus), _) => bus.rules.rows.set_page_size(size),
        _ => state.buses.rows.set_page_size(size),
    }
}

/// Whether the screen is a table (preferences enabled).
pub(crate) fn is_table(app: &App) -> bool {
    !matches!(level(app), Level::Rule(Tab::Pattern | Tab::Monitoring))
}

#[cfg(test)]
mod tests;
