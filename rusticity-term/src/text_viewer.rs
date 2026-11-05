//! Search and horizontal scrolling for the shared JSON/text viewer.
//!
//! One registry (`focused`) names the viewer currently on screen and in focus,
//! with its text and its owning view's vertical scroll field. Each view keeps
//! its own scrolling and rendering; this module only adds:
//!
//! * `/` to type a query (live matches, Enter keeps it, Esc restores the old one),
//! * `n` / `N` to jump to the next / previous match, wrapping around,
//! * `→` / `←` to scroll long lines sideways,
//! * Esc in normal mode to clear an active query before Esc goes back.
//!
//! Matching is literal and smart-case: a query with an uppercase letter is
//! case-sensitive, otherwise case-insensitive. Nothing here reads AWS.
use crate::{
    app::{App, Service, ViewMode},
    keymap::{Action, Mode},
    ui::{Highlight, ViewerOverlay},
};
use ratatui::{layout::Rect, Frame};
use std::cell::Cell;

/// Columns moved per `←` / `→`.
pub(crate) const HSCROLL_STEP: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub line: usize,
    /// Character offsets within the line.
    pub start: usize,
    pub len: usize,
}

/// The search state of the one focused viewer. A different viewer (another
/// tab, resource or pane) resets it, so a query never leaks across views.
#[derive(Default)]
pub struct State {
    id: String,
    query: String,
    /// The query being typed in `Mode::ViewerSearch`.
    draft: Option<String>,
    current: Option<usize>,
    hscroll: usize,
    /// Content width and height from the last render.
    viewport: Cell<(u16, u16)>,
}

impl State {
    fn effective_query(&self) -> &str {
        self.draft.as_deref().unwrap_or(&self.query)
    }
}

/// Which registered viewer is focused. Each arm mirrors the condition under
/// which that view renders its JSON/text pane, so only the visible viewer
/// responds to keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    CliOutput(u64),
    KmsPolicy,
    SqsPolicy,
    EfsPolicy,
    IamTrustPolicy,
    IamPolicyDocument,
    CfnTemplate,
    CloudTrailEvent,
}

fn kind(app: &App) -> Option<Kind> {
    if !app.service_selected && !matches!(app.current_service, Service::CliOutput(_)) {
        return None;
    }
    Some(match app.current_service {
        Service::CliOutput(id) if crate::cli_output::viewer_text(app, id).is_some() => {
            Kind::CliOutput(id)
        }
        Service::KmsKeys
            if crate::kms::detail::active(app)
                && app.kms_state.detail.view().is_some_and(|view| {
                    view.tab == crate::kms::detail::Tab::Policy && view.policy().is_some()
                }) =>
        {
            Kind::KmsPolicy
        }
        Service::SqsQueues
            if app.sqs_state.current_queue.is_some()
                && app.sqs_state.detail_tab == crate::ui::sqs::QueueDetailTab::QueuePolicies =>
        {
            Kind::SqsPolicy
        }
        // The policy pane renders only once the entered file system's row exists.
        Service::EfsFileSystems
            if app
                .efs_state
                .current_file_system
                .as_deref()
                .is_some_and(|id| {
                    app.efs_state
                        .file_systems
                        .items
                        .iter()
                        .any(|fs| fs.file_system_id == id)
                })
                && app.efs_state.detail_tab == crate::ui::efs::DetailTab::FileSystemPolicy
                && !app.efs_state.policy_document.is_empty() =>
        {
            Kind::EfsPolicy
        }
        Service::IamRoles if app.view_mode == ViewMode::PolicyView => Kind::IamPolicyDocument,
        Service::IamRoles
            if app.iam_state.current_role.is_some()
                && app.iam_state.role_tab == crate::ui::iam::RoleTab::TrustRelationships =>
        {
            Kind::IamTrustPolicy
        }
        // The template pane renders only once the entered stack's row exists.
        Service::CloudFormationStacks
            if app.cfn_state.current_stack.as_deref().is_some_and(|name| {
                app.cfn_state
                    .table
                    .items
                    .iter()
                    .any(|stack| stack.name == name)
            }) && app.cfn_state.detail_tab == crate::ui::cfn::DetailTab::Template =>
        {
            Kind::CfnTemplate
        }
        Service::CloudTrailEvents
            if app.cloudtrail_state.current_event.is_some()
                && app.cloudtrail_state.detail_focus
                    == crate::app::CloudTrailDetailFocus::EventRecord =>
        {
            Kind::CloudTrailEvent
        }
        _ => return None,
    })
}

fn text(app: &App, kind: Kind) -> &str {
    match kind {
        Kind::CliOutput(id) => crate::cli_output::viewer_text(app, id).unwrap_or_default(),
        Kind::KmsPolicy => app
            .kms_state
            .detail
            .view()
            .and_then(|view| view.policy())
            .unwrap_or_default(),
        Kind::SqsPolicy => &app.sqs_state.policy_document,
        Kind::EfsPolicy => &app.efs_state.policy_document,
        Kind::IamTrustPolicy => &app.iam_state.trust_policy_document,
        Kind::IamPolicyDocument => &app.iam_state.policy_document,
        Kind::CfnTemplate => &app.cfn_state.template_body,
        Kind::CloudTrailEvent => app
            .cloudtrail_state
            .current_event
            .as_ref()
            .map(|event| event.cloud_trail_event_json.as_str())
            .unwrap_or_default(),
    }
}

fn scroll_mut(app: &mut App, kind: Kind) -> Option<&mut usize> {
    Some(match kind {
        Kind::CliOutput(id) => crate::cli_output::viewer_scroll_mut(app, id)?,
        Kind::KmsPolicy => &mut app.kms_state.detail.view_mut()?.policy_scroll,
        Kind::SqsPolicy => &mut app.sqs_state.policy_scroll,
        Kind::EfsPolicy => &mut app.efs_state.policy_scroll,
        Kind::IamTrustPolicy => &mut app.iam_state.trust_policy_scroll,
        Kind::IamPolicyDocument => &mut app.iam_state.policy_scroll,
        Kind::CfnTemplate => &mut app.cfn_state.template_scroll,
        Kind::CloudTrailEvent => &mut app.cloudtrail_state.event_json_scroll,
    })
}

/// Identity: owning tab, viewer and resource. Any change starts a fresh search.
fn identity(app: &App, kind: Kind) -> String {
    let tab = app
        .tabs
        .get(app.current_tab)
        .map(|tab| format!("{:?}", tab.id))
        .unwrap_or_default();
    let resource = match kind {
        Kind::CliOutput(id) => id.to_string(),
        Kind::KmsPolicy => app
            .kms_state
            .detail
            .source
            .as_ref()
            .map(|key| key.key_arn.clone())
            .unwrap_or_default(),
        Kind::SqsPolicy => app.sqs_state.current_queue.clone().unwrap_or_default(),
        Kind::EfsPolicy => app
            .efs_state
            .current_file_system
            .clone()
            .unwrap_or_default(),
        Kind::IamTrustPolicy => app.iam_state.current_role.clone().unwrap_or_default(),
        Kind::IamPolicyDocument => format!(
            "{}:{}",
            app.iam_state.current_role.clone().unwrap_or_default(),
            app.iam_state.policy_document.len()
        ),
        Kind::CfnTemplate => app.cfn_state.current_stack.clone().unwrap_or_default(),
        Kind::CloudTrailEvent => app
            .cloudtrail_state
            .current_event
            .as_ref()
            .map(|event| event.event_id.clone())
            .unwrap_or_default(),
    };
    format!("{tab}:{kind:?}:{resource}")
}

/// Every literal, smart-case occurrence of `query`, in reading order.
pub fn find_matches(text: &str, query: &str) -> Vec<Match> {
    if query.is_empty() {
        return Vec::new();
    }
    let sensitive = query.chars().any(char::is_uppercase);
    let fold = |s: &str| -> Vec<char> {
        if sensitive {
            s.chars().collect()
        } else {
            // Per-character folding keeps offsets aligned with the original.
            s.chars()
                .map(|c| c.to_lowercase().next().unwrap_or(c))
                .collect()
        }
    };
    let needle = fold(query);
    let mut out = Vec::new();
    for (line_no, line) in text.lines().enumerate() {
        let hay = fold(line);
        let mut i = 0;
        while i + needle.len() <= hay.len() {
            if hay[i..i + needle.len()] == needle[..] {
                out.push(Match {
                    line: line_no,
                    start: i,
                    len: needle.len(),
                });
                i += needle.len();
            } else {
                i += 1;
            }
        }
    }
    out
}

/// Bring the viewer state in line with the focused viewer; returns it with
/// its kind, or None when no registered viewer is focused.
fn sync(app: &mut App) -> Option<Kind> {
    let kind = kind(app)?;
    let id = identity(app, kind);
    if app.text_viewer.id != id {
        app.text_viewer = State {
            id,
            ..State::default()
        };
    }
    Some(kind)
}

/// Move the owning view's scroll (and the sideways scroll) so `m` is visible.
fn reveal(app: &mut App, kind: Kind, m: &Match) {
    let (width, height) = app.text_viewer.viewport.get();
    let height = (height as usize).max(1);
    if let Some(scroll) = scroll_mut(app, kind) {
        if m.line < *scroll || m.line >= *scroll + height {
            *scroll = m.line.saturating_sub(height / 3);
        }
    }
    let width = (width as usize).max(1);
    let state = &mut app.text_viewer;
    if m.start < state.hscroll || m.start + m.len > state.hscroll + width {
        state.hscroll = m.start.saturating_sub(width / 4);
    }
}

/// The first match at or after the viewer's top line, else the first.
fn first_visible(matches: &[Match], top: usize) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    Some(matches.iter().position(|m| m.line >= top).unwrap_or(0))
}

fn top_line(app: &mut App, kind: Kind) -> usize {
    scroll_mut(app, kind).map(|s| *s).unwrap_or(0)
}

fn refresh_draft_matches(app: &mut App, kind: Kind) {
    let matches = find_matches(text(app, kind), app.text_viewer.effective_query());
    let top = top_line(app, kind);
    app.text_viewer.current = first_visible(&matches, top);
    if let Some(index) = app.text_viewer.current {
        reveal(app, kind, &matches[index]);
    }
}

/// Consumes viewer keys while a registered viewer is focused. Runs before the
/// owning view so `←`/`→`, `n`/`N` and Esc mean viewer navigation there.
pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if app.mode == Mode::ViewerSearch {
        let Some(kind) = sync(app) else {
            // The viewer went away while typing; leave search mode cleanly.
            app.mode = Mode::Normal;
            return false;
        };
        let state = &mut app.text_viewer;
        let draft = state.draft.get_or_insert_with(String::new);
        match action {
            Action::FilterInput(c) => draft.push(*c),
            Action::FilterBackspace => {
                draft.pop();
            }
            Action::DeleteWord => {
                let keep = draft.trim_end().rfind(' ').map_or(0, |i| i + 1);
                draft.truncate(keep);
            }
            Action::Select => {
                state.query = state.draft.take().unwrap_or_default();
                app.mode = Mode::Normal;
                return true;
            }
            Action::CloseMenu => {
                state.draft = None;
                app.mode = Mode::Normal;
                refresh_draft_matches(app, kind);
                return true;
            }
            _ => return true,
        }
        refresh_draft_matches(app, kind);
        return true;
    }
    if app.mode != Mode::Normal {
        return false;
    }
    let Some(kind) = sync(app) else {
        return false;
    };
    match action {
        Action::ViewerSearch => {
            app.text_viewer.draft = Some(String::new());
            app.text_viewer.current = None;
            app.mode = Mode::ViewerSearch;
        }
        Action::ViewerNextMatch | Action::ViewerPrevMatch => {
            let matches = find_matches(text(app, kind), &app.text_viewer.query);
            if matches.is_empty() {
                return true;
            }
            let count = matches.len();
            let next = match (app.text_viewer.current, action) {
                (None, _) => first_visible(&matches, top_line(app, kind)).unwrap_or(0),
                (Some(i), Action::ViewerNextMatch) => (i + 1) % count,
                (Some(i), _) => (i + count - 1) % count,
            };
            app.text_viewer.current = Some(next);
            reveal(app, kind, &matches[next]);
        }
        Action::ExpandRow => app.text_viewer.hscroll += HSCROLL_STEP,
        Action::CollapseRow => {
            let state = &mut app.text_viewer;
            state.hscroll = state.hscroll.saturating_sub(HSCROLL_STEP);
        }
        Action::GoBack if !app.text_viewer.query.is_empty() => {
            app.text_viewer.query.clear();
            app.text_viewer.current = None;
        }
        _ => return false,
    }
    true
}

/// Render a registered viewer pane with its search/scroll overlay. Views call
/// this instead of the plain renderer; it falls back to no overlay when the
/// pane is not the focused viewer.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    body: &str,
    scroll: usize,
    title: &str,
    is_active: bool,
    highlight: Highlight,
) {
    let state = &app.text_viewer;
    let focused = kind(app).is_some_and(|kind| identity(app, kind) == state.id);
    let fresh = State::default();
    let state = if focused { state } else { &fresh };
    let typing = app.mode == Mode::ViewerSearch && focused;
    let query = state.effective_query();
    let matches = find_matches(body, query);
    let footer = if typing {
        format!(" /{query}▏ {} ", count_text(state.current, matches.len()))
    } else if !query.is_empty() {
        format!(
            " /{query}  {}  n/N next/prev · ⎋ clear ",
            count_text(state.current, matches.len())
        )
    } else if is_active && kind(app).is_some() {
        " / search · ←→ scroll ".into()
    } else {
        String::new()
    };
    let overlay = ViewerOverlay {
        matches: &matches,
        current: state.current,
        hscroll: state.hscroll,
        footer,
        viewport: &app.text_viewer.viewport,
    };
    crate::ui::render_text_viewer(
        frame,
        area,
        body,
        scroll,
        title,
        is_active,
        highlight,
        Some(&overlay),
    );
}

#[cfg(test)]
impl App {
    pub(crate) fn text_viewer_hscroll_for_test(&self) -> usize {
        self.text_viewer.hscroll
    }
}

fn count_text(current: Option<usize>, total: usize) -> String {
    match (current, total) {
        (_, 0) => "No matches".into(),
        (Some(i), n) => format!("{} of {n}", i + 1),
        (None, n) => format!("{n} matches"),
    }
}

#[cfg(test)]
mod tests;
