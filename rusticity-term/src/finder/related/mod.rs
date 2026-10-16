//! Identify the currently selected resource so `related::discover` can find
//! cross-service dependencies. No network calls — pure state inspection.
//!
//! The public API is [`selected`]: it maps the active pane/service to a
//! [`Resource`] by consulting existing filtered/sorted helpers and detail
//! "current_*" pointers. When a detail view is open the detail parent takes
//! precedence over any sub-table selection.
use crate::app::{App, Service};
use rusticity_core::finder::{Resource, ResourceKind};

#[cfg(test)]
mod tests;

/// Construct a [`Resource`] with an empty `arn`.  `Resource::new` is
/// `pub(super)` inside `rusticity_core` and therefore not callable from this
/// crate, so we build the struct literal directly here.
fn resource(kind: ResourceKind, id: &str, name: &str, region: &str) -> Resource {
    Resource {
        kind,
        id: id.to_string(),
        name: name.to_string(),
        arn: String::new(),
        region: region.to_string(),
    }
}

/// The currently selected resource, or `None` when there is no meaningful
/// top-level context (no service selected, empty/stale list, or services
/// whose details don't map to a finder resource kind).
///
/// `Resource.arn` is populated only when the underlying data model stores an
/// actual ARN; otherwise it is left empty and `id` + `region` are sufficient
/// for core scoped requests.
pub fn selected(app: &App) -> Option<Resource> {
    if !app.service_selected {
        return None;
    }

    let region = app.config.region.as_str();

    match app.current_service {
        Service::CliOutput(_) => None,
        // ── Services with explicit detail parent tracking ──
        Service::DynamoDbTables => {
            if let Some(name) = &app.dynamodb_state.current_table {
                return dynamodb_from_items(app, name, region);
            }
            let filtered = crate::ui::dynamodb::filtered_dynamodb_tables(app);
            let table = filtered.get(app.dynamodb_state.tables.selected)?;
            Some(Resource {
                kind: ResourceKind::Table,
                id: table.name.clone(),
                name: table.name.clone(),
                arn: table.table_arn.clone(),
                region: region.to_string(),
            })
        }

        Service::S3Buckets => {
            if let Some(name) = &app.s3_state.current_bucket {
                return Some(resource(ResourceKind::Bucket, name, name, region));
            }
            // Route through the canonical `ui::s3::filtered_buckets` helper so
            // the name filter (case-insensitive contains) matches the renderer
            // exactly. The GeneralPurpose-tab gate and the visual-row walk that
            // maps `selected_row` to its owning bucket stay here. Child rows
            // resolve to their owning bucket.
            let filtered = crate::ui::s3::filtered_buckets(
                &app.s3_state.buckets.items,
                &app.s3_state.buckets.filter,
            );
            if app.s3_state.bucket_type != crate::ui::s3::BucketType::GeneralPurpose {
                return None;
            }
            let mut row = 0;
            let bucket = filtered.into_iter().find(|bucket| {
                let start = row;
                row += 1;
                if app.s3_state.expanded_prefixes.contains(&bucket.name) {
                    row += app
                        .s3_state
                        .bucket_preview
                        .get(&bucket.name)
                        .map_or(1, |objects| {
                            crate::s3::actions::count_visual_rows_of_objects(
                                objects,
                                &app.s3_state.expanded_prefixes,
                                &app.s3_state.prefix_preview,
                            )
                        });
                }
                (start..row).contains(&app.s3_state.selected_row)
            })?;
            Some(resource(
                ResourceKind::Bucket,
                &bucket.name,
                &bucket.name,
                &bucket.region,
            ))
        }

        Service::Ec2Instances => {
            if let Some(id) = &app.ec2_state.current_instance {
                return ec2_from_items(app, id, region);
            }
            let filtered = crate::ui::ec2::filtered_ec2_instances(app);
            let instance = filtered.get(app.ec2_state.table.selected)?;
            Some(resource(
                ResourceKind::Instance,
                &instance.instance_id,
                &instance.name,
                region,
            ))
        }

        Service::SqsQueues => {
            if let Some(name) = &app.sqs_state.current_queue {
                return sqs_from_items(app, name, region);
            }
            let filtered = crate::ui::sqs::filtered_queues(
                &app.sqs_state.queues.items,
                &app.sqs_state.queues.filter,
            );
            let queue = filtered.get(app.sqs_state.queues.selected)?;
            sqs_from_items(app, &queue.name, region)
        }

        Service::LambdaFunctions => {
            if let Some(name) = &app.lambda_state.current_function {
                return lambda_from_items(app, name, region);
            }
            let filtered = crate::ui::lambda::filtered_lambda_functions(app);
            let func = filtered.get(app.lambda_state.table.selected)?;
            lambda_from_items(app, &func.name, region)
        }

        Service::CloudFormationStacks => {
            if let Some(name) = &app.cfn_state.current_stack {
                return cfn_from_items(app, name, region);
            }
            let filtered = crate::ui::cfn::filtered_cloudformation_stacks(app);
            let stack = filtered.get(app.cfn_state.table.selected)?;
            cfn_from_items(app, &stack.name, region)
        }

        Service::ApiGatewayApis => {
            if let Some(api) = &app.apig_state.current_api {
                return Some(resource(ResourceKind::Api, &api.id, &api.name, region));
            }
            let filtered = crate::ui::apig::filtered_apis(app);
            let api = filtered.get(app.apig_state.apis.selected)?;
            Some(resource(ResourceKind::Api, &api.id, &api.name, region))
        }

        Service::EfsFileSystems => {
            if let Some(id) = &app.efs_state.current_file_system {
                return efs_from_items(app, id, region);
            }
            let filtered = crate::ui::efs::filtered_efs_file_systems(app);
            let fs = filtered.get(app.efs_state.file_systems.selected)?;
            efs_from_items(app, &fs.file_system_id, region)
        }

        Service::FsxFileSystems => {
            if let Some(id) = &app.fsx_state.current_file_system {
                return fsx_from_items(app, id, region);
            }
            let filtered = crate::ui::fsx::filtered_fsx_file_systems(app);
            let fs = filtered.get(app.fsx_state.file_systems.selected)?;
            fsx_from_items(app, &fs.file_system_id, region)
        }

        Service::EcrRepositories => {
            let kind = ecr_kind(&app.ecr_state.tab);
            if let Some(name) = &app.ecr_state.current_repository {
                return Some(resource(kind, name, name, region));
            }
            let filtered = crate::ui::ecr::filtered_ecr_repositories(app);
            let repo = filtered.get(app.ecr_state.repositories.selected)?;
            Some(resource(kind, &repo.name, &repo.name, region))
        }

        Service::KmsKeys => {
            let filtered = crate::ui::kms::filtered_kms_keys(app);
            let key = app
                .kms_state
                .detail
                .source
                .as_ref()
                .or_else(|| filtered.get(app.kms_state.keys.selected).copied())?;
            Some(Resource {
                kind: ResourceKind::Key,
                id: key.key_id.clone(),
                name: key.alias.clone(),
                arn: key.key_arn.clone(),
                region: region.to_string(),
            })
        }

        Service::IamUsers => {
            if let Some(name) = &app.iam_state.current_user {
                return iam_user_from_items(app, name);
            }
            let filtered = crate::ui::iam::filtered_iam_users(app);
            let user = filtered.get(app.iam_state.users.selected)?;
            Some(Resource {
                kind: ResourceKind::User,
                id: user.user_name.clone(),
                name: user.user_name.clone(),
                arn: user.arn.clone(),
                region: "global".to_string(),
            })
        }

        Service::IamRoles => {
            if let Some(name) = &app.iam_state.current_role {
                return iam_role_from_items(app, name);
            }
            let filtered = crate::ui::iam::filtered_iam_roles(app);
            let role = filtered.get(app.iam_state.roles.selected)?;
            Some(Resource {
                kind: ResourceKind::Role,
                id: role.role_name.clone(),
                name: role.role_name.clone(),
                arn: role.arn.clone(),
                region: "global".to_string(),
            })
        }

        Service::IamUserGroups => {
            if let Some(name) = &app.iam_state.current_group {
                return Some(resource(ResourceKind::Group, name, name, "global"));
            }
            // Route through the canonical `ui::iam::filtered_iam_groups` helper
            // so the name filter (case-insensitive contains on group_name)
            // matches the renderer exactly.
            let filtered = crate::ui::iam::filtered_iam_groups(app);
            let group = filtered.get(app.iam_state.groups.selected)?;
            Some(resource(
                ResourceKind::Group,
                &group.group_name,
                &group.group_name,
                "global",
            ))
        }

        // ── Services with list-only (no detail parent) ──
        Service::RdsInstances => {
            let filtered = crate::ui::rds::filtered_instances(app);
            let db = filtered.get(app.rds_state.instances.selected)?;
            Some(Resource {
                kind: ResourceKind::Database,
                id: db.identifier.clone(),
                name: db.identifier.clone(),
                arn: db.arn.clone(),
                region: region.to_string(),
            })
        }

        Service::VpcVpcs => {
            let filtered = crate::ui::vpc::filtered_vpcs(app);
            let vpc = filtered.get(app.vpc_state.vpcs.selected)?;
            Some(Resource {
                kind: ResourceKind::Vpc,
                id: vpc.vpc_id.clone(),
                name: vpc.name.clone(),
                arn: vpc.arn.clone(),
                region: region.to_string(),
            })
        }

        Service::SnsTopics => {
            if let Some(topic) = &app.sns_state.subscriptions.topic {
                return Some(Resource {
                    kind: ResourceKind::Topic,
                    id: topic.name.clone(),
                    name: topic.name.clone(),
                    arn: topic.arn.clone(),
                    region: region.to_string(),
                });
            }
            let filtered = crate::ui::sns::filtered_topics(app);
            let topic = filtered.get(app.sns_state.topics.selected)?;
            Some(Resource {
                kind: ResourceKind::Topic,
                id: topic.name.clone(),
                name: topic.name.clone(),
                arn: topic.arn.clone(),
                region: region.to_string(),
            })
        }

        Service::StepFunctionsStateMachines => {
            let filtered = crate::ui::stepfunctions::filtered_machines(app);
            let machine = app
                .stepfunctions_state
                .executions
                .machine
                .as_ref()
                .or_else(|| {
                    filtered
                        .get(app.stepfunctions_state.machines.selected)
                        .copied()
                })?;
            Some(Resource {
                kind: ResourceKind::StateMachine,
                id: machine.arn.clone(),
                name: machine.name.clone(),
                arn: machine.arn.clone(),
                region: region.to_string(),
            })
        }
        Service::SecretsManagerSecrets => {
            // The id is the full ARN (its suffix disambiguates same-named
            // secrets); the SDK name is the display name.
            let filtered = crate::ui::secretsmanager::filtered_secrets(app);
            let secret = filtered.get(app.secretsmanager_state.secrets.selected)?;
            Some(Resource {
                kind: ResourceKind::Secret,
                id: secret.arn.clone(),
                name: secret.name.clone(),
                arn: secret.arn.clone(),
                region: region.to_string(),
            })
        }

        Service::CloudWatchLogGroups => {
            // When viewing streams/events (Detail/Events mode) the
            // `log_groups.selected` index was reset to the unfiltered items
            // position by the action handler, so we index into the raw items.
            // In List mode `selected` is an index into the filtered list.
            if app.view_mode == crate::app::ViewMode::Detail
                || app.view_mode == crate::app::ViewMode::Events
            {
                let group = app
                    .log_groups_state
                    .log_groups
                    .items
                    .get(app.log_groups_state.log_groups.selected)?;
                log_group_resource(group, region)
            } else {
                let filtered = crate::ui::cw::logs::filtered_log_groups(app);
                let group = filtered.get(app.log_groups_state.log_groups.selected)?;
                log_group_resource(group, region)
            }
        }

        Service::CloudWatchAlarms => {
            if let Some(name) = &app.alarms_state.current_alarm {
                let arn = alarm_arn_from_items(app, name);
                return Some(Resource {
                    kind: ResourceKind::Alarm,
                    id: name.clone(),
                    name: name.clone(),
                    arn,
                    region: region.to_string(),
                });
            }
            // Route through the canonical `ui::cw::alarms::filtered_alarms`
            // helper so the tab predicate (AllAlarms vs InAlarm state ==
            // "ALARM") and the case-insensitive name `contains` filter match
            // the renderer exactly.
            let filtered = crate::ui::cw::alarms::filtered_alarms(
                &app.alarms_state.table.items,
                app.alarms_state.alarm_tab,
                &app.alarms_state.table.filter,
            );
            let alarm = filtered.get(app.alarms_state.table.selected)?;
            Some(Resource {
                kind: ResourceKind::Alarm,
                id: alarm.name.clone(),
                name: alarm.name.clone(),
                arn: alarm.alarm_arn.clone(),
                region: region.to_string(),
            })
        }

        // ── Services that don't map to a finder ResourceKind ──
        Service::CloudTrailEvents => None,

        Service::CloudWatchInsights => None,

        Service::LambdaApplications => {
            // A Lambda Application is backed by a CloudFormation stack. The
            // `arn` field holds the stack_id (an ARN) from the CFN API
            // response. Only return a Resource when we have that explicit
            // stack ID; otherwise None.
            if let Some(name) = &app.lambda_application_state.current_application {
                let item = app
                    .lambda_application_state
                    .table
                    .items
                    .iter()
                    .find(|a| &a.name == name)?;
                return lambda_application_resource(item, region);
            }
            let filtered = crate::ui::lambda::filtered_lambda_applications(app);
            let item = filtered.get(app.lambda_application_state.table.selected)?;
            lambda_application_resource(item, region)
        }
    }
}

// ── Helpers that look up items by id to populate ARNs ───────────────────

fn dynamodb_from_items(app: &App, name: &str, region: &str) -> Option<Resource> {
    let table = app
        .dynamodb_state
        .tables
        .items
        .iter()
        .find(|t| t.name == name)?;
    Some(Resource {
        kind: ResourceKind::Table,
        id: table.name.clone(),
        name: table.name.clone(),
        arn: table.table_arn.clone(),
        region: region.to_string(),
    })
}

fn ec2_from_items(app: &App, id: &str, region: &str) -> Option<Resource> {
    let instance = app
        .ec2_state
        .table
        .items
        .iter()
        .find(|i| i.instance_id == id)?;
    Some(resource(
        ResourceKind::Instance,
        &instance.instance_id,
        &instance.name,
        region,
    ))
}

fn sqs_from_items(app: &App, name: &str, region: &str) -> Option<Resource> {
    let queue = app.sqs_state.queues.items.iter().find(|q| q.name == name)?;
    Some(resource(
        ResourceKind::Queue,
        &queue.name,
        &queue.name,
        region,
    ))
}

fn lambda_from_items(app: &App, name: &str, region: &str) -> Option<Resource> {
    let func = app
        .lambda_state
        .table
        .items
        .iter()
        .find(|f| f.name == name)?;

    // Build the qualified id and ARN when an alias or version is active.
    // Alias takes priority over version (matching Lambda's own precedence).
    let (id, arn) = if let Some(alias) = &app.lambda_state.current_alias {
        (format!("{name}:{alias}"), format!("{}:{alias}", func.arn))
    } else if let Some(version) = &app.lambda_state.current_version {
        (
            format!("{name}:{version}"),
            format!("{}:{version}", func.arn),
        )
    } else {
        (func.name.clone(), func.arn.clone())
    };

    Some(Resource {
        kind: ResourceKind::Function,
        id,
        name: func.name.clone(),
        arn,
        region: region.to_string(),
    })
}

fn cfn_from_items(app: &App, name: &str, region: &str) -> Option<Resource> {
    let stack = app.cfn_state.table.items.iter().find(|s| s.name == name)?;
    Some(Resource {
        kind: ResourceKind::Stack,
        id: stack.name.clone(),
        name: stack.name.clone(),
        arn: stack.stack_id.clone(),
        region: region.to_string(),
    })
}

fn efs_from_items(app: &App, id: &str, region: &str) -> Option<Resource> {
    let fs = app
        .efs_state
        .file_systems
        .items
        .iter()
        .find(|f| f.file_system_id == id)?;
    Some(Resource {
        kind: ResourceKind::Efs,
        id: fs.file_system_id.clone(),
        name: fs.name.clone(),
        arn: fs.file_system_arn.clone(),
        region: region.to_string(),
    })
}

fn fsx_from_items(app: &App, id: &str, region: &str) -> Option<Resource> {
    let fs = app
        .fsx_state
        .file_systems
        .items
        .iter()
        .find(|f| f.file_system_id == id)?;
    Some(Resource {
        kind: ResourceKind::Fsx,
        id: fs.file_system_id.clone(),
        name: fs.name.clone(),
        arn: fs.file_system_arn.clone(),
        region: region.to_string(),
    })
}

fn iam_user_from_items(app: &App, name: &str) -> Option<Resource> {
    let user = app
        .iam_state
        .users
        .items
        .iter()
        .find(|u| u.user_name == name)?;
    Some(Resource {
        kind: ResourceKind::User,
        id: user.user_name.clone(),
        name: user.user_name.clone(),
        arn: user.arn.clone(),
        region: "global".to_string(),
    })
}

fn iam_role_from_items(app: &App, name: &str) -> Option<Resource> {
    let role = app
        .iam_state
        .roles
        .items
        .iter()
        .find(|r| r.role_name == name)?;
    Some(Resource {
        kind: ResourceKind::Role,
        id: role.role_name.clone(),
        name: role.role_name.clone(),
        arn: role.arn.clone(),
        region: "global".to_string(),
    })
}

fn alarm_arn_from_items(app: &App, name: &str) -> String {
    app.alarms_state
        .table
        .items
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.alarm_arn.clone())
        .unwrap_or_default()
}

fn log_group_resource(group: &rusticity_core::LogGroup, region: &str) -> Option<Resource> {
    if group.name.is_empty() {
        return None;
    }
    Some(Resource {
        kind: ResourceKind::LogGroup,
        id: group.name.clone(),
        name: group.name.clone(),
        arn: group
            .log_group_arn
            .clone()
            .or_else(|| group.arn.clone())
            .unwrap_or_default(),
        region: region.to_string(),
    })
}

fn lambda_application_resource(
    item: &crate::lambda::Application,
    region: &str,
) -> Option<Resource> {
    // Only return a Resource when we have an explicit stack ID (stored in
    // the `arn` field, which is the CFN stack_id from the API response).
    if item.arn.is_empty() {
        return None;
    }
    Some(Resource {
        kind: ResourceKind::Stack,
        id: item.name.clone(),
        name: item.name.clone(),
        arn: item.arn.clone(),
        region: region.to_string(),
    })
}

fn ecr_kind(tab: &crate::ui::ecr::Tab) -> ResourceKind {
    if *tab == crate::ui::ecr::Tab::Public {
        ResourceKind::PublicRepository
    } else {
        ResourceKind::Repository
    }
}
