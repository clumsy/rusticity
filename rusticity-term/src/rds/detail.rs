//! DB instance detail page with a cancelable, read-only read on entry.
//!
//! One read fills every tab: DescribeDBInstances, then bounded parallel reads
//! for Monitoring, Logs & events, Zero-ETL integrations, Data migrations and
//! Recommendations.
//! A refresh clears it at once; nothing older is shown during a reload or
//! after a failed one. Metadata only: no connections, passwords or writes.
use crate::{
    app::{App, Service},
    async_read::{AsyncRead, ReadError, ReadScope, ReadStatus},
    common::{CyclicEnum, InputFocus},
    inventory::{self, InventoryInput},
    keymap::{Action, Mode},
    table::TableState,
};
use rusticity_core::{
    rds::{detail::RdsInstanceDetail, RdsInstance},
    RdsClient,
};

mod view;
pub use view::render;

/// Console tabs, in console order, below the fixed "Summary" pane.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(usize)]
pub enum Tab {
    #[default]
    Connectivity,
    Monitoring,
    LogsEvents,
    Configuration,
    ZeroEtl,
    Maintenance,
    DataMigrations,
    Tags,
    Recommendations,
}

impl CyclicEnum for Tab {
    const ALL: &'static [Self] = &[
        Self::Connectivity,
        Self::Monitoring,
        Self::LogsEvents,
        Self::Configuration,
        Self::ZeroEtl,
        Self::Maintenance,
        Self::DataMigrations,
        Self::Tags,
        Self::Recommendations,
    ];
}

/// Every interactive table, in `View::tables` order. Connectivity & security
/// and Logs & events each hold several under a second ribbon; `f`/`F`
/// switch between those.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Slot {
    ZeroEtl,
    DataMigrations,
    Tags,
    Recommendations,
    Alarms,
    Events,
    Logs,
    Compute,
    SecurityGroupRules,
    Replication,
    IamRoles,
}

const SLOTS: [Slot; 11] = [
    Slot::ZeroEtl,
    Slot::DataMigrations,
    Slot::Tags,
    Slot::Recommendations,
    Slot::Alarms,
    Slot::Events,
    Slot::Logs,
    Slot::Compute,
    Slot::SecurityGroupRules,
    Slot::Replication,
    Slot::IamRoles,
];

/// The Logs & events tables, in console order.
pub(crate) const LOGS_EVENTS: [Slot; 3] = [Slot::Alarms, Slot::Events, Slot::Logs];

/// The Connectivity & security tables, in console order.
pub(crate) const CONNECTIVITY: [Slot; 4] = [
    Slot::Compute,
    Slot::SecurityGroupRules,
    Slot::Replication,
    Slot::IamRoles,
];

impl Tab {
    /// The tables a tab stacks under a second ribbon, and that ribbon index.
    pub(crate) fn stack(self) -> Option<(usize, &'static [Slot])> {
        match self {
            Self::Connectivity => Some((0, &CONNECTIVITY)),
            Self::LogsEvents => Some((1, &LOGS_EVENTS)),
            _ => None,
        }
    }
}

impl Slot {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::ZeroEtl => Tab::ZeroEtl.name(),
            Self::DataMigrations => Tab::DataMigrations.name(),
            Self::Tags => Tab::Tags.name(),
            Self::Recommendations => Tab::Recommendations.name(),
            Self::Alarms => "CloudWatch alarms",
            Self::Events => "Recent events",
            Self::Logs => "Logs",
            Self::Compute => "Connected compute resources",
            Self::SecurityGroupRules => "Security group rules",
            Self::Replication => "Replication",
            Self::IamRoles => "Manage IAM roles",
        }
    }

    pub(crate) fn columns(self) -> &'static [(&'static str, u16)] {
        match self {
            Self::Tags => Tab::TAG_COLUMNS,
            Self::ZeroEtl => Tab::INTEGRATION_COLUMNS,
            Self::DataMigrations => Tab::MIGRATION_COLUMNS,
            Self::Recommendations => Tab::RECOMMENDATION_COLUMNS,
            Self::Alarms => &[("Name", 40), ("State", 20), ("Conditions", 60)],
            Self::Events => &[("Time", 28), ("System notes", 120)],
            Self::Logs => &[("Name", 60), ("Last written", 28), ("Size", 14)],
            Self::Compute => &[
                ("Resource identifier", 26),
                ("Resource type", 18),
                ("Availability Zone", 18),
                ("VPC security group", 36),
                ("Compute resource security group", 36),
                ("Connected proxy", 20),
            ],
            Self::SecurityGroupRules => &[("Security group", 44), ("Type", 30), ("Rule", 44)],
            Self::Replication => &[
                ("DB identifier", 32),
                ("Role", 12),
                ("Region & AZ", 16),
                ("Replication source", 32),
                ("Replication state", 20),
                ("Lag", 14),
            ],
            Self::IamRoles => &[("Role", 72), ("Feature", 24), ("Status", 14)],
        }
    }
}

impl Tab {
    pub fn name(self) -> &'static str {
        match self {
            Self::Connectivity => "Connectivity & security",
            Self::Monitoring => "Monitoring",
            Self::LogsEvents => "Logs & events",
            Self::Configuration => "Configuration",
            Self::ZeroEtl => "Zero-ETL integrations",
            Self::Maintenance => "Maintenance & backups",
            Self::DataMigrations => "Data migrations",
            Self::Tags => "Tags",
            Self::Recommendations => "Recommendations",
        }
    }

    pub(crate) const TAG_COLUMNS: &'static [(&'static str, u16)] = &[("Key", 32), ("Value", 60)];
    pub(crate) const INTEGRATION_COLUMNS: &'static [(&'static str, u16)] = &[
        ("Integration name", 32),
        ("Creation date", 28),
        ("Integration ARN", 60),
        ("Status", 16),
        ("Source", 32),
        ("Source ARN", 60),
        ("Destination AWS account", 24),
        ("Destination type", 28),
        ("Destination ARN", 60),
    ];
    pub(crate) const MIGRATION_COLUMNS: &'static [(&'static str, u16)] = &[
        ("Name", 32),
        ("Status", 16),
        ("Migration progress", 20),
        ("Type", 20),
        ("Server name", 50),
        ("Port", 8),
        ("CloudWatch", 12),
        ("Created", 28),
        ("Migration started", 28),
        ("Migration stopped", 28),
        ("Tables loaded", 14),
        ("Tables loading", 14),
        ("Tables queued", 14),
        ("Tables errored", 14),
        ("Elapsed time", 14),
        ("Homogeneous data migration ARN", 60),
    ];
    pub(crate) const RECOMMENDATION_COLUMNS: &'static [(&'static str, u16)] = &[
        ("Severity", 14),
        ("Detection", 40),
        ("Recommendation", 40),
        ("Impact", 30),
        ("Category", 24),
        ("Status", 12),
        ("Start time", 28),
        ("Last modified", 28),
        ("End time", 28),
        ("Resource identifier", 32),
    ];

    pub(crate) fn is_table(self) -> bool {
        matches!(
            self,
            Self::ZeroEtl
                | Self::DataMigrations
                | Self::Tags
                | Self::Recommendations
                | Self::LogsEvents
                | Self::Connectivity
        )
    }
}

/// One table row; cells follow `Slot::columns` order.
#[derive(Clone, Debug)]
pub struct Row {
    pub cells: Vec<String>,
}

/// One interactive table tab: rows, column preferences and filter focus.
pub(crate) struct Table {
    pub(crate) rows: TableState<Row>,
    pub(crate) visible_column_ids: Vec<crate::common::ColumnId>,
    focus: InputFocus,
}

impl Table {
    fn new(slot: Slot) -> Self {
        Self {
            rows: TableState::new(),
            visible_column_ids: slot.columns().iter().map(|(id, _)| *id).collect(),
            focus: InputFocus::Filter,
        }
    }

    fn filtered(&self) -> Vec<&Row> {
        let filter = self.rows.filter.to_lowercase();
        self.rows
            .items
            .iter()
            .filter(|row| {
                filter.is_empty()
                    || row
                        .cells
                        .iter()
                        .any(|cell| cell.to_lowercase().contains(&filter))
            })
            .collect()
    }
}

pub struct View {
    pub tab: Tab,
    detail: Option<RdsInstanceDetail>,
    error: Option<String>,
    tables: [Table; 11],
    /// Shown table of each stacked tab, by `Tab::stack` index.
    pub(crate) stack_focus: [usize; 2],
    /// First Monitoring chart shown.
    pub(crate) monitoring_scroll: usize,
}

impl View {
    fn new() -> Self {
        Self {
            tab: Tab::Connectivity,
            detail: None,
            error: None,
            tables: SLOTS.map(Table::new),
            stack_focus: [0; 2],
            monitoring_scroll: 0,
        }
    }

    /// The table that takes input on the current tab.
    pub(crate) fn slot(&self) -> Option<Slot> {
        match self.tab {
            Tab::ZeroEtl => Some(Slot::ZeroEtl),
            Tab::DataMigrations => Some(Slot::DataMigrations),
            Tab::Tags => Some(Slot::Tags),
            Tab::Recommendations => Some(Slot::Recommendations),
            tab => tab.stack().map(|(i, slots)| slots[self.stack_focus[i]]),
        }
    }

    pub(crate) fn table(&self, slot: Slot) -> &Table {
        &self.tables[slot as usize]
    }

    /// The current tab table; Tags on a non-table tab, so preferences that
    /// carry no columns still read a page size.
    pub(crate) fn active_table(&self) -> &Table {
        self.table(self.slot().unwrap_or(Slot::Tags))
    }

    pub(crate) fn active_table_mut(&mut self) -> &mut Table {
        let slot = self.slot().unwrap_or(Slot::Tags);
        &mut self.tables[slot as usize]
    }

    /// Columns of the table that takes input; none on a summary tab.
    pub(crate) fn active_columns(&self) -> &'static [(&'static str, u16)] {
        self.slot().map(Slot::columns).unwrap_or(&[])
    }

    pub(crate) fn detail(&self) -> Option<&RdsInstanceDetail> {
        self.detail.as_ref()
    }

    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub(crate) fn status(&self, loading: Option<ReadStatus>) -> ReadStatus {
        if let Some(loading) = loading {
            loading
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
        for table in &mut self.tables {
            table.rows.items.clear();
            table.rows.reset();
        }
        self.monitoring_scroll = 0;
    }

    fn fill(&mut self, detail: RdsInstanceDetail) {
        let s = &detail.sections;
        let rows = |slot: Slot| -> Vec<Row> {
            let cells: Vec<Vec<String>> = match slot {
                Slot::Tags => detail
                    .tags
                    .iter()
                    .map(|(k, v)| vec![k.clone(), v.clone()])
                    .collect(),
                Slot::ZeroEtl => s
                    .integrations
                    .iter()
                    .flatten()
                    .map(|i| {
                        vec![
                            i.name.clone(),
                            view::timestamp(&i.created_time),
                            i.arn.clone(),
                            view::capitalize(&i.status),
                            i.source.clone(),
                            i.source_arn.clone(),
                            i.destination_account.clone(),
                            i.destination_type.clone(),
                            i.destination_arn.clone(),
                        ]
                    })
                    .collect(),
                Slot::DataMigrations => s
                    .migrations
                    .iter()
                    .flatten()
                    .map(view::migration_cells)
                    .collect(),
                Slot::Recommendations => s
                    .recommendations
                    .iter()
                    .flatten()
                    .map(|r| {
                        vec![
                            view::capitalize(&r.severity),
                            r.detection.clone(),
                            r.recommendation.clone(),
                            r.impact.clone(),
                            r.category.clone(),
                            view::capitalize(&r.status),
                            view::timestamp(&r.created_time),
                            view::timestamp(&r.updated_time),
                            view::timestamp(&r.end_time),
                            r.resource_identifier.clone(),
                        ]
                    })
                    .collect(),
                Slot::Alarms => s
                    .alarms
                    .iter()
                    .flatten()
                    .map(|a| vec![a.name.clone(), a.state.clone(), a.condition.clone()])
                    .collect(),
                Slot::Events => s
                    .events
                    .iter()
                    .flatten()
                    .map(|e| vec![view::timestamp(&e.time), e.message.clone()])
                    .collect(),
                Slot::Compute => s
                    .compute
                    .iter()
                    .flatten()
                    .map(|c| {
                        vec![
                            c.identifier.clone(),
                            c.resource_type.clone(),
                            c.availability_zone.clone(),
                            c.db_security_group.clone(),
                            c.compute_security_group.clone(),
                            c.proxy.clone(),
                        ]
                    })
                    .collect(),
                Slot::SecurityGroupRules => s
                    .security_group_rules
                    .iter()
                    .flatten()
                    .map(|r| {
                        vec![
                            r.security_group.clone(),
                            r.rule_type.clone(),
                            r.rule.clone(),
                        ]
                    })
                    .collect(),
                Slot::Replication => s
                    .replication
                    .iter()
                    .flatten()
                    .map(|r| {
                        vec![
                            r.identifier.clone(),
                            r.role.clone(),
                            r.availability_zone.clone(),
                            r.source.clone(),
                            view::capitalize(&r.state),
                            r.lag.clone(),
                        ]
                    })
                    .collect(),
                Slot::IamRoles => detail
                    .iam_roles
                    .iter()
                    .map(|r| {
                        vec![
                            r.role_arn.clone(),
                            r.feature.clone(),
                            view::capitalize(&r.status.to_lowercase()),
                        ]
                    })
                    .collect(),
                Slot::Logs => s
                    .logs
                    .iter()
                    .flatten()
                    .map(|l| {
                        vec![
                            l.name.clone(),
                            view::timestamp(&l.last_written),
                            view::size(l.size),
                        ]
                    })
                    .collect(),
            };
            cells.into_iter().map(|cells| Row { cells }).collect()
        };
        for slot in SLOTS {
            self.tables[slot as usize].rows.items = rows(slot);
        }
        self.detail = Some(detail);
    }

    /// Why the Monitoring section is unavailable, after a successful read.
    pub(crate) fn monitoring_error(&self) -> Option<&str> {
        let s = &self.detail.as_ref()?.sections;
        s.monitoring.as_ref().err().map(String::as_str)
    }

    /// Why a table section is unavailable, after a successful read.
    pub(crate) fn section_error(&self, slot: Slot) -> Option<&str> {
        let s = &self.detail.as_ref()?.sections;
        match slot {
            Slot::ZeroEtl => s.integrations.as_ref().err(),
            Slot::DataMigrations => s.migrations.as_ref().err(),
            Slot::Recommendations => s.recommendations.as_ref().err(),
            Slot::Alarms => s.alarms.as_ref().err(),
            Slot::Events => s.events.as_ref().err(),
            Slot::Logs => s.logs.as_ref().err(),
            Slot::Compute => s.compute.as_ref().err(),
            Slot::SecurityGroupRules => s.security_group_rules.as_ref().err(),
            Slot::Replication => s.replication.as_ref().err(),
            Slot::Tags | Slot::IamRoles => None,
        }
        .map(String::as_str)
    }
}

/// Embedded at `app.rds_state.detail`.
#[derive(Default)]
pub struct State {
    /// Identity snapshot of the entered instance; `Some` while the page is open.
    pub source: Option<RdsInstance>,
    view: Option<View>,
    read: AsyncRead<Box<RdsInstanceDetail>>,
}

impl State {
    pub fn busy(&self) -> bool {
        self.read.is_busy()
    }

    pub(crate) fn loading_status(&self) -> Option<ReadStatus> {
        self.read.is_busy().then(|| self.read.status())
    }

    pub(crate) fn read_status(&self) -> ReadStatus {
        match self.view.as_ref() {
            Some(view) => view.status(self.loading_status()),
            None => ReadStatus::Idle,
        }
    }

    pub fn close(&mut self) {
        *self = State::default();
    }

    pub(crate) fn view(&self) -> Option<&View> {
        self.view.as_ref()
    }

    pub(crate) fn view_mut(&mut self) -> Option<&mut View> {
        self.view.as_mut()
    }
}

pub fn active(app: &App) -> bool {
    app.service_selected
        && app.current_service == Service::RdsInstances
        && app.rds_state.detail.source.is_some()
}

/// Status bar label for `f`/`F` while a tab holds several tables.
pub(crate) fn table_focus_hint(app: &App) -> Option<&'static str> {
    if !active(app) {
        return None;
    }
    let view = app.rds_state.detail.view()?;
    view.tab.stack().map(|_| "next / previous table")
}

fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(
        app,
        app.rds_state
            .detail
            .source
            .as_ref()
            .map(|i| i.arn.clone())
            .unwrap_or_default(),
    )
}

/// Open the selected filtered row. The row is cloned, so the inventory
/// filter, selection and page stay untouched.
fn open(app: &mut App) {
    let rows = crate::ui::rds::filtered_instances(app);
    let Some(instance) = app.rds_state.instances.get_selected(&rows).cloned() else {
        return;
    };
    if instance.arn.is_empty() {
        return;
    }
    let read_scope = ReadScope::from_app(app, instance.arn.clone());
    app.rds_state.detail = State {
        source: Some(instance),
        view: Some(View::new()),
        read: AsyncRead::requested(read_scope),
    };
    app.update_current_tab_breadcrumb();
}

const TABLE_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

/// Filter and pagination input for a table tab, as in every other table.
fn handle_table_input(app: &mut App, action: &Action) -> bool {
    let view = app.rds_state.detail.view.as_mut().unwrap();
    if !view.tab.is_table() {
        return false;
    }
    let table = view.active_table_mut();
    if app.mode == Mode::FilterInput
        && matches!(action, Action::NextFilterFocus | Action::PrevFilterFocus)
    {
        table.focus = if *action == Action::NextFilterFocus {
            table.focus.next(&TABLE_CONTROLS)
        } else {
            table.focus.prev(&TABLE_CONTROLS)
        };
        app.page_input.clear();
        return true;
    }
    if app.mode != Mode::FilterInput {
        return false;
    }
    let filtered_count = table.filtered().len();
    if *action == Action::DeleteWord {
        if table.focus == InputFocus::Filter {
            table.rows.filter_clear();
        } else {
            app.page_input.clear();
        }
        return true;
    }
    inventory::handle_action(
        InventoryInput {
            table: &mut table.rows,
            focus: &mut table.focus,
            mode: &mut app.mode,
            page_input: &mut app.page_input,
            error_message: &mut app.error_message,
            error_scroll: &mut app.error_scroll,
            filtered_count,
            selected_arn: None,
            clipboard: &app.clipboard,
        },
        action,
    );
    true
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !(app.service_selected && app.current_service == Service::RdsInstances) {
        return false;
    }
    if app.rds_state.detail.view.is_none() {
        if app.mode == Mode::Normal && *action == Action::Select {
            open(app);
            return app.rds_state.detail.view.is_some();
        }
        return false;
    }
    if !matches!(app.mode, Mode::Normal | Mode::FilterInput) {
        return false;
    }
    if handle_table_input(app, action) {
        return true;
    }
    if app.mode != Mode::Normal {
        return false;
    }
    let read_scope = scope(app);
    let state = &mut app.rds_state.detail;
    let view = state.view.as_mut().unwrap();
    let tab = view.tab;
    let count = view.active_table().filtered().len();
    let charts = rusticity_core::rds::detail::sections::METRICS.len();
    match action {
        Action::NextDetailTab | Action::PrevDetailTab => {
            view.tab = if *action == Action::NextDetailTab {
                tab.next()
            } else {
                tab.prev()
            };
        }
        Action::NextTableFocus | Action::PrevTableFocus if tab.stack().is_some() => {
            let (i, slots) = tab.stack().unwrap();
            let n = slots.len();
            let focus = &mut view.stack_focus[i];
            *focus = if *action == Action::NextTableFocus {
                (*focus + 1) % n
            } else {
                (*focus + n - 1) % n
            };
        }
        Action::NextItem if tab == Tab::Monitoring => {
            view.monitoring_scroll = (view.monitoring_scroll + 1).min(charts - 1);
        }
        Action::PrevItem if tab == Tab::Monitoring => {
            view.monitoring_scroll = view.monitoring_scroll.saturating_sub(1);
        }
        Action::NextItem if tab.is_table() => view.active_table_mut().rows.next_item(count),
        Action::PrevItem if tab.is_table() => view.active_table_mut().rows.prev_item(),
        Action::PageDown if tab.is_table() => view.active_table_mut().rows.page_down(count),
        Action::PageUp if tab.is_table() => view.active_table_mut().rows.page_up(),
        Action::ExpandRow | Action::Select if tab.is_table() && count > 0 => {
            view.active_table_mut().rows.toggle_expand()
        }
        Action::CollapseRow if tab.is_table() => view.active_table_mut().rows.collapse(),
        Action::StartFilter if tab.is_table() => {
            view.active_table_mut().focus = InputFocus::Filter;
            app.page_input.clear();
            app.mode = Mode::FilterInput;
        }
        Action::Refresh => {
            if state.read.matches_scope(&read_scope) {
                view.clear();
                state.read.request(read_scope);
            } else {
                state.close();
                app.update_current_tab_breadcrumb();
            }
        }
        Action::Yank => {
            if let Some(instance) = state.source.as_ref() {
                app.clipboard.request_text(&instance.arn);
            }
        }
        Action::GoBack => {
            state.close();
            app.update_current_tab_breadcrumb();
        }
        Action::OpenColumnSelector if tab.is_table() => return false,
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
        | Action::OpenInConsole => return false,
        _ => {}
    }
    true
}

/// Drive the read; called by the tab runtime for the owning tab only.
pub async fn poll(app: &mut App) -> bool {
    if app.rds_state.detail.view.is_none() {
        return false;
    }
    let read_scope = scope(app);
    if !app.rds_state.detail.read.matches_scope(&read_scope) {
        // AWS context or owner moved: drop the page with no late repaint.
        app.rds_state.detail.close();
        app.update_current_tab_breadcrumb();
        return true;
    }
    let state = &mut app.rds_state.detail;
    let mut changed = false;
    if state.read.is_pending() {
        let config = app.config.clone();
        let arn = state.source.as_ref().unwrap().arn.clone();
        changed = state.read.start(async move {
            RdsClient::new(config)
                .instance_details(&arn)
                .await
                .map(Box::new)
                .map_err(|error| view::clean(&format!("{error:#}")))
        });
    }
    if let Some(result) = state.read.take_ready(&read_scope).await {
        let view = state.view.as_mut().unwrap();
        view.clear();
        match result {
            Ok(detail) => view.fill(*detail),
            Err(error) => {
                view.error = Some(match error {
                    error @ ReadError::Service(_) => error.into_message("Read unavailable"),
                    error @ ReadError::Task(_) => {
                        view::clean(&error.into_message("Read unavailable"))
                    }
                });
            }
        }
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests;
