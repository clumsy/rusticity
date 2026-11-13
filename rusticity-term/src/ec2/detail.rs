//! EC2 instance detail with a cancelable, read-only read on entry.
//!
//! One read fills every tab: DescribeInstances, then bounded parallel reads
//! for the AMI, protection attributes, status checks, alarms, security group
//! rules, volumes and metrics. A refresh clears it at once; nothing older is
//! shown during a reload or after a failed one. The entered instance ID and
//! the tab stay on `ec2_state` (`current_instance`, `detail_tab`), and the
//! Tags tab keeps the shared tags table at `ec2_state.tags`.
use crate::{
    app::{App, Service, ViewMode},
    async_read::{AsyncRead, ReadError, ReadScope, ReadStatus},
    common::{ColumnId, CyclicEnum, InputFocus},
    ec2::tag::InstanceTag,
    inventory::{self, InventoryInput},
    keymap::{Action, Mode},
    table::TableState,
    ui::ec2::{DetailTab as Tab, State as Ec2State},
};
use rusticity_core::{ec2::detail::Ec2InstanceDetail, Ec2Client};

mod view;
pub use view::render;

/// Every interactive table, in `SLOTS` order. Security holds two under a
/// second ribbon; `f`/`F` switch between them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Slot {
    Alarms,
    InboundRules,
    OutboundRules,
    NetworkInterfaces,
    BlockDevices,
}

const SLOTS: [Slot; 5] = [
    Slot::Alarms,
    Slot::InboundRules,
    Slot::OutboundRules,
    Slot::NetworkInterfaces,
    Slot::BlockDevices,
];

/// The Security tables, in console order.
pub(crate) const SECURITY: [Slot; 2] = [Slot::InboundRules, Slot::OutboundRules];

impl Slot {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Alarms => "Alarms",
            Self::InboundRules => "Inbound rules",
            Self::OutboundRules => "Outbound rules",
            Self::NetworkInterfaces => "Network interfaces",
            Self::BlockDevices => "Block devices",
        }
    }

    pub(crate) fn columns(self) -> &'static [(&'static str, u16)] {
        match self {
            Self::Alarms => &[("Name", 40), ("State", 20), ("Conditions", 60)],
            Self::InboundRules => &[
                ("Name", 20),
                ("Security group rule ID", 24),
                ("Port range", 14),
                ("Protocol", 10),
                ("Source", 30),
                ("Security groups", 24),
                ("Description", 30),
            ],
            Self::OutboundRules => &[
                ("Name", 20),
                ("Security group rule ID", 24),
                ("Port range", 14),
                ("Protocol", 10),
                ("Destination", 30),
                ("Security groups", 24),
                ("Description", 30),
            ],
            Self::NetworkInterfaces => &[
                ("Interface ID", 24),
                ("Description", 30),
                ("IPv4 Prefixes", 20),
                ("IPv6 Prefixes", 20),
                ("Public IPv4 address", 18),
                ("Private IPv4 address", 18),
                ("Private IPv4 DNS", 40),
                ("IPv6 addresses", 30),
                ("Attachment time", 26),
                ("Attachment status", 18),
                ("VPC ID", 24),
                ("Subnet ID", 26),
                ("Delete on termination", 22),
                ("Security groups", 30),
                ("Interface type", 16),
            ],
            Self::BlockDevices => &[
                ("Volume ID", 24),
                ("Device name", 14),
                ("Volume size (GiB)", 18),
                ("Attachment status", 18),
                ("Attachment time", 26),
                ("Encrypted", 10),
                ("KMS key ID", 60),
                ("Delete on termination", 22),
            ],
        }
    }
}

/// The tables a tab shows below its pane, in console order.
pub(crate) fn tab_tables(tab: Tab) -> &'static [Slot] {
    match tab {
        Tab::StatusAndAlarms => &[Slot::Alarms],
        Tab::Security => &SECURITY,
        Tab::Networking => &[Slot::NetworkInterfaces],
        Tab::Storage => &[Slot::BlockDevices],
        Tab::Details | Tab::Monitoring | Tab::Tags => &[],
    }
}

/// One table row; cells follow `Slot::columns` order.
#[derive(Clone, Debug)]
pub struct Row {
    pub cells: Vec<String>,
}

/// One interactive table: rows, column preferences and filter focus.
pub(crate) struct Table {
    pub(crate) rows: TableState<Row>,
    pub(crate) visible_column_ids: Vec<ColumnId>,
    focus: InputFocus,
    pub(crate) sort: crate::sort::Choice,
    columns: &'static [(&'static str, u16)],
}

impl Table {
    fn new(slot: Slot) -> Self {
        Self {
            rows: TableState::new(),
            visible_column_ids: slot.columns().iter().map(|(id, _)| *id).collect(),
            focus: InputFocus::Filter,
            sort: Default::default(),
            columns: slot.columns(),
        }
    }

    pub(crate) fn filtered(&self) -> Vec<&Row> {
        let filter = self.rows.filter.to_lowercase();
        let mut rows: Vec<&Row> = self
            .rows
            .items
            .iter()
            .filter(|row| {
                filter.is_empty()
                    || row
                        .cells
                        .iter()
                        .any(|cell| cell.to_lowercase().contains(&filter))
            })
            .collect();
        let columns = self.columns;
        self.sort
            .apply(&mut rows, &self.visible_column_ids, |row, id| {
                columns
                    .iter()
                    .position(|(name, _)| *name == id)
                    .and_then(|i| row.cells.get(i).cloned())
                    .unwrap_or_default()
            });
        rows
    }
}

/// Embedded at `app.ec2_state.detail`.
pub struct State {
    read: AsyncRead<Box<Ec2InstanceDetail>>,
    detail: Option<Ec2InstanceDetail>,
    error: Option<String>,
    tables: [Table; 5],
    /// Shown Security table, an index into `SECURITY`.
    pub(crate) security_focus: usize,
    /// First Monitoring chart shown.
    pub(crate) monitoring_scroll: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            read: AsyncRead::default(),
            detail: None,
            error: None,
            tables: SLOTS.map(Table::new),
            security_focus: 0,
            monitoring_scroll: 0,
        }
    }
}

impl State {
    pub(crate) fn detail(&self) -> Option<&Ec2InstanceDetail> {
        self.detail.as_ref()
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

    pub(crate) fn table(&self, slot: Slot) -> &Table {
        &self.tables[slot as usize]
    }

    pub(crate) fn table_mut(&mut self, slot: Slot) -> &mut Table {
        &mut self.tables[slot as usize]
    }

    /// Why a table section is unavailable, after a successful read.
    pub(crate) fn section_error(&self, slot: Slot) -> Option<&str> {
        let s = &self.detail.as_ref()?.sections;
        match slot {
            Slot::Alarms => s.alarms.as_ref().err(),
            Slot::InboundRules => s.inbound_rules.as_ref().err(),
            Slot::OutboundRules => s.outbound_rules.as_ref().err(),
            Slot::BlockDevices => s.block_devices.as_ref().err(),
            Slot::NetworkInterfaces => None,
        }
        .map(String::as_str)
    }

    fn clear_rows(&mut self) {
        self.detail = None;
        self.error = None;
        for table in &mut self.tables {
            table.rows.items.clear();
            table.rows.reset();
        }
        self.monitoring_scroll = 0;
    }
}

/// The table that takes input on the current tab, if the tab has one.
pub(crate) fn slot(state: &Ec2State) -> Option<Slot> {
    let tables = tab_tables(state.detail_tab);
    match tables.len() {
        0 => None,
        n => Some(tables[state.detail.security_focus.min(n - 1)]),
    }
}

/// True on an entered instance.
pub fn active(app: &App) -> bool {
    app.service_selected
        && app.current_service == Service::Ec2Instances
        && app.ec2_state.current_instance.is_some()
}

/// Status bar label for `f`/`F` while a tab holds several tables.
pub(crate) fn table_focus_hint(app: &App) -> Option<&'static str> {
    (active(app) && tab_tables(app.ec2_state.detail_tab).len() > 1)
        .then_some("next / previous table")
}

fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(
        app,
        app.ec2_state.current_instance.clone().unwrap_or_default(),
    )
}

/// Enter `instance_id` on its Details tab and queue the read. Every tab,
/// Tags included, starts empty.
pub(crate) fn open(app: &mut App, instance_id: String) {
    let state = &mut app.ec2_state;
    state.current_instance = Some(instance_id);
    state.detail_tab = Tab::Details;
    clear_tags(state);
    state.tags.filter.clear();
    state.detail = State::default();
    let read_scope = scope(app);
    app.ec2_state.detail.read.request(read_scope);
    app.view_mode = ViewMode::Detail;
    app.update_current_tab_breadcrumb();
}

/// Leave the instance; its read is dropped with the state.
pub(crate) fn close(app: &mut App) {
    let state = &mut app.ec2_state;
    state.current_instance = None;
    state.detail = State::default();
    clear_tags(state);
    app.view_mode = ViewMode::List;
    app.update_current_tab_breadcrumb();
}

fn clear_tags(state: &mut Ec2State) {
    state.tags.items.clear();
    state.tags.reset();
}

fn clear(state: &mut Ec2State) {
    state.detail.clear_rows();
    clear_tags(state);
}

fn fill(state: &mut Ec2State, detail: Ec2InstanceDetail) {
    state.tags.items = detail
        .tags
        .iter()
        .map(|(key, value)| InstanceTag {
            key: key.clone(),
            value: value.clone(),
        })
        .collect();
    for slot in SLOTS {
        state.detail.tables[slot as usize].rows.items = view::rows(&detail, slot)
            .into_iter()
            .map(|cells| Row { cells })
            .collect();
    }
    state.detail.detail = Some(detail);
}

const TABLE_CONTROLS: [InputFocus; 2] = [InputFocus::Filter, InputFocus::Pagination];

/// Filter and pagination input for a table tab, as in every other table.
fn handle_table_input(app: &mut App, action: &Action) -> bool {
    let Some(slot) = slot(&app.ec2_state) else {
        return false;
    };
    let table = app.ec2_state.detail.table_mut(slot);
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

/// Keys every service keeps for itself.
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
    )
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app) || !matches!(app.mode, Mode::Normal | Mode::FilterInput) {
        return false;
    }
    let tab = app.ec2_state.detail_tab;
    if tab == Tab::Tags {
        // The shared tags table owns its filter and rows; the page keys stay here.
        let page = matches!(
            action,
            Action::NextDetailTab
                | Action::PrevDetailTab
                | Action::Refresh
                | Action::GoBack
                | Action::Yank
        );
        if app.mode != Mode::Normal || !page {
            return false;
        }
    } else if handle_table_input(app, action) {
        return true;
    }
    if app.mode != Mode::Normal || global(action) {
        return false;
    }
    let read_scope = scope(app);
    let slot = slot(&app.ec2_state);
    let state = &mut app.ec2_state;
    let count = slot.map_or(0, |s| state.detail.table(s).filtered().len());
    let charts = rusticity_core::ec2::detail::METRICS.len();
    match action {
        Action::NextDetailTab | Action::PrevDetailTab => {
            state.detail_tab = if *action == Action::NextDetailTab {
                tab.next()
            } else {
                tab.prev()
            };
        }
        Action::NextTableFocus | Action::PrevTableFocus if tab_tables(tab).len() > 1 => {
            let n = tab_tables(tab).len();
            let focus = &mut state.detail.security_focus;
            *focus = if *action == Action::NextTableFocus {
                (*focus + 1) % n
            } else {
                (*focus + n - 1) % n
            };
        }
        Action::NextItem | Action::PageDown | Action::ScrollDown if tab == Tab::Monitoring => {
            let scroll = &mut state.detail.monitoring_scroll;
            *scroll = (*scroll + 1).min(charts - 1);
        }
        Action::PrevItem | Action::PageUp | Action::ScrollUp if tab == Tab::Monitoring => {
            let scroll = &mut state.detail.monitoring_scroll;
            *scroll = scroll.saturating_sub(1);
        }
        Action::OpenColumnSelector if slot.is_some() => return false,
        Action::Refresh => {
            if state.detail.read.matches_scope(&read_scope) {
                clear(state);
                state.detail.read.request(read_scope);
            } else {
                close(app);
            }
        }
        Action::Yank => {
            if let Some(id) = state.current_instance.as_ref() {
                app.clipboard.request_text(id);
            }
        }
        Action::GoBack => close(app),
        _ => {
            if let Some(slot) = slot {
                let rows = &mut state.detail.table_mut(slot).rows;
                match action {
                    Action::NextItem => rows.next_item(count),
                    Action::PrevItem => rows.prev_item(),
                    Action::PageDown => rows.page_down(count),
                    Action::PageUp => rows.page_up(),
                    Action::ExpandRow | Action::Select if count > 0 => rows.toggle_expand(),
                    Action::CollapseRow => rows.collapse(),
                    Action::StartFilter => {
                        state.detail.table_mut(slot).focus = InputFocus::Filter;
                        app.page_input.clear();
                        app.mode = Mode::FilterInput;
                    }
                    _ => {}
                }
            }
        }
    }
    true
}

/// Drive the read; called by the tab runtime for the owning tab only.
pub async fn poll(app: &mut App) -> bool {
    if app.ec2_state.current_instance.is_none() {
        return false;
    }
    let read_scope = scope(app);
    if !app.ec2_state.detail.read.matches_scope(&read_scope) {
        // AWS context or owner moved: drop the page with no late repaint.
        close(app);
        return true;
    }
    let state = &mut app.ec2_state;
    let mut changed = false;
    if state.detail.read.is_pending() {
        let config = app.config.clone();
        let id = state.current_instance.clone().unwrap_or_default();
        changed = state.detail.read.start(async move {
            Ec2Client::new(config)
                .instance_details(&id)
                .await
                .map(Box::new)
                .map_err(|error| view::clean(&format!("{error:#}")))
        });
    }
    if let Some(result) = state.detail.read.take_ready(&read_scope).await {
        clear(state);
        match result {
            Ok(detail) => fill(state, *detail),
            Err(error) => {
                state.detail.error = Some(match error {
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
