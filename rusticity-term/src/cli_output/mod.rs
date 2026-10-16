//! Dedicated CLI output tabs. Results and commands stay in memory, never sessions.
mod run;
use crate::{
    app::{App, Service, Tab},
    cli_command::{self, Invocation},
    keymap::{Action, Mode},
    ui,
};
use crossterm::event::KeyEvent;
use ratatui::{
    prelude::*,
    widgets::{Paragraph, Wrap},
};
use std::collections::BTreeMap;
use tokio::task::JoinHandle;

struct Running(JoinHandle<Result<String, String>>);
impl Drop for Running {
    fn drop(&mut self) {
        self.0.abort();
    }
}
#[derive(Default)]
pub struct State {
    views: BTreeMap<u64, View>,
    next_id: u64,
}
struct View {
    invocation: Option<Invocation>,
    text: String,
    failed: bool,
    scroll: usize,
    lines: usize,
    scope: String,
    pending: bool,
    running: Option<Running>,
}
impl State {
    pub fn busy(&self) -> bool {
        self.views
            .values()
            .any(|v| v.pending || v.running.is_some())
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
fn scope(app: &App) -> String {
    format!(
        "{:?}",
        (
            &app.profile,
            &app.config.region,
            &app.config.account_id,
            &app.config.role_arn
        )
    )
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
    let view = View {
        pending: invocation.is_some(),
        invocation,
        text,
        failed,
        scroll: 0,
        lines: 1,
        scope: scope(app),
        running: None,
    };
    app.cli_output.views.insert(id, view);
    app.tabs.push(Tab {
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
    let context_matches = app
        .cli_output
        .views
        .get(&id)
        .is_some_and(|view| view.scope == scope(app));
    let Some(view) = app.cli_output.views.get_mut(&id) else {
        return true;
    };
    let busy = view.pending || view.running.is_some();
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
            view.running = None;
            view.pending = true;
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
    let current_scope = scope(app);
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
        if view.scope != current_scope {
            view.running = None;
            view.pending = false;
            view.invocation = None;
            view.text =
                "AWS context changed. Close this tab and open CLI output from the current service."
                    .into();
            view.failed = true;
            view.scroll = 0;
            view.lines = 1;
            view.scope = current_scope.clone();
            changed = true;
        }
        if view.pending {
            view.pending = false;
            if let Some(invocation) = &view.invocation {
                let args = invocation.args.clone();
                view.running = Some(Running(tokio::spawn(run::run(args))));
                changed = true;
            }
        }
        if view
            .running
            .as_ref()
            .is_some_and(|running| running.0.is_finished())
        {
            let mut running = view.running.take().unwrap();
            let result = match (&mut running.0).await {
                Ok(result) => result,
                Err(error) => Err(format!("CLI task failed: {error}")),
            };
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
    } else if view.pending || view.running.is_some() {
        frame.render_widget(
            Paragraph::new(&*view.text).block(ui::titled_block("CLI output")),
            panels[1],
        );
    } else {
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
