//! Dedicated CLI output tabs. Results and commands stay in memory, never sessions.
mod run;
use crate::{
    app::{App, Service, Tab},
    async_read::{AsyncRead, ReadScope, ReadStatus},
    cli_command::{self, Invocation},
    keymap::{Action, Mode},
    tab_runtime::TabId,
    ui,
};
use crossterm::event::KeyEvent;
use ratatui::{
    prelude::*,
    widgets::{Paragraph, Wrap},
};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct State {
    views: BTreeMap<u64, View>,
    next_id: u64,
}
struct View {
    /// The stable identity of the TAB this output belongs to, allocated before
    /// the tab is built. A background completion is accepted only against this
    /// owner, never against whichever tab currently has focus.
    owner: TabId,
    invocation: Option<Invocation>,
    text: String,
    failed: bool,
    scroll: usize,
    lines: usize,
    read: AsyncRead<String>,
}
impl State {
    pub(crate) fn retain_tabs(&mut self, tabs: &[Tab]) {
        self.views.retain(|id, view| {
            tabs.iter()
                .any(|tab| tab.id == view.owner && tab.service == Service::CliOutput(*id))
        });
    }
    pub fn busy(&self) -> bool {
        self.views.values().any(|v| v.read.is_busy())
    }
    /// View-facing read state for one output tab, so the parent's status worker
    /// can report a CLI tab without reaching into its private fields. A failed
    /// capture reports Failed; otherwise the shared read's status drives it.
    pub(crate) fn status_for(&self, output_id: u64) -> ReadStatus {
        match self.views.get(&output_id) {
            Some(view) if view.failed => ReadStatus::Failed,
            Some(view) => view.read.status(),
            None => ReadStatus::Idle,
        }
    }
}
/// Invalidate before drawing a frame with a changed AWS identity.
pub(crate) fn invalidate_context(app: &mut App) {
    let context = ReadScope::from_app(app, "").with_owner(None);
    for view in app.cli_output.views.values_mut() {
        let scope = context.clone().with_owner(Some(view.owner));
        if !view.read.matches_scope(&scope) {
            view.read.request(scope);
            view.read.cancel();
            view.invocation = None;
            view.text =
                "AWS context changed. Close this tab and open CLI output from the current service."
                    .into();
            view.failed = true;
            view.scroll = 0;
            view.lines = 1;
        }
    }
}

pub fn active(app: &App) -> bool {
    matches!(app.current_service, Service::CliOutput(_))
}
fn active_id(app: &App) -> Option<u64> {
    match app.current_service {
        Service::CliOutput(id) => Some(id),
        _ => None,
    }
}
/// AWS-identity fingerprint for a CLI output tab, pinned to the tab that OWNS
/// it rather than the currently-focused tab. A CLI tab is scoped to the
/// invocation it captured on open (empty resource), so AWS identity plus this
/// owner decide whether the captured command is still valid — switching focus
/// to another tab never invalidates it.
fn scope_for(app: &App, owner: TabId) -> ReadScope {
    ReadScope::from_app(app, "").with_owner(Some(owner))
}
fn open(app: &mut App) {
    let result = cli_command::invocation(app);
    let source_name = app.current_service.name();
    let (invocation, text, failed) = match result {
        Ok(invocation) => (
            Some(invocation),
            "Loading CLI JSON… Ctrl+W closes/cancels".into(),
            false,
        ),
        Err(error) => (None, error, true),
    };
    app.cli_output.next_id += 1;
    let id = app.cli_output.next_id;
    // Allocate the owning tab's stable identity BEFORE building the view, so the
    // view can be scoped to its own tab and a later focus change to a sibling
    // tab never invalidates this one. The same id goes onto the Tab literal.
    let owner = TabId::new();
    // Every view records its open-time scope (pinned to its owner) so a later
    // AWS-identity change is detected. A supported command queues a read; an
    // unsupported one stays idle but keeps the scope (so it is not spuriously
    // flagged "context changed").
    let mut read = AsyncRead::requested(scope_for(app, owner));
    if invocation.is_none() {
        read.cancel();
    }
    let view = View {
        owner,
        invocation,
        text,
        failed,
        scroll: 0,
        lines: 1,
        read,
    };
    app.cli_output.views.insert(id, view);
    app.tabs.push(Tab {
        id: owner,
        service: Service::CliOutput(id),
        title: format!("CLI · {source_name}"),
        breadcrumb: format!("CLI output · {source_name}"),
    });
    app.current_tab = app.tabs.len() - 1;
    app.current_service = Service::CliOutput(id);
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.pending_key = None;
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    if !active(app) || app.mode != Mode::Normal {
        return false;
    }
    // Let the main loop retain its shared g-prefix tab navigation.
    if key.code == crossterm::event::KeyCode::Char('g') || app.pending_key.is_some() {
        return false;
    }
    if let Some(action) = crate::keymap::handle_key(key, Mode::Normal) {
        if !handle_action(app, &action) {
            app.handle_action(action);
        }
    }
    true
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if *action == Action::OpenCliOutput && !active(app) {
        open(app);
        return true;
    }
    let Some(id) = active_id(app) else {
        return false;
    };
    if app.mode != Mode::Normal {
        return false;
    }
    match action {
        Action::NextTab | Action::PrevTab => {
            if !app.tabs.is_empty() {
                app.current_tab = if *action == Action::NextTab {
                    (app.current_tab + 1) % app.tabs.len()
                } else {
                    (app.current_tab + app.tabs.len() - 1) % app.tabs.len()
                };
                app.current_service = app.tabs[app.current_tab].service;
            }
            return true;
        }
        Action::GoBack => {
            app.handle_action(Action::CloseTab);
            return true;
        }
        Action::CloseService | Action::CloseTab => {
            app.cli_output.views.remove(&id); // abort pending subprocess immediately
                                              // Delegate structural removal to the common tab implementation.
            return false;
        }
        Action::Quit
        | Action::ConfirmQuit
        | Action::OpenSpaceMenu
        | Action::OpenTabPicker
        | Action::OpenServicePicker
        | Action::ShowHelp => return false,
        _ => {}
    }
    // Compare against the ACTIVE view's own owner, not the focused tab, so only
    // an AWS-identity change (not a focus change) counts as "context changed".
    let current_scope = app
        .cli_output
        .views
        .get(&id)
        .map(|view| scope_for(app, view.owner));
    let context_matches = current_scope
        .as_ref()
        .zip(app.cli_output.views.get(&id))
        .is_some_and(|(scope, view)| view.read.matches_scope(scope));
    let Some(current_scope) = current_scope else {
        return true;
    };
    let Some(view) = app.cli_output.views.get_mut(&id) else {
        return true;
    };
    let busy = view.read.is_busy();
    let max = view.lines.saturating_sub(1);
    match action {
        Action::NextItem | Action::ScrollDown => {
            view.scroll = view.scroll.saturating_add(1).min(max)
        }
        Action::PrevItem | Action::ScrollUp => view.scroll = view.scroll.saturating_sub(1),
        Action::PageDown => view.scroll = view.scroll.saturating_add(20).min(max),
        Action::PageUp => view.scroll = view.scroll.saturating_sub(20),
        Action::Yank if !busy => app.clipboard.request_text(&view.text),
        Action::CopyCliCommand => {
            if let Some(i) = &view.invocation {
                app.clipboard.request_text(&i.display);
            }
        }
        Action::CopyToClipboard => app.clipboard.request_snapshot(),
        Action::Refresh | Action::OpenCliOutput if view.invocation.is_some() => {
            if !context_matches {
                return true;
            }
            // Supersede any in-flight/finished read and re-queue the captured
            // invocation; the view clears its old text/loading state exactly as
            // before, and poll() starts the subprocess next cycle.
            view.read.request(current_scope);
            view.text = "Loading CLI JSON… Ctrl+W closes/cancels".into();
            view.failed = false;
            view.scroll = 0;
            view.lines = 1;
        }
        _ => {}
    }
    true
}

pub async fn poll(app: &mut App) -> bool {
    // One AWS-identity snapshot (owner unset); each view re-owns it with its own
    // tab id below, so a background completion is matched to the tab that opened
    // it and a focus change to a sibling tab never invalidates anything.
    let base_scope = ReadScope::from_app(app, "").with_owner(None);
    let ids: Vec<u64> = app
        .tabs
        .iter()
        .filter_map(|tab| match tab.service {
            Service::CliOutput(id) => Some(id),
            _ => None,
        })
        .collect();
    let state = &mut app.cli_output;
    let before = state.views.len();
    state.views.retain(|id, _| ids.contains(id));
    let mut changed = state.views.len() != before;
    for view in state.views.values_mut() {
        // This view's current scope = the AWS snapshot re-owned by its own tab.
        let current_scope = base_scope.clone().with_owner(Some(view.owner));
        if !view.read.matches_scope(&current_scope) {
            // AWS context changed: invalidate, clear the captured invocation and
            // show the context-changed message. No automatic rerun — re-point the
            // read's scope to the current one (idle) so it is not flagged again.
            view.read.request(current_scope.clone());
            view.read.cancel();
            view.invocation = None;
            view.text =
                "AWS context changed. Close this tab and open CLI output from the current service."
                    .into();
            view.failed = true;
            view.scroll = 0;
            view.lines = 1;
            changed = true;
        }
        // Spawn only when a request is queued; no AWS on the input path.
        if view.read.is_pending() {
            if let Some(invocation) = &view.invocation {
                let args = invocation.args.clone();
                changed |= view.read.start(run::run(args));
            }
        }
        if let Some(result) = view.read.take_ready(&current_scope).await {
            // Preserve the view's own prefix for a crashed/cancelled task and its
            // own display sanitization of the message.
            let result = result.map_err(|error| error.into_message("CLI task failed"));
            match result {
                Ok(text) => {
                    view.text = text;
                    view.failed = false;
                }
                Err(error) => {
                    view.text = run::clean(&error);
                    view.failed = true;
                }
            }
            view.lines = view.text.lines().count();
            view.scroll = 0;
            changed = true;
        }
    }
    changed
}

pub fn render(frame: &mut Frame, app: &App, id: u64, area: Rect) {
    let Some(view) = app.cli_output.views.get(&id) else {
        frame.render_widget(
            Paragraph::new("CLI output closed").block(ui::titled_block("CLI output")),
            area,
        );
        return;
    };
    let panels = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);
    let command = view
        .invocation
        .as_ref()
        .map(|i| i.display.as_str())
        .unwrap_or("No supported command for this selection");
    frame.render_widget(
        Paragraph::new(command)
            .wrap(Wrap { trim: false })
            .block(ui::titled_block("Read-only CLI · selected resource")),
        panels[0],
    );
    if view.failed {
        ui::render_plain_text(frame, panels[1], &view.text, view.scroll, "CLI error", true);
    } else if !view.read.is_busy() {
        ui::render_json_highlighted(
            frame,
            panels[1],
            &view.text,
            view.scroll,
            "CLI output · JSON",
            true,
        );
    }
}

#[cfg(test)]
mod tests;
