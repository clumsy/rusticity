//! Image detail page with lazy, cancelable, read-only tab data.
use crate::{
    app::{App, Service},
    common::CyclicEnum,
    ecr::image::Image,
    keymap::{Action, Mode},
    table::TableState,
    ui::{self, ecr::Tab},
};
use rusticity_core::{
    ecr::{
        detail::{DetailReport, ImageDetails},
        scan::ScanReport,
    },
    EcrClient,
};
use tokio::task::JoinHandle;

mod input;
mod view;
pub use view::render;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(usize)]
pub enum DetailTab {
    #[default]
    General,
    Scanning,
    Referrers,
    Replication,
    Signing,
}
impl CyclicEnum for DetailTab {
    const ALL: &'static [Self] = &[
        Self::General,
        Self::Scanning,
        Self::Referrers,
        Self::Replication,
        Self::Signing,
    ];
}
impl DetailTab {
    pub fn name(self) -> &'static str {
        match self {
            Self::General => "General information",
            Self::Scanning => "Scanning and vulnerabilities",
            Self::Referrers => "Referrers",
            Self::Replication => "Replication status",
            Self::Signing => "Managed signing status",
        }
    }
    /// One schema drives both table rendering and the shared Preferences model.
    pub(crate) fn columns(self) -> &'static [(&'static str, u16)] {
        match self {
            Self::Scanning => &[("Severity", 14), ("Vulnerability", 26), ("Description", 70)],
            Self::Referrers => &[("Referrer digest", 74), ("Type", 48)],
            Self::Replication => &[
                ("Target account", 18),
                ("Region", 18),
                ("Status", 18),
                ("Error", 40),
            ],
            Self::Signing => &[
                ("Signer profile", 70),
                ("Status", 16),
                ("Failure reason", 40),
                ("Failure code", 24),
            ],
            Self::General => &[],
        }
    }
    fn status_filter(self) -> bool {
        matches!(self, Self::Replication | Self::Signing)
    }
}

#[derive(Clone, Debug)]
pub struct Row(pub Vec<(String, String)>);
impl Row {
    fn field(&self, name: &str) -> &str {
        self.0
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
            .unwrap_or("-")
    }
}
#[derive(Default)]
pub struct TabData {
    loaded: bool,
    error: Option<String>,
    pub(crate) table: TableState<Row>,
    pub(crate) visible_column_ids: Vec<crate::common::ColumnId>,
    focus: crate::common::InputFocus,
}
impl TabData {
    fn new(tab: DetailTab) -> Self {
        Self {
            visible_column_ids: tab.columns().iter().map(|(id, _)| *id).collect(),
            ..Self::default()
        }
    }
    fn clear_results(&mut self) {
        self.loaded = false;
        self.error = None;
        self.table.items.clear();
        self.table.reset();
    }
    fn filtered(&self, tab: DetailTab) -> Vec<&Row> {
        let query = self.table.filter.to_lowercase();
        self.table
            .items
            .iter()
            .filter(|row| {
                if tab.status_filter() {
                    row.field("Status").to_lowercase().contains(&query)
                } else {
                    row.0
                        .iter()
                        .any(|(_, value)| value.to_lowercase().contains(&query))
                }
            })
            .collect()
    }
}

pub struct View {
    pub digest: String,
    pub tab: DetailTab,
    image: Image,
    metadata: Option<ImageDetails>,
    scan: Option<ScanReport>,
    tabs: [TabData; 5],
    /// None selects the immutable digest; Some selects a mutable tag explicitly.
    source_tag: Option<String>,
    source_picker: Option<usize>,
    private: bool,
}
impl View {
    pub(crate) fn selecting_source(&self) -> bool {
        self.source_picker.is_some()
    }
    pub(crate) fn data(&self) -> &TabData {
        &self.tabs[self.tab as usize]
    }
    pub(crate) fn data_mut(&mut self) -> &mut TabData {
        &mut self.tabs[self.tab as usize]
    }
    fn sources(&self) -> Vec<Option<String>> {
        let mut sources = vec![None];
        if let Some(details) = &self.metadata {
            sources.extend(details.tags.iter().cloned().map(Some));
        }
        sources
    }
}

enum Output {
    Metadata(ImageDetails),
    Scan(ScanReport),
    Report(DetailReport),
}
struct Running {
    tab: DetailTab,
    task: JoinHandle<Result<Output, String>>,
}
impl Drop for Running {
    fn drop(&mut self) {
        self.task.abort();
    }
}
#[derive(Default)]
pub struct State {
    pub view: Option<View>,
    pending: Option<DetailTab>,
    running: Option<Running>,
    scope: String,
}
impl State {
    pub fn busy(&self) -> bool {
        self.pending.is_some() || self.running.is_some()
    }
    /// Select a tab, retaining successful data and filters for other tabs.
    fn activate(&mut self, tab: DetailTab, refresh: bool) {
        self.pending = None;
        self.running = None;
        let Some(view) = self.view.as_mut() else {
            return;
        };
        view.tab = tab;
        view.source_picker = None;
        if refresh {
            view.tabs[tab as usize].clear_results();
            if tab == DetailTab::Scanning {
                view.scan = None;
            }
        }
        let digest_missing = view.digest.is_empty();
        let private = view.private;
        let data = view.data_mut();
        if !data.loaded {
            if !private || digest_missing {
                data.loaded = true;
                data.error = Some(if !private {
                    "Image detail reads are supported for private ECR images only".into()
                } else {
                    "Image digest unavailable; return to the image list and refresh".into()
                });
            } else {
                self.pending = Some(tab);
            }
        }
    }
}

pub fn active(app: &App) -> bool {
    app.service_selected
        && app.current_service == Service::EcrRepositories
        && app.ecr_state.current_repository.is_some()
}
fn scope(app: &App) -> String {
    format!(
        "{:?}",
        (
            &app.profile,
            &app.config.region,
            &app.config.account_id,
            &app.config.role_arn,
            &app.ecr_state.current_repository,
            app.ecr_state.tab,
            app.current_tab
        )
    )
}
fn open(app: &mut App) {
    let filtered = ui::ecr::filtered_ecr_images(app);
    let Some(image) = app.ecr_state.images.get_selected(&filtered).cloned() else {
        return;
    };
    app.ecr_state.scan = State {
        view: Some(View {
            digest: image.digest.clone(),
            image,
            tab: DetailTab::General,
            metadata: None,
            scan: None,
            tabs: std::array::from_fn(|index| TabData::new(DetailTab::ALL[index])),
            source_tag: None,
            source_picker: None,
            private: app.ecr_state.tab == Tab::Private,
        }),
        scope: scope(app),
        ..State::default()
    };
    app.ecr_state.scan.activate(DetailTab::General, false);
    app.update_current_tab_breadcrumb();
}

/// Consume page actions before generic inventory loaders/filter handlers.
pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app) {
        return false;
    }
    if app.ecr_state.scan.view.is_none() {
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
    let state = &mut app.ecr_state.scan;
    let view = state.view.as_mut().unwrap();
    let tab = view.tab;
    let count = view.data().filtered(tab).len();
    match action {
        Action::NextDetailTab | Action::PrevDetailTab => {
            let tab = if *action == Action::NextDetailTab {
                tab.next()
            } else {
                tab.prev()
            };
            state.activate(tab, false);
            app.update_current_tab_breadcrumb();
        }
        Action::NextItem => view.data_mut().table.next_item(count),
        Action::PrevItem => view.data_mut().table.prev_item(),
        Action::PageDown => view.data_mut().table.page_down(count),
        Action::PageUp => view.data_mut().table.page_up(),
        Action::ExpandRow | Action::Select if tab != DetailTab::General && count > 0 => {
            view.data_mut().table.toggle_expand()
        }
        Action::CollapseRow => view.data_mut().table.collapse(),
        Action::StartFilter if tab != DetailTab::General => {
            view.data_mut().focus = crate::common::InputFocus::Filter;
            app.page_input.clear();
            app.mode = Mode::FilterInput;
        }
        Action::CycleSortColumn if tab == DetailTab::Replication => {
            view.data_mut().focus = input::SOURCE;
            view.source_picker = Some(
                view.sources()
                    .iter()
                    .position(|s| *s == view.source_tag)
                    .unwrap_or(0),
            );
        }
        Action::Refresh => state.activate(tab, true),
        Action::OpenColumnSelector if !tab.columns().is_empty() => return false,
        Action::Yank => app.clipboard.request_text(view::copy_uri(app)),
        Action::GoBack => {
            app.ecr_state.scan = State::default();
            app.update_current_tab_breadcrumb();
        }
        Action::Quit
        | Action::ConfirmQuit
        | Action::CloseService
        | Action::NextTab
        | Action::PrevTab
        | Action::OpenSpaceMenu
        | Action::ShowHelp
        | Action::CopyToClipboard
        | Action::OpenInConsole => return false,
        _ => {}
    }
    true
}

pub async fn poll(app: &mut App) -> bool {
    if app.ecr_state.scan.view.is_none() {
        return false;
    }
    if !active(app) || app.ecr_state.scan.scope != scope(app) {
        app.ecr_state.scan = State::default();
        app.update_current_tab_breadcrumb();
        return true;
    }
    let state = &mut app.ecr_state.scan;
    let mut changed = false;
    if let Some(tab) = state.pending.take() {
        let config = app.config.clone();
        let repo = app.ecr_state.current_repository.clone().unwrap();
        let view = state.view.as_ref().unwrap();
        let digest = view.digest.clone();
        let tag = view.source_tag.clone();
        state.running = Some(Running {
            tab,
            task: tokio::spawn(async move {
                let client = EcrClient::new(config);
                let result = match tab {
                    DetailTab::General => client
                        .image_details(&repo, &digest)
                        .await
                        .map(Output::Metadata),
                    DetailTab::Scanning => client
                        .image_scan_findings(&repo, &digest)
                        .await
                        .map(Output::Scan),
                    DetailTab::Referrers => client
                        .image_referrers(&repo, &digest)
                        .await
                        .map(Output::Report),
                    DetailTab::Replication => match tag {
                        Some(tag) => client
                            .image_replication_by_tag(&repo, &tag)
                            .await
                            .map(Output::Report),
                        None => client
                            .image_replication(&repo, &digest)
                            .await
                            .map(Output::Report),
                    },
                    DetailTab::Signing => client
                        .image_signing(&repo, &digest)
                        .await
                        .map(Output::Report),
                };
                result.map_err(|error| view::clean(&format!("{error:#}")))
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
        let data = &mut view.tabs[running.tab as usize];
        data.loaded = true;
        match result {
            Ok(Output::Metadata(details)) => {
                if view
                    .source_tag
                    .as_ref()
                    .is_some_and(|tag| !details.tags.contains(tag))
                {
                    view.source_tag = None;
                    view.tabs[DetailTab::Replication as usize].clear_results();
                }
                view.metadata = Some(details);
            }
            Ok(Output::Scan(report)) => {
                data.table.items = report
                    .findings
                    .iter()
                    .map(|finding| {
                        let mut fields = vec![
                            ("Severity".into(), finding.severity.clone()),
                            ("Vulnerability".into(), finding.name.clone()),
                            ("Description".into(), finding.description.clone()),
                        ];
                        fields.extend(finding.details.clone());
                        Row(fields)
                    })
                    .collect();
                data.error = report.warning.clone();
                view.scan = Some(report);
            }
            Ok(Output::Report(report)) => {
                data.table.items = report.entries.into_iter().map(Row).collect();
                data.error = report.warning;
            }
            Err(error) => data.error = Some(error),
        }
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests;
