//! View-scoped CLI resolver: given the CURRENT table view, produce the read-only
//! AWS CLI command that lists THAT collection — never a command derived from the
//! highlighted row. A table's Ctrl+Y/Ctrl+E copies the collection command for the
//! table in front of the user (e.g. the Step Functions state-machine table copies
//! `list-state-machines`, and an entered executions table copies `list-executions`
//! scoped to its parent machine), matching the resource-specific row actions that
//! live elsewhere.
//!
//! Contract:
//!   * `Ok(Some(Target::View { .. }))` — this is one of our owned table views and
//!     has a representable, fixed-operation collection command.
//!   * `Ok(None)` — this view is a true summary/detail pane (no collection), or it
//!     belongs to a service another worker owns. The caller falls through to its
//!     existing row/detail handling (e.g. S3 head-object on an open object preview).
//!   * `Err(_)` — this view IS a table but its contents have no representable
//!     standalone query (an aggregate assembled from a parent describe, or an
//!     explicitly unsupported variant). Never silently fall through to the
//!     selected row in that case.
//!
//! Every operation literal is a compile-time `&'static str`; nothing here is
//! derived from runtime user input. All identifiers travel inside RAW JSON so the
//! AWS CLI's parameter-file / shorthand expansion cannot reinterpret them. Region
//! is `None` for "use the caller's current region" (the default), except IAM
//! (global, not owned here) and ecr-public (always us-east-1).
use super::render::Target;
use crate::app::{App, Service};
use rusticity_core::finder::{Resource, ResourceKind};
use serde_json::{json, Value};

/// Build a `Target::View` with a static operation, raw JSON input, an optional
/// region override (None = caller's current region) and an optional parent-scope
/// resource used by the renderer to validate region/account.
fn view(
    operation: &'static str,
    input: Value,
    region: Option<String>,
    scope: Option<Resource>,
) -> Target {
    Target::View {
        operation,
        input,
        region,
        scope,
    }
}

/// A parent-identity `Resource` for scope validation. The ARN (when known) lets
/// the renderer reject a foreign account/region exactly as a row command would;
/// an empty ARN simply skips that cross-check.
fn scope_resource(kind: ResourceKind, id: &str, arn: &str, region: &str) -> Resource {
    Resource {
        kind,
        id: id.to_string(),
        name: id.to_string(),
        arn: arn.to_string(),
        region: region.to_string(),
    }
}

/// Resolve the collection command for the current table view. See the module
/// docs for the `Ok(None)` vs `Err` distinction.
pub(super) fn target(app: &App) -> Result<Option<Target>, String> {
    match app.current_service {
        Service::StepFunctionsStateMachines => step_functions(app),
        Service::SqsQueues => sqs(app),
        Service::SnsTopics => sns(app),
        Service::SecretsManagerSecrets => secrets(app),
        Service::RdsInstances => rds(app),
        Service::VpcVpcs => vpc(app),
        Service::Ec2Instances => ec2(app),
        Service::KmsKeys => kms(app),
        Service::EfsFileSystems => efs(app),
        Service::FsxFileSystems => fsx(app),
        Service::S3Buckets => s3(app),
        Service::EcrRepositories => ecr(app),
        // Any other service is owned by another worker; defer to the caller.
        _ => Ok(None),
    }
}

// ── Step Functions ─────────────────────────────────────────────────────────
// Top inventory → list-state-machines (EXPRESS machines included; no selection
// needed). Entered machine → list-executions scoped to the parent ARN, but only
// for STANDARD machines — EXPRESS execution history is not listable, so that is
// an explicit error rather than a silent fall-through.
fn step_functions(app: &App) -> Result<Option<Target>, String> {
    let executions = &app.stepfunctions_state.executions;
    if let Some(machine) = &executions.machine {
        if machine.machine_type != "STANDARD" {
            return Err(
                "Execution listing is supported only for Standard workflows; EXPRESS execution \
                 history is not available via ListExecutions"
                    .into(),
            );
        }
        let region = machine
            .arn
            .split(':')
            .nth(3)
            .unwrap_or_default()
            .to_string();
        return Ok(Some(view(
            "stepfunctions list-executions",
            json!({ "stateMachineArn": machine.arn }),
            None,
            Some(scope_resource(
                ResourceKind::StateMachine,
                &machine.name,
                &machine.arn,
                &region,
            )),
        )));
    }
    Ok(Some(view(
        "stepfunctions list-state-machines",
        json!({}),
        None,
        None,
    )))
}

// ── SNS ──────────────────────────────────────────────────────────────────────
// Top inventory → list-topics. Entered topic → list-subscriptions-by-topic for
// the parent topic ARN.
fn sns(app: &App) -> Result<Option<Target>, String> {
    if let Some(topic) = &app.sns_state.subscriptions.topic {
        let region = topic.arn.split(':').nth(3).unwrap_or_default().to_string();
        return Ok(Some(view(
            "sns list-subscriptions-by-topic",
            json!({ "TopicArn": topic.arn }),
            None,
            Some(scope_resource(
                ResourceKind::Topic,
                &topic.name,
                &topic.arn,
                &region,
            )),
        )));
    }
    Ok(Some(view("sns list-topics", json!({}), None, None)))
}

// ── Secrets Manager ───────────────────────────────────────────────────────────
// Only a top inventory of secrets; no entered table. list-secrets, unscoped.
fn secrets(_app: &App) -> Result<Option<Target>, String> {
    Ok(Some(view(
        "secretsmanager list-secrets",
        json!({}),
        None,
        None,
    )))
}

// ── RDS ────────────────────────────────────────────────────────────────────────
// Databases table → describe-db-instances with no DBInstanceIdentifier (the whole
// collection). No entered table.
fn rds(_app: &App) -> Result<Option<Target>, String> {
    Ok(Some(view(
        "rds describe-db-instances",
        json!({}),
        None,
        None,
    )))
}

// ── VPC ────────────────────────────────────────────────────────────────────────
// VPCs table → describe-vpcs with no VpcIds. No entered table.
fn vpc(_app: &App) -> Result<Option<Target>, String> {
    Ok(Some(view("ec2 describe-vpcs", json!({}), None, None)))
}

// ── EC2 ────────────────────────────────────────────────────────────────────────
// Instances table → describe-instances with no InstanceIds. The view's state
// filter is a CLIENT-side filter over already-loaded rows, NOT a server-side AWS
// Filter, so it is deliberately not translated into a `--filters` argument (that
// would silently change the command's meaning). The entered instance detail is a
// summary view → None.
fn ec2(app: &App) -> Result<Option<Target>, String> {
    if let Some(instance) = &app.ec2_state.current_instance {
        if app.ec2_state.detail_tab == crate::ui::ec2::DetailTab::Tags {
            return Ok(Some(view(
                "ec2 describe-tags",
                json!({"Filters":[{"Name":"resource-id","Values":[instance]}]}),
                None,
                None,
            )));
        }
        return Ok(None);
    }
    Ok(Some(view("ec2 describe-instances", json!({}), None, None)))
}

// ── KMS ────────────────────────────────────────────────────────────────────────
// Top inventory → list-keys. Entered key detail: the Aliases and Tags tabs are
// tables with representable per-key queries (ListAliases / ListResourceTags,
// both scoped by the full key ARN). The General, Rotation and Policy tabs are
// summary/JSON panes → None.
fn kms(app: &App) -> Result<Option<Target>, String> {
    use crate::kms::detail::Tab;
    let detail = &app.kms_state.detail;
    if let Some(source) = &detail.source {
        let key_arn = source.key_arn.clone();
        let region = key_arn.split(':').nth(3).unwrap_or_default().to_string();
        let scope = Some(scope_resource(
            ResourceKind::Key,
            &source.key_id,
            &key_arn,
            &region,
        ));
        let tab = detail.view().map(|v| v.tab).unwrap_or(Tab::General);
        return match tab {
            Tab::Aliases => Ok(Some(view(
                "kms list-aliases",
                json!({ "KeyId": key_arn }),
                None,
                scope,
            ))),
            Tab::Tags => Ok(Some(view(
                "kms list-resource-tags",
                json!({ "KeyId": key_arn }),
                None,
                scope,
            ))),
            // General configuration, Key rotation and Key policy are detail panes.
            Tab::General | Tab::Rotation | Tab::Policy => Ok(None),
        };
    }
    Ok(Some(view("kms list-keys", json!({}), None, None)))
}

// ── EFS ────────────────────────────────────────────────────────────────────────
// Top inventory → describe-file-systems (no FileSystemId). Entered file system:
//   * Access points → describe-access-points (FileSystemId)
//   * Network (mount targets) → describe-mount-targets (FileSystemId)
//   * Tags → list-tags-for-resource (ResourceId = file system id)
// The Metered size, Monitoring, File system policy and Replication tabs are
// summary/JSON panes → None.
fn efs(app: &App) -> Result<Option<Target>, String> {
    use crate::ui::efs::DetailTab;
    if let Some(fs_id) = &app.efs_state.current_file_system {
        let fs = app
            .efs_state
            .file_systems
            .items
            .iter()
            .find(|f| &f.file_system_id == fs_id);
        let arn = fs.map(|f| f.file_system_arn.as_str()).unwrap_or_default();
        let scope = Some(scope_resource(ResourceKind::Efs, fs_id, arn, ""));
        return match app.efs_state.detail_tab {
            DetailTab::AccessPoints => Ok(Some(view(
                "efs describe-access-points",
                json!({ "FileSystemId": fs_id }),
                None,
                scope,
            ))),
            DetailTab::Network => Ok(Some(view(
                "efs describe-mount-targets",
                json!({ "FileSystemId": fs_id }),
                None,
                scope,
            ))),
            DetailTab::Tags => Ok(Some(view(
                "efs list-tags-for-resource",
                json!({ "ResourceId": fs_id }),
                None,
                scope,
            ))),
            DetailTab::MeteredSize
            | DetailTab::Monitoring
            | DetailTab::FileSystemPolicy
            | DetailTab::Replication => Ok(None),
        };
    }
    Ok(Some(view(
        "efs describe-file-systems",
        json!({}),
        None,
        None,
    )))
}

// ── FSx ────────────────────────────────────────────────────────────────────────
// Top inventory → describe-file-systems (no FileSystemIds). Entered file system:
//   * Backups → describe-backups with a server-side file-system-id filter (the
//     only entered FSx table with a representable standalone query).
//   * Tags and Updates (administrative actions) are tables assembled FROM the
//     parent DescribeFileSystems response; there is no standalone list API for
//     them, so they are unrepresentable aggregate tables → Err (never a
//     selected-row fall-through).
// The remaining detail tabs are summary panes → None.
fn fsx(app: &App) -> Result<Option<Target>, String> {
    use crate::ui::fsx::DetailTab;
    if let Some(fs_id) = &app.fsx_state.current_file_system {
        let fs = app
            .fsx_state
            .file_systems
            .items
            .iter()
            .find(|f| &f.file_system_id == fs_id);
        let arn = fs.map(|f| f.file_system_arn.as_str()).unwrap_or_default();
        let scope = Some(scope_resource(ResourceKind::Fsx, fs_id, arn, ""));
        return match app.fsx_state.detail_tab {
            DetailTab::Backups => Ok(Some(view(
                "fsx describe-backups",
                json!({ "Filters": [{ "Name": "file-system-id", "Values": [fs_id] }] }),
                None,
                scope,
            ))),
            DetailTab::Tags => {
                if arn.is_empty() {
                    return Err(
                        "Refresh the file system to load its ARN before listing tags".into(),
                    );
                }
                Ok(Some(view(
                    "fsx list-tags-for-resource",
                    json!({"ResourceARN":arn}),
                    None,
                    scope,
                )))
            }
            DetailTab::Updates => Ok(Some(view(
                "fsx describe-file-systems",
                json!({"FileSystemIds":[fs_id]}),
                None,
                scope,
            ))),
            DetailTab::NetworkSecurity
            | DetailTab::MonitoringPerformance
            | DetailTab::Administration
            | DetailTab::DataRepository => Ok(None),
        };
    }
    Ok(Some(view(
        "fsx describe-file-systems",
        json!({}),
        None,
        None,
    )))
}

// ── S3 ─────────────────────────────────────────────────────────────────────────
// Top inventory → list-buckets (region None; a global list). Entered bucket on the
// Objects tab → list-objects-v2 with the bucket's own region override and the same
// Bucket/Prefix/Delimiter the UI backend uses, UNLESS a single object's preview is
// open — that is a detail view, so we defer to the caller's head-object command.
// Other object tabs are detail/summary panes → None.
fn s3(app: &App) -> Result<Option<Target>, String> {
    use crate::ui::s3::ObjectTab;
    let Some(bucket) = &app.s3_state.current_bucket else {
        return Ok(Some(view("s3api list-buckets", json!({}), None, None)));
    };
    if app.s3_state.object_tab != ObjectTab::Objects {
        // Properties / Metadata / Permissions / … are detail panes.
        return Ok(None);
    }
    // An open object preview is a single-object detail; preserve head-object.
    if app.s3_state.object_view.preview.is_some() {
        return Ok(None);
    }
    let region = app
        .s3_state
        .buckets
        .items
        .iter()
        .find(|item| &item.name == bucket)
        .map(|item| item.region.clone())
        .unwrap_or_default();
    let region = if region.is_empty() || region == "global" {
        None
    } else {
        Some(region)
    };
    // The current prefix (if the user has descended into a folder). The UI object
    // FILTER is a client-side text filter and is deliberately NOT passed as an
    // S3 key prefix — only the navigated prefix stack scopes the request.
    let mut input = json!({ "Bucket": bucket, "Delimiter": "/" });
    if let Some(prefix) = app.s3_state.prefix_stack.last() {
        if !prefix.is_empty() {
            input["Prefix"] = json!(prefix);
        }
    }
    Ok(Some(view(
        "s3api list-objects-v2",
        input,
        region,
        Some(scope_resource(ResourceKind::Bucket, bucket, "", "")),
    )))
}

// ── ECR ────────────────────────────────────────────────────────────────────────
// Top repository list:
//   * Private tab → describe-repositories (no repositoryNames), current region.
//   * Public tab → ecr-public describe-repositories, always us-east-1.
// Entered repository (Private):
//   * The images table → describe-images for the repository WITHOUT imageIds (the
//     whole collection).
//   * An open image's scan detail: the Scanning / Referrers / Replication /
//     Signing tabs are tables scoped to the parent image digest (Replication
//     preserves the source-tag selector when one was chosen); the General
//     information tab is an image summary → None.
// Entered repository on the Public tab has no supported image command → Err.
fn ecr(app: &App) -> Result<Option<Target>, String> {
    use crate::ecr::scan::DetailTab;
    use crate::ui::ecr::Tab;

    if app.ecr_state.current_repository.is_none() {
        return match app.ecr_state.tab {
            Tab::Private => Ok(Some(view(
                "ecr describe-repositories",
                json!({}),
                None,
                None,
            ))),
            Tab::Public => Ok(Some(view(
                "ecr-public describe-repositories",
                json!({}),
                Some("us-east-1".into()),
                None,
            ))),
        };
    }

    let repository = app.ecr_state.current_repository.clone().unwrap();
    if app.ecr_state.tab != Tab::Private {
        if app.ecr_state.scan.view.is_some() {
            return Err("Public ECR image-detail commands are not supported".into());
        }
        return Ok(Some(view(
            "ecr-public describe-images",
            json!({"repositoryName":repository}),
            Some("us-east-1".into()),
            None,
        )));
    }

    // An open image scan view: the detail-table tabs map to digest-scoped APIs.
    if let Some(scan) = &app.ecr_state.scan.view {
        let digest = scan.digest.clone();
        if digest.is_empty() {
            return Err("Image digest unavailable; return to the image list and refresh".into());
        }
        let image_id = json!({ "imageDigest": digest });
        return match scan.tab {
            DetailTab::Scanning => Ok(Some(view(
                "ecr describe-image-scan-findings",
                json!({ "repositoryName": repository, "imageId": image_id }),
                None,
                None,
            ))),
            DetailTab::Referrers => Ok(Some(view(
                "ecr list-image-referrers",
                json!({ "repositoryName": repository, "subjectId": image_id }),
                None,
                None,
            ))),
            DetailTab::Replication => {
                let image_id = match scan.source_tag() {
                    Some(tag) => json!({"imageTag":tag}),
                    None => image_id,
                };
                Ok(Some(view(
                    "ecr describe-image-replication-status",
                    json!({ "repositoryName": repository, "imageId": image_id }),
                    None,
                    None,
                )))
            }
            DetailTab::Signing => Ok(Some(view(
                "ecr describe-image-signing-status",
                json!({ "repositoryName": repository, "imageId": image_id }),
                None,
                None,
            ))),
            // General information is an image summary pane.
            DetailTab::General => Ok(None),
        };
    }

    // The images table for the entered repository (no image opened).
    Ok(Some(view(
        "ecr describe-images",
        json!({ "repositoryName": repository }),
        None,
        None,
    )))
}

fn sqs(app: &App) -> Result<Option<Target>, String> {
    use crate::ui::sqs::QueueDetailTab;
    let Some(queue) = &app.sqs_state.current_queue else {
        return Ok(Some(view("sqs list-queues", json!({}), None, None)));
    };
    let row = app
        .sqs_state
        .queues
        .items
        .iter()
        .find(|row| &row.url == queue || &row.name == queue)
        .ok_or("Refresh the entered queue to load its URL")?;
    let queue = &row.url;
    let (host, path) = queue
        .strip_prefix("https://")
        .and_then(|s| s.split_once('/'))
        .ok_or("The queue URL is malformed")?;
    let suffix = if app.config.region.starts_with("cn-") {
        "amazonaws.com.cn"
    } else {
        "amazonaws.com"
    };
    if host != format!("sqs.{}.{suffix}", app.config.region) {
        return Err("The queue URL does not match the active Region's AWS endpoint".into());
    }
    let (account, name) = path.split_once('/').ok_or("The queue URL is malformed")?;
    if account.len() != 12
        || !account.bytes().all(|c| c.is_ascii_digit())
        || name.is_empty()
        || name.contains('/')
    {
        return Err("The queue URL identity is malformed".into());
    }
    let arn = format!(
        "arn:{}:sqs:{}:{account}:{name}",
        crate::console::partition(&app.config.region),
        app.config.region
    );
    let parent = scope_resource(ResourceKind::Queue, name, &arn, &app.config.region);
    match app.sqs_state.detail_tab {
        QueueDetailTab::Tagging => Ok(Some(view("sqs list-queue-tags", json!({"QueueUrl":queue}), None, Some(parent)))),
        QueueDetailTab::LambdaTriggers => {
            if parent.arn.is_empty() { return Err("The entered queue ARN is unavailable".into()); }
            Ok(Some(view("lambda list-event-source-mappings", json!({"EventSourceArn":parent.arn}), None, Some(parent))))
        }
        QueueDetailTab::SnsSubscriptions | QueueDetailTab::EventBridgePipes => Err("This queue table combines discovery with client-side source matching; no single scoped CLI command is available".into()),
        QueueDetailTab::DeadLetterQueueRedriveTasks => {
            if parent.arn.is_empty() { return Err("The entered queue ARN is unavailable".into()); }
            Ok(Some(view("sqs list-message-move-tasks", json!({"SourceArn":parent.arn}), None, Some(parent))))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests;
