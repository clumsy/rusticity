//! Secret detail page with a cancelable, read-only metadata read on entry.
//!
//! One bounded read (DescribeSecret + ListSecretVersionIds) fills all five
//! tabs. A refresh clears every section immediately; nothing stale is shown
//! during a reload or after a failed one. A failed section shows its error,
//! never older data and never a misleading empty result. Metadata only: no
//! secret value is ever requested, and nothing is written.
use crate::{
    app::{App, Service},
    async_read::{AsyncRead, ReadError, ReadScope, ReadStatus},
    common::{CyclicEnum, InputFocus},
    keymap::{Action, Mode},
    table::TableState,
};
use rusticity_core::{
    secretsmanager::detail::{SecretDescription, SecretDetail, SecretVersion},
    SecretMetadata, SecretsManagerClient,
};

mod input;
mod view;
pub use view::render;

/// Detail sub-tabs below the fixed "Secret details" pane. `Overview` and
/// `Rotation` are summary panes; the others are shared tables with filter,
/// pagination and preferences.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(usize)]
pub enum Tab {
    #[default]
    Overview,
    Rotation,
    Versions,
    Replication,
    Tags,
}

impl CyclicEnum for Tab {
    const ALL: &'static [Self] = &[
        Self::Overview,
        Self::Rotation,
        Self::Versions,
        Self::Replication,
        Self::Tags,
    ];
}

impl Tab {
    pub fn name(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Rotation => "Rotation",
            Self::Replication => "Replication",
            Self::Versions => "Versions",
            Self::Tags => "Tags",
        }
    }

    /// One schema drives table rendering and the shared Preferences model.
    pub(crate) fn columns(self) -> &'static [(&'static str, u16)] {
        match self {
            Self::Versions => &[
                ("Version ID", 38),
                ("Staging labels", 32),
                ("Last accessed", 14),
                ("Created on (UTC)", crate::common::UTC_TIMESTAMP_WIDTH),
            ],
            // A failed replica's status message is in its expanded row.
            Self::Replication => &[
                ("Region", 16),
                ("Replication status", 20),
                ("Secret ARN", 72),
                ("Encryption key", 40),
                ("Last accessed date (UTC)", 26),
            ],
            Self::Tags => &[("Key", 32), ("Value", 60)],
            Self::Overview | Self::Rotation => &[],
        }
    }

    pub(crate) fn is_table(self) -> bool {
        !self.columns().is_empty()
    }
}

/// A detail-table row: ordered (label, value) pairs addressed by column name.
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

/// Per-table UI and load state. A section never keeps rows behind an error.
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

    pub(crate) fn section_status(&self, loading: Option<ReadStatus>) -> ReadStatus {
        section_status(loading, self.error.is_some(), self.loaded)
    }

    /// Keep the filter string and column preferences; drop rows and errors.
    fn clear(&mut self) {
        self.loaded = false;
        self.error = None;
        self.table.items.clear();
        self.table.reset();
    }

    fn fill(&mut self, rows: Result<Vec<Row>, String>) {
        self.table.items.clear();
        self.table.reset();
        match rows {
            Ok(rows) => {
                self.error = None;
                self.table.items = rows;
            }
            Err(error) => self.error = Some(error),
        }
        self.loaded = true;
    }
}

/// The open detail view. Metadata is only ever the fresh DescribeSecret
/// result; the inventory snapshot supplies identity (name and ARN) alone.
pub struct View {
    pub tab: Tab,
    metadata: Option<SecretDescription>,
    error: Option<String>,
    tabs: [TabData; 5],
}

impl View {
    fn new() -> Self {
        Self {
            tab: Tab::Overview,
            metadata: None,
            error: None,
            tabs: std::array::from_fn(|index| TabData::new(Tab::ALL[index])),
        }
    }

    pub(crate) fn data(&self) -> &TabData {
        &self.tabs[self.tab as usize]
    }

    pub(crate) fn data_mut(&mut self) -> &mut TabData {
        &mut self.tabs[self.tab as usize]
    }

    pub(crate) fn metadata(&self) -> Option<&SecretDescription> {
        self.metadata.as_ref()
    }

    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Status of the DescribeSecret-backed summary panes.
    pub(crate) fn metadata_status(&self, loading: Option<ReadStatus>) -> ReadStatus {
        section_status(loading, self.error.is_some(), self.metadata.is_some())
    }

    fn clear_all(&mut self) {
        self.metadata = None;
        self.error = None;
        for data in &mut self.tabs {
            data.clear();
        }
    }
}

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

/// Embedded at `app.secretsmanager_state.detail`.
#[derive(Default)]
pub struct State {
    /// Identity snapshot of the entered secret; `Some` while a detail page is open.
    pub source: Option<SecretMetadata>,
    view: Option<View>,
    read: AsyncRead<Box<SecretDetail>>,
}

impl State {
    pub fn busy(&self) -> bool {
        self.read.is_busy()
    }

    pub(crate) fn loading_status(&self) -> Option<ReadStatus> {
        self.read.is_busy().then(|| self.read.status())
    }

    /// Whole-page status: Loading while the read runs, Failed if any section
    /// failed, Ready once fresh metadata arrived, Idle when nothing is entered.
    pub(crate) fn read_status(&self) -> ReadStatus {
        if self.read.is_busy() {
            return self.read.status();
        }
        let Some(view) = self.view.as_ref() else {
            return ReadStatus::Idle;
        };
        if view.error.is_some() || view.tabs.iter().any(|tab| tab.error.is_some()) {
            ReadStatus::Failed
        } else if view.metadata.is_some() {
            ReadStatus::Ready
        } else {
            ReadStatus::Idle
        }
    }

    /// Tear the page down (aborting any read) and return to the inventory.
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

/// True when a secret detail page is open and in focus.
pub fn active(app: &App) -> bool {
    app.service_selected
        && app.current_service == Service::SecretsManagerSecrets
        && app.secretsmanager_state.detail.source.is_some()
}

/// AWS identity, owning tab and entered ARN. A change invalidates the detail.
fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(
        app,
        app.secretsmanager_state
            .detail
            .source
            .as_ref()
            .map(|s| s.arn.clone())
            .unwrap_or_default(),
    )
}

/// Open the detail for the selected filtered row. The row is cloned so the
/// inventory filter, selection and page stay untouched.
fn open(app: &mut App) {
    let rows = crate::ui::secretsmanager::filtered_secrets(app);
    let Some(secret) = app
        .secretsmanager_state
        .secrets
        .get_selected(&rows)
        .cloned()
    else {
        return;
    };
    let read_scope = ReadScope::from_app(app, secret.arn.clone());
    app.secretsmanager_state.detail = State {
        source: Some(secret),
        view: Some(View::new()),
        read: AsyncRead::requested(read_scope),
    };
    app.update_current_tab_breadcrumb();
}

/// Consume detail actions before the inventory handlers. `false` defers shared
/// bindings (preferences, CLI, console, help, tabs) to the parent.
pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !(app.service_selected && app.current_service == Service::SecretsManagerSecrets) {
        return false;
    }
    if app.secretsmanager_state.detail.view.is_none() {
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
    let read_scope = scope(app);
    let state = &mut app.secretsmanager_state.detail;
    let view = state.view.as_mut().unwrap();
    let tab = view.tab;
    let count = view.data().filtered().len();
    match action {
        Action::NextDetailTab | Action::PrevDetailTab => {
            view.tab = if *action == Action::NextDetailTab {
                tab.next()
            } else {
                tab.prev()
            };
            app.update_current_tab_breadcrumb();
        }
        Action::NextItem if tab.is_table() => view.data_mut().table.next_item(count),
        Action::PrevItem if tab.is_table() => view.data_mut().table.prev_item(),
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
            if state.read.matches_scope(&read_scope) {
                view.clear_all();
                state.read.request(read_scope);
            } else {
                state.close();
                app.update_current_tab_breadcrumb();
            }
        }
        Action::Yank => {
            if let Some(secret) = state.source.as_ref() {
                app.clipboard.request_text(&secret.arn);
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

/// Drive the detail read; called by the tab runtime for the owning tab only.
/// Returns true when state changed. No AWS call is made on the input path.
pub async fn poll(app: &mut App) -> bool {
    if app.secretsmanager_state.detail.view.is_none() {
        return false;
    }
    let read_scope = scope(app);
    // AWS context (profile, Region, account, role) or owner moved: drop the
    // detail with no late repaint of its data.
    if !app
        .secretsmanager_state
        .detail
        .read
        .matches_scope(&read_scope)
    {
        app.secretsmanager_state.detail.close();
        app.update_current_tab_breadcrumb();
        return true;
    }
    let state = &mut app.secretsmanager_state.detail;
    let mut changed = false;
    if state.read.is_pending() {
        let config = app.config.clone();
        let arn = state.source.as_ref().unwrap().arn.clone();
        changed = state.read.start(async move {
            SecretsManagerClient::new(config)
                .secret_details(&arn)
                .await
                .map(Box::new)
                .map_err(|error| view::clean(&format!("{error:#}")))
        });
    }
    if let Some(result) = state.read.take_ready(&read_scope).await {
        let view = state.view.as_mut().unwrap();
        match result {
            Ok(detail) => apply(view, *detail),
            Err(error) => {
                let error = match error {
                    error @ ReadError::Service(_) => error.into_message("Read unavailable"),
                    error @ ReadError::Task(_) => {
                        view::clean(&error.into_message("Read unavailable"))
                    }
                };
                // A whole-detail failure fails every section: nothing older is
                // kept and no section claims an empty success.
                view.clear_all();
                view.error = Some(error.clone());
                for data in &mut view.tabs {
                    data.fill(Err(error.clone()));
                }
            }
        }
        changed = true;
    }
    changed
}

/// Replace every section from one fresh result.
fn apply(view: &mut View, detail: SecretDetail) {
    let meta = detail.metadata;
    view.tabs[Tab::Replication as usize].fill(Ok(meta
        .replicas
        .iter()
        .map(|replica| {
            Row(vec![
                ("Region".into(), replica.region.clone()),
                ("Replication status".into(), replica.status.clone()),
                ("Secret ARN".into(), replica_arn(&meta.arn, &replica.region)),
                ("Encryption key".into(), replica.kms_key_id.clone()),
                (
                    "Last accessed date (UTC)".into(),
                    replica.last_accessed_date.clone(),
                ),
                ("Status message".into(), replica.status_message.clone()),
            ])
        })
        .collect()));
    view.tabs[Tab::Tags as usize].fill(Ok(meta
        .tags
        .iter()
        .map(|tag| {
            Row(vec![
                ("Key".into(), tag.key.clone()),
                ("Value".into(), tag.value.clone()),
            ])
        })
        .collect()));
    view.tabs[Tab::Versions as usize].fill(
        detail
            .versions
            .map(|list| list.iter().map(version_row).collect()),
    );
    view.error = None;
    view.metadata = Some(meta);
}

fn version_row(version: &SecretVersion) -> Row {
    Row(vec![
        ("Version ID".into(), version.version_id.clone()),
        ("Staging labels".into(), version.stages.join(", ")),
        ("Last accessed".into(), version.last_accessed_date.clone()),
        (
            "Created on (UTC)".into(),
            crate::common::format_iso_timestamp(&version.created_date),
        ),
    ])
}

/// A replica keeps the primary secret name and suffix in another Region, so
/// its ARN is the primary ARN with the Region segment replaced. ReplicationStatus
/// carries no ARN of its own. Empty when either input is missing or malformed.
fn replica_arn(primary: &str, region: &str) -> String {
    let parts: Vec<_> = primary.splitn(6, ':').collect();
    if parts.len() != 6 || region.is_empty() || parts[3].is_empty() {
        return String::new();
    }
    format!(
        "{}:{}:{}:{region}:{}:{}",
        parts[0], parts[1], parts[2], parts[4], parts[5]
    )
}

#[cfg(test)]
mod tests;
