//! Resource Explorer › Resources: the console Resource search page. The
//! filter line holds the query, which Resource Explorer runs with the
//! default view of the tab's Region; a dropdown in the same line applies a
//! console query template. Read-only.
//!
//! The tab searches when it opens (an empty query: everything the view holds,
//! up to 1,000 resources). Enter in the filter searches the typed query, Esc
//! puts back the query on screen, and Ctrl+R runs that query again. Every
//! search clears the results at once, and a change of the tab's AWS context
//! searches again, so no result outlives the context it was read in.
use crate::{
    app::{App, Service},
    async_read::{ReadScope, ReadStatus},
    common::{InputFocus, SortDirection},
    console::ConsoleUrl,
    elb::{keys, DetailRead},
    keymap::{Action, Mode},
    sort::SortOrder,
    spec_table::{Spec, Table, Texts},
};
use ratatui::prelude::*;
use rusticity_core::resource_explorer::{self, ExplorerResource, ResourceSearch};

/// The console result columns, in console order.
pub(crate) const COLUMNS: Spec<ExplorerResource> = &[
    ("Identifier", |r| r.identifier.clone()),
    ("Service", |r| r.service.clone()),
    ("Resource type", |r| r.resource_type.clone()),
    ("Region", |r| r.region.clone()),
    ("Owner account", |r| r.owner_account.clone()),
    ("Application", |r| r.application.clone()),
    // The number of tags; empty when the view does not include tags.
    ("Tags", |r| {
        r.tags
            .as_ref()
            .map(|t| t.len().to_string())
            .unwrap_or_default()
    }),
];

/// The console query templates, in console order.
pub(crate) const TEMPLATES: &[&str] = &[
    "Tagged resources",
    "All untagged resources",
    "All non-taggable resources",
    "All untagged resources of [type]",
    "Resources not in [application]",
    "All resources in [application]",
    "Amazon EC2 resources that are not instances in [application]",
];

/// The query of each template. One that ends with `:` takes a value: a
/// resource type such as `ec2:volume`, or an application name.
pub(crate) const TEMPLATE_QUERIES: &[&str] = &[
    "tag:all",
    "tag:none",
    "-resourcetype.supports:tags",
    "tag:none resourcetype:",
    "-application:",
    "application:",
    "service:ec2 -resourcetype:ec2:instance application:",
];

/// An expanded result: its ARN and report time, then each tag.
fn details(r: &ExplorerResource) -> Vec<String> {
    let mut lines = vec![format!("ARN: {}", r.arn)];
    if !r.last_reported_at.is_empty() {
        lines.push(format!(
            "Last reported: {}",
            crate::common::format_iso_timestamp(&r.last_reported_at)
        ));
    }
    match &r.tags {
        None => lines.push("Tags: the view does not include tags".into()),
        Some(tags) => lines.extend(tags.iter().map(|(k, v)| {
            format!(
                "Tag: {k}: {}",
                if v.is_empty() { "(empty value)" } else { v }
            )
        })),
    }
    lines
}

/// Embedded at `app.resource_explorer_state`.
pub struct State {
    /// The query of the results on screen; Ctrl+R runs it again.
    pub(crate) query: String,
    /// None until the tab first searches.
    pub(crate) read: Option<DetailRead<ResourceSearch>>,
    /// Its filter text is the query being typed.
    pub(crate) list: Table<ExplorerResource>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            query: String::new(),
            read: None,
            list: Table::new(COLUMNS)
                .querying()
                .with_dropdown("Query template", TEMPLATES)
                .sorted_by(SortOrder::new("Identifier", SortDirection::Asc))
                .detailing(details),
        }
    }
}

impl State {
    pub(crate) fn load_status(&self) -> ReadStatus {
        self.read.as_ref().map_or(ReadStatus::Idle, |r| r.status())
    }
}

pub fn active(app: &App) -> bool {
    app.service_selected && app.current_service == Service::ResourceExplorerResources
}

pub(crate) fn scope(app: &App) -> ReadScope {
    ReadScope::from_app(app, "resource-explorer")
}

/// Clear the results at once and queue a search for `query`.
fn search(app: &mut App, query: String) {
    let scope = scope(app);
    let state = &mut app.resource_explorer_state;
    state.list.clear();
    state.list.rows.filter = query.clone();
    state.list.focus = InputFocus::Filter;
    state.query = query;
    state.read = Some(DetailRead::requested(scope));
}

/// The tab runtime asks for a fresh read: the query on screen again.
pub fn request(app: &mut App) {
    let query = app.resource_explorer_state.query.clone();
    search(app, query);
}

/// Esc while searching cancels; otherwise Esc is not ours.
fn cancel(app: &mut App) -> bool {
    let state = &mut app.resource_explorer_state;
    match state.read.as_mut() {
        Some(read) if read.is_busy() => {
            read.cancel();
            state.list.clear();
            true
        }
        _ => false,
    }
}

/// Enter on the template list: put its query in the filter. A template that
/// takes a value waits for it; any other searches at once.
fn apply_template(app: &mut App, index: usize) {
    let query = TEMPLATE_QUERIES[index].to_string();
    if query.ends_with(':') {
        let list = &mut app.resource_explorer_state.list;
        list.rows.filter = query;
        list.focus = InputFocus::Filter;
        return;
    }
    app.mode = Mode::Normal;
    search(app, query);
}

/// The ARN `y` copies.
pub(crate) fn selected_arn(app: &App) -> Option<String> {
    let list = &app.resource_explorer_state.list;
    let rows = list.filtered();
    list.rows.get_selected(&rows).map(|r| r.arn.clone())
}

fn list_input(app: &mut App, action: &Action) -> bool {
    let selected = selected_arn(app);
    let (state, keys) = keys!(app, resource_explorer_state, selected);
    crate::spec_table::input(&mut state.list, keys, action)
}

pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app)
        || !matches!(
            app.mode,
            Mode::Normal | Mode::FilterInput | Mode::ErrorModal
        )
    {
        return false;
    }
    match app.mode {
        Mode::ErrorModal => {
            if crate::elb::retry_from_error(app, action) {
                request(app);
                return true;
            }
            return false;
        }
        Mode::FilterInput => match action {
            Action::ApplyFilter => {
                let state = &app.resource_explorer_state;
                if let Some(index) = state.list.dropdown_choice() {
                    apply_template(app, index);
                    return true;
                }
                if state.list.focus == InputFocus::Filter {
                    let query = state.list.rows.filter.trim().to_string();
                    app.mode = Mode::Normal;
                    search(app, query);
                    return true;
                }
            }
            Action::CloseMenu => {
                // Leave the typed query: the filter shows the query on screen.
                let state = &mut app.resource_explorer_state;
                state.list.rows.filter = state.query.clone();
                state.list.focus = InputFocus::Filter;
                app.mode = Mode::Normal;
                return true;
            }
            _ => {}
        },
        _ => match action {
            Action::Refresh => {
                request(app);
                return true;
            }
            Action::GoBack => return cancel(app),
            Action::OpenColumnSelector => return false,
            _ if crate::elb::global(action) => return false,
            _ => {}
        },
    }
    list_input(app, action)
}

/// Drive the search; called for the owning tab only.
pub async fn poll(app: &mut App) -> bool {
    let scope = scope(app);
    let Some(read) = app.resource_explorer_state.read.as_mut() else {
        return false;
    };
    if !read.matches_scope(&scope) {
        // The profile, Region, account or role moved: search again in it.
        request(app);
        return true;
    }
    let config = app.aws.config.clone();
    let query = app.resource_explorer_state.query.clone();
    let state = &mut app.resource_explorer_state;
    let read = state.read.as_mut().unwrap();
    let (moved, arrived) = read
        .drive(&scope, "Resource search failed", || async move {
            resource_explorer::search(&config, &query)
                .await
                .map(Box::new)
                .map_err(|e| format!("{e:#}"))
        })
        .await;
    if arrived {
        state.list.clear();
        if let Some(found) = state.read.as_ref().and_then(|r| r.detail()) {
            let rows = found.resources.clone();
            let partial = !found.complete;
            state.list.rows.items = rows;
            state.list.partial = partial;
        }
    }
    moved
}

pub fn breadcrumb() -> Vec<String> {
    vec!["Resource Explorer".into(), "Resources".into()]
}

pub fn console_url(app: &App) -> String {
    let region = &app.aws.config.region;
    ConsoleUrl::regional(region, "/resource-explorer/home")
        .query("region", region)
        .fragment("/search")
        .finish()
}

/// The query on screen as one `search` call.
pub(crate) fn cli_input(app: &App) -> serde_json::Value {
    serde_json::json!({ "QueryString": app.resource_explorer_state.query })
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let state = &app.resource_explorer_state;
    let (status, error) = match &state.read {
        Some(read) => (read.status(), read.error()),
        None => (ReadStatus::Idle, None),
    };
    crate::spec_table::render(
        frame,
        app,
        &state.list,
        Texts {
            title: "Resources",
            placeholder: "Query: keywords and filters, such as service:ec2 tag:none",
            no_items: "No resources match this query.",
        },
        status,
        error,
        |_| Style::default(),
        area,
    );
}

#[cfg(test)]
mod tests;
