//! Open finder results through existing service loaders and views.
use crate::app::{App, Service, Tab, ViewMode};
use crate::keymap::Mode;
use crate::ui;
use anyhow::{bail, Result};
use rusticity_core::finder::{Resource, ResourceKind};

pub fn service(kind: ResourceKind) -> Service {
    match kind {
        ResourceKind::Api => Service::ApiGatewayApis,
        ResourceKind::Stack => Service::CloudFormationStacks,
        ResourceKind::Alarm => Service::CloudWatchAlarms,
        ResourceKind::LogGroup => Service::CloudWatchLogGroups,
        ResourceKind::Instance => Service::Ec2Instances,
        ResourceKind::Table => Service::DynamoDbTables,
        ResourceKind::Repository | ResourceKind::PublicRepository => Service::EcrRepositories,
        ResourceKind::Efs => Service::EfsFileSystems,
        ResourceKind::Fsx => Service::FsxFileSystems,
        ResourceKind::User => Service::IamUsers,
        ResourceKind::Role => Service::IamRoles,
        ResourceKind::Group => Service::IamUserGroups,
        ResourceKind::Key => Service::KmsKeys,
        ResourceKind::Function => Service::LambdaFunctions,
        ResourceKind::Database => Service::RdsInstances,
        ResourceKind::Bucket => Service::S3Buckets,
        ResourceKind::Queue => Service::SqsQueues,
    }
}

pub async fn open_resource(app: &mut App, resource: Resource) -> Result<()> {
    // The finder never silently switches credentials or the regional client set.
    if resource.region != app.config.region
        && resource.region != "global"
        && !resource.region.is_empty()
        && !matches!(
            resource.kind,
            ResourceKind::Bucket | ResourceKind::PublicRepository
        )
    {
        bail!(
            "Resource is in {}; switch Region before opening it",
            resource.region
        );
    }
    let target = service(resource.kind);
    app.current_service = target;
    app.service_selected = true;
    app.view_mode = ViewMode::List;
    app.mode = Mode::Normal;
    app.page_input.clear();
    app.tabs.push(Tab {
        service: target,
        title: target.name().into(),
        breadcrumb: target.name().into(),
    });
    app.current_tab = app.tabs.len() - 1;
    // No search filter is reused accidentally from an older visit to this service.
    macro_rules! focus {
        ($table:expr, $field:ident) => {{
            let table = &mut $table;
            if !table.items.iter().any(|row| row.$field == resource.id) {
                bail!(
                    "Resource no longer appears in the accessible inventory: {}",
                    resource.id
                );
            }
            table.reset();
            table.filter = resource.id.clone();
            table.selected = table
                .items
                .iter()
                .filter(|row| {
                    row.$field
                        .to_lowercase()
                        .contains(&resource.id.to_lowercase())
                })
                .position(|row| row.$field == resource.id)
                .unwrap_or(0);
        }};
    }
    match resource.kind {
        ResourceKind::Table => {
            app.dynamodb_state = ui::dynamodb::State::new();
            app.load_dynamodb_tables().await?;
            focus!(app.dynamodb_state.tables, name);
            app.dynamodb_state.tables.selected = ui::dynamodb::filtered_dynamodb_tables(app)
                .iter()
                .position(|row| row.name == resource.id)
                .unwrap_or(0);
            crate::dynamodb::actions::select_item(app);
            app.load_dynamodb_items().await?;
        }
        ResourceKind::Bucket => {
            app.s3_state = ui::s3::State::new();
            ui::s3::load_s3_buckets(app).await?;
            focus!(app.s3_state.buckets, name);
            app.s3_state.current_bucket = Some(resource.id.clone());
            if resource.region != "global" && !resource.region.is_empty() {
                if let Some(bucket) = app
                    .s3_state
                    .buckets
                    .items
                    .iter_mut()
                    .find(|b| b.name == resource.id)
                {
                    bucket.region = resource.region.clone();
                }
            }
            app.load_s3_objects().await?;
        }
        ResourceKind::Database => {
            app.load_rds_instances().await?;
            focus!(app.rds_state.instances, identifier);
            app.rds_state.instances.selected = ui::rds::filtered_instances(app)
                .iter()
                .position(|row| row.identifier == resource.id)
                .unwrap_or(0);
        }
        ResourceKind::Instance => {
            app.ec2_state.current_instance = None;
            app.ec2_state.state_filter = ui::ec2::StateFilter::AllStates;
            app.load_ec2_instances().await?;
            focus!(app.ec2_state.table, instance_id);
        }
        ResourceKind::Repository | ResourceKind::PublicRepository => {
            app.ecr_state.current_repository = None;
            app.ecr_state.tab = if resource.kind == ResourceKind::PublicRepository {
                ui::ecr::Tab::Public
            } else {
                ui::ecr::Tab::Private
            };
            app.load_ecr_repositories().await?;
            focus!(app.ecr_state.repositories, name);
        }
        ResourceKind::Efs => {
            app.efs_state.current_file_system = None;
            app.load_efs_file_systems().await?;
            focus!(app.efs_state.file_systems, file_system_id);
        }
        ResourceKind::Fsx => {
            app.fsx_state.current_file_system = None;
            app.load_fsx_file_systems().await?;
            focus!(app.fsx_state.file_systems, file_system_id);
        }
        ResourceKind::Key => {
            app.load_kms_keys().await?;
            let key = app
                .kms_state
                .keys
                .items
                .iter()
                .find(|k| k.key_id == resource.id)
                .ok_or_else(|| anyhow::anyhow!("Key no longer appears in inventory"))?;
            app.kms_state.tab = if key.key_manager.eq_ignore_ascii_case("aws") {
                ui::kms::Tab::AwsManaged
            } else {
                ui::kms::Tab::CustomerManaged
            };
            focus!(app.kms_state.keys, key_id);
        }
        ResourceKind::Function => {
            app.lambda_state.current_function = None;
            ui::lambda::load_lambda_functions(app).await?;
            focus!(app.lambda_state.table, name);
            app.lambda_state.table.selected = ui::lambda::filtered_lambda_functions(app)
                .iter()
                .position(|row| row.name == resource.id)
                .unwrap_or(0);
        }
        ResourceKind::Stack => {
            app.cfn_state.current_stack = None;
            app.cfn_state.view_nested = true;
            app.cfn_state.status_filter = ui::cfn::StatusFilter::All;
            app.load_cloudformation_stacks().await?;
            focus!(app.cfn_state.table, name);
            app.cfn_state.table.selected = ui::cfn::filtered_cloudformation_stacks(app)
                .iter()
                .position(|row| row.name == resource.id)
                .unwrap_or(0);
        }
        ResourceKind::Api => {
            app.apig_state.current_api = None;
            app.load_apis().await?;
            // API filtering is name-based, whereas the finder identifier is the API ID.
            let name = app
                .apig_state
                .apis
                .items
                .iter()
                .find(|a| a.id == resource.id)
                .map(|a| a.name.clone())
                .ok_or_else(|| anyhow::anyhow!("API no longer appears in inventory"))?;
            app.apig_state.apis.reset();
            app.apig_state.apis.filter = name;
            app.apig_state.apis.selected = ui::apig::filtered_apis(app)
                .iter()
                .position(|a| a.id == resource.id)
                .unwrap_or(0);
        }
        ResourceKind::Queue => {
            app.sqs_state.current_queue = None;
            ui::sqs::load_sqs_queues(app).await?;
            focus!(app.sqs_state.queues, name);
            app.sqs_state.queues.selected =
                ui::sqs::filtered_queues(&app.sqs_state.queues.items, &resource.id)
                    .iter()
                    .position(|row| row.name == resource.id)
                    .unwrap_or(0);
        }
        ResourceKind::User => {
            app.iam_state.current_user = None;
            ui::iam::load_iam_users(app).await?;
            focus!(app.iam_state.users, user_name);
        }
        ResourceKind::Role => {
            app.iam_state.current_role = None;
            ui::iam::load_iam_roles(app).await?;
            focus!(app.iam_state.roles, role_name);
        }
        ResourceKind::Group => {
            app.iam_state.current_group = None;
            ui::iam::load_iam_user_groups(app).await?;
            focus!(app.iam_state.groups, group_name);
        }
        ResourceKind::LogGroup => {
            app.log_groups_state.exact_match = true;
            app.load_log_groups().await?;
            focus!(app.log_groups_state.log_groups, name);
            app.log_groups_state.log_groups.selected = 0;
        }
        ResourceKind::Alarm => {
            app.alarms_state.current_alarm = None;
            app.alarms_state.alarm_tab = ui::cw::alarms::AlarmTab::AllAlarms;
            app.load_alarms().await?;
            focus!(app.alarms_state.table, name);
        }
    }
    app.tabs[app.current_tab].breadcrumb = app.breadcrumbs();
    Ok(())
}
