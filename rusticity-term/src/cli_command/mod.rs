//! Copy a scoped, read-only AWS CLI command from already-loaded UI state.
//! No AWS requests, subprocesses, credential reads, or automatic execution.
mod render;
use crate::{
    app::{App, Service},
    finder::{self, Focus},
    keymap::{Action, Mode},
    ui,
};
use render::{Context, Target};

/// Trusted, fixed-operation argv plus its POSIX display form. Execution never
/// reparses the display string or passes it to a shell.
#[derive(Clone)]
pub(crate) struct Invocation {
    pub args: Vec<String>,
    pub display: String,
}

pub(crate) fn invocation(app: &App) -> Result<Invocation, String> {
    render::invocation(
        &selected(app)?,
        Context {
            profile: &app.profile,
            region: &app.config.region,
            account_id: &app.config.account_id,
        },
    )
}

pub fn shortcut_visible(app: &App) -> bool {
    if crate::cli_output::active(app) {
        return false;
    }
    match app.mode {
        Mode::Normal => {
            app.service_selected
                && !app.ecr_state.scan.view.as_ref().is_some_and(|view| {
                    app.current_service == Service::EcrRepositories && view.selecting_source()
                })
        }
        Mode::ResourceFinder => {
            app.resource_finder.focus == Focus::Table && !app.resource_finder.opening
        }
        _ => false,
    }
}

fn selected(app: &App) -> Result<Target, String> {
    if !shortcut_visible(app) {
        return Err("Focus a resource row before copying its CLI command".into());
    }
    if app.mode == Mode::ResourceFinder {
        return app
            .resource_finder
            .rows
            .get(app.resource_finder.selected)
            .cloned()
            .map(Target::Resource)
            .ok_or_else(|| "No resource selected in the finder".into());
    }
    if app.current_service == Service::EcrRepositories {
        if let Some(repository) = &app.ecr_state.current_repository {
            if app.ecr_state.tab != ui::ecr::Tab::Private {
                return Err(
                    "Public ECR image commands are not supported; select the repository instead"
                        .into(),
                );
            }
            let digest = if let Some(view) = &app.ecr_state.scan.view {
                view.digest.clone()
            } else {
                let filtered = ui::ecr::filtered_ecr_images(app);
                app.ecr_state
                    .images
                    .get_selected(&filtered)
                    .ok_or("No image selected")?
                    .digest
                    .clone()
            };
            return Ok(Target::EcrImage {
                repository: repository.clone(),
                digest,
            });
        }
    }
    if crate::s3::object::active(app) {
        let bucket = app
            .s3_state
            .current_bucket
            .as_ref()
            .ok_or("No bucket selected")?;
        let key = if let Some(preview) = &app.s3_state.object_view.preview {
            preview.key.clone()
        } else {
            let object = crate::s3::object::selected(app).ok_or("No object selected")?;
            if object.is_prefix {
                return Err("Select a file, not an S3 folder prefix, for an object command".into());
            }
            object.key.clone()
        };
        let region = app
            .s3_state
            .buckets
            .items
            .iter()
            .find(|item| &item.name == bucket)
            .map(|bucket| bucket.region.clone())
            .unwrap_or_else(|| app.config.region.clone());
        return Ok(Target::S3Object {
            bucket: bucket.clone(),
            key,
            region,
        });
    }
    if app.current_service == Service::ApiGatewayApis {
        let filtered = ui::apig::filtered_apis(app);
        let api = app
            .apig_state
            .current_api
            .as_ref()
            .or_else(|| app.apig_state.apis.get_selected(&filtered))
            .ok_or("No API selected")?;
        return Ok(Target::Api {
            id: api.id.clone(),
            protocol: api.protocol_type.clone(),
        });
    }
    finder::related::selected(app)
        .map(Target::Resource)
        .ok_or_else(|| "No supported resource selected in this view".into())
}

fn command(app: &App) -> Result<String, String> {
    render::command(
        &selected(app)?,
        Context {
            profile: &app.profile,
            region: &app.config.region,
            account_id: &app.config.account_id,
        },
    )
}

/// Handle before service-specific actions so nested views cannot swallow Ctrl+Y.
pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if *action != Action::CopyCliCommand {
        return false;
    }
    match command(app) {
        Ok(command) => app.clipboard.request_text(command),
        Err(reason) => app
            .clipboard
            .report_failure(format!("CLI command: {reason}")),
    }
    true
}

#[cfg(test)]
mod tests;
