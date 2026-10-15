//! S3 object actions. Transfer futures are owned, cancelable, and never awaited
//! by key handlers. This reuses the common JSON/YAML/plain text viewer.
use crate::{
    app::{App, Service},
    keymap::{Action, Mode},
    s3::Object,
    ui::{self, s3::ObjectTab},
};
use ratatui::{prelude::*, widgets::Paragraph};
use rusticity_core::{
    s3::object::{TextFormat, TextPreview},
    S3Client,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::task::JoinHandle;

pub struct Preview {
    pub key: String,
    pub content: TextPreview,
    pub scroll: usize,
}
struct Request {
    bucket: String,
    region: String,
    key: String,
    destination: Option<PathBuf>,
    scope: String,
}
enum Output {
    Preview(TextPreview),
    Download(PathBuf),
}
struct Running {
    task: JoinHandle<Result<Output, String>>,
    key: String,
    scope: String,
}
impl Drop for Running {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub struct Notice {
    pub message: String,
    pub failed: bool,
    at: Instant,
}
#[derive(Default)]
pub struct State {
    pub preview: Option<Preview>,
    pub notice: Option<Notice>,
    pending: Option<Request>,
    running: Option<Running>,
    scope: String,
}
impl State {
    pub fn busy(&self) -> bool {
        self.pending.is_some() || self.running.is_some()
    }
    fn notify(&mut self, message: impl Into<String>, failed: bool) {
        self.notice = Some(Notice {
            message: message.into(),
            failed,
            at: Instant::now(),
        });
    }
    fn cancel(&mut self) {
        self.pending = None;
        self.running = None;
    }
}

pub fn active(app: &App) -> bool {
    app.service_selected
        && app.current_service == Service::S3Buckets
        && app.s3_state.current_bucket.is_some()
        && app.s3_state.object_tab == ObjectTab::Objects
}
fn scope(app: &App) -> String {
    format!(
        "{}|{}|{}|{}|{:?}|{:?}",
        app.profile,
        app.config.region,
        app.config.account_id,
        app.config.role_arn,
        app.s3_state.current_bucket,
        app.s3_state.prefix_stack
    )
}

/// The same visible ordering as the objects table: filter root rows, then append
/// expanded descendants. Loading prefixes have no selectable object yet.
pub fn visible_objects(state: &ui::s3::State) -> Vec<&Object> {
    fn append<'a>(obj: &'a Object, state: &'a ui::s3::State, rows: &mut Vec<&'a Object>) {
        rows.push(obj);
        if obj.is_prefix && state.expanded_prefixes.contains(&obj.key) {
            if let Some(children) = state.prefix_preview.get(&obj.key) {
                for child in children {
                    append(child, state, rows);
                }
            }
        }
    }
    let mut rows = vec![];
    let prefix = state
        .prefix_stack
        .last()
        .map(String::as_str)
        .unwrap_or_default();
    let filter = state.object_filter.to_lowercase();
    for object in state.objects.iter().filter(|object| {
        object
            .key
            .strip_prefix(prefix)
            .unwrap_or(&object.key)
            .to_lowercase()
            .contains(&filter)
    }) {
        append(object, state, &mut rows);
    }
    rows
}

pub fn selected(app: &App) -> Option<&Object> {
    visible_objects(&app.s3_state)
        .get(app.s3_state.selected_object)
        .copied()
}

fn request(app: &mut App, download: bool) {
    if app.s3_state.object_view.busy() {
        app.s3_state
            .object_view
            .notify("Object transfer in progress; Esc cancels", false);
        return;
    }
    let Some(object) = selected(app).filter(|object| !object.is_prefix) else {
        return;
    };
    let key = object.key.clone();
    let bucket = app
        .s3_state
        .current_bucket
        .clone()
        .expect("active object view");
    let region = app
        .s3_state
        .buckets
        .items
        .iter()
        .find(|item| item.name == bucket)
        .map(|item| item.region.clone())
        .unwrap_or_default();
    let destination = if download {
        match std::env::current_dir() {
            Ok(path) => Some(path),
            Err(error) => {
                app.s3_state.object_view.notify(error.to_string(), true);
                return;
            }
        }
    } else {
        None
    };
    let scope = scope(app);
    let state = &mut app.s3_state.object_view;
    state.scope = scope.clone();
    state.notify(
        if download {
            "Downloading object… Esc cancels"
        } else {
            "Loading text preview… Esc cancels"
        },
        false,
    );
    state.pending = Some(Request {
        bucket,
        region,
        key,
        destination,
        scope,
    });
}

/// True means the S3 object view handled the action; don't run generic loaders.
pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !active(app) || app.mode != Mode::Normal {
        return false;
    }
    if app.s3_state.object_view.preview.is_some() {
        let preview = app.s3_state.object_view.preview.as_mut().unwrap();
        let max = preview.content.text.lines().count().saturating_sub(1);
        match action {
            Action::NextItem => preview.scroll = preview.scroll.saturating_add(1).min(max),
            Action::PrevItem => preview.scroll = preview.scroll.saturating_sub(1),
            Action::PageDown => preview.scroll = preview.scroll.saturating_add(20).min(max),
            Action::PageUp => preview.scroll = preview.scroll.saturating_sub(20),
            Action::GoBack => {
                app.s3_state.object_view.preview = None;
            }
            Action::Yank => app.clipboard.request_text(&preview.content.text),
            // Keep app-wide close/quit/tab controls available; service controls must
            // not mutate the underlying object list while previewing content.
            Action::Quit
            | Action::ConfirmQuit
            | Action::CloseService
            | Action::NextTab
            | Action::PrevTab
            | Action::OpenSpaceMenu
            | Action::ShowHelp
            | Action::CopyToClipboard => return false,
            _ => {}
        }
        return true;
    }
    match action {
        Action::PageDown => {
            request(app, true);
            true
        }
        Action::Select if selected(app).is_some_and(|object| !object.is_prefix) => {
            request(app, false);
            true
        }
        Action::GoBack if app.s3_state.object_view.busy() => {
            app.s3_state.object_view.cancel();
            app.s3_state
                .object_view
                .notify("Object transfer cancelled", false);
            true
        }
        _ => false,
    }
}

/// Poll only finished tasks; no UI input waits for an S3 response or file write.
pub async fn poll(app: &mut App) -> bool {
    let current_scope = scope(app);
    let state = &mut app.s3_state.object_view;
    if !state.scope.is_empty()
        && (state.scope != current_scope
            || !app.service_selected
            || app.current_service != Service::S3Buckets
            || app.s3_state.object_tab != ObjectTab::Objects)
    {
        *state = State::default();
        return true;
    }
    let mut changed = false;
    if state
        .notice
        .as_ref()
        .is_some_and(|notice| notice.at.elapsed() >= Duration::from_secs(8))
        && !state.busy()
    {
        state.notice = None;
        changed = true;
    }
    if let Some(request) = state.pending.take() {
        let config = app.config.clone();
        let key = request.key.clone();
        let scope = request.scope.clone();
        let task = tokio::spawn(async move {
            let client = S3Client::new(config);
            let operation = async {
                let region = if request.region.is_empty() {
                    client.get_bucket_location(&request.bucket).await?
                } else {
                    request.region
                };
                if let Some(directory) = request.destination {
                    client
                        .download_object(&request.bucket, &region, &request.key, &directory)
                        .await
                        .map(Output::Download)
                } else {
                    client
                        .preview_object(&request.bucket, &region, &request.key)
                        .await
                        .map(Output::Preview)
                }
            };
            match tokio::time::timeout(Duration::from_secs(300), operation).await {
                Ok(result) => result.map_err(|error| format!("{error:#}")),
                Err(_) => Err("Object transfer timed out".into()),
            }
        });
        state.running = Some(Running { task, key, scope });
        changed = true;
    }
    if state
        .running
        .as_ref()
        .is_some_and(|running| running.task.is_finished())
    {
        let mut running = state.running.take().unwrap();
        let result = (&mut running.task).await;
        if running.scope != current_scope {
            return true;
        }
        match result {
            Ok(Ok(Output::Preview(content))) => {
                state.preview = Some(Preview {
                    key: running.key.clone(),
                    content,
                    scroll: 0,
                });
                state.notice = None;
            }
            Ok(Ok(Output::Download(path))) => {
                state.notify(format!("Downloaded to {}", path.display()), false)
            }
            Ok(Err(error)) => state.notify(error, true),
            Err(error) => state.notify(format!("Object transfer failed: {error}"), true),
        }
        changed = true;
    }
    changed
}

pub fn render_preview(frame: &mut Frame, app: &App, area: Rect) -> bool {
    let Some(preview) = &app.s3_state.object_view.preview else {
        return false;
    };
    let title = format!(
        "{}{}",
        preview
            .key
            .chars()
            .filter(|c| !c.is_control())
            .collect::<String>(),
        if preview.content.truncated {
            " — preview truncated at 1 MiB"
        } else {
            ""
        }
    );
    if preview.content.text.is_empty() {
        frame.render_widget(
            Paragraph::new("Empty text object").block(ui::titled_block(&title)),
            area,
        );
    } else {
        let render = match preview.content.format {
            TextFormat::Json => ui::render_json_highlighted,
            TextFormat::Yaml => ui::render_yaml_highlighted,
            TextFormat::Plain => ui::render_plain_text,
        };
        render(
            frame,
            area,
            &preview.content.text,
            preview.scroll,
            &title,
            true,
        );
    }
    true
}

#[cfg(test)]
mod tests;
