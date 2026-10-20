//! KMS key detail page with a cancelable, read-only detail read on entry.
//!
//! One bounded read returns independent section results for all five tabs.
//! A refresh clears every section immediately; nothing stale is ever shown
//! during a reload or after a failed one. A failed section shows an explicit
//! error, never the previous successful data and never a misleading empty
//! result. Reads only — no key edits, policy edits or rotation writes.
use crate::{
    app::{App, Service},
    async_read::{AsyncRead, ReadScope, ReadStatus},
    common::{CyclicEnum, InputFocus},
    keymap::{Action, Mode},
    kms::key::Key,
    table::TableState,
};
use rusticity_core::{
    kms::detail::{KmsAlias, KmsKeyDetail, KmsRotation, KmsTag},
    KmsClient,
};

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
/// A section never retains data behind an error: a failed section is cleared.
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

    /// This section's own render state, given the shared read's status while a
    /// fetch is in flight. A section that succeeded is Ready even when another
    /// section failed; a failed section is Failed (its error is shown, never
    /// stale data); a not-yet-loaded section is Idle.
    pub(crate) fn section_status(&self, loading: Option<ReadStatus>) -> ReadStatus {
        if let Some(loading) = loading {
            loading
        } else if self.error.is_some() {
            ReadStatus::Failed
        } else if self.loaded {
            ReadStatus::Ready
        } else {
            ReadStatus::Idle
        }
    }

    /// Reset a section to "not loaded" with no data or error. Column
    /// preferences and the filter string survive; the rows do not.
    fn clear(&mut self) {
        self.loaded = false;
        self.error = None;
        self.table.items.clear();
        self.table.reset();
    }
}

/// The open detail view. `source` is a cloned snapshot of the selected key so
/// the view is stable even if the parent inventory selection or filter changes.
pub struct View {
    pub tab: Tab,
    /// Authoritative metadata from the fresh DescribeKey. The General pane shows
    /// ONLY the initial key-identity snapshot (key id/ARN/alias) before this
    /// arrives, never the stale list metadata for mutable fields.
    metadata: Option<rusticity_core::kms::KmsKey>,
    general_error: Option<String>,
    /// Summary metadata panes (General, Rotation) + policy JSON are stored
    /// alongside the per-tab tables; aliases/tags live in `tabs`.
    rotation: Option<KmsRotation>,
    rotation_loaded: bool,
    rotation_error: Option<String>,
    policy: Option<String>,
    policy_loaded: bool,
    policy_error: Option<String>,
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

    /// The General summary pane's own render state, given the shared loading
    /// status while a fetch runs. Failed shows the error with no stale fields.
    pub(crate) fn general_status(&self, loading: Option<ReadStatus>) -> ReadStatus {
        section_status(
            loading,
            self.general_error.is_some(),
            self.metadata.is_some(),
        )
    }

    pub(crate) fn rotation_status(&self, loading: Option<ReadStatus>) -> ReadStatus {
        section_status(loading, self.rotation_error.is_some(), self.rotation_loaded)
    }

    pub(crate) fn policy_status(&self, loading: Option<ReadStatus>) -> ReadStatus {
        section_status(loading, self.policy_error.is_some(), self.policy_loaded)
    }

    pub(crate) fn general_error(&self) -> Option<&str> {
        self.general_error.as_deref()
    }

    pub(crate) fn rotation_error(&self) -> Option<&str> {
        self.rotation_error.as_deref()
    }

    pub(crate) fn policy_error(&self) -> Option<&str> {
        self.policy_error.as_deref()
    }

    /// Clear every section for a refresh: drop all fetched data and errors so
    /// nothing stale survives into the reload. Only tab selection, per-tab
    /// filter strings and column preferences are retained.
    fn clear_all(&mut self) {
        self.metadata = None;
        self.general_error = None;
        self.rotation = None;
        self.rotation_loaded = false;
        self.rotation_error = None;
        self.policy = None;
        self.policy_loaded = false;
        self.policy_error = None;
        self.policy_scroll = 0;
        for data in &mut self.tabs {
            data.clear();
        }
    }
}

/// Shared section-status rule: Loading while the shared read runs, else Failed
/// on this section's error, else Ready once loaded, else Idle.
fn section_status(loading: Option<ReadStatus>, has_error: bool, loaded: bool) -> ReadStatus {
    if let Some(loading) = loading {
        loading
    } else if has_error {
        ReadStatus::Failed
    } else if loaded {
        ReadStatus::Ready
    } else {
        ReadStatus::Idle
    }
}

/// Output of an async section fetch, routed back to the right tab on completion.
enum Output {
    Detail(Box<KmsKeyDetail>),
}

/// `pub State` is the build interface the parent embeds at `app.kms_state.detail`.
#[derive(Default)]
pub struct State {
    /// `Some` when a key detail page is open.
    pub source: Option<Key>,
    view: Option<View>,
    read: AsyncRead<Output>,
}

impl State {
    /// True while a background section fetch is scheduled or in flight.
    pub fn busy(&self) -> bool {
        self.read.is_busy()
    }

    /// The shared read's loading status while a fetch runs, else None. Section
    /// renderers use it so every pane shows one consistent elapsed spinner.
    pub(crate) fn loading_status(&self) -> Option<ReadStatus> {
        self.read.is_busy().then(|| self.read.status())
    }

    /// Whole-page read state, never inferred from row counts. Loading while the
    /// read runs; Failed if ANY section failed; Ready once the fetch completed
    /// (metadata present); Idle when no key is entered. Specific sections may
    /// still render Ready via their own `section_status` even when another
    /// section failed.
    pub(crate) fn read_status(&self) -> ReadStatus {
        if self.read.is_busy() {
            return self.read.status();
        }
        let Some(view) = self.view.as_ref() else {
            return ReadStatus::Idle;
        };
        let any_error = view.general_error.is_some()
            || view.rotation_error.is_some()
            || view.policy_error.is_some()
            || view.tabs.iter().any(|tab| tab.error.is_some());
        if any_error {
            ReadStatus::Failed
        } else if view.metadata.is_some() {
            ReadStatus::Ready
        } else {
            ReadStatus::Idle
        }
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

    /// Select a tab. Tab selection, per-tab filters and column preferences are
    /// retained; a refresh clears ALL section data immediately so nothing stale
    /// is shown during or after the reload.
    fn activate(&mut self, tab: Tab, refresh: Option<ReadScope>) {
        let Some(view) = self.view.as_mut() else {
            return;
        };
        view.tab = tab;
        if let Some(scope) = refresh {
            view.clear_all();
            // Supersede any in-flight/finished read, re-queueing the same scope.
            self.read.request(scope);
        }
    }
}

/// True when a KMS key detail page is open and in focus.
pub fn active(app: &App) -> bool {
    app.service_selected
        && app.current_service == Service::KmsKeys
        && app.kms_state.detail.source.is_some()
}

/// A stable fingerprint of the context the detail belongs to: AWS identity plus
/// the entered key ARN, independent of the highlighted inventory row. When it
/// changes — profile, region, account, role, or the entered key — the detail is
/// invalidated on the next poll with no late repaint of stale data.
fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(
        app,
        app.kms_state
            .detail
            .source
            .as_ref()
            .map(|k| k.key_arn.clone())
            .unwrap_or_default(),
    )
}

/// Open the detail page for the currently selected inventory key. The key is
/// cloned so the parent inventory's selection/filter/tab remain untouched.
fn open(app: &mut App) {
    let filtered = crate::ui::kms::filtered_kms_keys(app);
    let Some(key) = app.kms_state.keys.get_selected(&filtered).cloned() else {
        return;
    };
    // scope() reads app.kms_state.detail.source, which is not set yet; build the
    // ReadScope from the same tuple against the key we just cloned.
    let read_scope = ReadScope::from_app(app, key.key_arn.clone());
    app.kms_state.detail = State {
        source: Some(key),
        view: Some(View {
            tab: Tab::General,
            metadata: None,
            general_error: None,
            rotation: None,
            rotation_loaded: false,
            rotation_error: None,
            policy: None,
            policy_loaded: false,
            policy_error: None,
            policy_scroll: 0,
            tabs: std::array::from_fn(|index| TabData::new(Tab::ALL[index])),
        }),
        read: AsyncRead::requested(read_scope),
    };
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
    // The entered key fixes the read scope; compute it before the mutable borrow
    // so a refresh can re-queue against it.
    let read_scope = scope(app);
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
            state.activate(next, None);
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
        Action::Refresh => {
            if !state.read.matches_scope(&read_scope) {
                state.close();
                app.update_current_tab_breadcrumb();
            } else {
                state.activate(tab, Some(read_scope));
            }
        }
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
    // The detail-close is a view policy; `AsyncRead` only owns the read.
    if !app.service_selected
        || !matches!(
            app.current_service,
            Service::KmsKeys | Service::CliOutput(_)
        )
        || !app.kms_state.detail.read.matches_scope(&scope(app))
    {
        app.kms_state.detail.close();
        app.update_current_tab_breadcrumb();
        return true;
    }
    let read_scope = scope(app);
    let state = &mut app.kms_state.detail;
    let mut changed = false;
    // Do not clone request inputs or construct a read on idle poll cycles.
    if state.read.is_pending() {
        let config = app.config.clone();
        let key_arn = state.source.as_ref().unwrap().key_arn.clone();
        changed = state.read.start(async move {
            let client = KmsClient::new(config);
            client
                .key_details(&key_arn)
                .await
                .map(|detail| Output::Detail(Box::new(detail)))
                .map_err(|error| view::clean(&format!("{error:#}")))
        });
    }
    if let Some(result) = state.read.take_ready(&read_scope).await {
        // Preserve existing wording: the service error passes through verbatim
        // (already sanitized when produced); a crashed/cancelled task keeps the
        // "Read unavailable: …" prefix and is sanitized here.
        let result = match result {
            Ok(output) => Ok(output),
            Err(error @ crate::async_read::ReadError::Service(_)) => {
                Err(error.into_message("Read unavailable"))
            }
            Err(error @ crate::async_read::ReadError::Task(_)) => {
                Err(view::clean(&error.into_message("Read unavailable")))
            }
        };
        let view = state.view.as_mut().unwrap();
        match result {
            Ok(Output::Detail(detail)) => apply_detail(view, *detail),
            Err(error) => {
                // A whole-detail failure clears every section and shows the
                // error on each: no previously loaded data is retained behind a
                // failed read, and no section is left as a misleading empty.
                view.metadata = None;
                view.general_error = Some(error.clone());
                view.rotation = None;
                view.rotation_loaded = true;
                view.rotation_error = Some(error.clone());
                view.policy = None;
                view.policy_loaded = true;
                view.policy_error = Some(error.clone());
                view.policy_scroll = 0;
                for data in &mut view.tabs {
                    data.table.items.clear();
                    data.table.reset();
                    data.loaded = true;
                    data.error = Some(error.clone());
                }
            }
        }
        changed = true;
    }
    changed
}

/// Fan one worker result out across the tab sections. Each section is replaced
/// wholesale: a successful section shows fresh data, a failed section is cleared
/// and shows its own error — never the previous data, never a misleading empty.
fn apply_detail(view: &mut View, detail: KmsKeyDetail) {
    view.metadata = Some(detail.metadata);
    view.general_error = None;
    let aliases = &mut view.tabs[Tab::Aliases as usize];
    match detail.aliases {
        Ok(items) => {
            aliases.error = None;
            aliases.table.items = items.iter().map(alias_row).collect();
            aliases.table.reset();
        }
        Err(error) => {
            aliases.error = Some(error);
            aliases.table.items.clear();
            aliases.table.reset();
        }
    }
    aliases.loaded = true;
    let tags = &mut view.tabs[Tab::Tags as usize];
    match detail.tags {
        Ok(items) => {
            tags.error = None;
            tags.table.items = items.iter().map(tag_row).collect();
            tags.table.reset();
        }
        Err(error) => {
            tags.error = Some(error);
            tags.table.items.clear();
            tags.table.reset();
        }
    }
    tags.loaded = true;
    match detail.rotation {
        Ok(rotation) => {
            view.rotation_error = None;
            view.rotation = Some(rotation);
        }
        Err(error) => {
            view.rotation_error = Some(error);
            view.rotation = None;
        }
    }
    view.rotation_loaded = true;
    match detail.policy {
        Ok(policy) => {
            view.policy_error = None;
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
        Err(error) => {
            view.policy_error = Some(error);
            view.policy = None;
        }
    }
    view.policy_loaded = true;
    view.policy_scroll = view.policy_scroll.min(policy_max_scroll(view));
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
