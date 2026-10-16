//! KMS key detail page with a cancelable, read-only detail read on entry.
//!
//! One bounded read returns independent section results for all five tabs.
//! Successful data stays visible during refresh; failed sections are explicitly
//! marked stale rather than shown as a misleading empty result.
//! Reads only — no key edits, policy edits or rotation writes.
use crate::{
    app::{App, Service},
    common::{CyclicEnum, InputFocus},
    keymap::{Action, Mode},
    kms::key::Key,
    table::TableState,
};
use rusticity_core::{
    kms::detail::{KmsAlias, KmsKeyDetail, KmsRotation, KmsTag},
    KmsClient,
};
use tokio::task::JoinHandle;

mod input;
mod view;
pub use view::render;

/// The detail sub-tabs. `General` and `Rotation` are summary panes; `Aliases`
/// and `Tags` are shared tables with filter/pagination/preferences; `Policy` is
/// a scrollable read-only JSON pane.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(usize)]
pub enum Tab {
    #[default]
    General,
    Aliases,
    Rotation,
    Policy,
    Tags,
}

impl CyclicEnum for Tab {
    const ALL: &'static [Self] = &[
        Self::General,
        Self::Aliases,
        Self::Rotation,
        Self::Policy,
        Self::Tags,
    ];
}

impl Tab {
    pub fn name(self) -> &'static str {
        match self {
            Self::General => "General configuration",
            Self::Aliases => "Aliases",
            Self::Rotation => "Key rotation",
            Self::Policy => "Key policy",
            Self::Tags => "Tags",
        }
    }

    /// One schema drives both table rendering and the shared Preferences model.
    /// Only `Aliases` and `Tags` are table tabs; the others carry no columns and
    /// therefore leak no inventory column preferences.
    pub(crate) fn columns(self) -> &'static [(&'static str, u16)] {
        match self {
            Self::Aliases => &[
                ("Alias", 36),
                ("ARN", 60),
                ("Creation date", crate::common::UTC_TIMESTAMP_WIDTH),
                ("Last updated", crate::common::UTC_TIMESTAMP_WIDTH),
            ],
            Self::Tags => &[("Key", 32), ("Value", 60)],
            _ => &[],
        }
    }

    pub(crate) fn is_table(self) -> bool {
        matches!(self, Self::Aliases | Self::Tags)
    }
}

/// A single detail-table row: ordered (label, value) pairs so the shared table
/// renderer and the expanded-row view can address fields by name.
#[derive(Clone, Debug)]
pub struct Row(pub Vec<(String, String)>);

impl Row {
    pub(crate) fn field(&self, name: &str) -> &str {
        self.0
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
            .unwrap_or("-")
    }
}

/// Per-tab mutable UI + load state. `loaded` distinguishes "not fetched yet"
/// from "fetched and empty"; `error` carries a per-section failure message.
#[derive(Default)]
pub struct TabData {
    loaded: bool,
    error: Option<String>,
    pub(crate) table: TableState<Row>,
    pub(crate) visible_column_ids: Vec<crate::common::ColumnId>,
    focus: InputFocus,
}

impl TabData {
    fn new(tab: Tab) -> Self {
        Self {
            visible_column_ids: tab.columns().iter().map(|(id, _)| *id).collect(),
            ..Self::default()
        }
    }

    fn filtered(&self) -> Vec<&Row> {
        crate::common::filter_by_fields(&self.table.items, &self.table.filter, |row| {
            row.0.iter().map(|(_, value)| value.as_str())
        })
    }
}

/// The open detail view. `source` is a cloned snapshot of the selected key so
/// the view is stable even if the parent inventory selection or filter changes.
pub struct View {
    pub tab: Tab,
    /// Authoritative metadata from the fresh DescribeKey, preferred over the
    /// cloned list snapshot once loaded.
    metadata: Option<rusticity_core::kms::KmsKey>,
    /// Summary metadata panes (General, Rotation) + policy JSON are stored
    /// alongside the per-tab tables; aliases/tags live in `tabs`.
    rotation: Option<KmsRotation>,
    rotation_loaded: bool,
    policy: Option<String>,
    policy_loaded: bool,
    pub(crate) policy_scroll: usize,
    tabs: [TabData; 5],
}

impl View {
    pub(crate) fn data(&self) -> &TabData {
        &self.tabs[self.tab as usize]
    }

    pub(crate) fn data_mut(&mut self) -> &mut TabData {
        &mut self.tabs[self.tab as usize]
    }

    pub(crate) fn rotation(&self) -> Option<&KmsRotation> {
        self.rotation.as_ref()
    }

    pub(crate) fn policy(&self) -> Option<&str> {
        self.policy.as_deref()
    }

    pub(crate) fn metadata(&self) -> Option<&rusticity_core::kms::KmsKey> {
        self.metadata.as_ref()
    }
}

/// Output of an async section fetch, routed back to the right tab on completion.
enum Output {
    Detail(Box<KmsKeyDetail>),
}

struct Running {
    task: JoinHandle<Result<Output, String>>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// `pub State` is the build interface the parent embeds at `app.kms_state.detail`.
#[derive(Default)]
pub struct State {
    /// `Some` when a key detail page is open.
    pub source: Option<Key>,
    view: Option<View>,
    pending: bool,
    running: Option<Running>,
    scope: String,
}

impl State {
    /// True while a background section fetch is scheduled or in flight.
    pub fn busy(&self) -> bool {
        self.pending || self.running.is_some()
    }

    /// Tear the detail page down and return to the parent inventory.
    pub fn close(&mut self) {
        *self = State::default();
    }

    pub(crate) fn view(&self) -> Option<&View> {
        self.view.as_ref()
    }

    pub(crate) fn view_mut(&mut self) -> Option<&mut View> {
        self.view.as_mut()
    }

    /// Select a tab, retaining successful data and filters for other tabs. All
    /// section data lives in one fetch, so switching tabs never reloads unless
    /// an explicit refresh was requested.
    fn activate(&mut self, tab: Tab, refresh: bool) {
        let Some(view) = self.view.as_mut() else {
            return;
        };
        view.tab = tab;
        if refresh {
            // Keep successful data, filter/selection and preferences while a
            // new read runs. Failed sections are marked stale on completion.
            for data in &mut view.tabs {
                data.error = None;
            }
            self.running = None;
            self.pending = true;
        }
    }
}

/// True when a KMS key detail page is open and in focus.
pub fn active(app: &App) -> bool {
    app.service_selected
        && app.current_service == Service::KmsKeys
        && app.kms_state.detail.source.is_some()
}

/// A stable fingerprint of the context the detail belongs to. When it changes —
/// profile, region, account, role, or the user leaving KMS (except via the CLI
/// output tabs) — the detail is torn down on the next poll with no late repaint.
fn scope(app: &App) -> String {
    format!(
        "{:?}",
        (
            &app.profile,
            &app.config.region,
            &app.config.account_id,
            &app.config.role_arn,
            app.kms_state
                .detail
                .source
                .as_ref()
                .map(|k| k.key_arn.clone()),
        )
    )
}

/// Open the detail page for the currently selected inventory key. The key is
/// cloned so the parent inventory's selection/filter/tab remain untouched.
fn open(app: &mut App) {
    let filtered = crate::ui::kms::filtered_kms_keys(app);
    let Some(key) = app.kms_state.keys.get_selected(&filtered).cloned() else {
        return;
    };
    let mut state = State {
        source: Some(key),
        view: Some(View {
            tab: Tab::General,
            metadata: None,
            rotation: None,
            rotation_loaded: false,
            policy: None,
            policy_loaded: false,
            policy_scroll: 0,
            tabs: std::array::from_fn(|index| TabData::new(Tab::ALL[index])),
        }),
        pending: true,
        running: None,
        scope: String::new(),
    };
    state.scope = {
        // scope() reads app.kms_state.detail.source, which is not set yet; build
        // it from the same tuple against the key we just cloned.
        format!(
            "{:?}",
            (
                &app.profile,
                &app.config.region,
                &app.config.account_id,
                &app.config.role_arn,
                state.source.as_ref().map(|k| k.key_arn.clone()),
            )
        )
    };
    app.kms_state.detail = state;
    app.update_current_tab_breadcrumb();
}

/// Consume detail page actions before the generic inventory loaders / filter
/// handlers run. Returns `false` to defer to the parent (so common bindings like
/// `p`/`Ctrl+y`/`Ctrl+e`/`Ctrl+o`/`Ctrl+p` still work), `true` when handled here.
pub fn handle_action(app: &mut App, action: &Action) -> bool {
    // Gate on being in the KMS keys service (inventory OR detail) — `active`
    // means the detail is already open, which cannot gate opening it.
    if !(app.service_selected && app.current_service == Service::KmsKeys) {
        return false;
    }
    // Not open yet: an Enter in normal mode on the inventory opens the detail
    // for the filtered, cloned selected key.
    if app.kms_state.detail.view.is_none() {
        if app.mode == Mode::Normal && *action == Action::Select {
            open(app);
            return true;
        }
        return false;
    }
    if !matches!(app.mode, Mode::Normal | Mode::FilterInput) {
        return false;
    }
    if input::handle_action(app, action) {
        return true;
    }
    if app.mode != Mode::Normal {
        return false;
    }
    let state = &mut app.kms_state.detail;
    let view = state.view.as_mut().unwrap();
    let tab = view.tab;
    let count = view.data().filtered().len();
    match action {
        Action::NextDetailTab | Action::PrevDetailTab => {
            let next = if *action == Action::NextDetailTab {
                tab.next()
            } else {
                tab.prev()
            };
            state.activate(next, false);
            app.update_current_tab_breadcrumb();
        }
        Action::NextItem if tab.is_table() => view.data_mut().table.next_item(count),
        Action::PrevItem if tab.is_table() => view.data_mut().table.prev_item(),
        Action::NextItem if tab == Tab::Policy => {
            view.policy_scroll = view
                .policy_scroll
                .saturating_add(1)
                .min(policy_max_scroll(view));
        }
        Action::PrevItem if tab == Tab::Policy => {
            view.policy_scroll = view.policy_scroll.saturating_sub(1);
        }
        Action::PageDown if tab == Tab::Policy => {
            view.policy_scroll = view
                .policy_scroll
                .saturating_add(10)
                .min(policy_max_scroll(view));
        }
        Action::PageUp if tab == Tab::Policy => {
            view.policy_scroll = view.policy_scroll.saturating_sub(10);
        }
        Action::PageDown if tab.is_table() => view.data_mut().table.page_down(count),
        Action::PageUp if tab.is_table() => view.data_mut().table.page_up(),
        Action::ExpandRow | Action::Select if tab.is_table() && count > 0 => {
            view.data_mut().table.toggle_expand()
        }
        Action::CollapseRow if tab.is_table() => view.data_mut().table.collapse(),
        Action::StartFilter if tab.is_table() => {
            view.data_mut().focus = InputFocus::Filter;
            app.page_input.clear();
            app.mode = Mode::FilterInput;
        }
        Action::Refresh => state.activate(tab, true),
        // Preferences only exist for the table tabs; defer to the parent so the
        // shared column selector opens. Non-table tabs own `p` as a no-op defer.
        Action::OpenColumnSelector if tab.is_table() => return false,
        // `y` copies the key ARN on every detail tab. If the parent handles it
        // first via return-false it must behave the same; we handle it directly.
        Action::Yank => {
            if let Some(key) = state.source.as_ref() {
                app.clipboard.request_text(&key.key_arn);
            }
        }
        Action::GoBack => {
            state.close();
            app.update_current_tab_breadcrumb();
        }
        // Everything else (console, breadcrumb, CLI tabs, help, quit, tab
        // switch, space menu) belongs to the parent.
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
        | Action::OpenColumnSelector
        | Action::OpenInConsole => return false,
        _ => {}
    }
    true
}

/// Drive the async section fetch. Returns `true` when state changed (so the
/// caller repaints). Performs no AWS calls on the input path — only here.
pub async fn poll(app: &mut App) -> bool {
    if app.kms_state.detail.view.is_none() {
        return false;
    }
    // Context moved out from under the detail (scope/profile/region/account/
    // role change, or left KMS): drop it with no late repaint of stale data.
    if !app.service_selected
        || !matches!(
            app.current_service,
            Service::KmsKeys | Service::CliOutput(_)
        )
        || app.kms_state.detail.scope != scope(app)
    {
        app.kms_state.detail.close();
        app.update_current_tab_breadcrumb();
        return true;
    }
    let state = &mut app.kms_state.detail;
    let mut changed = false;
    if state.pending {
        state.pending = false;
        let config = app.config.clone();
        let key_arn = state.source.as_ref().unwrap().key_arn.clone();
        state.running = Some(Running {
            task: tokio::spawn(async move {
                let client = KmsClient::new(config);
                client
                    .key_details(&key_arn)
                    .await
                    .map(|detail| Output::Detail(Box::new(detail)))
                    .map_err(|error| view::clean(&format!("{error:#}")))
            }),
        });
        changed = true;
    }
    if state
        .running
        .as_ref()
        .is_some_and(|running| running.task.is_finished())
    {
        let mut running = state.running.take().unwrap();
        let result = match (&mut running.task).await {
            Ok(result) => result,
            Err(error) => Err(format!(
                "Read unavailable: {}",
                view::clean(&error.to_string())
            )),
        };
        let view = state.view.as_mut().unwrap();
        match result {
            Ok(Output::Detail(detail)) => apply_detail(view, *detail),
            Err(error) => {
                // A whole-detail failure shows on every section, but previously
                // loaded data (if any) is retained and marked visibly stale.
                for data in &mut view.tabs {
                    if !data.loaded {
                        data.error = Some(error.clone());
                        data.loaded = true;
                    } else {
                        data.error = Some(format!("Stale — refresh failed: {error}"));
                    }
                }
                if view.rotation.is_none() {
                    view.rotation_loaded = true;
                }
                if view.policy.is_none() {
                    view.policy_loaded = true;
                }
            }
        }
        changed = true;
    }
    changed
}

/// Fan one worker result out across the tab sections. Failed refreshes retain
/// only data from the same entered key and clearly label it as stale.
fn apply_detail(view: &mut View, detail: KmsKeyDetail) {
    view.metadata = Some(detail.metadata);
    let aliases = &mut view.tabs[Tab::Aliases as usize];
    match detail.aliases {
        Ok(items) => {
            aliases.error = None;
            aliases.table.items = items.iter().map(alias_row).collect();
        }
        Err(error) => aliases.error = Some(section_error(aliases.loaded, error)),
    }
    aliases.loaded = true;
    let tags = &mut view.tabs[Tab::Tags as usize];
    match detail.tags {
        Ok(items) => {
            tags.error = None;
            tags.table.items = items.iter().map(tag_row).collect();
        }
        Err(error) => tags.error = Some(section_error(tags.loaded, error)),
    }
    tags.loaded = true;
    let general = &mut view.tabs[Tab::General as usize];
    general.loaded = true;
    general.error = None;
    let rotation_tab = &mut view.tabs[Tab::Rotation as usize];
    match detail.rotation {
        Ok(rotation) => {
            rotation_tab.error = None;
            view.rotation = Some(rotation);
        }
        Err(error) => rotation_tab.error = Some(section_error(view.rotation.is_some(), error)),
    }
    rotation_tab.loaded = true;
    view.rotation_loaded = true;
    let policy_tab = &mut view.tabs[Tab::Policy as usize];
    match detail.policy {
        Ok(policy) => {
            policy_tab.error = None;
            // Pretty print valid JSON; retain readable text if AWS returns a
            // non-JSON document. Strip terminal controls but preserve newlines.
            let text = serde_json::from_str::<serde_json::Value>(&policy)
                .ok()
                .and_then(|value| serde_json::to_string_pretty(&value).ok())
                .unwrap_or(policy);
            view.policy = Some(
                text.chars()
                    .map(|c| {
                        if c.is_control() && c != '\n' && c != '\t' {
                            ' '
                        } else {
                            c
                        }
                    })
                    .collect(),
            );
        }
        Err(error) => policy_tab.error = Some(section_error(view.policy.is_some(), error)),
    }
    policy_tab.loaded = true;
    view.policy_loaded = true;
    view.policy_scroll = view.policy_scroll.min(policy_max_scroll(view));
}

fn section_error(had_data: bool, error: String) -> String {
    if had_data {
        format!("Stale — refresh failed: {error}")
    } else {
        error
    }
}

fn policy_max_scroll(view: &View) -> usize {
    view.policy
        .as_deref()
        .map_or(0, |policy| policy.lines().count().saturating_sub(1))
}

fn alias_row(alias: &KmsAlias) -> Row {
    Row(vec![
        ("Alias".into(), alias.name.clone()),
        ("ARN".into(), alias.arn.clone()),
        (
            "Creation date".into(),
            crate::common::format_iso_timestamp(&alias.creation_date),
        ),
        (
            "Last updated".into(),
            crate::common::format_iso_timestamp(&alias.last_updated_date),
        ),
    ])
}

fn tag_row(tag: &KmsTag) -> Row {
    Row(vec![
        ("Key".into(), tag.key.clone()),
        ("Value".into(), tag.value.clone()),
    ])
}

#[cfg(test)]
mod tests;
