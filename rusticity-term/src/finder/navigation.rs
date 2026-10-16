//! Open finder results through existing service loaders and views.
use crate::app::{App, Service, Tab, ViewMode};
use crate::keymap::Mode;
use crate::ui;
use anyhow::{bail, Result};
use rusticity_core::finder::{Resource, ResourceKind};
use rusticity_core::LogGroup;

pub fn service(kind: ResourceKind) -> Service {
    match kind {
        ResourceKind::Reference => unreachable!("unsupported references have no service view"),
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
        ResourceKind::Vpc => Service::VpcVpcs,
        ResourceKind::Bucket => Service::S3Buckets,
        ResourceKind::Queue => Service::SqsQueues,
        ResourceKind::Topic => Service::SnsTopics,
        ResourceKind::Secret => Service::SecretsManagerSecrets,
        ResourceKind::StateMachine => Service::StepFunctionsStateMachines,
    }
}

pub fn validate_scope(config: &rusticity_core::AwsConfig, resource: &Resource) -> Result<()> {
    if resource.kind == ResourceKind::Reference {
        bail!("This resource type has no Rusticity view yet");
    }
    if let Some(account) = resource
        .arn
        .split(':')
        .nth(4)
        .filter(|account| !account.is_empty())
    {
        if !config.account_id.is_empty() && account != config.account_id {
            bail!("Resource is in account {account}; switch profile before opening it");
        }
    }
    // The finder never silently switches credentials or the regional client set.
    if resource.region != config.region
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
    Ok(())
}

fn start_resource_view(app: &mut App, kind: ResourceKind) {
    let target = service(kind);
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
}

fn open_log_group(app: &mut App, name: &str, lookup: Result<Option<LogGroup>>) -> Result<()> {
    // Resolve errors before changing tabs, selection, or the current service.
    let group = lookup?
        .filter(|group| group.name == name)
        .ok_or_else(|| anyhow::anyhow!("Log group not found or not visible: {name}"))?;
    let page_size = app.log_groups_state.log_groups.page_size;
    let stream_page_size = app.log_groups_state.stream_page_size;
    let mut state = ui::cw::logs::CloudWatchLogGroupsState::new();
    state.log_groups.page_size = page_size;
    state.stream_page_size = stream_page_size;
    state.log_groups.items = vec![group];
    state.log_groups.filter = name.into();
    state.exact_match = true;
    app.log_groups_state = state;
    start_resource_view(app, ResourceKind::LogGroup);
    app.tabs[app.current_tab].breadcrumb = app.breadcrumbs();
    Ok(())
}

pub async fn open_resource(app: &mut App, resource: Resource) -> Result<()> {
    validate_scope(&app.config, &resource)?;
    if resource.kind == ResourceKind::LogGroup {
        let lookup = app.cloudwatch_client.get_log_group(&resource.id).await;
        return open_log_group(app, &resource.id, lookup);
    }
    start_resource_view(app, resource.kind);
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
        ResourceKind::Reference => unreachable!("scope validation rejects unsupported references"),
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
        ResourceKind::Vpc => {
            app.load_vpcs().await?;
            focus!(app.vpc_state.vpcs, vpc_id);
            app.vpc_state.vpcs.selected = ui::vpc::filtered_vpcs(app)
                .iter()
                .position(|vpc| vpc.vpc_id == resource.id)
                .unwrap_or(0);
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
            app.kms_state.detail.close();
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
        ResourceKind::Topic => {
            app.sns_state.subscriptions.close();
            app.load_sns_topics().await?;
            focus!(app.sns_state.topics, name);
            app.sns_state.topics.selected = ui::sns::filtered_topics(app)
                .iter()
                .position(|row| row.name == resource.id)
                .unwrap_or(0);
        }
        ResourceKind::StateMachine => {
            app.stepfunctions_state.executions.close();
            app.load_stepfunctions().await?;
            focus!(app.stepfunctions_state.machines, arn);
            app.stepfunctions_state.machines.selected = ui::stepfunctions::filtered_machines(app)
                .iter()
                .position(|row| row.arn == resource.id)
                .unwrap_or(0);
        }
        ResourceKind::Secret => {
            // The id is the full ARN; focus on it and select the exact row via
            // the filtered helper so same-named secrets stay distinct.
            app.load_secrets().await?;
            focus!(app.secretsmanager_state.secrets, arn);
            app.secretsmanager_state.secrets.selected = ui::secretsmanager::filtered_secrets(app)
                .iter()
                .position(|row| row.arn == resource.id)
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
            unreachable!("exact log-group lookup is handled before opening a view")
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

#[cfg(test)]
mod log_group_navigation_tests {
    use super::*;
    use crate::common::PageSize;

    fn group(name: &str) -> LogGroup {
        LogGroup {
            name: name.into(),
            creation_time: None,
            stored_bytes: Some(13529),
            retention_days: Some(30),
            log_class: Some("STANDARD".into()),
            arn: None,
            log_group_arn: None,
            deletion_protection_enabled: Some(true),
        }
    }

    #[test]
    fn linked_log_group_outside_first_fifty_rows_opens_exact_target() {
        let mut app = App::new_without_client("test".into(), Some("us-east-1".into()));
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.mode = Mode::ResourceFinder;
        app.log_groups_state.log_groups.items = (0..50)
            .map(|i| group(&format!("/aws/earlier/{i}")))
            .collect();
        app.log_groups_state.log_groups.selected = 49;
        app.log_groups_state.log_groups.scroll_offset = 40;
        app.log_groups_state.log_groups.expanded_item = Some(49);
        app.log_groups_state.log_groups.page_size = PageSize::TwentyFive;
        app.log_groups_state.stream_filter = "previous-stream".into();
        app.log_groups_state.stream_page_size = 10;
        app.log_groups_state.loading = true;
        app.page_input = "9".into();
        let tabs = app.tabs.len();
        let name = "/aws/lambda/GrantFn";
        open_log_group(&mut app, name, Ok(Some(group(name)))).unwrap();
        assert_eq!(app.current_service, Service::CloudWatchLogGroups);
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.view_mode, ViewMode::List);
        assert_eq!(app.tabs.len(), tabs + 1);
        assert_eq!(app.log_groups_state.log_groups.selected, 0);
        assert_eq!(app.log_groups_state.log_groups.scroll_offset, 0);
        assert_eq!(app.log_groups_state.log_groups.expanded_item, None);
        assert_eq!(
            app.log_groups_state.log_groups.page_size,
            PageSize::TwentyFive
        );
        assert_eq!(app.log_groups_state.stream_page_size, 10);
        assert!(app.page_input.is_empty());
        assert!(app.log_groups_state.exact_match);
        assert!(app.log_groups_state.stream_filter.is_empty());
        assert!(!app.log_groups_state.loading);
        let rows = ui::cw::logs::filtered_log_groups(&app);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, name);
        assert_eq!(rows[0].stored_bytes, Some(13529));
    }

    #[test]
    fn failed_log_group_lookup_preserves_the_original_view_and_tabs() {
        for lookup in [
            Ok(None),
            Err(anyhow::anyhow!("AccessDenied")),
            Ok(Some(group("/aws/lambda/GrantFn-other"))),
        ] {
            let mut app = App::new_without_client("test".into(), None);
            app.current_service = Service::LambdaFunctions;
            app.service_selected = true;
            app.mode = Mode::ResourceFinder;
            app.log_groups_state.log_groups.items = vec![group("old")];
            app.log_groups_state.log_groups.filter = "old-filter".into();
            app.page_input = "2".into();
            let tabs = app.tabs.len();
            let current_tab = app.current_tab;
            assert!(open_log_group(&mut app, "/aws/lambda/GrantFn", lookup).is_err());
            assert_eq!(app.current_service, Service::LambdaFunctions);
            assert_eq!(app.mode, Mode::ResourceFinder);
            assert_eq!(app.tabs.len(), tabs);
            assert_eq!(app.current_tab, current_tab);
            assert_eq!(app.log_groups_state.log_groups.items[0].name, "old");
            assert_eq!(app.log_groups_state.log_groups.filter, "old-filter");
            assert_eq!(app.page_input, "2");
        }
    }
}
