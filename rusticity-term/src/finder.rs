//! Resource finder state and cancellation. AWS work never runs in input handlers.
use crate::app::App;
use crate::keymap::{Action, Mode};
use rusticity_core::finder::{
    self, Resource, ResourceKind, SearchCache, SearchQuery, SearchUpdate, SharedSearchCache,
    RESULT_LIMIT,
};
use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    sync::{mpsc, Mutex},
    task::JoinHandle,
};

pub mod navigation;
mod related;
const RELATED_TTL: Duration = Duration::from_secs(60);

struct RelatedCache {
    source: Resource,
    at: Instant,
    rows: Vec<Resource>,
}

const DEBOUNCE: Duration = Duration::from_millis(250);

struct RunningSearch {
    task: JoinHandle<()>,
    receiver: mpsc::Receiver<SearchUpdate>,
    generation: u64,
}
impl Drop for RunningSearch {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    #[default]
    Filter,
    Table,
}

impl Focus {
    fn toggle(self) -> Self {
        match self {
            Self::Filter => Self::Table,
            Self::Table => Self::Filter,
        }
    }
}

pub struct State {
    pub focus: Focus,
    pub input: String,
    pub rows: Vec<Resource>,
    pub selected: usize,
    pub searching: bool,
    pub opening: bool,
    pub limited: bool,
    pub provider: String,
    pub warnings: Vec<String>,
    pub pending_open: Option<Resource>,
    pub related_source: Option<Resource>,
    related_rows: Vec<Resource>,
    related_cache: Vec<RelatedCache>,
    query: Option<SearchQuery>,
    changed: Instant,
    generation: u64,
    started: bool,
    running: Option<RunningSearch>,
    scope: String,
    cache: SharedSearchCache,
    seen: HashSet<(finder::ResourceKind, String, String)>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            focus: Focus::Filter,
            input: String::new(),
            rows: vec![],
            selected: 0,
            searching: false,
            opening: false,
            limited: false,
            provider: String::new(),
            warnings: vec![],
            pending_open: None,
            related_source: None,
            related_rows: vec![],
            related_cache: vec![],
            query: None,
            changed: Instant::now(),
            generation: 0,
            started: false,
            running: None,
            scope: String::new(),
            cache: Arc::new(Mutex::new(SearchCache::default())),
            seen: HashSet::new(),
        }
    }
}

impl State {
    fn scope(app: &App) -> String {
        format!(
            "{}|{}|{}|{}",
            app.profile, app.config.region, app.config.account_id, app.config.role_arn
        )
    }
    pub fn cancel(&mut self) {
        self.running = None;
        self.searching = false;
        self.generation = self.generation.wrapping_add(1);
    }
    fn changed(&mut self) {
        if self.related_source.is_some() {
            self.filter_related();
            return;
        }
        self.cancel();
        self.rows.clear();
        self.seen.clear();
        self.selected = 0;
        self.warnings.clear();
        self.provider.clear();
        self.limited = false;
        self.changed = Instant::now();
        self.started = false;
        self.query = match SearchQuery::parse(&self.input) {
            Ok(query) => Some(query),
            Err(error) => {
                self.warnings.push(error.to_string());
                None
            }
        };
    }
    pub fn related_available(app: &App) -> bool {
        related::selected(app).is_some()
    }

    fn filter_related(&mut self) {
        let selected = self.rows.get(self.selected).cloned();
        self.query = SearchQuery::parse(&self.input).ok();
        self.rows = self
            .related_rows
            .iter()
            .filter(|row| self.query.as_ref().is_some_and(|query| query.matches(row)))
            .cloned()
            .collect();
        self.selected = selected
            .and_then(|selected| self.rows.iter().position(|row| *row == selected))
            .unwrap_or(0);
    }

    fn restart_related(&mut self) {
        self.cancel();
        self.rows.clear();
        self.related_rows.clear();
        self.seen.clear();
        self.warnings.clear();
        self.limited = false;
        self.selected = 0;
        self.started = false;
        self.provider = "Explicit resource references".into();
        self.filter_related();
    }

    fn accept(&mut self, generation: u64, update: SearchUpdate) {
        if generation != self.generation {
            return;
        }
        match update {
            SearchUpdate::Provider(provider) => self.provider = provider.into(),
            SearchUpdate::Results(rows) => {
                for row in rows {
                    if self.related_source.as_ref().is_some_and(|source| {
                        !source.arn.is_empty() && source.arn == row.arn && source.kind == row.kind
                    }) {
                        continue;
                    }
                    let count = if self.related_source.is_some() {
                        self.related_rows.len()
                    } else {
                        self.rows.len()
                    };
                    if count >= RESULT_LIMIT {
                        self.limited = true;
                        break;
                    }
                    if self.seen.insert((
                        row.kind,
                        if self.related_source.is_some() && !row.arn.is_empty() {
                            row.arn.clone()
                        } else {
                            row.id.clone()
                        },
                        row.region.clone(),
                    )) {
                        if self.related_source.is_some() {
                            self.related_rows.push(row);
                        } else {
                            self.rows.push(row);
                        }
                    }
                }
                if self.related_source.is_some() {
                    self.filter_related();
                }
                // Append order keeps keyboard selection stable as later sources finish.
            }
            SearchUpdate::Warning(warning) => self.warnings.push(warning),
            SearchUpdate::Finished { limited } => {
                self.searching = false;
                self.limited |= limited;
                if let Some(source) = self
                    .related_source
                    .clone()
                    .filter(|_| self.warnings.is_empty() && !self.limited)
                {
                    self.related_cache
                        .retain(|entry| entry.source != source && entry.at.elapsed() < RELATED_TTL);
                    if self.related_cache.len() >= 16 {
                        self.related_cache.remove(0);
                    }
                    self.related_cache.push(RelatedCache {
                        source,
                        at: Instant::now(),
                        rows: self.related_rows.clone(),
                    });
                }
            }
        }
    }
    pub fn message(&self) -> String {
        if self.opening {
            return "Opening resource...".into();
        }
        if !self.warnings.is_empty() {
            return format!(
                "{} warning(s): {}",
                self.warnings.len(),
                self.warnings.join(" | ")
            );
        }
        if self.related_source.is_some() {
            if let Err(error) = SearchQuery::parse(&self.input) {
                return error.to_string();
            }
            return format!(
                "{} matches / {} linked resources{}{} — {}",
                self.rows.len(),
                self.related_rows.len(),
                if self.searching || !self.started {
                    " ..."
                } else {
                    ""
                },
                if self.limited {
                    " — partial results"
                } else {
                    ""
                },
                self.provider
            );
        }
        if !self.query.as_ref().is_some_and(SearchQuery::ready) {
            return String::new();
        }
        let suffix = if self.searching || !self.started {
            " ..."
        } else {
            ""
        };
        let limited = if self.limited {
            " — partial results; refine your search"
        } else {
            ""
        };
        format!(
            "{} results{} — {}{}",
            self.rows.len(),
            suffix,
            self.provider,
            limited
        )
    }
}

pub fn open(app: &mut App) {
    let scope = State::scope(app);
    if app.resource_finder.scope != scope {
        app.resource_finder = State::default();
        app.resource_finder.scope = scope;
    }
    if app.resource_finder.related_source.take().is_some() {
        app.resource_finder.input.clear();
        app.resource_finder.related_rows.clear();
    }
    app.resource_finder.focus = Focus::Filter;
    app.resource_finder.opening = false;
    app.resource_finder.pending_open = None;
    app.resource_finder.changed();
    app.pending_key = None;
    app.mode = Mode::ResourceFinder;
}

pub fn open_related(app: &mut App) {
    let Some(source) = related::selected(app) else {
        return;
    };
    let scope = State::scope(app);
    if app.resource_finder.scope != scope {
        app.resource_finder = State::default();
        app.resource_finder.scope = scope;
    }
    let finder = &mut app.resource_finder;
    finder.related_source = Some(source.clone());
    finder.input.clear();
    finder.opening = false;
    finder.pending_open = None;
    finder.focus = Focus::Filter;
    finder.restart_related();
    if let Some(entry) = finder
        .related_cache
        .iter()
        .find(|entry| entry.source == source && entry.at.elapsed() < RELATED_TTL)
    {
        finder.related_rows = entry.rows.clone();
        finder.started = true;
        finder.provider = "Explicit resource references (60s cache)".into();
        finder.filter_related();
    }
    app.pending_key = None;
    app.mode = Mode::ResourceFinder;
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if *action == Action::OpenRelatedResources {
        open_related(app);
        return true;
    }
    if *action == Action::OpenResourceFinder {
        open(app);
        return true;
    }
    if app.mode != Mode::ResourceFinder {
        return false;
    }
    let finder = &mut app.resource_finder;
    match action {
        Action::CloseMenu | Action::GoBack => {
            finder.cancel();
            finder.pending_open = None;
            finder.opening = false;
            app.mode = Mode::Normal;
        }
        Action::NextFilterFocus | Action::PrevFilterFocus => finder.focus = finder.focus.toggle(),
        Action::StartFilter | Action::EnterFilterMode => finder.focus = Focus::Filter,
        Action::FilterInput('i' | '/') if finder.focus == Focus::Table => {
            finder.focus = Focus::Filter
        }
        Action::FilterInput(c) if finder.focus == Focus::Filter => {
            finder.input.push(*c);
            finder.changed();
        }
        Action::FilterBackspace if finder.focus == Focus::Filter => {
            finder.input.pop();
            finder.changed();
        }
        Action::DeleteWord if finder.focus == Focus::Filter => {
            finder.input = finder.input.trim_end().to_string();
            let end = finder
                .input
                .rfind(char::is_whitespace)
                .map(|i| i + 1)
                .unwrap_or(0);
            finder.input.truncate(end);
            finder.changed();
        }
        Action::NextItem if finder.focus == Focus::Table => {
            finder.selected = finder
                .selected
                .saturating_add(1)
                .min(finder.rows.len().saturating_sub(1))
        }
        Action::PrevItem if finder.focus == Focus::Table => {
            finder.selected = finder.selected.saturating_sub(1)
        }
        Action::PageDown if finder.focus == Focus::Table => {
            finder.selected = finder
                .selected
                .saturating_add(10)
                .min(finder.rows.len().saturating_sub(1))
        }
        Action::PageUp if finder.focus == Focus::Table => {
            finder.selected = finder.selected.saturating_sub(10)
        }
        Action::Refresh => {
            if finder.related_source.is_some() {
                finder
                    .related_cache
                    .retain(|entry| Some(&entry.source) != finder.related_source.as_ref());
                finder.restart_related();
            } else {
                finder.cache = Arc::new(Mutex::new(SearchCache::default()));
                finder.changed();
            }
        }
        Action::Select if finder.focus == Focus::Filter => finder.focus = Focus::Table,
        Action::FilterInput('y')
            if finder.focus == Focus::Table && finder.related_source.is_some() =>
        {
            if let Some(resource) = finder.rows.get(finder.selected) {
                app.clipboard.request_text(if resource.arn.is_empty() {
                    &resource.id
                } else {
                    &resource.arn
                });
            }
        }
        Action::Select => {
            if let Some(resource) = finder.rows.get(finder.selected).cloned() {
                if resource.kind == ResourceKind::Reference {
                    let warning =
                        "This resource type has no Rusticity view yet; y copies its reference"
                            .to_string();
                    if !finder.warnings.contains(&warning) {
                        finder.warnings.push(warning);
                    }
                    return true;
                }
                if let Err(error) = navigation::validate_scope(&app.config, &resource) {
                    let warning = error.to_string();
                    if !finder.warnings.contains(&warning) {
                        finder.warnings.push(warning);
                    }
                    return true;
                }
                finder.cancel();
                finder.pending_open = Some(resource);
                finder.opening = true;
            }
        }
        _ => {}
    }
    true
}

/// Called from the event loop, including idle ticks. No AWS call is awaited here.
pub fn poll(app: &mut App) {
    if app.mode != Mode::ResourceFinder {
        app.resource_finder.cancel();
        return;
    }
    let scope = State::scope(app);
    if scope != app.resource_finder.scope {
        if app.resource_finder.related_source.is_some() {
            app.resource_finder = State::default();
            app.mode = Mode::Normal;
            return;
        }
        open(app);
    }
    let finder = &mut app.resource_finder;
    if finder.opening {
        return;
    }
    if !finder.started && finder.related_source.is_some() {
        let source = finder.related_source.clone().expect("related source");
        let (tx, receiver) = mpsc::channel(32);
        let task = tokio::spawn(rusticity_core::related::discover(
            app.config.clone(),
            source,
            tx,
        ));
        finder.running = Some(RunningSearch {
            task,
            receiver,
            generation: finder.generation,
        });
        finder.started = true;
        finder.searching = true;
    } else if !finder.started && finder.changed.elapsed() >= DEBOUNCE {
        if let Some(query) = finder.query.clone().filter(SearchQuery::ready) {
            let (tx, receiver) = mpsc::channel(32);
            let config = app.config.clone();
            let cache = finder.cache.clone();
            let task = tokio::spawn(finder::search(config, query, cache, tx));
            finder.running = Some(RunningSearch {
                task,
                receiver,
                generation: finder.generation,
            });
            finder.started = true;
            finder.searching = true;
        }
    }
    // Bound drain work so a busy producer can't monopolize the UI thread.
    for _ in 0..32 {
        let Some(running) = finder.running.as_mut() else {
            break;
        };
        match running.receiver.try_recv() {
            Ok(update) => {
                let generation = running.generation;
                finder.accept(generation, update);
            }
            Err(mpsc::error::TryRecvError::Empty) => break,
            Err(mpsc::error::TryRecvError::Disconnected) => {
                if finder.searching {
                    finder
                        .warnings
                        .push("Search ended unexpectedly; Ctrl+R retries".into());
                }
                finder.searching = false;
                finder.running = None;
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests;
