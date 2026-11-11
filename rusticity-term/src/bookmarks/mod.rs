//! Local resource bookmarks. Selection and persistence never perform AWS reads.
pub mod store;
mod ui;

use crate::{
    app::App,
    keymap::{Action, Mode},
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use store::Bookmark;
pub use ui::render;

#[derive(Default)]
pub struct State {
    pub rows: Vec<Bookmark>,
    pub filter: String,
    pub selected: usize,
    pub expanded: Option<usize>,
    pub filtering: bool,
    pub pending_open: Option<Bookmark>,
    pub opening: bool,
    return_mode: Option<Mode>,
    path: Option<PathBuf>,
    notice: Option<(String, bool, Instant)>,
}

impl State {
    fn path(&self) -> anyhow::Result<PathBuf> {
        self.path
            .clone()
            .map(Ok)
            .unwrap_or_else(store::default_path)
    }

    pub fn filtered(&self) -> Vec<&Bookmark> {
        let filter = crate::common::TextFilter::new(&self.filter);
        self.rows
            .iter()
            .filter(|entry| {
                filter.matches_any([
                    entry.resource.name.as_str(),
                    entry.resource.id.as_str(),
                    entry.resource.arn.as_str(),
                    entry.resource.kind.label(),
                    entry.profile.as_str(),
                    entry.region.as_str(),
                    entry.account_id.as_str(),
                ])
            })
            .collect()
    }

    pub fn notice(&self) -> Option<(&str, bool)> {
        self.notice
            .as_ref()
            .filter(|(_, _, at)| at.elapsed() < Duration::from_secs(4))
            .map(|(message, failed, _)| (message.as_str(), *failed))
    }

    pub fn expire_notice(&mut self) -> bool {
        if self
            .notice
            .as_ref()
            .is_some_and(|(_, _, at)| at.elapsed() >= Duration::from_secs(4))
        {
            self.notice = None;
            true
        } else {
            false
        }
    }

    fn report(&mut self, message: impl Into<String>, failed: bool) {
        self.notice = Some((message.into(), failed, Instant::now()));
    }
}

pub fn available(app: &App) -> bool {
    matches!(app.mode, Mode::Normal | Mode::ResourceFinder) && selected(app).is_ok()
}

fn selected(app: &App) -> anyhow::Result<Bookmark> {
    let resource = if app.mode == Mode::ResourceFinder {
        if app.resource_finder.focus != crate::finder::Focus::Table {
            anyhow::bail!("Select a finder result before bookmarking it");
        }
        app.resource_finder
            .rows
            .get(app.resource_finder.selected)
            .cloned()
    } else {
        crate::finder::related::selected(app)
    }
    .ok_or_else(|| anyhow::anyhow!("No supported resource selected to bookmark"))?;
    if !crate::finder::navigation::has_detail(resource.kind) {
        anyhow::bail!(
            "{} has no dedicated resource view; bookmarking is unavailable",
            resource.kind.label()
        );
    }
    let region = if resource.region.is_empty() || resource.region == "global" {
        app.config.region.clone()
    } else {
        resource.region.clone()
    };
    Bookmark::new(
        app.profile.clone(),
        region,
        app.config.account_id.clone(),
        resource,
    )
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if *action == Action::ToggleBookmark && matches!(app.mode, Mode::Normal | Mode::ResourceFinder)
    {
        let result = selected(app).and_then(|entry| {
            let path = app.bookmarks.path()?;
            let saved = store::toggle(&path, &entry)?;
            app.bookmarks.rows = store::load(&path)?;
            Ok(saved)
        });
        match result {
            Ok(true) => app.bookmarks.report("Resource bookmarked", false),
            Ok(false) => app.bookmarks.report("Bookmark removed", false),
            Err(error) => app
                .bookmarks
                .report(format!("Bookmark failed: {error:#}"), true),
        }
        return true;
    }
    if *action == Action::OpenBookmarkPicker {
        let result = app.bookmarks.path().and_then(|path| store::load(&path));
        match result {
            Ok(rows) => {
                app.bookmarks.rows = rows;
                app.bookmarks.selected = 0;
                app.bookmarks.expanded = None;
                app.bookmarks.filter.clear();
                app.bookmarks.filtering = false;
                app.bookmarks.return_mode = Some(if app.mode == Mode::SpaceMenu {
                    Mode::Normal
                } else {
                    app.mode
                });
                app.mode = Mode::BookmarkPicker;
            }
            Err(error) => app
                .bookmarks
                .report(format!("Cannot load bookmarks: {error:#}"), true),
        }
        return true;
    }
    if app.mode != Mode::BookmarkPicker {
        return false;
    }
    match action {
        Action::CloseMenu | Action::GoBack => {
            if app.bookmarks.filtering {
                app.bookmarks.filtering = false;
            } else {
                app.mode = app.bookmarks.return_mode.take().unwrap_or(Mode::Normal);
            }
        }
        Action::FilterInput('i') if !app.bookmarks.filtering => app.bookmarks.filtering = true,
        Action::NextFilterFocus | Action::PrevFilterFocus => {
            app.bookmarks.filtering = !app.bookmarks.filtering
        }
        Action::ExpandRow if !app.bookmarks.filtering => {
            app.bookmarks.expanded = Some(app.bookmarks.selected)
        }
        Action::CollapseRow => app.bookmarks.expanded = None,
        Action::Refresh => match app.bookmarks.path().and_then(|path| store::load(&path)) {
            Ok(rows) => {
                app.bookmarks.rows = rows;
                app.bookmarks.selected = app
                    .bookmarks
                    .selected
                    .min(app.bookmarks.filtered().len().saturating_sub(1));
                app.bookmarks.expanded = None;
            }
            Err(error) => app
                .bookmarks
                .report(format!("Cannot reload bookmarks: {error:#}"), true),
        },
        Action::StartFilter | Action::EnterFilterMode => app.bookmarks.filtering = true,
        Action::FilterInput(c) if app.bookmarks.filtering => {
            app.bookmarks.filter.push(*c);
            app.bookmarks.selected = 0;
            app.bookmarks.expanded = None;
        }
        Action::FilterBackspace if app.bookmarks.filtering => {
            app.bookmarks.filter.pop();
            app.bookmarks.selected = 0;
            app.bookmarks.expanded = None;
        }
        Action::NextItem | Action::PageDown => {
            let step = if *action == Action::PageDown { 10 } else { 1 };
            app.bookmarks.selected = app
                .bookmarks
                .selected
                .saturating_add(step)
                .min(app.bookmarks.filtered().len().saturating_sub(1));
        }
        Action::PrevItem | Action::PageUp => {
            let step = if *action == Action::PageUp { 10 } else { 1 };
            app.bookmarks.selected = app.bookmarks.selected.saturating_sub(step);
        }
        Action::Select => {
            if app.bookmarks.filtering {
                app.bookmarks.filtering = false;
            } else if let Some(entry) = app
                .bookmarks
                .filtered()
                .get(app.bookmarks.selected)
                .copied()
                .cloned()
            {
                app.bookmarks.pending_open = Some(entry);
                app.bookmarks.opening = true;
            }
        }
        Action::DeleteBookmark if !app.bookmarks.filtering => {
            if let Some(entry) = app
                .bookmarks
                .filtered()
                .get(app.bookmarks.selected)
                .copied()
                .cloned()
            {
                let result = app.bookmarks.path().and_then(|path| {
                    store::remove(&path, &entry)?;
                    store::load(&path)
                });
                match result {
                    Ok(rows) => {
                        app.bookmarks.rows = rows;
                        app.bookmarks.selected = app
                            .bookmarks
                            .selected
                            .min(app.bookmarks.filtered().len().saturating_sub(1));
                        app.bookmarks.report("Bookmark removed", false);
                    }
                    Err(error) => app
                        .bookmarks
                        .report(format!("Cannot remove bookmark: {error:#}"), true),
                }
            }
        }
        _ => {}
    }
    true
}

/// Only an explicit Enter in the picker performs network reads or changes AWS
/// context. Build a replacement off-screen and validate its account first, so
/// failed authentication / wrong-profile resolution leaves the current app intact.
pub async fn open_pending(app: &mut App) -> anyhow::Result<bool> {
    open_with(app, |profile, region| App::new(Some(profile), Some(region))).await
}

async fn open_with<F, Fut>(app: &mut App, connect: F) -> anyhow::Result<bool>
where
    F: FnOnce(String, String) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<App>>,
{
    let Some(entry) = app.bookmarks.pending_open.take() else {
        return Ok(false);
    };
    app.bookmarks.opening = false;
    if !crate::finder::navigation::has_detail(entry.resource.kind) {
        anyhow::bail!(
            "This bookmark has no dedicated resource view. It can be removed from the picker."
        );
    }
    let replacing = app.profile != entry.profile || app.config.region != entry.region;
    if replacing {
        crate::finder::navigation::switch_context(
            app,
            connect(entry.profile.clone(), entry.region.clone()),
            |next| validate_open(next, &entry),
        )
        .await?;
    } else {
        validate_open(app, &entry)?;
    }
    let enter = crate::finder::navigation::open_resource(app, entry.resource).await?;
    app.bookmarks.return_mode = None;
    Ok(enter)
}

fn validate_open(app: &App, entry: &Bookmark) -> anyhow::Result<()> {
    if app.config.account_id != entry.account_id {
        anyhow::bail!(
            "Bookmark expects account {}, but profile {} resolved to {}. No resource was opened.",
            entry.account_id,
            entry.profile,
            app.config.account_id
        );
    }
    crate::finder::navigation::validate_scope(&app.config, &entry.resource)
}

#[cfg(test)]
mod tests;
