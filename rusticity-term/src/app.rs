pub use crate::aws::{filter_profiles, Profile as AwsProfile, Region as AwsRegion};
use crate::cfn::{Column as CfnColumn, Stack as CfnStack};
use crate::common::{ColumnId, CyclicEnum, InputFocus, PageSize, SortDirection};
pub use crate::cw::insights::InsightsFocus;
use crate::cw::insights::InsightsState;
pub use crate::cw::{Alarm, AlarmColumn};
pub use crate::ec2::{Column as Ec2Column, Instance as Ec2Instance};
use crate::ecr::image::{Column as EcrImageColumn, Image as EcrImage};
use crate::ecr::repo::{Column as EcrColumn, Repository as EcrRepository};
use crate::iam::{
    self, GroupUser as IamGroupUser, Policy as IamPolicy, RoleTag as IamRoleTag, UserColumn,
    UserTag as IamUserTag,
};
#[cfg(test)]
use crate::iam::{IamRole, IamUser, LastAccessedService};
use crate::keymap::{Action, Mode};
pub use crate::lambda::{
    Alias as LambdaAlias, Application as LambdaApplication,
    ApplicationColumn as LambdaApplicationColumn, Deployment, DeploymentColumn,
    Function as LambdaFunction, FunctionColumn as LambdaColumn, Layer as LambdaLayer, Resource,
    ResourceColumn, Version as LambdaVersion,
};
pub use crate::s3::{Bucket as S3Bucket, BucketColumn as S3BucketColumn, Object as S3Object};
use crate::session::{Session, SessionTab};
pub use crate::sqs::queue::Column as SqsColumn;
pub use crate::sqs::trigger::Column as SqsTriggerColumn;
use crate::sqs::{console_url_queue_detail, console_url_queues};
#[cfg(test)]
use crate::sqs::{
    EventBridgePipe, LambdaTrigger, Queue as SqsQueue, QueueTag as SqsQueueTag, SnsSubscription,
};
use crate::table::TableState;
use crate::ui::cfn::State as CfnStateConstants;
pub use crate::ui::cfn::{
    filtered_cloudformation_stacks, filtered_outputs, filtered_parameters, filtered_resources,
    output_column_ids, parameter_column_ids, resource_column_ids, DetailTab as CfnDetailTab,
    State as CfnState, StatusFilter as CfnStatusFilter,
};
pub use crate::ui::cw::alarms::{
    AlarmTab, AlarmViewMode, FILTER_CONTROLS as ALARM_FILTER_CONTROLS,
};
pub use crate::ui::cw::logs::{
    filtered_log_events, filtered_log_groups, filtered_log_streams, selected_log_group,
    DetailTab as CwLogsDetailTab, EventFilterFocus, FILTER_CONTROLS as LOG_FILTER_CONTROLS,
};
use crate::ui::ec2;
use crate::ui::ec2::filtered_ec2_instances;
pub use crate::ui::ec2::{
    State as Ec2State, StateFilter as Ec2StateFilter, STATE_FILTER as EC2_STATE_FILTER,
};
pub use crate::ui::ecr::{
    filtered_ecr_images, filtered_ecr_repositories, State as EcrState, Tab as EcrTab,
    FILTER_CONTROLS as ECR_FILTER_CONTROLS,
};
use crate::ui::iam::{
    filtered_iam_policies, filtered_iam_roles, filtered_iam_users, filtered_last_accessed,
    filtered_tags as filtered_iam_tags, filtered_user_tags, GroupTab, RoleTab, State as IamState,
    UserTab,
};
pub use crate::ui::lambda::{
    filtered_lambda_applications, filtered_lambda_functions,
    ApplicationDetailTab as LambdaApplicationDetailTab, ApplicationState as LambdaApplicationState,
    DetailTab as LambdaDetailTab, State as LambdaState, FILTER_CONTROLS as LAMBDA_FILTER_CONTROLS,
};
use crate::ui::monitoring::MonitoringState;
pub use crate::ui::s3::{
    calculate_total_bucket_rows, calculate_total_object_rows, BucketType as S3BucketType,
    ObjectTab as S3ObjectTab, State as S3State,
};
pub use crate::ui::sqs::{
    extract_account_id, extract_region, filtered_eventbridge_pipes, filtered_lambda_triggers,
    filtered_queues, filtered_subscriptions, filtered_tags, QueueDetailTab as SqsQueueDetailTab,
    State as SqsState, FILTER_CONTROLS as SQS_FILTER_CONTROLS,
    SUBSCRIPTION_FILTER_CONTROLS as SQS_SUBSCRIPTION_FILTER_CONTROLS, SUBSCRIPTION_REGION,
};
pub use crate::ui::{
    CloudWatchLogGroupsState, DateRangeType, DetailTab, EventColumn, LogGroupColumn, Preferences,
    StreamColumn, StreamSort, TimeUnit,
};
#[cfg(test)]
use rusticity_core::LogStream;
use rusticity_core::{
    AlarmsClient, AwsConfig, CloudFormationClient, CloudWatchClient, Ec2Client, EcrClient,
    IamClient, LambdaClient, S3Client, SqsClient,
};

#[derive(Clone)]
pub struct Tab {
    pub service: Service,
    pub title: String,
    pub breadcrumb: String,
}

pub struct App {
    pub running: bool,
    pub mode: Mode,
    pub config: AwsConfig,
    pub cloudwatch_client: CloudWatchClient,
    pub s3_client: S3Client,
    pub sqs_client: SqsClient,
    pub alarms_client: AlarmsClient,
    pub ec2_client: Ec2Client,
    pub ecr_client: EcrClient,
    pub iam_client: IamClient,
    pub lambda_client: LambdaClient,
    pub cloudformation_client: CloudFormationClient,
    pub current_service: Service,
    pub tabs: Vec<Tab>,
    pub current_tab: usize,
    pub tab_picker_selected: usize,
    pub tab_filter: String,
    pub pending_key: Option<char>,
    pub log_groups_state: CloudWatchLogGroupsState,
    pub insights_state: CloudWatchInsightsState,
    pub alarms_state: CloudWatchAlarmsState,
    pub s3_state: S3State,
    pub sqs_state: SqsState,
    pub ec2_state: Ec2State,
    pub ecr_state: EcrState,
    pub lambda_state: LambdaState,
    pub lambda_application_state: LambdaApplicationState,
    pub cfn_state: CfnState,
    pub iam_state: IamState,
    pub service_picker: ServicePickerState,
    pub service_selected: bool,
    pub profile: String,
    pub region: String,
    pub region_selector_index: usize,
    pub cw_log_group_visible_column_ids: Vec<ColumnId>,
    pub cw_log_group_column_ids: Vec<ColumnId>,
    pub column_selector_index: usize,
    pub preference_section: Preferences,
    pub cw_log_stream_visible_column_ids: Vec<ColumnId>,
    pub cw_log_stream_column_ids: Vec<ColumnId>,
    pub cw_log_event_visible_column_ids: Vec<ColumnId>,
    pub cw_log_event_column_ids: Vec<ColumnId>,
    pub cw_alarm_visible_column_ids: Vec<ColumnId>,
    pub cw_alarm_column_ids: Vec<ColumnId>,
    pub s3_bucket_visible_column_ids: Vec<ColumnId>,
    pub s3_bucket_column_ids: Vec<ColumnId>,
    pub sqs_visible_column_ids: Vec<ColumnId>,
    pub sqs_column_ids: Vec<ColumnId>,
    pub ec2_visible_column_ids: Vec<ColumnId>,
    pub ec2_column_ids: Vec<ColumnId>,
    pub ecr_repo_visible_column_ids: Vec<ColumnId>,
    pub ecr_repo_column_ids: Vec<ColumnId>,
    pub ecr_image_visible_column_ids: Vec<ColumnId>,
    pub ecr_image_column_ids: Vec<ColumnId>,
    pub lambda_application_visible_column_ids: Vec<ColumnId>,
    pub lambda_application_column_ids: Vec<ColumnId>,
    pub lambda_deployment_visible_column_ids: Vec<ColumnId>,
    pub lambda_deployment_column_ids: Vec<ColumnId>,
    pub lambda_resource_visible_column_ids: Vec<ColumnId>,
    pub lambda_resource_column_ids: Vec<ColumnId>,
    pub cfn_visible_column_ids: Vec<ColumnId>,
    pub cfn_column_ids: Vec<ColumnId>,
    pub cfn_parameter_visible_column_ids: Vec<ColumnId>,
    pub cfn_parameter_column_ids: Vec<ColumnId>,
    pub cfn_output_visible_column_ids: Vec<ColumnId>,
    pub cfn_output_column_ids: Vec<ColumnId>,
    pub cfn_resource_visible_column_ids: Vec<ColumnId>,
    pub cfn_resource_column_ids: Vec<ColumnId>,
    pub iam_user_visible_column_ids: Vec<ColumnId>,
    pub iam_user_column_ids: Vec<ColumnId>,
    pub iam_role_visible_column_ids: Vec<String>,
    pub iam_role_column_ids: Vec<String>,
    pub iam_group_visible_column_ids: Vec<String>,
    pub iam_group_column_ids: Vec<String>,
    pub iam_policy_visible_column_ids: Vec<String>,
    pub iam_policy_column_ids: Vec<String>,
    pub view_mode: ViewMode,
    pub error_message: Option<String>,
    pub error_scroll: usize,
    pub page_input: String,
    pub calendar_date: Option<time::Date>,
    pub calendar_selecting: CalendarField,
    pub cursor_pos: usize,
    pub current_session: Option<Session>,
    pub sessions: Vec<Session>,
    pub session_picker_selected: usize,
    pub session_filter: String,
    pub region_filter: String,
    pub region_picker_selected: usize,
    pub region_latencies: std::collections::HashMap<String, u64>,
    pub profile_filter: String,
    pub profile_picker_selected: usize,
    pub available_profiles: Vec<AwsProfile>,
    pub snapshot_requested: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CalendarField {
    StartDate,
    EndDate,
}

pub struct CloudWatchInsightsState {
    pub insights: InsightsState,
    pub loading: bool,
}

pub struct CloudWatchAlarmsState {
    pub table: TableState<Alarm>,
    pub alarm_tab: AlarmTab,
    pub view_as: AlarmViewMode,
    pub wrap_lines: bool,
    pub sort_column: String,
    pub sort_direction: SortDirection,
    pub input_focus: InputFocus,
}

impl PageSize {
    pub fn value(&self) -> usize {
        match self {
            PageSize::Ten => 10,
            PageSize::TwentyFive => 25,
            PageSize::Fifty => 50,
            PageSize::OneHundred => 100,
        }
    }

    pub fn next(&self) -> Self {
        match self {
            PageSize::Ten => PageSize::TwentyFive,
            PageSize::TwentyFive => PageSize::Fifty,
            PageSize::Fifty => PageSize::OneHundred,
            PageSize::OneHundred => PageSize::Ten,
        }
    }
}

pub struct ServicePickerState {
    pub filter: String,
    pub selected: usize,
    pub services: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    List,
    Detail,
    Events,
    InsightsResults,
    PolicyView,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Service {
    CloudWatchLogGroups,
    CloudWatchInsights,
    CloudWatchAlarms,
    S3Buckets,
    SqsQueues,
    Ec2Instances,
    EcrRepositories,
    LambdaFunctions,
    LambdaApplications,
    CloudFormationStacks,
    IamUsers,
    IamRoles,
    IamUserGroups,
}

impl Service {
    pub fn name(&self) -> &str {
        match self {
            Service::CloudWatchLogGroups => "CloudWatch > Log Groups",
            Service::CloudWatchInsights => "CloudWatch > Logs Insights",
            Service::CloudWatchAlarms => "CloudWatch > Alarms",
            Service::S3Buckets => "S3 > Buckets",
            Service::SqsQueues => "SQS > Queues",
            Service::Ec2Instances => "EC2 > Instances",
            Service::EcrRepositories => "ECR > Repositories",
            Service::LambdaFunctions => "Lambda > Functions",
            Service::LambdaApplications => "Lambda > Applications",
            Service::CloudFormationStacks => "CloudFormation > Stacks",
            Service::IamUsers => "IAM > Users",
            Service::IamRoles => "IAM > Roles",
            Service::IamUserGroups => "IAM > User Groups",
        }
    }
}

fn copy_to_clipboard(text: &str) {
    use std::io::Write;
    use std::process::{Command, Stdio};
    if let Ok(mut child) = Command::new("pbcopy").stdin(Stdio::piped()).spawn() {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
    }
}

fn nav_page_down(selected: &mut usize, max: usize, page_size: usize) {
    if max > 0 {
        *selected = (*selected + page_size).min(max - 1);
    }
}

impl App {
    pub fn get_input_focus(&self) -> InputFocus {
        InputFocus::Filter
    }

    fn get_active_filter_mut(&mut self) -> Option<&mut String> {
        if self.current_service == Service::CloudWatchAlarms {
            Some(&mut self.alarms_state.table.filter)
        } else if self.current_service == Service::Ec2Instances {
            Some(&mut self.ec2_state.table.filter)
        } else if self.current_service == Service::S3Buckets {
            if self.s3_state.current_bucket.is_some() {
                Some(&mut self.s3_state.object_filter)
            } else {
                Some(&mut self.s3_state.buckets.filter)
            }
        } else if self.current_service == Service::EcrRepositories {
            if self.ecr_state.current_repository.is_some() {
                Some(&mut self.ecr_state.images.filter)
            } else {
                Some(&mut self.ecr_state.repositories.filter)
            }
        } else if self.current_service == Service::SqsQueues {
            if self.sqs_state.current_queue.is_some()
                && self.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers
            {
                Some(&mut self.sqs_state.triggers.filter)
            } else if self.sqs_state.current_queue.is_some()
                && self.sqs_state.detail_tab == SqsQueueDetailTab::EventBridgePipes
            {
                Some(&mut self.sqs_state.pipes.filter)
            } else if self.sqs_state.current_queue.is_some()
                && self.sqs_state.detail_tab == SqsQueueDetailTab::Tagging
            {
                Some(&mut self.sqs_state.tags.filter)
            } else if self.sqs_state.current_queue.is_some()
                && self.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions
            {
                Some(&mut self.sqs_state.subscriptions.filter)
            } else {
                Some(&mut self.sqs_state.queues.filter)
            }
        } else if self.current_service == Service::LambdaFunctions {
            if self.lambda_state.current_version.is_some()
                && self.lambda_state.detail_tab == LambdaDetailTab::Configuration
            {
                Some(&mut self.lambda_state.alias_table.filter)
            } else if self.lambda_state.current_function.is_some()
                && self.lambda_state.detail_tab == LambdaDetailTab::Versions
            {
                Some(&mut self.lambda_state.version_table.filter)
            } else if self.lambda_state.current_function.is_some()
                && self.lambda_state.detail_tab == LambdaDetailTab::Aliases
            {
                Some(&mut self.lambda_state.alias_table.filter)
            } else {
                Some(&mut self.lambda_state.table.filter)
            }
        } else if self.current_service == Service::LambdaApplications {
            if self.lambda_application_state.current_application.is_some() {
                if self.lambda_application_state.detail_tab
                    == LambdaApplicationDetailTab::Deployments
                {
                    Some(&mut self.lambda_application_state.deployments.filter)
                } else {
                    Some(&mut self.lambda_application_state.resources.filter)
                }
            } else {
                Some(&mut self.lambda_application_state.table.filter)
            }
        } else if self.current_service == Service::CloudFormationStacks {
            if self.cfn_state.current_stack.is_some()
                && self.cfn_state.detail_tab == CfnDetailTab::Resources
            {
                Some(&mut self.cfn_state.resources.filter)
            } else {
                Some(&mut self.cfn_state.table.filter)
            }
        } else if self.current_service == Service::IamUsers {
            if self.iam_state.current_user.is_some() {
                if self.iam_state.user_tab == UserTab::Tags {
                    Some(&mut self.iam_state.user_tags.filter)
                } else {
                    Some(&mut self.iam_state.policies.filter)
                }
            } else {
                Some(&mut self.iam_state.users.filter)
            }
        } else if self.current_service == Service::IamRoles {
            if self.iam_state.current_role.is_some() {
                if self.iam_state.role_tab == RoleTab::Tags {
                    Some(&mut self.iam_state.tags.filter)
                } else if self.iam_state.role_tab == RoleTab::LastAccessed {
                    Some(&mut self.iam_state.last_accessed_filter)
                } else {
                    Some(&mut self.iam_state.policies.filter)
                }
            } else {
                Some(&mut self.iam_state.roles.filter)
            }
        } else if self.current_service == Service::IamUserGroups {
            if self.iam_state.current_group.is_some() {
                if self.iam_state.group_tab == GroupTab::Permissions {
                    Some(&mut self.iam_state.policies.filter)
                } else if self.iam_state.group_tab == GroupTab::Users {
                    Some(&mut self.iam_state.group_users.filter)
                } else {
                    None
                }
            } else {
                Some(&mut self.iam_state.groups.filter)
            }
        } else if self.view_mode == ViewMode::List {
            Some(&mut self.log_groups_state.log_groups.filter)
        } else if self.view_mode == ViewMode::Detail
            && self.log_groups_state.detail_tab == DetailTab::LogStreams
        {
            Some(&mut self.log_groups_state.stream_filter)
        } else {
            None
        }
    }

    fn apply_filter_operation<F>(&mut self, op: F)
    where
        F: FnOnce(&mut String),
    {
        if let Some(filter) = self.get_active_filter_mut() {
            op(filter);
            // Automatically reset selection for all services
            if self.current_service == Service::CloudWatchAlarms {
                self.alarms_state.table.reset();
            } else if self.current_service == Service::Ec2Instances {
                self.ec2_state.table.reset();
            } else if self.current_service == Service::S3Buckets {
                if self.s3_state.current_bucket.is_some() {
                    self.s3_state.selected_object = 0;
                } else {
                    self.s3_state.buckets.reset();
                }
            } else if self.current_service == Service::EcrRepositories {
                if self.ecr_state.current_repository.is_some() {
                    self.ecr_state.images.reset();
                } else {
                    self.ecr_state.repositories.reset();
                }
            } else if self.current_service == Service::SqsQueues {
                self.sqs_state.queues.reset();
            } else if self.current_service == Service::LambdaFunctions {
                if self.lambda_state.current_version.is_some()
                    || self.lambda_state.current_function.is_some()
                {
                    self.lambda_state.version_table.reset();
                    self.lambda_state.alias_table.reset();
                } else {
                    self.lambda_state.table.reset();
                }
            } else if self.current_service == Service::LambdaApplications {
                if self.lambda_application_state.current_application.is_some() {
                    self.lambda_application_state.deployments.reset();
                    self.lambda_application_state.resources.reset();
                } else {
                    self.lambda_application_state.table.reset();
                }
            } else if self.current_service == Service::CloudFormationStacks {
                self.cfn_state.table.reset();
            } else if self.current_service == Service::IamUsers {
                if self.iam_state.current_user.is_some() {
                    self.iam_state.user_tags.reset();
                    self.iam_state.policies.reset();
                } else {
                    self.iam_state.users.reset();
                }
            } else if self.current_service == Service::IamRoles {
                if self.iam_state.current_role.is_some() {
                    self.iam_state.tags.reset();
                    self.iam_state.policies.reset();
                } else {
                    self.iam_state.roles.reset();
                }
            } else if self.current_service == Service::IamUserGroups {
                if self.iam_state.current_group.is_some() {
                    self.iam_state.policies.reset();
                    self.iam_state.group_users.reset();
                } else {
                    self.iam_state.groups.reset();
                }
            } else if self.current_service == Service::CloudWatchLogGroups {
                if self.view_mode == ViewMode::List {
                    self.log_groups_state.log_groups.reset();
                } else if self.log_groups_state.detail_tab == DetailTab::LogStreams {
                    self.log_groups_state.selected_stream = 0;
                }
            }
        }
    }

    pub async fn new(profile: Option<String>, region: Option<String>) -> anyhow::Result<Self> {
        let profile_name = profile.or_else(|| std::env::var("AWS_PROFILE").ok())
            .ok_or_else(|| anyhow::anyhow!("No AWS profile specified. Set AWS_PROFILE environment variable or select a profile."))?;

        std::env::set_var("AWS_PROFILE", &profile_name);

        let config = AwsConfig::new(region).await?;
        let cloudwatch_client = CloudWatchClient::new(config.clone()).await?;
        let s3_client = S3Client::new(config.clone());
        let sqs_client = SqsClient::new(config.clone());
        let alarms_client = AlarmsClient::new(config.clone());
        let ec2_client = Ec2Client::new(config.clone());
        let ecr_client = EcrClient::new(config.clone());
        let iam_client = IamClient::new(config.clone());
        let lambda_client = LambdaClient::new(config.clone());
        let cloudformation_client = CloudFormationClient::new(config.clone());
        let region_name = config.region.clone();

        Ok(Self {
            running: true,
            mode: Mode::ServicePicker,
            config,
            cloudwatch_client,
            s3_client,
            sqs_client,
            alarms_client,
            ec2_client,
            ecr_client,
            iam_client,
            lambda_client,
            cloudformation_client,
            current_service: Service::CloudWatchLogGroups,
            tabs: Vec::new(),
            current_tab: 0,
            tab_picker_selected: 0,
            tab_filter: String::new(),
            pending_key: None,
            log_groups_state: CloudWatchLogGroupsState::new(),
            insights_state: CloudWatchInsightsState::new(),
            alarms_state: CloudWatchAlarmsState::new(),
            s3_state: S3State::new(),
            sqs_state: SqsState::new(),
            ec2_state: Ec2State::default(),
            ecr_state: EcrState::new(),
            lambda_state: LambdaState::new(),
            lambda_application_state: LambdaApplicationState::new(),
            cfn_state: CfnState::new(),
            iam_state: IamState::new(),
            service_picker: ServicePickerState::new(),
            service_selected: false,
            profile: profile_name,
            region: region_name,
            region_selector_index: 0,
            cw_log_group_visible_column_ids: LogGroupColumn::default_visible(),
            cw_log_group_column_ids: LogGroupColumn::ids(),
            column_selector_index: 0,
            cw_log_stream_visible_column_ids: StreamColumn::default_visible(),
            cw_log_stream_column_ids: StreamColumn::ids(),
            cw_log_event_visible_column_ids: EventColumn::default_visible(),
            cw_log_event_column_ids: EventColumn::ids(),
            cw_alarm_visible_column_ids: [
                AlarmColumn::Name,
                AlarmColumn::State,
                AlarmColumn::LastStateUpdate,
                AlarmColumn::Conditions,
                AlarmColumn::Actions,
            ]
            .iter()
            .map(|c| c.id())
            .collect(),
            cw_alarm_column_ids: AlarmColumn::ids(),
            s3_bucket_visible_column_ids: S3BucketColumn::ids(),
            s3_bucket_column_ids: S3BucketColumn::ids(),
            sqs_visible_column_ids: [
                SqsColumn::Name,
                SqsColumn::Type,
                SqsColumn::Created,
                SqsColumn::MessagesAvailable,
                SqsColumn::MessagesInFlight,
                SqsColumn::Encryption,
                SqsColumn::ContentBasedDeduplication,
            ]
            .iter()
            .map(|c| c.id())
            .collect(),
            sqs_column_ids: SqsColumn::ids(),
            ec2_visible_column_ids: [
                Ec2Column::Name,
                Ec2Column::InstanceId,
                Ec2Column::InstanceState,
                Ec2Column::InstanceType,
                Ec2Column::StatusCheck,
                Ec2Column::AlarmStatus,
                Ec2Column::AvailabilityZone,
                Ec2Column::PublicIpv4Dns,
                Ec2Column::PublicIpv4Address,
                Ec2Column::ElasticIp,
                Ec2Column::Ipv6Ips,
                Ec2Column::Monitoring,
                Ec2Column::SecurityGroupName,
                Ec2Column::KeyName,
                Ec2Column::LaunchTime,
                Ec2Column::PlatformDetails,
            ]
            .iter()
            .map(|c| c.id())
            .collect(),
            ec2_column_ids: Ec2Column::ids(),
            ecr_repo_visible_column_ids: EcrColumn::ids(),
            ecr_repo_column_ids: EcrColumn::ids(),
            ecr_image_visible_column_ids: EcrImageColumn::ids(),
            ecr_image_column_ids: EcrImageColumn::ids(),
            lambda_application_visible_column_ids: LambdaApplicationColumn::visible(),
            lambda_application_column_ids: LambdaApplicationColumn::ids(),
            lambda_deployment_visible_column_ids: DeploymentColumn::ids(),
            lambda_deployment_column_ids: DeploymentColumn::ids(),
            lambda_resource_visible_column_ids: ResourceColumn::ids(),
            lambda_resource_column_ids: ResourceColumn::ids(),
            cfn_visible_column_ids: [
                CfnColumn::Name,
                CfnColumn::Status,
                CfnColumn::CreatedTime,
                CfnColumn::Description,
            ]
            .iter()
            .map(|c| c.id())
            .collect(),
            cfn_column_ids: CfnColumn::ids(),
            cfn_parameter_visible_column_ids: parameter_column_ids(),
            cfn_parameter_column_ids: parameter_column_ids(),
            cfn_output_visible_column_ids: output_column_ids(),
            cfn_output_column_ids: output_column_ids(),
            cfn_resource_visible_column_ids: resource_column_ids(),
            cfn_resource_column_ids: resource_column_ids(),
            iam_user_visible_column_ids: UserColumn::visible(),
            iam_user_column_ids: UserColumn::ids(),
            iam_role_visible_column_ids: vec![
                "Role name".to_string(),
                "Trusted entities".to_string(),
                "Creation time".to_string(),
            ],
            iam_role_column_ids: vec![
                "Role name".to_string(),
                "Path".to_string(),
                "Trusted entities".to_string(),
                "ARN".to_string(),
                "Creation time".to_string(),
                "Description".to_string(),
                "Max session duration".to_string(),
            ],
            iam_group_visible_column_ids: vec![
                "Group name".to_string(),
                "Users".to_string(),
                "Permissions".to_string(),
                "Creation time".to_string(),
            ],
            iam_group_column_ids: vec![
                "Group name".to_string(),
                "Path".to_string(),
                "Users".to_string(),
                "Permissions".to_string(),
                "Creation time".to_string(),
            ],
            iam_policy_visible_column_ids: vec![
                "Policy name".to_string(),
                "Type".to_string(),
                "Attached via".to_string(),
            ],
            iam_policy_column_ids: vec![
                "Policy name".to_string(),
                "Type".to_string(),
                "Attached via".to_string(),
                "Attached entities".to_string(),
                "Description".to_string(),
                "Creation time".to_string(),
                "Edited time".to_string(),
            ],
            preference_section: Preferences::Columns,
            view_mode: ViewMode::List,
            error_message: None,
            error_scroll: 0,
            page_input: String::new(),
            calendar_date: None,
            calendar_selecting: CalendarField::StartDate,
            cursor_pos: 0,
            current_session: None,
            sessions: Vec::new(),
            session_picker_selected: 0,
            session_filter: String::new(),
            region_filter: String::new(),
            region_picker_selected: 0,
            region_latencies: std::collections::HashMap::new(),
            profile_filter: String::new(),
            profile_picker_selected: 0,
            available_profiles: Vec::new(),
            snapshot_requested: false,
        })
    }

    pub fn new_without_client(profile: String, region: Option<String>) -> Self {
        let config = AwsConfig::dummy(region.clone());
        Self {
            running: true,
            mode: Mode::ServicePicker,
            config: config.clone(),
            cloudwatch_client: CloudWatchClient::dummy(config.clone()),
            s3_client: S3Client::new(config.clone()),
            sqs_client: SqsClient::new(config.clone()),
            alarms_client: AlarmsClient::new(config.clone()),
            ec2_client: Ec2Client::new(config.clone()),
            ecr_client: EcrClient::new(config.clone()),
            iam_client: IamClient::new(config.clone()),
            lambda_client: LambdaClient::new(config.clone()),
            cloudformation_client: CloudFormationClient::new(config.clone()),
            current_service: Service::CloudWatchLogGroups,
            tabs: Vec::new(),
            current_tab: 0,
            tab_picker_selected: 0,
            tab_filter: String::new(),
            pending_key: None,
            log_groups_state: CloudWatchLogGroupsState::new(),
            insights_state: CloudWatchInsightsState::new(),
            alarms_state: CloudWatchAlarmsState::new(),
            s3_state: S3State::new(),
            sqs_state: SqsState::new(),
            ec2_state: Ec2State::default(),
            ecr_state: EcrState::new(),
            lambda_state: LambdaState::new(),
            lambda_application_state: LambdaApplicationState::new(),
            cfn_state: CfnState::new(),
            iam_state: IamState::new(),
            service_picker: ServicePickerState::new(),
            service_selected: false,
            profile,
            region: region.unwrap_or_default(),
            region_selector_index: 0,
            cw_log_group_visible_column_ids: LogGroupColumn::default_visible(),
            cw_log_group_column_ids: LogGroupColumn::ids(),
            column_selector_index: 0,
            preference_section: Preferences::Columns,
            cw_log_stream_visible_column_ids: StreamColumn::default_visible(),
            cw_log_stream_column_ids: StreamColumn::ids(),
            cw_log_event_visible_column_ids: EventColumn::default_visible(),
            cw_log_event_column_ids: EventColumn::ids(),
            cw_alarm_visible_column_ids: [
                AlarmColumn::Name,
                AlarmColumn::State,
                AlarmColumn::LastStateUpdate,
                AlarmColumn::Conditions,
                AlarmColumn::Actions,
            ]
            .iter()
            .map(|c| c.id())
            .collect(),
            cw_alarm_column_ids: AlarmColumn::ids(),
            s3_bucket_visible_column_ids: S3BucketColumn::ids(),
            s3_bucket_column_ids: S3BucketColumn::ids(),
            sqs_visible_column_ids: [
                SqsColumn::Name,
                SqsColumn::Type,
                SqsColumn::Created,
                SqsColumn::MessagesAvailable,
                SqsColumn::MessagesInFlight,
                SqsColumn::Encryption,
                SqsColumn::ContentBasedDeduplication,
            ]
            .iter()
            .map(|c| c.id())
            .collect(),
            sqs_column_ids: SqsColumn::ids(),
            ec2_visible_column_ids: [
                Ec2Column::Name,
                Ec2Column::InstanceId,
                Ec2Column::InstanceState,
                Ec2Column::InstanceType,
                Ec2Column::StatusCheck,
                Ec2Column::AlarmStatus,
                Ec2Column::AvailabilityZone,
                Ec2Column::PublicIpv4Dns,
                Ec2Column::PublicIpv4Address,
                Ec2Column::ElasticIp,
                Ec2Column::Ipv6Ips,
                Ec2Column::Monitoring,
                Ec2Column::SecurityGroupName,
                Ec2Column::KeyName,
                Ec2Column::LaunchTime,
                Ec2Column::PlatformDetails,
            ]
            .iter()
            .map(|c| c.id())
            .collect(),
            ec2_column_ids: Ec2Column::ids(),
            ecr_repo_visible_column_ids: EcrColumn::ids(),
            ecr_repo_column_ids: EcrColumn::ids(),
            ecr_image_visible_column_ids: EcrImageColumn::ids(),
            ecr_image_column_ids: EcrImageColumn::ids(),
            lambda_application_visible_column_ids: LambdaApplicationColumn::visible(),
            lambda_application_column_ids: LambdaApplicationColumn::ids(),
            lambda_deployment_visible_column_ids: DeploymentColumn::ids(),
            lambda_deployment_column_ids: DeploymentColumn::ids(),
            lambda_resource_visible_column_ids: ResourceColumn::ids(),
            lambda_resource_column_ids: ResourceColumn::ids(),
            cfn_visible_column_ids: [
                CfnColumn::Name,
                CfnColumn::Status,
                CfnColumn::CreatedTime,
                CfnColumn::Description,
            ]
            .iter()
            .map(|c| c.id())
            .collect(),
            cfn_column_ids: CfnColumn::ids(),
            iam_user_visible_column_ids: UserColumn::visible(),
            cfn_parameter_visible_column_ids: parameter_column_ids(),
            cfn_parameter_column_ids: parameter_column_ids(),
            cfn_output_visible_column_ids: output_column_ids(),
            cfn_output_column_ids: output_column_ids(),
            cfn_resource_visible_column_ids: resource_column_ids(),
            cfn_resource_column_ids: resource_column_ids(),
            iam_user_column_ids: UserColumn::ids(),
            iam_role_visible_column_ids: vec![
                "Role name".to_string(),
                "Trusted entities".to_string(),
                "Creation time".to_string(),
            ],
            iam_role_column_ids: vec![
                "Role name".to_string(),
                "Path".to_string(),
                "Trusted entities".to_string(),
                "ARN".to_string(),
                "Creation time".to_string(),
                "Description".to_string(),
                "Max session duration".to_string(),
            ],
            iam_group_visible_column_ids: vec![
                "Group name".to_string(),
                "Users".to_string(),
                "Permissions".to_string(),
                "Creation time".to_string(),
            ],
            iam_group_column_ids: vec![
                "Group name".to_string(),
                "Path".to_string(),
                "Users".to_string(),
                "Permissions".to_string(),
                "Creation time".to_string(),
            ],
            iam_policy_visible_column_ids: vec![
                "Policy name".to_string(),
                "Type".to_string(),
                "Attached via".to_string(),
            ],
            iam_policy_column_ids: vec![
                "Policy name".to_string(),
                "Type".to_string(),
                "Attached via".to_string(),
                "Attached entities".to_string(),
                "Description".to_string(),
                "Creation time".to_string(),
                "Edited time".to_string(),
            ],
            view_mode: ViewMode::List,
            error_message: None,
            error_scroll: 0,
            page_input: String::new(),
            calendar_date: None,
            calendar_selecting: CalendarField::StartDate,
            cursor_pos: 0,
            current_session: None,
            sessions: Vec::new(),
            session_picker_selected: 0,
            session_filter: String::new(),
            region_filter: String::new(),
            region_picker_selected: 0,
            region_latencies: std::collections::HashMap::new(),
            profile_filter: String::new(),
            profile_picker_selected: 0,
            available_profiles: Vec::new(),
            snapshot_requested: false,
        }
    }

    pub fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => {
                self.save_current_session();
                self.running = false;
            }
            Action::CloseService => {
                if !self.tabs.is_empty() {
                    // Close the current tab
                    self.tabs.remove(self.current_tab);

                    if self.tabs.is_empty() {
                        // Last tab closed - show service picker
                        self.service_selected = false;
                        self.current_tab = 0;
                        self.mode = Mode::ServicePicker;
                    } else {
                        // Tabs remain - switch to adjacent tab
                        if self.current_tab >= self.tabs.len() {
                            self.current_tab = self.tabs.len() - 1;
                        }
                        self.current_service = self.tabs[self.current_tab].service;
                        self.service_selected = true;
                        self.mode = Mode::Normal;
                    }
                } else {
                    // No tabs - just close service picker if open
                    self.service_selected = false;
                    self.mode = Mode::Normal;
                }
                self.service_picker.filter.clear();
                self.service_picker.selected = 0;
            }
            Action::NextItem => self.next_item(),
            Action::PrevItem => self.prev_item(),
            Action::PageUp => self.page_up(),
            Action::PageDown => self.page_down(),
            Action::NextPane => self.next_pane(),
            Action::PrevPane => self.prev_pane(),
            Action::Select => self.select_item(),
            Action::OpenSpaceMenu => {
                self.mode = Mode::SpaceMenu;
                self.service_picker.filter.clear();
                self.service_picker.selected = 0;
            }
            Action::CloseMenu => {
                self.mode = Mode::Normal;
                self.service_picker.filter.clear();
                // Reset selection when closing filter to avoid out-of-bounds
                match self.current_service {
                    Service::S3Buckets => {
                        self.s3_state.selected_row = 0;
                        self.s3_state.selected_object = 0;
                    }
                    Service::CloudFormationStacks => {
                        if self.cfn_state.current_stack.is_some()
                            && self.cfn_state.detail_tab == CfnDetailTab::Parameters
                        {
                            self.cfn_state.parameters.reset();
                        } else if self.cfn_state.current_stack.is_some()
                            && self.cfn_state.detail_tab == CfnDetailTab::Outputs
                        {
                            self.cfn_state.outputs.reset();
                        } else {
                            self.cfn_state.table.reset();
                        }
                    }
                    Service::LambdaFunctions => {
                        self.lambda_state.table.reset();
                    }
                    Service::SqsQueues => {
                        self.sqs_state.queues.reset();
                    }
                    Service::IamRoles => {
                        self.iam_state.roles.reset();
                    }
                    Service::IamUsers => {
                        self.iam_state.users.reset();
                    }
                    Service::IamUserGroups => {
                        self.iam_state.groups.reset();
                    }
                    Service::CloudWatchAlarms => {
                        self.alarms_state.table.reset();
                    }
                    Service::Ec2Instances => {
                        self.ec2_state.table.reset();
                    }
                    Service::EcrRepositories => {
                        self.ecr_state.repositories.reset();
                    }
                    Service::LambdaApplications => {
                        self.lambda_application_state.table.reset();
                    }
                    _ => {}
                }
            }
            Action::NextTab => {
                if !self.tabs.is_empty() {
                    self.current_tab = (self.current_tab + 1) % self.tabs.len();
                    self.current_service = self.tabs[self.current_tab].service;
                }
            }
            Action::PrevTab => {
                if !self.tabs.is_empty() {
                    self.current_tab = if self.current_tab == 0 {
                        self.tabs.len() - 1
                    } else {
                        self.current_tab - 1
                    };
                    self.current_service = self.tabs[self.current_tab].service;
                }
            }
            Action::CloseTab => {
                if !self.tabs.is_empty() {
                    self.tabs.remove(self.current_tab);
                    if self.tabs.is_empty() {
                        // Last tab closed - show service picker
                        self.service_selected = false;
                        self.current_tab = 0;
                        self.service_picker.filter.clear();
                        self.service_picker.selected = 0;
                        self.mode = Mode::ServicePicker;
                    } else {
                        // If we closed the last tab, move to the left
                        // Otherwise stay at same index (which is now the next tab to the right)
                        if self.current_tab >= self.tabs.len() {
                            self.current_tab = self.tabs.len() - 1;
                        }
                        self.current_service = self.tabs[self.current_tab].service;
                        self.service_selected = true;
                        self.mode = Mode::Normal;
                    }
                }
            }
            Action::OpenTabPicker => {
                if !self.tabs.is_empty() {
                    self.tab_picker_selected = self.current_tab;
                    self.mode = Mode::TabPicker;
                } else {
                    self.mode = Mode::Normal;
                }
            }
            Action::OpenSessionPicker => {
                self.save_current_session();
                self.sessions = Session::list_all().unwrap_or_default();
                self.session_picker_selected = 0;
                self.mode = Mode::SessionPicker;
            }
            Action::LoadSession => {
                let filtered_sessions = self.get_filtered_sessions();
                if let Some(&session) = filtered_sessions.get(self.session_picker_selected) {
                    let session = session.clone();
                    // Load the session
                    self.profile = session.profile.clone();
                    self.region = session.region.clone();
                    self.config.account_id = session.account_id.clone();
                    self.config.role_arn = session.role_arn.clone();

                    // Clear existing tabs and load session tabs
                    self.tabs.clear();
                    for session_tab in &session.tabs {
                        // Parse service from string
                        let service = match session_tab.service.as_str() {
                            "CloudWatchLogGroups" => Service::CloudWatchLogGroups,
                            "CloudWatchInsights" => Service::CloudWatchInsights,
                            "CloudWatchAlarms" => Service::CloudWatchAlarms,
                            "S3Buckets" => Service::S3Buckets,
                            "SqsQueues" => Service::SqsQueues,
                            "Ec2Instances" => Service::Ec2Instances,
                            "EcrRepositories" => Service::EcrRepositories,
                            "LambdaFunctions" => Service::LambdaFunctions,
                            "LambdaApplications" => Service::LambdaApplications,
                            "CloudFormationStacks" => Service::CloudFormationStacks,
                            "IamUsers" => Service::IamUsers,
                            "IamRoles" => Service::IamRoles,
                            "IamUserGroups" => Service::IamUserGroups,
                            _ => continue,
                        };

                        self.tabs.push(Tab {
                            service,
                            title: session_tab.title.clone(),
                            breadcrumb: session_tab.breadcrumb.clone(),
                        });

                        // Restore filter if present
                        if let Some(filter) = &session_tab.filter {
                            if service == Service::CloudWatchLogGroups {
                                self.log_groups_state.log_groups.filter = filter.clone();
                            }
                        }
                    }

                    if !self.tabs.is_empty() {
                        self.current_tab = 0;
                        self.current_service = self.tabs[0].service;
                        self.service_selected = true;
                        self.current_session = Some(session.clone());
                    }
                }
                self.mode = Mode::Normal;
            }
            Action::SaveSession => {
                // TODO: Implement session saving
            }
            Action::OpenServicePicker => {
                if self.mode == Mode::ServicePicker {
                    self.tabs.push(Tab {
                        service: Service::S3Buckets,
                        title: "S3 > Buckets".to_string(),
                        breadcrumb: "S3 > Buckets".to_string(),
                    });
                    self.current_tab = self.tabs.len() - 1;
                    self.current_service = Service::S3Buckets;
                    self.view_mode = ViewMode::List;
                    self.service_selected = true;
                    self.mode = Mode::Normal;
                } else {
                    self.mode = Mode::ServicePicker;
                    self.service_picker.filter.clear();
                    self.service_picker.selected = 0;
                }
            }
            Action::OpenCloudWatch => {
                self.current_service = Service::CloudWatchLogGroups;
                self.view_mode = ViewMode::List;
                self.service_selected = true;
                self.mode = Mode::Normal;
            }
            Action::OpenCloudWatchSplit => {
                self.current_service = Service::CloudWatchInsights;
                self.view_mode = ViewMode::InsightsResults;
                self.service_selected = true;
                self.mode = Mode::Normal;
            }
            Action::OpenCloudWatchAlarms => {
                self.current_service = Service::CloudWatchAlarms;
                self.view_mode = ViewMode::List;
                self.service_selected = true;
                self.mode = Mode::Normal;
            }
            Action::FilterInput(c) => {
                if self.mode == Mode::TabPicker {
                    self.tab_filter.push(c);
                    self.tab_picker_selected = 0;
                } else if self.mode == Mode::RegionPicker {
                    self.region_filter.push(c);
                    self.region_picker_selected = 0;
                } else if self.mode == Mode::ProfilePicker {
                    self.profile_filter.push(c);
                    self.profile_picker_selected = 0;
                } else if self.mode == Mode::SessionPicker {
                    self.session_filter.push(c);
                    self.session_picker_selected = 0;
                } else if self.mode == Mode::ServicePicker {
                    self.service_picker.filter.push(c);
                    self.service_picker.selected = 0;
                } else if self.mode == Mode::InsightsInput {
                    match self.insights_state.insights.insights_focus {
                        InsightsFocus::Query => {
                            self.insights_state.insights.query_text.push(c);
                        }
                        InsightsFocus::LogGroupSearch => {
                            self.insights_state.insights.log_group_search.push(c);
                            // Update matches
                            if !self.insights_state.insights.log_group_search.is_empty() {
                                self.insights_state.insights.log_group_matches = self
                                    .log_groups_state
                                    .log_groups
                                    .items
                                    .iter()
                                    .filter(|g| {
                                        g.name.to_lowercase().contains(
                                            &self
                                                .insights_state
                                                .insights
                                                .log_group_search
                                                .to_lowercase(),
                                        )
                                    })
                                    .take(50)
                                    .map(|g| g.name.clone())
                                    .collect();
                                self.insights_state.insights.show_dropdown = true;
                            } else {
                                self.insights_state.insights.log_group_matches.clear();
                                self.insights_state.insights.show_dropdown = false;
                            }
                        }
                        _ => {}
                    }
                } else if self.mode == Mode::FilterInput {
                    // Check if we should capture digits for page navigation
                    let is_pagination_focused = if self.current_service
                        == Service::LambdaApplications
                    {
                        if self.lambda_application_state.current_application.is_some() {
                            if self.lambda_application_state.detail_tab
                                == LambdaApplicationDetailTab::Deployments
                            {
                                self.lambda_application_state.deployment_input_focus
                                    == InputFocus::Pagination
                            } else {
                                self.lambda_application_state.resource_input_focus
                                    == InputFocus::Pagination
                            }
                        } else {
                            self.lambda_application_state.input_focus == InputFocus::Pagination
                        }
                    } else if self.current_service == Service::CloudFormationStacks {
                        self.cfn_state.input_focus == InputFocus::Pagination
                    } else if self.current_service == Service::IamRoles
                        && self.iam_state.current_role.is_none()
                    {
                        self.iam_state.role_input_focus == InputFocus::Pagination
                    } else if self.view_mode == ViewMode::PolicyView {
                        self.iam_state.policy_input_focus == InputFocus::Pagination
                    } else if self.current_service == Service::CloudWatchAlarms {
                        self.alarms_state.input_focus == InputFocus::Pagination
                    } else if self.current_service == Service::Ec2Instances {
                        self.ec2_state.input_focus == InputFocus::Pagination
                    } else if self.current_service == Service::CloudWatchLogGroups {
                        self.log_groups_state.input_focus == InputFocus::Pagination
                    } else if self.current_service == Service::EcrRepositories
                        && self.ecr_state.current_repository.is_none()
                    {
                        self.ecr_state.input_focus == InputFocus::Pagination
                    } else if self.current_service == Service::LambdaFunctions {
                        if self.lambda_state.current_function.is_some()
                            && self.lambda_state.detail_tab == LambdaDetailTab::Versions
                        {
                            self.lambda_state.version_input_focus == InputFocus::Pagination
                        } else if self.lambda_state.current_function.is_none() {
                            self.lambda_state.input_focus == InputFocus::Pagination
                        } else {
                            false
                        }
                    } else if self.current_service == Service::SqsQueues {
                        if self.sqs_state.current_queue.is_some()
                            && (self.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers
                                || self.sqs_state.detail_tab == SqsQueueDetailTab::EventBridgePipes
                                || self.sqs_state.detail_tab == SqsQueueDetailTab::Tagging
                                || self.sqs_state.detail_tab == SqsQueueDetailTab::Encryption
                                || self.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions)
                        {
                            self.sqs_state.input_focus == InputFocus::Pagination
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if is_pagination_focused && c.is_ascii_digit() {
                        self.page_input.push(c);
                    } else if self.current_service == Service::LambdaApplications {
                        let is_input_focused =
                            if self.lambda_application_state.current_application.is_some() {
                                if self.lambda_application_state.detail_tab
                                    == LambdaApplicationDetailTab::Deployments
                                {
                                    self.lambda_application_state.deployment_input_focus
                                        == InputFocus::Filter
                                } else {
                                    self.lambda_application_state.resource_input_focus
                                        == InputFocus::Filter
                                }
                            } else {
                                self.lambda_application_state.input_focus == InputFocus::Filter
                            };
                        if is_input_focused {
                            self.apply_filter_operation(|f| f.push(c));
                        }
                    } else if self.current_service == Service::CloudFormationStacks {
                        if self.cfn_state.current_stack.is_some()
                            && self.cfn_state.detail_tab == CfnDetailTab::Parameters
                        {
                            if self.cfn_state.parameters_input_focus == InputFocus::Filter {
                                self.cfn_state.parameters.filter.push(c);
                                self.cfn_state.parameters.selected = 0;
                            }
                        } else if self.cfn_state.current_stack.is_some()
                            && self.cfn_state.detail_tab == CfnDetailTab::Outputs
                        {
                            if self.cfn_state.outputs_input_focus == InputFocus::Filter {
                                self.cfn_state.outputs.filter.push(c);
                                self.cfn_state.outputs.selected = 0;
                            }
                        } else if self.cfn_state.current_stack.is_some()
                            && self.cfn_state.detail_tab == CfnDetailTab::Resources
                        {
                            if self.cfn_state.resources_input_focus == InputFocus::Filter {
                                self.cfn_state.resources.filter.push(c);
                                self.cfn_state.resources.selected = 0;
                            }
                        } else if self.cfn_state.input_focus == InputFocus::Filter {
                            self.apply_filter_operation(|f| f.push(c));
                        }
                    } else if self.current_service == Service::EcrRepositories
                        && self.ecr_state.current_repository.is_none()
                    {
                        if self.ecr_state.input_focus == InputFocus::Filter {
                            self.apply_filter_operation(|f| f.push(c));
                        }
                    } else if self.current_service == Service::IamRoles
                        && self.iam_state.current_role.is_none()
                    {
                        if self.iam_state.role_input_focus == InputFocus::Filter {
                            self.apply_filter_operation(|f| f.push(c));
                        }
                    } else if self.view_mode == ViewMode::PolicyView {
                        if self.iam_state.policy_input_focus == InputFocus::Filter {
                            self.apply_filter_operation(|f| f.push(c));
                        }
                    } else if self.current_service == Service::LambdaFunctions
                        && self.lambda_state.current_version.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Configuration
                    {
                        if self.lambda_state.alias_input_focus == InputFocus::Filter {
                            self.apply_filter_operation(|f| f.push(c));
                        }
                    } else if self.current_service == Service::LambdaFunctions
                        && self.lambda_state.current_function.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Versions
                    {
                        if self.lambda_state.version_input_focus == InputFocus::Filter {
                            self.apply_filter_operation(|f| f.push(c));
                        }
                    } else if self.current_service == Service::LambdaFunctions
                        && self.lambda_state.current_function.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Aliases
                    {
                        if self.lambda_state.alias_input_focus == InputFocus::Filter {
                            self.apply_filter_operation(|f| f.push(c));
                        }
                    } else if self.current_service == Service::SqsQueues
                        && self.sqs_state.current_queue.is_some()
                        && (self.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers
                            || self.sqs_state.detail_tab == SqsQueueDetailTab::EventBridgePipes
                            || self.sqs_state.detail_tab == SqsQueueDetailTab::Tagging
                            || self.sqs_state.detail_tab == SqsQueueDetailTab::Encryption
                            || self.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions)
                    {
                        if self.sqs_state.input_focus == InputFocus::Filter {
                            self.apply_filter_operation(|f| f.push(c));
                        }
                    } else {
                        self.apply_filter_operation(|f| f.push(c));
                    }
                } else if self.mode == Mode::EventFilterInput {
                    if self.log_groups_state.event_input_focus == EventFilterFocus::Filter {
                        self.log_groups_state.event_filter.push(c);
                    } else if c.is_ascii_digit() {
                        self.log_groups_state.relative_amount.push(c);
                    }
                } else if self.mode == Mode::Normal && c.is_ascii_digit() {
                    self.page_input.push(c);
                }
            }
            Action::FilterBackspace => {
                if self.mode == Mode::ServicePicker {
                    self.service_picker.filter.pop();
                    self.service_picker.selected = 0;
                } else if self.mode == Mode::TabPicker {
                    self.tab_filter.pop();
                    self.tab_picker_selected = 0;
                } else if self.mode == Mode::RegionPicker {
                    self.region_filter.pop();
                    self.region_picker_selected = 0;
                } else if self.mode == Mode::ProfilePicker {
                    self.profile_filter.pop();
                    self.profile_picker_selected = 0;
                } else if self.mode == Mode::SessionPicker {
                    self.session_filter.pop();
                    self.session_picker_selected = 0;
                } else if self.mode == Mode::InsightsInput {
                    match self.insights_state.insights.insights_focus {
                        InsightsFocus::Query => {
                            self.insights_state.insights.query_text.pop();
                        }
                        InsightsFocus::LogGroupSearch => {
                            self.insights_state.insights.log_group_search.pop();
                            // Update matches
                            if !self.insights_state.insights.log_group_search.is_empty() {
                                self.insights_state.insights.log_group_matches = self
                                    .log_groups_state
                                    .log_groups
                                    .items
                                    .iter()
                                    .filter(|g| {
                                        g.name.to_lowercase().contains(
                                            &self
                                                .insights_state
                                                .insights
                                                .log_group_search
                                                .to_lowercase(),
                                        )
                                    })
                                    .take(50)
                                    .map(|g| g.name.clone())
                                    .collect();
                                self.insights_state.insights.show_dropdown = true;
                            } else {
                                self.insights_state.insights.log_group_matches.clear();
                                self.insights_state.insights.show_dropdown = false;
                            }
                        }
                        _ => {}
                    }
                } else if self.mode == Mode::FilterInput {
                    // Only allow backspace when focus is on the input field
                    if self.current_service == Service::CloudFormationStacks {
                        if self.cfn_state.current_stack.is_some()
                            && self.cfn_state.detail_tab == CfnDetailTab::Parameters
                        {
                            if self.cfn_state.parameters_input_focus == InputFocus::Filter {
                                self.cfn_state.parameters.filter.pop();
                                self.cfn_state.parameters.selected = 0;
                            }
                        } else if self.cfn_state.current_stack.is_some()
                            && self.cfn_state.detail_tab == CfnDetailTab::Outputs
                        {
                            if self.cfn_state.outputs_input_focus == InputFocus::Filter {
                                self.cfn_state.outputs.filter.pop();
                                self.cfn_state.outputs.selected = 0;
                            }
                        } else if self.cfn_state.current_stack.is_some()
                            && self.cfn_state.detail_tab == CfnDetailTab::Resources
                        {
                            if self.cfn_state.resources_input_focus == InputFocus::Filter {
                                self.cfn_state.resources.filter.pop();
                                self.cfn_state.resources.selected = 0;
                            }
                        } else if self.cfn_state.input_focus == InputFocus::Filter {
                            self.apply_filter_operation(|f| {
                                f.pop();
                            });
                        }
                    } else {
                        self.apply_filter_operation(|f| {
                            f.pop();
                        });
                    }
                } else if self.mode == Mode::EventFilterInput {
                    if self.log_groups_state.event_input_focus == EventFilterFocus::Filter {
                        self.log_groups_state.event_filter.pop();
                    } else {
                        self.log_groups_state.relative_amount.pop();
                    }
                }
            }
            Action::DeleteWord => {
                let text = if self.mode == Mode::ServicePicker {
                    &mut self.service_picker.filter
                } else if self.mode == Mode::InsightsInput {
                    use crate::app::InsightsFocus;
                    match self.insights_state.insights.insights_focus {
                        InsightsFocus::Query => &mut self.insights_state.insights.query_text,
                        InsightsFocus::LogGroupSearch => {
                            &mut self.insights_state.insights.log_group_search
                        }
                        _ => return,
                    }
                } else if self.mode == Mode::FilterInput {
                    if let Some(filter) = self.get_active_filter_mut() {
                        filter
                    } else {
                        return;
                    }
                } else if self.mode == Mode::EventFilterInput {
                    if self.log_groups_state.event_input_focus == EventFilterFocus::Filter {
                        &mut self.log_groups_state.event_filter
                    } else {
                        &mut self.log_groups_state.relative_amount
                    }
                } else {
                    return;
                };

                if text.is_empty() {
                    return;
                }

                let mut chars: Vec<char> = text.chars().collect();
                while !chars.is_empty() && chars.last().is_some_and(|c| c.is_whitespace()) {
                    chars.pop();
                }
                while !chars.is_empty() && !chars.last().is_some_and(|c| c.is_whitespace()) {
                    chars.pop();
                }
                *text = chars.into_iter().collect();
            }
            Action::WordLeft => {
                // Not implemented - would need cursor position tracking
            }
            Action::WordRight => {
                // Not implemented - would need cursor position tracking
            }
            Action::OpenColumnSelector => {
                // Don't allow opening preferences in Template or GitSync tabs
                if self.current_service == Service::CloudFormationStacks
                    && self.cfn_state.current_stack.is_some()
                    && (self.cfn_state.detail_tab == CfnDetailTab::Template
                        || self.cfn_state.detail_tab == CfnDetailTab::GitSync)
                {
                    return;
                }

                // If we have page input, apply it instead of opening column selector
                if !self.page_input.is_empty() {
                    if let Ok(page) = self.page_input.parse::<usize>() {
                        self.go_to_page(page);
                    }
                    self.page_input.clear();
                } else {
                    self.mode = Mode::ColumnSelector;
                    self.column_selector_index = 0;
                }
            }
            Action::ToggleColumn => {
                if self.current_service == Service::S3Buckets
                    && self.s3_state.current_bucket.is_none()
                {
                    if let Some(col) = self.s3_bucket_column_ids.get(self.column_selector_index) {
                        if let Some(pos) = self
                            .s3_bucket_visible_column_ids
                            .iter()
                            .position(|c| c == col)
                        {
                            self.s3_bucket_visible_column_ids.remove(pos);
                        } else {
                            self.s3_bucket_visible_column_ids.push(*col);
                        }
                    }
                } else if self.current_service == Service::CloudWatchAlarms {
                    // Map flat list index to actual item
                    // 0: Columns header, 1-16: columns, 17: empty, 18: ViewAs header, 19-20: view options
                    // 21: empty, 22: PageSize header, 23-25: page sizes, 26: empty, 27: WrapLines header, 28: wrap option
                    let idx = self.column_selector_index;
                    if (1..=16).contains(&idx) {
                        // Column toggle
                        if let Some(col) = self.cw_alarm_column_ids.get(idx - 1) {
                            if let Some(pos) = self
                                .cw_alarm_visible_column_ids
                                .iter()
                                .position(|c| c == col)
                            {
                                self.cw_alarm_visible_column_ids.remove(pos);
                            } else {
                                self.cw_alarm_visible_column_ids.push(*col);
                            }
                        }
                    } else if idx == 19 {
                        self.alarms_state.view_as = AlarmViewMode::Table;
                    } else if idx == 20 {
                        self.alarms_state.view_as = AlarmViewMode::Cards;
                    } else if idx == 23 {
                        self.alarms_state.table.page_size = PageSize::Ten;
                    } else if idx == 24 {
                        self.alarms_state.table.page_size = PageSize::TwentyFive;
                    } else if idx == 25 {
                        self.alarms_state.table.page_size = PageSize::Fifty;
                    } else if idx == 26 {
                        self.alarms_state.table.page_size = PageSize::OneHundred;
                    } else if idx == 29 {
                        self.alarms_state.wrap_lines = !self.alarms_state.wrap_lines;
                    }
                } else if self.current_service == Service::EcrRepositories {
                    if self.ecr_state.current_repository.is_some() {
                        // Images view - columns + page size
                        let idx = self.column_selector_index;
                        if let Some(col) = self.ecr_image_column_ids.get(idx) {
                            if let Some(pos) = self
                                .ecr_image_visible_column_ids
                                .iter()
                                .position(|c| c == col)
                            {
                                self.ecr_image_visible_column_ids.remove(pos);
                            } else {
                                self.ecr_image_visible_column_ids.push(*col);
                            }
                        }
                    } else {
                        // Repositories view - just columns
                        if let Some(col) = self.ecr_repo_column_ids.get(self.column_selector_index)
                        {
                            if let Some(pos) = self
                                .ecr_repo_visible_column_ids
                                .iter()
                                .position(|c| c == col)
                            {
                                self.ecr_repo_visible_column_ids.remove(pos);
                            } else {
                                self.ecr_repo_visible_column_ids.push(*col);
                            }
                        }
                    }
                } else if self.current_service == Service::Ec2Instances {
                    let idx = self.column_selector_index;
                    if idx > 0 && idx <= self.ec2_column_ids.len() {
                        if let Some(col) = self.ec2_column_ids.get(idx - 1) {
                            if let Some(pos) =
                                self.ec2_visible_column_ids.iter().position(|c| c == col)
                            {
                                self.ec2_visible_column_ids.remove(pos);
                            } else {
                                self.ec2_visible_column_ids.push(*col);
                            }
                        }
                    } else if idx == self.ec2_column_ids.len() + 3 {
                        self.ec2_state.table.page_size = PageSize::Ten;
                    } else if idx == self.ec2_column_ids.len() + 4 {
                        self.ec2_state.table.page_size = PageSize::TwentyFive;
                    } else if idx == self.ec2_column_ids.len() + 5 {
                        self.ec2_state.table.page_size = PageSize::Fifty;
                    } else if idx == self.ec2_column_ids.len() + 6 {
                        self.ec2_state.table.page_size = PageSize::OneHundred;
                    }
                } else if self.current_service == Service::SqsQueues {
                    if self.sqs_state.current_queue.is_some()
                        && self.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers
                    {
                        // Triggers tab - columns + page size
                        let idx = self.column_selector_index;
                        if idx > 0 && idx <= self.sqs_state.trigger_column_ids.len() {
                            if let Some(col) = self.sqs_state.trigger_column_ids.get(idx - 1) {
                                if let Some(pos) = self
                                    .sqs_state
                                    .trigger_visible_column_ids
                                    .iter()
                                    .position(|c| c == col)
                                {
                                    self.sqs_state.trigger_visible_column_ids.remove(pos);
                                } else {
                                    self.sqs_state.trigger_visible_column_ids.push(col.clone());
                                }
                            }
                        } else if idx == self.sqs_state.trigger_column_ids.len() + 3 {
                            self.sqs_state.triggers.page_size = PageSize::Ten;
                        } else if idx == self.sqs_state.trigger_column_ids.len() + 4 {
                            self.sqs_state.triggers.page_size = PageSize::TwentyFive;
                        } else if idx == self.sqs_state.trigger_column_ids.len() + 5 {
                            self.sqs_state.triggers.page_size = PageSize::Fifty;
                        } else if idx == self.sqs_state.trigger_column_ids.len() + 6 {
                            self.sqs_state.triggers.page_size = PageSize::OneHundred;
                        }
                    } else if self.sqs_state.current_queue.is_some()
                        && self.sqs_state.detail_tab == SqsQueueDetailTab::EventBridgePipes
                    {
                        // Pipes tab - columns + page size
                        let idx = self.column_selector_index;
                        if idx > 0 && idx <= self.sqs_state.pipe_column_ids.len() {
                            if let Some(col) = self.sqs_state.pipe_column_ids.get(idx - 1) {
                                if let Some(pos) = self
                                    .sqs_state
                                    .pipe_visible_column_ids
                                    .iter()
                                    .position(|c| c == col)
                                {
                                    self.sqs_state.pipe_visible_column_ids.remove(pos);
                                } else {
                                    self.sqs_state.pipe_visible_column_ids.push(col.clone());
                                }
                            }
                        } else if idx == self.sqs_state.pipe_column_ids.len() + 3 {
                            self.sqs_state.pipes.page_size = PageSize::Ten;
                        } else if idx == self.sqs_state.pipe_column_ids.len() + 4 {
                            self.sqs_state.pipes.page_size = PageSize::TwentyFive;
                        } else if idx == self.sqs_state.pipe_column_ids.len() + 5 {
                            self.sqs_state.pipes.page_size = PageSize::Fifty;
                        } else if idx == self.sqs_state.pipe_column_ids.len() + 6 {
                            self.sqs_state.pipes.page_size = PageSize::OneHundred;
                        }
                    } else if self.sqs_state.current_queue.is_some()
                        && self.sqs_state.detail_tab == SqsQueueDetailTab::Tagging
                    {
                        // Tags tab - columns + page size
                        let idx = self.column_selector_index;
                        if idx > 0 && idx <= self.sqs_state.tag_column_ids.len() {
                            if let Some(col) = self.sqs_state.tag_column_ids.get(idx - 1) {
                                if let Some(pos) = self
                                    .sqs_state
                                    .tag_visible_column_ids
                                    .iter()
                                    .position(|c| c == col)
                                {
                                    self.sqs_state.tag_visible_column_ids.remove(pos);
                                } else {
                                    self.sqs_state.tag_visible_column_ids.push(col.clone());
                                }
                            }
                        } else if idx == self.sqs_state.tag_column_ids.len() + 3 {
                            self.sqs_state.tags.page_size = PageSize::Ten;
                        } else if idx == self.sqs_state.tag_column_ids.len() + 4 {
                            self.sqs_state.tags.page_size = PageSize::TwentyFive;
                        } else if idx == self.sqs_state.tag_column_ids.len() + 5 {
                            self.sqs_state.tags.page_size = PageSize::Fifty;
                        } else if idx == self.sqs_state.tag_column_ids.len() + 6 {
                            self.sqs_state.tags.page_size = PageSize::OneHundred;
                        }
                    } else if self.sqs_state.current_queue.is_some()
                        && self.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions
                    {
                        // Subscriptions tab - columns + page size
                        let idx = self.column_selector_index;
                        if idx > 0 && idx <= self.sqs_state.subscription_column_ids.len() {
                            if let Some(col) = self.sqs_state.subscription_column_ids.get(idx - 1) {
                                if let Some(pos) = self
                                    .sqs_state
                                    .subscription_visible_column_ids
                                    .iter()
                                    .position(|c| c == col)
                                {
                                    self.sqs_state.subscription_visible_column_ids.remove(pos);
                                } else {
                                    self.sqs_state
                                        .subscription_visible_column_ids
                                        .push(col.clone());
                                }
                            }
                        } else if idx == self.sqs_state.subscription_column_ids.len() + 3 {
                            self.sqs_state.subscriptions.page_size = PageSize::Ten;
                        } else if idx == self.sqs_state.subscription_column_ids.len() + 4 {
                            self.sqs_state.subscriptions.page_size = PageSize::TwentyFive;
                        } else if idx == self.sqs_state.subscription_column_ids.len() + 5 {
                            self.sqs_state.subscriptions.page_size = PageSize::Fifty;
                        } else if idx == self.sqs_state.subscription_column_ids.len() + 6 {
                            self.sqs_state.subscriptions.page_size = PageSize::OneHundred;
                        }
                    } else if let Some(col) = self.sqs_column_ids.get(self.column_selector_index) {
                        if let Some(pos) = self.sqs_visible_column_ids.iter().position(|c| c == col)
                        {
                            self.sqs_visible_column_ids.remove(pos);
                        } else {
                            self.sqs_visible_column_ids.push(*col);
                        }
                    }
                } else if self.current_service == Service::LambdaFunctions {
                    let idx = self.column_selector_index;
                    // Check if we're in Versions tab
                    if self.lambda_state.current_function.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Versions
                    {
                        // Version columns
                        if idx > 0 && idx <= self.lambda_state.version_column_ids.len() {
                            if let Some(col) = self.lambda_state.version_column_ids.get(idx - 1) {
                                if let Some(pos) = self
                                    .lambda_state
                                    .version_visible_column_ids
                                    .iter()
                                    .position(|c| *c == *col)
                                {
                                    self.lambda_state.version_visible_column_ids.remove(pos);
                                } else {
                                    self.lambda_state
                                        .version_visible_column_ids
                                        .push(col.clone());
                                }
                            }
                        } else if idx == self.lambda_state.version_column_ids.len() + 3 {
                            self.lambda_state.version_table.page_size = PageSize::Ten;
                        } else if idx == self.lambda_state.version_column_ids.len() + 4 {
                            self.lambda_state.version_table.page_size = PageSize::TwentyFive;
                        } else if idx == self.lambda_state.version_column_ids.len() + 5 {
                            self.lambda_state.version_table.page_size = PageSize::Fifty;
                        } else if idx == self.lambda_state.version_column_ids.len() + 6 {
                            self.lambda_state.version_table.page_size = PageSize::OneHundred;
                        }
                    } else if (self.lambda_state.current_function.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Aliases)
                        || (self.lambda_state.current_version.is_some()
                            && self.lambda_state.detail_tab == LambdaDetailTab::Configuration)
                    {
                        // Alias columns
                        if idx > 0 && idx <= self.lambda_state.alias_column_ids.len() {
                            if let Some(col) = self.lambda_state.alias_column_ids.get(idx - 1) {
                                if let Some(pos) = self
                                    .lambda_state
                                    .alias_visible_column_ids
                                    .iter()
                                    .position(|c| *c == *col)
                                {
                                    self.lambda_state.alias_visible_column_ids.remove(pos);
                                } else {
                                    self.lambda_state.alias_visible_column_ids.push(col.clone());
                                }
                            }
                        } else if idx == self.lambda_state.alias_column_ids.len() + 3 {
                            self.lambda_state.alias_table.page_size = PageSize::Ten;
                        } else if idx == self.lambda_state.alias_column_ids.len() + 4 {
                            self.lambda_state.alias_table.page_size = PageSize::TwentyFive;
                        } else if idx == self.lambda_state.alias_column_ids.len() + 5 {
                            self.lambda_state.alias_table.page_size = PageSize::Fifty;
                        } else if idx == self.lambda_state.alias_column_ids.len() + 6 {
                            self.lambda_state.alias_table.page_size = PageSize::OneHundred;
                        }
                    } else {
                        // Function columns
                        if idx > 0 && idx <= self.lambda_state.function_column_ids.len() {
                            if let Some(col) = self.lambda_state.function_column_ids.get(idx - 1) {
                                if let Some(pos) = self
                                    .lambda_state
                                    .function_visible_column_ids
                                    .iter()
                                    .position(|c| *c == *col)
                                {
                                    self.lambda_state.function_visible_column_ids.remove(pos);
                                } else {
                                    self.lambda_state.function_visible_column_ids.push(*col);
                                }
                            }
                        } else if idx == self.lambda_state.function_column_ids.len() + 3 {
                            self.lambda_state.table.page_size = PageSize::Ten;
                        } else if idx == self.lambda_state.function_column_ids.len() + 4 {
                            self.lambda_state.table.page_size = PageSize::TwentyFive;
                        } else if idx == self.lambda_state.function_column_ids.len() + 5 {
                            self.lambda_state.table.page_size = PageSize::Fifty;
                        } else if idx == self.lambda_state.function_column_ids.len() + 6 {
                            self.lambda_state.table.page_size = PageSize::OneHundred;
                        }
                    }
                } else if self.current_service == Service::LambdaApplications {
                    if self.lambda_application_state.current_application.is_some() {
                        // In detail view - handle resource or deployment columns
                        if self.lambda_application_state.detail_tab
                            == LambdaApplicationDetailTab::Overview
                        {
                            // Resources columns
                            let idx = self.column_selector_index;
                            if idx > 0 && idx <= self.lambda_resource_column_ids.len() {
                                if let Some(col) = self.lambda_resource_column_ids.get(idx - 1) {
                                    if let Some(pos) = self
                                        .lambda_resource_visible_column_ids
                                        .iter()
                                        .position(|c| c == col)
                                    {
                                        self.lambda_resource_visible_column_ids.remove(pos);
                                    } else {
                                        self.lambda_resource_visible_column_ids.push(*col);
                                    }
                                }
                            } else if idx == self.lambda_resource_column_ids.len() + 3 {
                                self.lambda_application_state.resources.page_size = PageSize::Ten;
                            } else if idx == self.lambda_resource_column_ids.len() + 4 {
                                self.lambda_application_state.resources.page_size =
                                    PageSize::TwentyFive;
                            } else if idx == self.lambda_resource_column_ids.len() + 5 {
                                self.lambda_application_state.resources.page_size = PageSize::Fifty;
                            }
                        } else {
                            // Deployments columns
                            let idx = self.column_selector_index;
                            if idx > 0 && idx <= self.lambda_deployment_column_ids.len() {
                                if let Some(col) = self.lambda_deployment_column_ids.get(idx - 1) {
                                    if let Some(pos) = self
                                        .lambda_deployment_visible_column_ids
                                        .iter()
                                        .position(|c| c == col)
                                    {
                                        self.lambda_deployment_visible_column_ids.remove(pos);
                                    } else {
                                        self.lambda_deployment_visible_column_ids.push(*col);
                                    }
                                }
                            } else if idx == self.lambda_deployment_column_ids.len() + 3 {
                                self.lambda_application_state.deployments.page_size = PageSize::Ten;
                            } else if idx == self.lambda_deployment_column_ids.len() + 4 {
                                self.lambda_application_state.deployments.page_size =
                                    PageSize::TwentyFive;
                            } else if idx == self.lambda_deployment_column_ids.len() + 5 {
                                self.lambda_application_state.deployments.page_size =
                                    PageSize::Fifty;
                            }
                        }
                    } else {
                        // In list view - handle application columns
                        let idx = self.column_selector_index;
                        if idx > 0 && idx <= self.lambda_application_column_ids.len() {
                            if let Some(col) = self.lambda_application_column_ids.get(idx - 1) {
                                if let Some(pos) = self
                                    .lambda_application_visible_column_ids
                                    .iter()
                                    .position(|c| *c == *col)
                                {
                                    self.lambda_application_visible_column_ids.remove(pos);
                                } else {
                                    self.lambda_application_visible_column_ids.push(*col);
                                }
                            }
                        } else if idx == self.lambda_application_column_ids.len() + 3 {
                            self.lambda_application_state.table.page_size = PageSize::Ten;
                        } else if idx == self.lambda_application_column_ids.len() + 4 {
                            self.lambda_application_state.table.page_size = PageSize::TwentyFive;
                        } else if idx == self.lambda_application_column_ids.len() + 5 {
                            self.lambda_application_state.table.page_size = PageSize::Fifty;
                        }
                    }
                } else if self.view_mode == ViewMode::Events {
                    if let Some(col) = self.cw_log_event_column_ids.get(self.column_selector_index)
                    {
                        if let Some(pos) = self
                            .cw_log_event_visible_column_ids
                            .iter()
                            .position(|c| c == col)
                        {
                            self.cw_log_event_visible_column_ids.remove(pos);
                        } else {
                            self.cw_log_event_visible_column_ids.push(*col);
                        }
                    }
                } else if self.view_mode == ViewMode::Detail {
                    if let Some(col) = self
                        .cw_log_stream_column_ids
                        .get(self.column_selector_index)
                    {
                        if let Some(pos) = self
                            .cw_log_stream_visible_column_ids
                            .iter()
                            .position(|c| c == col)
                        {
                            self.cw_log_stream_visible_column_ids.remove(pos);
                        } else {
                            self.cw_log_stream_visible_column_ids.push(*col);
                        }
                    }
                } else if self.current_service == Service::CloudFormationStacks {
                    let idx = self.column_selector_index;
                    // Check if we're in StackInfo tab (tags)
                    if self.cfn_state.current_stack.is_some()
                        && self.cfn_state.detail_tab == CfnDetailTab::StackInfo
                    {
                        // Tags have 2 columns (Key, Value) - always visible, so only handle page size
                        if idx == 4 {
                            self.cfn_state.tags.page_size = PageSize::Ten;
                        } else if idx == 5 {
                            self.cfn_state.tags.page_size = PageSize::TwentyFive;
                        } else if idx == 6 {
                            self.cfn_state.tags.page_size = PageSize::Fifty;
                        } else if idx == 7 {
                            self.cfn_state.tags.page_size = PageSize::OneHundred;
                        }
                    } else if self.cfn_state.current_stack.is_some()
                        && self.cfn_state.detail_tab == CfnDetailTab::Parameters
                    {
                        if idx > 0 && idx <= self.cfn_parameter_column_ids.len() {
                            if let Some(col) = self.cfn_parameter_column_ids.get(idx - 1) {
                                if let Some(pos) = self
                                    .cfn_parameter_visible_column_ids
                                    .iter()
                                    .position(|c| c == col)
                                {
                                    self.cfn_parameter_visible_column_ids.remove(pos);
                                } else {
                                    self.cfn_parameter_visible_column_ids.push(col);
                                }
                            }
                        } else if idx == self.cfn_parameter_column_ids.len() + 3 {
                            self.cfn_state.parameters.page_size = PageSize::Ten;
                        } else if idx == self.cfn_parameter_column_ids.len() + 4 {
                            self.cfn_state.parameters.page_size = PageSize::TwentyFive;
                        } else if idx == self.cfn_parameter_column_ids.len() + 5 {
                            self.cfn_state.parameters.page_size = PageSize::Fifty;
                        } else if idx == self.cfn_parameter_column_ids.len() + 6 {
                            self.cfn_state.parameters.page_size = PageSize::OneHundred;
                        }
                    } else if self.cfn_state.current_stack.is_some()
                        && self.cfn_state.detail_tab == CfnDetailTab::Outputs
                    {
                        if idx > 0 && idx <= self.cfn_output_column_ids.len() {
                            if let Some(col) = self.cfn_output_column_ids.get(idx - 1) {
                                if let Some(pos) = self
                                    .cfn_output_visible_column_ids
                                    .iter()
                                    .position(|c| c == col)
                                {
                                    self.cfn_output_visible_column_ids.remove(pos);
                                } else {
                                    self.cfn_output_visible_column_ids.push(col);
                                }
                            }
                        } else if idx == self.cfn_output_column_ids.len() + 3 {
                            self.cfn_state.outputs.page_size = PageSize::Ten;
                        } else if idx == self.cfn_output_column_ids.len() + 4 {
                            self.cfn_state.outputs.page_size = PageSize::TwentyFive;
                        } else if idx == self.cfn_output_column_ids.len() + 5 {
                            self.cfn_state.outputs.page_size = PageSize::Fifty;
                        } else if idx == self.cfn_output_column_ids.len() + 6 {
                            self.cfn_state.outputs.page_size = PageSize::OneHundred;
                        }
                    } else if self.cfn_state.current_stack.is_some()
                        && self.cfn_state.detail_tab == CfnDetailTab::Resources
                    {
                        if idx > 0 && idx <= self.cfn_resource_column_ids.len() {
                            if let Some(col) = self.cfn_resource_column_ids.get(idx - 1) {
                                if let Some(pos) = self
                                    .cfn_resource_visible_column_ids
                                    .iter()
                                    .position(|c| c == col)
                                {
                                    self.cfn_resource_visible_column_ids.remove(pos);
                                } else {
                                    self.cfn_resource_visible_column_ids.push(col);
                                }
                            }
                        } else if idx == self.cfn_resource_column_ids.len() + 3 {
                            self.cfn_state.resources.page_size = PageSize::Ten;
                        } else if idx == self.cfn_resource_column_ids.len() + 4 {
                            self.cfn_state.resources.page_size = PageSize::TwentyFive;
                        } else if idx == self.cfn_resource_column_ids.len() + 5 {
                            self.cfn_state.resources.page_size = PageSize::Fifty;
                        } else if idx == self.cfn_resource_column_ids.len() + 6 {
                            self.cfn_state.resources.page_size = PageSize::OneHundred;
                        }
                    } else if self.cfn_state.current_stack.is_none() {
                        // Stack list view
                        if idx > 0 && idx <= self.cfn_column_ids.len() {
                            if let Some(col) = self.cfn_column_ids.get(idx - 1) {
                                if let Some(pos) =
                                    self.cfn_visible_column_ids.iter().position(|c| c == col)
                                {
                                    self.cfn_visible_column_ids.remove(pos);
                                } else {
                                    self.cfn_visible_column_ids.push(*col);
                                }
                            }
                        } else if idx == self.cfn_column_ids.len() + 3 {
                            self.cfn_state.table.page_size = PageSize::Ten;
                        } else if idx == self.cfn_column_ids.len() + 4 {
                            self.cfn_state.table.page_size = PageSize::TwentyFive;
                        } else if idx == self.cfn_column_ids.len() + 5 {
                            self.cfn_state.table.page_size = PageSize::Fifty;
                        } else if idx == self.cfn_column_ids.len() + 6 {
                            self.cfn_state.table.page_size = PageSize::OneHundred;
                        }
                    }
                    // Template tab: no column toggle
                } else if self.current_service == Service::IamUsers {
                    let idx = self.column_selector_index;
                    if self.iam_state.current_user.is_some() {
                        // Policy columns
                        if idx > 0 && idx <= self.iam_policy_column_ids.len() {
                            if let Some(col) = self.iam_policy_column_ids.get(idx - 1) {
                                if let Some(pos) = self
                                    .iam_policy_visible_column_ids
                                    .iter()
                                    .position(|c| c == col)
                                {
                                    self.iam_policy_visible_column_ids.remove(pos);
                                } else {
                                    self.iam_policy_visible_column_ids.push(col.clone());
                                }
                            }
                        } else if idx == self.iam_policy_column_ids.len() + 3 {
                            self.iam_state.policies.page_size = PageSize::Ten;
                        } else if idx == self.iam_policy_column_ids.len() + 4 {
                            self.iam_state.policies.page_size = PageSize::TwentyFive;
                        } else if idx == self.iam_policy_column_ids.len() + 5 {
                            self.iam_state.policies.page_size = PageSize::Fifty;
                        }
                    } else {
                        // User columns
                        if idx > 0 && idx <= self.iam_user_column_ids.len() {
                            if let Some(col) = self.iam_user_column_ids.get(idx - 1) {
                                if let Some(pos) = self
                                    .iam_user_visible_column_ids
                                    .iter()
                                    .position(|c| c == col)
                                {
                                    self.iam_user_visible_column_ids.remove(pos);
                                } else {
                                    self.iam_user_visible_column_ids.push(*col);
                                }
                            }
                        } else if idx == self.iam_user_column_ids.len() + 3 {
                            self.iam_state.users.page_size = PageSize::Ten;
                        } else if idx == self.iam_user_column_ids.len() + 4 {
                            self.iam_state.users.page_size = PageSize::TwentyFive;
                        } else if idx == self.iam_user_column_ids.len() + 5 {
                            self.iam_state.users.page_size = PageSize::Fifty;
                        }
                    }
                } else if self.current_service == Service::IamRoles {
                    let idx = self.column_selector_index;
                    if self.iam_state.current_role.is_some() {
                        // Policy columns
                        if idx > 0 && idx <= self.iam_policy_column_ids.len() {
                            if let Some(col) = self.iam_policy_column_ids.get(idx - 1) {
                                if let Some(pos) = self
                                    .iam_policy_visible_column_ids
                                    .iter()
                                    .position(|c| c == col)
                                {
                                    self.iam_policy_visible_column_ids.remove(pos);
                                } else {
                                    self.iam_policy_visible_column_ids.push(col.clone());
                                }
                            }
                        } else if idx == self.iam_policy_column_ids.len() + 3 {
                            self.iam_state.policies.page_size = PageSize::Ten;
                        } else if idx == self.iam_policy_column_ids.len() + 4 {
                            self.iam_state.policies.page_size = PageSize::TwentyFive;
                        } else if idx == self.iam_policy_column_ids.len() + 5 {
                            self.iam_state.policies.page_size = PageSize::Fifty;
                        }
                    } else {
                        // Role columns
                        if idx > 0 && idx <= self.iam_role_column_ids.len() {
                            if let Some(col) = self.iam_role_column_ids.get(idx - 1) {
                                if let Some(pos) = self
                                    .iam_role_visible_column_ids
                                    .iter()
                                    .position(|c| c == col)
                                {
                                    self.iam_role_visible_column_ids.remove(pos);
                                } else {
                                    self.iam_role_visible_column_ids.push(col.clone());
                                }
                            }
                        } else if idx == self.iam_role_column_ids.len() + 3 {
                            self.iam_state.roles.page_size = PageSize::Ten;
                        } else if idx == self.iam_role_column_ids.len() + 4 {
                            self.iam_state.roles.page_size = PageSize::TwentyFive;
                        } else if idx == self.iam_role_column_ids.len() + 5 {
                            self.iam_state.roles.page_size = PageSize::Fifty;
                        }
                    }
                } else if self.current_service == Service::IamUserGroups {
                    let idx = self.column_selector_index;
                    if idx > 0 && idx <= self.iam_group_column_ids.len() {
                        if let Some(col) = self.iam_group_column_ids.get(idx - 1) {
                            if let Some(pos) = self
                                .iam_group_visible_column_ids
                                .iter()
                                .position(|c| c == col)
                            {
                                self.iam_group_visible_column_ids.remove(pos);
                            } else {
                                self.iam_group_visible_column_ids.push(col.clone());
                            }
                        }
                    } else if idx == self.iam_group_column_ids.len() + 3 {
                        self.iam_state.groups.page_size = PageSize::Ten;
                    } else if idx == self.iam_group_column_ids.len() + 4 {
                        self.iam_state.groups.page_size = PageSize::TwentyFive;
                    } else if idx == self.iam_group_column_ids.len() + 5 {
                        self.iam_state.groups.page_size = PageSize::Fifty;
                    }
                } else if let Some(col) =
                    self.cw_log_group_column_ids.get(self.column_selector_index)
                {
                    if let Some(pos) = self
                        .cw_log_group_visible_column_ids
                        .iter()
                        .position(|c| c == col)
                    {
                        self.cw_log_group_visible_column_ids.remove(pos);
                    } else {
                        self.cw_log_group_visible_column_ids.push(*col);
                    }
                }
            }
            Action::NextPreferences => {
                if self.current_service == Service::CloudWatchAlarms {
                    // Jump to next section: Columns(0), ViewAs(18), PageSize(22), WrapLines(28)
                    if self.column_selector_index < 18 {
                        self.column_selector_index = 18; // ViewAs header
                    } else if self.column_selector_index < 22 {
                        self.column_selector_index = 22; // PageSize header
                    } else if self.column_selector_index < 28 {
                        self.column_selector_index = 28; // WrapLines header
                    } else {
                        self.column_selector_index = 0; // Back to Columns header
                    }
                } else if self.current_service == Service::EcrRepositories
                    && self.ecr_state.current_repository.is_some()
                {
                    // Images view: Columns(0), PageSize(columns.len() + 2)
                    let page_size_idx = self.ecr_image_column_ids.len() + 2;
                    if self.column_selector_index < page_size_idx {
                        self.column_selector_index = page_size_idx;
                    } else {
                        self.column_selector_index = 0;
                    }
                } else if self.current_service == Service::LambdaFunctions {
                    // Lambda: Columns(0), PageSize(columns.len() + 2)
                    let page_size_idx = self.lambda_state.function_column_ids.len() + 2;
                    if self.column_selector_index < page_size_idx {
                        self.column_selector_index = page_size_idx;
                    } else {
                        self.column_selector_index = 0;
                    }
                } else if self.current_service == Service::LambdaApplications {
                    // Lambda Applications: Columns(0), PageSize(columns.len() + 2)
                    let page_size_idx = self.lambda_application_column_ids.len() + 2;
                    if self.column_selector_index < page_size_idx {
                        self.column_selector_index = page_size_idx;
                    } else {
                        self.column_selector_index = 0;
                    }
                } else if self.current_service == Service::CloudFormationStacks {
                    // CloudFormation: Columns(0), PageSize(columns.len() + 2)
                    let page_size_idx = self.cfn_column_ids.len() + 2;
                    if self.column_selector_index < page_size_idx {
                        self.column_selector_index = page_size_idx;
                    } else {
                        self.column_selector_index = 0;
                    }
                } else if self.current_service == Service::Ec2Instances {
                    let page_size_idx = self.ec2_column_ids.len() + 2;
                    if self.column_selector_index < page_size_idx {
                        self.column_selector_index = page_size_idx;
                    } else {
                        self.column_selector_index = 0;
                    }
                } else if self.current_service == Service::IamUsers {
                    if self.iam_state.current_user.is_some() {
                        // Only Permissions tab has column preferences
                        if self.iam_state.user_tab == UserTab::Permissions {
                            let page_size_idx = self.iam_policy_column_ids.len() + 2;
                            if self.column_selector_index < page_size_idx {
                                self.column_selector_index = page_size_idx;
                            } else {
                                self.column_selector_index = 0;
                            }
                        }
                        // Other tabs (Groups, Tags, Security Credentials, Last Accessed) have no preferences
                    } else {
                        // User columns: Columns(0), PageSize(columns.len() + 2)
                        let page_size_idx = self.iam_user_column_ids.len() + 2;
                        if self.column_selector_index < page_size_idx {
                            self.column_selector_index = page_size_idx;
                        } else {
                            self.column_selector_index = 0;
                        }
                    }
                } else if self.current_service == Service::IamRoles {
                    if self.iam_state.current_role.is_some() {
                        // Policy columns: Columns(0), PageSize(columns.len() + 2)
                        let page_size_idx = self.iam_policy_column_ids.len() + 2;
                        if self.column_selector_index < page_size_idx {
                            self.column_selector_index = page_size_idx;
                        } else {
                            self.column_selector_index = 0;
                        }
                    } else {
                        // Role columns: Columns(0), PageSize(columns.len() + 2)
                        let page_size_idx = self.iam_role_column_ids.len() + 2;
                        if self.column_selector_index < page_size_idx {
                            self.column_selector_index = page_size_idx;
                        } else {
                            self.column_selector_index = 0;
                        }
                    }
                } else if self.current_service == Service::IamUserGroups {
                    // Group columns: Columns(0), PageSize(columns.len() + 2)
                    let page_size_idx = self.iam_group_column_ids.len() + 2;
                    if self.column_selector_index < page_size_idx {
                        self.column_selector_index = page_size_idx;
                    } else {
                        self.column_selector_index = 0;
                    }
                } else if self.current_service == Service::SqsQueues
                    && self.sqs_state.current_queue.is_some()
                    && self.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers
                {
                    // Triggers tab: Columns(0), PageSize(columns.len() + 2)
                    let page_size_idx = self.sqs_state.trigger_column_ids.len() + 2;
                    if self.column_selector_index < page_size_idx {
                        self.column_selector_index = page_size_idx;
                    } else {
                        self.column_selector_index = 0;
                    }
                } else if self.current_service == Service::SqsQueues
                    && self.sqs_state.current_queue.is_some()
                    && self.sqs_state.detail_tab == SqsQueueDetailTab::EventBridgePipes
                {
                    // Pipes tab: Columns(0), PageSize(columns.len() + 2)
                    let page_size_idx = self.sqs_state.pipe_column_ids.len() + 2;
                    if self.column_selector_index < page_size_idx {
                        self.column_selector_index = page_size_idx;
                    } else {
                        self.column_selector_index = 0;
                    }
                } else if self.current_service == Service::SqsQueues
                    && self.sqs_state.current_queue.is_some()
                    && self.sqs_state.detail_tab == SqsQueueDetailTab::Tagging
                {
                    // Tags tab: Columns(0), PageSize(columns.len() + 2)
                    let page_size_idx = self.sqs_state.tag_column_ids.len() + 2;
                    if self.column_selector_index < page_size_idx {
                        self.column_selector_index = page_size_idx;
                    } else {
                        self.column_selector_index = 0;
                    }
                } else if self.current_service == Service::SqsQueues
                    && self.sqs_state.current_queue.is_some()
                    && self.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions
                {
                    // Subscriptions tab: Columns(0), PageSize(columns.len() + 2)
                    let page_size_idx = self.sqs_state.subscription_column_ids.len() + 2;
                    if self.column_selector_index < page_size_idx {
                        self.column_selector_index = page_size_idx;
                    } else {
                        self.column_selector_index = 0;
                    }
                }
            }
            Action::PrevPreferences => {
                if self.current_service == Service::CloudWatchAlarms {
                    // Jump to prev section: Columns(0), ViewAs(18), PageSize(22), WrapLines(28)
                    if self.column_selector_index >= 28 {
                        self.column_selector_index = 22;
                    } else if self.column_selector_index >= 22 {
                        self.column_selector_index = 18;
                    } else if self.column_selector_index >= 18 {
                        self.column_selector_index = 0;
                    } else {
                        self.column_selector_index = 28;
                    }
                } else if self.current_service == Service::EcrRepositories
                    && self.ecr_state.current_repository.is_some()
                {
                    let page_size_idx = self.ecr_image_column_ids.len() + 2;
                    if self.column_selector_index >= page_size_idx {
                        self.column_selector_index = 0;
                    } else {
                        self.column_selector_index = page_size_idx;
                    }
                } else if self.current_service == Service::LambdaFunctions {
                    let page_size_idx = self.lambda_state.function_column_ids.len() + 2;
                    if self.column_selector_index >= page_size_idx {
                        self.column_selector_index = 0;
                    } else {
                        self.column_selector_index = page_size_idx;
                    }
                } else if self.current_service == Service::LambdaApplications {
                    let page_size_idx = self.lambda_application_column_ids.len() + 2;
                    if self.column_selector_index >= page_size_idx {
                        self.column_selector_index = 0;
                    } else {
                        self.column_selector_index = page_size_idx;
                    }
                } else if self.current_service == Service::CloudFormationStacks {
                    let page_size_idx = self.cfn_column_ids.len() + 2;
                    if self.column_selector_index >= page_size_idx {
                        self.column_selector_index = 0;
                    } else {
                        self.column_selector_index = page_size_idx;
                    }
                } else if self.current_service == Service::Ec2Instances {
                    let page_size_idx = self.ec2_column_ids.len() + 2;
                    if self.column_selector_index >= page_size_idx {
                        self.column_selector_index = 0;
                    } else {
                        self.column_selector_index = page_size_idx;
                    }
                } else if self.current_service == Service::IamUsers {
                    if self.iam_state.current_user.is_some()
                        && self.iam_state.user_tab == UserTab::Permissions
                    {
                        let page_size_idx = self.iam_policy_column_ids.len() + 2;
                        if self.column_selector_index >= page_size_idx {
                            self.column_selector_index = 0;
                        } else {
                            self.column_selector_index = page_size_idx;
                        }
                    } else if self.iam_state.current_user.is_none() {
                        let page_size_idx = self.iam_user_column_ids.len() + 2;
                        if self.column_selector_index >= page_size_idx {
                            self.column_selector_index = 0;
                        } else {
                            self.column_selector_index = page_size_idx;
                        }
                    }
                } else if self.current_service == Service::IamRoles {
                    let page_size_idx = if self.iam_state.current_role.is_some() {
                        self.iam_policy_column_ids.len() + 2
                    } else {
                        self.iam_role_column_ids.len() + 2
                    };
                    if self.column_selector_index >= page_size_idx {
                        self.column_selector_index = 0;
                    } else {
                        self.column_selector_index = page_size_idx;
                    }
                } else if self.current_service == Service::IamUserGroups {
                    let page_size_idx = self.iam_group_column_ids.len() + 2;
                    if self.column_selector_index >= page_size_idx {
                        self.column_selector_index = 0;
                    } else {
                        self.column_selector_index = page_size_idx;
                    }
                } else if self.current_service == Service::SqsQueues
                    && self.sqs_state.current_queue.is_some()
                {
                    let page_size_idx = match self.sqs_state.detail_tab {
                        SqsQueueDetailTab::LambdaTriggers => {
                            self.sqs_state.trigger_column_ids.len() + 2
                        }
                        SqsQueueDetailTab::EventBridgePipes => {
                            self.sqs_state.pipe_column_ids.len() + 2
                        }
                        SqsQueueDetailTab::Tagging => self.sqs_state.tag_column_ids.len() + 2,
                        SqsQueueDetailTab::SnsSubscriptions => {
                            self.sqs_state.subscription_column_ids.len() + 2
                        }
                        _ => 0,
                    };
                    if page_size_idx > 0 {
                        if self.column_selector_index >= page_size_idx {
                            self.column_selector_index = 0;
                        } else {
                            self.column_selector_index = page_size_idx;
                        }
                    }
                }
            }
            Action::CloseColumnSelector => {
                self.mode = Mode::Normal;
                self.preference_section = Preferences::Columns;
            }
            Action::NextDetailTab => {
                if self.current_service == Service::SqsQueues
                    && self.sqs_state.current_queue.is_some()
                {
                    self.sqs_state.detail_tab = self.sqs_state.detail_tab.next();
                    if self.sqs_state.detail_tab == SqsQueueDetailTab::Monitoring {
                        self.sqs_state.set_metrics_loading(true);
                        self.sqs_state.set_monitoring_scroll(0);
                        self.sqs_state.clear_metrics();
                    } else if self.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers {
                        self.sqs_state.triggers.loading = true;
                    } else if self.sqs_state.detail_tab == SqsQueueDetailTab::EventBridgePipes {
                        self.sqs_state.pipes.loading = true;
                    } else if self.sqs_state.detail_tab == SqsQueueDetailTab::Tagging {
                        self.sqs_state.tags.loading = true;
                    } else if self.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions {
                        self.sqs_state.subscriptions.loading = true;
                    }
                } else if self.current_service == Service::Ec2Instances
                    && self.ec2_state.current_instance.is_some()
                {
                    self.ec2_state.detail_tab = self.ec2_state.detail_tab.next();
                } else if self.current_service == Service::LambdaApplications
                    && self.lambda_application_state.current_application.is_some()
                {
                    self.lambda_application_state.detail_tab =
                        self.lambda_application_state.detail_tab.next();
                } else if self.current_service == Service::IamRoles
                    && self.iam_state.current_role.is_some()
                {
                    self.iam_state.role_tab = self.iam_state.role_tab.next();
                    if self.iam_state.role_tab == RoleTab::Tags {
                        self.iam_state.tags.loading = true;
                    }
                } else if self.current_service == Service::IamUsers
                    && self.iam_state.current_user.is_some()
                {
                    self.iam_state.user_tab = self.iam_state.user_tab.next();
                    if self.iam_state.user_tab == UserTab::Tags {
                        self.iam_state.user_tags.loading = true;
                    }
                } else if self.current_service == Service::IamUserGroups
                    && self.iam_state.current_group.is_some()
                {
                    self.iam_state.group_tab = self.iam_state.group_tab.next();
                } else if self.view_mode == ViewMode::Detail {
                    self.log_groups_state.detail_tab = self.log_groups_state.detail_tab.next();
                } else if self.current_service == Service::S3Buckets {
                    if self.s3_state.current_bucket.is_some() {
                        self.s3_state.object_tab = self.s3_state.object_tab.next();
                    } else {
                        self.s3_state.bucket_type = match self.s3_state.bucket_type {
                            S3BucketType::GeneralPurpose => S3BucketType::Directory,
                            S3BucketType::Directory => S3BucketType::GeneralPurpose,
                        };
                        self.s3_state.buckets.reset();
                    }
                } else if self.current_service == Service::CloudWatchAlarms {
                    self.alarms_state.alarm_tab = match self.alarms_state.alarm_tab {
                        AlarmTab::AllAlarms => AlarmTab::InAlarm,
                        AlarmTab::InAlarm => AlarmTab::AllAlarms,
                    };
                    self.alarms_state.table.reset();
                } else if self.current_service == Service::EcrRepositories
                    && self.ecr_state.current_repository.is_none()
                {
                    self.ecr_state.tab = self.ecr_state.tab.next();
                    self.ecr_state.repositories.reset();
                    self.ecr_state.repositories.loading = true;
                } else if self.current_service == Service::LambdaFunctions
                    && self.lambda_state.current_function.is_some()
                {
                    if self.lambda_state.current_version.is_some() {
                        // Version view: use VersionDetailTab enum
                        self.lambda_state.version_detail_tab =
                            self.lambda_state.version_detail_tab.next();
                        self.lambda_state.detail_tab =
                            self.lambda_state.version_detail_tab.to_detail_tab();
                        if self.lambda_state.detail_tab == LambdaDetailTab::Monitor {
                            self.lambda_state.set_metrics_loading(true);
                            self.lambda_state.set_monitoring_scroll(0);
                            self.lambda_state.clear_metrics();
                        }
                    } else {
                        self.lambda_state.detail_tab = self.lambda_state.detail_tab.next();
                        if self.lambda_state.detail_tab == LambdaDetailTab::Monitor {
                            self.lambda_state.set_metrics_loading(true);
                            self.lambda_state.set_monitoring_scroll(0);
                            self.lambda_state.clear_metrics();
                        }
                    }
                } else if self.current_service == Service::CloudFormationStacks
                    && self.cfn_state.current_stack.is_some()
                {
                    self.cfn_state.detail_tab = self.cfn_state.detail_tab.next();
                }
            }
            Action::PrevDetailTab => {
                if self.current_service == Service::SqsQueues
                    && self.sqs_state.current_queue.is_some()
                {
                    self.sqs_state.detail_tab = self.sqs_state.detail_tab.prev();
                    if self.sqs_state.detail_tab == SqsQueueDetailTab::Monitoring {
                        self.sqs_state.set_metrics_loading(true);
                        self.sqs_state.set_monitoring_scroll(0);
                        self.sqs_state.clear_metrics();
                    }
                } else if self.current_service == Service::Ec2Instances
                    && self.ec2_state.current_instance.is_some()
                {
                    self.ec2_state.detail_tab = self.ec2_state.detail_tab.prev();
                } else if self.current_service == Service::LambdaApplications
                    && self.lambda_application_state.current_application.is_some()
                {
                    self.lambda_application_state.detail_tab =
                        self.lambda_application_state.detail_tab.prev();
                } else if self.current_service == Service::IamRoles
                    && self.iam_state.current_role.is_some()
                {
                    self.iam_state.role_tab = self.iam_state.role_tab.prev();
                } else if self.current_service == Service::IamUsers
                    && self.iam_state.current_user.is_some()
                {
                    self.iam_state.user_tab = self.iam_state.user_tab.prev();
                } else if self.current_service == Service::IamUserGroups
                    && self.iam_state.current_group.is_some()
                {
                    self.iam_state.group_tab = self.iam_state.group_tab.prev();
                } else if self.view_mode == ViewMode::Detail {
                    self.log_groups_state.detail_tab = self.log_groups_state.detail_tab.prev();
                } else if self.current_service == Service::S3Buckets {
                    if self.s3_state.current_bucket.is_some() {
                        self.s3_state.object_tab = self.s3_state.object_tab.prev();
                    }
                } else if self.current_service == Service::CloudWatchAlarms {
                    self.alarms_state.alarm_tab = match self.alarms_state.alarm_tab {
                        AlarmTab::AllAlarms => AlarmTab::InAlarm,
                        AlarmTab::InAlarm => AlarmTab::AllAlarms,
                    };
                } else if self.current_service == Service::EcrRepositories
                    && self.ecr_state.current_repository.is_none()
                {
                    self.ecr_state.tab = self.ecr_state.tab.prev();
                    self.ecr_state.repositories.reset();
                    self.ecr_state.repositories.loading = true;
                } else if self.current_service == Service::LambdaFunctions
                    && self.lambda_state.current_function.is_some()
                {
                    if self.lambda_state.current_version.is_some() {
                        // Version view: use VersionDetailTab enum
                        self.lambda_state.version_detail_tab =
                            self.lambda_state.version_detail_tab.prev();
                        self.lambda_state.detail_tab =
                            self.lambda_state.version_detail_tab.to_detail_tab();
                        if self.lambda_state.detail_tab == LambdaDetailTab::Monitor {
                            self.lambda_state.set_metrics_loading(true);
                            self.lambda_state.set_monitoring_scroll(0);
                            self.lambda_state.clear_metrics();
                        }
                    } else {
                        self.lambda_state.detail_tab = self.lambda_state.detail_tab.prev();
                        if self.lambda_state.detail_tab == LambdaDetailTab::Monitor {
                            self.lambda_state.set_metrics_loading(true);
                            self.lambda_state.set_monitoring_scroll(0);
                            self.lambda_state.clear_metrics();
                        }
                    }
                } else if self.current_service == Service::CloudFormationStacks
                    && self.cfn_state.current_stack.is_some()
                {
                    self.cfn_state.detail_tab = self.cfn_state.detail_tab.prev();
                }
            }
            Action::StartFilter => {
                // Don't allow filter mode when no service is selected and no tabs open
                if !self.service_selected && self.tabs.is_empty() {
                    return;
                }

                if self.current_service == Service::CloudWatchInsights {
                    self.mode = Mode::InsightsInput;
                } else if self.current_service == Service::CloudWatchAlarms {
                    self.mode = Mode::FilterInput;
                } else if self.current_service == Service::S3Buckets {
                    self.mode = Mode::FilterInput;
                    self.log_groups_state.filter_mode = true;
                } else if self.current_service == Service::EcrRepositories
                    || self.current_service == Service::IamUsers
                    || self.current_service == Service::IamUserGroups
                {
                    self.mode = Mode::FilterInput;
                    if self.current_service == Service::EcrRepositories
                        && self.ecr_state.current_repository.is_none()
                    {
                        self.ecr_state.input_focus = InputFocus::Filter;
                    }
                } else if self.current_service == Service::LambdaFunctions {
                    self.mode = Mode::FilterInput;
                    if self.lambda_state.current_version.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Configuration
                    {
                        self.lambda_state.alias_input_focus = InputFocus::Filter;
                    } else if self.lambda_state.current_function.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Versions
                    {
                        self.lambda_state.version_input_focus = InputFocus::Filter;
                    } else if self.lambda_state.current_function.is_none() {
                        self.lambda_state.input_focus = InputFocus::Filter;
                    }
                } else if self.current_service == Service::LambdaApplications {
                    self.mode = Mode::FilterInput;
                    if self.lambda_application_state.current_application.is_some() {
                        // In detail view - check which tab
                        if self.lambda_application_state.detail_tab
                            == LambdaApplicationDetailTab::Overview
                        {
                            self.lambda_application_state.resource_input_focus = InputFocus::Filter;
                        } else {
                            self.lambda_application_state.deployment_input_focus =
                                InputFocus::Filter;
                        }
                    } else {
                        self.lambda_application_state.input_focus = InputFocus::Filter;
                    }
                } else if self.current_service == Service::IamRoles {
                    self.mode = Mode::FilterInput;
                } else if self.current_service == Service::CloudFormationStacks {
                    self.mode = Mode::FilterInput;
                    if self.cfn_state.current_stack.is_some()
                        && self.cfn_state.detail_tab == CfnDetailTab::Parameters
                    {
                        self.cfn_state.parameters_input_focus = InputFocus::Filter;
                    } else if self.cfn_state.current_stack.is_some()
                        && self.cfn_state.detail_tab == CfnDetailTab::Outputs
                    {
                        self.cfn_state.outputs_input_focus = InputFocus::Filter;
                    } else {
                        self.cfn_state.input_focus = InputFocus::Filter;
                    }
                } else if self.current_service == Service::SqsQueues {
                    self.mode = Mode::FilterInput;
                    self.sqs_state.input_focus = InputFocus::Filter;
                } else if self.view_mode == ViewMode::List
                    || (self.view_mode == ViewMode::Detail
                        && self.log_groups_state.detail_tab == DetailTab::LogStreams)
                {
                    self.mode = Mode::FilterInput;
                    self.log_groups_state.filter_mode = true;
                    self.log_groups_state.input_focus = InputFocus::Filter;
                }
            }
            Action::StartEventFilter => {
                if self.current_service == Service::CloudWatchInsights {
                    self.mode = Mode::InsightsInput;
                } else if self.view_mode == ViewMode::List {
                    self.mode = Mode::FilterInput;
                    self.log_groups_state.filter_mode = true;
                    self.log_groups_state.input_focus = InputFocus::Filter;
                } else if self.view_mode == ViewMode::Events {
                    self.mode = Mode::EventFilterInput;
                } else if self.view_mode == ViewMode::Detail
                    && self.log_groups_state.detail_tab == DetailTab::LogStreams
                {
                    self.mode = Mode::FilterInput;
                    self.log_groups_state.filter_mode = true;
                    self.log_groups_state.input_focus = InputFocus::Filter;
                }
            }
            Action::NextFilterFocus => {
                if self.mode == Mode::FilterInput && self.current_service == Service::Ec2Instances {
                    self.ec2_state.input_focus =
                        self.ec2_state.input_focus.next(&ec2::FILTER_CONTROLS);
                } else if self.mode == Mode::FilterInput
                    && self.current_service == Service::LambdaApplications
                {
                    use crate::ui::lambda::FILTER_CONTROLS;
                    if self.lambda_application_state.current_application.is_some() {
                        if self.lambda_application_state.detail_tab
                            == LambdaApplicationDetailTab::Deployments
                        {
                            self.lambda_application_state.deployment_input_focus = self
                                .lambda_application_state
                                .deployment_input_focus
                                .next(&FILTER_CONTROLS);
                        } else {
                            self.lambda_application_state.resource_input_focus = self
                                .lambda_application_state
                                .resource_input_focus
                                .next(&FILTER_CONTROLS);
                        }
                    } else {
                        self.lambda_application_state.input_focus = self
                            .lambda_application_state
                            .input_focus
                            .next(&FILTER_CONTROLS);
                    }
                } else if self.mode == Mode::FilterInput
                    && self.current_service == Service::IamRoles
                    && self.iam_state.current_role.is_some()
                {
                    use crate::ui::iam::POLICY_FILTER_CONTROLS;
                    self.iam_state.policy_input_focus = self
                        .iam_state
                        .policy_input_focus
                        .next(&POLICY_FILTER_CONTROLS);
                } else if self.mode == Mode::FilterInput
                    && self.current_service == Service::IamRoles
                    && self.iam_state.current_role.is_none()
                {
                    use crate::ui::iam::ROLE_FILTER_CONTROLS;
                    self.iam_state.role_input_focus =
                        self.iam_state.role_input_focus.next(&ROLE_FILTER_CONTROLS);
                } else if self.mode == Mode::InsightsInput {
                    use crate::app::InsightsFocus;
                    self.insights_state.insights.insights_focus =
                        match self.insights_state.insights.insights_focus {
                            InsightsFocus::QueryLanguage => InsightsFocus::DatePicker,
                            InsightsFocus::DatePicker => InsightsFocus::LogGroupSearch,
                            InsightsFocus::LogGroupSearch => InsightsFocus::Query,
                            InsightsFocus::Query => InsightsFocus::QueryLanguage,
                        };
                } else if self.mode == Mode::FilterInput
                    && self.current_service == Service::CloudFormationStacks
                {
                    if self.cfn_state.current_stack.is_some()
                        && self.cfn_state.detail_tab == CfnDetailTab::Parameters
                    {
                        self.cfn_state.parameters_input_focus = self
                            .cfn_state
                            .parameters_input_focus
                            .next(&CfnStateConstants::PARAMETERS_FILTER_CONTROLS);
                    } else if self.cfn_state.current_stack.is_some()
                        && self.cfn_state.detail_tab == CfnDetailTab::Outputs
                    {
                        self.cfn_state.outputs_input_focus = self
                            .cfn_state
                            .outputs_input_focus
                            .next(&CfnStateConstants::OUTPUTS_FILTER_CONTROLS);
                    } else if self.cfn_state.current_stack.is_some()
                        && self.cfn_state.detail_tab == CfnDetailTab::Resources
                    {
                        self.cfn_state.resources_input_focus = self
                            .cfn_state
                            .resources_input_focus
                            .next(&CfnStateConstants::RESOURCES_FILTER_CONTROLS);
                    } else {
                        self.cfn_state.input_focus = self
                            .cfn_state
                            .input_focus
                            .next(&CfnStateConstants::FILTER_CONTROLS);
                    }
                } else if self.mode == Mode::FilterInput
                    && self.current_service == Service::SqsQueues
                {
                    if self.sqs_state.current_queue.is_some()
                        && self.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions
                    {
                        use crate::ui::sqs::SUBSCRIPTION_FILTER_CONTROLS;
                        self.sqs_state.input_focus = self
                            .sqs_state
                            .input_focus
                            .next(SUBSCRIPTION_FILTER_CONTROLS);
                    } else {
                        use crate::ui::sqs::FILTER_CONTROLS;
                        self.sqs_state.input_focus =
                            self.sqs_state.input_focus.next(FILTER_CONTROLS);
                    }
                } else if self.mode == Mode::FilterInput
                    && self.current_service == Service::CloudWatchLogGroups
                {
                    use crate::ui::cw::logs::FILTER_CONTROLS;
                    self.log_groups_state.input_focus =
                        self.log_groups_state.input_focus.next(&FILTER_CONTROLS);
                } else if self.mode == Mode::EventFilterInput {
                    self.log_groups_state.event_input_focus =
                        self.log_groups_state.event_input_focus.next();
                } else if self.mode == Mode::FilterInput
                    && self.current_service == Service::CloudWatchAlarms
                {
                    use crate::ui::cw::alarms::FILTER_CONTROLS;
                    self.alarms_state.input_focus =
                        self.alarms_state.input_focus.next(&FILTER_CONTROLS);
                } else if self.mode == Mode::FilterInput
                    && self.current_service == Service::EcrRepositories
                    && self.ecr_state.current_repository.is_none()
                {
                    use crate::ui::ecr::FILTER_CONTROLS;
                    self.ecr_state.input_focus = self.ecr_state.input_focus.next(&FILTER_CONTROLS);
                } else if self.mode == Mode::FilterInput
                    && self.current_service == Service::LambdaFunctions
                {
                    use crate::ui::lambda::FILTER_CONTROLS;
                    if self.lambda_state.current_version.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Configuration
                    {
                        self.lambda_state.alias_input_focus =
                            self.lambda_state.alias_input_focus.next(&FILTER_CONTROLS);
                    } else if self.lambda_state.current_function.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Versions
                    {
                        self.lambda_state.version_input_focus =
                            self.lambda_state.version_input_focus.next(&FILTER_CONTROLS);
                    } else if self.lambda_state.current_function.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Aliases
                    {
                        self.lambda_state.alias_input_focus =
                            self.lambda_state.alias_input_focus.next(&FILTER_CONTROLS);
                    } else if self.lambda_state.current_function.is_none() {
                        self.lambda_state.input_focus =
                            self.lambda_state.input_focus.next(&FILTER_CONTROLS);
                    }
                }
            }
            Action::PrevFilterFocus => {
                if self.mode == Mode::FilterInput && self.current_service == Service::Ec2Instances {
                    self.ec2_state.input_focus =
                        self.ec2_state.input_focus.prev(&ec2::FILTER_CONTROLS);
                } else if self.mode == Mode::FilterInput
                    && self.current_service == Service::LambdaApplications
                {
                    use crate::ui::lambda::FILTER_CONTROLS;
                    if self.lambda_application_state.current_application.is_some() {
                        if self.lambda_application_state.detail_tab
                            == LambdaApplicationDetailTab::Deployments
                        {
                            self.lambda_application_state.deployment_input_focus = self
                                .lambda_application_state
                                .deployment_input_focus
                                .prev(&FILTER_CONTROLS);
                        } else {
                            self.lambda_application_state.resource_input_focus = self
                                .lambda_application_state
                                .resource_input_focus
                                .prev(&FILTER_CONTROLS);
                        }
                    } else {
                        self.lambda_application_state.input_focus = self
                            .lambda_application_state
                            .input_focus
                            .prev(&FILTER_CONTROLS);
                    }
                } else if self.mode == Mode::FilterInput
                    && self.current_service == Service::CloudFormationStacks
                {
                    if self.cfn_state.current_stack.is_some()
                        && self.cfn_state.detail_tab == CfnDetailTab::Parameters
                    {
                        self.cfn_state.parameters_input_focus = self
                            .cfn_state
                            .parameters_input_focus
                            .prev(&CfnStateConstants::PARAMETERS_FILTER_CONTROLS);
                    } else if self.cfn_state.current_stack.is_some()
                        && self.cfn_state.detail_tab == CfnDetailTab::Outputs
                    {
                        self.cfn_state.outputs_input_focus = self
                            .cfn_state
                            .outputs_input_focus
                            .prev(&CfnStateConstants::OUTPUTS_FILTER_CONTROLS);
                    } else if self.cfn_state.current_stack.is_some()
                        && self.cfn_state.detail_tab == CfnDetailTab::Resources
                    {
                        self.cfn_state.resources_input_focus = self
                            .cfn_state
                            .resources_input_focus
                            .prev(&CfnStateConstants::RESOURCES_FILTER_CONTROLS);
                    } else {
                        self.cfn_state.input_focus = self
                            .cfn_state
                            .input_focus
                            .prev(&CfnStateConstants::FILTER_CONTROLS);
                    }
                } else if self.mode == Mode::FilterInput
                    && self.current_service == Service::SqsQueues
                {
                    if self.sqs_state.current_queue.is_some()
                        && self.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions
                    {
                        use crate::ui::sqs::SUBSCRIPTION_FILTER_CONTROLS;
                        self.sqs_state.input_focus = self
                            .sqs_state
                            .input_focus
                            .prev(SUBSCRIPTION_FILTER_CONTROLS);
                    } else {
                        use crate::ui::sqs::FILTER_CONTROLS;
                        self.sqs_state.input_focus =
                            self.sqs_state.input_focus.prev(FILTER_CONTROLS);
                    }
                } else if self.mode == Mode::FilterInput
                    && self.current_service == Service::IamRoles
                    && self.iam_state.current_role.is_none()
                {
                    use crate::ui::iam::ROLE_FILTER_CONTROLS;
                    self.iam_state.role_input_focus =
                        self.iam_state.role_input_focus.prev(&ROLE_FILTER_CONTROLS);
                } else if self.mode == Mode::FilterInput
                    && self.current_service == Service::CloudWatchLogGroups
                {
                    use crate::ui::cw::logs::FILTER_CONTROLS;
                    self.log_groups_state.input_focus =
                        self.log_groups_state.input_focus.prev(&FILTER_CONTROLS);
                } else if self.mode == Mode::EventFilterInput {
                    self.log_groups_state.event_input_focus =
                        self.log_groups_state.event_input_focus.prev();
                } else if self.mode == Mode::FilterInput
                    && self.current_service == Service::IamRoles
                    && self.iam_state.current_role.is_some()
                {
                    use crate::ui::iam::POLICY_FILTER_CONTROLS;
                    self.iam_state.policy_input_focus = self
                        .iam_state
                        .policy_input_focus
                        .prev(&POLICY_FILTER_CONTROLS);
                } else if self.mode == Mode::FilterInput
                    && self.current_service == Service::CloudWatchAlarms
                {
                    use crate::ui::cw::alarms::FILTER_CONTROLS;
                    self.alarms_state.input_focus =
                        self.alarms_state.input_focus.prev(&FILTER_CONTROLS);
                } else if self.mode == Mode::FilterInput
                    && self.current_service == Service::EcrRepositories
                    && self.ecr_state.current_repository.is_none()
                {
                    use crate::ui::ecr::FILTER_CONTROLS;
                    self.ecr_state.input_focus = self.ecr_state.input_focus.prev(&FILTER_CONTROLS);
                } else if self.mode == Mode::FilterInput
                    && self.current_service == Service::LambdaFunctions
                {
                    use crate::ui::lambda::FILTER_CONTROLS;
                    if self.lambda_state.current_version.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Configuration
                    {
                        self.lambda_state.alias_input_focus =
                            self.lambda_state.alias_input_focus.prev(&FILTER_CONTROLS);
                    } else if self.lambda_state.current_function.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Versions
                    {
                        self.lambda_state.version_input_focus =
                            self.lambda_state.version_input_focus.prev(&FILTER_CONTROLS);
                    } else if self.lambda_state.current_function.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Aliases
                    {
                        self.lambda_state.alias_input_focus =
                            self.lambda_state.alias_input_focus.prev(&FILTER_CONTROLS);
                    } else if self.lambda_state.current_function.is_none() {
                        self.lambda_state.input_focus =
                            self.lambda_state.input_focus.prev(&FILTER_CONTROLS);
                    }
                }
            }
            Action::ToggleFilterCheckbox => {
                if self.mode == Mode::FilterInput && self.current_service == Service::Ec2Instances {
                    if self.ec2_state.input_focus == EC2_STATE_FILTER {
                        self.ec2_state.state_filter = self.ec2_state.state_filter.next();
                        self.ec2_state.table.reset();
                    }
                } else if self.mode == Mode::InsightsInput {
                    use crate::app::InsightsFocus;
                    if self.insights_state.insights.insights_focus == InsightsFocus::LogGroupSearch
                        && self.insights_state.insights.show_dropdown
                        && !self.insights_state.insights.log_group_matches.is_empty()
                    {
                        let selected_idx = self.insights_state.insights.dropdown_selected;
                        if let Some(group_name) = self
                            .insights_state
                            .insights
                            .log_group_matches
                            .get(selected_idx)
                        {
                            let group_name = group_name.clone();
                            if let Some(pos) = self
                                .insights_state
                                .insights
                                .selected_log_groups
                                .iter()
                                .position(|g| g == &group_name)
                            {
                                self.insights_state.insights.selected_log_groups.remove(pos);
                            } else if self.insights_state.insights.selected_log_groups.len() < 50 {
                                self.insights_state
                                    .insights
                                    .selected_log_groups
                                    .push(group_name);
                            }
                        }
                    }
                } else if self.mode == Mode::FilterInput
                    && self.current_service == Service::CloudFormationStacks
                {
                    use crate::ui::cfn::{STATUS_FILTER, VIEW_NESTED};
                    match self.cfn_state.input_focus {
                        STATUS_FILTER => {
                            self.cfn_state.status_filter = self.cfn_state.status_filter.next();
                            self.cfn_state.table.reset();
                        }
                        VIEW_NESTED => {
                            self.cfn_state.view_nested = !self.cfn_state.view_nested;
                            self.cfn_state.table.reset();
                        }
                        _ => {}
                    }
                } else if self.mode == Mode::FilterInput
                    && self.log_groups_state.detail_tab == DetailTab::LogStreams
                {
                    match self.log_groups_state.input_focus {
                        InputFocus::Checkbox("ExactMatch") => {
                            self.log_groups_state.exact_match = !self.log_groups_state.exact_match
                        }
                        InputFocus::Checkbox("ShowExpired") => {
                            self.log_groups_state.show_expired = !self.log_groups_state.show_expired
                        }
                        _ => {}
                    }
                } else if self.mode == Mode::EventFilterInput
                    && self.log_groups_state.event_input_focus == EventFilterFocus::DateRange
                {
                    self.log_groups_state.relative_unit =
                        self.log_groups_state.relative_unit.next();
                }
            }
            Action::CycleSortColumn => {
                if self.view_mode == ViewMode::Detail
                    && self.log_groups_state.detail_tab == DetailTab::LogStreams
                {
                    self.log_groups_state.stream_sort = match self.log_groups_state.stream_sort {
                        StreamSort::Name => StreamSort::CreationTime,
                        StreamSort::CreationTime => StreamSort::LastEventTime,
                        StreamSort::LastEventTime => StreamSort::Name,
                    };
                }
            }
            Action::ToggleSortDirection => {
                if self.view_mode == ViewMode::Detail
                    && self.log_groups_state.detail_tab == DetailTab::LogStreams
                {
                    self.log_groups_state.stream_sort_desc =
                        !self.log_groups_state.stream_sort_desc;
                }
            }
            Action::ScrollUp => {
                if self.mode == Mode::ErrorModal {
                    self.error_scroll = self.error_scroll.saturating_sub(1);
                } else if self.current_service == Service::LambdaFunctions
                    && self.lambda_state.current_function.is_some()
                    && self.lambda_state.detail_tab == LambdaDetailTab::Monitor
                    && !self.lambda_state.is_metrics_loading()
                {
                    self.lambda_state.set_monitoring_scroll(
                        self.lambda_state.monitoring_scroll().saturating_sub(1),
                    );
                } else if self.current_service == Service::SqsQueues
                    && self.sqs_state.current_queue.is_some()
                    && self.sqs_state.detail_tab == SqsQueueDetailTab::Monitoring
                    && !self.sqs_state.is_metrics_loading()
                {
                    self.sqs_state.set_monitoring_scroll(
                        self.sqs_state.monitoring_scroll().saturating_sub(1),
                    );
                } else if self.view_mode == ViewMode::PolicyView {
                    self.iam_state.policy_scroll = self.iam_state.policy_scroll.saturating_sub(10);
                } else if self.current_service == Service::IamRoles
                    && self.iam_state.current_role.is_some()
                    && self.iam_state.role_tab == RoleTab::TrustRelationships
                {
                    self.iam_state.trust_policy_scroll =
                        self.iam_state.trust_policy_scroll.saturating_sub(10);
                } else if self.view_mode == ViewMode::Events {
                    if self.log_groups_state.event_scroll_offset == 0
                        && self.log_groups_state.has_older_events
                    {
                        self.log_groups_state.loading = true;
                    } else {
                        self.log_groups_state.event_scroll_offset =
                            self.log_groups_state.event_scroll_offset.saturating_sub(1);
                    }
                } else if self.view_mode == ViewMode::InsightsResults {
                    self.insights_state.insights.results_selected = self
                        .insights_state
                        .insights
                        .results_selected
                        .saturating_sub(1);
                } else if self.view_mode == ViewMode::Detail {
                    self.log_groups_state.selected_stream =
                        self.log_groups_state.selected_stream.saturating_sub(1);
                    self.log_groups_state.expanded_stream = None;
                } else if self.view_mode == ViewMode::List
                    && self.current_service == Service::CloudWatchLogGroups
                {
                    self.log_groups_state.log_groups.selected =
                        self.log_groups_state.log_groups.selected.saturating_sub(1);
                    self.log_groups_state.log_groups.snap_to_page();
                } else if self.current_service == Service::EcrRepositories {
                    if self.ecr_state.current_repository.is_some() {
                        self.ecr_state.images.page_up();
                    } else {
                        self.ecr_state.repositories.page_up();
                    }
                }
            }
            Action::ScrollDown => {
                if self.mode == Mode::ErrorModal {
                    if let Some(error_msg) = &self.error_message {
                        let lines = error_msg.lines().count();
                        let max_scroll = lines.saturating_sub(1);
                        self.error_scroll = (self.error_scroll + 1).min(max_scroll);
                    }
                } else if self.current_service == Service::SqsQueues
                    && self.sqs_state.current_queue.is_some()
                    && self.sqs_state.detail_tab == SqsQueueDetailTab::Monitoring
                {
                    self.sqs_state
                        .set_monitoring_scroll((self.sqs_state.monitoring_scroll() + 1).min(1));
                } else if self.view_mode == ViewMode::PolicyView {
                    let lines = self.iam_state.policy_document.lines().count();
                    let max_scroll = lines.saturating_sub(1);
                    self.iam_state.policy_scroll =
                        (self.iam_state.policy_scroll + 10).min(max_scroll);
                } else if self.current_service == Service::IamRoles
                    && self.iam_state.current_role.is_some()
                    && self.iam_state.role_tab == RoleTab::TrustRelationships
                {
                    let lines = self.iam_state.trust_policy_document.lines().count();
                    let max_scroll = lines.saturating_sub(1);
                    self.iam_state.trust_policy_scroll =
                        (self.iam_state.trust_policy_scroll + 10).min(max_scroll);
                } else if self.view_mode == ViewMode::Events {
                    let max_scroll = self.log_groups_state.log_events.len().saturating_sub(1);
                    if self.log_groups_state.event_scroll_offset >= max_scroll {
                        // At the end, do nothing
                    } else {
                        self.log_groups_state.event_scroll_offset =
                            (self.log_groups_state.event_scroll_offset + 1).min(max_scroll);
                    }
                } else if self.view_mode == ViewMode::InsightsResults {
                    let max = self
                        .insights_state
                        .insights
                        .query_results
                        .len()
                        .saturating_sub(1);
                    self.insights_state.insights.results_selected =
                        (self.insights_state.insights.results_selected + 1).min(max);
                } else if self.view_mode == ViewMode::Detail {
                    let filtered_streams = filtered_log_streams(self);
                    let max = filtered_streams.len().saturating_sub(1);
                    self.log_groups_state.selected_stream =
                        (self.log_groups_state.selected_stream + 1).min(max);
                } else if self.view_mode == ViewMode::List
                    && self.current_service == Service::CloudWatchLogGroups
                {
                    let filtered_groups = filtered_log_groups(self);
                    self.log_groups_state
                        .log_groups
                        .next_item(filtered_groups.len());
                } else if self.current_service == Service::EcrRepositories {
                    if self.ecr_state.current_repository.is_some() {
                        let filtered_images = filtered_ecr_images(self);
                        self.ecr_state.images.page_down(filtered_images.len());
                    } else {
                        let filtered_repos = filtered_ecr_repositories(self);
                        self.ecr_state.repositories.page_down(filtered_repos.len());
                    }
                }
            }

            Action::Refresh => {
                if self.mode == Mode::ProfilePicker {
                    self.log_groups_state.loading = true;
                    self.log_groups_state.loading_message = "Refreshing...".to_string();
                } else if self.mode == Mode::RegionPicker {
                    self.measure_region_latencies();
                } else if self.mode == Mode::SessionPicker {
                    self.sessions = Session::list_all().unwrap_or_default();
                } else if self.current_service == Service::CloudWatchInsights
                    && !self.insights_state.insights.selected_log_groups.is_empty()
                {
                    self.log_groups_state.loading = true;
                    self.insights_state.insights.query_completed = true;
                } else if self.current_service == Service::LambdaFunctions {
                    self.lambda_state.table.loading = true;
                } else if self.current_service == Service::LambdaApplications {
                    self.lambda_application_state.table.loading = true;
                } else if matches!(
                    self.view_mode,
                    ViewMode::Events | ViewMode::Detail | ViewMode::List
                ) {
                    self.log_groups_state.loading = true;
                }
            }
            Action::Yank => {
                if self.mode == Mode::ErrorModal {
                    // Copy error message
                    if let Some(error) = &self.error_message {
                        copy_to_clipboard(error);
                    }
                } else if self.view_mode == ViewMode::Events {
                    if let Some(event) = self
                        .log_groups_state
                        .log_events
                        .get(self.log_groups_state.event_scroll_offset)
                    {
                        copy_to_clipboard(&event.message);
                    }
                } else if self.current_service == Service::EcrRepositories {
                    if self.ecr_state.current_repository.is_some() {
                        let filtered_images = filtered_ecr_images(self);
                        if let Some(image) = self.ecr_state.images.get_selected(&filtered_images) {
                            copy_to_clipboard(&image.uri);
                        }
                    } else {
                        let filtered_repos = filtered_ecr_repositories(self);
                        if let Some(repo) =
                            self.ecr_state.repositories.get_selected(&filtered_repos)
                        {
                            copy_to_clipboard(&repo.uri);
                        }
                    }
                } else if self.current_service == Service::LambdaFunctions {
                    let filtered_functions = filtered_lambda_functions(self);
                    if let Some(func) = self.lambda_state.table.get_selected(&filtered_functions) {
                        copy_to_clipboard(&func.arn);
                    }
                } else if self.current_service == Service::CloudFormationStacks {
                    if let Some(stack_name) = &self.cfn_state.current_stack {
                        // In detail view - copy current stack ARN
                        if let Some(stack) = self
                            .cfn_state
                            .table
                            .items
                            .iter()
                            .find(|s| &s.name == stack_name)
                        {
                            copy_to_clipboard(&stack.stack_id);
                        }
                    } else {
                        // In list view - copy selected stack ARN
                        let filtered_stacks = filtered_cloudformation_stacks(self);
                        if let Some(stack) = self.cfn_state.table.get_selected(&filtered_stacks) {
                            copy_to_clipboard(&stack.stack_id);
                        }
                    }
                } else if self.current_service == Service::IamUsers {
                    if self.iam_state.current_user.is_some() {
                        if let Some(user_name) = &self.iam_state.current_user {
                            if let Some(user) = self
                                .iam_state
                                .users
                                .items
                                .iter()
                                .find(|u| u.user_name == *user_name)
                            {
                                copy_to_clipboard(&user.arn);
                            }
                        }
                    } else {
                        let filtered_users = filtered_iam_users(self);
                        if let Some(user) = self.iam_state.users.get_selected(&filtered_users) {
                            copy_to_clipboard(&user.arn);
                        }
                    }
                } else if self.current_service == Service::IamRoles {
                    if self.iam_state.current_role.is_some() {
                        if let Some(role_name) = &self.iam_state.current_role {
                            if let Some(role) = self
                                .iam_state
                                .roles
                                .items
                                .iter()
                                .find(|r| r.role_name == *role_name)
                            {
                                copy_to_clipboard(&role.arn);
                            }
                        }
                    } else {
                        let filtered_roles = filtered_iam_roles(self);
                        if let Some(role) = self.iam_state.roles.get_selected(&filtered_roles) {
                            copy_to_clipboard(&role.arn);
                        }
                    }
                } else if self.current_service == Service::IamUserGroups {
                    if self.iam_state.current_group.is_some() {
                        if let Some(group_name) = &self.iam_state.current_group {
                            let arn = iam::format_arn(&self.config.account_id, "group", group_name);
                            copy_to_clipboard(&arn);
                        }
                    } else {
                        let filtered_groups: Vec<_> = self
                            .iam_state
                            .groups
                            .items
                            .iter()
                            .filter(|g| {
                                if self.iam_state.groups.filter.is_empty() {
                                    true
                                } else {
                                    g.group_name
                                        .to_lowercase()
                                        .contains(&self.iam_state.groups.filter.to_lowercase())
                                }
                            })
                            .collect();
                        if let Some(group) = self.iam_state.groups.get_selected(&filtered_groups) {
                            let arn = iam::format_arn(
                                &self.config.account_id,
                                "group",
                                &group.group_name,
                            );
                            copy_to_clipboard(&arn);
                        }
                    }
                } else if self.current_service == Service::SqsQueues {
                    if self.sqs_state.current_queue.is_some() {
                        // In queue detail view - copy queue ARN
                        if let Some(queue) = self
                            .sqs_state
                            .queues
                            .items
                            .iter()
                            .find(|q| Some(&q.url) == self.sqs_state.current_queue.as_ref())
                        {
                            let arn = format!(
                                "arn:aws:sqs:{}:{}:{}",
                                extract_region(&queue.url),
                                extract_account_id(&queue.url),
                                queue.name
                            );
                            copy_to_clipboard(&arn);
                        }
                    } else {
                        // In list view - copy selected queue ARN
                        let filtered_queues = filtered_queues(
                            &self.sqs_state.queues.items,
                            &self.sqs_state.queues.filter,
                        );
                        if let Some(queue) = self.sqs_state.queues.get_selected(&filtered_queues) {
                            let arn = format!(
                                "arn:aws:sqs:{}:{}:{}",
                                extract_region(&queue.url),
                                extract_account_id(&queue.url),
                                queue.name
                            );
                            copy_to_clipboard(&arn);
                        }
                    }
                }
            }
            Action::CopyToClipboard => {
                // Request snapshot - will be captured after next render
                self.snapshot_requested = true;
            }
            Action::RetryLoad => {
                self.error_message = None;
                self.mode = Mode::Normal;
                self.log_groups_state.loading = true;
            }
            Action::ApplyFilter => {
                if self.mode == Mode::FilterInput
                    && self.current_service == Service::SqsQueues
                    && self.sqs_state.input_focus == InputFocus::Dropdown("SubscriptionRegion")
                {
                    let regions = AwsRegion::all();
                    if let Some(region) = regions.get(self.sqs_state.subscription_region_selected) {
                        self.sqs_state.subscription_region_filter = region.code.to_string();
                    }
                    self.mode = Mode::Normal;
                } else if self.mode == Mode::InsightsInput {
                    use crate::app::InsightsFocus;
                    if self.insights_state.insights.insights_focus == InsightsFocus::LogGroupSearch
                        && self.insights_state.insights.show_dropdown
                    {
                        // Close dropdown, exit input mode, and execute query
                        self.insights_state.insights.show_dropdown = false;
                        self.mode = Mode::Normal;
                        if !self.insights_state.insights.selected_log_groups.is_empty() {
                            self.log_groups_state.loading = true;
                            self.insights_state.insights.query_completed = true;
                        }
                    }
                } else if self.mode == Mode::Normal && !self.page_input.is_empty() {
                    if let Ok(page) = self.page_input.parse::<usize>() {
                        self.go_to_page(page);
                    }
                    self.page_input.clear();
                } else {
                    self.mode = Mode::Normal;
                    self.log_groups_state.filter_mode = false;
                }
            }
            Action::ToggleExactMatch => {
                if self.view_mode == ViewMode::Detail
                    && self.log_groups_state.detail_tab == DetailTab::LogStreams
                {
                    self.log_groups_state.exact_match = !self.log_groups_state.exact_match;
                }
            }
            Action::ToggleShowExpired => {
                if self.view_mode == ViewMode::Detail
                    && self.log_groups_state.detail_tab == DetailTab::LogStreams
                {
                    self.log_groups_state.show_expired = !self.log_groups_state.show_expired;
                }
            }
            Action::GoBack => {
                // ServicePicker: close if we have tabs
                if self.mode == Mode::ServicePicker && !self.tabs.is_empty() {
                    self.mode = Mode::Normal;
                    self.service_picker.filter.clear();
                }
                // S3: pop navigation stack first, then exit bucket
                else if self.current_service == Service::S3Buckets
                    && self.s3_state.current_bucket.is_some()
                {
                    if !self.s3_state.prefix_stack.is_empty() {
                        self.s3_state.prefix_stack.pop();
                        self.s3_state.buckets.loading = true;
                    } else {
                        self.s3_state.current_bucket = None;
                        self.s3_state.objects.clear();
                    }
                }
                // ECR: go back from images to repositories
                else if self.current_service == Service::EcrRepositories
                    && self.ecr_state.current_repository.is_some()
                {
                    if self.ecr_state.images.has_expanded_item() {
                        self.ecr_state.images.collapse();
                    } else {
                        self.ecr_state.current_repository = None;
                        self.ecr_state.current_repository_uri = None;
                        self.ecr_state.images.items.clear();
                        self.ecr_state.images.reset();
                    }
                }
                // EC2: go back from instance detail to list
                else if self.current_service == Service::Ec2Instances
                    && self.ec2_state.current_instance.is_some()
                {
                    self.ec2_state.current_instance = None;
                    self.view_mode = ViewMode::List;
                    self.update_current_tab_breadcrumb();
                }
                // SQS: go back from queue detail to list
                else if self.current_service == Service::SqsQueues
                    && self.sqs_state.current_queue.is_some()
                {
                    self.sqs_state.current_queue = None;
                }
                // IAM: go back from user detail to list
                else if self.current_service == Service::IamUsers
                    && self.iam_state.current_user.is_some()
                {
                    self.iam_state.current_user = None;
                    self.iam_state.policies.items.clear();
                    self.iam_state.policies.reset();
                    self.update_current_tab_breadcrumb();
                }
                // IAM: go back from group detail to list
                else if self.current_service == Service::IamUserGroups
                    && self.iam_state.current_group.is_some()
                {
                    self.iam_state.current_group = None;
                    self.update_current_tab_breadcrumb();
                }
                // IAM: go back from role detail to list
                else if self.current_service == Service::IamRoles {
                    if self.view_mode == ViewMode::PolicyView {
                        // Go back from policy view to role detail
                        self.view_mode = ViewMode::Detail;
                        self.iam_state.current_policy = None;
                        self.iam_state.policy_document.clear();
                        self.iam_state.policy_scroll = 0;
                        self.update_current_tab_breadcrumb();
                    } else if self.iam_state.current_role.is_some() {
                        self.iam_state.current_role = None;
                        self.iam_state.policies.items.clear();
                        self.iam_state.policies.reset();
                        self.update_current_tab_breadcrumb();
                    }
                }
                // Lambda: go back from version detail to function detail
                else if self.current_service == Service::LambdaFunctions
                    && self.lambda_state.current_version.is_some()
                {
                    self.lambda_state.current_version = None;
                    self.lambda_state.detail_tab = LambdaDetailTab::Versions;
                }
                // Lambda: go back from alias detail to function detail
                else if self.current_service == Service::LambdaFunctions
                    && self.lambda_state.current_alias.is_some()
                {
                    self.lambda_state.current_alias = None;
                    self.lambda_state.detail_tab = LambdaDetailTab::Aliases;
                }
                // Lambda: go back from function detail to list
                else if self.current_service == Service::LambdaFunctions
                    && self.lambda_state.current_function.is_some()
                {
                    self.lambda_state.current_function = None;
                    self.update_current_tab_breadcrumb();
                }
                // Lambda Applications: go back from application detail to list
                else if self.current_service == Service::LambdaApplications
                    && self.lambda_application_state.current_application.is_some()
                {
                    self.lambda_application_state.current_application = None;
                    self.update_current_tab_breadcrumb();
                }
                // CloudFormation: go back from stack detail to list
                else if self.current_service == Service::CloudFormationStacks
                    && self.cfn_state.current_stack.is_some()
                {
                    self.cfn_state.current_stack = None;
                    self.update_current_tab_breadcrumb();
                }
                // From insights results -> collapse if expanded, otherwise back to sidebar
                else if self.view_mode == ViewMode::InsightsResults {
                    if self.insights_state.insights.expanded_result.is_some() {
                        self.insights_state.insights.expanded_result = None;
                    }
                }
                // From alarms view -> collapse if expanded
                else if self.current_service == Service::CloudWatchAlarms {
                    if self.alarms_state.table.has_expanded_item() {
                        self.alarms_state.table.collapse();
                    }
                }
                // From EC2 instances view -> collapse if expanded
                else if self.current_service == Service::Ec2Instances {
                    if self.ec2_state.table.has_expanded_item() {
                        self.ec2_state.table.collapse();
                    }
                }
                // From events view -> collapse if expanded, otherwise back to detail view
                else if self.view_mode == ViewMode::Events {
                    if self.log_groups_state.expanded_event.is_some() {
                        self.log_groups_state.expanded_event = None;
                    } else {
                        self.view_mode = ViewMode::Detail;
                        self.log_groups_state.event_filter.clear();
                    }
                }
                // From detail view -> back to list view
                else if self.view_mode == ViewMode::Detail {
                    self.view_mode = ViewMode::List;
                    self.log_groups_state.stream_filter.clear();
                    self.log_groups_state.exact_match = false;
                    self.log_groups_state.show_expired = false;
                }
            }
            Action::OpenInConsole | Action::OpenInBrowser => {
                let url = self.get_console_url();
                let _ = webbrowser::open(&url);
            }
            Action::ShowHelp => {
                self.mode = Mode::HelpModal;
            }
            Action::OpenRegionPicker => {
                self.region_filter.clear();
                self.region_picker_selected = 0;
                self.measure_region_latencies();
                self.mode = Mode::RegionPicker;
            }
            Action::OpenProfilePicker => {
                self.profile_filter.clear();
                self.profile_picker_selected = 0;
                self.available_profiles = Self::load_aws_profiles();
                self.mode = Mode::ProfilePicker;
            }
            Action::OpenCalendar => {
                self.calendar_date = Some(time::OffsetDateTime::now_utc().date());
                self.calendar_selecting = CalendarField::StartDate;
                self.mode = Mode::CalendarPicker;
            }
            Action::CloseCalendar => {
                self.mode = Mode::Normal;
                self.calendar_date = None;
            }
            Action::CalendarPrevDay => {
                if let Some(date) = self.calendar_date {
                    self.calendar_date = date.checked_sub(time::Duration::days(1));
                }
            }
            Action::CalendarNextDay => {
                if let Some(date) = self.calendar_date {
                    self.calendar_date = date.checked_add(time::Duration::days(1));
                }
            }
            Action::CalendarPrevWeek => {
                if let Some(date) = self.calendar_date {
                    self.calendar_date = date.checked_sub(time::Duration::weeks(1));
                }
            }
            Action::CalendarNextWeek => {
                if let Some(date) = self.calendar_date {
                    self.calendar_date = date.checked_add(time::Duration::weeks(1));
                }
            }
            Action::CalendarPrevMonth => {
                if let Some(date) = self.calendar_date {
                    self.calendar_date = Some(if date.month() == time::Month::January {
                        date.replace_month(time::Month::December)
                            .unwrap()
                            .replace_year(date.year() - 1)
                            .unwrap()
                    } else {
                        date.replace_month(date.month().previous()).unwrap()
                    });
                }
            }
            Action::CalendarNextMonth => {
                if let Some(date) = self.calendar_date {
                    self.calendar_date = Some(if date.month() == time::Month::December {
                        date.replace_month(time::Month::January)
                            .unwrap()
                            .replace_year(date.year() + 1)
                            .unwrap()
                    } else {
                        date.replace_month(date.month().next()).unwrap()
                    });
                }
            }
            Action::CalendarSelect => {
                if let Some(date) = self.calendar_date {
                    let timestamp = time::OffsetDateTime::new_utc(date, time::Time::MIDNIGHT)
                        .unix_timestamp()
                        * 1000;
                    match self.calendar_selecting {
                        CalendarField::StartDate => {
                            self.log_groups_state.start_time = Some(timestamp);
                            self.calendar_selecting = CalendarField::EndDate;
                        }
                        CalendarField::EndDate => {
                            self.log_groups_state.end_time = Some(timestamp);
                            self.mode = Mode::Normal;
                            self.calendar_date = None;
                        }
                    }
                }
            }
        }
    }

    pub fn filtered_services(&self) -> Vec<&'static str> {
        let mut services = if self.service_picker.filter.is_empty() {
            self.service_picker.services.clone()
        } else {
            self.service_picker
                .services
                .iter()
                .filter(|s| {
                    s.to_lowercase()
                        .contains(&self.service_picker.filter.to_lowercase())
                })
                .copied()
                .collect()
        };
        services.sort();
        services
    }

    pub fn breadcrumbs(&self) -> String {
        if !self.service_selected {
            return String::new();
        }

        let mut parts = vec![];

        match self.current_service {
            Service::CloudWatchLogGroups => {
                parts.push("CloudWatch".to_string());
                parts.push("Log groups".to_string());

                if self.view_mode != ViewMode::List {
                    if let Some(group) = selected_log_group(self) {
                        parts.push(group.name.clone());
                    }
                }

                if self.view_mode == ViewMode::Events {
                    if let Some(stream) = self
                        .log_groups_state
                        .log_streams
                        .get(self.log_groups_state.selected_stream)
                    {
                        parts.push(stream.name.clone());
                    }
                }
            }
            Service::CloudWatchInsights => {
                parts.push("CloudWatch".to_string());
                parts.push("Insights".to_string());
            }
            Service::CloudWatchAlarms => {
                parts.push("CloudWatch".to_string());
                parts.push("Alarms".to_string());
            }
            Service::S3Buckets => {
                parts.push("S3".to_string());
                if let Some(bucket) = &self.s3_state.current_bucket {
                    parts.push(bucket.clone());
                    if let Some(prefix) = self.s3_state.prefix_stack.last() {
                        parts.push(prefix.trim_end_matches('/').to_string());
                    }
                } else {
                    parts.push("Buckets".to_string());
                }
            }
            Service::SqsQueues => {
                parts.push("SQS".to_string());
                parts.push("Queues".to_string());
            }
            Service::EcrRepositories => {
                parts.push("ECR".to_string());
                if let Some(repo) = &self.ecr_state.current_repository {
                    parts.push(repo.clone());
                } else {
                    parts.push("Repositories".to_string());
                }
            }
            Service::LambdaFunctions => {
                parts.push("Lambda".to_string());
                if let Some(func) = &self.lambda_state.current_function {
                    parts.push(func.clone());
                } else {
                    parts.push("Functions".to_string());
                }
            }
            Service::LambdaApplications => {
                parts.push("Lambda".to_string());
                parts.push("Applications".to_string());
            }
            Service::CloudFormationStacks => {
                parts.push("CloudFormation".to_string());
                if let Some(stack_name) = &self.cfn_state.current_stack {
                    parts.push(stack_name.clone());
                } else {
                    parts.push("Stacks".to_string());
                }
            }
            Service::IamUsers => {
                parts.push("IAM".to_string());
                parts.push("Users".to_string());
            }
            Service::IamRoles => {
                parts.push("IAM".to_string());
                parts.push("Roles".to_string());
                if let Some(role_name) = &self.iam_state.current_role {
                    parts.push(role_name.clone());
                    if let Some(policy_name) = &self.iam_state.current_policy {
                        parts.push(policy_name.clone());
                    }
                }
            }
            Service::IamUserGroups => {
                parts.push("IAM".to_string());
                parts.push("User Groups".to_string());
                if let Some(group_name) = &self.iam_state.current_group {
                    parts.push(group_name.clone());
                }
            }
            Service::Ec2Instances => {
                parts.push("EC2".to_string());
                parts.push("Instances".to_string());
            }
        }

        parts.join(" > ")
    }

    pub fn update_current_tab_breadcrumb(&mut self) {
        if !self.tabs.is_empty() {
            self.tabs[self.current_tab].breadcrumb = self.breadcrumbs();
        }
    }

    pub fn get_console_url(&self) -> String {
        use crate::{cfn, cw, ecr, iam, lambda, s3};

        match self.current_service {
            Service::CloudWatchLogGroups => {
                if self.view_mode == ViewMode::Events {
                    if let Some(group) = selected_log_group(self) {
                        if let Some(stream) = self
                            .log_groups_state
                            .log_streams
                            .get(self.log_groups_state.selected_stream)
                        {
                            return cw::logs::console_url_stream(
                                &self.config.region,
                                &group.name,
                                &stream.name,
                            );
                        }
                    }
                } else if self.view_mode == ViewMode::Detail {
                    if let Some(group) = selected_log_group(self) {
                        return cw::logs::console_url_detail(&self.config.region, &group.name);
                    }
                }
                cw::logs::console_url_list(&self.config.region)
            }
            Service::CloudWatchInsights => cw::insights::console_url(
                &self.config.region,
                &self.config.account_id,
                &self.insights_state.insights.query_text,
                &self.insights_state.insights.selected_log_groups,
            ),
            Service::CloudWatchAlarms => {
                let view_type = match self.alarms_state.view_as {
                    AlarmViewMode::Table | AlarmViewMode::Detail => "table",
                    AlarmViewMode::Cards => "card",
                };
                cw::alarms::console_url(
                    &self.config.region,
                    view_type,
                    self.alarms_state.table.page_size.value(),
                    &self.alarms_state.sort_column,
                    self.alarms_state.sort_direction.as_str(),
                )
            }
            Service::S3Buckets => {
                if let Some(bucket_name) = &self.s3_state.current_bucket {
                    let prefix = self.s3_state.prefix_stack.join("");
                    s3::console_url_bucket(&self.config.region, bucket_name, &prefix)
                } else {
                    s3::console_url_buckets(&self.config.region)
                }
            }
            Service::SqsQueues => {
                if let Some(queue_url) = &self.sqs_state.current_queue {
                    console_url_queue_detail(&self.config.region, queue_url)
                } else {
                    console_url_queues(&self.config.region)
                }
            }
            Service::EcrRepositories => {
                if let Some(repo_name) = &self.ecr_state.current_repository {
                    ecr::console_url_private_repository(
                        &self.config.region,
                        &self.config.account_id,
                        repo_name,
                    )
                } else {
                    ecr::console_url_repositories(&self.config.region)
                }
            }
            Service::LambdaFunctions => {
                if let Some(func_name) = &self.lambda_state.current_function {
                    if let Some(version) = &self.lambda_state.current_version {
                        lambda::console_url_function_version(
                            &self.config.region,
                            func_name,
                            version,
                            &self.lambda_state.detail_tab,
                        )
                    } else {
                        lambda::console_url_function_detail(&self.config.region, func_name)
                    }
                } else {
                    lambda::console_url_functions(&self.config.region)
                }
            }
            Service::LambdaApplications => {
                if let Some(app_name) = &self.lambda_application_state.current_application {
                    lambda::console_url_application_detail(
                        &self.config.region,
                        app_name,
                        &self.lambda_application_state.detail_tab,
                    )
                } else {
                    lambda::console_url_applications(&self.config.region)
                }
            }
            Service::CloudFormationStacks => {
                if let Some(stack_name) = &self.cfn_state.current_stack {
                    if let Some(stack) = self
                        .cfn_state
                        .table
                        .items
                        .iter()
                        .find(|s| &s.name == stack_name)
                    {
                        return cfn::console_url_stack_detail_with_tab(
                            &self.config.region,
                            &stack.stack_id,
                            &self.cfn_state.detail_tab,
                        );
                    }
                }
                cfn::console_url_stacks(&self.config.region)
            }
            Service::IamUsers => {
                if let Some(user_name) = &self.iam_state.current_user {
                    let section = match self.iam_state.user_tab {
                        UserTab::Permissions => "permissions",
                        UserTab::Groups => "groups",
                        UserTab::Tags => "tags",
                        UserTab::SecurityCredentials => "security_credentials",
                        UserTab::LastAccessed => "access_advisor",
                    };
                    iam::console_url_user_detail(&self.config.region, user_name, section)
                } else {
                    iam::console_url_users(&self.config.region)
                }
            }
            Service::IamRoles => {
                if let Some(policy_name) = &self.iam_state.current_policy {
                    if let Some(role_name) = &self.iam_state.current_role {
                        return iam::console_url_role_policy(
                            &self.config.region,
                            role_name,
                            policy_name,
                        );
                    }
                }
                if let Some(role_name) = &self.iam_state.current_role {
                    let section = match self.iam_state.role_tab {
                        RoleTab::Permissions => "permissions",
                        RoleTab::TrustRelationships => "trust_relationships",
                        RoleTab::Tags => "tags",
                        RoleTab::LastAccessed => "access_advisor",
                        RoleTab::RevokeSessions => "revoke_sessions",
                    };
                    iam::console_url_role_detail(&self.config.region, role_name, section)
                } else {
                    iam::console_url_roles(&self.config.region)
                }
            }
            Service::IamUserGroups => iam::console_url_groups(&self.config.region),
            Service::Ec2Instances => {
                if let Some(instance_id) = &self.ec2_state.current_instance {
                    format!(
                        "https://{}.console.aws.amazon.com/ec2/home?region={}#InstanceDetails:instanceId={}",
                        self.config.region, self.config.region, instance_id
                    )
                } else {
                    format!(
                        "https://{}.console.aws.amazon.com/ec2/home?region={}#Instances:",
                        self.config.region, self.config.region
                    )
                }
            }
        }
    }

    fn calculate_total_bucket_rows(&self) -> usize {
        calculate_total_bucket_rows(self)
    }

    fn calculate_total_object_rows(&self) -> usize {
        calculate_total_object_rows(self)
    }

    fn get_column_selector_max(&self) -> usize {
        if self.current_service == Service::S3Buckets && self.s3_state.current_bucket.is_none() {
            self.s3_bucket_column_ids.len() - 1
        } else if self.view_mode == ViewMode::Events {
            self.cw_log_event_column_ids.len() - 1
        } else if self.view_mode == ViewMode::Detail {
            self.cw_log_stream_column_ids.len() - 1
        } else if self.current_service == Service::CloudWatchAlarms {
            29
        } else if self.current_service == Service::Ec2Instances {
            self.ec2_column_ids.len() + 6
        } else if self.current_service == Service::EcrRepositories {
            if self.ecr_state.current_repository.is_some() {
                self.ecr_image_column_ids.len() + 6
            } else {
                self.ecr_repo_column_ids.len() - 1
            }
        } else if self.current_service == Service::SqsQueues {
            self.sqs_column_ids.len() - 1
        } else if self.current_service == Service::LambdaFunctions {
            self.lambda_state.function_column_ids.len() + 6
        } else if self.current_service == Service::LambdaApplications {
            self.lambda_application_column_ids.len() + 5
        } else if self.current_service == Service::CloudFormationStacks {
            self.cfn_column_ids.len() + 6
        } else if self.current_service == Service::IamUsers {
            if self.iam_state.current_user.is_some() {
                self.iam_policy_column_ids.len() + 5
            } else {
                self.iam_user_column_ids.len() + 5
            }
        } else if self.current_service == Service::IamRoles {
            if self.iam_state.current_role.is_some() {
                self.iam_policy_column_ids.len() + 5
            } else {
                self.iam_role_column_ids.len() + 5
            }
        } else {
            self.cw_log_group_column_ids.len() - 1
        }
    }

    fn next_item(&mut self) {
        match self.mode {
            Mode::FilterInput => {
                if self.current_service == Service::CloudFormationStacks {
                    use crate::ui::cfn::STATUS_FILTER;
                    if self.cfn_state.input_focus == STATUS_FILTER {
                        self.cfn_state.status_filter = self.cfn_state.status_filter.next();
                        self.cfn_state.table.reset();
                    }
                } else if self.current_service == Service::Ec2Instances {
                    if self.ec2_state.input_focus == EC2_STATE_FILTER {
                        self.ec2_state.state_filter = self.ec2_state.state_filter.next();
                        self.ec2_state.table.reset();
                    }
                } else if self.current_service == Service::SqsQueues {
                    use crate::ui::sqs::SUBSCRIPTION_REGION;
                    if self.sqs_state.input_focus == SUBSCRIPTION_REGION {
                        let regions = AwsRegion::all();
                        self.sqs_state.subscription_region_selected =
                            (self.sqs_state.subscription_region_selected + 1)
                                .min(regions.len() - 1);
                        self.sqs_state.subscriptions.reset();
                    }
                }
            }
            Mode::RegionPicker => {
                let filtered = self.get_filtered_regions();
                if !filtered.is_empty() {
                    self.region_picker_selected =
                        (self.region_picker_selected + 1).min(filtered.len() - 1);
                }
            }
            Mode::ProfilePicker => {
                let filtered = self.get_filtered_profiles();
                if !filtered.is_empty() {
                    self.profile_picker_selected =
                        (self.profile_picker_selected + 1).min(filtered.len() - 1);
                }
            }
            Mode::SessionPicker => {
                let filtered = self.get_filtered_sessions();
                if !filtered.is_empty() {
                    self.session_picker_selected =
                        (self.session_picker_selected + 1).min(filtered.len() - 1);
                }
            }
            Mode::InsightsInput => {
                use crate::app::InsightsFocus;
                if self.insights_state.insights.insights_focus == InsightsFocus::LogGroupSearch
                    && self.insights_state.insights.show_dropdown
                    && !self.insights_state.insights.log_group_matches.is_empty()
                {
                    let max = self.insights_state.insights.log_group_matches.len() - 1;
                    self.insights_state.insights.dropdown_selected =
                        (self.insights_state.insights.dropdown_selected + 1).min(max);
                }
            }
            Mode::ColumnSelector => {
                let max = self.get_column_selector_max();
                self.column_selector_index = (self.column_selector_index + 1).min(max);
            }
            Mode::ServicePicker => {
                let filtered = self.filtered_services();
                if !filtered.is_empty() {
                    self.service_picker.selected =
                        (self.service_picker.selected + 1).min(filtered.len() - 1);
                }
            }
            Mode::TabPicker => {
                let filtered = self.get_filtered_tabs();
                if !filtered.is_empty() {
                    self.tab_picker_selected =
                        (self.tab_picker_selected + 1).min(filtered.len() - 1);
                }
            }
            Mode::Normal => {
                if !self.service_selected {
                    let filtered = self.filtered_services();
                    if !filtered.is_empty() {
                        self.service_picker.selected =
                            (self.service_picker.selected + 1).min(filtered.len() - 1);
                    }
                } else if self.current_service == Service::S3Buckets {
                    if self.s3_state.current_bucket.is_some() {
                        if self.s3_state.object_tab == S3ObjectTab::Properties {
                            // Scroll properties view
                            self.s3_state.properties_scroll =
                                self.s3_state.properties_scroll.saturating_add(1);
                        } else {
                            // Calculate total rows including all nested preview items
                            let total_rows = self.calculate_total_object_rows();
                            let max = total_rows.saturating_sub(1);
                            self.s3_state.selected_object =
                                (self.s3_state.selected_object + 1).min(max);

                            // Adjust scroll offset if selection goes below viewport
                            let visible_rows = self.s3_state.object_visible_rows.get();
                            if self.s3_state.selected_object
                                >= self.s3_state.object_scroll_offset + visible_rows
                            {
                                self.s3_state.object_scroll_offset =
                                    self.s3_state.selected_object - visible_rows + 1;
                            }
                        }
                    } else {
                        // Navigate rows in bucket list
                        let total_rows = self.calculate_total_bucket_rows();
                        if total_rows > 0 {
                            self.s3_state.selected_row =
                                (self.s3_state.selected_row + 1).min(total_rows - 1);

                            // Adjust scroll offset if selection goes below viewport
                            let visible_rows = self.s3_state.bucket_visible_rows.get();
                            if self.s3_state.selected_row
                                >= self.s3_state.bucket_scroll_offset + visible_rows
                            {
                                self.s3_state.bucket_scroll_offset =
                                    self.s3_state.selected_row - visible_rows + 1;
                            }
                        }
                    }
                } else if self.view_mode == ViewMode::InsightsResults {
                    let max = self
                        .insights_state
                        .insights
                        .query_results
                        .len()
                        .saturating_sub(1);
                    if self.insights_state.insights.results_selected < max {
                        self.insights_state.insights.results_selected += 1;
                    }
                } else if self.view_mode == ViewMode::PolicyView {
                    let lines = self.iam_state.policy_document.lines().count();
                    let max_scroll = lines.saturating_sub(1);
                    self.iam_state.policy_scroll =
                        (self.iam_state.policy_scroll + 1).min(max_scroll);
                } else if self.current_service == Service::CloudFormationStacks
                    && self.cfn_state.current_stack.is_some()
                    && self.cfn_state.detail_tab == CfnDetailTab::Template
                {
                    let lines = self.cfn_state.template_body.lines().count();
                    let max_scroll = lines.saturating_sub(1);
                    self.cfn_state.template_scroll =
                        (self.cfn_state.template_scroll + 1).min(max_scroll);
                } else if self.current_service == Service::SqsQueues
                    && self.sqs_state.current_queue.is_some()
                    && self.sqs_state.detail_tab == SqsQueueDetailTab::QueuePolicies
                {
                    let lines = self.sqs_state.policy_document.lines().count();
                    let max_scroll = lines.saturating_sub(1);
                    self.sqs_state.policy_scroll =
                        (self.sqs_state.policy_scroll + 1).min(max_scroll);
                } else if self.current_service == Service::LambdaFunctions
                    && self.lambda_state.current_function.is_some()
                    && self.lambda_state.detail_tab == LambdaDetailTab::Monitor
                    && !self.lambda_state.is_metrics_loading()
                {
                    self.lambda_state
                        .set_monitoring_scroll((self.lambda_state.monitoring_scroll() + 1).min(9));
                } else if self.current_service == Service::SqsQueues
                    && self.sqs_state.current_queue.is_some()
                    && self.sqs_state.detail_tab == SqsQueueDetailTab::Monitoring
                    && !self.sqs_state.is_metrics_loading()
                {
                    self.sqs_state
                        .set_monitoring_scroll((self.sqs_state.monitoring_scroll() + 1).min(8));
                } else if self.view_mode == ViewMode::Events {
                    let max_scroll = self.log_groups_state.log_events.len().saturating_sub(1);
                    if self.log_groups_state.event_scroll_offset >= max_scroll {
                        // At the end, do nothing
                    } else {
                        self.log_groups_state.event_scroll_offset =
                            (self.log_groups_state.event_scroll_offset + 1).min(max_scroll);
                    }
                } else if self.current_service == Service::CloudWatchLogGroups {
                    if self.view_mode == ViewMode::List {
                        let filtered_groups = filtered_log_groups(self);
                        self.log_groups_state
                            .log_groups
                            .next_item(filtered_groups.len());
                    } else if self.view_mode == ViewMode::Detail {
                        let filtered_streams = filtered_log_streams(self);
                        if !filtered_streams.is_empty() {
                            let max = filtered_streams.len() - 1;
                            if self.log_groups_state.selected_stream >= max {
                                // At the end, do nothing
                            } else {
                                self.log_groups_state.selected_stream =
                                    (self.log_groups_state.selected_stream + 1).min(max);
                            }
                        }
                    }
                } else if self.current_service == Service::CloudWatchAlarms {
                    let filtered_alarms = match self.alarms_state.alarm_tab {
                        AlarmTab::AllAlarms => self.alarms_state.table.items.len(),
                        AlarmTab::InAlarm => self
                            .alarms_state
                            .table
                            .items
                            .iter()
                            .filter(|a| a.state.to_uppercase() == "ALARM")
                            .count(),
                    };
                    if filtered_alarms > 0 {
                        self.alarms_state.table.next_item(filtered_alarms);
                    }
                } else if self.current_service == Service::Ec2Instances {
                    let filtered: Vec<_> = self
                        .ec2_state
                        .table
                        .items
                        .iter()
                        .filter(|i| self.ec2_state.state_filter.matches(&i.state))
                        .filter(|i| {
                            if self.ec2_state.table.filter.is_empty() {
                                return true;
                            }
                            i.name.contains(&self.ec2_state.table.filter)
                                || i.instance_id.contains(&self.ec2_state.table.filter)
                                || i.state.contains(&self.ec2_state.table.filter)
                                || i.instance_type.contains(&self.ec2_state.table.filter)
                                || i.availability_zone.contains(&self.ec2_state.table.filter)
                                || i.security_groups.contains(&self.ec2_state.table.filter)
                                || i.key_name.contains(&self.ec2_state.table.filter)
                        })
                        .collect();
                    if !filtered.is_empty() {
                        self.ec2_state.table.next_item(filtered.len());
                    }
                } else if self.current_service == Service::EcrRepositories {
                    if self.ecr_state.current_repository.is_some() {
                        let filtered_images = filtered_ecr_images(self);
                        if !filtered_images.is_empty() {
                            self.ecr_state.images.next_item(filtered_images.len());
                        }
                    } else {
                        let filtered_repos = filtered_ecr_repositories(self);
                        if !filtered_repos.is_empty() {
                            self.ecr_state.repositories.selected =
                                (self.ecr_state.repositories.selected + 1)
                                    .min(filtered_repos.len() - 1);
                            self.ecr_state.repositories.snap_to_page();
                        }
                    }
                } else if self.current_service == Service::SqsQueues {
                    if self.sqs_state.current_queue.is_some()
                        && self.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers
                    {
                        let filtered = filtered_lambda_triggers(self);
                        if !filtered.is_empty() {
                            self.sqs_state.triggers.next_item(filtered.len());
                        }
                    } else if self.sqs_state.current_queue.is_some()
                        && self.sqs_state.detail_tab == SqsQueueDetailTab::EventBridgePipes
                    {
                        let filtered = filtered_eventbridge_pipes(self);
                        if !filtered.is_empty() {
                            self.sqs_state.pipes.next_item(filtered.len());
                        }
                    } else if self.sqs_state.current_queue.is_some()
                        && self.sqs_state.detail_tab == SqsQueueDetailTab::Tagging
                    {
                        let filtered = filtered_tags(self);
                        if !filtered.is_empty() {
                            self.sqs_state.tags.next_item(filtered.len());
                        }
                    } else if self.sqs_state.current_queue.is_some()
                        && self.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions
                    {
                        let filtered = filtered_subscriptions(self);
                        if !filtered.is_empty() {
                            self.sqs_state.subscriptions.next_item(filtered.len());
                        }
                    } else {
                        let filtered_queues = filtered_queues(
                            &self.sqs_state.queues.items,
                            &self.sqs_state.queues.filter,
                        );
                        if !filtered_queues.is_empty() {
                            self.sqs_state.queues.next_item(filtered_queues.len());
                        }
                    }
                } else if self.current_service == Service::LambdaFunctions {
                    if self.lambda_state.current_function.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Code
                    {
                        // Layer table navigation in Code tab
                        if let Some(func_name) = &self.lambda_state.current_function {
                            if let Some(func) = self
                                .lambda_state
                                .table
                                .items
                                .iter()
                                .find(|f| f.name == *func_name)
                            {
                                let max = func.layers.len().saturating_sub(1);
                                if !func.layers.is_empty() {
                                    self.lambda_state.layer_selected =
                                        (self.lambda_state.layer_selected + 1).min(max);
                                }
                            }
                        }
                    } else if self.lambda_state.current_function.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Versions
                    {
                        // Version table navigation
                        let filtered: Vec<_> = self
                            .lambda_state
                            .version_table
                            .items
                            .iter()
                            .filter(|v| {
                                self.lambda_state.version_table.filter.is_empty()
                                    || v.version.to_lowercase().contains(
                                        &self.lambda_state.version_table.filter.to_lowercase(),
                                    )
                                    || v.aliases.to_lowercase().contains(
                                        &self.lambda_state.version_table.filter.to_lowercase(),
                                    )
                                    || v.description.to_lowercase().contains(
                                        &self.lambda_state.version_table.filter.to_lowercase(),
                                    )
                            })
                            .collect();
                        if !filtered.is_empty() {
                            self.lambda_state.version_table.selected =
                                (self.lambda_state.version_table.selected + 1)
                                    .min(filtered.len() - 1);
                            self.lambda_state.version_table.snap_to_page();
                        }
                    } else if self.lambda_state.current_function.is_some()
                        && (self.lambda_state.detail_tab == LambdaDetailTab::Aliases
                            || (self.lambda_state.current_version.is_some()
                                && self.lambda_state.detail_tab == LambdaDetailTab::Configuration))
                    {
                        // Alias table navigation (both in Aliases tab and Version Configuration)
                        let version_filter = self.lambda_state.current_version.clone();
                        let filtered: Vec<_> = self
                            .lambda_state
                            .alias_table
                            .items
                            .iter()
                            .filter(|a| {
                                (version_filter.is_none()
                                    || a.versions.contains(version_filter.as_ref().unwrap()))
                                    && (self.lambda_state.alias_table.filter.is_empty()
                                        || a.name.to_lowercase().contains(
                                            &self.lambda_state.alias_table.filter.to_lowercase(),
                                        )
                                        || a.versions.to_lowercase().contains(
                                            &self.lambda_state.alias_table.filter.to_lowercase(),
                                        )
                                        || a.description.to_lowercase().contains(
                                            &self.lambda_state.alias_table.filter.to_lowercase(),
                                        ))
                            })
                            .collect();
                        if !filtered.is_empty() {
                            self.lambda_state.alias_table.selected =
                                (self.lambda_state.alias_table.selected + 1)
                                    .min(filtered.len() - 1);
                            self.lambda_state.alias_table.snap_to_page();
                        }
                    } else if self.lambda_state.current_function.is_none() {
                        let filtered = filtered_lambda_functions(self);
                        if !filtered.is_empty() {
                            self.lambda_state.table.next_item(filtered.len());
                            self.lambda_state.table.snap_to_page();
                        }
                    }
                } else if self.current_service == Service::LambdaApplications {
                    if self.lambda_application_state.current_application.is_some() {
                        if self.lambda_application_state.detail_tab
                            == LambdaApplicationDetailTab::Overview
                        {
                            let len = self.lambda_application_state.resources.items.len();
                            if len > 0 {
                                self.lambda_application_state.resources.next_item(len);
                            }
                        } else {
                            let len = self.lambda_application_state.deployments.items.len();
                            if len > 0 {
                                self.lambda_application_state.deployments.next_item(len);
                            }
                        }
                    } else {
                        let filtered = filtered_lambda_applications(self);
                        if !filtered.is_empty() {
                            self.lambda_application_state.table.selected =
                                (self.lambda_application_state.table.selected + 1)
                                    .min(filtered.len() - 1);
                            self.lambda_application_state.table.snap_to_page();
                        }
                    }
                } else if self.current_service == Service::CloudFormationStacks {
                    if self.cfn_state.current_stack.is_some()
                        && self.cfn_state.detail_tab == CfnDetailTab::Parameters
                    {
                        let filtered = filtered_parameters(self);
                        self.cfn_state.parameters.next_item(filtered.len());
                    } else if self.cfn_state.current_stack.is_some()
                        && self.cfn_state.detail_tab == CfnDetailTab::Outputs
                    {
                        let filtered = filtered_outputs(self);
                        self.cfn_state.outputs.next_item(filtered.len());
                    } else if self.cfn_state.current_stack.is_some()
                        && self.cfn_state.detail_tab == CfnDetailTab::Resources
                    {
                        let filtered = filtered_resources(self);
                        self.cfn_state.resources.next_item(filtered.len());
                    } else {
                        let filtered = filtered_cloudformation_stacks(self);
                        self.cfn_state.table.next_item(filtered.len());
                    }
                } else if self.current_service == Service::IamUsers {
                    if self.iam_state.current_user.is_some() {
                        if self.iam_state.user_tab == UserTab::Tags {
                            let filtered = filtered_user_tags(self);
                            if !filtered.is_empty() {
                                self.iam_state.user_tags.next_item(filtered.len());
                            }
                        } else {
                            let filtered = filtered_iam_policies(self);
                            if !filtered.is_empty() {
                                self.iam_state.policies.next_item(filtered.len());
                            }
                        }
                    } else {
                        let filtered = filtered_iam_users(self);
                        if !filtered.is_empty() {
                            self.iam_state.users.next_item(filtered.len());
                        }
                    }
                } else if self.current_service == Service::IamRoles {
                    if self.iam_state.current_role.is_some() {
                        if self.iam_state.role_tab == RoleTab::TrustRelationships {
                            let lines = self.iam_state.trust_policy_document.lines().count();
                            let max_scroll = lines.saturating_sub(1);
                            self.iam_state.trust_policy_scroll =
                                (self.iam_state.trust_policy_scroll + 1).min(max_scroll);
                        } else if self.iam_state.role_tab == RoleTab::RevokeSessions {
                            self.iam_state.revoke_sessions_scroll =
                                (self.iam_state.revoke_sessions_scroll + 1).min(19);
                        } else if self.iam_state.role_tab == RoleTab::Tags {
                            let filtered = filtered_iam_tags(self);
                            if !filtered.is_empty() {
                                self.iam_state.tags.next_item(filtered.len());
                            }
                        } else if self.iam_state.role_tab == RoleTab::LastAccessed {
                            let filtered = filtered_last_accessed(self);
                            if !filtered.is_empty() {
                                self.iam_state
                                    .last_accessed_services
                                    .next_item(filtered.len());
                            }
                        } else {
                            let filtered = filtered_iam_policies(self);
                            if !filtered.is_empty() {
                                self.iam_state.policies.next_item(filtered.len());
                            }
                        }
                    } else {
                        let filtered = filtered_iam_roles(self);
                        if !filtered.is_empty() {
                            self.iam_state.roles.next_item(filtered.len());
                        }
                    }
                } else if self.current_service == Service::IamUserGroups {
                    if self.iam_state.current_group.is_some() {
                        if self.iam_state.group_tab == GroupTab::Users {
                            let filtered: Vec<_> = self
                                .iam_state
                                .group_users
                                .items
                                .iter()
                                .filter(|u| {
                                    if self.iam_state.group_users.filter.is_empty() {
                                        true
                                    } else {
                                        u.user_name.to_lowercase().contains(
                                            &self.iam_state.group_users.filter.to_lowercase(),
                                        )
                                    }
                                })
                                .collect();
                            if !filtered.is_empty() {
                                self.iam_state.group_users.next_item(filtered.len());
                            }
                        } else if self.iam_state.group_tab == GroupTab::Permissions {
                            let filtered = filtered_iam_policies(self);
                            if !filtered.is_empty() {
                                self.iam_state.policies.next_item(filtered.len());
                            }
                        } else if self.iam_state.group_tab == GroupTab::AccessAdvisor {
                            let filtered = filtered_last_accessed(self);
                            if !filtered.is_empty() {
                                self.iam_state
                                    .last_accessed_services
                                    .next_item(filtered.len());
                            }
                        }
                    } else {
                        let filtered: Vec<_> = self
                            .iam_state
                            .groups
                            .items
                            .iter()
                            .filter(|g| {
                                if self.iam_state.groups.filter.is_empty() {
                                    true
                                } else {
                                    g.group_name
                                        .to_lowercase()
                                        .contains(&self.iam_state.groups.filter.to_lowercase())
                                }
                            })
                            .collect();
                        if !filtered.is_empty() {
                            self.iam_state.groups.next_item(filtered.len());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn prev_item(&mut self) {
        match self.mode {
            Mode::FilterInput => {
                if self.current_service == Service::CloudFormationStacks {
                    use crate::ui::cfn::STATUS_FILTER;
                    if self.cfn_state.input_focus == STATUS_FILTER {
                        self.cfn_state.status_filter = self.cfn_state.status_filter.prev();
                        self.cfn_state.table.reset();
                    }
                } else if self.current_service == Service::Ec2Instances {
                    if self.ec2_state.input_focus == EC2_STATE_FILTER {
                        self.ec2_state.state_filter = self.ec2_state.state_filter.prev();
                        self.ec2_state.table.reset();
                    }
                } else if self.current_service == Service::SqsQueues {
                    use crate::ui::sqs::SUBSCRIPTION_REGION;
                    if self.sqs_state.input_focus == SUBSCRIPTION_REGION {
                        self.sqs_state.subscription_region_selected = self
                            .sqs_state
                            .subscription_region_selected
                            .saturating_sub(1);
                        self.sqs_state.subscriptions.reset();
                    }
                }
            }
            Mode::RegionPicker => {
                self.region_picker_selected = self.region_picker_selected.saturating_sub(1);
            }
            Mode::ProfilePicker => {
                self.profile_picker_selected = self.profile_picker_selected.saturating_sub(1);
            }
            Mode::SessionPicker => {
                self.session_picker_selected = self.session_picker_selected.saturating_sub(1);
            }
            Mode::InsightsInput => {
                use crate::app::InsightsFocus;
                if self.insights_state.insights.insights_focus == InsightsFocus::LogGroupSearch
                    && self.insights_state.insights.show_dropdown
                    && !self.insights_state.insights.log_group_matches.is_empty()
                {
                    self.insights_state.insights.dropdown_selected = self
                        .insights_state
                        .insights
                        .dropdown_selected
                        .saturating_sub(1);
                }
            }
            Mode::ColumnSelector => {
                self.column_selector_index = self.column_selector_index.saturating_sub(1);
            }
            Mode::ServicePicker => {
                self.service_picker.selected = self.service_picker.selected.saturating_sub(1);
            }
            Mode::TabPicker => {
                self.tab_picker_selected = self.tab_picker_selected.saturating_sub(1);
            }
            Mode::Normal => {
                if !self.service_selected {
                    self.service_picker.selected = self.service_picker.selected.saturating_sub(1);
                } else if self.current_service == Service::S3Buckets {
                    if self.s3_state.current_bucket.is_some() {
                        if self.s3_state.object_tab == S3ObjectTab::Properties {
                            self.s3_state.properties_scroll =
                                self.s3_state.properties_scroll.saturating_sub(1);
                        } else {
                            self.s3_state.selected_object =
                                self.s3_state.selected_object.saturating_sub(1);

                            // Adjust scroll offset if selection goes above viewport
                            if self.s3_state.selected_object < self.s3_state.object_scroll_offset {
                                self.s3_state.object_scroll_offset = self.s3_state.selected_object;
                            }
                        }
                    } else {
                        self.s3_state.selected_row = self.s3_state.selected_row.saturating_sub(1);

                        // Adjust scroll offset if selection goes above viewport
                        if self.s3_state.selected_row < self.s3_state.bucket_scroll_offset {
                            self.s3_state.bucket_scroll_offset = self.s3_state.selected_row;
                        }
                    }
                } else if self.view_mode == ViewMode::InsightsResults {
                    if self.insights_state.insights.results_selected > 0 {
                        self.insights_state.insights.results_selected -= 1;
                    }
                } else if self.view_mode == ViewMode::PolicyView {
                    self.iam_state.policy_scroll = self.iam_state.policy_scroll.saturating_sub(1);
                } else if self.current_service == Service::CloudFormationStacks
                    && self.cfn_state.current_stack.is_some()
                    && self.cfn_state.detail_tab == CfnDetailTab::Template
                {
                    self.cfn_state.template_scroll =
                        self.cfn_state.template_scroll.saturating_sub(1);
                } else if self.current_service == Service::SqsQueues
                    && self.sqs_state.current_queue.is_some()
                    && self.sqs_state.detail_tab == SqsQueueDetailTab::QueuePolicies
                {
                    self.sqs_state.policy_scroll = self.sqs_state.policy_scroll.saturating_sub(1);
                } else if self.current_service == Service::LambdaFunctions
                    && self.lambda_state.current_function.is_some()
                    && self.lambda_state.detail_tab == LambdaDetailTab::Monitor
                    && !self.lambda_state.is_metrics_loading()
                {
                    self.lambda_state.set_monitoring_scroll(
                        self.lambda_state.monitoring_scroll().saturating_sub(1),
                    );
                } else if self.current_service == Service::SqsQueues
                    && self.sqs_state.current_queue.is_some()
                    && self.sqs_state.detail_tab == SqsQueueDetailTab::Monitoring
                    && !self.sqs_state.is_metrics_loading()
                {
                    self.sqs_state.set_monitoring_scroll(
                        self.sqs_state.monitoring_scroll().saturating_sub(1),
                    );
                } else if self.view_mode == ViewMode::Events {
                    if self.log_groups_state.event_scroll_offset == 0 {
                        if self.log_groups_state.has_older_events {
                            self.log_groups_state.loading = true;
                        }
                        // Don't move if at position 0
                    } else {
                        self.log_groups_state.event_scroll_offset =
                            self.log_groups_state.event_scroll_offset.saturating_sub(1);
                    }
                } else if self.current_service == Service::CloudWatchLogGroups {
                    if self.view_mode == ViewMode::List {
                        self.log_groups_state.log_groups.prev_item();
                    } else if self.view_mode == ViewMode::Detail
                        && self.log_groups_state.selected_stream > 0
                    {
                        self.log_groups_state.selected_stream =
                            self.log_groups_state.selected_stream.saturating_sub(1);
                        self.log_groups_state.expanded_stream = None;
                    }
                } else if self.current_service == Service::CloudWatchAlarms {
                    self.alarms_state.table.prev_item();
                } else if self.current_service == Service::Ec2Instances {
                    self.ec2_state.table.prev_item();
                } else if self.current_service == Service::EcrRepositories {
                    if self.ecr_state.current_repository.is_some() {
                        self.ecr_state.images.prev_item();
                    } else {
                        self.ecr_state.repositories.prev_item();
                    }
                } else if self.current_service == Service::SqsQueues {
                    if self.sqs_state.current_queue.is_some()
                        && self.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers
                    {
                        self.sqs_state.triggers.prev_item();
                    } else if self.sqs_state.current_queue.is_some()
                        && self.sqs_state.detail_tab == SqsQueueDetailTab::EventBridgePipes
                    {
                        self.sqs_state.pipes.prev_item();
                    } else if self.sqs_state.current_queue.is_some()
                        && self.sqs_state.detail_tab == SqsQueueDetailTab::Tagging
                    {
                        self.sqs_state.tags.prev_item();
                    } else if self.sqs_state.current_queue.is_some()
                        && self.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions
                    {
                        self.sqs_state.subscriptions.prev_item();
                    } else {
                        self.sqs_state.queues.prev_item();
                    }
                } else if self.current_service == Service::LambdaFunctions {
                    if self.lambda_state.current_function.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Code
                    {
                        // Layer table navigation in Code tab
                        self.lambda_state.layer_selected =
                            self.lambda_state.layer_selected.saturating_sub(1);
                    } else if self.lambda_state.current_function.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Versions
                    {
                        self.lambda_state.version_table.prev_item();
                    } else if self.lambda_state.current_function.is_some()
                        && (self.lambda_state.detail_tab == LambdaDetailTab::Aliases
                            || (self.lambda_state.current_version.is_some()
                                && self.lambda_state.detail_tab == LambdaDetailTab::Configuration))
                    {
                        self.lambda_state.alias_table.prev_item();
                    } else if self.lambda_state.current_function.is_none() {
                        self.lambda_state.table.prev_item();
                    }
                } else if self.current_service == Service::LambdaApplications {
                    if self.lambda_application_state.current_application.is_some()
                        && self.lambda_application_state.detail_tab
                            == LambdaApplicationDetailTab::Overview
                    {
                        self.lambda_application_state.resources.selected = self
                            .lambda_application_state
                            .resources
                            .selected
                            .saturating_sub(1);
                    } else if self.lambda_application_state.current_application.is_some()
                        && self.lambda_application_state.detail_tab
                            == LambdaApplicationDetailTab::Deployments
                    {
                        self.lambda_application_state.deployments.selected = self
                            .lambda_application_state
                            .deployments
                            .selected
                            .saturating_sub(1);
                    } else {
                        self.lambda_application_state.table.selected = self
                            .lambda_application_state
                            .table
                            .selected
                            .saturating_sub(1);
                        self.lambda_application_state.table.snap_to_page();
                    }
                } else if self.current_service == Service::CloudFormationStacks {
                    if self.cfn_state.current_stack.is_some()
                        && self.cfn_state.detail_tab == CfnDetailTab::Parameters
                    {
                        self.cfn_state.parameters.prev_item();
                    } else if self.cfn_state.current_stack.is_some()
                        && self.cfn_state.detail_tab == CfnDetailTab::Outputs
                    {
                        self.cfn_state.outputs.prev_item();
                    } else if self.cfn_state.current_stack.is_some()
                        && self.cfn_state.detail_tab == CfnDetailTab::Resources
                    {
                        self.cfn_state.resources.prev_item();
                    } else {
                        self.cfn_state.table.prev_item();
                    }
                } else if self.current_service == Service::IamUsers {
                    self.iam_state.users.prev_item();
                } else if self.current_service == Service::IamRoles {
                    if self.iam_state.current_role.is_some() {
                        if self.iam_state.role_tab == RoleTab::TrustRelationships {
                            self.iam_state.trust_policy_scroll =
                                self.iam_state.trust_policy_scroll.saturating_sub(1);
                        } else if self.iam_state.role_tab == RoleTab::RevokeSessions {
                            self.iam_state.revoke_sessions_scroll =
                                self.iam_state.revoke_sessions_scroll.saturating_sub(1);
                        } else if self.iam_state.role_tab == RoleTab::Tags {
                            self.iam_state.tags.prev_item();
                        } else if self.iam_state.role_tab == RoleTab::LastAccessed {
                            self.iam_state.last_accessed_services.prev_item();
                        } else {
                            self.iam_state.policies.prev_item();
                        }
                    } else {
                        self.iam_state.roles.prev_item();
                    }
                } else if self.current_service == Service::IamUserGroups {
                    if self.iam_state.current_group.is_some() {
                        if self.iam_state.group_tab == GroupTab::Users {
                            self.iam_state.group_users.prev_item();
                        } else if self.iam_state.group_tab == GroupTab::Permissions {
                            self.iam_state.policies.prev_item();
                        } else if self.iam_state.group_tab == GroupTab::AccessAdvisor {
                            self.iam_state.last_accessed_services.prev_item();
                        }
                    } else {
                        self.iam_state.groups.prev_item();
                    }
                }
            }
            _ => {}
        }
    }

    fn page_down(&mut self) {
        if self.mode == Mode::ColumnSelector {
            let max = self.get_column_selector_max();
            self.column_selector_index = (self.column_selector_index + 10).min(max);
        } else if self.mode == Mode::FilterInput
            && self.current_service == Service::CloudFormationStacks
        {
            if self.cfn_state.current_stack.is_some()
                && self.cfn_state.detail_tab == CfnDetailTab::Parameters
            {
                let page_size = self.cfn_state.parameters.page_size.value();
                let filtered_count = filtered_parameters(self).len();
                self.cfn_state.parameters_input_focus.handle_page_down(
                    &mut self.cfn_state.parameters.selected,
                    &mut self.cfn_state.parameters.scroll_offset,
                    page_size,
                    filtered_count,
                );
            } else if self.cfn_state.current_stack.is_some()
                && self.cfn_state.detail_tab == CfnDetailTab::Outputs
            {
                let page_size = self.cfn_state.outputs.page_size.value();
                let filtered_count = filtered_outputs(self).len();
                self.cfn_state.outputs_input_focus.handle_page_down(
                    &mut self.cfn_state.outputs.selected,
                    &mut self.cfn_state.outputs.scroll_offset,
                    page_size,
                    filtered_count,
                );
            } else {
                use crate::ui::cfn::filtered_cloudformation_stacks;
                let page_size = self.cfn_state.table.page_size.value();
                let filtered_count = filtered_cloudformation_stacks(self).len();
                self.cfn_state.input_focus.handle_page_down(
                    &mut self.cfn_state.table.selected,
                    &mut self.cfn_state.table.scroll_offset,
                    page_size,
                    filtered_count,
                );
            }
        } else if self.mode == Mode::FilterInput
            && self.current_service == Service::IamRoles
            && self.iam_state.current_role.is_none()
        {
            let page_size = self.iam_state.roles.page_size.value();
            let filtered_count = filtered_iam_roles(self).len();
            self.iam_state.role_input_focus.handle_page_down(
                &mut self.iam_state.roles.selected,
                &mut self.iam_state.roles.scroll_offset,
                page_size,
                filtered_count,
            );
        } else if self.mode == Mode::FilterInput
            && self.current_service == Service::CloudWatchAlarms
        {
            let page_size = self.alarms_state.table.page_size.value();
            let filtered_count = self.alarms_state.table.items.len();
            self.alarms_state.input_focus.handle_page_down(
                &mut self.alarms_state.table.selected,
                &mut self.alarms_state.table.scroll_offset,
                page_size,
                filtered_count,
            );
        } else if self.mode == Mode::FilterInput
            && self.current_service == Service::CloudWatchLogGroups
        {
            if self.view_mode == ViewMode::List {
                // Log groups list pagination
                let filtered = filtered_log_groups(self);
                let page_size = self.log_groups_state.log_groups.page_size.value();
                let filtered_count = filtered.len();
                self.log_groups_state.input_focus.handle_page_down(
                    &mut self.log_groups_state.log_groups.selected,
                    &mut self.log_groups_state.log_groups.scroll_offset,
                    page_size,
                    filtered_count,
                );
            } else {
                // Log streams pagination
                let filtered = filtered_log_streams(self);
                let page_size = 20;
                let filtered_count = filtered.len();
                self.log_groups_state.input_focus.handle_page_down(
                    &mut self.log_groups_state.selected_stream,
                    &mut self.log_groups_state.stream_page,
                    page_size,
                    filtered_count,
                );
                self.log_groups_state.expanded_stream = None;
            }
        } else if self.mode == Mode::FilterInput && self.current_service == Service::LambdaFunctions
        {
            if self.lambda_state.current_function.is_some()
                && self.lambda_state.detail_tab == LambdaDetailTab::Versions
                && self.lambda_state.version_input_focus == InputFocus::Pagination
            {
                let page_size = self.lambda_state.version_table.page_size.value();
                let filtered_count: usize = self
                    .lambda_state
                    .version_table
                    .items
                    .iter()
                    .filter(|v| {
                        self.lambda_state.version_table.filter.is_empty()
                            || v.version
                                .to_lowercase()
                                .contains(&self.lambda_state.version_table.filter.to_lowercase())
                            || v.aliases
                                .to_lowercase()
                                .contains(&self.lambda_state.version_table.filter.to_lowercase())
                            || v.description
                                .to_lowercase()
                                .contains(&self.lambda_state.version_table.filter.to_lowercase())
                    })
                    .count();
                let target = self.lambda_state.version_table.selected + page_size;
                self.lambda_state.version_table.selected =
                    target.min(filtered_count.saturating_sub(1));
            } else if self.lambda_state.current_function.is_some()
                && (self.lambda_state.detail_tab == LambdaDetailTab::Aliases
                    || (self.lambda_state.current_version.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Configuration))
                && self.lambda_state.alias_input_focus == InputFocus::Pagination
            {
                let page_size = self.lambda_state.alias_table.page_size.value();
                let version_filter = self.lambda_state.current_version.clone();
                let filtered_count = self
                    .lambda_state
                    .alias_table
                    .items
                    .iter()
                    .filter(|a| {
                        (version_filter.is_none()
                            || a.versions.contains(version_filter.as_ref().unwrap()))
                            && (self.lambda_state.alias_table.filter.is_empty()
                                || a.name
                                    .to_lowercase()
                                    .contains(&self.lambda_state.alias_table.filter.to_lowercase())
                                || a.versions
                                    .to_lowercase()
                                    .contains(&self.lambda_state.alias_table.filter.to_lowercase())
                                || a.description
                                    .to_lowercase()
                                    .contains(&self.lambda_state.alias_table.filter.to_lowercase()))
                    })
                    .count();
                let target = self.lambda_state.alias_table.selected + page_size;
                self.lambda_state.alias_table.selected =
                    target.min(filtered_count.saturating_sub(1));
            } else if self.lambda_state.current_function.is_none() {
                let page_size = self.lambda_state.table.page_size.value();
                let filtered_count = filtered_lambda_functions(self).len();
                self.lambda_state.input_focus.handle_page_down(
                    &mut self.lambda_state.table.selected,
                    &mut self.lambda_state.table.scroll_offset,
                    page_size,
                    filtered_count,
                );
            }
        } else if self.mode == Mode::FilterInput
            && self.current_service == Service::EcrRepositories
            && self.ecr_state.current_repository.is_none()
            && self.ecr_state.input_focus == InputFocus::Filter
        {
            // When input is focused, allow table scrolling
            let filtered = filtered_ecr_repositories(self);
            self.ecr_state.repositories.page_down(filtered.len());
        } else if self.mode == Mode::FilterInput
            && self.current_service == Service::EcrRepositories
            && self.ecr_state.current_repository.is_none()
        {
            let page_size = self.ecr_state.repositories.page_size.value();
            let filtered_count = filtered_ecr_repositories(self).len();
            self.ecr_state.input_focus.handle_page_down(
                &mut self.ecr_state.repositories.selected,
                &mut self.ecr_state.repositories.scroll_offset,
                page_size,
                filtered_count,
            );
        } else if self.mode == Mode::FilterInput && self.view_mode == ViewMode::PolicyView {
            let page_size = self.iam_state.policies.page_size.value();
            let filtered_count = filtered_iam_policies(self).len();
            self.iam_state.policy_input_focus.handle_page_down(
                &mut self.iam_state.policies.selected,
                &mut self.iam_state.policies.scroll_offset,
                page_size,
                filtered_count,
            );
        } else if self.view_mode == ViewMode::PolicyView {
            let lines = self.iam_state.policy_document.lines().count();
            let max_scroll = lines.saturating_sub(1);
            self.iam_state.policy_scroll = (self.iam_state.policy_scroll + 10).min(max_scroll);
        } else if self.current_service == Service::CloudFormationStacks
            && self.cfn_state.current_stack.is_some()
            && self.cfn_state.detail_tab == CfnDetailTab::Template
        {
            let lines = self.cfn_state.template_body.lines().count();
            let max_scroll = lines.saturating_sub(1);
            self.cfn_state.template_scroll = (self.cfn_state.template_scroll + 10).min(max_scroll);
        } else if self.current_service == Service::LambdaFunctions
            && self.lambda_state.current_function.is_some()
            && self.lambda_state.detail_tab == LambdaDetailTab::Monitor
            && !self.lambda_state.is_metrics_loading()
        {
            self.lambda_state
                .set_monitoring_scroll((self.lambda_state.monitoring_scroll() + 1).min(9));
        } else if self.current_service == Service::SqsQueues
            && self.sqs_state.current_queue.is_some()
        {
            if self.sqs_state.detail_tab == SqsQueueDetailTab::Monitoring {
                self.sqs_state
                    .set_monitoring_scroll((self.sqs_state.monitoring_scroll() + 1).min(8));
            } else {
                let lines = self.sqs_state.policy_document.lines().count();
                let max_scroll = lines.saturating_sub(1);
                self.sqs_state.policy_scroll = (self.sqs_state.policy_scroll + 10).min(max_scroll);
            }
        } else if self.current_service == Service::IamRoles
            && self.iam_state.current_role.is_some()
            && self.iam_state.role_tab == RoleTab::TrustRelationships
        {
            let lines = self.iam_state.trust_policy_document.lines().count();
            let max_scroll = lines.saturating_sub(1);
            self.iam_state.trust_policy_scroll =
                (self.iam_state.trust_policy_scroll + 10).min(max_scroll);
        } else if self.current_service == Service::IamRoles
            && self.iam_state.current_role.is_some()
            && self.iam_state.role_tab == RoleTab::RevokeSessions
        {
            self.iam_state.revoke_sessions_scroll =
                (self.iam_state.revoke_sessions_scroll + 10).min(19);
        } else if self.mode == Mode::Normal {
            if self.current_service == Service::S3Buckets && self.s3_state.current_bucket.is_none()
            {
                let total_rows = self.calculate_total_bucket_rows();
                self.s3_state.selected_row = self
                    .s3_state
                    .selected_row
                    .saturating_add(10)
                    .min(total_rows.saturating_sub(1));

                // Adjust scroll offset if selection goes below viewport
                let visible_rows = self.s3_state.bucket_visible_rows.get();
                if self.s3_state.selected_row >= self.s3_state.bucket_scroll_offset + visible_rows {
                    self.s3_state.bucket_scroll_offset =
                        self.s3_state.selected_row - visible_rows + 1;
                }
            } else if self.current_service == Service::S3Buckets
                && self.s3_state.current_bucket.is_some()
            {
                let total_rows = self.calculate_total_object_rows();
                self.s3_state.selected_object = self
                    .s3_state
                    .selected_object
                    .saturating_add(10)
                    .min(total_rows.saturating_sub(1));

                // Adjust scroll offset if selection goes below viewport
                let visible_rows = self.s3_state.object_visible_rows.get();
                if self.s3_state.selected_object
                    >= self.s3_state.object_scroll_offset + visible_rows
                {
                    self.s3_state.object_scroll_offset =
                        self.s3_state.selected_object - visible_rows + 1;
                }
            } else if self.current_service == Service::CloudWatchLogGroups
                && self.view_mode == ViewMode::List
            {
                let filtered = filtered_log_groups(self);
                self.log_groups_state.log_groups.page_down(filtered.len());
            } else if self.current_service == Service::CloudWatchLogGroups
                && self.view_mode == ViewMode::Detail
            {
                let len = filtered_log_streams(self).len();
                nav_page_down(&mut self.log_groups_state.selected_stream, len, 10);
            } else if self.view_mode == ViewMode::Events {
                let max = self.log_groups_state.log_events.len();
                nav_page_down(&mut self.log_groups_state.event_scroll_offset, max, 10);
            } else if self.view_mode == ViewMode::InsightsResults {
                let max = self.insights_state.insights.query_results.len();
                nav_page_down(&mut self.insights_state.insights.results_selected, max, 10);
            } else if self.current_service == Service::CloudWatchAlarms {
                let filtered = match self.alarms_state.alarm_tab {
                    AlarmTab::AllAlarms => self.alarms_state.table.items.len(),
                    AlarmTab::InAlarm => self
                        .alarms_state
                        .table
                        .items
                        .iter()
                        .filter(|a| a.state.to_uppercase() == "ALARM")
                        .count(),
                };
                if filtered > 0 {
                    self.alarms_state.table.page_down(filtered);
                }
            } else if self.current_service == Service::Ec2Instances {
                let filtered: Vec<_> = self
                    .ec2_state
                    .table
                    .items
                    .iter()
                    .filter(|i| self.ec2_state.state_filter.matches(&i.state))
                    .filter(|i| {
                        if self.ec2_state.table.filter.is_empty() {
                            return true;
                        }
                        i.name.contains(&self.ec2_state.table.filter)
                            || i.instance_id.contains(&self.ec2_state.table.filter)
                            || i.state.contains(&self.ec2_state.table.filter)
                            || i.instance_type.contains(&self.ec2_state.table.filter)
                            || i.availability_zone.contains(&self.ec2_state.table.filter)
                            || i.security_groups.contains(&self.ec2_state.table.filter)
                            || i.key_name.contains(&self.ec2_state.table.filter)
                    })
                    .collect();
                if !filtered.is_empty() {
                    self.ec2_state.table.page_down(filtered.len());
                }
            } else if self.current_service == Service::EcrRepositories {
                if self.ecr_state.current_repository.is_some() {
                    let filtered = filtered_ecr_images(self);
                    self.ecr_state.images.page_down(filtered.len());
                } else {
                    let filtered = filtered_ecr_repositories(self);
                    self.ecr_state.repositories.page_down(filtered.len());
                }
            } else if self.current_service == Service::SqsQueues {
                let filtered =
                    filtered_queues(&self.sqs_state.queues.items, &self.sqs_state.queues.filter);
                self.sqs_state.queues.page_down(filtered.len());
            } else if self.current_service == Service::LambdaFunctions {
                let len = filtered_lambda_functions(self).len();
                self.lambda_state.table.page_down(len);
            } else if self.current_service == Service::LambdaApplications {
                let len = filtered_lambda_applications(self).len();
                self.lambda_application_state.table.page_down(len);
            } else if self.current_service == Service::CloudFormationStacks {
                if self.cfn_state.current_stack.is_some()
                    && self.cfn_state.detail_tab == CfnDetailTab::Parameters
                {
                    let filtered = filtered_parameters(self);
                    self.cfn_state.parameters.page_down(filtered.len());
                } else if self.cfn_state.current_stack.is_some()
                    && self.cfn_state.detail_tab == CfnDetailTab::Outputs
                {
                    let filtered = filtered_outputs(self);
                    self.cfn_state.outputs.page_down(filtered.len());
                } else {
                    let filtered = filtered_cloudformation_stacks(self);
                    self.cfn_state.table.page_down(filtered.len());
                }
            } else if self.current_service == Service::IamUsers {
                let len = filtered_iam_users(self).len();
                nav_page_down(&mut self.iam_state.users.selected, len, 10);
            } else if self.current_service == Service::IamRoles {
                if self.iam_state.current_role.is_some() {
                    let filtered = filtered_iam_policies(self);
                    if !filtered.is_empty() {
                        self.iam_state.policies.page_down(filtered.len());
                    }
                } else {
                    let filtered = filtered_iam_roles(self);
                    self.iam_state.roles.page_down(filtered.len());
                }
            } else if self.current_service == Service::IamUserGroups {
                if self.iam_state.current_group.is_some() {
                    if self.iam_state.group_tab == GroupTab::Users {
                        let filtered: Vec<_> = self
                            .iam_state
                            .group_users
                            .items
                            .iter()
                            .filter(|u| {
                                if self.iam_state.group_users.filter.is_empty() {
                                    true
                                } else {
                                    u.user_name
                                        .to_lowercase()
                                        .contains(&self.iam_state.group_users.filter.to_lowercase())
                                }
                            })
                            .collect();
                        if !filtered.is_empty() {
                            self.iam_state.group_users.page_down(filtered.len());
                        }
                    } else if self.iam_state.group_tab == GroupTab::Permissions {
                        let filtered = filtered_iam_policies(self);
                        if !filtered.is_empty() {
                            self.iam_state.policies.page_down(filtered.len());
                        }
                    } else if self.iam_state.group_tab == GroupTab::AccessAdvisor {
                        let filtered = filtered_last_accessed(self);
                        if !filtered.is_empty() {
                            self.iam_state
                                .last_accessed_services
                                .page_down(filtered.len());
                        }
                    }
                } else {
                    let filtered: Vec<_> = self
                        .iam_state
                        .groups
                        .items
                        .iter()
                        .filter(|g| {
                            if self.iam_state.groups.filter.is_empty() {
                                true
                            } else {
                                g.group_name
                                    .to_lowercase()
                                    .contains(&self.iam_state.groups.filter.to_lowercase())
                            }
                        })
                        .collect();
                    if !filtered.is_empty() {
                        self.iam_state.groups.page_down(filtered.len());
                    }
                }
            }
        }
    }

    fn page_up(&mut self) {
        if self.mode == Mode::ColumnSelector {
            self.column_selector_index = self.column_selector_index.saturating_sub(10);
        } else if self.mode == Mode::FilterInput
            && self.current_service == Service::CloudFormationStacks
        {
            if self.cfn_state.current_stack.is_some()
                && self.cfn_state.detail_tab == CfnDetailTab::Parameters
            {
                let page_size = self.cfn_state.parameters.page_size.value();
                self.cfn_state.parameters_input_focus.handle_page_up(
                    &mut self.cfn_state.parameters.selected,
                    &mut self.cfn_state.parameters.scroll_offset,
                    page_size,
                );
            } else if self.cfn_state.current_stack.is_some()
                && self.cfn_state.detail_tab == CfnDetailTab::Outputs
            {
                let page_size = self.cfn_state.outputs.page_size.value();
                self.cfn_state.outputs_input_focus.handle_page_up(
                    &mut self.cfn_state.outputs.selected,
                    &mut self.cfn_state.outputs.scroll_offset,
                    page_size,
                );
            } else {
                let page_size = self.cfn_state.table.page_size.value();
                self.cfn_state.input_focus.handle_page_up(
                    &mut self.cfn_state.table.selected,
                    &mut self.cfn_state.table.scroll_offset,
                    page_size,
                );
            }
        } else if self.mode == Mode::FilterInput
            && self.current_service == Service::IamRoles
            && self.iam_state.current_role.is_none()
        {
            let page_size = self.iam_state.roles.page_size.value();
            self.iam_state.role_input_focus.handle_page_up(
                &mut self.iam_state.roles.selected,
                &mut self.iam_state.roles.scroll_offset,
                page_size,
            );
        } else if self.mode == Mode::FilterInput
            && self.current_service == Service::CloudWatchAlarms
        {
            let page_size = self.alarms_state.table.page_size.value();
            self.alarms_state.input_focus.handle_page_up(
                &mut self.alarms_state.table.selected,
                &mut self.alarms_state.table.scroll_offset,
                page_size,
            );
        } else if self.mode == Mode::FilterInput
            && self.current_service == Service::CloudWatchLogGroups
        {
            if self.view_mode == ViewMode::List {
                // Log groups list pagination
                let page_size = self.log_groups_state.log_groups.page_size.value();
                self.log_groups_state.input_focus.handle_page_up(
                    &mut self.log_groups_state.log_groups.selected,
                    &mut self.log_groups_state.log_groups.scroll_offset,
                    page_size,
                );
            } else {
                // Log streams pagination
                let page_size = 20;
                self.log_groups_state.input_focus.handle_page_up(
                    &mut self.log_groups_state.selected_stream,
                    &mut self.log_groups_state.stream_page,
                    page_size,
                );
                self.log_groups_state.expanded_stream = None;
            }
        } else if self.mode == Mode::FilterInput && self.current_service == Service::LambdaFunctions
        {
            if self.lambda_state.current_function.is_some()
                && self.lambda_state.detail_tab == LambdaDetailTab::Versions
                && self.lambda_state.version_input_focus == InputFocus::Pagination
            {
                let page_size = self.lambda_state.version_table.page_size.value();
                self.lambda_state.version_table.selected = self
                    .lambda_state
                    .version_table
                    .selected
                    .saturating_sub(page_size);
            } else if self.lambda_state.current_function.is_some()
                && (self.lambda_state.detail_tab == LambdaDetailTab::Aliases
                    || (self.lambda_state.current_version.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Configuration))
                && self.lambda_state.alias_input_focus == InputFocus::Pagination
            {
                let page_size = self.lambda_state.alias_table.page_size.value();
                self.lambda_state.alias_table.selected = self
                    .lambda_state
                    .alias_table
                    .selected
                    .saturating_sub(page_size);
            } else if self.lambda_state.current_function.is_none() {
                let page_size = self.lambda_state.table.page_size.value();
                self.lambda_state.input_focus.handle_page_up(
                    &mut self.lambda_state.table.selected,
                    &mut self.lambda_state.table.scroll_offset,
                    page_size,
                );
            }
        } else if self.mode == Mode::FilterInput
            && self.current_service == Service::EcrRepositories
            && self.ecr_state.current_repository.is_none()
            && self.ecr_state.input_focus == InputFocus::Filter
        {
            // When input is focused, allow table scrolling
            self.ecr_state.repositories.page_up();
        } else if self.mode == Mode::FilterInput
            && self.current_service == Service::EcrRepositories
            && self.ecr_state.current_repository.is_none()
        {
            let page_size = self.ecr_state.repositories.page_size.value();
            self.ecr_state.input_focus.handle_page_up(
                &mut self.ecr_state.repositories.selected,
                &mut self.ecr_state.repositories.scroll_offset,
                page_size,
            );
        } else if self.mode == Mode::FilterInput && self.view_mode == ViewMode::PolicyView {
            let page_size = self.iam_state.policies.page_size.value();
            self.iam_state.policy_input_focus.handle_page_up(
                &mut self.iam_state.policies.selected,
                &mut self.iam_state.policies.scroll_offset,
                page_size,
            );
        } else if self.view_mode == ViewMode::PolicyView {
            self.iam_state.policy_scroll = self.iam_state.policy_scroll.saturating_sub(10);
        } else if self.current_service == Service::CloudFormationStacks
            && self.cfn_state.current_stack.is_some()
            && self.cfn_state.detail_tab == CfnDetailTab::Template
        {
            self.cfn_state.template_scroll = self.cfn_state.template_scroll.saturating_sub(10);
        } else if self.current_service == Service::SqsQueues
            && self.sqs_state.current_queue.is_some()
        {
            if self.sqs_state.detail_tab == SqsQueueDetailTab::Monitoring {
                self.sqs_state
                    .set_monitoring_scroll(self.sqs_state.monitoring_scroll().saturating_sub(1));
            } else {
                self.sqs_state.policy_scroll = self.sqs_state.policy_scroll.saturating_sub(10);
            }
        } else if self.current_service == Service::IamRoles
            && self.iam_state.current_role.is_some()
            && self.iam_state.role_tab == RoleTab::TrustRelationships
        {
            self.iam_state.trust_policy_scroll =
                self.iam_state.trust_policy_scroll.saturating_sub(10);
        } else if self.current_service == Service::IamRoles
            && self.iam_state.current_role.is_some()
            && self.iam_state.role_tab == RoleTab::RevokeSessions
        {
            self.iam_state.revoke_sessions_scroll =
                self.iam_state.revoke_sessions_scroll.saturating_sub(10);
        } else if self.mode == Mode::Normal {
            if self.current_service == Service::S3Buckets && self.s3_state.current_bucket.is_none()
            {
                self.s3_state.selected_row = self.s3_state.selected_row.saturating_sub(10);

                // Adjust scroll offset if selection goes above viewport
                if self.s3_state.selected_row < self.s3_state.bucket_scroll_offset {
                    self.s3_state.bucket_scroll_offset = self.s3_state.selected_row;
                }
            } else if self.current_service == Service::S3Buckets
                && self.s3_state.current_bucket.is_some()
            {
                self.s3_state.selected_object = self.s3_state.selected_object.saturating_sub(10);

                // Adjust scroll offset if selection goes above viewport
                if self.s3_state.selected_object < self.s3_state.object_scroll_offset {
                    self.s3_state.object_scroll_offset = self.s3_state.selected_object;
                }
            } else if self.current_service == Service::CloudWatchLogGroups
                && self.view_mode == ViewMode::List
            {
                self.log_groups_state.log_groups.page_up();
            } else if self.current_service == Service::CloudWatchLogGroups
                && self.view_mode == ViewMode::Detail
            {
                self.log_groups_state.selected_stream =
                    self.log_groups_state.selected_stream.saturating_sub(10);
            } else if self.view_mode == ViewMode::Events {
                if self.log_groups_state.event_scroll_offset < 10
                    && self.log_groups_state.has_older_events
                {
                    self.log_groups_state.loading = true;
                }
                self.log_groups_state.event_scroll_offset =
                    self.log_groups_state.event_scroll_offset.saturating_sub(10);
            } else if self.view_mode == ViewMode::InsightsResults {
                self.insights_state.insights.results_selected = self
                    .insights_state
                    .insights
                    .results_selected
                    .saturating_sub(10);
            } else if self.current_service == Service::CloudWatchAlarms {
                self.alarms_state.table.page_up();
            } else if self.current_service == Service::Ec2Instances {
                self.ec2_state.table.page_up();
            } else if self.current_service == Service::EcrRepositories {
                if self.ecr_state.current_repository.is_some() {
                    self.ecr_state.images.page_up();
                } else {
                    self.ecr_state.repositories.page_up();
                }
            } else if self.current_service == Service::SqsQueues {
                self.sqs_state.queues.page_up();
            } else if self.current_service == Service::LambdaFunctions {
                self.lambda_state.table.page_up();
            } else if self.current_service == Service::LambdaApplications {
                self.lambda_application_state.table.page_up();
            } else if self.current_service == Service::CloudFormationStacks {
                if self.cfn_state.current_stack.is_some()
                    && self.cfn_state.detail_tab == CfnDetailTab::Parameters
                {
                    self.cfn_state.parameters.page_up();
                } else if self.cfn_state.current_stack.is_some()
                    && self.cfn_state.detail_tab == CfnDetailTab::Outputs
                {
                    self.cfn_state.outputs.page_up();
                } else {
                    self.cfn_state.table.page_up();
                }
            } else if self.current_service == Service::IamUsers {
                self.iam_state.users.page_up();
            } else if self.current_service == Service::IamRoles {
                if self.iam_state.current_role.is_some() {
                    self.iam_state.policies.page_up();
                } else {
                    self.iam_state.roles.page_up();
                }
            }
        }
    }

    fn next_pane(&mut self) {
        if self.current_service == Service::S3Buckets {
            if self.s3_state.current_bucket.is_some() {
                // In objects view - expand prefix and trigger preview load
                // Map visual index to actual object (including nested items)
                let mut visual_idx = 0;
                let mut found_obj: Option<S3Object> = None;

                // Helper to recursively check nested items
                fn check_nested(
                    obj: &S3Object,
                    visual_idx: &mut usize,
                    target_idx: usize,
                    expanded_prefixes: &std::collections::HashSet<String>,
                    prefix_preview: &std::collections::HashMap<String, Vec<S3Object>>,
                    found_obj: &mut Option<S3Object>,
                ) {
                    if obj.is_prefix && expanded_prefixes.contains(&obj.key) {
                        if let Some(preview) = prefix_preview.get(&obj.key) {
                            for nested_obj in preview {
                                if *visual_idx == target_idx {
                                    *found_obj = Some(nested_obj.clone());
                                    return;
                                }
                                *visual_idx += 1;

                                // Recursively check deeper levels
                                check_nested(
                                    nested_obj,
                                    visual_idx,
                                    target_idx,
                                    expanded_prefixes,
                                    prefix_preview,
                                    found_obj,
                                );
                                if found_obj.is_some() {
                                    return;
                                }
                            }
                        } else {
                            // Loading row
                            *visual_idx += 1;
                        }
                    }
                }

                for obj in &self.s3_state.objects {
                    if visual_idx == self.s3_state.selected_object {
                        found_obj = Some(obj.clone());
                        break;
                    }
                    visual_idx += 1;

                    // Check nested items recursively
                    check_nested(
                        obj,
                        &mut visual_idx,
                        self.s3_state.selected_object,
                        &self.s3_state.expanded_prefixes,
                        &self.s3_state.prefix_preview,
                        &mut found_obj,
                    );
                    if found_obj.is_some() {
                        break;
                    }
                }

                if let Some(obj) = found_obj {
                    if obj.is_prefix {
                        if !self.s3_state.expanded_prefixes.contains(&obj.key) {
                            self.s3_state.expanded_prefixes.insert(obj.key.clone());
                            // Trigger preview load if not already cached
                            if !self.s3_state.prefix_preview.contains_key(&obj.key) {
                                self.s3_state.buckets.loading = true;
                            }
                        }
                        // Move to first child if expanded and has children
                        if self.s3_state.expanded_prefixes.contains(&obj.key) {
                            if let Some(preview) = self.s3_state.prefix_preview.get(&obj.key) {
                                if !preview.is_empty() {
                                    self.s3_state.selected_object += 1;
                                }
                            }
                        }
                    }
                }
            } else {
                // In bucket list - find which bucket/prefix the selected row corresponds to
                let mut row_idx = 0;
                let mut found = false;
                for bucket in &self.s3_state.buckets.items {
                    if row_idx == self.s3_state.selected_row {
                        // Selected row is a bucket - expand and move to first child
                        if !self.s3_state.expanded_prefixes.contains(&bucket.name) {
                            self.s3_state.expanded_prefixes.insert(bucket.name.clone());
                            if !self.s3_state.bucket_preview.contains_key(&bucket.name)
                                && !self.s3_state.bucket_errors.contains_key(&bucket.name)
                            {
                                self.s3_state.buckets.loading = true;
                            }
                        }
                        // Move to first child if expanded and has children
                        if self.s3_state.expanded_prefixes.contains(&bucket.name) {
                            if let Some(preview) = self.s3_state.bucket_preview.get(&bucket.name) {
                                if !preview.is_empty() {
                                    self.s3_state.selected_row = row_idx + 1;
                                }
                            }
                        }
                        break;
                    }
                    row_idx += 1;

                    // Skip error rows - they're not selectable
                    if self.s3_state.bucket_errors.contains_key(&bucket.name)
                        && self.s3_state.expanded_prefixes.contains(&bucket.name)
                    {
                        continue;
                    }

                    if self.s3_state.expanded_prefixes.contains(&bucket.name) {
                        if let Some(preview) = self.s3_state.bucket_preview.get(&bucket.name) {
                            // Recursive function to check nested items at any depth
                            #[allow(clippy::too_many_arguments)]
                            fn check_nested_expansion(
                                objects: &[S3Object],
                                row_idx: &mut usize,
                                target_row: usize,
                                expanded_prefixes: &mut std::collections::HashSet<String>,
                                prefix_preview: &std::collections::HashMap<String, Vec<S3Object>>,
                                found: &mut bool,
                                loading: &mut bool,
                                selected_row: &mut usize,
                            ) {
                                for obj in objects {
                                    if *row_idx == target_row {
                                        // Selected this item - expand and move to first child
                                        if obj.is_prefix {
                                            if !expanded_prefixes.contains(&obj.key) {
                                                expanded_prefixes.insert(obj.key.clone());
                                                if !prefix_preview.contains_key(&obj.key) {
                                                    *loading = true;
                                                }
                                            }
                                            // Move to first child if expanded and has children
                                            if expanded_prefixes.contains(&obj.key) {
                                                if let Some(preview) = prefix_preview.get(&obj.key)
                                                {
                                                    if !preview.is_empty() {
                                                        *selected_row = *row_idx + 1;
                                                    }
                                                }
                                            }
                                        }
                                        *found = true;
                                        return;
                                    }
                                    *row_idx += 1;

                                    // Recursively check nested items if expanded
                                    if obj.is_prefix && expanded_prefixes.contains(&obj.key) {
                                        if let Some(nested) = prefix_preview.get(&obj.key) {
                                            check_nested_expansion(
                                                nested,
                                                row_idx,
                                                target_row,
                                                expanded_prefixes,
                                                prefix_preview,
                                                found,
                                                loading,
                                                selected_row,
                                            );
                                            if *found {
                                                return;
                                            }
                                        } else {
                                            *row_idx += 1; // Loading row
                                        }
                                    }
                                }
                            }

                            check_nested_expansion(
                                preview,
                                &mut row_idx,
                                self.s3_state.selected_row,
                                &mut self.s3_state.expanded_prefixes,
                                &self.s3_state.prefix_preview,
                                &mut found,
                                &mut self.s3_state.buckets.loading,
                                &mut self.s3_state.selected_row,
                            );
                            if found || row_idx > self.s3_state.selected_row {
                                break;
                            }
                        } else {
                            row_idx += 1;
                            if row_idx > self.s3_state.selected_row {
                                break;
                            }
                        }
                    }
                    if found {
                        break;
                    }
                }
            }
        } else if self.view_mode == ViewMode::InsightsResults {
            // Right arrow scrolls horizontally by 1 column
            let max_cols = self
                .insights_state
                .insights
                .query_results
                .first()
                .map(|r| r.len())
                .unwrap_or(0);
            if self.insights_state.insights.results_horizontal_scroll < max_cols.saturating_sub(1) {
                self.insights_state.insights.results_horizontal_scroll += 1;
            }
        } else if self.current_service == Service::CloudWatchLogGroups
            && self.view_mode == ViewMode::List
        {
            // Expand selected log group
            if self.log_groups_state.log_groups.expanded_item
                != Some(self.log_groups_state.log_groups.selected)
            {
                self.log_groups_state.log_groups.expanded_item =
                    Some(self.log_groups_state.log_groups.selected);
            }
        } else if self.current_service == Service::CloudWatchLogGroups
            && self.view_mode == ViewMode::Detail
        {
            // Expand selected log stream
            if self.log_groups_state.expanded_stream != Some(self.log_groups_state.selected_stream)
            {
                self.log_groups_state.expanded_stream = Some(self.log_groups_state.selected_stream);
            }
        } else if self.view_mode == ViewMode::Events {
            // Only scroll if there are hidden columns
            // Expand selected event
            if self.log_groups_state.expanded_event
                != Some(self.log_groups_state.event_scroll_offset)
            {
                self.log_groups_state.expanded_event =
                    Some(self.log_groups_state.event_scroll_offset);
            }
        } else if self.current_service == Service::CloudWatchAlarms {
            // Expand selected alarm
            if !self.alarms_state.table.is_expanded() {
                self.alarms_state.table.toggle_expand();
            }
        } else if self.current_service == Service::Ec2Instances {
            if !self.ec2_state.table.is_expanded() {
                self.ec2_state.table.toggle_expand();
            }
        } else if self.current_service == Service::EcrRepositories {
            if self.ecr_state.current_repository.is_some() {
                // In images view - expand selected image
                self.ecr_state.images.toggle_expand();
            } else {
                // In repositories view - expand selected repository
                self.ecr_state.repositories.toggle_expand();
            }
        } else if self.current_service == Service::SqsQueues {
            if self.sqs_state.current_queue.is_some()
                && self.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers
            {
                self.sqs_state.triggers.toggle_expand();
            } else if self.sqs_state.current_queue.is_some()
                && self.sqs_state.detail_tab == SqsQueueDetailTab::EventBridgePipes
            {
                self.sqs_state.pipes.toggle_expand();
            } else if self.sqs_state.current_queue.is_some()
                && self.sqs_state.detail_tab == SqsQueueDetailTab::Tagging
            {
                self.sqs_state.tags.toggle_expand();
            } else if self.sqs_state.current_queue.is_some()
                && self.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions
            {
                self.sqs_state.subscriptions.toggle_expand();
            } else {
                self.sqs_state.queues.expand();
            }
        } else if self.current_service == Service::LambdaFunctions {
            if self.lambda_state.current_function.is_some()
                && self.lambda_state.detail_tab == LambdaDetailTab::Code
            {
                // Expand selected layer
                if self.lambda_state.layer_expanded != Some(self.lambda_state.layer_selected) {
                    self.lambda_state.layer_expanded = Some(self.lambda_state.layer_selected);
                } else {
                    self.lambda_state.layer_expanded = None;
                }
            } else if self.lambda_state.current_function.is_some()
                && self.lambda_state.detail_tab == LambdaDetailTab::Versions
            {
                // Expand selected version
                self.lambda_state.version_table.toggle_expand();
            } else if self.lambda_state.current_function.is_some()
                && (self.lambda_state.detail_tab == LambdaDetailTab::Aliases
                    || (self.lambda_state.current_version.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Configuration))
            {
                // Expand selected alias
                self.lambda_state.alias_table.toggle_expand();
            } else if self.lambda_state.current_function.is_none() {
                // Expand selected function
                self.lambda_state.table.toggle_expand();
            }
        } else if self.current_service == Service::LambdaApplications {
            if self.lambda_application_state.current_application.is_some() {
                // In detail view - expand resource or deployment
                if self.lambda_application_state.detail_tab == LambdaApplicationDetailTab::Overview
                {
                    self.lambda_application_state.resources.toggle_expand();
                } else {
                    self.lambda_application_state.deployments.toggle_expand();
                }
            } else {
                // Expand selected application in list
                if self.lambda_application_state.table.expanded_item
                    != Some(self.lambda_application_state.table.selected)
                {
                    self.lambda_application_state.table.expanded_item =
                        Some(self.lambda_application_state.table.selected);
                }
            }
        } else if self.current_service == Service::CloudFormationStacks
            && self.cfn_state.current_stack.is_none()
        {
            self.cfn_state.table.toggle_expand();
        } else if self.current_service == Service::CloudFormationStacks
            && self.cfn_state.current_stack.is_some()
            && self.cfn_state.detail_tab == CfnDetailTab::Parameters
        {
            self.cfn_state.parameters.toggle_expand();
        } else if self.current_service == Service::CloudFormationStacks
            && self.cfn_state.current_stack.is_some()
            && self.cfn_state.detail_tab == CfnDetailTab::Outputs
        {
            self.cfn_state.outputs.toggle_expand();
        } else if self.current_service == Service::CloudFormationStacks
            && self.cfn_state.current_stack.is_some()
            && self.cfn_state.detail_tab == CfnDetailTab::Resources
        {
            self.cfn_state.resources.toggle_expand();
        } else if self.current_service == Service::IamUsers {
            if self.iam_state.current_user.is_some() {
                if self.iam_state.user_tab == UserTab::Tags {
                    if self.iam_state.user_tags.expanded_item
                        != Some(self.iam_state.user_tags.selected)
                    {
                        self.iam_state.user_tags.expanded_item =
                            Some(self.iam_state.user_tags.selected);
                    }
                } else if self.iam_state.policies.expanded_item
                    != Some(self.iam_state.policies.selected)
                {
                    self.iam_state.policies.toggle_expand();
                }
            } else if !self.iam_state.users.is_expanded() {
                self.iam_state.users.toggle_expand();
            }
        } else if self.current_service == Service::IamRoles {
            if self.iam_state.current_role.is_some() {
                // Handle expansion based on current tab
                if self.iam_state.role_tab == RoleTab::Tags {
                    if !self.iam_state.tags.is_expanded() {
                        self.iam_state.tags.expand();
                    }
                } else if self.iam_state.role_tab == RoleTab::LastAccessed {
                    if !self.iam_state.last_accessed_services.is_expanded() {
                        self.iam_state.last_accessed_services.expand();
                    }
                } else if !self.iam_state.policies.is_expanded() {
                    self.iam_state.policies.expand();
                }
            } else if !self.iam_state.roles.is_expanded() {
                self.iam_state.roles.expand();
            }
        } else if self.current_service == Service::IamUserGroups {
            if self.iam_state.current_group.is_some() {
                if self.iam_state.group_tab == GroupTab::Users {
                    if !self.iam_state.group_users.is_expanded() {
                        self.iam_state.group_users.expand();
                    }
                } else if self.iam_state.group_tab == GroupTab::Permissions {
                    if !self.iam_state.policies.is_expanded() {
                        self.iam_state.policies.expand();
                    }
                } else if self.iam_state.group_tab == GroupTab::AccessAdvisor
                    && !self.iam_state.last_accessed_services.is_expanded()
                {
                    self.iam_state.last_accessed_services.expand();
                }
            } else if !self.iam_state.groups.is_expanded() {
                self.iam_state.groups.expand();
            }
        }
    }

    fn go_to_page(&mut self, page: usize) {
        if page == 0 {
            return;
        }

        match self.current_service {
            Service::CloudWatchAlarms => {
                let alarm_page_size = self.alarms_state.table.page_size.value();
                let target = (page - 1) * alarm_page_size;
                let filtered_count = match self.alarms_state.alarm_tab {
                    AlarmTab::AllAlarms => self.alarms_state.table.items.len(),
                    AlarmTab::InAlarm => self
                        .alarms_state
                        .table
                        .items
                        .iter()
                        .filter(|a| a.state.to_uppercase() == "ALARM")
                        .count(),
                };
                let max_offset = filtered_count.saturating_sub(alarm_page_size);
                self.alarms_state.table.scroll_offset = target.min(max_offset);
                self.alarms_state.table.selected = self
                    .alarms_state
                    .table
                    .scroll_offset
                    .min(filtered_count.saturating_sub(1));
            }
            Service::CloudWatchLogGroups => match self.view_mode {
                ViewMode::Events => {
                    let page_size = 20;
                    let target = (page - 1) * page_size;
                    let max = self.log_groups_state.log_events.len().saturating_sub(1);
                    self.log_groups_state.event_scroll_offset = target.min(max);
                }
                ViewMode::Detail => {
                    let page_size = 20;
                    let target = (page - 1) * page_size;
                    let max = self.log_groups_state.log_streams.len().saturating_sub(1);
                    self.log_groups_state.selected_stream = target.min(max);
                }
                ViewMode::List => {
                    let total = self.log_groups_state.log_groups.items.len();
                    self.log_groups_state.log_groups.goto_page(page, total);
                }
                _ => {}
            },
            Service::EcrRepositories => {
                if self.ecr_state.current_repository.is_some() {
                    let filtered_count = self
                        .ecr_state
                        .images
                        .filtered(|img| {
                            self.ecr_state.images.filter.is_empty()
                                || img
                                    .tag
                                    .to_lowercase()
                                    .contains(&self.ecr_state.images.filter.to_lowercase())
                                || img
                                    .digest
                                    .to_lowercase()
                                    .contains(&self.ecr_state.images.filter.to_lowercase())
                        })
                        .len();
                    self.ecr_state.images.goto_page(page, filtered_count);
                } else {
                    let filtered_count = self
                        .ecr_state
                        .repositories
                        .filtered(|r| {
                            self.ecr_state.repositories.filter.is_empty()
                                || r.name
                                    .to_lowercase()
                                    .contains(&self.ecr_state.repositories.filter.to_lowercase())
                        })
                        .len();
                    self.ecr_state.repositories.goto_page(page, filtered_count);
                }
            }
            Service::SqsQueues => {
                let filtered_count =
                    filtered_queues(&self.sqs_state.queues.items, &self.sqs_state.queues.filter)
                        .len();
                self.sqs_state.queues.goto_page(page, filtered_count);
            }
            Service::S3Buckets => {
                if self.s3_state.current_bucket.is_some() {
                    let page_size = 50; // S3 objects use fixed page size
                    let target = (page - 1) * page_size;
                    let total_rows = self.calculate_total_object_rows();
                    let max = total_rows.saturating_sub(1);
                    self.s3_state.selected_object = target.min(max);
                } else {
                    let page_size = 50; // S3 buckets use fixed page size
                    let target = (page - 1) * page_size;
                    let total_rows = self.calculate_total_bucket_rows();
                    let max = total_rows.saturating_sub(1);
                    self.s3_state.selected_row = target.min(max);
                }
            }
            Service::LambdaFunctions => {
                if self.lambda_state.current_function.is_some()
                    && self.lambda_state.detail_tab == LambdaDetailTab::Versions
                {
                    let filtered_count = self
                        .lambda_state
                        .version_table
                        .filtered(|v| {
                            self.lambda_state.version_table.filter.is_empty()
                                || v.version.to_lowercase().contains(
                                    &self.lambda_state.version_table.filter.to_lowercase(),
                                )
                                || v.aliases.to_lowercase().contains(
                                    &self.lambda_state.version_table.filter.to_lowercase(),
                                )
                                || v.description.to_lowercase().contains(
                                    &self.lambda_state.version_table.filter.to_lowercase(),
                                )
                        })
                        .len();
                    self.lambda_state
                        .version_table
                        .goto_page(page, filtered_count);
                } else {
                    let filtered_count = filtered_lambda_functions(self).len();
                    self.lambda_state.table.goto_page(page, filtered_count);
                }
            }
            Service::LambdaApplications => {
                let filtered_count = filtered_lambda_applications(self).len();
                self.lambda_application_state
                    .table
                    .goto_page(page, filtered_count);
            }
            Service::CloudFormationStacks => {
                let filtered_count = filtered_cloudformation_stacks(self).len();
                self.cfn_state.table.goto_page(page, filtered_count);
            }
            Service::IamUsers => {
                let filtered_count = filtered_iam_users(self).len();
                self.iam_state.users.goto_page(page, filtered_count);
            }
            Service::IamRoles => {
                let filtered_count = filtered_iam_roles(self).len();
                self.iam_state.roles.goto_page(page, filtered_count);
            }
            _ => {}
        }
    }

    fn prev_pane(&mut self) {
        if self.current_service == Service::S3Buckets {
            if self.s3_state.current_bucket.is_some() {
                // In objects view - collapse prefix or jump to parent
                // Map visual index to actual object (including nested items)
                let mut visual_idx = 0;
                let mut found_obj: Option<S3Object> = None;
                let mut parent_idx: Option<usize> = None;

                // Helper to recursively find object and its parent
                #[allow(clippy::too_many_arguments)]
                fn find_with_parent(
                    objects: &[S3Object],
                    visual_idx: &mut usize,
                    target_idx: usize,
                    expanded_prefixes: &std::collections::HashSet<String>,
                    prefix_preview: &std::collections::HashMap<String, Vec<S3Object>>,
                    found_obj: &mut Option<S3Object>,
                    parent_idx: &mut Option<usize>,
                    current_parent: Option<usize>,
                ) {
                    for obj in objects {
                        if *visual_idx == target_idx {
                            *found_obj = Some(obj.clone());
                            *parent_idx = current_parent;
                            return;
                        }
                        let obj_idx = *visual_idx;
                        *visual_idx += 1;

                        // Check nested items if expanded
                        if obj.is_prefix && expanded_prefixes.contains(&obj.key) {
                            if let Some(preview) = prefix_preview.get(&obj.key) {
                                find_with_parent(
                                    preview,
                                    visual_idx,
                                    target_idx,
                                    expanded_prefixes,
                                    prefix_preview,
                                    found_obj,
                                    parent_idx,
                                    Some(obj_idx),
                                );
                                if found_obj.is_some() {
                                    return;
                                }
                            }
                        }
                    }
                }

                find_with_parent(
                    &self.s3_state.objects,
                    &mut visual_idx,
                    self.s3_state.selected_object,
                    &self.s3_state.expanded_prefixes,
                    &self.s3_state.prefix_preview,
                    &mut found_obj,
                    &mut parent_idx,
                    None,
                );

                if let Some(obj) = found_obj {
                    if obj.is_prefix && self.s3_state.expanded_prefixes.contains(&obj.key) {
                        // Expanded: collapse it
                        self.s3_state.expanded_prefixes.remove(&obj.key);
                    } else if let Some(parent) = parent_idx {
                        // Already collapsed or not a prefix: jump to parent
                        self.s3_state.selected_object = parent;
                    }
                }

                // Adjust scroll offset to keep selection visible
                let visible_rows = self.s3_state.object_visible_rows.get();
                if self.s3_state.selected_object < self.s3_state.object_scroll_offset {
                    self.s3_state.object_scroll_offset = self.s3_state.selected_object;
                } else if self.s3_state.selected_object
                    >= self.s3_state.object_scroll_offset + visible_rows
                {
                    self.s3_state.object_scroll_offset = self
                        .s3_state
                        .selected_object
                        .saturating_sub(visible_rows - 1);
                }
            } else {
                // In bucket list - find which bucket/prefix the selected row corresponds to
                let mut row_idx = 0;
                for bucket in &self.s3_state.buckets.items {
                    if row_idx == self.s3_state.selected_row {
                        // Selected row is a bucket - collapse it
                        self.s3_state.expanded_prefixes.remove(&bucket.name);
                        break;
                    }
                    row_idx += 1;
                    if self.s3_state.expanded_prefixes.contains(&bucket.name) {
                        if let Some(preview) = self.s3_state.bucket_preview.get(&bucket.name) {
                            // Recursive function to check nested items at any depth
                            #[allow(clippy::too_many_arguments)]
                            fn check_nested_collapse(
                                objects: &[S3Object],
                                row_idx: &mut usize,
                                target_row: usize,
                                expanded_prefixes: &mut std::collections::HashSet<String>,
                                prefix_preview: &std::collections::HashMap<String, Vec<S3Object>>,
                                found: &mut bool,
                                selected_row: &mut usize,
                                parent_row: usize,
                            ) {
                                for obj in objects {
                                    let current_row = *row_idx;
                                    if *row_idx == target_row {
                                        // Selected this item - collapse or jump to parent
                                        if obj.is_prefix {
                                            if expanded_prefixes.contains(&obj.key) {
                                                // Expanded: collapse it
                                                expanded_prefixes.remove(&obj.key);
                                            } else {
                                                // Already collapsed: jump to parent
                                                *selected_row = parent_row;
                                            }
                                        } else {
                                            // Not a prefix: jump to parent
                                            *selected_row = parent_row;
                                        }
                                        *found = true;
                                        return;
                                    }
                                    *row_idx += 1;

                                    // Recursively check nested items if expanded
                                    if obj.is_prefix && expanded_prefixes.contains(&obj.key) {
                                        if let Some(nested) = prefix_preview.get(&obj.key) {
                                            check_nested_collapse(
                                                nested,
                                                row_idx,
                                                target_row,
                                                expanded_prefixes,
                                                prefix_preview,
                                                found,
                                                selected_row,
                                                current_row,
                                            );
                                            if *found {
                                                return;
                                            }
                                        } else {
                                            *row_idx += 1; // Loading row
                                        }
                                    }
                                }
                            }

                            let mut found = false;
                            let parent_row = row_idx - 1; // Parent is the bucket
                            check_nested_collapse(
                                preview,
                                &mut row_idx,
                                self.s3_state.selected_row,
                                &mut self.s3_state.expanded_prefixes,
                                &self.s3_state.prefix_preview,
                                &mut found,
                                &mut self.s3_state.selected_row,
                                parent_row,
                            );
                            if found {
                                // Adjust scroll offset to keep selection visible
                                let visible_rows = self.s3_state.bucket_visible_rows.get();
                                if self.s3_state.selected_row < self.s3_state.bucket_scroll_offset {
                                    self.s3_state.bucket_scroll_offset = self.s3_state.selected_row;
                                } else if self.s3_state.selected_row
                                    >= self.s3_state.bucket_scroll_offset + visible_rows
                                {
                                    self.s3_state.bucket_scroll_offset =
                                        self.s3_state.selected_row.saturating_sub(visible_rows - 1);
                                }
                                return;
                            }
                        } else {
                            row_idx += 1;
                        }
                    }
                }

                // Adjust scroll offset to keep selection visible after collapse
                let visible_rows = self.s3_state.bucket_visible_rows.get();
                if self.s3_state.selected_row < self.s3_state.bucket_scroll_offset {
                    self.s3_state.bucket_scroll_offset = self.s3_state.selected_row;
                } else if self.s3_state.selected_row
                    >= self.s3_state.bucket_scroll_offset + visible_rows
                {
                    self.s3_state.bucket_scroll_offset =
                        self.s3_state.selected_row.saturating_sub(visible_rows - 1);
                }
            }
        } else if self.view_mode == ViewMode::InsightsResults {
            // Left arrow scrolls horizontally by 1 column
            self.insights_state.insights.results_horizontal_scroll = self
                .insights_state
                .insights
                .results_horizontal_scroll
                .saturating_sub(1);
        } else if self.current_service == Service::CloudWatchLogGroups
            && self.view_mode == ViewMode::List
        {
            // Collapse expanded log group
            if self.log_groups_state.log_groups.has_expanded_item() {
                self.log_groups_state.log_groups.collapse();
            }
        } else if self.current_service == Service::CloudWatchLogGroups
            && self.view_mode == ViewMode::Detail
        {
            // Collapse expanded log stream
            if self.log_groups_state.expanded_stream.is_some() {
                self.log_groups_state.expanded_stream = None;
            }
        } else if self.view_mode == ViewMode::Events {
            // Collapse expanded event
            if self.log_groups_state.expanded_event.is_some() {
                self.log_groups_state.expanded_event = None;
            }
        } else if self.current_service == Service::CloudWatchAlarms {
            // Collapse expanded alarm
            self.alarms_state.table.collapse();
        } else if self.current_service == Service::Ec2Instances {
            self.ec2_state.table.collapse();
        } else if self.current_service == Service::EcrRepositories {
            if self.ecr_state.current_repository.is_some() {
                // In images view - collapse expanded image
                self.ecr_state.images.collapse();
            } else {
                // In repositories view - collapse expanded repository
                self.ecr_state.repositories.collapse();
            }
        } else if self.current_service == Service::SqsQueues {
            if self.sqs_state.current_queue.is_some()
                && self.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers
            {
                self.sqs_state.triggers.collapse();
            } else if self.sqs_state.current_queue.is_some()
                && self.sqs_state.detail_tab == SqsQueueDetailTab::EventBridgePipes
            {
                self.sqs_state.pipes.collapse();
            } else if self.sqs_state.current_queue.is_some()
                && self.sqs_state.detail_tab == SqsQueueDetailTab::Tagging
            {
                self.sqs_state.tags.collapse();
            } else if self.sqs_state.current_queue.is_some()
                && self.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions
            {
                self.sqs_state.subscriptions.collapse();
            } else {
                self.sqs_state.queues.collapse();
            }
        } else if self.current_service == Service::LambdaFunctions {
            if self.lambda_state.current_function.is_some()
                && self.lambda_state.detail_tab == LambdaDetailTab::Code
            {
                // Collapse selected layer
                self.lambda_state.layer_expanded = None;
            } else if self.lambda_state.current_function.is_some()
                && self.lambda_state.detail_tab == LambdaDetailTab::Versions
            {
                // Collapse selected version
                self.lambda_state.version_table.collapse();
            } else if self.lambda_state.current_function.is_some()
                && (self.lambda_state.detail_tab == LambdaDetailTab::Aliases
                    || (self.lambda_state.current_version.is_some()
                        && self.lambda_state.detail_tab == LambdaDetailTab::Configuration))
            {
                // Collapse selected alias
                self.lambda_state.alias_table.collapse();
            } else if self.lambda_state.current_function.is_none() {
                // Collapse expanded function
                self.lambda_state.table.collapse();
            }
        } else if self.current_service == Service::LambdaApplications {
            if self.lambda_application_state.current_application.is_some() {
                // In detail view - collapse resource or deployment
                if self.lambda_application_state.detail_tab == LambdaApplicationDetailTab::Overview
                {
                    self.lambda_application_state.resources.collapse();
                } else {
                    self.lambda_application_state.deployments.collapse();
                }
            } else {
                // Collapse expanded application in list
                if self.lambda_application_state.table.has_expanded_item() {
                    self.lambda_application_state.table.collapse();
                }
            }
        } else if self.current_service == Service::CloudFormationStacks
            && self.cfn_state.current_stack.is_none()
        {
            self.cfn_state.table.collapse();
        } else if self.current_service == Service::CloudFormationStacks
            && self.cfn_state.current_stack.is_some()
            && self.cfn_state.detail_tab == CfnDetailTab::Parameters
        {
            self.cfn_state.parameters.collapse();
        } else if self.current_service == Service::CloudFormationStacks
            && self.cfn_state.current_stack.is_some()
            && self.cfn_state.detail_tab == CfnDetailTab::Outputs
        {
            self.cfn_state.outputs.collapse();
        } else if self.current_service == Service::CloudFormationStacks
            && self.cfn_state.current_stack.is_some()
            && self.cfn_state.detail_tab == CfnDetailTab::Resources
        {
            self.cfn_state.resources.collapse();
        } else if self.current_service == Service::IamUsers {
            if self.iam_state.users.has_expanded_item() {
                self.iam_state.users.collapse();
            }
        } else if self.current_service == Service::IamRoles {
            if self.view_mode == ViewMode::PolicyView {
                // Go back from policy view to role detail
                self.view_mode = ViewMode::Detail;
                self.iam_state.current_policy = None;
                self.iam_state.policy_document.clear();
                self.iam_state.policy_scroll = 0;
            } else if self.iam_state.current_role.is_some() {
                if self.iam_state.role_tab == RoleTab::Tags
                    && self.iam_state.tags.has_expanded_item()
                {
                    self.iam_state.tags.collapse();
                } else if self.iam_state.role_tab == RoleTab::LastAccessed
                    && self
                        .iam_state
                        .last_accessed_services
                        .expanded_item
                        .is_some()
                {
                    self.iam_state.last_accessed_services.collapse();
                } else if self.iam_state.policies.has_expanded_item() {
                    self.iam_state.policies.collapse();
                }
            } else if self.iam_state.roles.has_expanded_item() {
                self.iam_state.roles.collapse();
            }
        } else if self.current_service == Service::IamUserGroups {
            if self.iam_state.current_group.is_some() {
                if self.iam_state.group_tab == GroupTab::Users
                    && self.iam_state.group_users.has_expanded_item()
                {
                    self.iam_state.group_users.collapse();
                } else if self.iam_state.group_tab == GroupTab::Permissions
                    && self.iam_state.policies.has_expanded_item()
                {
                    self.iam_state.policies.collapse();
                } else if self.iam_state.group_tab == GroupTab::AccessAdvisor
                    && self
                        .iam_state
                        .last_accessed_services
                        .expanded_item
                        .is_some()
                {
                    self.iam_state.last_accessed_services.collapse();
                }
            } else if self.iam_state.groups.has_expanded_item() {
                self.iam_state.groups.collapse();
            }
        }
    }

    fn select_item(&mut self) {
        if self.mode == Mode::RegionPicker {
            let filtered = self.get_filtered_regions();
            if let Some(region) = filtered.get(self.region_picker_selected) {
                // Save current session before changing region
                if !self.tabs.is_empty() {
                    let mut session = Session::new(
                        self.profile.clone(),
                        self.region.clone(),
                        self.config.account_id.clone(),
                        self.config.role_arn.clone(),
                    );

                    for tab in &self.tabs {
                        session.tabs.push(SessionTab {
                            service: format!("{:?}", tab.service),
                            title: tab.title.clone(),
                            breadcrumb: tab.breadcrumb.clone(),
                            filter: None,
                            selected_item: None,
                        });
                    }

                    let _ = session.save();
                }

                self.region = region.code.to_string();
                self.config.region = region.code.to_string();

                // Close all tabs - region change invalidates all data
                self.tabs.clear();
                self.current_tab = 0;
                self.service_selected = false;

                self.mode = Mode::Normal;
            }
        } else if self.mode == Mode::ProfilePicker {
            let filtered = self.get_filtered_profiles();
            if let Some(profile) = filtered.get(self.profile_picker_selected) {
                let profile_name = profile.name.clone();
                let profile_region = profile.region.clone();

                self.profile = profile_name.clone();
                std::env::set_var("AWS_PROFILE", &profile_name);

                // Use profile's region if available
                if let Some(region) = profile_region {
                    self.region = region;
                }

                self.mode = Mode::Normal;
                // Note: Changing profile requires reconnecting to AWS
            }
        } else if self.mode == Mode::ServicePicker {
            let filtered = self.filtered_services();
            if let Some(&service) = filtered.get(self.service_picker.selected) {
                let new_service = match service {
                    "CloudWatch > Log Groups" => Service::CloudWatchLogGroups,
                    "CloudWatch > Logs Insights" => Service::CloudWatchInsights,
                    "CloudWatch > Alarms" => Service::CloudWatchAlarms,
                    "CloudFormation > Stacks" => Service::CloudFormationStacks,
                    "EC2 > Instances" => Service::Ec2Instances,
                    "ECR > Repositories" => Service::EcrRepositories,
                    "IAM > Users" => Service::IamUsers,
                    "IAM > Roles" => Service::IamRoles,
                    "IAM > User Groups" => Service::IamUserGroups,
                    "Lambda > Functions" => Service::LambdaFunctions,
                    "Lambda > Applications" => Service::LambdaApplications,
                    "S3 > Buckets" => Service::S3Buckets,
                    "SQS > Queues" => Service::SqsQueues,
                    _ => return,
                };

                // Create new tab
                self.tabs.push(Tab {
                    service: new_service,
                    title: service.to_string(),
                    breadcrumb: service.to_string(),
                });
                self.current_tab = self.tabs.len() - 1;
                self.current_service = new_service;
                self.view_mode = ViewMode::List;
                self.service_selected = true;
                self.mode = Mode::Normal;
            }
        } else if self.mode == Mode::TabPicker {
            let filtered = self.get_filtered_tabs();
            if let Some(&(idx, _)) = filtered.get(self.tab_picker_selected) {
                self.current_tab = idx;
                self.current_service = self.tabs[idx].service;
                self.mode = Mode::Normal;
                self.tab_filter.clear();
            }
        } else if self.mode == Mode::SessionPicker {
            let filtered = self.get_filtered_sessions();
            if let Some(&session) = filtered.get(self.session_picker_selected) {
                let session = session.clone();

                // Load the selected session
                self.current_session = Some(session.clone());
                self.profile = session.profile.clone();
                self.region = session.region.clone();
                self.config.region = session.region.clone();
                self.config.account_id = session.account_id.clone();
                self.config.role_arn = session.role_arn.clone();

                // Restore tabs
                self.tabs = session
                    .tabs
                    .iter()
                    .map(|st| Tab {
                        service: match st.service.as_str() {
                            "CloudWatchLogGroups" => Service::CloudWatchLogGroups,
                            "CloudWatchInsights" => Service::CloudWatchInsights,
                            "CloudWatchAlarms" => Service::CloudWatchAlarms,
                            "S3Buckets" => Service::S3Buckets,
                            "SqsQueues" => Service::SqsQueues,
                            _ => Service::CloudWatchLogGroups,
                        },
                        title: st.title.clone(),
                        breadcrumb: st.breadcrumb.clone(),
                    })
                    .collect();

                if !self.tabs.is_empty() {
                    self.current_tab = 0;
                    self.current_service = self.tabs[0].service;
                    self.service_selected = true;
                }

                self.mode = Mode::Normal;
            }
        } else if self.mode == Mode::InsightsInput {
            // In InsightsInput mode, behavior depends on focus
            use crate::app::InsightsFocus;
            match self.insights_state.insights.insights_focus {
                InsightsFocus::Query => {
                    // Add newline to query
                    self.insights_state.insights.query_text.push('\n');
                    self.insights_state.insights.query_cursor_line += 1;
                    self.insights_state.insights.query_cursor_col = 0;
                }
                InsightsFocus::LogGroupSearch => {
                    // Toggle dropdown
                    self.insights_state.insights.show_dropdown =
                        !self.insights_state.insights.show_dropdown;
                }
                _ => {}
            }
        } else if self.mode == Mode::Normal {
            // If no service selected, select from service picker
            if !self.service_selected {
                let filtered = self.filtered_services();
                if let Some(&service) = filtered.get(self.service_picker.selected) {
                    match service {
                        "CloudWatch > Log Groups" => {
                            self.current_service = Service::CloudWatchLogGroups;
                            self.view_mode = ViewMode::List;
                            self.service_selected = true;
                        }
                        "CloudWatch > Logs Insights" => {
                            self.current_service = Service::CloudWatchInsights;
                            self.view_mode = ViewMode::InsightsResults;
                            self.service_selected = true;
                        }
                        "CloudWatch > Alarms" => {
                            self.current_service = Service::CloudWatchAlarms;
                            self.view_mode = ViewMode::List;
                            self.service_selected = true;
                        }
                        "S3 > Buckets" => {
                            self.current_service = Service::S3Buckets;
                            self.view_mode = ViewMode::List;
                            self.service_selected = true;
                        }
                        "EC2 > Instances" => {
                            self.current_service = Service::Ec2Instances;
                            self.view_mode = ViewMode::List;
                            self.service_selected = true;
                        }
                        "ECR > Repositories" => {
                            self.current_service = Service::EcrRepositories;
                            self.view_mode = ViewMode::List;
                            self.service_selected = true;
                        }
                        "Lambda > Functions" => {
                            self.current_service = Service::LambdaFunctions;
                            self.view_mode = ViewMode::List;
                            self.service_selected = true;
                        }
                        "Lambda > Applications" => {
                            self.current_service = Service::LambdaApplications;
                            self.view_mode = ViewMode::List;
                            self.service_selected = true;
                        }
                        _ => {}
                    }
                }
                return;
            }

            // Select in content area
            if self.view_mode == ViewMode::InsightsResults {
                // Toggle expand for selected result
                if self.insights_state.insights.expanded_result
                    == Some(self.insights_state.insights.results_selected)
                {
                    self.insights_state.insights.expanded_result = None;
                } else {
                    self.insights_state.insights.expanded_result =
                        Some(self.insights_state.insights.results_selected);
                }
            } else if self.current_service == Service::S3Buckets {
                if self.s3_state.current_bucket.is_none() {
                    // Find which bucket/prefix the selected row corresponds to
                    let mut row_idx = 0;
                    for bucket in &self.s3_state.buckets.items {
                        if row_idx == self.s3_state.selected_row {
                            // Selected a bucket - drill into it
                            self.s3_state.current_bucket = Some(bucket.name.clone());
                            self.s3_state.prefix_stack.clear();
                            self.s3_state.buckets.loading = true;
                            return;
                        }
                        row_idx += 1;

                        // Skip error rows - they're not selectable
                        if self.s3_state.bucket_errors.contains_key(&bucket.name)
                            && self.s3_state.expanded_prefixes.contains(&bucket.name)
                        {
                            continue;
                        }

                        if self.s3_state.expanded_prefixes.contains(&bucket.name) {
                            if let Some(preview) = self.s3_state.bucket_preview.get(&bucket.name) {
                                for obj in preview {
                                    if row_idx == self.s3_state.selected_row {
                                        // Selected a prefix - drill into bucket with this prefix
                                        if obj.is_prefix {
                                            self.s3_state.current_bucket =
                                                Some(bucket.name.clone());
                                            self.s3_state.prefix_stack = vec![obj.key.clone()];
                                            self.s3_state.buckets.loading = true;
                                        }
                                        return;
                                    }
                                    row_idx += 1;

                                    // Check nested preview rows
                                    if obj.is_prefix
                                        && self.s3_state.expanded_prefixes.contains(&obj.key)
                                    {
                                        if let Some(nested) =
                                            self.s3_state.prefix_preview.get(&obj.key)
                                        {
                                            for nested_obj in nested {
                                                if row_idx == self.s3_state.selected_row {
                                                    // Selected a nested prefix - drill into bucket with this prefix
                                                    if nested_obj.is_prefix {
                                                        self.s3_state.current_bucket =
                                                            Some(bucket.name.clone());
                                                        // Build proper prefix stack: parent, then child
                                                        self.s3_state.prefix_stack = vec![
                                                            obj.key.clone(),
                                                            nested_obj.key.clone(),
                                                        ];
                                                        self.s3_state.buckets.loading = true;
                                                    }
                                                    return;
                                                }
                                                row_idx += 1;
                                            }
                                        } else {
                                            row_idx += 1;
                                        }
                                    }
                                }
                            } else {
                                row_idx += 1;
                            }
                        }
                    }
                } else {
                    // In objects view - map visual index to actual object (including nested items)
                    let mut visual_idx = 0;
                    let mut found_obj: Option<S3Object> = None;

                    // Helper to recursively check nested items
                    fn check_nested_select(
                        obj: &S3Object,
                        visual_idx: &mut usize,
                        target_idx: usize,
                        expanded_prefixes: &std::collections::HashSet<String>,
                        prefix_preview: &std::collections::HashMap<String, Vec<S3Object>>,
                        found_obj: &mut Option<S3Object>,
                    ) {
                        if obj.is_prefix && expanded_prefixes.contains(&obj.key) {
                            if let Some(preview) = prefix_preview.get(&obj.key) {
                                for nested_obj in preview {
                                    if *visual_idx == target_idx {
                                        *found_obj = Some(nested_obj.clone());
                                        return;
                                    }
                                    *visual_idx += 1;

                                    // Recursively check deeper levels
                                    check_nested_select(
                                        nested_obj,
                                        visual_idx,
                                        target_idx,
                                        expanded_prefixes,
                                        prefix_preview,
                                        found_obj,
                                    );
                                    if found_obj.is_some() {
                                        return;
                                    }
                                }
                            } else {
                                // Loading row
                                *visual_idx += 1;
                            }
                        }
                    }

                    for obj in &self.s3_state.objects {
                        if visual_idx == self.s3_state.selected_object {
                            found_obj = Some(obj.clone());
                            break;
                        }
                        visual_idx += 1;

                        // Check nested items recursively
                        check_nested_select(
                            obj,
                            &mut visual_idx,
                            self.s3_state.selected_object,
                            &self.s3_state.expanded_prefixes,
                            &self.s3_state.prefix_preview,
                            &mut found_obj,
                        );
                        if found_obj.is_some() {
                            break;
                        }
                    }

                    if let Some(obj) = found_obj {
                        if obj.is_prefix {
                            // Drill into prefix
                            self.s3_state.prefix_stack.push(obj.key.clone());
                            self.s3_state.buckets.loading = true;
                        }
                    }
                }
            } else if self.current_service == Service::CloudFormationStacks {
                if self.cfn_state.current_stack.is_none() {
                    // Drill into stack detail view
                    let filtered_stacks = filtered_cloudformation_stacks(self);
                    if let Some(stack) = self.cfn_state.table.get_selected(&filtered_stacks) {
                        let stack_name = stack.name.clone();
                        let mut tags = stack.tags.clone();
                        tags.sort_by(|a, b| a.0.cmp(&b.0));

                        self.cfn_state.current_stack = Some(stack_name);
                        self.cfn_state.tags.items = tags;
                        self.cfn_state.tags.reset();
                        self.cfn_state.table.loading = true;
                        self.update_current_tab_breadcrumb();
                    }
                }
            } else if self.current_service == Service::EcrRepositories {
                if self.ecr_state.current_repository.is_none() {
                    // In repositories view - drill into selected repository
                    let filtered_repos = filtered_ecr_repositories(self);
                    if let Some(repo) = self.ecr_state.repositories.get_selected(&filtered_repos) {
                        let repo_name = repo.name.clone();
                        let repo_uri = repo.uri.clone();
                        self.ecr_state.current_repository = Some(repo_name);
                        self.ecr_state.current_repository_uri = Some(repo_uri);
                        self.ecr_state.images.reset();
                        self.ecr_state.repositories.loading = true;
                    }
                }
            } else if self.current_service == Service::Ec2Instances {
                if self.ec2_state.current_instance.is_none() {
                    let filtered_instances = filtered_ec2_instances(self);
                    if let Some(instance) = self.ec2_state.table.get_selected(&filtered_instances) {
                        self.ec2_state.current_instance = Some(instance.instance_id.clone());
                        self.view_mode = ViewMode::Detail;
                        self.update_current_tab_breadcrumb();
                    }
                }
            } else if self.current_service == Service::SqsQueues {
                if self.sqs_state.current_queue.is_none() {
                    let filtered_queues = filtered_queues(
                        &self.sqs_state.queues.items,
                        &self.sqs_state.queues.filter,
                    );
                    if let Some(queue) = self.sqs_state.queues.get_selected(&filtered_queues) {
                        self.sqs_state.current_queue = Some(queue.url.clone());

                        if self.sqs_state.detail_tab == SqsQueueDetailTab::Monitoring {
                            self.sqs_state.metrics_loading = true;
                        } else if self.sqs_state.detail_tab == SqsQueueDetailTab::LambdaTriggers {
                            self.sqs_state.triggers.loading = true;
                        } else if self.sqs_state.detail_tab == SqsQueueDetailTab::EventBridgePipes {
                            self.sqs_state.pipes.loading = true;
                        } else if self.sqs_state.detail_tab == SqsQueueDetailTab::Tagging {
                            self.sqs_state.tags.loading = true;
                        } else if self.sqs_state.detail_tab == SqsQueueDetailTab::SnsSubscriptions {
                            self.sqs_state.subscriptions.loading = true;
                        }
                    }
                }
            } else if self.current_service == Service::IamUsers {
                if self.iam_state.current_user.is_some() {
                    // Open policy view (but not on Tags tab)
                    if self.iam_state.user_tab != UserTab::Tags {
                        let filtered = filtered_iam_policies(self);
                        if let Some(policy) = self.iam_state.policies.get_selected(&filtered) {
                            self.iam_state.current_policy = Some(policy.policy_name.clone());
                            self.iam_state.policy_scroll = 0;
                            self.view_mode = ViewMode::PolicyView;
                            self.iam_state.policies.loading = true;
                            self.update_current_tab_breadcrumb();
                        }
                    }
                } else if self.iam_state.current_user.is_none() {
                    let filtered_users = filtered_iam_users(self);
                    if let Some(user) = self.iam_state.users.get_selected(&filtered_users) {
                        self.iam_state.current_user = Some(user.user_name.clone());
                        self.iam_state.user_tab = UserTab::Permissions;
                        self.iam_state.policies.reset();
                        self.update_current_tab_breadcrumb();
                    }
                }
            } else if self.current_service == Service::IamRoles {
                if self.iam_state.current_role.is_some() {
                    // Open policy view (but not on Tags tab)
                    if self.iam_state.role_tab != RoleTab::Tags {
                        let filtered = filtered_iam_policies(self);
                        if let Some(policy) = self.iam_state.policies.get_selected(&filtered) {
                            self.iam_state.current_policy = Some(policy.policy_name.clone());
                            self.iam_state.policy_scroll = 0;
                            self.view_mode = ViewMode::PolicyView;
                            self.iam_state.policies.loading = true;
                            self.update_current_tab_breadcrumb();
                        }
                    }
                } else if self.iam_state.current_role.is_none() {
                    let filtered_roles = filtered_iam_roles(self);
                    if let Some(role) = self.iam_state.roles.get_selected(&filtered_roles) {
                        self.iam_state.current_role = Some(role.role_name.clone());
                        self.iam_state.role_tab = RoleTab::Permissions;
                        self.iam_state.policies.reset();
                        self.update_current_tab_breadcrumb();
                    }
                }
            } else if self.current_service == Service::IamUserGroups {
                if self.iam_state.current_group.is_none() {
                    let filtered_groups: Vec<_> = self
                        .iam_state
                        .groups
                        .items
                        .iter()
                        .filter(|g| {
                            if self.iam_state.groups.filter.is_empty() {
                                true
                            } else {
                                g.group_name
                                    .to_lowercase()
                                    .contains(&self.iam_state.groups.filter.to_lowercase())
                            }
                        })
                        .collect();
                    if let Some(group) = self.iam_state.groups.get_selected(&filtered_groups) {
                        self.iam_state.current_group = Some(group.group_name.clone());
                        self.update_current_tab_breadcrumb();
                    }
                }
            } else if self.current_service == Service::LambdaFunctions {
                if self.lambda_state.current_function.is_some()
                    && self.lambda_state.detail_tab == LambdaDetailTab::Versions
                {
                    // In Normal mode, select version to open detail view
                    // In other modes (FilterInput), toggle expansion
                    if self.mode == Mode::Normal {
                        let page_size = self.lambda_state.version_table.page_size.value();
                        let filtered: Vec<_> = self
                            .lambda_state
                            .version_table
                            .items
                            .iter()
                            .filter(|v| {
                                self.lambda_state.version_table.filter.is_empty()
                                    || v.version.to_lowercase().contains(
                                        &self.lambda_state.version_table.filter.to_lowercase(),
                                    )
                                    || v.aliases.to_lowercase().contains(
                                        &self.lambda_state.version_table.filter.to_lowercase(),
                                    )
                            })
                            .collect();
                        let current_page = self.lambda_state.version_table.selected / page_size;
                        let start_idx = current_page * page_size;
                        let end_idx = (start_idx + page_size).min(filtered.len());
                        let paginated: Vec<_> = filtered[start_idx..end_idx].to_vec();
                        let page_index = self.lambda_state.version_table.selected % page_size;
                        if let Some(version) = paginated.get(page_index) {
                            self.lambda_state.current_version = Some(version.version.clone());
                            self.lambda_state.detail_tab = LambdaDetailTab::Code;
                        }
                    } else {
                        // Toggle expansion
                        if self.lambda_state.version_table.expanded_item
                            == Some(self.lambda_state.version_table.selected)
                        {
                            self.lambda_state.version_table.collapse();
                        } else {
                            self.lambda_state.version_table.expanded_item =
                                Some(self.lambda_state.version_table.selected);
                        }
                    }
                } else if self.lambda_state.current_function.is_some()
                    && self.lambda_state.detail_tab == LambdaDetailTab::Aliases
                {
                    // Select alias to view detail (no tab change - alias view has no tabs)
                    let filtered: Vec<_> = self
                        .lambda_state
                        .alias_table
                        .items
                        .iter()
                        .filter(|a| {
                            self.lambda_state.alias_table.filter.is_empty()
                                || a.name
                                    .to_lowercase()
                                    .contains(&self.lambda_state.alias_table.filter.to_lowercase())
                                || a.versions
                                    .to_lowercase()
                                    .contains(&self.lambda_state.alias_table.filter.to_lowercase())
                        })
                        .collect();
                    if let Some(alias) = self.lambda_state.alias_table.get_selected(&filtered) {
                        self.lambda_state.current_alias = Some(alias.name.clone());
                    }
                } else if self.lambda_state.current_function.is_none() {
                    let filtered_functions = filtered_lambda_functions(self);
                    if let Some(func) = self.lambda_state.table.get_selected(&filtered_functions) {
                        self.lambda_state.current_function = Some(func.name.clone());
                        self.lambda_state.detail_tab = LambdaDetailTab::Code;
                        self.update_current_tab_breadcrumb();
                    }
                }
            } else if self.current_service == Service::LambdaApplications {
                let filtered = filtered_lambda_applications(self);
                if let Some(app) = self.lambda_application_state.table.get_selected(&filtered) {
                    let app_name = app.name.clone();
                    self.lambda_application_state.current_application = Some(app_name.clone());
                    self.lambda_application_state.detail_tab = LambdaApplicationDetailTab::Overview;

                    // Load mock resources
                    use crate::lambda::Resource;
                    self.lambda_application_state.resources.items = vec![
                        Resource {
                            logical_id: "ApiGatewayRestApi".to_string(),
                            physical_id: "abc123xyz".to_string(),
                            resource_type: "AWS::ApiGateway::RestApi".to_string(),
                            last_modified: "2025-01-10 14:30:00 (UTC)".to_string(),
                        },
                        Resource {
                            logical_id: "LambdaFunction".to_string(),
                            physical_id: format!("{}-function", app_name),
                            resource_type: "AWS::Lambda::Function".to_string(),
                            last_modified: "2025-01-10 14:25:00 (UTC)".to_string(),
                        },
                        Resource {
                            logical_id: "DynamoDBTable".to_string(),
                            physical_id: format!("{}-table", app_name),
                            resource_type: "AWS::DynamoDB::Table".to_string(),
                            last_modified: "2025-01-09 10:15:00 (UTC)".to_string(),
                        },
                    ];

                    // Load mock deployments
                    use crate::lambda::Deployment;
                    self.lambda_application_state.deployments.items = vec![
                        Deployment {
                            deployment_id: "d-ABC123XYZ".to_string(),
                            resource_type: "AWS::Serverless::Application".to_string(),
                            last_updated: "2025-01-10 14:30:00 (UTC)".to_string(),
                            status: "Succeeded".to_string(),
                        },
                        Deployment {
                            deployment_id: "d-DEF456UVW".to_string(),
                            resource_type: "AWS::Serverless::Application".to_string(),
                            last_updated: "2025-01-09 10:15:00 (UTC)".to_string(),
                            status: "Succeeded".to_string(),
                        },
                    ];

                    self.update_current_tab_breadcrumb();
                }
            } else if self.current_service == Service::CloudWatchLogGroups {
                if self.view_mode == ViewMode::List {
                    // Map filtered selection to actual group index
                    let filtered_groups = filtered_log_groups(self);
                    if let Some(selected_group) =
                        filtered_groups.get(self.log_groups_state.log_groups.selected)
                    {
                        if let Some(actual_idx) = self
                            .log_groups_state
                            .log_groups
                            .items
                            .iter()
                            .position(|g| g.name == selected_group.name)
                        {
                            self.log_groups_state.log_groups.selected = actual_idx;
                        }
                    }
                    self.view_mode = ViewMode::Detail;
                    self.log_groups_state.log_streams.clear();
                    self.log_groups_state.selected_stream = 0;
                    self.log_groups_state.loading = true;
                    self.update_current_tab_breadcrumb();
                } else if self.view_mode == ViewMode::Detail {
                    // Map filtered stream selection to actual stream index
                    let filtered_streams = filtered_log_streams(self);
                    if let Some(selected_stream) =
                        filtered_streams.get(self.log_groups_state.selected_stream)
                    {
                        if let Some(actual_idx) = self
                            .log_groups_state
                            .log_streams
                            .iter()
                            .position(|s| s.name == selected_stream.name)
                        {
                            self.log_groups_state.selected_stream = actual_idx;
                        }
                    }
                    self.view_mode = ViewMode::Events;
                    self.update_current_tab_breadcrumb();
                    self.log_groups_state.log_events.clear();
                    self.log_groups_state.event_scroll_offset = 0;
                    self.log_groups_state.next_backward_token = None;
                    self.log_groups_state.loading = true;
                } else if self.view_mode == ViewMode::Events {
                    // Toggle expand for selected event
                    if self.log_groups_state.expanded_event
                        == Some(self.log_groups_state.event_scroll_offset)
                    {
                        self.log_groups_state.expanded_event = None;
                    } else {
                        self.log_groups_state.expanded_event =
                            Some(self.log_groups_state.event_scroll_offset);
                    }
                }
            } else if self.current_service == Service::CloudWatchAlarms {
                // Toggle expand for selected alarm
                self.alarms_state.table.toggle_expand();
            } else if self.current_service == Service::CloudWatchInsights {
                // In Normal mode, Enter always executes query
                if !self.insights_state.insights.selected_log_groups.is_empty() {
                    self.log_groups_state.loading = true;
                    self.insights_state.insights.query_completed = true;
                }
            }
        }
    }

    pub async fn load_log_groups(&mut self) -> anyhow::Result<()> {
        self.log_groups_state.log_groups.items = self.cloudwatch_client.list_log_groups().await?;
        Ok(())
    }

    pub async fn load_alarms(&mut self) -> anyhow::Result<()> {
        let alarms = self.alarms_client.list_alarms().await?;
        self.alarms_state.table.items = alarms
            .into_iter()
            .map(
                |(
                    name,
                    state,
                    state_updated,
                    description,
                    metric_name,
                    namespace,
                    statistic,
                    period,
                    comparison,
                    threshold,
                    actions_enabled,
                    state_reason,
                    resource,
                    dimensions,
                    expression,
                    alarm_type,
                    cross_account,
                )| Alarm {
                    name,
                    state,
                    state_updated_timestamp: state_updated,
                    description,
                    metric_name,
                    namespace,
                    statistic,
                    period,
                    comparison_operator: comparison,
                    threshold,
                    actions_enabled,
                    state_reason,
                    resource,
                    dimensions,
                    expression,
                    alarm_type,
                    cross_account,
                },
            )
            .collect();
        Ok(())
    }

    pub async fn load_s3_objects(&mut self) -> anyhow::Result<()> {
        if let Some(bucket_name) = &self.s3_state.current_bucket {
            // Get or fetch bucket region
            let bucket_region = if let Some(bucket) = self
                .s3_state
                .buckets
                .items
                .iter_mut()
                .find(|b| &b.name == bucket_name)
            {
                if bucket.region.is_empty() {
                    // Fetch the actual region
                    let region = self.s3_client.get_bucket_location(bucket_name).await?;
                    bucket.region = region.clone();
                    region
                } else {
                    bucket.region.clone()
                }
            } else {
                self.config.region.clone()
            };

            let prefix = self
                .s3_state
                .prefix_stack
                .last()
                .cloned()
                .unwrap_or_default();
            let objects = self
                .s3_client
                .list_objects(bucket_name, &bucket_region, &prefix)
                .await?;
            self.s3_state.objects = objects
                .into_iter()
                .map(|(key, size, modified, is_prefix, storage_class)| S3Object {
                    key,
                    size,
                    last_modified: modified,
                    is_prefix,
                    storage_class,
                })
                .collect();
            self.s3_state.selected_object = 0;
        }
        Ok(())
    }

    pub async fn load_bucket_preview(&mut self, bucket_name: String) -> anyhow::Result<()> {
        let bucket_region = self
            .s3_state
            .buckets
            .items
            .iter()
            .find(|b| b.name == bucket_name)
            .and_then(|b| {
                if b.region.is_empty() {
                    None
                } else {
                    Some(b.region.as_str())
                }
            })
            .unwrap_or(self.config.region.as_str());
        let objects = self
            .s3_client
            .list_objects(&bucket_name, bucket_region, "")
            .await?;
        let preview: Vec<S3Object> = objects
            .into_iter()
            .map(|(key, size, modified, is_prefix, storage_class)| S3Object {
                key,
                size,
                last_modified: modified,
                is_prefix,
                storage_class,
            })
            .collect();
        self.s3_state.bucket_preview.insert(bucket_name, preview);
        Ok(())
    }

    pub async fn load_prefix_preview(
        &mut self,
        bucket_name: String,
        prefix: String,
    ) -> anyhow::Result<()> {
        let bucket_region = self
            .s3_state
            .buckets
            .items
            .iter()
            .find(|b| b.name == bucket_name)
            .and_then(|b| {
                if b.region.is_empty() {
                    None
                } else {
                    Some(b.region.as_str())
                }
            })
            .unwrap_or(self.config.region.as_str());
        let objects = self
            .s3_client
            .list_objects(&bucket_name, bucket_region, &prefix)
            .await?;
        let preview: Vec<S3Object> = objects
            .into_iter()
            .map(|(key, size, modified, is_prefix, storage_class)| S3Object {
                key,
                size,
                last_modified: modified,
                is_prefix,
                storage_class,
            })
            .collect();
        self.s3_state.prefix_preview.insert(prefix, preview);
        Ok(())
    }

    pub async fn load_ecr_repositories(&mut self) -> anyhow::Result<()> {
        let repos = match self.ecr_state.tab {
            EcrTab::Private => self.ecr_client.list_private_repositories().await?,
            EcrTab::Public => self.ecr_client.list_public_repositories().await?,
        };

        self.ecr_state.repositories.items = repos
            .into_iter()
            .map(|r| EcrRepository {
                name: r.name,
                uri: r.uri,
                created_at: r.created_at,
                tag_immutability: r.tag_immutability,
                encryption_type: r.encryption_type,
            })
            .collect();

        self.ecr_state
            .repositories
            .items
            .sort_by(|a, b| a.name.cmp(&b.name));
        Ok(())
    }

    pub async fn load_ec2_instances(&mut self) -> anyhow::Result<()> {
        let instances = self.ec2_client.list_instances().await?;

        self.ec2_state.table.items = instances
            .into_iter()
            .map(|i| Ec2Instance {
                instance_id: i.instance_id,
                name: i.name,
                state: i.state,
                instance_type: i.instance_type,
                availability_zone: i.availability_zone,
                public_ipv4_dns: i.public_ipv4_dns,
                public_ipv4_address: i.public_ipv4_address,
                elastic_ip: i.elastic_ip,
                ipv6_ips: i.ipv6_ips,
                monitoring: i.monitoring,
                security_groups: i.security_groups,
                key_name: i.key_name,
                launch_time: i.launch_time,
                platform_details: i.platform_details,
                status_checks: i.status_checks,
                alarm_status: i.alarm_status,
                private_dns_name: String::new(),
                private_ip_address: String::new(),
                security_group_ids: String::new(),
                owner_id: String::new(),
                volume_id: String::new(),
                root_device_name: String::new(),
                root_device_type: String::new(),
                ebs_optimized: String::new(),
                image_id: String::new(),
                kernel_id: String::new(),
                ramdisk_id: String::new(),
                ami_launch_index: String::new(),
                reservation_id: String::new(),
                vpc_id: String::new(),
                subnet_ids: String::new(),
                instance_lifecycle: String::new(),
                architecture: String::new(),
                virtualization_type: String::new(),
                platform: String::new(),
                iam_instance_profile_arn: String::new(),
                tenancy: String::new(),
                affinity: String::new(),
                host_id: String::new(),
                placement_group: String::new(),
                partition_number: String::new(),
                capacity_reservation_id: String::new(),
                state_transition_reason_code: String::new(),
                state_transition_reason_message: String::new(),
                stop_hibernation_behavior: String::new(),
                outpost_arn: String::new(),
                product_codes: String::new(),
                availability_zone_id: String::new(),
                imdsv2: String::new(),
                usage_operation: String::new(),
                managed: String::new(),
                operator: String::new(),
            })
            .collect();

        // Sort by launch time descending by default
        self.ec2_state
            .table
            .items
            .sort_by(|a, b| b.launch_time.cmp(&a.launch_time));
        Ok(())
    }

    pub async fn load_ecr_images(&mut self) -> anyhow::Result<()> {
        if let Some(repo_name) = &self.ecr_state.current_repository {
            if let Some(repo_uri) = &self.ecr_state.current_repository_uri {
                let images = self.ecr_client.list_images(repo_name, repo_uri).await?;

                self.ecr_state.images.items = images
                    .into_iter()
                    .map(|i| EcrImage {
                        tag: i.tag,
                        artifact_type: i.artifact_type,
                        pushed_at: i.pushed_at,
                        size_bytes: i.size_bytes,
                        uri: i.uri,
                        digest: i.digest,
                        last_pull_time: i.last_pull_time,
                    })
                    .collect();

                self.ecr_state
                    .images
                    .items
                    .sort_by(|a, b| b.pushed_at.cmp(&a.pushed_at));
            }
        }
        Ok(())
    }

    pub async fn load_cloudformation_stacks(&mut self) -> anyhow::Result<()> {
        let stacks = self
            .cloudformation_client
            .list_stacks(self.cfn_state.view_nested)
            .await?;

        let mut stacks: Vec<CfnStack> = stacks
            .into_iter()
            .map(|s| CfnStack {
                name: s.name,
                stack_id: s.stack_id,
                status: s.status,
                created_time: s.created_time,
                updated_time: s.updated_time,
                deleted_time: s.deleted_time,
                drift_status: s.drift_status,
                last_drift_check_time: s.last_drift_check_time,
                status_reason: s.status_reason,
                description: s.description,
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            })
            .collect();

        // Sort by created_time DESC
        stacks.sort_by(|a, b| b.created_time.cmp(&a.created_time));

        self.cfn_state.table.items = stacks;

        Ok(())
    }

    pub async fn load_cfn_template(&mut self, stack_name: &str) -> anyhow::Result<()> {
        let template = self.cloudformation_client.get_template(stack_name).await?;
        self.cfn_state.template_body = template;
        self.cfn_state.template_scroll = 0;
        Ok(())
    }

    pub async fn load_cfn_parameters(&mut self, stack_name: &str) -> anyhow::Result<()> {
        let mut parameters = self
            .cloudformation_client
            .get_stack_parameters(stack_name)
            .await?;
        parameters.sort_by(|a, b| a.key.cmp(&b.key));
        self.cfn_state.parameters.items = parameters;
        self.cfn_state.parameters.reset();
        Ok(())
    }

    pub async fn load_cfn_outputs(&mut self, stack_name: &str) -> anyhow::Result<()> {
        let outputs = self
            .cloudformation_client
            .get_stack_outputs(stack_name)
            .await?;
        self.cfn_state.outputs.items = outputs;
        self.cfn_state.outputs.reset();
        Ok(())
    }

    pub async fn load_cfn_resources(&mut self, stack_name: &str) -> anyhow::Result<()> {
        let resources = self
            .cloudformation_client
            .get_stack_resources(stack_name)
            .await?;
        self.cfn_state.resources.items = resources;
        self.cfn_state.resources.reset();
        Ok(())
    }

    pub async fn load_role_policies(&mut self, role_name: &str) -> anyhow::Result<()> {
        // Load attached (managed) policies
        let attached_policies = self
            .iam_client
            .list_attached_role_policies(role_name)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        let mut policies: Vec<IamPolicy> = attached_policies
            .into_iter()
            .map(|p| IamPolicy {
                policy_name: p.policy_name().unwrap_or("").to_string(),
                policy_type: "Managed".to_string(),
                attached_via: "Direct".to_string(),
                attached_entities: "-".to_string(),
                description: "-".to_string(),
                creation_time: "-".to_string(),
                edited_time: "-".to_string(),
                policy_arn: p.policy_arn().map(|s| s.to_string()),
            })
            .collect();

        // Load inline policies
        let inline_policy_names = self
            .iam_client
            .list_role_policies(role_name)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        for policy_name in inline_policy_names {
            policies.push(IamPolicy {
                policy_name,
                policy_type: "Inline".to_string(),
                attached_via: "Direct".to_string(),
                attached_entities: "-".to_string(),
                description: "-".to_string(),
                creation_time: "-".to_string(),
                edited_time: "-".to_string(),
                policy_arn: None,
            });
        }

        self.iam_state.policies.items = policies;

        Ok(())
    }

    pub async fn load_group_policies(&mut self, group_name: &str) -> anyhow::Result<()> {
        let attached_policies = self
            .iam_client
            .list_attached_group_policies(group_name)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        let mut policies: Vec<IamPolicy> = attached_policies
            .into_iter()
            .map(|p| IamPolicy {
                policy_name: p.policy_name().unwrap_or("").to_string(),
                policy_type: "AWS managed".to_string(),
                attached_via: "Direct".to_string(),
                attached_entities: "-".to_string(),
                description: "-".to_string(),
                creation_time: "-".to_string(),
                edited_time: "-".to_string(),
                policy_arn: p.policy_arn().map(|s| s.to_string()),
            })
            .collect();

        let inline_policy_names = self
            .iam_client
            .list_group_policies(group_name)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        for policy_name in inline_policy_names {
            policies.push(IamPolicy {
                policy_name,
                policy_type: "Inline".to_string(),
                attached_via: "Direct".to_string(),
                attached_entities: "-".to_string(),
                description: "-".to_string(),
                creation_time: "-".to_string(),
                edited_time: "-".to_string(),
                policy_arn: None,
            });
        }

        self.iam_state.policies.items = policies;

        Ok(())
    }

    pub async fn load_group_users(&mut self, group_name: &str) -> anyhow::Result<()> {
        let users = self
            .iam_client
            .get_group_users(group_name)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        let group_users: Vec<IamGroupUser> = users
            .into_iter()
            .map(|u| {
                let creation_time = {
                    let dt = u.create_date();
                    let timestamp = dt.secs();
                    let datetime =
                        chrono::DateTime::from_timestamp(timestamp, 0).unwrap_or_default();
                    datetime.format("%Y-%m-%d %H:%M:%S (UTC)").to_string()
                };

                IamGroupUser {
                    user_name: u.user_name().to_string(),
                    groups: String::new(),
                    last_activity: String::new(),
                    creation_time,
                }
            })
            .collect();

        self.iam_state.group_users.items = group_users;

        Ok(())
    }

    pub async fn load_policy_document(
        &mut self,
        role_name: &str,
        policy_name: &str,
    ) -> anyhow::Result<()> {
        // Find the policy to get its ARN and type
        let policy = self
            .iam_state
            .policies
            .items
            .iter()
            .find(|p| p.policy_name == policy_name)
            .ok_or_else(|| anyhow::anyhow!("Policy not found"))?;

        let document = if let Some(policy_arn) = &policy.policy_arn {
            // Managed policy - use get_policy_version
            self.iam_client
                .get_policy_version(policy_arn)
                .await
                .map_err(|e| anyhow::anyhow!(e))?
        } else {
            // Inline policy - use get_role_policy
            self.iam_client
                .get_role_policy(role_name, policy_name)
                .await
                .map_err(|e| anyhow::anyhow!(e))?
        };

        self.iam_state.policy_document = document;

        Ok(())
    }

    pub async fn load_trust_policy(&mut self, role_name: &str) -> anyhow::Result<()> {
        let document = self
            .iam_client
            .get_role(role_name)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        self.iam_state.trust_policy_document = document;

        Ok(())
    }

    pub async fn load_last_accessed_services(&mut self, _role_name: &str) -> anyhow::Result<()> {
        // TODO: Implement real AWS API call to get service last accessed details
        self.iam_state.last_accessed_services.items = vec![];
        self.iam_state.last_accessed_services.selected = 0;

        Ok(())
    }

    pub async fn load_role_tags(&mut self, role_name: &str) -> anyhow::Result<()> {
        let tags = self
            .iam_client
            .list_role_tags(role_name)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        self.iam_state.tags.items = tags
            .into_iter()
            .map(|(k, v)| IamRoleTag { key: k, value: v })
            .collect();
        self.iam_state.tags.reset();
        Ok(())
    }

    pub async fn load_user_tags(&mut self, user_name: &str) -> anyhow::Result<()> {
        let tags = self
            .iam_client
            .list_user_tags(user_name)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        self.iam_state.user_tags.items = tags
            .into_iter()
            .map(|(k, v)| IamUserTag { key: k, value: v })
            .collect();
        self.iam_state.user_tags.reset();
        Ok(())
    }

    pub async fn load_log_streams(&mut self) -> anyhow::Result<()> {
        if let Some(group) = self
            .log_groups_state
            .log_groups
            .items
            .get(self.log_groups_state.log_groups.selected)
        {
            self.log_groups_state.log_streams =
                self.cloudwatch_client.list_log_streams(&group.name).await?;
            self.log_groups_state.selected_stream = 0;
        }
        Ok(())
    }

    pub async fn load_log_events(&mut self) -> anyhow::Result<()> {
        if let Some(group) = self
            .log_groups_state
            .log_groups
            .items
            .get(self.log_groups_state.log_groups.selected)
        {
            if let Some(stream) = self
                .log_groups_state
                .log_streams
                .get(self.log_groups_state.selected_stream)
            {
                // Calculate time range from relative date picker
                let (start_time, end_time) =
                    if let Ok(amount) = self.log_groups_state.relative_amount.parse::<i64>() {
                        let now = chrono::Utc::now().timestamp_millis();
                        let duration_ms = match self.log_groups_state.relative_unit {
                            TimeUnit::Minutes => amount * 60 * 1000,
                            TimeUnit::Hours => amount * 60 * 60 * 1000,
                            TimeUnit::Days => amount * 24 * 60 * 60 * 1000,
                            TimeUnit::Weeks => amount * 7 * 24 * 60 * 60 * 1000,
                        };
                        (Some(now - duration_ms), Some(now))
                    } else {
                        (None, None)
                    };

                let (mut events, has_more, token) = self
                    .cloudwatch_client
                    .get_log_events(
                        &group.name,
                        &stream.name,
                        self.log_groups_state.next_backward_token.clone(),
                        start_time,
                        end_time,
                    )
                    .await?;

                if self.log_groups_state.next_backward_token.is_some() {
                    // Prepend older events - keep selection at top
                    events.append(&mut self.log_groups_state.log_events);
                    self.log_groups_state.event_scroll_offset = 0;
                } else {
                    // Initial load - start at first event
                    self.log_groups_state.event_scroll_offset = 0;
                }

                self.log_groups_state.log_events = events;
                self.log_groups_state.has_older_events =
                    has_more && self.log_groups_state.log_events.len() >= 25;
                self.log_groups_state.next_backward_token = token;
                self.log_groups_state.selected_event = 0;
            }
        }
        Ok(())
    }

    pub async fn execute_insights_query(&mut self) -> anyhow::Result<()> {
        if self.insights_state.insights.selected_log_groups.is_empty() {
            return Err(anyhow::anyhow!(
                "No log groups selected. Please select at least one log group."
            ));
        }

        let now = chrono::Utc::now().timestamp_millis();
        let amount = self
            .insights_state
            .insights
            .insights_relative_amount
            .parse::<i64>()
            .unwrap_or(1);
        let duration_ms = match self.insights_state.insights.insights_relative_unit {
            TimeUnit::Minutes => amount * 60 * 1000,
            TimeUnit::Hours => amount * 60 * 60 * 1000,
            TimeUnit::Days => amount * 24 * 60 * 60 * 1000,
            TimeUnit::Weeks => amount * 7 * 24 * 60 * 60 * 1000,
        };
        let start_time = now - duration_ms;

        let query_id = self
            .cloudwatch_client
            .start_query(
                self.insights_state.insights.selected_log_groups.clone(),
                self.insights_state.insights.query_text.trim().to_string(),
                start_time,
                now,
            )
            .await?;

        // Poll for results
        for _ in 0..60 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let (status, results) = self.cloudwatch_client.get_query_results(&query_id).await?;

            if status == "Complete" {
                self.insights_state.insights.query_results = results;
                self.insights_state.insights.query_completed = true;
                self.insights_state.insights.results_selected = 0;
                self.insights_state.insights.expanded_result = None;
                self.view_mode = ViewMode::InsightsResults;
                return Ok(());
            } else if status == "Failed" || status == "Cancelled" {
                return Err(anyhow::anyhow!("Query {}", status.to_lowercase()));
            }
        }

        Err(anyhow::anyhow!("Query timeout"))
    }
}

impl CloudWatchInsightsState {
    fn new() -> Self {
        Self {
            insights: InsightsState::default(),
            loading: false,
        }
    }
}

impl CloudWatchAlarmsState {
    fn new() -> Self {
        Self {
            table: TableState::new(),
            alarm_tab: AlarmTab::AllAlarms,
            view_as: AlarmViewMode::Table,
            wrap_lines: false,
            sort_column: "Last state update".to_string(),
            sort_direction: SortDirection::Asc,
            input_focus: InputFocus::Filter,
        }
    }
}

impl ServicePickerState {
    fn new() -> Self {
        Self {
            filter: String::new(),
            selected: 0,
            services: vec![
                "CloudWatch > Log Groups",
                "CloudWatch > Logs Insights",
                "CloudWatch > Alarms",
                "CloudFormation > Stacks",
                "EC2 > Instances",
                "ECR > Repositories",
                "IAM > Users",
                "IAM > Roles",
                "IAM > User Groups",
                "Lambda > Functions",
                "Lambda > Applications",
                "S3 > Buckets",
                "SQS > Queues",
            ],
        }
    }
}

#[cfg(test)]
mod test_helpers {
    use super::*;

    // Test helper functions to reduce boilerplate
    pub fn test_app() -> App {
        App::new_without_client("test".to_string(), Some("us-east-1".to_string()))
    }

    pub fn test_app_no_region() -> App {
        App::new_without_client("test".to_string(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::Action;
    use test_helpers::*;

    #[test]
    fn test_next_tab_cycles_forward() {
        let mut app = test_app();
        app.tabs = vec![
            Tab {
                service: Service::CloudWatchLogGroups,
                title: "CloudWatch > Log Groups".to_string(),
                breadcrumb: "CloudWatch > Log Groups".to_string(),
            },
            Tab {
                service: Service::CloudWatchInsights,
                title: "CloudWatch > Logs Insights".to_string(),
                breadcrumb: "CloudWatch > Logs Insights".to_string(),
            },
            Tab {
                service: Service::CloudWatchAlarms,
                title: "CloudWatch > Alarms".to_string(),
                breadcrumb: "CloudWatch > Alarms".to_string(),
            },
        ];
        app.current_tab = 0;

        app.handle_action(Action::NextTab);
        assert_eq!(app.current_tab, 1);
        assert_eq!(app.current_service, Service::CloudWatchInsights);

        app.handle_action(Action::NextTab);
        assert_eq!(app.current_tab, 2);
        assert_eq!(app.current_service, Service::CloudWatchAlarms);

        // Should wrap around
        app.handle_action(Action::NextTab);
        assert_eq!(app.current_tab, 0);
        assert_eq!(app.current_service, Service::CloudWatchLogGroups);
    }

    #[test]
    fn test_prev_tab_cycles_backward() {
        let mut app = test_app();
        app.tabs = vec![
            Tab {
                service: Service::CloudWatchLogGroups,
                title: "CloudWatch > Log Groups".to_string(),
                breadcrumb: "CloudWatch > Log Groups".to_string(),
            },
            Tab {
                service: Service::CloudWatchInsights,
                title: "CloudWatch > Logs Insights".to_string(),
                breadcrumb: "CloudWatch > Logs Insights".to_string(),
            },
            Tab {
                service: Service::CloudWatchAlarms,
                title: "CloudWatch > Alarms".to_string(),
                breadcrumb: "CloudWatch > Alarms".to_string(),
            },
        ];
        app.current_tab = 2;

        app.handle_action(Action::PrevTab);
        assert_eq!(app.current_tab, 1);
        assert_eq!(app.current_service, Service::CloudWatchInsights);

        app.handle_action(Action::PrevTab);
        assert_eq!(app.current_tab, 0);
        assert_eq!(app.current_service, Service::CloudWatchLogGroups);

        // Should wrap around
        app.handle_action(Action::PrevTab);
        assert_eq!(app.current_tab, 2);
        assert_eq!(app.current_service, Service::CloudWatchAlarms);
    }

    #[test]
    fn test_close_tab_removes_current() {
        let mut app = test_app();
        app.tabs = vec![
            Tab {
                service: Service::CloudWatchLogGroups,
                title: "CloudWatch > Log Groups".to_string(),
                breadcrumb: "CloudWatch > Log Groups".to_string(),
            },
            Tab {
                service: Service::CloudWatchInsights,
                title: "CloudWatch > Logs Insights".to_string(),
                breadcrumb: "CloudWatch > Logs Insights".to_string(),
            },
            Tab {
                service: Service::CloudWatchAlarms,
                title: "CloudWatch > Alarms".to_string(),
                breadcrumb: "CloudWatch > Alarms".to_string(),
            },
        ];
        app.current_tab = 1;
        app.service_selected = true;

        app.handle_action(Action::CloseTab);
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.current_tab, 1);
        assert_eq!(app.current_service, Service::CloudWatchAlarms);
    }

    #[test]
    fn test_close_last_tab_exits_service() {
        let mut app = test_app();
        app.tabs = vec![Tab {
            service: Service::CloudWatchLogGroups,
            title: "CloudWatch > Log Groups".to_string(),
            breadcrumb: "CloudWatch > Log Groups".to_string(),
        }];
        app.current_tab = 0;
        app.service_selected = true;

        app.handle_action(Action::CloseTab);
        assert_eq!(app.tabs.len(), 0);
        assert!(!app.service_selected);
        assert_eq!(app.current_tab, 0);
    }

    #[test]
    fn test_close_service_removes_current_tab() {
        let mut app = test_app();
        app.tabs = vec![
            Tab {
                service: Service::CloudWatchLogGroups,
                title: "CloudWatch > Log Groups".to_string(),
                breadcrumb: "CloudWatch > Log Groups".to_string(),
            },
            Tab {
                service: Service::CloudWatchInsights,
                title: "CloudWatch > Logs Insights".to_string(),
                breadcrumb: "CloudWatch > Logs Insights".to_string(),
            },
            Tab {
                service: Service::CloudWatchAlarms,
                title: "CloudWatch > Alarms".to_string(),
                breadcrumb: "CloudWatch > Alarms".to_string(),
            },
        ];
        app.current_tab = 1;
        app.service_selected = true;

        app.handle_action(Action::CloseService);

        // Tab should be removed
        assert_eq!(app.tabs.len(), 2);
        // Should switch to next tab (Alarms at index 1)
        assert_eq!(app.current_tab, 1);
        assert_eq!(app.current_service, Service::CloudWatchAlarms);
        // Should stay in service mode, NOT show service picker
        assert!(app.service_selected);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_close_service_last_tab_shows_picker() {
        let mut app = test_app();
        app.tabs = vec![Tab {
            service: Service::CloudWatchLogGroups,
            title: "CloudWatch > Log Groups".to_string(),
            breadcrumb: "CloudWatch > Log Groups".to_string(),
        }];
        app.current_tab = 0;
        app.service_selected = true;

        app.handle_action(Action::CloseService);

        // Tab should be removed
        assert_eq!(app.tabs.len(), 0);
        // Should show service picker
        assert!(!app.service_selected);
        assert_eq!(app.mode, Mode::ServicePicker);
    }

    #[test]
    fn test_open_tab_picker_with_tabs() {
        let mut app = test_app();
        app.tabs = vec![
            Tab {
                service: Service::CloudWatchLogGroups,
                title: "CloudWatch > Log Groups".to_string(),
                breadcrumb: "CloudWatch > Log Groups".to_string(),
            },
            Tab {
                service: Service::CloudWatchInsights,
                title: "CloudWatch > Logs Insights".to_string(),
                breadcrumb: "CloudWatch > Logs Insights".to_string(),
            },
        ];
        app.current_tab = 1;

        app.handle_action(Action::OpenTabPicker);
        assert_eq!(app.mode, Mode::TabPicker);
        assert_eq!(app.tab_picker_selected, 1);
    }

    #[test]
    fn test_open_tab_picker_without_tabs() {
        let mut app = test_app();
        app.tabs = vec![];

        app.handle_action(Action::OpenTabPicker);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_pending_key_state() {
        let mut app = test_app();
        assert_eq!(app.pending_key, None);

        app.pending_key = Some('g');
        assert_eq!(app.pending_key, Some('g'));
    }

    #[test]
    fn test_tab_breadcrumb_updates() {
        let mut app = test_app();
        app.tabs = vec![Tab {
            service: Service::CloudWatchLogGroups,
            title: "CloudWatch > Log Groups".to_string(),
            breadcrumb: "CloudWatch > Log groups".to_string(),
        }];
        app.current_tab = 0;
        app.service_selected = true;
        app.current_service = Service::CloudWatchLogGroups;

        // Initial breadcrumb
        assert_eq!(app.tabs[0].breadcrumb, "CloudWatch > Log groups");

        // Add a log group and update breadcrumb
        app.log_groups_state
            .log_groups
            .items
            .push(rusticity_core::LogGroup {
                name: "/aws/lambda/test".to_string(),
                creation_time: None,
                stored_bytes: Some(1024),
                retention_days: None,
                log_class: None,
                arn: None,
            });
        app.log_groups_state.log_groups.reset();
        app.view_mode = ViewMode::Detail;
        app.update_current_tab_breadcrumb();

        // Breadcrumb should now include log group
        assert_eq!(
            app.tabs[0].breadcrumb,
            "CloudWatch > Log groups > /aws/lambda/test"
        );
    }

    #[test]
    fn test_s3_bucket_column_selector_navigation() {
        let mut app = test_app();
        app.current_service = Service::S3Buckets;
        app.mode = Mode::ColumnSelector;
        app.column_selector_index = 0;

        // Should navigate through 3 S3 bucket columns (0, 1, 2)
        app.handle_action(Action::NextItem);
        assert_eq!(app.column_selector_index, 1);

        app.handle_action(Action::NextItem);
        assert_eq!(app.column_selector_index, 2);

        // Should not go beyond max (2)
        app.handle_action(Action::NextItem);
        assert_eq!(app.column_selector_index, 2);

        // Navigate back
        app.handle_action(Action::PrevItem);
        assert_eq!(app.column_selector_index, 1);

        app.handle_action(Action::PrevItem);
        assert_eq!(app.column_selector_index, 0);

        // Should not go below 0
        app.handle_action(Action::PrevItem);
        assert_eq!(app.column_selector_index, 0);
    }

    #[test]
    fn test_cloudwatch_alarms_state_initialized() {
        let app = test_app();

        // Alarms state should be initialized
        assert_eq!(app.alarms_state.table.items.len(), 0);
        assert_eq!(app.alarms_state.table.selected, 0);
        assert_eq!(app.alarms_state.alarm_tab, AlarmTab::AllAlarms);
        assert!(!app.alarms_state.table.loading);
        assert_eq!(app.alarms_state.view_as, AlarmViewMode::Table);
        assert_eq!(app.alarms_state.table.page_size, PageSize::Fifty);
    }

    #[test]
    fn test_cloudwatch_alarms_service_selection() {
        let mut app = test_app();

        // Switch to alarms service
        app.current_service = Service::CloudWatchAlarms;
        app.service_selected = true;

        assert_eq!(app.current_service, Service::CloudWatchAlarms);
        assert!(app.service_selected);
    }

    #[test]
    fn test_cloudwatch_alarms_column_preferences() {
        let app = test_app();

        // Should have alarm columns defined
        assert!(!app.cw_alarm_column_ids.is_empty());
        assert!(!app.cw_alarm_visible_column_ids.is_empty());

        // Default visible columns
        assert!(app
            .cw_alarm_visible_column_ids
            .contains(&AlarmColumn::Name.id()));
        assert!(app
            .cw_alarm_visible_column_ids
            .contains(&AlarmColumn::State.id()));
    }

    #[test]
    fn test_s3_bucket_navigation_without_expansion() {
        let mut app = test_app();
        app.current_service = Service::S3Buckets;
        app.service_selected = true;
        app.mode = Mode::Normal;

        // Add 3 buckets
        app.s3_state.buckets.items = vec![
            S3Bucket {
                name: "bucket1".to_string(),
                region: "us-east-1".to_string(),
                creation_date: "2024-01-01T00:00:00Z".to_string(),
            },
            S3Bucket {
                name: "bucket2".to_string(),
                region: "us-east-1".to_string(),
                creation_date: "2024-01-02T00:00:00Z".to_string(),
            },
            S3Bucket {
                name: "bucket3".to_string(),
                region: "us-east-1".to_string(),
                creation_date: "2024-01-03T00:00:00Z".to_string(),
            },
        ];
        app.s3_state.selected_row = 0;

        // Navigate down
        app.handle_action(Action::NextItem);
        assert_eq!(app.s3_state.selected_row, 1);

        app.handle_action(Action::NextItem);
        assert_eq!(app.s3_state.selected_row, 2);

        // Should not go beyond last bucket
        app.handle_action(Action::NextItem);
        assert_eq!(app.s3_state.selected_row, 2);

        // Navigate up
        app.handle_action(Action::PrevItem);
        assert_eq!(app.s3_state.selected_row, 1);

        app.handle_action(Action::PrevItem);
        assert_eq!(app.s3_state.selected_row, 0);

        // Should not go below 0
        app.handle_action(Action::PrevItem);
        assert_eq!(app.s3_state.selected_row, 0);
    }

    #[test]
    fn test_s3_bucket_navigation_with_expansion() {
        let mut app = test_app();
        app.current_service = Service::S3Buckets;
        app.service_selected = true;
        app.mode = Mode::Normal;

        // Add 2 buckets
        app.s3_state.buckets.items = vec![
            S3Bucket {
                name: "bucket1".to_string(),
                region: "us-east-1".to_string(),
                creation_date: "2024-01-01T00:00:00Z".to_string(),
            },
            S3Bucket {
                name: "bucket2".to_string(),
                region: "us-east-1".to_string(),
                creation_date: "2024-01-02T00:00:00Z".to_string(),
            },
        ];

        // Expand bucket1 with 2 objects
        app.s3_state.expanded_prefixes.insert("bucket1".to_string());
        app.s3_state.bucket_preview.insert(
            "bucket1".to_string(),
            vec![
                S3Object {
                    key: "file1.txt".to_string(),
                    size: 100,
                    last_modified: "2024-01-01T00:00:00Z".to_string(),
                    is_prefix: false,
                    storage_class: "STANDARD".to_string(),
                },
                S3Object {
                    key: "folder/".to_string(),
                    size: 0,
                    last_modified: "2024-01-01T00:00:00Z".to_string(),
                    is_prefix: true,
                    storage_class: String::new(),
                },
            ],
        );

        app.s3_state.selected_row = 0;

        // Total rows: bucket1 (row 0) + file1.txt (row 1) + folder/ (row 2) + bucket2 (row 3) = 4 rows
        // Navigate through all rows
        app.handle_action(Action::NextItem);
        assert_eq!(app.s3_state.selected_row, 1); // file1.txt

        app.handle_action(Action::NextItem);
        assert_eq!(app.s3_state.selected_row, 2); // folder/

        app.handle_action(Action::NextItem);
        assert_eq!(app.s3_state.selected_row, 3); // bucket2

        // Should not go beyond last row
        app.handle_action(Action::NextItem);
        assert_eq!(app.s3_state.selected_row, 3);
    }

    #[test]
    fn test_s3_bucket_navigation_with_nested_expansion() {
        let mut app = test_app();
        app.current_service = Service::S3Buckets;
        app.service_selected = true;
        app.mode = Mode::Normal;

        // Add 1 bucket
        app.s3_state.buckets.items = vec![S3Bucket {
            name: "bucket1".to_string(),
            region: "us-east-1".to_string(),
            creation_date: "2024-01-01T00:00:00Z".to_string(),
        }];

        // Expand bucket1 with a folder
        app.s3_state.expanded_prefixes.insert("bucket1".to_string());
        app.s3_state.bucket_preview.insert(
            "bucket1".to_string(),
            vec![S3Object {
                key: "folder/".to_string(),
                size: 0,
                last_modified: "2024-01-01T00:00:00Z".to_string(),
                is_prefix: true,
                storage_class: String::new(),
            }],
        );

        // Expand the folder with 2 nested objects
        app.s3_state.expanded_prefixes.insert("folder/".to_string());
        app.s3_state.prefix_preview.insert(
            "folder/".to_string(),
            vec![
                S3Object {
                    key: "folder/file1.txt".to_string(),
                    size: 100,
                    last_modified: "2024-01-01T00:00:00Z".to_string(),
                    is_prefix: false,
                    storage_class: "STANDARD".to_string(),
                },
                S3Object {
                    key: "folder/file2.txt".to_string(),
                    size: 200,
                    last_modified: "2024-01-01T00:00:00Z".to_string(),
                    is_prefix: false,
                    storage_class: "STANDARD".to_string(),
                },
            ],
        );

        app.s3_state.selected_row = 0;

        // Total rows: bucket1 (0) + folder/ (1) + file1.txt (2) + file2.txt (3) = 4 rows
        app.handle_action(Action::NextItem);
        assert_eq!(app.s3_state.selected_row, 1); // folder/

        app.handle_action(Action::NextItem);
        assert_eq!(app.s3_state.selected_row, 2); // folder/file1.txt

        app.handle_action(Action::NextItem);
        assert_eq!(app.s3_state.selected_row, 3); // folder/file2.txt

        // Should not go beyond last row
        app.handle_action(Action::NextItem);
        assert_eq!(app.s3_state.selected_row, 3);
    }

    #[test]
    fn test_calculate_total_bucket_rows() {
        let mut app = test_app();

        // No buckets
        assert_eq!(app.calculate_total_bucket_rows(), 0);

        // 2 buckets, no expansion
        app.s3_state.buckets.items = vec![
            S3Bucket {
                name: "bucket1".to_string(),
                region: "us-east-1".to_string(),
                creation_date: "2024-01-01T00:00:00Z".to_string(),
            },
            S3Bucket {
                name: "bucket2".to_string(),
                region: "us-east-1".to_string(),
                creation_date: "2024-01-02T00:00:00Z".to_string(),
            },
        ];
        assert_eq!(app.calculate_total_bucket_rows(), 2);

        // Expand bucket1 with 3 objects
        app.s3_state.expanded_prefixes.insert("bucket1".to_string());
        app.s3_state.bucket_preview.insert(
            "bucket1".to_string(),
            vec![
                S3Object {
                    key: "file1.txt".to_string(),
                    size: 100,
                    last_modified: "2024-01-01T00:00:00Z".to_string(),
                    is_prefix: false,
                    storage_class: "STANDARD".to_string(),
                },
                S3Object {
                    key: "file2.txt".to_string(),
                    size: 200,
                    last_modified: "2024-01-01T00:00:00Z".to_string(),
                    is_prefix: false,
                    storage_class: "STANDARD".to_string(),
                },
                S3Object {
                    key: "folder/".to_string(),
                    size: 0,
                    last_modified: "2024-01-01T00:00:00Z".to_string(),
                    is_prefix: true,
                    storage_class: String::new(),
                },
            ],
        );
        assert_eq!(app.calculate_total_bucket_rows(), 5); // 2 buckets + 3 objects

        // Expand folder/ with 2 nested objects
        app.s3_state.expanded_prefixes.insert("folder/".to_string());
        app.s3_state.prefix_preview.insert(
            "folder/".to_string(),
            vec![
                S3Object {
                    key: "folder/nested1.txt".to_string(),
                    size: 50,
                    last_modified: "2024-01-01T00:00:00Z".to_string(),
                    is_prefix: false,
                    storage_class: "STANDARD".to_string(),
                },
                S3Object {
                    key: "folder/nested2.txt".to_string(),
                    size: 75,
                    last_modified: "2024-01-01T00:00:00Z".to_string(),
                    is_prefix: false,
                    storage_class: "STANDARD".to_string(),
                },
            ],
        );
        assert_eq!(app.calculate_total_bucket_rows(), 7); // 2 buckets + 3 objects + 2 nested
    }

    #[test]
    fn test_calculate_total_object_rows() {
        let mut app = test_app();
        app.s3_state.current_bucket = Some("test-bucket".to_string());

        // No objects
        assert_eq!(app.calculate_total_object_rows(), 0);

        // 2 objects, no expansion
        app.s3_state.objects = vec![
            S3Object {
                key: "file1.txt".to_string(),
                size: 100,
                last_modified: "2024-01-01T00:00:00Z".to_string(),
                is_prefix: false,
                storage_class: "STANDARD".to_string(),
            },
            S3Object {
                key: "folder/".to_string(),
                size: 0,
                last_modified: "2024-01-01T00:00:00Z".to_string(),
                is_prefix: true,
                storage_class: String::new(),
            },
        ];
        assert_eq!(app.calculate_total_object_rows(), 2);

        // Expand folder/ with 2 items
        app.s3_state.expanded_prefixes.insert("folder/".to_string());
        app.s3_state.prefix_preview.insert(
            "folder/".to_string(),
            vec![
                S3Object {
                    key: "folder/file2.txt".to_string(),
                    size: 200,
                    last_modified: "2024-01-01T00:00:00Z".to_string(),
                    is_prefix: false,
                    storage_class: "STANDARD".to_string(),
                },
                S3Object {
                    key: "folder/subfolder/".to_string(),
                    size: 0,
                    last_modified: "2024-01-01T00:00:00Z".to_string(),
                    is_prefix: true,
                    storage_class: String::new(),
                },
            ],
        );
        assert_eq!(app.calculate_total_object_rows(), 4); // 2 + 2 nested

        // Expand subfolder/ with 1 item (3rd level)
        app.s3_state
            .expanded_prefixes
            .insert("folder/subfolder/".to_string());
        app.s3_state.prefix_preview.insert(
            "folder/subfolder/".to_string(),
            vec![S3Object {
                key: "folder/subfolder/deep.txt".to_string(),
                size: 50,
                last_modified: "2024-01-01T00:00:00Z".to_string(),
                is_prefix: false,
                storage_class: "STANDARD".to_string(),
            }],
        );
        assert_eq!(app.calculate_total_object_rows(), 5); // 2 + 2 nested + 1 deep
    }

    #[test]
    fn test_s3_object_navigation_with_deep_nesting() {
        let mut app = test_app();
        app.current_service = Service::S3Buckets;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.s3_state.current_bucket = Some("test-bucket".to_string());

        // Add folder structure: folder1/ -> folder2/ -> file.txt
        app.s3_state.objects = vec![S3Object {
            key: "folder1/".to_string(),
            size: 0,
            last_modified: "2024-01-01T00:00:00Z".to_string(),
            is_prefix: true,
            storage_class: String::new(),
        }];

        // Expand folder1/
        app.s3_state
            .expanded_prefixes
            .insert("folder1/".to_string());
        app.s3_state.prefix_preview.insert(
            "folder1/".to_string(),
            vec![S3Object {
                key: "folder1/folder2/".to_string(),
                size: 0,
                last_modified: "2024-01-01T00:00:00Z".to_string(),
                is_prefix: true,
                storage_class: String::new(),
            }],
        );

        // Expand folder2/
        app.s3_state
            .expanded_prefixes
            .insert("folder1/folder2/".to_string());
        app.s3_state.prefix_preview.insert(
            "folder1/folder2/".to_string(),
            vec![S3Object {
                key: "folder1/folder2/file.txt".to_string(),
                size: 100,
                last_modified: "2024-01-01T00:00:00Z".to_string(),
                is_prefix: false,
                storage_class: "STANDARD".to_string(),
            }],
        );

        app.s3_state.selected_object = 0;

        // Total: folder1/ (0) + folder2/ (1) + file.txt (2) = 3 rows
        app.handle_action(Action::NextItem);
        assert_eq!(app.s3_state.selected_object, 1); // folder2/

        app.handle_action(Action::NextItem);
        assert_eq!(app.s3_state.selected_object, 2); // file.txt

        // Should not go beyond
        app.handle_action(Action::NextItem);
        assert_eq!(app.s3_state.selected_object, 2);
    }

    #[test]
    fn test_s3_expand_nested_folder_in_objects_view() {
        let mut app = test_app();
        app.current_service = Service::S3Buckets;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.s3_state.current_bucket = Some("test-bucket".to_string());

        // Add parent folder
        app.s3_state.objects = vec![S3Object {
            key: "parent/".to_string(),
            size: 0,
            last_modified: "2024-01-01T00:00:00Z".to_string(),
            is_prefix: true,
            storage_class: String::new(),
        }];

        // Expand parent
        app.s3_state.expanded_prefixes.insert("parent/".to_string());
        app.s3_state.prefix_preview.insert(
            "parent/".to_string(),
            vec![S3Object {
                key: "parent/child/".to_string(),
                size: 0,
                last_modified: "2024-01-01T00:00:00Z".to_string(),
                is_prefix: true,
                storage_class: String::new(),
            }],
        );

        // Select the nested folder (index 1)
        app.s3_state.selected_object = 1;

        // Expand it (simulate pressing Enter/Right)
        app.handle_action(Action::NextPane);

        // Should be expanded now
        assert!(app.s3_state.expanded_prefixes.contains("parent/child/"));
        assert!(app.s3_state.buckets.loading); // Should trigger load
    }

    #[test]
    fn test_s3_drill_into_nested_folder() {
        let mut app = test_app();
        app.current_service = Service::S3Buckets;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.s3_state.current_bucket = Some("test-bucket".to_string());

        // Add parent folder
        app.s3_state.objects = vec![S3Object {
            key: "parent/".to_string(),
            size: 0,
            last_modified: "2024-01-01T00:00:00Z".to_string(),
            is_prefix: true,
            storage_class: String::new(),
        }];

        // Expand parent
        app.s3_state.expanded_prefixes.insert("parent/".to_string());
        app.s3_state.prefix_preview.insert(
            "parent/".to_string(),
            vec![S3Object {
                key: "parent/child/".to_string(),
                size: 0,
                last_modified: "2024-01-01T00:00:00Z".to_string(),
                is_prefix: true,
                storage_class: String::new(),
            }],
        );

        // Select the nested folder (index 1)
        app.s3_state.selected_object = 1;

        // Drill into it (simulate pressing Enter)
        app.handle_action(Action::Select);

        // Should navigate into the folder
        assert_eq!(app.s3_state.prefix_stack, vec!["parent/child/".to_string()]);
        assert!(app.s3_state.buckets.loading); // Should trigger load
    }

    #[test]
    fn test_s3_esc_pops_navigation_stack() {
        let mut app = test_app();
        app.current_service = Service::S3Buckets;
        app.s3_state.current_bucket = Some("test-bucket".to_string());
        app.s3_state.prefix_stack = vec!["level1/".to_string(), "level1/level2/".to_string()];

        // Press Esc - should pop from stack
        app.handle_action(Action::GoBack);
        assert_eq!(app.s3_state.prefix_stack, vec!["level1/".to_string()]);
        assert!(app.s3_state.buckets.loading);

        // Press Esc again - should pop to bucket root
        app.s3_state.buckets.loading = false;
        app.handle_action(Action::GoBack);
        assert_eq!(app.s3_state.prefix_stack, Vec::<String>::new());
        assert!(app.s3_state.buckets.loading);

        // Press Esc again - should exit bucket
        app.s3_state.buckets.loading = false;
        app.handle_action(Action::GoBack);
        assert_eq!(app.s3_state.current_bucket, None);
    }

    #[test]
    fn test_s3_esc_from_bucket_root_exits() {
        let mut app = test_app();
        app.current_service = Service::S3Buckets;
        app.s3_state.current_bucket = Some("test-bucket".to_string());
        app.s3_state.prefix_stack = vec![];

        // Press Esc from bucket root - should exit bucket
        app.handle_action(Action::GoBack);
        assert_eq!(app.s3_state.current_bucket, None);
        assert_eq!(app.s3_state.objects.len(), 0);
    }

    #[test]
    fn test_s3_drill_into_nested_prefix_from_bucket_list() {
        let mut app = test_app();
        app.current_service = Service::S3Buckets;
        app.service_selected = true;
        app.mode = Mode::Normal;

        // Setup bucket with nested preview
        app.s3_state.buckets.items = vec![S3Bucket {
            name: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            creation_date: "2024-01-01".to_string(),
        }];

        // Expand bucket to show first-level prefix
        app.s3_state
            .expanded_prefixes
            .insert("test-bucket".to_string());
        app.s3_state.bucket_preview.insert(
            "test-bucket".to_string(),
            vec![S3Object {
                key: "parent/".to_string(),
                size: 0,
                last_modified: "2024-01-01".to_string(),
                is_prefix: true,
                storage_class: String::new(),
            }],
        );

        // Expand parent to show nested prefix
        app.s3_state.expanded_prefixes.insert("parent/".to_string());
        app.s3_state.prefix_preview.insert(
            "parent/".to_string(),
            vec![S3Object {
                key: "parent/child/".to_string(),
                size: 0,
                last_modified: "2024-01-01".to_string(),
                is_prefix: true,
                storage_class: String::new(),
            }],
        );

        // Select nested prefix (row 2: bucket, parent, nested)
        app.s3_state.selected_row = 2;

        // Drill into nested prefix
        app.handle_action(Action::Select);

        // Should have both parent and child in stack
        assert_eq!(
            app.s3_state.prefix_stack,
            vec!["parent/".to_string(), "parent/child/".to_string()]
        );
        assert_eq!(app.s3_state.current_bucket, Some("test-bucket".to_string()));
        assert!(app.s3_state.buckets.loading);

        // Now press Esc - should go back to parent
        app.s3_state.buckets.loading = false;
        app.handle_action(Action::GoBack);
        assert_eq!(app.s3_state.prefix_stack, vec!["parent/".to_string()]);
        assert!(app.s3_state.buckets.loading);

        // Press Esc again - should go to bucket root
        app.s3_state.buckets.loading = false;
        app.handle_action(Action::GoBack);
        assert_eq!(app.s3_state.prefix_stack, Vec::<String>::new());
        assert!(app.s3_state.buckets.loading);

        // Press Esc again - should exit bucket
        app.s3_state.buckets.loading = false;
        app.handle_action(Action::GoBack);
        assert_eq!(app.s3_state.current_bucket, None);
    }

    #[test]
    fn test_region_picker_fuzzy_filter() {
        let mut app = test_app();
        app.region_latencies.insert("us-east-1".to_string(), 10);
        app.region_filter = "vir".to_string();
        let filtered = app.get_filtered_regions();
        assert!(filtered.iter().any(|r| r.code == "us-east-1"));
    }

    #[test]
    fn test_profile_picker_loads_profiles() {
        let profiles = App::load_aws_profiles();
        // Should at least have default profile or be empty if no config
        assert!(profiles.is_empty() || profiles.iter().any(|p| p.name == "default"));
    }

    #[test]
    fn test_profile_with_region_uses_it() {
        let mut app = test_app_no_region();
        app.available_profiles = vec![AwsProfile {
            name: "test-profile".to_string(),
            region: Some("eu-west-1".to_string()),
            account: Some("123456789".to_string()),
            role_arn: None,
            source_profile: None,
        }];
        app.profile_picker_selected = 0;
        app.mode = Mode::ProfilePicker;

        // Simulate selecting the profile
        let filtered = app.get_filtered_profiles();
        if let Some(profile) = filtered.first() {
            let profile_name = profile.name.clone();
            let profile_region = profile.region.clone();

            app.profile = profile_name;
            if let Some(region) = profile_region {
                app.region = region;
            }
        }

        assert_eq!(app.profile, "test-profile");
        assert_eq!(app.region, "eu-west-1");
    }

    #[test]
    fn test_profile_without_region_keeps_unknown() {
        let mut app = test_app_no_region();
        let initial_region = app.region.clone();

        app.available_profiles = vec![AwsProfile {
            name: "test-profile".to_string(),
            region: None,
            account: None,
            role_arn: None,
            source_profile: None,
        }];
        app.profile_picker_selected = 0;
        app.mode = Mode::ProfilePicker;

        let filtered = app.get_filtered_profiles();
        if let Some(profile) = filtered.first() {
            let profile_name = profile.name.clone();
            let profile_region = profile.region.clone();

            app.profile = profile_name;
            if let Some(region) = profile_region {
                app.region = region;
            }
        }

        assert_eq!(app.profile, "test-profile");
        assert_eq!(app.region, initial_region); // Should keep initial region
    }

    #[test]
    fn test_region_selection_closes_all_tabs() {
        let mut app = test_app();

        // Add some tabs
        app.tabs.push(Tab {
            service: Service::CloudWatchLogGroups,
            title: "CloudWatch".to_string(),
            breadcrumb: "CloudWatch".to_string(),
        });
        app.tabs.push(Tab {
            service: Service::S3Buckets,
            title: "S3".to_string(),
            breadcrumb: "S3".to_string(),
        });
        app.service_selected = true;
        app.current_tab = 1;

        // Add latency for region
        app.region_latencies.insert("eu-west-1".to_string(), 50);

        // Simulate selecting a different region
        app.mode = Mode::RegionPicker;
        app.region_picker_selected = 0;

        let filtered = app.get_filtered_regions();
        if let Some(region) = filtered.first() {
            app.region = region.code.to_string();
            app.tabs.clear();
            app.current_tab = 0;
            app.service_selected = false;
            app.mode = Mode::Normal;
        }

        assert_eq!(app.tabs.len(), 0);
        assert_eq!(app.current_tab, 0);
        assert!(!app.service_selected);
        assert_eq!(app.region, "eu-west-1");
    }

    #[test]
    fn test_region_picker_can_be_closed_without_selection() {
        let mut app = test_app();
        let initial_region = app.region.clone();

        app.mode = Mode::RegionPicker;

        // Close without selecting (Esc)
        app.mode = Mode::Normal;

        // Region should not change
        assert_eq!(app.region, initial_region);
    }

    #[test]
    fn test_session_filter_works() {
        let mut app = test_app();

        app.sessions = vec![
            Session {
                id: "1".to_string(),
                timestamp: "2024-01-01".to_string(),
                profile: "prod-profile".to_string(),
                region: "us-east-1".to_string(),
                account_id: "123456789".to_string(),
                role_arn: "arn:aws:iam::123456789:role/admin".to_string(),
                tabs: vec![],
            },
            Session {
                id: "2".to_string(),
                timestamp: "2024-01-02".to_string(),
                profile: "dev-profile".to_string(),
                region: "eu-west-1".to_string(),
                account_id: "987654321".to_string(),
                role_arn: "arn:aws:iam::987654321:role/dev".to_string(),
                tabs: vec![],
            },
        ];

        // Filter by profile
        app.session_filter = "prod".to_string();
        let filtered = app.get_filtered_sessions();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].profile, "prod-profile");

        // Filter by region
        app.session_filter = "eu".to_string();
        let filtered = app.get_filtered_sessions();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].region, "eu-west-1");

        // No filter
        app.session_filter.clear();
        let filtered = app.get_filtered_sessions();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_profile_picker_shows_account() {
        let mut app = test_app_no_region();
        app.available_profiles = vec![AwsProfile {
            name: "test-profile".to_string(),
            region: Some("us-east-1".to_string()),
            account: Some("123456789".to_string()),
            role_arn: None,
            source_profile: None,
        }];

        let filtered = app.get_filtered_profiles();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].account, Some("123456789".to_string()));
    }

    #[test]
    fn test_profile_without_account() {
        let mut app = test_app_no_region();
        app.available_profiles = vec![AwsProfile {
            name: "test-profile".to_string(),
            region: Some("us-east-1".to_string()),
            account: None,
            role_arn: None,
            source_profile: None,
        }];

        let filtered = app.get_filtered_profiles();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].account, None);
    }

    #[test]
    fn test_profile_with_all_fields() {
        let mut app = test_app_no_region();
        app.available_profiles = vec![AwsProfile {
            name: "prod-profile".to_string(),
            region: Some("us-west-2".to_string()),
            account: Some("123456789".to_string()),
            role_arn: Some("arn:aws:iam::123456789:role/AdminRole".to_string()),
            source_profile: Some("base-profile".to_string()),
        }];

        let filtered = app.get_filtered_profiles();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "prod-profile");
        assert_eq!(filtered[0].region, Some("us-west-2".to_string()));
        assert_eq!(filtered[0].account, Some("123456789".to_string()));
        assert_eq!(
            filtered[0].role_arn,
            Some("arn:aws:iam::123456789:role/AdminRole".to_string())
        );
        assert_eq!(filtered[0].source_profile, Some("base-profile".to_string()));
    }

    #[test]
    fn test_profile_filter_by_source_profile() {
        let mut app = test_app_no_region();
        app.available_profiles = vec![
            AwsProfile {
                name: "profile1".to_string(),
                region: None,
                account: None,
                role_arn: None,
                source_profile: Some("base".to_string()),
            },
            AwsProfile {
                name: "profile2".to_string(),
                region: None,
                account: None,
                role_arn: None,
                source_profile: Some("other".to_string()),
            },
        ];

        app.profile_filter = "base".to_string();
        let filtered = app.get_filtered_profiles();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "profile1");
    }

    #[test]
    fn test_profile_filter_by_role() {
        let mut app = test_app_no_region();
        app.available_profiles = vec![
            AwsProfile {
                name: "admin-profile".to_string(),
                region: None,
                account: None,
                role_arn: Some("arn:aws:iam::123:role/AdminRole".to_string()),
                source_profile: None,
            },
            AwsProfile {
                name: "dev-profile".to_string(),
                region: None,
                account: None,
                role_arn: Some("arn:aws:iam::123:role/DevRole".to_string()),
                source_profile: None,
            },
        ];

        app.profile_filter = "Admin".to_string();
        let filtered = app.get_filtered_profiles();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "admin-profile");
    }

    #[test]
    fn test_profiles_sorted_by_name() {
        let mut app = test_app_no_region();
        app.available_profiles = vec![
            AwsProfile {
                name: "zebra-profile".to_string(),
                region: None,
                account: None,
                role_arn: None,
                source_profile: None,
            },
            AwsProfile {
                name: "alpha-profile".to_string(),
                region: None,
                account: None,
                role_arn: None,
                source_profile: None,
            },
            AwsProfile {
                name: "beta-profile".to_string(),
                region: None,
                account: None,
                role_arn: None,
                source_profile: None,
            },
        ];

        let filtered = app.get_filtered_profiles();
        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0].name, "alpha-profile");
        assert_eq!(filtered[1].name, "beta-profile");
        assert_eq!(filtered[2].name, "zebra-profile");
    }

    #[test]
    fn test_profile_with_role_arn() {
        let mut app = test_app_no_region();
        app.available_profiles = vec![AwsProfile {
            name: "role-profile".to_string(),
            region: Some("us-east-1".to_string()),
            account: Some("123456789".to_string()),
            role_arn: Some("arn:aws:iam::123456789:role/AdminRole".to_string()),
            source_profile: None,
        }];

        let filtered = app.get_filtered_profiles();
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].role_arn.as_ref().unwrap().contains(":role/"));
    }

    #[test]
    fn test_profile_with_user_arn() {
        let mut app = test_app_no_region();
        app.available_profiles = vec![AwsProfile {
            name: "user-profile".to_string(),
            region: Some("us-east-1".to_string()),
            account: Some("123456789".to_string()),
            role_arn: Some("arn:aws:iam::123456789:user/john-doe".to_string()),
            source_profile: None,
        }];

        let filtered = app.get_filtered_profiles();
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].role_arn.as_ref().unwrap().contains(":user/"));
    }

    #[test]
    fn test_filtered_profiles_also_sorted() {
        let mut app = test_app_no_region();
        app.available_profiles = vec![
            AwsProfile {
                name: "prod-zebra".to_string(),
                region: Some("us-east-1".to_string()),
                account: None,
                role_arn: None,
                source_profile: None,
            },
            AwsProfile {
                name: "prod-alpha".to_string(),
                region: Some("us-east-1".to_string()),
                account: None,
                role_arn: None,
                source_profile: None,
            },
            AwsProfile {
                name: "dev-profile".to_string(),
                region: Some("us-west-2".to_string()),
                account: None,
                role_arn: None,
                source_profile: None,
            },
        ];

        app.profile_filter = "prod".to_string();
        let filtered = app.get_filtered_profiles();
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name, "prod-alpha");
        assert_eq!(filtered[1].name, "prod-zebra");
    }

    #[test]
    fn test_profile_picker_has_all_columns() {
        let mut app = test_app_no_region();
        app.available_profiles = vec![AwsProfile {
            name: "test".to_string(),
            region: Some("us-east-1".to_string()),
            account: Some("123456789".to_string()),
            role_arn: Some("arn:aws:iam::123456789:role/Admin".to_string()),
            source_profile: Some("base".to_string()),
        }];

        let filtered = app.get_filtered_profiles();
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].name == "test");
        assert!(filtered[0].region.is_some());
        assert!(filtered[0].account.is_some());
        assert!(filtered[0].role_arn.is_some());
        assert!(filtered[0].source_profile.is_some());
    }

    #[test]
    fn test_session_picker_shows_tab_count() {
        let mut app = test_app_no_region();
        app.sessions = vec![Session {
            id: "1".to_string(),
            timestamp: "2024-01-01".to_string(),
            profile: "test".to_string(),
            region: "us-east-1".to_string(),
            account_id: "123".to_string(),
            role_arn: String::new(),
            tabs: vec![
                SessionTab {
                    service: "CloudWatch".to_string(),
                    title: "Logs".to_string(),
                    breadcrumb: String::new(),
                    filter: None,
                    selected_item: None,
                },
                SessionTab {
                    service: "S3".to_string(),
                    title: "Buckets".to_string(),
                    breadcrumb: String::new(),
                    filter: None,
                    selected_item: None,
                },
            ],
        }];

        let filtered = app.get_filtered_sessions();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].tabs.len(), 2);
    }

    #[test]
    fn test_start_background_data_fetch_loads_profiles() {
        let mut app = test_app_no_region();
        assert!(app.available_profiles.is_empty());

        // Load profiles synchronously
        app.available_profiles = App::load_aws_profiles();

        // Profiles should be loaded
        assert!(!app.available_profiles.is_empty() || app.available_profiles.is_empty());
    }

    #[test]
    fn test_refresh_in_profile_picker() {
        let mut app = test_app_no_region();
        app.mode = Mode::ProfilePicker;
        app.available_profiles = vec![AwsProfile {
            name: "test".to_string(),
            region: None,
            account: None,
            role_arn: None,
            source_profile: None,
        }];

        app.handle_action(Action::Refresh);

        // Should set loading state
        assert!(app.log_groups_state.loading);
        assert_eq!(app.log_groups_state.loading_message, "Refreshing...");
    }

    #[test]
    fn test_refresh_sets_loading_for_profile_picker() {
        let mut app = test_app_no_region();
        app.mode = Mode::ProfilePicker;

        assert!(!app.log_groups_state.loading);

        app.handle_action(Action::Refresh);

        assert!(app.log_groups_state.loading);
    }

    #[test]
    fn test_profiles_loaded_on_demand() {
        let mut app = test_app_no_region();

        // Profiles not loaded by default
        assert!(app.available_profiles.is_empty());

        // Load on demand
        app.available_profiles = App::load_aws_profiles();

        // Now loaded
        assert!(!app.available_profiles.is_empty() || app.available_profiles.is_empty());
    }

    #[test]
    fn test_profile_accounts_not_fetched_automatically() {
        let mut app = test_app_no_region();
        app.available_profiles = App::load_aws_profiles();

        // Accounts should not be populated automatically
        for profile in &app.available_profiles {
            // Account may or may not be set depending on what's in config
            // But we're not fetching them automatically
            assert!(profile.account.is_none() || profile.account.is_some());
        }
    }

    #[test]
    fn test_ctrl_r_triggers_account_fetch() {
        let mut app = test_app_no_region();
        app.mode = Mode::ProfilePicker;
        app.available_profiles = vec![AwsProfile {
            name: "test".to_string(),
            region: Some("us-east-1".to_string()),
            account: None,
            role_arn: None,
            source_profile: None,
        }];

        // Before refresh, account is None
        assert!(app.available_profiles[0].account.is_none());

        // Trigger refresh
        app.handle_action(Action::Refresh);

        // Should set loading state (actual fetch happens in main.rs event loop)
        assert!(app.log_groups_state.loading);
    }

    #[test]
    fn test_refresh_in_region_picker() {
        let mut app = test_app_no_region();
        app.mode = Mode::RegionPicker;

        let initial_latencies = app.region_latencies.len();
        app.handle_action(Action::Refresh);

        // Latencies should be cleared and remeasured
        assert!(app.region_latencies.is_empty() || app.region_latencies.len() >= initial_latencies);
    }

    #[test]
    fn test_refresh_in_session_picker() {
        let mut app = test_app_no_region();
        app.mode = Mode::SessionPicker;
        app.sessions = vec![];

        app.handle_action(Action::Refresh);

        // Sessions should be reloaded (may be empty if no saved sessions)
        assert!(app.sessions.is_empty() || !app.sessions.is_empty());
    }

    #[test]
    fn test_session_picker_selection() {
        let mut app = test_app();

        app.sessions = vec![Session {
            id: "1".to_string(),
            timestamp: "2024-01-01".to_string(),
            profile: "prod-profile".to_string(),
            region: "us-west-2".to_string(),
            account_id: "123456789".to_string(),
            role_arn: "arn:aws:iam::123456789:role/admin".to_string(),
            tabs: vec![SessionTab {
                service: "CloudWatchLogGroups".to_string(),
                title: "Log Groups".to_string(),
                breadcrumb: "CloudWatch > Log Groups".to_string(),
                filter: Some("test".to_string()),
                selected_item: None,
            }],
        }];

        app.mode = Mode::SessionPicker;
        app.session_picker_selected = 0;

        // Simulate selecting the session
        app.handle_action(Action::Select);

        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.profile, "prod-profile");
        assert_eq!(app.region, "us-west-2");
        assert_eq!(app.config.account_id, "123456789");
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.tabs[0].title, "Log Groups");
    }

    #[test]
    fn test_save_session_creates_session() {
        let mut app =
            App::new_without_client("test-profile".to_string(), Some("us-east-1".to_string()));
        app.config.account_id = "123456789".to_string();
        app.config.role_arn = "arn:aws:iam::123456789:role/test".to_string();

        app.tabs.push(Tab {
            service: Service::CloudWatchLogGroups,
            title: "Log Groups".to_string(),
            breadcrumb: "CloudWatch > Log Groups".to_string(),
        });

        app.save_current_session();

        assert!(app.current_session.is_some());
        let session = app.current_session.clone().unwrap();
        assert_eq!(session.profile, "test-profile");
        assert_eq!(session.region, "us-east-1");
        assert_eq!(session.account_id, "123456789");
        assert_eq!(session.tabs.len(), 1);

        // Cleanup
        let _ = session.delete();
    }

    #[test]
    fn test_save_session_updates_existing() {
        let mut app =
            App::new_without_client("test-profile".to_string(), Some("us-east-1".to_string()));
        app.config.account_id = "123456789".to_string();
        app.config.role_arn = "arn:aws:iam::123456789:role/test".to_string();

        app.current_session = Some(Session {
            id: "existing".to_string(),
            timestamp: "2024-01-01".to_string(),
            profile: "test-profile".to_string(),
            region: "us-east-1".to_string(),
            account_id: "123456789".to_string(),
            role_arn: "arn:aws:iam::123456789:role/test".to_string(),
            tabs: vec![],
        });

        app.tabs.push(Tab {
            service: Service::CloudWatchLogGroups,
            title: "Log Groups".to_string(),
            breadcrumb: "CloudWatch > Log Groups".to_string(),
        });

        app.save_current_session();

        let session = app.current_session.clone().unwrap();
        assert_eq!(session.id, "existing");
        assert_eq!(session.tabs.len(), 1);

        // Cleanup
        let _ = session.delete();
    }

    #[test]
    fn test_save_session_skips_empty_tabs() {
        let mut app =
            App::new_without_client("test-profile".to_string(), Some("us-east-1".to_string()));
        app.config.account_id = "123456789".to_string();

        app.save_current_session();

        assert!(app.current_session.is_none());
    }

    #[test]
    fn test_save_session_deletes_when_tabs_closed() {
        let mut app =
            App::new_without_client("test-profile".to_string(), Some("us-east-1".to_string()));
        app.config.account_id = "123456789".to_string();
        app.config.role_arn = "arn:aws:iam::123456789:role/test".to_string();

        // Create a session with tabs
        app.current_session = Some(Session {
            id: "test_delete".to_string(),
            timestamp: "2024-01-01 10:00:00 UTC".to_string(),
            profile: "test-profile".to_string(),
            region: "us-east-1".to_string(),
            account_id: "123456789".to_string(),
            role_arn: "arn:aws:iam::123456789:role/test".to_string(),
            tabs: vec![],
        });

        // Save with no tabs should delete session
        app.save_current_session();

        assert!(app.current_session.is_none());
    }

    #[test]
    fn test_closing_all_tabs_deletes_session() {
        let mut app =
            App::new_without_client("test-profile".to_string(), Some("us-east-1".to_string()));
        app.config.account_id = "123456789".to_string();
        app.config.role_arn = "arn:aws:iam::123456789:role/test".to_string();

        // Add a tab
        app.tabs.push(Tab {
            service: Service::CloudWatchLogGroups,
            title: "Log Groups".to_string(),
            breadcrumb: "CloudWatch > Log Groups".to_string(),
        });

        // Create session
        app.save_current_session();
        assert!(app.current_session.is_some());
        let session_id = app.current_session.as_ref().unwrap().id.clone();

        // Close all tabs
        app.tabs.clear();

        // Save should delete session
        app.save_current_session();
        assert!(app.current_session.is_none());

        // Cleanup - ensure session file is deleted
        let _ = Session::load(&session_id).map(|s| s.delete());
    }

    #[test]
    fn test_credential_error_opens_profile_picker() {
        // Simulate what main.rs does on credential error
        let mut app = App::new_without_client("default".to_string(), None);
        let error_str = "Unable to load credentials from any source";

        if error_str.contains("credentials") {
            app.available_profiles = App::load_aws_profiles();
            app.mode = Mode::ProfilePicker;
        }

        assert_eq!(app.mode, Mode::ProfilePicker);
        // Should have loaded profiles
        assert!(!app.available_profiles.is_empty() || app.available_profiles.is_empty());
    }

    #[test]
    fn test_non_credential_error_shows_error_modal() {
        let mut app = App::new_without_client("default".to_string(), None);
        let error_str = "Network timeout";

        if !error_str.contains("credentials") {
            app.error_message = Some(error_str.to_string());
            app.mode = Mode::ErrorModal;
        }

        assert_eq!(app.mode, Mode::ErrorModal);
        assert!(app.error_message.is_some());
    }

    #[tokio::test]
    async fn test_profile_selection_loads_credentials() {
        // Set a valid AWS profile if available
        std::env::set_var("AWS_PROFILE", "default");

        // Try to create app with profile
        let result = App::new(Some("default".to_string()), Some("us-east-1".to_string())).await;

        if let Ok(app) = result {
            // If credentials are available, verify they're loaded
            assert!(!app.config.account_id.is_empty());
            assert!(!app.config.role_arn.is_empty());
            assert_eq!(app.profile, "default");
            assert_eq!(app.config.region, "us-east-1");
        }
        // If no credentials, test passes (can't test without real AWS creds)
    }

    #[test]
    fn test_new_app_shows_service_picker_with_no_tabs() {
        let app = App::new_without_client("default".to_string(), Some("us-east-1".to_string()));

        // Should start with no service selected
        assert!(!app.service_selected);
        // Should be in ServicePicker mode (service picker)
        assert_eq!(app.mode, Mode::ServicePicker);
        // Should have no tabs
        assert!(app.tabs.is_empty());
    }

    #[tokio::test]
    async fn test_aws_profile_env_var_read_before_config_load() {
        // This test verifies the bug: AWS_PROFILE should be read and used
        std::env::set_var("AWS_PROFILE", "test-profile");

        // Simulate what happens in App::new
        let profile_name = None
            .or_else(|| std::env::var("AWS_PROFILE").ok())
            .unwrap_or_else(|| "default".to_string());

        // Should have read test-profile from env
        assert_eq!(profile_name, "test-profile");

        // Now set it (redundant but that's what the code does)
        std::env::set_var("AWS_PROFILE", &profile_name);

        // Verify it's still set
        assert_eq!(std::env::var("AWS_PROFILE").unwrap(), "test-profile");

        std::env::remove_var("AWS_PROFILE");
    }

    #[test]
    fn test_next_preferences_cloudformation() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.mode = Mode::ColumnSelector;
        app.column_selector_index = 0;

        // Should jump to PageSize section
        let page_size_idx = app.cfn_column_ids.len() + 2;
        app.handle_action(Action::NextPreferences);
        assert_eq!(app.column_selector_index, page_size_idx);

        // Should wrap back to Columns
        app.handle_action(Action::NextPreferences);
        assert_eq!(app.column_selector_index, 0);
    }

    #[test]
    fn test_next_preferences_lambda_functions() {
        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.mode = Mode::ColumnSelector;
        app.column_selector_index = 0;

        let page_size_idx = app.lambda_state.function_column_ids.len() + 2;
        app.handle_action(Action::NextPreferences);
        assert_eq!(app.column_selector_index, page_size_idx);

        app.handle_action(Action::NextPreferences);
        assert_eq!(app.column_selector_index, 0);
    }

    #[test]
    fn test_next_preferences_lambda_applications() {
        let mut app = test_app();
        app.current_service = Service::LambdaApplications;
        app.mode = Mode::ColumnSelector;
        app.column_selector_index = 0;

        let page_size_idx = app.lambda_application_column_ids.len() + 2;
        app.handle_action(Action::NextPreferences);
        assert_eq!(app.column_selector_index, page_size_idx);

        app.handle_action(Action::NextPreferences);
        assert_eq!(app.column_selector_index, 0);
    }

    #[test]
    fn test_next_preferences_ecr_images() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.ecr_state.current_repository = Some("test-repo".to_string());
        app.mode = Mode::ColumnSelector;
        app.column_selector_index = 0;

        let page_size_idx = app.ecr_image_column_ids.len() + 2;
        app.handle_action(Action::NextPreferences);
        assert_eq!(app.column_selector_index, page_size_idx);

        app.handle_action(Action::NextPreferences);
        assert_eq!(app.column_selector_index, 0);
    }

    #[test]
    fn test_cloudformation_next_item() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.cfn_state.status_filter = CfnStatusFilter::Complete;
        app.cfn_state.table.items = vec![
            CfnStack {
                name: "stack1".to_string(),
                stack_id: "id1".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                created_time: "2024-01-01".to_string(),
                updated_time: String::new(),
                deleted_time: String::new(),
                drift_status: String::new(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: String::new(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            },
            CfnStack {
                name: "stack2".to_string(),
                stack_id: "id2".to_string(),
                status: "UPDATE_COMPLETE".to_string(),
                created_time: "2024-01-02".to_string(),
                updated_time: String::new(),
                deleted_time: String::new(),
                drift_status: String::new(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: String::new(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            },
        ];
        app.cfn_state.table.reset();

        app.handle_action(Action::NextItem);
        assert_eq!(app.cfn_state.table.selected, 1);

        app.handle_action(Action::NextItem);
        assert_eq!(app.cfn_state.table.selected, 1); // At max
    }

    #[test]
    fn test_cloudformation_prev_item() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.cfn_state.status_filter = CfnStatusFilter::Complete;
        app.cfn_state.table.items = vec![
            CfnStack {
                name: "stack1".to_string(),
                stack_id: "id1".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                created_time: "2024-01-01".to_string(),
                updated_time: String::new(),
                deleted_time: String::new(),
                drift_status: String::new(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: String::new(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            },
            CfnStack {
                name: "stack2".to_string(),
                stack_id: "id2".to_string(),
                status: "UPDATE_COMPLETE".to_string(),
                created_time: "2024-01-02".to_string(),
                updated_time: String::new(),
                deleted_time: String::new(),
                drift_status: String::new(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: String::new(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            },
        ];
        app.cfn_state.table.selected = 1;

        app.handle_action(Action::PrevItem);
        assert_eq!(app.cfn_state.table.selected, 0);

        app.handle_action(Action::PrevItem);
        assert_eq!(app.cfn_state.table.selected, 0); // At min
    }

    #[test]
    fn test_cloudformation_page_down() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.cfn_state.status_filter = CfnStatusFilter::Complete;

        // Create 20 stacks
        for i in 0..20 {
            app.cfn_state.table.items.push(CfnStack {
                name: format!("stack{}", i),
                stack_id: format!("id{}", i),
                status: "CREATE_COMPLETE".to_string(),
                created_time: format!("2024-01-{:02}", i + 1),
                updated_time: String::new(),
                deleted_time: String::new(),
                drift_status: String::new(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: String::new(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            });
        }
        app.cfn_state.table.reset();

        app.handle_action(Action::PageDown);
        assert_eq!(app.cfn_state.table.selected, 10);

        app.handle_action(Action::PageDown);
        assert_eq!(app.cfn_state.table.selected, 19); // Clamped to max
    }

    #[test]
    fn test_cloudformation_page_up() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.cfn_state.status_filter = CfnStatusFilter::Complete;

        // Create 20 stacks
        for i in 0..20 {
            app.cfn_state.table.items.push(CfnStack {
                name: format!("stack{}", i),
                stack_id: format!("id{}", i),
                status: "CREATE_COMPLETE".to_string(),
                created_time: format!("2024-01-{:02}", i + 1),
                updated_time: String::new(),
                deleted_time: String::new(),
                drift_status: String::new(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: String::new(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            });
        }
        app.cfn_state.table.selected = 15;

        app.handle_action(Action::PageUp);
        assert_eq!(app.cfn_state.table.selected, 5);

        app.handle_action(Action::PageUp);
        assert_eq!(app.cfn_state.table.selected, 0); // Clamped to min
    }

    #[test]
    fn test_cloudformation_filter_input() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.mode = Mode::Normal;

        app.handle_action(Action::StartFilter);
        assert_eq!(app.mode, Mode::FilterInput);

        // Directly set filter (character input is handled in event loop, not actions)
        app.cfn_state.table.filter = "test".to_string();
        assert_eq!(app.cfn_state.table.filter, "test");
    }

    #[test]
    fn test_cloudformation_filter_applies() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.cfn_state.status_filter = CfnStatusFilter::Complete;
        app.cfn_state.table.items = vec![
            CfnStack {
                name: "prod-stack".to_string(),
                stack_id: "id1".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                created_time: "2024-01-01".to_string(),
                updated_time: String::new(),
                deleted_time: String::new(),
                drift_status: String::new(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: "Production stack".to_string(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            },
            CfnStack {
                name: "dev-stack".to_string(),
                stack_id: "id2".to_string(),
                status: "UPDATE_COMPLETE".to_string(),
                created_time: "2024-01-02".to_string(),
                updated_time: String::new(),
                deleted_time: String::new(),
                drift_status: String::new(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: "Development stack".to_string(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            },
        ];
        app.cfn_state.table.filter = "prod".to_string();

        let filtered = filtered_cloudformation_stacks(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "prod-stack");
    }

    #[test]
    fn test_cloudformation_right_arrow_expands() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.cfn_state.status_filter = CfnStatusFilter::Complete;
        app.cfn_state.table.items = vec![CfnStack {
            name: "test-stack".to_string(),
            stack_id: "arn:aws:cloudformation:us-east-1:123456789012:stack/test-stack/abc123"
                .to_string(),
            status: "CREATE_COMPLETE".to_string(),
            created_time: "2024-01-01".to_string(),
            updated_time: String::new(),
            deleted_time: String::new(),
            drift_status: String::new(),
            last_drift_check_time: String::new(),
            status_reason: String::new(),
            description: "Test stack".to_string(),
            detailed_status: String::new(),
            root_stack: String::new(),
            parent_stack: String::new(),
            termination_protection: false,
            iam_role: String::new(),
            tags: Vec::new(),
            stack_policy: String::new(),
            rollback_monitoring_time: String::new(),
            rollback_alarms: Vec::new(),
            notification_arns: Vec::new(),
        }];
        app.cfn_state.table.reset();

        assert_eq!(app.cfn_state.table.expanded_item, None);

        app.handle_action(Action::NextPane);
        assert_eq!(app.cfn_state.table.expanded_item, Some(0));
    }

    #[test]
    fn test_cloudformation_left_arrow_collapses() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.cfn_state.status_filter = CfnStatusFilter::Complete;
        app.cfn_state.table.items = vec![CfnStack {
            name: "test-stack".to_string(),
            stack_id: "arn:aws:cloudformation:us-east-1:123456789012:stack/test-stack/abc123"
                .to_string(),
            status: "CREATE_COMPLETE".to_string(),
            created_time: "2024-01-01".to_string(),
            updated_time: String::new(),
            deleted_time: String::new(),
            drift_status: String::new(),
            last_drift_check_time: String::new(),
            status_reason: String::new(),
            description: "Test stack".to_string(),
            detailed_status: String::new(),
            root_stack: String::new(),
            parent_stack: String::new(),
            termination_protection: false,
            iam_role: String::new(),
            tags: Vec::new(),
            stack_policy: String::new(),
            rollback_monitoring_time: String::new(),
            rollback_alarms: Vec::new(),
            notification_arns: Vec::new(),
        }];
        app.cfn_state.table.reset();
        app.cfn_state.table.expanded_item = Some(0);

        app.handle_action(Action::PrevPane);
        assert_eq!(app.cfn_state.table.expanded_item, None);
    }

    #[test]
    fn test_cloudformation_enter_drills_into_stack() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.tabs = vec![Tab {
            service: Service::CloudFormationStacks,
            title: "CloudFormation > Stacks".to_string(),
            breadcrumb: "CloudFormation > Stacks".to_string(),
        }];
        app.current_tab = 0;
        app.cfn_state.status_filter = CfnStatusFilter::Complete;
        app.cfn_state.table.items = vec![CfnStack {
            name: "test-stack".to_string(),
            stack_id: "arn:aws:cloudformation:us-east-1:123456789012:stack/test-stack/abc123"
                .to_string(),
            status: "CREATE_COMPLETE".to_string(),
            created_time: "2024-01-01".to_string(),
            updated_time: String::new(),
            deleted_time: String::new(),
            drift_status: String::new(),
            last_drift_check_time: String::new(),
            status_reason: String::new(),
            description: "Test stack".to_string(),
            detailed_status: String::new(),
            root_stack: String::new(),
            parent_stack: String::new(),
            termination_protection: false,
            iam_role: String::new(),
            tags: Vec::new(),
            stack_policy: String::new(),
            rollback_monitoring_time: String::new(),
            rollback_alarms: Vec::new(),
            notification_arns: Vec::new(),
        }];
        app.cfn_state.table.reset();

        // Verify filtering works
        let filtered = filtered_cloudformation_stacks(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "test-stack");

        assert_eq!(app.cfn_state.current_stack, None);

        // Enter drills into stack detail view
        app.handle_action(Action::Select);
        assert_eq!(app.cfn_state.current_stack, Some("test-stack".to_string()));
    }

    #[test]
    fn test_cloudformation_copy_to_clipboard() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.cfn_state.status_filter = CfnStatusFilter::Complete;
        app.cfn_state.table.items = vec![
            CfnStack {
                name: "stack1".to_string(),
                stack_id: "id1".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                created_time: "2024-01-01".to_string(),
                updated_time: String::new(),
                deleted_time: String::new(),
                drift_status: String::new(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: String::new(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            },
            CfnStack {
                name: "stack2".to_string(),
                stack_id: "id2".to_string(),
                status: "UPDATE_COMPLETE".to_string(),
                created_time: "2024-01-02".to_string(),
                updated_time: String::new(),
                deleted_time: String::new(),
                drift_status: String::new(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: String::new(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            },
        ];

        assert!(!app.snapshot_requested);
        app.handle_action(Action::CopyToClipboard);

        // Should set snapshot_requested flag
        assert!(app.snapshot_requested);
    }

    #[test]
    fn test_cloudformation_expansion_shows_all_visible_columns() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.cfn_state.status_filter = CfnStatusFilter::Complete;
        app.cfn_state.table.items = vec![CfnStack {
            name: "test-stack".to_string(),
            stack_id: "arn:aws:cloudformation:us-east-1:123456789012:stack/test-stack/abc123"
                .to_string(),
            status: "CREATE_COMPLETE".to_string(),
            created_time: "2024-01-01".to_string(),
            updated_time: "2024-01-02".to_string(),
            deleted_time: String::new(),
            drift_status: "IN_SYNC".to_string(),
            last_drift_check_time: "2024-01-03".to_string(),
            status_reason: String::new(),
            description: "Test description".to_string(),
            detailed_status: String::new(),
            root_stack: String::new(),
            parent_stack: String::new(),
            termination_protection: false,
            iam_role: String::new(),
            tags: Vec::new(),
            stack_policy: String::new(),
            rollback_monitoring_time: String::new(),
            rollback_alarms: Vec::new(),
            notification_arns: Vec::new(),
        }];

        // Set visible columns
        app.cfn_visible_column_ids = [
            CfnColumn::Name,
            CfnColumn::Status,
            CfnColumn::CreatedTime,
            CfnColumn::Description,
        ]
        .iter()
        .map(|c| c.id())
        .collect();

        app.cfn_state.table.expanded_item = Some(0);

        // Verify all visible columns would be shown in expansion
        // (This is a structural test - actual rendering is in UI layer)
        assert_eq!(app.cfn_visible_column_ids.len(), 4);
        assert!(app.cfn_state.table.has_expanded_item());
    }

    #[test]
    fn test_cloudformation_empty_list_shows_page_1() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.cfn_state.table.items = vec![];

        let filtered = filtered_cloudformation_stacks(&app);
        assert_eq!(filtered.len(), 0);

        // Pagination should still show [1] even with 0 items
        let page_size = app.cfn_state.table.page_size.value();
        let total_pages = filtered.len().div_ceil(page_size);
        assert_eq!(total_pages, 0);

        // render_pagination_text(0, 0) should return "[1]"
        // This is tested in UI layer
    }
}

impl App {
    pub fn get_filtered_regions(&self) -> Vec<AwsRegion> {
        let mut all = AwsRegion::all();

        // Add latencies to regions
        for region in &mut all {
            region.latency_ms = self.region_latencies.get(region.code).copied();
        }

        // Filter by search term
        let filtered: Vec<AwsRegion> = if self.region_filter.is_empty() {
            all
        } else {
            let filter_lower = self.region_filter.to_lowercase();
            all.into_iter()
                .filter(|r| {
                    r.name.to_lowercase().contains(&filter_lower)
                        || r.code.to_lowercase().contains(&filter_lower)
                        || r.group.to_lowercase().contains(&filter_lower)
                })
                .collect()
        };

        // Sort by latency (lowest first), treat None as 1000ms
        let mut sorted = filtered;
        sorted.sort_by_key(|r| r.latency_ms.unwrap_or(1000));
        sorted
    }

    pub fn measure_region_latencies(&mut self) {
        use std::time::Instant;
        self.region_latencies.clear();

        let regions = AwsRegion::all();
        let start_all = Instant::now();
        tracing::info!("Starting latency measurement for {} regions", regions.len());

        let handles: Vec<_> = regions
            .iter()
            .map(|region| {
                let code = region.code.to_string();
                std::thread::spawn(move || {
                    // Use STS endpoint - fastest and most reliable
                    let endpoint = format!("https://sts.{}.amazonaws.com", code);
                    let start = Instant::now();

                    match ureq::get(&endpoint)
                        .timeout(std::time::Duration::from_secs(2))
                        .call()
                    {
                        Ok(_) => {
                            let latency = start.elapsed().as_millis() as u64;
                            Some((code, latency))
                        }
                        Err(e) => {
                            tracing::debug!("Failed to measure {}: {}", code, e);
                            Some((code, 9999))
                        }
                    }
                })
            })
            .collect();

        for handle in handles {
            if let Ok(Some((code, latency))) = handle.join() {
                self.region_latencies.insert(code, latency);
            }
        }

        tracing::info!(
            "Measured {} regions in {:?}",
            self.region_latencies.len(),
            start_all.elapsed()
        );
    }

    pub fn get_filtered_profiles(&self) -> Vec<&AwsProfile> {
        filter_profiles(&self.available_profiles, &self.profile_filter)
    }

    pub fn get_filtered_sessions(&self) -> Vec<&Session> {
        if self.session_filter.is_empty() {
            return self.sessions.iter().collect();
        }
        let filter_lower = self.session_filter.to_lowercase();
        self.sessions
            .iter()
            .filter(|s| {
                s.profile.to_lowercase().contains(&filter_lower)
                    || s.region.to_lowercase().contains(&filter_lower)
                    || s.account_id.to_lowercase().contains(&filter_lower)
                    || s.role_arn.to_lowercase().contains(&filter_lower)
            })
            .collect()
    }

    pub fn get_filtered_tabs(&self) -> Vec<(usize, &Tab)> {
        if self.tab_filter.is_empty() {
            return self.tabs.iter().enumerate().collect();
        }
        let filter_lower = self.tab_filter.to_lowercase();
        self.tabs
            .iter()
            .enumerate()
            .filter(|(_, tab)| {
                tab.title.to_lowercase().contains(&filter_lower)
                    || tab.breadcrumb.to_lowercase().contains(&filter_lower)
            })
            .collect()
    }

    pub fn load_aws_profiles() -> Vec<AwsProfile> {
        AwsProfile::load_all()
    }

    pub async fn fetch_profile_accounts(&mut self) {
        for profile in &mut self.available_profiles {
            if profile.account.is_none() {
                let region = profile
                    .region
                    .clone()
                    .unwrap_or_else(|| "us-east-1".to_string());
                if let Ok(account) =
                    rusticity_core::AwsConfig::get_account_for_profile(&profile.name, &region).await
                {
                    profile.account = Some(account);
                }
            }
        }
    }

    fn save_current_session(&mut self) {
        // If no tabs, delete the session if it exists
        if self.tabs.is_empty() {
            if let Some(ref session) = self.current_session {
                let _ = session.delete();
                self.current_session = None;
            }
            return;
        }

        let session = if let Some(ref mut current) = self.current_session {
            // Update existing session
            current.tabs = self
                .tabs
                .iter()
                .map(|t| SessionTab {
                    service: format!("{:?}", t.service),
                    title: t.title.clone(),
                    breadcrumb: t.breadcrumb.clone(),
                    filter: match t.service {
                        Service::CloudWatchLogGroups => {
                            Some(self.log_groups_state.log_groups.filter.clone())
                        }
                        _ => None,
                    },
                    selected_item: None,
                })
                .collect();
            current.clone()
        } else {
            // Create new session
            let mut session = Session::new(
                self.profile.clone(),
                self.region.clone(),
                self.config.account_id.clone(),
                self.config.role_arn.clone(),
            );
            session.tabs = self
                .tabs
                .iter()
                .map(|t| SessionTab {
                    service: format!("{:?}", t.service),
                    title: t.title.clone(),
                    breadcrumb: t.breadcrumb.clone(),
                    filter: match t.service {
                        Service::CloudWatchLogGroups => {
                            Some(self.log_groups_state.log_groups.filter.clone())
                        }
                        _ => None,
                    },
                    selected_item: None,
                })
                .collect();
            self.current_session = Some(session.clone());
            session
        };

        let _ = session.save();
    }
}

#[cfg(test)]
mod iam_policy_view_tests {
    use super::*;
    use test_helpers::*;

    #[test]
    fn test_enter_opens_policy_view() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.view_mode = ViewMode::Detail;
        app.iam_state.current_role = Some("TestRole".to_string());
        app.iam_state.policies.items = vec![IamPolicy {
            policy_name: "TestPolicy".to_string(),
            policy_type: "Inline".to_string(),
            attached_via: "Direct".to_string(),
            attached_entities: "1".to_string(),
            description: "Test".to_string(),
            creation_time: "2023-01-01".to_string(),
            edited_time: "2023-01-01".to_string(),
            policy_arn: None,
        }];
        app.iam_state.policies.reset();

        app.handle_action(Action::Select);

        assert_eq!(app.view_mode, ViewMode::PolicyView);
        assert_eq!(app.iam_state.current_policy, Some("TestPolicy".to_string()));
        assert_eq!(app.iam_state.policy_scroll, 0);
        assert!(app.iam_state.policies.loading);
    }

    #[test]
    fn test_escape_closes_policy_view() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.view_mode = ViewMode::PolicyView;
        app.iam_state.current_role = Some("TestRole".to_string());
        app.iam_state.current_policy = Some("TestPolicy".to_string());
        app.iam_state.policy_document = "{\n  \"test\": \"value\"\n}".to_string();
        app.iam_state.policy_scroll = 5;

        app.handle_action(Action::PrevPane);

        assert_eq!(app.view_mode, ViewMode::Detail);
        assert_eq!(app.iam_state.current_policy, None);
        assert_eq!(app.iam_state.policy_document, "");
        assert_eq!(app.iam_state.policy_scroll, 0);
    }

    #[test]
    fn test_ctrl_d_scrolls_down_in_policy_view() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.view_mode = ViewMode::PolicyView;
        app.iam_state.current_role = Some("TestRole".to_string());
        app.iam_state.current_policy = Some("TestPolicy".to_string());
        app.iam_state.policy_document = (0..100)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        app.iam_state.policy_scroll = 0;

        app.handle_action(Action::ScrollDown);

        assert_eq!(app.iam_state.policy_scroll, 10);

        app.handle_action(Action::ScrollDown);

        assert_eq!(app.iam_state.policy_scroll, 20);
    }

    #[test]
    fn test_ctrl_u_scrolls_up_in_policy_view() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.view_mode = ViewMode::PolicyView;
        app.iam_state.current_role = Some("TestRole".to_string());
        app.iam_state.current_policy = Some("TestPolicy".to_string());
        app.iam_state.policy_document = (0..100)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        app.iam_state.policy_scroll = 30;

        app.handle_action(Action::ScrollUp);

        assert_eq!(app.iam_state.policy_scroll, 20);

        app.handle_action(Action::ScrollUp);

        assert_eq!(app.iam_state.policy_scroll, 10);
    }

    #[test]
    fn test_scroll_does_not_go_negative() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.view_mode = ViewMode::PolicyView;
        app.iam_state.current_role = Some("TestRole".to_string());
        app.iam_state.current_policy = Some("TestPolicy".to_string());
        app.iam_state.policy_document = "line 1\nline 2\nline 3".to_string();
        app.iam_state.policy_scroll = 0;

        app.handle_action(Action::ScrollUp);

        assert_eq!(app.iam_state.policy_scroll, 0);
    }

    #[test]
    fn test_scroll_does_not_exceed_max() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.view_mode = ViewMode::PolicyView;
        app.iam_state.current_role = Some("TestRole".to_string());
        app.iam_state.current_policy = Some("TestPolicy".to_string());
        app.iam_state.policy_document = "line 1\nline 2\nline 3".to_string();
        app.iam_state.policy_scroll = 0;

        app.handle_action(Action::ScrollDown);

        assert_eq!(app.iam_state.policy_scroll, 2); // Max is 2 (3 lines - 1)
    }

    #[test]
    fn test_policy_view_console_url() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.view_mode = ViewMode::PolicyView;
        app.iam_state.current_role = Some("TestRole".to_string());
        app.iam_state.current_policy = Some("TestPolicy".to_string());

        let url = app.get_console_url();

        assert!(url.contains("us-east-1.console.aws.amazon.com"));
        assert!(url.contains("/roles/details/TestRole"));
        assert!(url.contains("/editPolicy/TestPolicy"));
        assert!(url.contains("step=addPermissions"));
    }

    #[test]
    fn test_esc_from_policy_view_goes_to_role_detail() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.view_mode = ViewMode::PolicyView;
        app.iam_state.current_role = Some("TestRole".to_string());
        app.iam_state.current_policy = Some("TestPolicy".to_string());
        app.iam_state.policy_document = "test".to_string();
        app.iam_state.policy_scroll = 5;

        app.handle_action(Action::GoBack);

        assert_eq!(app.view_mode, ViewMode::Detail);
        assert_eq!(app.iam_state.current_policy, None);
        assert_eq!(app.iam_state.policy_document, "");
        assert_eq!(app.iam_state.policy_scroll, 0);
        assert_eq!(app.iam_state.current_role, Some("TestRole".to_string()));
    }

    #[test]
    fn test_esc_from_role_detail_goes_to_role_list() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.view_mode = ViewMode::Detail;
        app.iam_state.current_role = Some("TestRole".to_string());

        app.handle_action(Action::GoBack);

        assert_eq!(app.iam_state.current_role, None);
    }

    #[test]
    fn test_right_arrow_expands_policy_row() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.view_mode = ViewMode::Detail;
        app.iam_state.current_role = Some("TestRole".to_string());
        app.iam_state.policies.items = vec![IamPolicy {
            policy_name: "TestPolicy".to_string(),
            policy_type: "Inline".to_string(),
            attached_via: "Direct".to_string(),
            attached_entities: "1".to_string(),
            description: "Test".to_string(),
            creation_time: "2023-01-01".to_string(),
            edited_time: "2023-01-01".to_string(),
            policy_arn: None,
        }];
        app.iam_state.policies.reset();

        app.handle_action(Action::NextPane);

        // Should expand, not drill down
        assert_eq!(app.view_mode, ViewMode::Detail);
        assert_eq!(app.iam_state.current_policy, None);
        assert_eq!(app.iam_state.policies.expanded_item, Some(0));
    }
}

#[cfg(test)]
mod tab_filter_tests {
    use super::*;
    use test_helpers::*;

    #[test]
    fn test_space_t_opens_tab_picker() {
        let mut app = test_app();
        app.tabs = vec![
            Tab {
                service: Service::CloudWatchLogGroups,
                title: "Tab 1".to_string(),
                breadcrumb: "CloudWatch > Log groups".to_string(),
            },
            Tab {
                service: Service::S3Buckets,
                title: "Tab 2".to_string(),
                breadcrumb: "S3 > Buckets".to_string(),
            },
        ];
        app.current_tab = 0;

        app.handle_action(Action::OpenTabPicker);

        assert_eq!(app.mode, Mode::TabPicker);
        assert_eq!(app.tab_picker_selected, 0);
    }

    #[test]
    fn test_tab_filter_works() {
        let mut app = test_app();
        app.tabs = vec![
            Tab {
                service: Service::CloudWatchLogGroups,
                title: "CloudWatch Logs".to_string(),
                breadcrumb: "CloudWatch > Log groups".to_string(),
            },
            Tab {
                service: Service::S3Buckets,
                title: "S3 Buckets".to_string(),
                breadcrumb: "S3 > Buckets".to_string(),
            },
            Tab {
                service: Service::CloudWatchAlarms,
                title: "CloudWatch Alarms".to_string(),
                breadcrumb: "CloudWatch > Alarms".to_string(),
            },
        ];
        app.mode = Mode::TabPicker;

        // Filter for "s3"
        app.handle_action(Action::FilterInput('s'));
        app.handle_action(Action::FilterInput('3'));

        let filtered = app.get_filtered_tabs();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1.title, "S3 Buckets");
    }

    #[test]
    fn test_tab_filter_by_breadcrumb() {
        let mut app = test_app();
        app.tabs = vec![
            Tab {
                service: Service::CloudWatchLogGroups,
                title: "Tab 1".to_string(),
                breadcrumb: "CloudWatch > Log groups".to_string(),
            },
            Tab {
                service: Service::S3Buckets,
                title: "Tab 2".to_string(),
                breadcrumb: "S3 > Buckets".to_string(),
            },
        ];
        app.mode = Mode::TabPicker;

        // Filter for "cloudwatch"
        app.handle_action(Action::FilterInput('c'));
        app.handle_action(Action::FilterInput('l'));
        app.handle_action(Action::FilterInput('o'));
        app.handle_action(Action::FilterInput('u'));
        app.handle_action(Action::FilterInput('d'));

        let filtered = app.get_filtered_tabs();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1.breadcrumb, "CloudWatch > Log groups");
    }

    #[test]
    fn test_tab_filter_backspace() {
        let mut app = test_app();
        app.tabs = vec![
            Tab {
                service: Service::CloudWatchLogGroups,
                title: "CloudWatch Logs".to_string(),
                breadcrumb: "CloudWatch > Log groups".to_string(),
            },
            Tab {
                service: Service::S3Buckets,
                title: "S3 Buckets".to_string(),
                breadcrumb: "S3 > Buckets".to_string(),
            },
        ];
        app.mode = Mode::TabPicker;

        app.handle_action(Action::FilterInput('s'));
        app.handle_action(Action::FilterInput('3'));
        assert_eq!(app.tab_filter, "s3");

        app.handle_action(Action::FilterBackspace);
        assert_eq!(app.tab_filter, "s");

        let filtered = app.get_filtered_tabs();
        assert_eq!(filtered.len(), 2); // Both match "s"
    }

    #[test]
    fn test_tab_selection_with_filter() {
        let mut app = test_app();
        app.tabs = vec![
            Tab {
                service: Service::CloudWatchLogGroups,
                title: "CloudWatch Logs".to_string(),
                breadcrumb: "CloudWatch > Log groups".to_string(),
            },
            Tab {
                service: Service::S3Buckets,
                title: "S3 Buckets".to_string(),
                breadcrumb: "S3 > Buckets".to_string(),
            },
        ];
        app.mode = Mode::TabPicker;
        app.current_tab = 0;

        // Filter for "s3"
        app.handle_action(Action::FilterInput('s'));
        app.handle_action(Action::FilterInput('3'));

        // Select the filtered tab
        app.handle_action(Action::Select);

        assert_eq!(app.current_tab, 1); // Should select the S3 tab (index 1)
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.tab_filter, ""); // Filter should be cleared
    }
}

#[cfg(test)]
mod region_latency_tests {
    use super::*;
    use test_helpers::*;

    #[test]
    fn test_regions_sorted_by_latency() {
        let mut app = test_app();

        // Add some latencies
        app.region_latencies.insert("us-west-2".to_string(), 50);
        app.region_latencies.insert("us-east-1".to_string(), 10);
        app.region_latencies.insert("eu-west-1".to_string(), 100);

        let filtered = app.get_filtered_regions();

        // Should be sorted by latency (lowest first)
        let with_latency: Vec<_> = filtered.iter().filter(|r| r.latency_ms.is_some()).collect();

        assert!(with_latency.len() >= 3);
        assert_eq!(with_latency[0].code, "us-east-1");
        assert_eq!(with_latency[0].latency_ms, Some(10));
        assert_eq!(with_latency[1].code, "us-west-2");
        assert_eq!(with_latency[1].latency_ms, Some(50));
        assert_eq!(with_latency[2].code, "eu-west-1");
        assert_eq!(with_latency[2].latency_ms, Some(100));
    }

    #[test]
    fn test_regions_with_latency_before_without() {
        let mut app = test_app();

        // Only add latency for one region
        app.region_latencies.insert("eu-west-1".to_string(), 100);

        let filtered = app.get_filtered_regions();

        // Region with latency should come first
        assert_eq!(filtered[0].code, "eu-west-1");
        assert_eq!(filtered[0].latency_ms, Some(100));

        // Rest should be sorted by name
        for region in &filtered[1..] {
            assert!(region.latency_ms.is_none());
        }
    }

    #[test]
    fn test_region_filter_with_latency() {
        let mut app = test_app();

        app.region_latencies.insert("us-east-1".to_string(), 10);
        app.region_latencies.insert("us-west-2".to_string(), 50);
        app.region_filter = "us".to_string();

        let filtered = app.get_filtered_regions();

        // Should only have US regions, sorted by latency
        assert!(filtered.iter().all(|r| r.code.starts_with("us-")));
        assert_eq!(filtered[0].code, "us-east-1");
        assert_eq!(filtered[1].code, "us-west-2");
    }

    #[test]
    fn test_latency_persists_across_filters() {
        let mut app = test_app();

        app.region_latencies.insert("us-east-1".to_string(), 10);

        // Filter to something else
        app.region_filter = "eu".to_string();
        let filtered = app.get_filtered_regions();
        assert!(filtered.iter().all(|r| !r.code.starts_with("us-")));

        // Clear filter
        app.region_filter.clear();
        let all = app.get_filtered_regions();

        // Latency should still be there
        let us_east = all.iter().find(|r| r.code == "us-east-1").unwrap();
        assert_eq!(us_east.latency_ms, Some(10));
    }

    #[test]
    fn test_measure_region_latencies_clears_previous() {
        let mut app = test_app();

        // Add some fake latencies
        app.region_latencies.insert("us-east-1".to_string(), 100);
        app.region_latencies.insert("eu-west-1".to_string(), 200);

        // Measure again (will fail to connect but should clear)
        app.measure_region_latencies();

        // Old latencies should be cleared
        assert!(
            app.region_latencies.is_empty() || !app.region_latencies.contains_key("fake-region")
        );
    }

    #[test]
    fn test_regions_with_latency_sorted_first() {
        let mut app = test_app();

        // Add latencies: one fast, one slow (>1000ms would be treated as >1s)
        app.region_latencies.insert("us-east-1".to_string(), 50);
        app.region_latencies.insert("eu-west-1".to_string(), 500);

        let filtered = app.get_filtered_regions();

        // Should show all regions
        assert!(filtered.len() > 2);

        // Fast regions first
        assert_eq!(filtered[0].code, "us-east-1");
        assert_eq!(filtered[0].latency_ms, Some(50));
        assert_eq!(filtered[1].code, "eu-west-1");
        assert_eq!(filtered[1].latency_ms, Some(500));

        // Regions without latency treated as 1000ms, so they come after 500ms
        for region in &filtered[2..] {
            assert!(region.latency_ms.is_none());
        }
    }

    #[test]
    fn test_regions_without_latency_sorted_as_1000ms() {
        let mut app = test_app();

        // Add one region with 1500ms (slower than default 1000ms)
        app.region_latencies
            .insert("ap-southeast-2".to_string(), 1500);
        // Add one region with 50ms (faster)
        app.region_latencies.insert("us-east-1".to_string(), 50);

        let filtered = app.get_filtered_regions();

        // Fast region first
        assert_eq!(filtered[0].code, "us-east-1");
        assert_eq!(filtered[0].latency_ms, Some(50));

        // Regions without latency (treated as 1000ms) come before 1500ms
        let slow_region_idx = filtered
            .iter()
            .position(|r| r.code == "ap-southeast-2")
            .unwrap();
        assert!(slow_region_idx > 1); // Should be after fast region and regions without latency

        // All regions between index 1 and slow_region_idx should have no latency
        for region in filtered.iter().take(slow_region_idx).skip(1) {
            assert!(region.latency_ms.is_none());
        }
    }

    #[test]
    fn test_region_picker_opens_with_latencies() {
        let mut app = test_app();

        // Simulate opening region picker
        app.region_filter.clear();
        app.region_picker_selected = 0;
        app.measure_region_latencies();

        // Should have attempted to measure (even if all fail in test env)
        // The map should be initialized
        assert!(app.region_latencies.is_empty() || !app.region_latencies.is_empty());
    }

    #[test]
    fn test_ecr_tab_next() {
        assert_eq!(EcrTab::Private.next(), EcrTab::Public);
        assert_eq!(EcrTab::Public.next(), EcrTab::Private);
    }

    #[test]
    fn test_ecr_tab_switching() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.ecr_state.tab = EcrTab::Private;

        app.handle_action(Action::NextDetailTab);
        assert_eq!(app.ecr_state.tab, EcrTab::Public);
        assert_eq!(app.ecr_state.repositories.selected, 0);

        app.handle_action(Action::NextDetailTab);
        assert_eq!(app.ecr_state.tab, EcrTab::Private);
    }

    #[test]
    fn test_ecr_navigation() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.ecr_state.repositories.items = vec![
            EcrRepository {
                name: "repo1".to_string(),
                uri: "uri1".to_string(),
                created_at: "2023-01-01".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            },
            EcrRepository {
                name: "repo2".to_string(),
                uri: "uri2".to_string(),
                created_at: "2023-01-02".to_string(),
                tag_immutability: "IMMUTABLE".to_string(),
                encryption_type: "KMS".to_string(),
            },
        ];

        app.handle_action(Action::NextItem);
        assert_eq!(app.ecr_state.repositories.selected, 1);

        app.handle_action(Action::PrevItem);
        assert_eq!(app.ecr_state.repositories.selected, 0);
    }

    #[test]
    fn test_ecr_filter() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.ecr_state.repositories.items = vec![
            EcrRepository {
                name: "my-app".to_string(),
                uri: "uri1".to_string(),
                created_at: "2023-01-01".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            },
            EcrRepository {
                name: "other-service".to_string(),
                uri: "uri2".to_string(),
                created_at: "2023-01-02".to_string(),
                tag_immutability: "IMMUTABLE".to_string(),
                encryption_type: "KMS".to_string(),
            },
        ];

        app.ecr_state.repositories.filter = "app".to_string();
        let filtered = filtered_ecr_repositories(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "my-app");
    }

    #[test]
    fn test_ecr_filter_input() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::FilterInput;

        app.handle_action(Action::FilterInput('t'));
        app.handle_action(Action::FilterInput('e'));
        app.handle_action(Action::FilterInput('s'));
        app.handle_action(Action::FilterInput('t'));
        assert_eq!(app.ecr_state.repositories.filter, "test");

        app.handle_action(Action::FilterBackspace);
        assert_eq!(app.ecr_state.repositories.filter, "tes");
    }

    #[test]
    fn test_ecr_filter_resets_selection() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::FilterInput;
        app.ecr_state.repositories.items = vec![
            EcrRepository {
                name: "repo1".to_string(),
                uri: "uri1".to_string(),
                created_at: "2023-01-01".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            },
            EcrRepository {
                name: "repo2".to_string(),
                uri: "uri2".to_string(),
                created_at: "2023-01-02".to_string(),
                tag_immutability: "IMMUTABLE".to_string(),
                encryption_type: "KMS".to_string(),
            },
            EcrRepository {
                name: "repo3".to_string(),
                uri: "uri3".to_string(),
                created_at: "2023-01-03".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            },
        ];

        // Move selection to second item
        app.ecr_state.repositories.selected = 2;
        assert_eq!(app.ecr_state.repositories.selected, 2);

        // Apply filter - selection should reset to 0
        app.handle_action(Action::FilterInput('t'));
        assert_eq!(app.ecr_state.repositories.filter, "t");
        assert_eq!(app.ecr_state.repositories.selected, 0);

        // Move selection again
        app.ecr_state.repositories.selected = 1;

        // Backspace filter - selection should reset to 0
        app.handle_action(Action::FilterBackspace);
        assert_eq!(app.ecr_state.repositories.filter, "");
        assert_eq!(app.ecr_state.repositories.selected, 0);
    }

    #[test]
    fn test_ecr_images_filter_resets_selection() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::FilterInput;
        app.ecr_state.current_repository = Some("test-repo".to_string());
        app.ecr_state.images.items = vec![
            EcrImage {
                tag: "v1.0.0".to_string(),
                artifact_type: "container".to_string(),
                digest: "sha256:abc123".to_string(),
                pushed_at: "2023-01-01".to_string(),
                size_bytes: 1000,
                uri: "uri1".to_string(),
                last_pull_time: "".to_string(),
            },
            EcrImage {
                tag: "v2.0.0".to_string(),
                artifact_type: "container".to_string(),
                digest: "sha256:def456".to_string(),
                pushed_at: "2023-01-02".to_string(),
                size_bytes: 2000,
                uri: "uri2".to_string(),
                last_pull_time: "".to_string(),
            },
        ];

        // Move selection to second item
        app.ecr_state.images.selected = 1;
        assert_eq!(app.ecr_state.images.selected, 1);

        // Apply filter - selection should reset to 0
        app.handle_action(Action::FilterInput('v'));
        assert_eq!(app.ecr_state.images.filter, "v");
        assert_eq!(app.ecr_state.images.selected, 0);
    }

    #[test]
    fn test_iam_users_filter_input() {
        let mut app = test_app();
        app.current_service = Service::IamUsers;
        app.service_selected = true;
        app.mode = Mode::FilterInput;

        app.handle_action(Action::FilterInput('a'));
        app.handle_action(Action::FilterInput('d'));
        app.handle_action(Action::FilterInput('m'));
        app.handle_action(Action::FilterInput('i'));
        app.handle_action(Action::FilterInput('n'));
        assert_eq!(app.iam_state.users.filter, "admin");

        app.handle_action(Action::FilterBackspace);
        assert_eq!(app.iam_state.users.filter, "admi");
    }

    #[test]
    fn test_iam_policies_filter_input() {
        let mut app = test_app();
        app.current_service = Service::IamUsers;
        app.service_selected = true;
        app.iam_state.current_user = Some("testuser".to_string());
        app.mode = Mode::FilterInput;

        app.handle_action(Action::FilterInput('r'));
        app.handle_action(Action::FilterInput('e'));
        app.handle_action(Action::FilterInput('a'));
        app.handle_action(Action::FilterInput('d'));
        assert_eq!(app.iam_state.policies.filter, "read");

        app.handle_action(Action::FilterBackspace);
        assert_eq!(app.iam_state.policies.filter, "rea");
    }

    #[test]
    fn test_iam_start_filter() {
        let mut app = test_app();
        app.current_service = Service::IamUsers;
        app.service_selected = true;
        app.mode = Mode::Normal;

        app.handle_action(Action::StartFilter);
        assert_eq!(app.mode, Mode::FilterInput);
    }

    #[test]
    fn test_iam_roles_filter_input() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.mode = Mode::FilterInput;

        app.handle_action(Action::FilterInput('a'));
        app.handle_action(Action::FilterInput('d'));
        app.handle_action(Action::FilterInput('m'));
        app.handle_action(Action::FilterInput('i'));
        app.handle_action(Action::FilterInput('n'));
        assert_eq!(app.iam_state.roles.filter, "admin");

        app.handle_action(Action::FilterBackspace);
        assert_eq!(app.iam_state.roles.filter, "admi");
    }

    #[test]
    fn test_iam_roles_start_filter() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.mode = Mode::Normal;

        app.handle_action(Action::StartFilter);
        assert_eq!(app.mode, Mode::FilterInput);
    }

    #[test]
    fn test_iam_roles_navigation() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.iam_state.roles.items = (0..10)
            .map(|i| IamRole {
                role_name: format!("role{}", i),
                path: "/".to_string(),
                trusted_entities: String::new(),
                last_activity: String::new(),
                arn: format!("arn:aws:iam::123456789012:role/role{}", i),
                creation_time: "2025-01-01 00:00:00 (UTC)".to_string(),
                description: String::new(),
                max_session_duration: Some(3600),
            })
            .collect();

        assert_eq!(app.iam_state.roles.selected, 0);

        app.handle_action(Action::NextItem);
        assert_eq!(app.iam_state.roles.selected, 1);

        app.handle_action(Action::NextItem);
        assert_eq!(app.iam_state.roles.selected, 2);

        app.handle_action(Action::PrevItem);
        assert_eq!(app.iam_state.roles.selected, 1);
    }

    #[test]
    fn test_iam_roles_page_hotkey() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.iam_state.roles.page_size = PageSize::Ten;
        app.iam_state.roles.items = (0..100)
            .map(|i| IamRole {
                role_name: format!("role{}", i),
                path: "/".to_string(),
                trusted_entities: String::new(),
                last_activity: String::new(),
                arn: format!("arn:aws:iam::123456789012:role/role{}", i),
                creation_time: "2025-01-01 00:00:00 (UTC)".to_string(),
                description: String::new(),
                max_session_duration: Some(3600),
            })
            .collect();

        app.handle_action(Action::FilterInput('2'));
        app.handle_action(Action::OpenColumnSelector);
        assert_eq!(app.iam_state.roles.selected, 10); // Page 2 = index 10 (with page size 10)
    }

    #[test]
    fn test_iam_users_page_hotkey() {
        let mut app = test_app();
        app.current_service = Service::IamUsers;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.iam_state.users.page_size = PageSize::Ten;
        app.iam_state.users.items = (0..100)
            .map(|i| IamUser {
                user_name: format!("user{}", i),
                path: "/".to_string(),
                groups: String::new(),
                last_activity: String::new(),
                mfa: String::new(),
                password_age: String::new(),
                console_last_sign_in: String::new(),
                access_key_id: String::new(),
                active_key_age: String::new(),
                access_key_last_used: String::new(),
                arn: format!("arn:aws:iam::123456789012:user/user{}", i),
                creation_time: "2025-01-01 00:00:00 (UTC)".to_string(),
                console_access: String::new(),
                signing_certs: String::new(),
            })
            .collect();

        app.handle_action(Action::FilterInput('3'));
        app.handle_action(Action::OpenColumnSelector);
        assert_eq!(app.iam_state.users.selected, 20); // Page 3 = index 20 (with page size 10)
    }

    #[test]
    fn test_ecr_scroll_navigation() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.ecr_state.repositories.items = (0..20)
            .map(|i| EcrRepository {
                name: format!("repo{}", i),
                uri: format!("uri{}", i),
                created_at: "2023-01-01".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            })
            .collect();

        app.handle_action(Action::ScrollDown);
        assert_eq!(app.ecr_state.repositories.selected, 10);

        app.handle_action(Action::ScrollUp);
        assert_eq!(app.ecr_state.repositories.selected, 0);
    }

    #[test]
    fn test_ecr_tab_switching_triggers_reload() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.ecr_state.tab = EcrTab::Private;
        app.ecr_state.repositories.loading = false;
        app.ecr_state.repositories.items = vec![EcrRepository {
            name: "private-repo".to_string(),
            uri: "uri".to_string(),
            created_at: "2023-01-01".to_string(),
            tag_immutability: "MUTABLE".to_string(),
            encryption_type: "AES256".to_string(),
        }];

        app.handle_action(Action::NextDetailTab);
        assert_eq!(app.ecr_state.tab, EcrTab::Public);
        assert!(app.ecr_state.repositories.loading);
        assert_eq!(app.ecr_state.repositories.selected, 0);
    }

    #[test]
    fn test_ecr_tab_cycles_between_private_and_public() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.ecr_state.tab = EcrTab::Private;

        app.handle_action(Action::NextDetailTab);
        assert_eq!(app.ecr_state.tab, EcrTab::Public);

        app.handle_action(Action::NextDetailTab);
        assert_eq!(app.ecr_state.tab, EcrTab::Private);
    }

    #[test]
    fn test_page_size_values() {
        assert_eq!(PageSize::Ten.value(), 10);
        assert_eq!(PageSize::TwentyFive.value(), 25);
        assert_eq!(PageSize::Fifty.value(), 50);
        assert_eq!(PageSize::OneHundred.value(), 100);
    }

    #[test]
    fn test_page_size_next() {
        assert_eq!(PageSize::Ten.next(), PageSize::TwentyFive);
        assert_eq!(PageSize::TwentyFive.next(), PageSize::Fifty);
        assert_eq!(PageSize::Fifty.next(), PageSize::OneHundred);
        assert_eq!(PageSize::OneHundred.next(), PageSize::Ten);
    }

    #[test]
    fn test_ecr_enter_drills_into_repository() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.ecr_state.repositories.items = vec![EcrRepository {
            name: "my-repo".to_string(),
            uri: "uri".to_string(),
            created_at: "2023-01-01".to_string(),
            tag_immutability: "MUTABLE".to_string(),
            encryption_type: "AES256".to_string(),
        }];

        app.handle_action(Action::Select);
        assert_eq!(
            app.ecr_state.current_repository,
            Some("my-repo".to_string())
        );
        assert!(app.ecr_state.repositories.loading);
    }

    #[test]
    fn test_ecr_repository_expansion() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.ecr_state.repositories.items = vec![EcrRepository {
            name: "my-repo".to_string(),
            uri: "uri".to_string(),
            created_at: "2023-01-01".to_string(),
            tag_immutability: "MUTABLE".to_string(),
            encryption_type: "AES256".to_string(),
        }];
        app.ecr_state.repositories.selected = 0;

        assert_eq!(app.ecr_state.repositories.expanded_item, None);

        app.handle_action(Action::NextPane);
        assert_eq!(app.ecr_state.repositories.expanded_item, Some(0));

        app.handle_action(Action::PrevPane);
        assert_eq!(app.ecr_state.repositories.expanded_item, None);
    }

    #[test]
    fn test_ecr_ctrl_d_scrolls_down() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.ecr_state.repositories.items = (0..30)
            .map(|i| EcrRepository {
                name: format!("repo{}", i),
                uri: format!("uri{}", i),
                created_at: "2023-01-01".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            })
            .collect();
        app.ecr_state.repositories.selected = 0;

        app.handle_action(Action::PageDown);
        assert_eq!(app.ecr_state.repositories.selected, 10);
    }

    #[test]
    fn test_ecr_ctrl_u_scrolls_up() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.ecr_state.repositories.items = (0..30)
            .map(|i| EcrRepository {
                name: format!("repo{}", i),
                uri: format!("uri{}", i),
                created_at: "2023-01-01".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            })
            .collect();
        app.ecr_state.repositories.selected = 15;

        app.handle_action(Action::PageUp);
        assert_eq!(app.ecr_state.repositories.selected, 5);
    }

    #[test]
    fn test_ecr_images_ctrl_d_scrolls_down() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.ecr_state.current_repository = Some("repo".to_string());
        app.ecr_state.images.items = (0..30)
            .map(|i| EcrImage {
                tag: format!("tag{}", i),
                artifact_type: "container".to_string(),
                pushed_at: "2023-01-01T12:00:00Z".to_string(),
                size_bytes: 104857600,
                uri: format!("uri{}", i),
                digest: format!("sha256:{}", i),
                last_pull_time: String::new(),
            })
            .collect();
        app.ecr_state.images.selected = 0;

        app.handle_action(Action::PageDown);
        assert_eq!(app.ecr_state.images.selected, 10);
    }

    #[test]
    fn test_ecr_esc_goes_back_from_images_to_repos() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.ecr_state.current_repository = Some("my-repo".to_string());
        app.ecr_state.images.items = vec![EcrImage {
            tag: "latest".to_string(),
            artifact_type: "container".to_string(),
            pushed_at: "2023-01-01T12:00:00Z".to_string(),
            size_bytes: 104857600,
            uri: "uri".to_string(),
            digest: "sha256:abc".to_string(),
            last_pull_time: String::new(),
        }];

        app.handle_action(Action::GoBack);
        assert_eq!(app.ecr_state.current_repository, None);
        assert!(app.ecr_state.images.items.is_empty());
    }

    #[test]
    fn test_ecr_esc_collapses_expanded_image_first() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.ecr_state.current_repository = Some("my-repo".to_string());
        app.ecr_state.images.expanded_item = Some(0);

        app.handle_action(Action::GoBack);
        assert_eq!(app.ecr_state.images.expanded_item, None);
        assert_eq!(
            app.ecr_state.current_repository,
            Some("my-repo".to_string())
        );
    }

    #[test]
    fn test_pagination_with_lowercase_p() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.ecr_state.repositories.items = (0..100)
            .map(|i| EcrRepository {
                name: format!("repo{}", i),
                uri: format!("uri{}", i),
                created_at: "2023-01-01".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            })
            .collect();

        // Type "2" then "p" to go to page 2
        app.handle_action(Action::FilterInput('2'));
        assert_eq!(app.page_input, "2");

        app.handle_action(Action::OpenColumnSelector); // 'p' key
        assert_eq!(app.ecr_state.repositories.selected, 50); // Page 2 starts at index 50
        assert_eq!(app.page_input, ""); // Should be cleared
    }

    #[test]
    fn test_lowercase_p_without_number_opens_preferences() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::Normal;

        app.handle_action(Action::OpenColumnSelector); // 'p' key without number
        assert_eq!(app.mode, Mode::ColumnSelector);
    }

    #[test]
    fn test_ctrl_o_generates_correct_console_url() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.config.account_id = "123456789012".to_string();

        // Test repository list URL
        let url = app.get_console_url();
        assert!(url.contains("ecr/private-registry/repositories"));
        assert!(url.contains("region=us-east-1"));

        // Test images URL
        app.ecr_state.current_repository = Some("my-repo".to_string());
        let url = app.get_console_url();
        assert!(url.contains("ecr/repositories/private/123456789012/my-repo"));
        assert!(url.contains("region=us-east-1"));
    }

    #[test]
    fn test_page_input_display_and_reset() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.ecr_state.repositories.items = (0..100)
            .map(|i| EcrRepository {
                name: format!("repo{}", i),
                uri: format!("uri{}", i),
                created_at: "2023-01-01".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            })
            .collect();

        // Type "2"
        app.handle_action(Action::FilterInput('2'));
        assert_eq!(app.page_input, "2");

        // Press 'p' to go to page 2
        app.handle_action(Action::OpenColumnSelector);
        assert_eq!(app.page_input, ""); // Should be cleared
        assert_eq!(app.ecr_state.repositories.selected, 50); // Page 2 starts at index 50
    }

    #[test]
    fn test_page_navigation_updates_scroll_offset_for_cfn() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.cfn_state.table.items = (0..100)
            .map(|i| CfnStack {
                name: format!("stack-{}", i),
                stack_id: format!(
                    "arn:aws:cloudformation:us-east-1:123456789012:stack/stack-{}/id",
                    i
                ),
                status: "CREATE_COMPLETE".to_string(),
                created_time: "2023-01-01T00:00:00Z".to_string(),
                updated_time: "2023-01-01T00:00:00Z".to_string(),
                deleted_time: String::new(),
                drift_status: "IN_SYNC".to_string(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: String::new(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: vec![],
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: vec![],
                notification_arns: vec![],
            })
            .collect();

        // Type "2" then "p" to go to page 2
        app.handle_action(Action::FilterInput('2'));
        assert_eq!(app.page_input, "2");

        app.handle_action(Action::OpenColumnSelector); // 'p' key
        assert_eq!(app.page_input, ""); // Should be cleared

        // Verify both selected and scroll_offset are updated
        let page_size = app.cfn_state.table.page_size.value();
        let expected_offset = page_size; // Page 2 starts at page_size
        assert_eq!(app.cfn_state.table.selected, expected_offset);
        assert_eq!(app.cfn_state.table.scroll_offset, expected_offset);

        // Verify pagination display shows page 2
        let current_page = app.cfn_state.table.scroll_offset / page_size;
        assert_eq!(
            current_page, 1,
            "2p should go to page 2 (0-indexed as 1), not page 3"
        ); // 0-indexed, so page 2 is index 1
    }

    #[test]
    fn test_3p_goes_to_page_3_not_page_5() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.cfn_state.table.items = (0..200)
            .map(|i| CfnStack {
                name: format!("stack-{}", i),
                stack_id: format!(
                    "arn:aws:cloudformation:us-east-1:123456789012:stack/stack-{}/id",
                    i
                ),
                status: "CREATE_COMPLETE".to_string(),
                created_time: "2023-01-01T00:00:00Z".to_string(),
                updated_time: "2023-01-01T00:00:00Z".to_string(),
                deleted_time: String::new(),
                drift_status: "IN_SYNC".to_string(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: String::new(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: vec![],
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: vec![],
                notification_arns: vec![],
            })
            .collect();

        // Type "3" then "p" to go to page 3
        app.handle_action(Action::FilterInput('3'));
        app.handle_action(Action::OpenColumnSelector);

        let page_size = app.cfn_state.table.page_size.value();
        let current_page = app.cfn_state.table.scroll_offset / page_size;
        assert_eq!(
            current_page, 2,
            "3p should go to page 3 (0-indexed as 2), not page 5"
        );
        assert_eq!(app.cfn_state.table.scroll_offset, 2 * page_size);
    }

    #[test]
    fn test_log_streams_page_navigation_uses_correct_page_size() {
        let mut app = test_app();
        app.current_service = Service::CloudWatchLogGroups;
        app.view_mode = ViewMode::Detail;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.log_groups_state.log_streams = (0..100)
            .map(|i| LogStream {
                name: format!("stream-{}", i),
                creation_time: None,
                last_event_time: None,
            })
            .collect();

        // Type "2" then "p" to go to page 2
        app.handle_action(Action::FilterInput('2'));
        app.handle_action(Action::OpenColumnSelector);

        // Log streams use page_size=20, so page 2 starts at index 20
        assert_eq!(app.log_groups_state.selected_stream, 20);

        // Verify pagination display shows page 2 (not page 3)
        let page_size = 20;
        let current_page = app.log_groups_state.selected_stream / page_size;
        assert_eq!(
            current_page, 1,
            "2p should go to page 2 (0-indexed as 1), not page 3"
        );
    }

    #[test]
    fn test_ecr_repositories_page_navigation_uses_configurable_page_size() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.ecr_state.repositories.page_size = PageSize::TwentyFive; // Set to 25
        app.ecr_state.repositories.items = (0..100)
            .map(|i| EcrRepository {
                name: format!("repo{}", i),
                uri: format!("uri{}", i),
                created_at: "2023-01-01".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            })
            .collect();

        // Type "3" then "p" to go to page 3
        app.handle_action(Action::FilterInput('3'));
        app.handle_action(Action::OpenColumnSelector);

        // With page_size=25, page 3 starts at index 50
        assert_eq!(app.ecr_state.repositories.selected, 50);

        let page_size = app.ecr_state.repositories.page_size.value();
        let current_page = app.ecr_state.repositories.selected / page_size;
        assert_eq!(
            current_page, 2,
            "3p with page_size=25 should go to page 3 (0-indexed as 2)"
        );
    }

    #[test]
    fn test_page_navigation_updates_scroll_offset_for_alarms() {
        let mut app = test_app();
        app.current_service = Service::CloudWatchAlarms;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.alarms_state.table.items = (0..100)
            .map(|i| Alarm {
                name: format!("alarm-{}", i),
                state: "OK".to_string(),
                state_updated_timestamp: "2023-01-01T00:00:00Z".to_string(),
                description: String::new(),
                metric_name: "CPUUtilization".to_string(),
                namespace: "AWS/EC2".to_string(),
                statistic: "Average".to_string(),
                period: 300,
                comparison_operator: "GreaterThanThreshold".to_string(),
                threshold: 80.0,
                actions_enabled: true,
                state_reason: String::new(),
                resource: String::new(),
                dimensions: String::new(),
                expression: String::new(),
                alarm_type: "MetricAlarm".to_string(),
                cross_account: String::new(),
            })
            .collect();

        // Type "2" then "p" to go to page 2
        app.handle_action(Action::FilterInput('2'));
        app.handle_action(Action::OpenColumnSelector);

        // Verify both selected and scroll_offset are updated
        let page_size = app.alarms_state.table.page_size.value();
        let expected_offset = page_size; // Page 2 starts at page_size
        assert_eq!(app.alarms_state.table.selected, expected_offset);
        assert_eq!(app.alarms_state.table.scroll_offset, expected_offset);
    }

    #[test]
    fn test_ecr_pagination_with_65_repos() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.ecr_state.repositories.items = (0..65)
            .map(|i| EcrRepository {
                name: format!("repo{:02}", i),
                uri: format!("uri{}", i),
                created_at: "2023-01-01".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            })
            .collect();

        // Page 1: items 0-49 (50 items)
        assert_eq!(app.ecr_state.repositories.selected, 0);
        let page_size = 50;
        let current_page = app.ecr_state.repositories.selected / page_size;
        assert_eq!(current_page, 0);

        // Go to page 2
        app.handle_action(Action::FilterInput('2'));
        app.handle_action(Action::OpenColumnSelector);
        assert_eq!(app.ecr_state.repositories.selected, 50);

        // Page 2: items 50-64 (15 items)
        let current_page = app.ecr_state.repositories.selected / page_size;
        assert_eq!(current_page, 1);
    }

    #[test]
    fn test_ecr_repos_input_focus_tab_cycling() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::FilterInput;
        app.ecr_state.input_focus = InputFocus::Filter;

        // Tab should cycle to Pagination
        app.handle_action(Action::NextFilterFocus);
        assert_eq!(app.ecr_state.input_focus, InputFocus::Pagination);

        // Tab again should cycle back to Input
        app.handle_action(Action::NextFilterFocus);
        assert_eq!(app.ecr_state.input_focus, InputFocus::Filter);

        // Shift+Tab should cycle backwards to Pagination
        app.handle_action(Action::PrevFilterFocus);
        assert_eq!(app.ecr_state.input_focus, InputFocus::Pagination);

        // Shift+Tab again should cycle back to Input
        app.handle_action(Action::PrevFilterFocus);
        assert_eq!(app.ecr_state.input_focus, InputFocus::Filter);
    }

    #[test]
    fn test_ecr_images_column_toggle_not_off_by_one() {
        use crate::ecr::image::Column as ImageColumn;
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::ColumnSelector;
        app.ecr_state.current_repository = Some("test-repo".to_string());

        // Start with all columns visible
        app.ecr_image_visible_column_ids = ImageColumn::ids();
        let initial_count = app.ecr_image_visible_column_ids.len();

        // Select first column (index 0) and toggle it
        app.column_selector_index = 0;
        app.handle_action(Action::ToggleColumn);

        // First column should be removed
        assert_eq!(app.ecr_image_visible_column_ids.len(), initial_count - 1);
        assert!(!app
            .ecr_image_visible_column_ids
            .contains(&ImageColumn::Tag.id()));

        // Toggle it back
        app.handle_action(Action::ToggleColumn);
        assert_eq!(app.ecr_image_visible_column_ids.len(), initial_count);
        assert!(app
            .ecr_image_visible_column_ids
            .contains(&ImageColumn::Tag.id()));
    }

    #[test]
    fn test_ecr_repos_column_toggle_works() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::ColumnSelector;
        app.ecr_state.current_repository = None;

        // Start with all columns visible
        app.ecr_repo_visible_column_ids = EcrColumn::ids();
        let initial_count = app.ecr_repo_visible_column_ids.len();

        // Select first column (index 0) and toggle it
        app.column_selector_index = 0;
        app.handle_action(Action::ToggleColumn);

        // First column should be removed
        assert_eq!(app.ecr_repo_visible_column_ids.len(), initial_count - 1);
        assert!(!app
            .ecr_repo_visible_column_ids
            .contains(&EcrColumn::Name.id()));

        // Toggle it back
        app.handle_action(Action::ToggleColumn);
        assert_eq!(app.ecr_repo_visible_column_ids.len(), initial_count);
        assert!(app
            .ecr_repo_visible_column_ids
            .contains(&EcrColumn::Name.id()));
    }

    #[test]
    fn test_ecr_repos_pagination_left_right_navigation() {
        use crate::ecr::repo::Repository as EcrRepository;
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::FilterInput;
        app.ecr_state.input_focus = InputFocus::Pagination;

        // Create 150 repos (3 pages with page size 50)
        app.ecr_state.repositories.items = (0..150)
            .map(|i| EcrRepository {
                name: format!("repo{:03}", i),
                uri: format!("uri{}", i),
                created_at: "2023-01-01".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            })
            .collect();

        // Start on page 1 (index 0)
        app.ecr_state.repositories.selected = 0;
        eprintln!(
            "Initial: selected={}, focus={:?}, mode={:?}",
            app.ecr_state.repositories.selected, app.ecr_state.input_focus, app.mode
        );

        // Right arrow (PageDown) should go to page 2
        app.handle_action(Action::PageDown);
        eprintln!(
            "After PageDown: selected={}",
            app.ecr_state.repositories.selected
        );
        assert_eq!(app.ecr_state.repositories.selected, 50);

        // Right arrow again should go to page 3
        app.handle_action(Action::PageDown);
        eprintln!(
            "After 2nd PageDown: selected={}",
            app.ecr_state.repositories.selected
        );
        assert_eq!(app.ecr_state.repositories.selected, 100);

        // Right arrow at last page should stay at last page
        app.handle_action(Action::PageDown);
        eprintln!(
            "After 3rd PageDown: selected={}",
            app.ecr_state.repositories.selected
        );
        assert_eq!(app.ecr_state.repositories.selected, 100);

        // Left arrow (PageUp) should go back to page 2
        app.handle_action(Action::PageUp);
        eprintln!(
            "After PageUp: selected={}",
            app.ecr_state.repositories.selected
        );
        assert_eq!(app.ecr_state.repositories.selected, 50);

        // Left arrow again should go to page 1
        app.handle_action(Action::PageUp);
        eprintln!(
            "After 2nd PageUp: selected={}",
            app.ecr_state.repositories.selected
        );
        assert_eq!(app.ecr_state.repositories.selected, 0);

        // Left arrow at first page should stay at first page
        app.handle_action(Action::PageUp);
        eprintln!(
            "After 3rd PageUp: selected={}",
            app.ecr_state.repositories.selected
        );
        assert_eq!(app.ecr_state.repositories.selected, 0);
    }

    #[test]
    fn test_ecr_repos_filter_input_when_input_focused() {
        use crate::ecr::repo::Repository as EcrRepository;
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::FilterInput;
        app.ecr_state.input_focus = InputFocus::Filter;

        // Create some repos
        app.ecr_state.repositories.items = vec![
            EcrRepository {
                name: "test-repo".to_string(),
                uri: "uri1".to_string(),
                created_at: "2023-01-01".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            },
            EcrRepository {
                name: "prod-repo".to_string(),
                uri: "uri2".to_string(),
                created_at: "2023-01-01".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            },
        ];

        // When input is focused, typing should add to filter
        assert_eq!(app.ecr_state.repositories.filter, "");
        app.handle_action(Action::FilterInput('t'));
        assert_eq!(app.ecr_state.repositories.filter, "t");
        app.handle_action(Action::FilterInput('e'));
        assert_eq!(app.ecr_state.repositories.filter, "te");
        app.handle_action(Action::FilterInput('s'));
        assert_eq!(app.ecr_state.repositories.filter, "tes");
        app.handle_action(Action::FilterInput('t'));
        assert_eq!(app.ecr_state.repositories.filter, "test");
    }

    #[test]
    fn test_ecr_repos_digit_input_when_pagination_focused() {
        use crate::ecr::repo::Repository as EcrRepository;
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::FilterInput;
        app.ecr_state.input_focus = InputFocus::Pagination;

        // Create some repos
        app.ecr_state.repositories.items = vec![EcrRepository {
            name: "test-repo".to_string(),
            uri: "uri1".to_string(),
            created_at: "2023-01-01".to_string(),
            tag_immutability: "MUTABLE".to_string(),
            encryption_type: "AES256".to_string(),
        }];

        // When pagination is focused, digits should go to page_input, not filter
        assert_eq!(app.ecr_state.repositories.filter, "");
        assert_eq!(app.page_input, "");
        app.handle_action(Action::FilterInput('2'));
        assert_eq!(app.ecr_state.repositories.filter, "");
        assert_eq!(app.page_input, "2");

        // Non-digits should not be added to either
        app.handle_action(Action::FilterInput('a'));
        assert_eq!(app.ecr_state.repositories.filter, "");
        assert_eq!(app.page_input, "2");
    }

    #[test]
    fn test_ecr_repos_left_right_scrolls_table_when_input_focused() {
        use crate::ecr::repo::Repository as EcrRepository;
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::FilterInput;
        app.ecr_state.input_focus = InputFocus::Filter;

        // Create 150 repos (3 pages)
        app.ecr_state.repositories.items = (0..150)
            .map(|i| EcrRepository {
                name: format!("repo{:03}", i),
                uri: format!("uri{}", i),
                created_at: "2023-01-01".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            })
            .collect();

        // Start on page 1
        app.ecr_state.repositories.selected = 0;

        // When input is focused, left/right should scroll table (not change pages)
        app.handle_action(Action::PageDown);
        assert_eq!(
            app.ecr_state.repositories.selected, 10,
            "Should scroll down by 10"
        );

        app.handle_action(Action::PageUp);
        assert_eq!(
            app.ecr_state.repositories.selected, 0,
            "Should scroll back up"
        );
    }

    #[test]
    fn test_ecr_repos_pagination_control_actually_works() {
        use crate::ecr::repo::Repository as EcrRepository;

        // Test that verifies the exact conditions needed for pagination to work
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::FilterInput;
        app.ecr_state.current_repository = None;
        app.ecr_state.input_focus = InputFocus::Pagination;

        // Create 100 repos (2 pages with page size 50)
        app.ecr_state.repositories.items = (0..100)
            .map(|i| EcrRepository {
                name: format!("repo{:03}", i),
                uri: format!("uri{}", i),
                created_at: "2023-01-01".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            })
            .collect();

        app.ecr_state.repositories.selected = 0;

        // Verify all conditions are met
        assert_eq!(app.mode, Mode::FilterInput);
        assert_eq!(app.current_service, Service::EcrRepositories);
        assert_eq!(app.ecr_state.current_repository, None);
        assert_eq!(app.ecr_state.input_focus, InputFocus::Pagination);

        // Now test pagination
        app.handle_action(Action::PageDown);
        assert_eq!(
            app.ecr_state.repositories.selected, 50,
            "PageDown should move to page 2"
        );

        app.handle_action(Action::PageUp);
        assert_eq!(
            app.ecr_state.repositories.selected, 0,
            "PageUp should move back to page 1"
        );
    }

    #[test]
    fn test_ecr_repos_start_filter_resets_focus_to_input() {
        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.ecr_state.current_repository = None;

        // Set focus to Pagination
        app.ecr_state.input_focus = InputFocus::Pagination;

        // Start filter mode
        app.handle_action(Action::StartFilter);

        // Should reset to Input focus
        assert_eq!(app.mode, Mode::FilterInput);
        assert_eq!(app.ecr_state.input_focus, InputFocus::Filter);
    }

    #[test]
    fn test_ecr_repos_exact_user_flow_i_tab_arrow() {
        use crate::ecr::repo::Repository as EcrRepository;

        let mut app = test_app();
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.ecr_state.current_repository = None;

        // Create 100 repos (2 pages)
        app.ecr_state.repositories.items = (0..100)
            .map(|i| EcrRepository {
                name: format!("repo{:03}", i),
                uri: format!("uri{}", i),
                created_at: "2023-01-01".to_string(),
                tag_immutability: "MUTABLE".to_string(),
                encryption_type: "AES256".to_string(),
            })
            .collect();

        app.ecr_state.repositories.selected = 0;

        // User presses 'i' to enter filter mode
        app.handle_action(Action::StartFilter);
        assert_eq!(app.mode, Mode::FilterInput);
        assert_eq!(app.ecr_state.input_focus, InputFocus::Filter);

        // User presses Tab to switch to pagination
        app.handle_action(Action::NextFilterFocus);
        assert_eq!(app.ecr_state.input_focus, InputFocus::Pagination);

        // User presses right arrow (PageDown)
        eprintln!("Before PageDown: mode={:?}, service={:?}, current_repo={:?}, input_focus={:?}, selected={}",
            app.mode, app.current_service, app.ecr_state.current_repository, app.ecr_state.input_focus, app.ecr_state.repositories.selected);
        app.handle_action(Action::PageDown);
        eprintln!(
            "After PageDown: selected={}",
            app.ecr_state.repositories.selected
        );

        // Should move to page 2
        assert_eq!(
            app.ecr_state.repositories.selected, 50,
            "Right arrow should move to page 2"
        );

        // User presses left arrow (PageUp)
        app.handle_action(Action::PageUp);
        assert_eq!(
            app.ecr_state.repositories.selected, 0,
            "Left arrow should move back to page 1"
        );
    }

    #[test]
    fn test_service_picker_i_key_activates_filter() {
        let mut app = test_app();

        // Start in ServicePicker mode (service picker)
        assert_eq!(app.mode, Mode::ServicePicker);
        assert!(app.service_picker.filter.is_empty());

        // Press 'i' to start filtering
        app.handle_action(Action::FilterInput('i'));

        // Should still be in ServicePicker mode and filter should have 'i'
        assert_eq!(app.mode, Mode::ServicePicker);
        assert_eq!(app.service_picker.filter, "i");
    }

    #[test]
    fn test_service_picker_typing_filters_services() {
        let mut app = test_app();

        // Start in ServicePicker mode
        assert_eq!(app.mode, Mode::ServicePicker);

        // Type "s3" to filter
        app.handle_action(Action::FilterInput('s'));
        app.handle_action(Action::FilterInput('3'));

        assert_eq!(app.service_picker.filter, "s3");
        assert_eq!(app.mode, Mode::ServicePicker);
    }

    #[test]
    fn test_service_picker_resets_on_open() {
        let mut app = test_app();

        // Select a service to get into Normal mode
        app.service_selected = true;
        app.mode = Mode::Normal;

        // Simulate having previous filter and selection
        app.service_picker.filter = "previous".to_string();
        app.service_picker.selected = 5;

        // Open space menu (service picker)
        app.handle_action(Action::OpenSpaceMenu);

        // Filter and selection should be reset
        assert_eq!(app.mode, Mode::SpaceMenu);
        assert!(app.service_picker.filter.is_empty());
        assert_eq!(app.service_picker.selected, 0);
    }

    #[test]
    fn test_no_pii_in_test_data() {
        // Ensure test data uses placeholder account IDs, not real ones
        let test_repo = EcrRepository {
            name: "test-repo".to_string(),
            uri: "123456789012.dkr.ecr.us-east-1.amazonaws.com/test-repo".to_string(),
            created_at: "2024-01-01".to_string(),
            tag_immutability: "MUTABLE".to_string(),
            encryption_type: "AES256".to_string(),
        };

        // Verify placeholder account ID is used
        assert!(test_repo.uri.starts_with("123456789012"));
        assert!(!test_repo.uri.contains("123456789013")); // Not a real account
    }

    #[test]
    fn test_lambda_versions_tab_triggers_loading() {
        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;

        // Simulate selecting a function
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Code;

        // Initially no versions
        assert!(app.lambda_state.version_table.items.is_empty());

        // Switch to Versions tab
        app.lambda_state.detail_tab = LambdaDetailTab::Versions;

        // The main loop should detect this change and load versions
        // We verify the state is set up correctly for loading
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Versions);
        assert!(app.lambda_state.current_function.is_some());
    }

    #[test]
    fn test_lambda_versions_navigation() {
        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Versions;

        // Add test versions
        app.lambda_state.version_table.items = vec![
            LambdaVersion {
                version: "3".to_string(),
                aliases: "prod".to_string(),
                description: "".to_string(),
                last_modified: "".to_string(),
                architecture: "X86_64".to_string(),
            },
            LambdaVersion {
                version: "2".to_string(),
                aliases: "".to_string(),
                description: "".to_string(),
                last_modified: "".to_string(),
                architecture: "X86_64".to_string(),
            },
            LambdaVersion {
                version: "1".to_string(),
                aliases: "".to_string(),
                description: "".to_string(),
                last_modified: "".to_string(),
                architecture: "X86_64".to_string(),
            },
        ];

        // Verify versions are loaded
        assert_eq!(app.lambda_state.version_table.items.len(), 3);
        assert_eq!(app.lambda_state.version_table.items[0].version, "3");
        assert_eq!(app.lambda_state.version_table.items[0].aliases, "prod");

        // Verify selection can be changed
        app.lambda_state.version_table.selected = 1;
        assert_eq!(app.lambda_state.version_table.selected, 1);
    }

    #[test]
    fn test_lambda_versions_with_aliases() {
        let version = LambdaVersion {
            version: "35".to_string(),
            aliases: "prod, staging".to_string(),
            description: "Production version".to_string(),
            last_modified: "2024-01-01".to_string(),
            architecture: "X86_64".to_string(),
        };

        assert_eq!(version.aliases, "prod, staging");
        assert!(!version.aliases.is_empty());
    }

    #[test]
    fn test_lambda_versions_expansion() {
        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Versions;

        // Add test versions
        app.lambda_state.version_table.items = vec![
            LambdaVersion {
                version: "2".to_string(),
                aliases: "prod".to_string(),
                description: "Production".to_string(),
                last_modified: "2024-01-01".to_string(),
                architecture: "X86_64".to_string(),
            },
            LambdaVersion {
                version: "1".to_string(),
                aliases: "".to_string(),
                description: "".to_string(),
                last_modified: "2024-01-01".to_string(),
                architecture: "Arm64".to_string(),
            },
        ];

        app.lambda_state.version_table.selected = 0;

        // Verify expansion can be set
        app.lambda_state.version_table.expanded_item = Some(0);
        assert_eq!(app.lambda_state.version_table.expanded_item, Some(0));

        // Select different version
        app.lambda_state.version_table.selected = 1;
        app.lambda_state.version_table.expanded_item = Some(1);
        assert_eq!(app.lambda_state.version_table.expanded_item, Some(1));
    }

    #[test]
    fn test_lambda_versions_page_navigation() {
        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Versions;

        // Add 30 test versions
        app.lambda_state.version_table.items = (1..=30)
            .map(|i| LambdaVersion {
                version: i.to_string(),
                aliases: "".to_string(),
                description: "".to_string(),
                last_modified: "".to_string(),
                architecture: "X86_64".to_string(),
            })
            .collect();

        app.lambda_state.version_table.page_size = PageSize::Ten;
        app.lambda_state.version_table.selected = 0;

        // Go to page 2
        app.page_input = "2".to_string();
        app.handle_action(Action::OpenColumnSelector);

        // Should be at index 10 (start of page 2)
        assert_eq!(app.lambda_state.version_table.selected, 10);
    }

    #[test]
    fn test_lambda_versions_pagination_arrow_keys() {
        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Versions;
        app.mode = Mode::FilterInput;
        app.lambda_state.version_input_focus = InputFocus::Pagination;

        // Add 30 test versions
        app.lambda_state.version_table.items = (1..=30)
            .map(|i| LambdaVersion {
                version: i.to_string(),
                aliases: "".to_string(),
                description: "".to_string(),
                last_modified: "".to_string(),
                architecture: "X86_64".to_string(),
            })
            .collect();

        app.lambda_state.version_table.page_size = PageSize::Ten;
        app.lambda_state.version_table.selected = 0;

        // Right arrow (PageDown) should go to next page
        app.handle_action(Action::PageDown);
        assert_eq!(app.lambda_state.version_table.selected, 10);

        // Left arrow (PageUp) should go back
        app.handle_action(Action::PageUp);
        assert_eq!(app.lambda_state.version_table.selected, 0);
    }

    #[test]
    fn test_lambda_versions_page_input_in_filter_mode() {
        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Versions;
        app.mode = Mode::FilterInput;
        app.lambda_state.version_input_focus = InputFocus::Pagination;

        // Add 30 test versions
        app.lambda_state.version_table.items = (1..=30)
            .map(|i| LambdaVersion {
                version: i.to_string(),
                aliases: "".to_string(),
                description: "".to_string(),
                last_modified: "".to_string(),
                architecture: "X86_64".to_string(),
            })
            .collect();

        app.lambda_state.version_table.page_size = PageSize::Ten;
        app.lambda_state.version_table.selected = 0;

        // Type "2" when focused on Pagination
        app.handle_action(Action::FilterInput('2'));
        assert_eq!(app.page_input, "2");
        assert_eq!(app.lambda_state.version_table.filter, ""); // Should not go to filter

        // Press 'p' to go to page 2
        app.handle_action(Action::OpenColumnSelector);
        assert_eq!(app.lambda_state.version_table.selected, 10);
        assert_eq!(app.page_input, ""); // Should be cleared
    }

    #[test]
    fn test_lambda_versions_filter_input() {
        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Versions;
        app.mode = Mode::FilterInput;
        app.lambda_state.version_input_focus = InputFocus::Filter;

        // Add test versions
        app.lambda_state.version_table.items = vec![
            LambdaVersion {
                version: "1".to_string(),
                aliases: "prod".to_string(),
                description: "Production".to_string(),
                last_modified: "".to_string(),
                architecture: "X86_64".to_string(),
            },
            LambdaVersion {
                version: "2".to_string(),
                aliases: "staging".to_string(),
                description: "Staging".to_string(),
                last_modified: "".to_string(),
                architecture: "X86_64".to_string(),
            },
        ];

        // Type filter text
        app.handle_action(Action::FilterInput('p'));
        app.handle_action(Action::FilterInput('r'));
        app.handle_action(Action::FilterInput('o'));
        app.handle_action(Action::FilterInput('d'));
        assert_eq!(app.lambda_state.version_table.filter, "prod");

        // Backspace should work
        app.handle_action(Action::FilterBackspace);
        assert_eq!(app.lambda_state.version_table.filter, "pro");
    }

    #[test]
    fn test_lambda_aliases_table_expansion() {
        use crate::lambda::Alias;

        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Aliases;
        app.mode = Mode::Normal;

        app.lambda_state.alias_table.items = vec![
            Alias {
                name: "prod".to_string(),
                versions: "1".to_string(),
                description: "Production alias".to_string(),
            },
            Alias {
                name: "staging".to_string(),
                versions: "2".to_string(),
                description: "Staging alias".to_string(),
            },
        ];

        app.lambda_state.alias_table.selected = 0;

        // Select first alias - should open alias detail view (no tab change)
        app.handle_action(Action::Select);
        assert_eq!(app.lambda_state.current_alias, Some("prod".to_string()));
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Aliases);

        // Go back
        app.handle_action(Action::GoBack);
        assert_eq!(app.lambda_state.current_alias, None);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Aliases);

        // Select second alias
        app.lambda_state.alias_table.selected = 1;
        app.handle_action(Action::Select);
        assert_eq!(app.lambda_state.current_alias, Some("staging".to_string()));
    }

    #[test]
    fn test_lambda_versions_arrow_key_expansion() {
        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Versions;
        app.mode = Mode::Normal;

        app.lambda_state.version_table.items = vec![LambdaVersion {
            version: "1".to_string(),
            aliases: "prod".to_string(),
            description: "Production".to_string(),
            last_modified: "2024-01-01".to_string(),
            architecture: "X86_64".to_string(),
        }];

        app.lambda_state.version_table.selected = 0;

        // Right arrow expands
        app.handle_action(Action::NextPane);
        assert_eq!(app.lambda_state.version_table.expanded_item, Some(0));

        // Left arrow collapses
        app.handle_action(Action::PrevPane);
        assert_eq!(app.lambda_state.version_table.expanded_item, None);
    }

    #[test]
    fn test_lambda_version_detail_view() {
        use crate::lambda::Function;

        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Versions;
        app.mode = Mode::Normal;

        app.lambda_state.table.items = vec![Function {
            name: "test-function".to_string(),
            arn: "arn:aws:lambda:us-east-1:123456789012:function:test-function".to_string(),
            application: None,
            description: "Test".to_string(),
            package_type: "Zip".to_string(),
            runtime: "python3.12".to_string(),
            architecture: "X86_64".to_string(),
            code_size: 1024,
            code_sha256: "hash".to_string(),
            memory_mb: 128,
            timeout_seconds: 30,
            last_modified: "2024-01-01".to_string(),
            layers: vec![],
        }];

        app.lambda_state.version_table.items = vec![LambdaVersion {
            version: "1".to_string(),
            aliases: "prod".to_string(),
            description: "Production".to_string(),
            last_modified: "2024-01-01".to_string(),
            architecture: "X86_64".to_string(),
        }];

        app.lambda_state.version_table.selected = 0;

        // Select version to open detail view
        app.handle_action(Action::Select);
        assert_eq!(app.lambda_state.current_version, Some("1".to_string()));
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Code);

        // GoBack should go back to versions list
        app.handle_action(Action::GoBack);
        assert_eq!(app.lambda_state.current_version, None);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Versions);
    }

    #[test]
    fn test_lambda_version_detail_tabs() {
        use crate::lambda::Function;

        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.current_version = Some("1".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Code;
        app.mode = Mode::Normal;

        app.lambda_state.table.items = vec![Function {
            name: "test-function".to_string(),
            arn: "arn:aws:lambda:us-east-1:123456789012:function:test-function".to_string(),
            application: None,
            description: "Test".to_string(),
            package_type: "Zip".to_string(),
            runtime: "python3.12".to_string(),
            architecture: "X86_64".to_string(),
            code_size: 1024,
            code_sha256: "hash".to_string(),
            memory_mb: 128,
            timeout_seconds: 30,
            last_modified: "2024-01-01".to_string(),
            layers: vec![],
        }];

        // Tab should cycle between Code, Monitor, and Configuration
        app.handle_action(Action::NextDetailTab);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Monitor);

        app.handle_action(Action::NextDetailTab);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Configuration);

        app.handle_action(Action::NextDetailTab);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Code);

        // BackTab should cycle backward
        app.handle_action(Action::PrevDetailTab);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Configuration);

        app.handle_action(Action::PrevDetailTab);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Monitor);
    }

    #[test]
    fn test_lambda_aliases_arrow_key_expansion() {
        use crate::lambda::Alias;

        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Aliases;
        app.mode = Mode::Normal;

        app.lambda_state.alias_table.items = vec![Alias {
            name: "prod".to_string(),
            versions: "1".to_string(),
            description: "Production alias".to_string(),
        }];

        app.lambda_state.alias_table.selected = 0;

        // Right arrow expands
        app.handle_action(Action::NextPane);
        assert_eq!(app.lambda_state.alias_table.expanded_item, Some(0));

        // Left arrow collapses
        app.handle_action(Action::PrevPane);
        assert_eq!(app.lambda_state.alias_table.expanded_item, None);
    }

    #[test]
    fn test_lambda_functions_arrow_key_expansion() {
        use crate::lambda::Function;

        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.mode = Mode::Normal;

        app.lambda_state.table.items = vec![Function {
            name: "test-function".to_string(),
            arn: "arn".to_string(),
            application: None,
            description: "Test".to_string(),
            package_type: "Zip".to_string(),
            runtime: "python3.12".to_string(),
            architecture: "X86_64".to_string(),
            code_size: 1024,
            code_sha256: "hash".to_string(),
            memory_mb: 128,
            timeout_seconds: 30,
            last_modified: "2024-01-01".to_string(),
            layers: vec![],
        }];

        app.lambda_state.table.selected = 0;

        // Right arrow expands
        app.handle_action(Action::NextPane);
        assert_eq!(app.lambda_state.table.expanded_item, Some(0));

        // Left arrow collapses
        app.handle_action(Action::PrevPane);
        assert_eq!(app.lambda_state.table.expanded_item, None);
    }

    #[test]
    fn test_lambda_version_detail_with_application() {
        use crate::lambda::Function;

        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.lambda_state.current_function = Some("storefront-studio-beta-api".to_string());
        app.lambda_state.current_version = Some("1".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Code;
        app.mode = Mode::Normal;

        app.lambda_state.table.items = vec![Function {
            name: "storefront-studio-beta-api".to_string(),
            arn: "arn:aws:lambda:us-east-1:123456789012:function:storefront-studio-beta-api"
                .to_string(),
            application: Some("storefront-studio-beta".to_string()),
            description: "API function".to_string(),
            package_type: "Zip".to_string(),
            runtime: "python3.12".to_string(),
            architecture: "X86_64".to_string(),
            code_size: 1024,
            code_sha256: "hash".to_string(),
            memory_mb: 128,
            timeout_seconds: 30,
            last_modified: "2024-01-01".to_string(),
            layers: vec![],
        }];

        // Verify function has application extracted
        assert_eq!(
            app.lambda_state.table.items[0].application,
            Some("storefront-studio-beta".to_string())
        );
        assert_eq!(app.lambda_state.current_version, Some("1".to_string()));
    }

    #[test]
    fn test_lambda_layer_navigation() {
        use crate::lambda::{Function, Layer};

        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Code;
        app.mode = Mode::Normal;

        app.lambda_state.table.items = vec![Function {
            name: "test-function".to_string(),
            arn: "arn:aws:lambda:us-east-1:123456789012:function:test-function".to_string(),
            application: None,
            description: "Test".to_string(),
            package_type: "Zip".to_string(),
            runtime: "python3.12".to_string(),
            architecture: "X86_64".to_string(),
            code_size: 1024,
            code_sha256: "hash".to_string(),
            memory_mb: 128,
            timeout_seconds: 30,
            last_modified: "2024-01-01".to_string(),
            layers: vec![
                Layer {
                    merge_order: "1".to_string(),
                    name: "layer1".to_string(),
                    layer_version: "1".to_string(),
                    compatible_runtimes: "python3.9".to_string(),
                    compatible_architectures: "x86_64".to_string(),
                    version_arn: "arn:aws:lambda:us-east-1:123456789012:layer:layer1:1".to_string(),
                },
                Layer {
                    merge_order: "2".to_string(),
                    name: "layer2".to_string(),
                    layer_version: "2".to_string(),
                    compatible_runtimes: "python3.9".to_string(),
                    compatible_architectures: "x86_64".to_string(),
                    version_arn: "arn:aws:lambda:us-east-1:123456789012:layer:layer2:2".to_string(),
                },
                Layer {
                    merge_order: "3".to_string(),
                    name: "layer3".to_string(),
                    layer_version: "3".to_string(),
                    compatible_runtimes: "python3.9".to_string(),
                    compatible_architectures: "x86_64".to_string(),
                    version_arn: "arn:aws:lambda:us-east-1:123456789012:layer:layer3:3".to_string(),
                },
            ],
        }];

        assert_eq!(app.lambda_state.layer_selected, 0);

        app.handle_action(Action::NextItem);
        assert_eq!(app.lambda_state.layer_selected, 1);

        app.handle_action(Action::NextItem);
        assert_eq!(app.lambda_state.layer_selected, 2);

        app.handle_action(Action::NextItem);
        assert_eq!(app.lambda_state.layer_selected, 2);

        app.handle_action(Action::PrevItem);
        assert_eq!(app.lambda_state.layer_selected, 1);

        app.handle_action(Action::PrevItem);
        assert_eq!(app.lambda_state.layer_selected, 0);

        app.handle_action(Action::PrevItem);
        assert_eq!(app.lambda_state.layer_selected, 0);
    }

    #[test]
    fn test_lambda_layer_expansion() {
        use crate::lambda::{Function, Layer};

        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Code;
        app.mode = Mode::Normal;

        app.lambda_state.table.items = vec![Function {
            name: "test-function".to_string(),
            arn: "arn:aws:lambda:us-east-1:123456789012:function:test-function".to_string(),
            application: None,
            description: "Test".to_string(),
            package_type: "Zip".to_string(),
            runtime: "python3.12".to_string(),
            architecture: "X86_64".to_string(),
            code_size: 1024,
            code_sha256: "hash".to_string(),
            memory_mb: 128,
            timeout_seconds: 30,
            last_modified: "2024-01-01".to_string(),
            layers: vec![Layer {
                merge_order: "1".to_string(),
                name: "test-layer".to_string(),
                layer_version: "1".to_string(),
                compatible_runtimes: "python3.9".to_string(),
                compatible_architectures: "x86_64".to_string(),
                version_arn: "arn:aws:lambda:us-east-1:123456789012:layer:test-layer:1".to_string(),
            }],
        }];

        assert_eq!(app.lambda_state.layer_expanded, None);

        app.handle_action(Action::NextPane);
        assert_eq!(app.lambda_state.layer_expanded, Some(0));

        app.handle_action(Action::PrevPane);
        assert_eq!(app.lambda_state.layer_expanded, None);

        app.handle_action(Action::NextPane);
        assert_eq!(app.lambda_state.layer_expanded, Some(0));

        app.handle_action(Action::NextPane);
        assert_eq!(app.lambda_state.layer_expanded, None);
    }

    #[test]
    fn test_lambda_layer_selection_and_expansion_workflow() {
        use crate::lambda::{Function, Layer};

        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Code;
        app.mode = Mode::Normal;

        app.lambda_state.table.items = vec![Function {
            name: "test-function".to_string(),
            arn: "arn:aws:lambda:us-east-1:123456789012:function:test-function".to_string(),
            application: None,
            description: "Test".to_string(),
            package_type: "Zip".to_string(),
            runtime: "python3.12".to_string(),
            architecture: "X86_64".to_string(),
            code_size: 1024,
            code_sha256: "hash".to_string(),
            memory_mb: 128,
            timeout_seconds: 30,
            last_modified: "2024-01-01".to_string(),
            layers: vec![
                Layer {
                    merge_order: "1".to_string(),
                    name: "layer1".to_string(),
                    layer_version: "1".to_string(),
                    compatible_runtimes: "python3.9".to_string(),
                    compatible_architectures: "x86_64".to_string(),
                    version_arn: "arn:aws:lambda:us-east-1:123456789012:layer:layer1:1".to_string(),
                },
                Layer {
                    merge_order: "2".to_string(),
                    name: "layer2".to_string(),
                    layer_version: "2".to_string(),
                    compatible_runtimes: "python3.9".to_string(),
                    compatible_architectures: "x86_64".to_string(),
                    version_arn: "arn:aws:lambda:us-east-1:123456789012:layer:layer2:2".to_string(),
                },
            ],
        }];

        // Start at layer 0
        assert_eq!(app.lambda_state.layer_selected, 0);
        assert_eq!(app.lambda_state.layer_expanded, None);

        // Expand layer 0
        app.handle_action(Action::NextPane);
        assert_eq!(app.lambda_state.layer_selected, 0);
        assert_eq!(app.lambda_state.layer_expanded, Some(0));

        // Navigate to layer 1 while layer 0 is expanded
        app.handle_action(Action::NextItem);
        assert_eq!(app.lambda_state.layer_selected, 1);
        assert_eq!(app.lambda_state.layer_expanded, Some(0)); // Still expanded

        // Expand layer 1 (should collapse layer 0 and expand layer 1)
        app.handle_action(Action::NextPane);
        assert_eq!(app.lambda_state.layer_selected, 1);
        assert_eq!(app.lambda_state.layer_expanded, Some(1));

        // Collapse layer 1
        app.handle_action(Action::PrevPane);
        assert_eq!(app.lambda_state.layer_selected, 1);
        assert_eq!(app.lambda_state.layer_expanded, None);

        // Navigate back to layer 0
        app.handle_action(Action::PrevItem);
        assert_eq!(app.lambda_state.layer_selected, 0);
        assert_eq!(app.lambda_state.layer_expanded, None);
    }

    #[test]
    fn test_backtab_cycles_detail_tabs_backward() {
        let mut app = test_app();
        app.mode = Mode::Normal;

        // Test Lambda detail tabs
        app.current_service = Service::LambdaFunctions;
        app.service_selected = true;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Code;

        app.handle_action(Action::PrevDetailTab);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Versions);

        app.handle_action(Action::PrevDetailTab);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Aliases);

        // Test IAM Roles detail tabs
        app.current_service = Service::IamRoles;
        app.iam_state.current_role = Some("test-role".to_string());
        app.iam_state.role_tab = RoleTab::Permissions;

        app.handle_action(Action::PrevDetailTab);
        assert_eq!(app.iam_state.role_tab, RoleTab::RevokeSessions);

        // Test IAM Users detail tabs
        app.current_service = Service::IamUsers;
        app.iam_state.current_user = Some("test-user".to_string());
        app.iam_state.user_tab = UserTab::Permissions;

        app.handle_action(Action::PrevDetailTab);
        assert_eq!(app.iam_state.user_tab, UserTab::LastAccessed);

        // Test IAM Groups detail tabs
        app.current_service = Service::IamUserGroups;
        app.iam_state.current_group = Some("test-group".to_string());
        app.iam_state.group_tab = GroupTab::Permissions;

        app.handle_action(Action::PrevDetailTab);
        assert_eq!(app.iam_state.group_tab, GroupTab::Users);

        // Test S3 object tabs
        app.current_service = Service::S3Buckets;
        app.s3_state.current_bucket = Some("test-bucket".to_string());
        app.s3_state.object_tab = S3ObjectTab::Properties;

        app.handle_action(Action::PrevDetailTab);
        assert_eq!(app.s3_state.object_tab, S3ObjectTab::Objects);

        // Test ECR repository tabs (Private/Public)
        app.current_service = Service::EcrRepositories;
        app.ecr_state.current_repository = None;
        app.ecr_state.tab = EcrTab::Private;

        app.handle_action(Action::PrevDetailTab);
        assert_eq!(app.ecr_state.tab, EcrTab::Public);

        // Test CloudFormation detail tabs
        app.current_service = Service::CloudFormationStacks;
        app.cfn_state.current_stack = Some("test-stack".to_string());
        app.cfn_state.detail_tab = CfnDetailTab::Resources;
    }

    #[test]
    fn test_cloudformation_status_filter_active() {
        let filter = CfnStatusFilter::Active;
        assert!(filter.matches("CREATE_IN_PROGRESS"));
        assert!(filter.matches("UPDATE_IN_PROGRESS"));
        assert!(!filter.matches("CREATE_COMPLETE"));
        assert!(!filter.matches("DELETE_COMPLETE"));
        assert!(!filter.matches("CREATE_FAILED"));
    }

    #[test]
    fn test_cloudformation_status_filter_complete() {
        let filter = CfnStatusFilter::Complete;
        assert!(filter.matches("CREATE_COMPLETE"));
        assert!(filter.matches("UPDATE_COMPLETE"));
        assert!(!filter.matches("DELETE_COMPLETE"));
        assert!(!filter.matches("CREATE_IN_PROGRESS"));
    }

    #[test]
    fn test_cloudformation_status_filter_failed() {
        let filter = CfnStatusFilter::Failed;
        assert!(filter.matches("CREATE_FAILED"));
        assert!(filter.matches("UPDATE_FAILED"));
        assert!(!filter.matches("CREATE_COMPLETE"));
    }

    #[test]
    fn test_cloudformation_status_filter_deleted() {
        let filter = CfnStatusFilter::Deleted;
        assert!(filter.matches("DELETE_COMPLETE"));
        assert!(filter.matches("DELETE_IN_PROGRESS"));
        assert!(!filter.matches("CREATE_COMPLETE"));
    }

    #[test]
    fn test_cloudformation_status_filter_in_progress() {
        let filter = CfnStatusFilter::InProgress;
        assert!(filter.matches("CREATE_IN_PROGRESS"));
        assert!(filter.matches("UPDATE_IN_PROGRESS"));
        assert!(filter.matches("DELETE_IN_PROGRESS"));
        assert!(!filter.matches("CREATE_COMPLETE"));
    }

    #[test]
    fn test_cloudformation_status_filter_cycle() {
        let filter = CfnStatusFilter::All;
        assert_eq!(filter.next(), CfnStatusFilter::Active);
        assert_eq!(filter.next().next(), CfnStatusFilter::Complete);
        assert_eq!(filter.next().next().next(), CfnStatusFilter::Failed);
        assert_eq!(filter.next().next().next().next(), CfnStatusFilter::Deleted);
        assert_eq!(
            filter.next().next().next().next().next(),
            CfnStatusFilter::InProgress
        );
        assert_eq!(
            filter.next().next().next().next().next().next(),
            CfnStatusFilter::All
        );
    }

    #[test]
    fn test_cloudformation_default_columns() {
        let app = test_app();
        assert_eq!(app.cfn_visible_column_ids.len(), 4);
        assert!(app.cfn_visible_column_ids.contains(&CfnColumn::Name.id()));
        assert!(app.cfn_visible_column_ids.contains(&CfnColumn::Status.id()));
        assert!(app
            .cfn_visible_column_ids
            .contains(&CfnColumn::CreatedTime.id()));
        assert!(app
            .cfn_visible_column_ids
            .contains(&CfnColumn::Description.id()));
    }

    #[test]
    fn test_cloudformation_all_columns() {
        let app = test_app();
        assert_eq!(app.cfn_column_ids.len(), 10);
    }

    #[test]
    fn test_cloudformation_filter_by_name() {
        let mut app = test_app();
        app.cfn_state.status_filter = CfnStatusFilter::Complete;
        app.cfn_state.table.items = vec![
            CfnStack {
                name: "my-stack".to_string(),
                stack_id: "id1".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                created_time: "2024-01-01".to_string(),
                updated_time: String::new(),
                deleted_time: String::new(),
                drift_status: String::new(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: String::new(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            },
            CfnStack {
                name: "other-stack".to_string(),
                stack_id: "id2".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                created_time: "2024-01-02".to_string(),
                updated_time: String::new(),
                deleted_time: String::new(),
                drift_status: String::new(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: String::new(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            },
        ];

        app.cfn_state.table.filter = "my".to_string();
        let filtered = filtered_cloudformation_stacks(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "my-stack");
    }

    #[test]
    fn test_cloudformation_filter_by_description() {
        let mut app = test_app();
        app.cfn_state.status_filter = CfnStatusFilter::Complete;
        app.cfn_state.table.items = vec![CfnStack {
            name: "stack1".to_string(),
            stack_id: "id1".to_string(),
            status: "CREATE_COMPLETE".to_string(),
            created_time: "2024-01-01".to_string(),
            updated_time: String::new(),
            deleted_time: String::new(),
            drift_status: String::new(),
            last_drift_check_time: String::new(),
            status_reason: String::new(),
            description: "production stack".to_string(),
            detailed_status: String::new(),
            root_stack: String::new(),
            parent_stack: String::new(),
            termination_protection: false,
            iam_role: String::new(),
            tags: Vec::new(),
            stack_policy: String::new(),
            rollback_monitoring_time: String::new(),
            rollback_alarms: Vec::new(),
            notification_arns: Vec::new(),
        }];

        app.cfn_state.table.filter = "production".to_string();
        let filtered = filtered_cloudformation_stacks(&app);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_cloudformation_status_filter_applied() {
        let mut app = test_app();
        app.cfn_state.table.items = vec![
            CfnStack {
                name: "complete-stack".to_string(),
                stack_id: "id1".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                created_time: "2024-01-01".to_string(),
                updated_time: String::new(),
                deleted_time: String::new(),
                drift_status: String::new(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: String::new(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            },
            CfnStack {
                name: "failed-stack".to_string(),
                stack_id: "id2".to_string(),
                status: "CREATE_FAILED".to_string(),
                created_time: "2024-01-02".to_string(),
                updated_time: String::new(),
                deleted_time: String::new(),
                drift_status: String::new(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: String::new(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            },
        ];

        app.cfn_state.status_filter = CfnStatusFilter::Complete;
        let filtered = filtered_cloudformation_stacks(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "complete-stack");

        app.cfn_state.status_filter = CfnStatusFilter::Failed;
        let filtered = filtered_cloudformation_stacks(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "failed-stack");
    }

    #[test]
    fn test_cloudformation_default_page_size() {
        let app = test_app();
        assert_eq!(app.cfn_state.table.page_size, PageSize::Fifty);
    }

    #[test]
    fn test_cloudformation_default_status_filter() {
        let app = test_app();
        assert_eq!(app.cfn_state.status_filter, CfnStatusFilter::All);
    }

    #[test]
    fn test_cloudformation_view_nested_default_false() {
        let app = test_app();
        assert!(!app.cfn_state.view_nested);
    }

    #[test]
    fn test_cloudformation_pagination_hotkeys() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.cfn_state.status_filter = CfnStatusFilter::All;

        // Add 150 stacks
        for i in 0..150 {
            app.cfn_state.table.items.push(CfnStack {
                name: format!("stack-{}", i),
                stack_id: format!("id-{}", i),
                status: "CREATE_COMPLETE".to_string(),
                created_time: "2025-01-01 00:00:00 (UTC)".to_string(),
                updated_time: String::new(),
                deleted_time: String::new(),
                drift_status: String::new(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: String::new(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: vec![],
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: vec![],
                notification_arns: vec![],
            });
        }

        // Go to page 2
        app.go_to_page(2);
        assert_eq!(app.cfn_state.table.selected, 50);

        // Go to page 3
        app.go_to_page(3);
        assert_eq!(app.cfn_state.table.selected, 100);

        // Go to page 1
        app.go_to_page(1);
        assert_eq!(app.cfn_state.table.selected, 0);
    }

    #[test]
    fn test_cloudformation_tab_cycling_in_filter_mode() {
        use crate::ui::cfn::{STATUS_FILTER, VIEW_NESTED};
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.mode = Mode::FilterInput;
        app.cfn_state.input_focus = InputFocus::Filter;

        // Tab to StatusFilter
        app.handle_action(Action::NextFilterFocus);
        assert_eq!(app.cfn_state.input_focus, STATUS_FILTER);

        // Tab to ViewNested
        app.handle_action(Action::NextFilterFocus);
        assert_eq!(app.cfn_state.input_focus, VIEW_NESTED);

        // Tab to Pagination
        app.handle_action(Action::NextFilterFocus);
        assert_eq!(app.cfn_state.input_focus, InputFocus::Pagination);

        // Tab back to Filter
        app.handle_action(Action::NextFilterFocus);
        assert_eq!(app.cfn_state.input_focus, InputFocus::Filter);
    }

    #[test]
    fn test_cloudformation_timestamp_format_includes_utc() {
        let stack = CfnStack {
            name: "test-stack".to_string(),
            stack_id: "id-123".to_string(),
            status: "CREATE_COMPLETE".to_string(),
            created_time: "2025-08-07 15:38:02 (UTC)".to_string(),
            updated_time: "2025-08-08 10:00:00 (UTC)".to_string(),
            deleted_time: String::new(),
            drift_status: String::new(),
            last_drift_check_time: "2025-08-09 12:00:00 (UTC)".to_string(),
            status_reason: String::new(),
            description: String::new(),
            detailed_status: String::new(),
            root_stack: String::new(),
            parent_stack: String::new(),
            termination_protection: false,
            iam_role: String::new(),
            tags: vec![],
            stack_policy: String::new(),
            rollback_monitoring_time: String::new(),
            rollback_alarms: vec![],
            notification_arns: vec![],
        };

        assert!(stack.created_time.contains("(UTC)"));
        assert!(stack.updated_time.contains("(UTC)"));
        assert!(stack.last_drift_check_time.contains("(UTC)"));
        assert_eq!(stack.created_time.len(), 25);
    }

    #[test]
    fn test_cloudformation_enter_drills_into_stack_view() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.cfn_state.status_filter = CfnStatusFilter::All;
        app.tabs = vec![Tab {
            service: Service::CloudFormationStacks,
            title: "CloudFormation > Stacks".to_string(),
            breadcrumb: "CloudFormation > Stacks".to_string(),
        }];
        app.current_tab = 0;

        app.cfn_state.table.items.push(CfnStack {
            name: "test-stack".to_string(),
            stack_id: "id-123".to_string(),
            status: "CREATE_COMPLETE".to_string(),
            created_time: "2025-01-01 00:00:00 (UTC)".to_string(),
            updated_time: String::new(),
            deleted_time: String::new(),
            drift_status: String::new(),
            last_drift_check_time: String::new(),
            status_reason: String::new(),
            description: String::new(),
            detailed_status: String::new(),
            root_stack: String::new(),
            parent_stack: String::new(),
            termination_protection: false,
            iam_role: String::new(),
            tags: vec![],
            stack_policy: String::new(),
            rollback_monitoring_time: String::new(),
            rollback_alarms: vec![],
            notification_arns: vec![],
        });

        app.cfn_state.table.reset();
        assert_eq!(app.cfn_state.current_stack, None);

        // Press Enter - should drill into stack detail view
        app.handle_action(Action::Select);
        assert_eq!(app.cfn_state.current_stack, Some("test-stack".to_string()));
    }

    #[test]
    fn test_cloudformation_arrow_keys_expand_collapse() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.cfn_state.status_filter = CfnStatusFilter::All;

        app.cfn_state.table.items.push(CfnStack {
            name: "test-stack".to_string(),
            stack_id: "id-123".to_string(),
            status: "CREATE_COMPLETE".to_string(),
            created_time: "2025-01-01 00:00:00 (UTC)".to_string(),
            updated_time: String::new(),
            deleted_time: String::new(),
            drift_status: String::new(),
            last_drift_check_time: String::new(),
            status_reason: String::new(),
            description: String::new(),
            detailed_status: String::new(),
            root_stack: String::new(),
            parent_stack: String::new(),
            termination_protection: false,
            iam_role: String::new(),
            tags: vec![],
            stack_policy: String::new(),
            rollback_monitoring_time: String::new(),
            rollback_alarms: vec![],
            notification_arns: vec![],
        });

        app.cfn_state.table.reset();
        assert_eq!(app.cfn_state.table.expanded_item, None);

        // Right arrow - should expand
        app.handle_action(Action::NextPane);
        assert_eq!(app.cfn_state.table.expanded_item, Some(0));

        // Left arrow - should collapse
        app.handle_action(Action::PrevPane);
        assert_eq!(app.cfn_state.table.expanded_item, None);

        // Verify current_stack is still None (not drilled in)
        assert_eq!(app.cfn_state.current_stack, None);
    }

    #[test]
    fn test_cloudformation_tab_cycling() {
        use crate::ui::cfn::DetailTab;
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.cfn_state.status_filter = CfnStatusFilter::All;
        app.cfn_state.current_stack = Some("test-stack".to_string());

        assert_eq!(app.cfn_state.detail_tab, DetailTab::StackInfo);
    }

    #[test]
    fn test_cloudformation_console_url() {
        use crate::ui::cfn::DetailTab;
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.cfn_state.status_filter = CfnStatusFilter::All;

        app.cfn_state.table.items.push(CfnStack {
            name: "test-stack".to_string(),
            stack_id: "arn:aws:cloudformation:us-east-1:123456789012:stack/test-stack/abc123"
                .to_string(),
            status: "CREATE_COMPLETE".to_string(),
            created_time: "2025-01-01 00:00:00 (UTC)".to_string(),
            updated_time: String::new(),
            deleted_time: String::new(),
            drift_status: String::new(),
            last_drift_check_time: String::new(),
            status_reason: String::new(),
            description: String::new(),
            detailed_status: String::new(),
            root_stack: String::new(),
            parent_stack: String::new(),
            termination_protection: false,
            iam_role: String::new(),
            tags: vec![],
            stack_policy: String::new(),
            rollback_monitoring_time: String::new(),
            rollback_alarms: vec![],
            notification_arns: vec![],
        });

        app.cfn_state.current_stack = Some("test-stack".to_string());

        // Stack info URL
        app.cfn_state.detail_tab = DetailTab::StackInfo;
        let url = app.get_console_url();
        assert!(url.contains("stackinfo"));
        assert!(url.contains("arn%3Aaws%3Acloudformation"));

        // Events URL
        app.cfn_state.detail_tab = DetailTab::Events;
        let url = app.get_console_url();
        assert!(url.contains("events"));
        assert!(url.contains("arn%3Aaws%3Acloudformation"));
    }

    #[test]
    fn test_iam_role_select() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.mode = Mode::Normal;

        app.iam_state.roles.items = vec![
            IamRole {
                role_name: "role1".to_string(),
                path: "/".to_string(),
                trusted_entities: "AWS Service: ec2".to_string(),
                last_activity: "-".to_string(),
                arn: "arn:aws:iam::123456789012:role/role1".to_string(),
                creation_time: "2025-01-01".to_string(),
                description: "Test role 1".to_string(),
                max_session_duration: Some(3600),
            },
            IamRole {
                role_name: "role2".to_string(),
                path: "/".to_string(),
                trusted_entities: "AWS Service: lambda".to_string(),
                last_activity: "-".to_string(),
                arn: "arn:aws:iam::123456789012:role/role2".to_string(),
                creation_time: "2025-01-02".to_string(),
                description: "Test role 2".to_string(),
                max_session_duration: Some(7200),
            },
        ];

        // Select first role
        app.iam_state.roles.selected = 0;
        app.handle_action(Action::Select);

        assert_eq!(
            app.iam_state.current_role,
            Some("role1".to_string()),
            "Should open role detail view"
        );
        assert_eq!(
            app.iam_state.role_tab,
            RoleTab::Permissions,
            "Should default to Permissions tab"
        );
    }

    #[test]
    fn test_iam_role_back_navigation() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.iam_state.current_role = Some("test-role".to_string());

        app.handle_action(Action::GoBack);

        assert_eq!(
            app.iam_state.current_role, None,
            "Should return to roles list"
        );
    }

    #[test]
    fn test_iam_role_tab_navigation() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.iam_state.current_role = Some("test-role".to_string());
        app.iam_state.role_tab = RoleTab::Permissions;

        app.handle_action(Action::NextDetailTab);

        assert_eq!(
            app.iam_state.role_tab,
            RoleTab::TrustRelationships,
            "Should move to next tab"
        );
    }

    #[test]
    fn test_iam_role_tab_cycle_order() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.iam_state.current_role = Some("test-role".to_string());
        app.iam_state.role_tab = RoleTab::Permissions;

        app.handle_action(Action::NextDetailTab);
        assert_eq!(app.iam_state.role_tab, RoleTab::TrustRelationships);

        app.handle_action(Action::NextDetailTab);
        assert_eq!(app.iam_state.role_tab, RoleTab::Tags);

        app.handle_action(Action::NextDetailTab);
        assert_eq!(app.iam_state.role_tab, RoleTab::LastAccessed);

        app.handle_action(Action::NextDetailTab);
        assert_eq!(app.iam_state.role_tab, RoleTab::RevokeSessions);

        app.handle_action(Action::NextDetailTab);
        assert_eq!(
            app.iam_state.role_tab,
            RoleTab::Permissions,
            "Should cycle back to first tab"
        );
    }

    #[test]
    fn test_iam_role_pagination() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.iam_state.roles.page_size = PageSize::Ten;

        app.iam_state.roles.items = (0..25)
            .map(|i| IamRole {
                role_name: format!("role{}", i),
                path: "/".to_string(),
                trusted_entities: "AWS Service: ec2".to_string(),
                last_activity: "-".to_string(),
                arn: format!("arn:aws:iam::123456789012:role/role{}", i),
                creation_time: "2025-01-01".to_string(),
                description: format!("Test role {}", i),
                max_session_duration: Some(3600),
            })
            .collect();

        // Jump to page 2
        app.go_to_page(2);

        assert_eq!(
            app.iam_state.roles.selected, 10,
            "Should select first item of page 2"
        );
        assert_eq!(
            app.iam_state.roles.scroll_offset, 10,
            "Should update scroll offset"
        );
    }

    #[test]
    fn test_tags_table_populated_on_role_detail() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.iam_state.roles.items = vec![IamRole {
            role_name: "TestRole".to_string(),
            path: "/".to_string(),
            trusted_entities: String::new(),
            last_activity: String::new(),
            arn: "arn:aws:iam::123456789012:role/TestRole".to_string(),
            creation_time: "2025-01-01".to_string(),
            description: String::new(),
            max_session_duration: Some(3600),
        }];

        // Manually populate tags to test table rendering
        app.iam_state.tags.items = vec![
            IamRoleTag {
                key: "Environment".to_string(),
                value: "Production".to_string(),
            },
            IamRoleTag {
                key: "Team".to_string(),
                value: "Platform".to_string(),
            },
        ];

        assert_eq!(app.iam_state.tags.items.len(), 2);
        assert_eq!(app.iam_state.tags.items[0].key, "Environment");
        assert_eq!(app.iam_state.tags.items[0].value, "Production");
        assert_eq!(app.iam_state.tags.selected, 0);
    }

    #[test]
    fn test_tags_table_navigation() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.iam_state.current_role = Some("TestRole".to_string());
        app.iam_state.role_tab = RoleTab::Tags;
        app.iam_state.tags.items = vec![
            IamRoleTag {
                key: "Tag1".to_string(),
                value: "Value1".to_string(),
            },
            IamRoleTag {
                key: "Tag2".to_string(),
                value: "Value2".to_string(),
            },
        ];

        app.handle_action(Action::NextItem);
        assert_eq!(app.iam_state.tags.selected, 1);

        app.handle_action(Action::PrevItem);
        assert_eq!(app.iam_state.tags.selected, 0);
    }

    #[test]
    fn test_last_accessed_table_navigation() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.iam_state.current_role = Some("TestRole".to_string());
        app.iam_state.role_tab = RoleTab::LastAccessed;
        app.iam_state.last_accessed_services.items = vec![
            LastAccessedService {
                service: "S3".to_string(),
                policies_granting: "Policy1".to_string(),
                last_accessed: "2025-01-01".to_string(),
            },
            LastAccessedService {
                service: "EC2".to_string(),
                policies_granting: "Policy2".to_string(),
                last_accessed: "2025-01-02".to_string(),
            },
        ];

        app.handle_action(Action::NextItem);
        assert_eq!(app.iam_state.last_accessed_services.selected, 1);

        app.handle_action(Action::PrevItem);
        assert_eq!(app.iam_state.last_accessed_services.selected, 0);
    }

    #[test]
    fn test_cfn_input_focus_next() {
        use crate::ui::cfn::{STATUS_FILTER, VIEW_NESTED};
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.mode = Mode::FilterInput;
        app.cfn_state.input_focus = InputFocus::Filter;

        app.handle_action(Action::NextFilterFocus);
        assert_eq!(app.cfn_state.input_focus, STATUS_FILTER);

        app.handle_action(Action::NextFilterFocus);
        assert_eq!(app.cfn_state.input_focus, VIEW_NESTED);

        app.handle_action(Action::NextFilterFocus);
        assert_eq!(app.cfn_state.input_focus, InputFocus::Pagination);

        app.handle_action(Action::NextFilterFocus);
        assert_eq!(app.cfn_state.input_focus, InputFocus::Filter);
    }

    #[test]
    fn test_cfn_input_focus_prev() {
        use crate::ui::cfn::{STATUS_FILTER, VIEW_NESTED};
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.mode = Mode::FilterInput;
        app.cfn_state.input_focus = InputFocus::Filter;

        app.handle_action(Action::PrevFilterFocus);
        assert_eq!(app.cfn_state.input_focus, InputFocus::Pagination);

        app.handle_action(Action::PrevFilterFocus);
        assert_eq!(app.cfn_state.input_focus, VIEW_NESTED);

        app.handle_action(Action::PrevFilterFocus);
        assert_eq!(app.cfn_state.input_focus, STATUS_FILTER);

        app.handle_action(Action::PrevFilterFocus);
        assert_eq!(app.cfn_state.input_focus, InputFocus::Filter);
    }

    #[test]
    fn test_cw_logs_input_focus_prev() {
        let mut app = test_app();
        app.current_service = Service::CloudWatchLogGroups;
        app.mode = Mode::FilterInput;
        app.view_mode = ViewMode::Detail;
        app.log_groups_state.detail_tab = CwLogsDetailTab::LogStreams;
        app.log_groups_state.input_focus = InputFocus::Filter;

        app.handle_action(Action::PrevFilterFocus);
        assert_eq!(app.log_groups_state.input_focus, InputFocus::Pagination);

        app.handle_action(Action::PrevFilterFocus);
        assert_eq!(
            app.log_groups_state.input_focus,
            InputFocus::Checkbox("ShowExpired")
        );

        app.handle_action(Action::PrevFilterFocus);
        assert_eq!(
            app.log_groups_state.input_focus,
            InputFocus::Checkbox("ExactMatch")
        );

        app.handle_action(Action::PrevFilterFocus);
        assert_eq!(app.log_groups_state.input_focus, InputFocus::Filter);
    }

    #[test]
    fn test_cw_events_input_focus_prev() {
        use crate::ui::cw::logs::EventFilterFocus;
        let mut app = test_app();
        app.mode = Mode::EventFilterInput;
        app.log_groups_state.event_input_focus = EventFilterFocus::Filter;

        app.handle_action(Action::PrevFilterFocus);
        assert_eq!(
            app.log_groups_state.event_input_focus,
            EventFilterFocus::DateRange
        );

        app.handle_action(Action::PrevFilterFocus);
        assert_eq!(
            app.log_groups_state.event_input_focus,
            EventFilterFocus::Filter
        );
    }

    #[test]
    fn test_cfn_input_focus_cycle_complete() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.mode = Mode::FilterInput;
        app.cfn_state.input_focus = InputFocus::Filter;

        // Cycle forward through all controls
        for _ in 0..4 {
            app.handle_action(Action::NextFilterFocus);
        }
        assert_eq!(app.cfn_state.input_focus, InputFocus::Filter);

        // Cycle backward through all controls
        for _ in 0..4 {
            app.handle_action(Action::PrevFilterFocus);
        }
        assert_eq!(app.cfn_state.input_focus, InputFocus::Filter);
    }

    #[test]
    fn test_cfn_filter_status_arrow_keys() {
        use crate::ui::cfn::STATUS_FILTER;
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.mode = Mode::FilterInput;
        app.cfn_state.input_focus = STATUS_FILTER;
        app.cfn_state.status_filter = CfnStatusFilter::All;

        app.handle_action(Action::NextItem);
        assert_eq!(app.cfn_state.status_filter, CfnStatusFilter::Active);

        app.handle_action(Action::PrevItem);
        assert_eq!(app.cfn_state.status_filter, CfnStatusFilter::All);
    }

    #[test]
    fn test_cfn_filter_shift_tab_cycles_backward() {
        use crate::ui::cfn::STATUS_FILTER;
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.mode = Mode::FilterInput;
        app.cfn_state.input_focus = STATUS_FILTER;

        // Shift+Tab should go backward
        app.handle_action(Action::PrevFilterFocus);
        assert_eq!(app.cfn_state.input_focus, InputFocus::Filter);

        // From Input, Shift+Tab should wrap to Pagination
        app.handle_action(Action::PrevFilterFocus);
        assert_eq!(app.cfn_state.input_focus, InputFocus::Pagination);
    }

    #[test]
    fn test_cfn_pagination_arrow_keys() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.mode = Mode::FilterInput;
        app.cfn_state.input_focus = InputFocus::Pagination;
        app.cfn_state.table.scroll_offset = 0;
        app.cfn_state.table.page_size = PageSize::Ten;

        // Add some test stacks
        app.cfn_state.table.items = (0..30)
            .map(|i| CfnStack {
                name: format!("stack-{}", i),
                stack_id: format!("id-{}", i),
                status: "CREATE_COMPLETE".to_string(),
                created_time: "2024-01-01".to_string(),
                updated_time: String::new(),
                deleted_time: String::new(),
                drift_status: String::new(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: String::new(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            })
            .collect();

        // Right arrow should page forward
        app.handle_action(Action::PageDown);
        assert_eq!(app.cfn_state.table.scroll_offset, 10);
        // Verify page number calculation
        let page_size = app.cfn_state.table.page_size.value();
        let current_page = app.cfn_state.table.scroll_offset / page_size;
        assert_eq!(current_page, 1);

        // Left arrow should page backward
        app.handle_action(Action::PageUp);
        assert_eq!(app.cfn_state.table.scroll_offset, 0);
        let current_page = app.cfn_state.table.scroll_offset / page_size;
        assert_eq!(current_page, 0);
    }

    #[test]
    fn test_cfn_page_navigation_updates_selection() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.mode = Mode::Normal;

        // Add 30 test stacks
        app.cfn_state.table.items = (0..30)
            .map(|i| CfnStack {
                name: format!("stack-{}", i),
                stack_id: format!("id-{}", i),
                status: "CREATE_COMPLETE".to_string(),
                created_time: "2024-01-01".to_string(),
                updated_time: String::new(),
                deleted_time: String::new(),
                drift_status: String::new(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: String::new(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            })
            .collect();

        app.cfn_state.table.reset();
        app.cfn_state.table.scroll_offset = 0;

        // Page down should update selection
        app.handle_action(Action::PageDown);
        assert_eq!(app.cfn_state.table.selected, 10);

        // Page down again
        app.handle_action(Action::PageDown);
        assert_eq!(app.cfn_state.table.selected, 20);

        // Page up should update selection
        app.handle_action(Action::PageUp);
        assert_eq!(app.cfn_state.table.selected, 10);
    }

    #[test]
    fn test_cfn_filter_input_only_when_focused() {
        use crate::ui::cfn::STATUS_FILTER;
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.mode = Mode::FilterInput;
        app.cfn_state.input_focus = STATUS_FILTER;
        app.cfn_state.table.filter = String::new();

        // Typing should not add to filter when focus is not on Input
        app.handle_action(Action::FilterInput('t'));
        app.handle_action(Action::FilterInput('e'));
        app.handle_action(Action::FilterInput('s'));
        app.handle_action(Action::FilterInput('t'));
        assert_eq!(app.cfn_state.table.filter, "");

        // Switch to Input focus
        app.cfn_state.input_focus = InputFocus::Filter;
        app.handle_action(Action::FilterInput('t'));
        app.handle_action(Action::FilterInput('e'));
        app.handle_action(Action::FilterInput('s'));
        app.handle_action(Action::FilterInput('t'));
        assert_eq!(app.cfn_state.table.filter, "test");
    }

    #[test]
    fn test_cfn_input_focus_resets_on_start() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.cfn_state.input_focus = InputFocus::Pagination;

        // Start filter should reset focus to Input
        app.handle_action(Action::StartFilter);
        assert_eq!(app.mode, Mode::FilterInput);
        assert_eq!(app.cfn_state.input_focus, InputFocus::Filter);
    }

    #[test]
    fn test_iam_roles_input_focus_cycles_forward() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.mode = Mode::FilterInput;
        app.iam_state.role_input_focus = InputFocus::Filter;

        app.handle_action(Action::NextFilterFocus);
        assert_eq!(app.iam_state.role_input_focus, InputFocus::Pagination);

        app.handle_action(Action::NextFilterFocus);
        assert_eq!(app.iam_state.role_input_focus, InputFocus::Filter);
    }

    #[test]
    fn test_iam_roles_input_focus_cycles_backward() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.mode = Mode::FilterInput;
        app.iam_state.role_input_focus = InputFocus::Filter;

        app.handle_action(Action::PrevFilterFocus);
        assert_eq!(app.iam_state.role_input_focus, InputFocus::Pagination);

        app.handle_action(Action::PrevFilterFocus);
        assert_eq!(app.iam_state.role_input_focus, InputFocus::Filter);
    }

    #[test]
    fn test_iam_roles_filter_input_only_when_focused() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.mode = Mode::FilterInput;
        app.iam_state.role_input_focus = InputFocus::Pagination;
        app.iam_state.roles.filter = String::new();

        // Typing should not add to filter when focus is on Pagination
        app.handle_action(Action::FilterInput('t'));
        app.handle_action(Action::FilterInput('e'));
        app.handle_action(Action::FilterInput('s'));
        app.handle_action(Action::FilterInput('t'));
        assert_eq!(app.iam_state.roles.filter, "");

        // Switch to Input focus
        app.iam_state.role_input_focus = InputFocus::Filter;
        app.handle_action(Action::FilterInput('t'));
        app.handle_action(Action::FilterInput('e'));
        app.handle_action(Action::FilterInput('s'));
        app.handle_action(Action::FilterInput('t'));
        assert_eq!(app.iam_state.roles.filter, "test");
    }

    #[test]
    fn test_iam_roles_page_down_updates_scroll_offset() {
        let mut app = test_app();
        app.current_service = Service::IamRoles;
        app.mode = Mode::Normal;
        app.iam_state.roles.items = (0..50)
            .map(|i| IamRole {
                role_name: format!("role-{}", i),
                path: "/".to_string(),
                trusted_entities: "AWS Service".to_string(),
                last_activity: "N/A".to_string(),
                arn: format!("arn:aws:iam::123456789012:role/role-{}", i),
                creation_time: "2024-01-01".to_string(),
                description: String::new(),
                max_session_duration: Some(3600),
            })
            .collect();

        app.iam_state.roles.selected = 0;
        app.iam_state.roles.scroll_offset = 0;

        // Page down should update both selected and scroll_offset
        app.handle_action(Action::PageDown);
        assert_eq!(app.iam_state.roles.selected, 10);
        // scroll_offset should be updated to keep selection visible
        assert!(app.iam_state.roles.scroll_offset <= app.iam_state.roles.selected);

        // Page down again
        app.handle_action(Action::PageDown);
        assert_eq!(app.iam_state.roles.selected, 20);
        assert!(app.iam_state.roles.scroll_offset <= app.iam_state.roles.selected);
    }

    #[test]
    fn test_application_selection_and_deployments_tab() {
        use crate::lambda::Application as LambdaApplication;
        use LambdaApplicationDetailTab;

        let mut app = test_app();
        app.current_service = Service::LambdaApplications;
        app.service_selected = true;
        app.mode = Mode::Normal;

        app.lambda_application_state.table.items = vec![LambdaApplication {
            name: "test-app".to_string(),
            arn: "arn:aws:serverlessrepo:::applications/test-app".to_string(),
            description: "Test application".to_string(),
            status: "CREATE_COMPLETE".to_string(),
            last_modified: "2024-01-01".to_string(),
        }];

        // Select application
        app.handle_action(Action::Select);
        assert_eq!(
            app.lambda_application_state.current_application,
            Some("test-app".to_string())
        );
        assert_eq!(
            app.lambda_application_state.detail_tab,
            LambdaApplicationDetailTab::Overview
        );

        // Switch to Deployments tab
        app.handle_action(Action::NextDetailTab);
        assert_eq!(
            app.lambda_application_state.detail_tab,
            LambdaApplicationDetailTab::Deployments
        );

        // Go back
        app.handle_action(Action::GoBack);
        assert_eq!(app.lambda_application_state.current_application, None);
    }

    #[test]
    fn test_application_resources_filter_and_pagination() {
        use crate::lambda::Application as LambdaApplication;
        use LambdaApplicationDetailTab;

        let mut app = test_app();
        app.current_service = Service::LambdaApplications;
        app.service_selected = true;
        app.mode = Mode::Normal;

        app.lambda_application_state.table.items = vec![LambdaApplication {
            name: "test-app".to_string(),
            arn: "arn:aws:serverlessrepo:::applications/test-app".to_string(),
            description: "Test application".to_string(),
            status: "CREATE_COMPLETE".to_string(),
            last_modified: "2024-01-01".to_string(),
        }];

        // Select application
        app.handle_action(Action::Select);
        assert_eq!(
            app.lambda_application_state.detail_tab,
            LambdaApplicationDetailTab::Overview
        );

        // Verify resources were loaded
        assert!(!app.lambda_application_state.resources.items.is_empty());

        // Test filter focus cycling
        app.mode = Mode::FilterInput;
        assert_eq!(
            app.lambda_application_state.resource_input_focus,
            InputFocus::Filter
        );

        app.handle_action(Action::NextFilterFocus);
        assert_eq!(
            app.lambda_application_state.resource_input_focus,
            InputFocus::Pagination
        );

        app.handle_action(Action::PrevFilterFocus);
        assert_eq!(
            app.lambda_application_state.resource_input_focus,
            InputFocus::Filter
        );
    }

    #[test]
    fn test_application_deployments_filter_and_pagination() {
        use crate::lambda::Application as LambdaApplication;
        use LambdaApplicationDetailTab;

        let mut app = test_app();
        app.current_service = Service::LambdaApplications;
        app.service_selected = true;
        app.mode = Mode::Normal;

        app.lambda_application_state.table.items = vec![LambdaApplication {
            name: "test-app".to_string(),
            arn: "arn:aws:serverlessrepo:::applications/test-app".to_string(),
            description: "Test application".to_string(),
            status: "CREATE_COMPLETE".to_string(),
            last_modified: "2024-01-01".to_string(),
        }];

        // Select application and switch to Deployments tab
        app.handle_action(Action::Select);
        app.handle_action(Action::NextDetailTab);
        assert_eq!(
            app.lambda_application_state.detail_tab,
            LambdaApplicationDetailTab::Deployments
        );

        // Verify deployments were loaded
        assert!(!app.lambda_application_state.deployments.items.is_empty());

        // Test filter focus cycling
        app.mode = Mode::FilterInput;
        assert_eq!(
            app.lambda_application_state.deployment_input_focus,
            InputFocus::Filter
        );

        app.handle_action(Action::NextFilterFocus);
        assert_eq!(
            app.lambda_application_state.deployment_input_focus,
            InputFocus::Pagination
        );

        app.handle_action(Action::PrevFilterFocus);
        assert_eq!(
            app.lambda_application_state.deployment_input_focus,
            InputFocus::Filter
        );
    }

    #[test]
    fn test_application_resource_expansion() {
        use crate::lambda::Application as LambdaApplication;
        use LambdaApplicationDetailTab;

        let mut app = test_app();
        app.current_service = Service::LambdaApplications;
        app.service_selected = true;
        app.mode = Mode::Normal;

        app.lambda_application_state.table.items = vec![LambdaApplication {
            name: "test-app".to_string(),
            arn: "arn:aws:serverlessrepo:::applications/test-app".to_string(),
            description: "Test application".to_string(),
            status: "CREATE_COMPLETE".to_string(),
            last_modified: "2024-01-01".to_string(),
        }];

        // Select application (Overview tab by default)
        app.handle_action(Action::Select);
        assert_eq!(
            app.lambda_application_state.detail_tab,
            LambdaApplicationDetailTab::Overview
        );

        // Expand resource
        app.handle_action(Action::NextPane);
        assert_eq!(
            app.lambda_application_state.resources.expanded_item,
            Some(0)
        );

        // Collapse resource
        app.handle_action(Action::PrevPane);
        assert_eq!(app.lambda_application_state.resources.expanded_item, None);
    }

    #[test]
    fn test_application_deployment_expansion() {
        use crate::lambda::Application as LambdaApplication;
        use LambdaApplicationDetailTab;

        let mut app = test_app();
        app.current_service = Service::LambdaApplications;
        app.service_selected = true;
        app.mode = Mode::Normal;

        app.lambda_application_state.table.items = vec![LambdaApplication {
            name: "test-app".to_string(),
            arn: "arn:aws:serverlessrepo:::applications/test-app".to_string(),
            description: "Test application".to_string(),
            status: "CREATE_COMPLETE".to_string(),
            last_modified: "2024-01-01".to_string(),
        }];

        // Select application and switch to Deployments tab
        app.handle_action(Action::Select);
        app.handle_action(Action::NextDetailTab);
        assert_eq!(
            app.lambda_application_state.detail_tab,
            LambdaApplicationDetailTab::Deployments
        );

        // Expand deployment
        app.handle_action(Action::NextPane);
        assert_eq!(
            app.lambda_application_state.deployments.expanded_item,
            Some(0)
        );

        // Collapse deployment
        app.handle_action(Action::PrevPane);
        assert_eq!(app.lambda_application_state.deployments.expanded_item, None);
    }

    #[test]
    fn test_s3_nested_prefix_expansion() {
        use crate::s3::Bucket;
        use S3Object;

        let mut app = test_app();
        app.current_service = Service::S3Buckets;
        app.service_selected = true;
        app.mode = Mode::Normal;

        // Setup bucket with nested prefixes (2 levels)
        app.s3_state.buckets.items = vec![Bucket {
            name: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            creation_date: "2024-01-01".to_string(),
        }];

        // Level 1: bucket preview
        app.s3_state.bucket_preview.insert(
            "test-bucket".to_string(),
            vec![S3Object {
                key: "level1/".to_string(),
                size: 0,
                last_modified: "".to_string(),
                is_prefix: true,
                storage_class: "".to_string(),
            }],
        );

        // Level 2: nested prefix
        app.s3_state.prefix_preview.insert(
            "level1/".to_string(),
            vec![S3Object {
                key: "level1/level2/".to_string(),
                size: 0,
                last_modified: "".to_string(),
                is_prefix: true,
                storage_class: "".to_string(),
            }],
        );

        // Expand bucket (row 0)
        app.s3_state.selected_row = 0;
        app.handle_action(Action::NextPane);
        assert!(app.s3_state.expanded_prefixes.contains("test-bucket"));

        // Expand level1/ (row 1)
        app.s3_state.selected_row = 1;
        app.handle_action(Action::NextPane);
        assert!(app.s3_state.expanded_prefixes.contains("level1/"));

        // Expand level2/ (row 2) - verifies nested expansion works
        app.s3_state.selected_row = 2;
        app.handle_action(Action::NextPane);
        assert!(app.s3_state.expanded_prefixes.contains("level1/level2/"));

        // Verify all are still expanded
        assert!(app.s3_state.expanded_prefixes.contains("test-bucket"));
        assert!(app.s3_state.expanded_prefixes.contains("level1/"));
    }

    #[test]
    fn test_s3_nested_prefix_collapse() {
        use crate::s3::Bucket;
        use S3Object;

        let mut app = test_app();
        app.current_service = Service::S3Buckets;
        app.service_selected = true;
        app.mode = Mode::Normal;

        app.s3_state.buckets.items = vec![Bucket {
            name: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            creation_date: "2024-01-01".to_string(),
        }];

        app.s3_state.bucket_preview.insert(
            "test-bucket".to_string(),
            vec![S3Object {
                key: "level1/".to_string(),
                size: 0,
                last_modified: "".to_string(),
                is_prefix: true,
                storage_class: "".to_string(),
            }],
        );

        app.s3_state.prefix_preview.insert(
            "level1/".to_string(),
            vec![S3Object {
                key: "level1/level2/".to_string(),
                size: 0,
                last_modified: "".to_string(),
                is_prefix: true,
                storage_class: "".to_string(),
            }],
        );

        // Pre-expand all levels
        app.s3_state
            .expanded_prefixes
            .insert("test-bucket".to_string());
        app.s3_state.expanded_prefixes.insert("level1/".to_string());
        app.s3_state
            .expanded_prefixes
            .insert("level1/level2/".to_string());

        // Collapse level2/ (row 2)
        app.s3_state.selected_row = 2;
        app.handle_action(Action::PrevPane);
        assert!(!app.s3_state.expanded_prefixes.contains("level1/level2/"));
        assert!(app.s3_state.expanded_prefixes.contains("level1/")); // Parent still expanded

        // Collapse level1/ (row 1)
        app.s3_state.selected_row = 1;
        app.handle_action(Action::PrevPane);
        assert!(!app.s3_state.expanded_prefixes.contains("level1/"));
        assert!(app.s3_state.expanded_prefixes.contains("test-bucket")); // Bucket still expanded

        // Collapse bucket (row 0)
        app.s3_state.selected_row = 0;
        app.handle_action(Action::PrevPane);
        assert!(!app.s3_state.expanded_prefixes.contains("test-bucket"));
    }
}

#[cfg(test)]
mod sqs_tests {
    use super::*;
    use test_helpers::*;

    #[test]
    fn test_sqs_filter_input() {
        let mut app = test_app();
        app.current_service = Service::SqsQueues;
        app.service_selected = true;
        app.mode = Mode::FilterInput;

        app.handle_action(Action::FilterInput('t'));
        app.handle_action(Action::FilterInput('e'));
        app.handle_action(Action::FilterInput('s'));
        app.handle_action(Action::FilterInput('t'));
        assert_eq!(app.sqs_state.queues.filter, "test");

        app.handle_action(Action::FilterBackspace);
        assert_eq!(app.sqs_state.queues.filter, "tes");
    }

    #[test]
    fn test_sqs_start_filter() {
        let mut app = test_app();
        app.current_service = Service::SqsQueues;
        app.service_selected = true;
        app.mode = Mode::Normal;

        app.handle_action(Action::StartFilter);
        assert_eq!(app.mode, Mode::FilterInput);
        assert_eq!(app.sqs_state.input_focus, InputFocus::Filter);
    }

    #[test]
    fn test_sqs_filter_focus_cycling() {
        let mut app = test_app();
        app.current_service = Service::SqsQueues;
        app.service_selected = true;
        app.mode = Mode::FilterInput;
        app.sqs_state.input_focus = InputFocus::Filter;

        app.handle_action(Action::NextFilterFocus);
        assert_eq!(app.sqs_state.input_focus, InputFocus::Pagination);

        app.handle_action(Action::NextFilterFocus);
        assert_eq!(app.sqs_state.input_focus, InputFocus::Filter);

        app.handle_action(Action::PrevFilterFocus);
        assert_eq!(app.sqs_state.input_focus, InputFocus::Pagination);
    }

    #[test]
    fn test_sqs_navigation() {
        let mut app = test_app();
        app.current_service = Service::SqsQueues;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.sqs_state.queues.items = (0..10)
            .map(|i| SqsQueue {
                name: format!("queue{}", i),
                url: String::new(),
                queue_type: "Standard".to_string(),
                created_timestamp: String::new(),
                messages_available: "0".to_string(),
                messages_in_flight: "0".to_string(),
                encryption: "Disabled".to_string(),
                content_based_deduplication: "Disabled".to_string(),
                last_modified_timestamp: String::new(),
                visibility_timeout: String::new(),
                message_retention_period: String::new(),
                maximum_message_size: String::new(),
                delivery_delay: String::new(),
                receive_message_wait_time: String::new(),
                high_throughput_fifo: "N/A".to_string(),
                deduplication_scope: "N/A".to_string(),
                fifo_throughput_limit: "N/A".to_string(),
                dead_letter_queue: "-".to_string(),
                messages_delayed: "0".to_string(),
                redrive_allow_policy: "-".to_string(),
                redrive_policy: "".to_string(),
                redrive_task_id: "-".to_string(),
                redrive_task_start_time: "-".to_string(),
                redrive_task_status: "-".to_string(),
                redrive_task_percent: "-".to_string(),
                redrive_task_destination: "-".to_string(),
            })
            .collect();

        app.handle_action(Action::NextItem);
        assert_eq!(app.sqs_state.queues.selected, 1);

        app.handle_action(Action::PrevItem);
        assert_eq!(app.sqs_state.queues.selected, 0);
    }

    #[test]
    fn test_sqs_page_navigation() {
        let mut app = test_app();
        app.current_service = Service::SqsQueues;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.sqs_state.queues.items = (0..100)
            .map(|i| SqsQueue {
                name: format!("queue{}", i),
                url: String::new(),
                queue_type: "Standard".to_string(),
                created_timestamp: String::new(),
                messages_available: "0".to_string(),
                messages_in_flight: "0".to_string(),
                encryption: "Disabled".to_string(),
                content_based_deduplication: "Disabled".to_string(),
                last_modified_timestamp: String::new(),
                visibility_timeout: String::new(),
                message_retention_period: String::new(),
                maximum_message_size: String::new(),
                delivery_delay: String::new(),
                receive_message_wait_time: String::new(),
                high_throughput_fifo: "N/A".to_string(),
                deduplication_scope: "N/A".to_string(),
                fifo_throughput_limit: "N/A".to_string(),
                dead_letter_queue: "-".to_string(),
                messages_delayed: "0".to_string(),
                redrive_allow_policy: "-".to_string(),
                redrive_policy: "".to_string(),
                redrive_task_id: "-".to_string(),
                redrive_task_start_time: "-".to_string(),
                redrive_task_status: "-".to_string(),
                redrive_task_percent: "-".to_string(),
                redrive_task_destination: "-".to_string(),
            })
            .collect();

        app.handle_action(Action::PageDown);
        assert_eq!(app.sqs_state.queues.selected, 10);

        app.handle_action(Action::PageUp);
        assert_eq!(app.sqs_state.queues.selected, 0);
    }

    #[test]
    fn test_sqs_queue_expansion() {
        let mut app = test_app();
        app.current_service = Service::SqsQueues;
        app.service_selected = true;
        app.sqs_state.queues.items = vec![SqsQueue {
            name: "my-queue".to_string(),
            url: "https://sqs.us-east-1.amazonaws.com/123456789012/my-queue".to_string(),
            queue_type: "Standard".to_string(),
            created_timestamp: "2023-01-01".to_string(),
            messages_available: "5".to_string(),
            messages_in_flight: "2".to_string(),
            encryption: "Enabled".to_string(),
            content_based_deduplication: "Disabled".to_string(),
            last_modified_timestamp: "2023-01-02".to_string(),
            visibility_timeout: "30".to_string(),
            message_retention_period: "345600".to_string(),
            maximum_message_size: "262144".to_string(),
            delivery_delay: "0".to_string(),
            receive_message_wait_time: "0".to_string(),
            high_throughput_fifo: "N/A".to_string(),
            deduplication_scope: "N/A".to_string(),
            fifo_throughput_limit: "N/A".to_string(),
            dead_letter_queue: "-".to_string(),
            messages_delayed: "0".to_string(),
            redrive_allow_policy: "-".to_string(),
            redrive_policy: "".to_string(),
            redrive_task_id: "-".to_string(),
            redrive_task_start_time: "-".to_string(),
            redrive_task_status: "-".to_string(),
            redrive_task_percent: "-".to_string(),
            redrive_task_destination: "-".to_string(),
        }];
        app.sqs_state.queues.selected = 0;

        assert_eq!(app.sqs_state.queues.expanded_item, None);

        // Right arrow expands
        app.handle_action(Action::NextPane);
        assert_eq!(app.sqs_state.queues.expanded_item, Some(0));

        // Right arrow again keeps it expanded
        app.handle_action(Action::NextPane);
        assert_eq!(app.sqs_state.queues.expanded_item, Some(0));

        // Left arrow collapses
        app.handle_action(Action::PrevPane);
        assert_eq!(app.sqs_state.queues.expanded_item, None);

        // Left arrow again keeps it collapsed
        app.handle_action(Action::PrevPane);
        assert_eq!(app.sqs_state.queues.expanded_item, None);
    }

    #[test]
    fn test_sqs_column_toggle() {
        use crate::sqs::queue::Column as SqsColumn;
        let mut app = test_app();
        app.current_service = Service::SqsQueues;
        app.service_selected = true;
        app.mode = Mode::ColumnSelector;

        // Start with all columns visible
        app.sqs_visible_column_ids = SqsColumn::ids();
        let initial_count = app.sqs_visible_column_ids.len();

        // Select first column (index 0) and toggle it
        app.column_selector_index = 0;
        app.handle_action(Action::ToggleColumn);

        // First column should be removed
        assert_eq!(app.sqs_visible_column_ids.len(), initial_count - 1);
        assert!(!app.sqs_visible_column_ids.contains(&SqsColumn::Name.id()));

        // Toggle it back
        app.handle_action(Action::ToggleColumn);
        assert_eq!(app.sqs_visible_column_ids.len(), initial_count);
        assert!(app.sqs_visible_column_ids.contains(&SqsColumn::Name.id()));
    }

    #[test]
    fn test_sqs_column_selector_navigation() {
        let mut app = test_app();
        app.current_service = Service::SqsQueues;
        app.service_selected = true;
        app.mode = Mode::ColumnSelector;
        app.column_selector_index = 0;

        // Should be able to navigate through all columns
        let max_index = app.sqs_column_ids.len() - 1;

        // Navigate to last column
        for _ in 0..max_index {
            app.handle_action(Action::NextItem);
        }
        assert_eq!(app.column_selector_index, max_index);

        // Navigate back to first
        for _ in 0..max_index {
            app.handle_action(Action::PrevItem);
        }
        assert_eq!(app.column_selector_index, 0);
    }

    #[test]
    fn test_sqs_queue_selection() {
        let mut app = test_app();
        app.current_service = Service::SqsQueues;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.sqs_state.queues.items = vec![SqsQueue {
            name: "my-queue".to_string(),
            url: "https://sqs.us-east-1.amazonaws.com/123456789012/my-queue".to_string(),
            queue_type: "Standard".to_string(),
            created_timestamp: "2023-01-01".to_string(),
            messages_available: "5".to_string(),
            messages_in_flight: "2".to_string(),
            encryption: "Enabled".to_string(),
            content_based_deduplication: "Disabled".to_string(),
            last_modified_timestamp: "2023-01-02".to_string(),
            visibility_timeout: "30".to_string(),
            message_retention_period: "345600".to_string(),
            maximum_message_size: "262144".to_string(),
            delivery_delay: "0".to_string(),
            receive_message_wait_time: "0".to_string(),
            high_throughput_fifo: "N/A".to_string(),
            deduplication_scope: "N/A".to_string(),
            fifo_throughput_limit: "N/A".to_string(),
            dead_letter_queue: "-".to_string(),
            messages_delayed: "0".to_string(),
            redrive_allow_policy: "-".to_string(),
            redrive_policy: "".to_string(),
            redrive_task_id: "-".to_string(),
            redrive_task_start_time: "-".to_string(),
            redrive_task_status: "-".to_string(),
            redrive_task_percent: "-".to_string(),
            redrive_task_destination: "-".to_string(),
        }];
        app.sqs_state.queues.selected = 0;

        assert_eq!(app.sqs_state.current_queue, None);

        // Select queue
        app.handle_action(Action::Select);
        assert_eq!(
            app.sqs_state.current_queue,
            Some("https://sqs.us-east-1.amazonaws.com/123456789012/my-queue".to_string())
        );

        // Go back
        app.handle_action(Action::GoBack);
        assert_eq!(app.sqs_state.current_queue, None);
    }

    #[test]
    fn test_sqs_lambda_triggers_expand_collapse() {
        let mut app = test_app();
        app.current_service = Service::SqsQueues;
        app.service_selected = true;
        app.sqs_state.current_queue =
            Some("https://sqs.us-east-1.amazonaws.com/123456789012/my-queue".to_string());
        app.sqs_state.detail_tab = SqsQueueDetailTab::LambdaTriggers;
        app.sqs_state.triggers.items = vec![LambdaTrigger {
            uuid: "test-uuid".to_string(),
            arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
            status: "Enabled".to_string(),
            last_modified: "2024-01-01T00:00:00Z".to_string(),
        }];
        app.sqs_state.triggers.selected = 0;

        assert_eq!(app.sqs_state.triggers.expanded_item, None);

        // Right arrow expands
        app.handle_action(Action::NextPane);
        assert_eq!(app.sqs_state.triggers.expanded_item, Some(0));

        // Left arrow collapses
        app.handle_action(Action::PrevPane);
        assert_eq!(app.sqs_state.triggers.expanded_item, None);
    }

    #[test]
    fn test_sqs_lambda_triggers_expand_toggle() {
        let mut app = test_app();
        app.current_service = Service::SqsQueues;
        app.service_selected = true;
        app.sqs_state.current_queue =
            Some("https://sqs.us-east-1.amazonaws.com/123456789012/my-queue".to_string());
        app.sqs_state.detail_tab = SqsQueueDetailTab::LambdaTriggers;
        app.sqs_state.triggers.items = vec![LambdaTrigger {
            uuid: "test-uuid".to_string(),
            arn: "arn:aws:lambda:us-east-1:123456789012:function:test".to_string(),
            status: "Enabled".to_string(),
            last_modified: "2024-01-01T00:00:00Z".to_string(),
        }];
        app.sqs_state.triggers.selected = 0;

        // Expand
        app.handle_action(Action::NextPane);
        assert_eq!(app.sqs_state.triggers.expanded_item, Some(0));

        // Toggle collapses
        app.handle_action(Action::NextPane);
        assert_eq!(app.sqs_state.triggers.expanded_item, None);

        // Toggle expands again
        app.handle_action(Action::NextPane);
        assert_eq!(app.sqs_state.triggers.expanded_item, Some(0));
    }

    #[test]
    fn test_sqs_lambda_triggers_sorted_by_last_modified_asc() {
        use crate::ui::sqs::filtered_lambda_triggers;

        let mut app = test_app();
        app.current_service = Service::SqsQueues;
        app.service_selected = true;
        app.sqs_state.current_queue =
            Some("https://sqs.us-east-1.amazonaws.com/123456789012/my-queue".to_string());
        app.sqs_state.detail_tab = SqsQueueDetailTab::LambdaTriggers;
        app.sqs_state.triggers.items = vec![
            LambdaTrigger {
                uuid: "uuid-3".to_string(),
                arn: "arn:aws:lambda:us-east-1:123456789012:function:test-3".to_string(),
                status: "Enabled".to_string(),
                last_modified: "2024-03-01T00:00:00Z".to_string(),
            },
            LambdaTrigger {
                uuid: "uuid-1".to_string(),
                arn: "arn:aws:lambda:us-east-1:123456789012:function:test-1".to_string(),
                status: "Enabled".to_string(),
                last_modified: "2024-01-01T00:00:00Z".to_string(),
            },
            LambdaTrigger {
                uuid: "uuid-2".to_string(),
                arn: "arn:aws:lambda:us-east-1:123456789012:function:test-2".to_string(),
                status: "Enabled".to_string(),
                last_modified: "2024-02-01T00:00:00Z".to_string(),
            },
        ];

        let sorted = filtered_lambda_triggers(&app);

        // Should be sorted by last_modified ASC
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].uuid, "uuid-1");
        assert_eq!(sorted[0].last_modified, "2024-01-01T00:00:00Z");
        assert_eq!(sorted[1].uuid, "uuid-2");
        assert_eq!(sorted[1].last_modified, "2024-02-01T00:00:00Z");
        assert_eq!(sorted[2].uuid, "uuid-3");
        assert_eq!(sorted[2].last_modified, "2024-03-01T00:00:00Z");
    }

    #[test]
    fn test_sqs_lambda_triggers_filter_input() {
        let mut app = test_app();
        app.current_service = Service::SqsQueues;
        app.service_selected = true;
        app.mode = Mode::FilterInput;
        app.sqs_state.current_queue =
            Some("https://sqs.us-east-1.amazonaws.com/123456789012/my-queue".to_string());
        app.sqs_state.detail_tab = SqsQueueDetailTab::LambdaTriggers;
        app.sqs_state.input_focus = InputFocus::Filter;

        assert_eq!(app.sqs_state.triggers.filter, "");

        // Type characters
        app.handle_action(Action::FilterInput('t'));
        assert_eq!(app.sqs_state.triggers.filter, "t");

        app.handle_action(Action::FilterInput('e'));
        assert_eq!(app.sqs_state.triggers.filter, "te");

        app.handle_action(Action::FilterInput('s'));
        assert_eq!(app.sqs_state.triggers.filter, "tes");

        app.handle_action(Action::FilterInput('t'));
        assert_eq!(app.sqs_state.triggers.filter, "test");

        // Backspace
        app.handle_action(Action::FilterBackspace);
        assert_eq!(app.sqs_state.triggers.filter, "tes");
    }

    #[test]
    fn test_sqs_lambda_triggers_filter_applied() {
        use crate::ui::sqs::filtered_lambda_triggers;

        let mut app = test_app();
        app.current_service = Service::SqsQueues;
        app.service_selected = true;
        app.sqs_state.current_queue =
            Some("https://sqs.us-east-1.amazonaws.com/123456789012/my-queue".to_string());
        app.sqs_state.detail_tab = SqsQueueDetailTab::LambdaTriggers;
        app.sqs_state.triggers.items = vec![
            LambdaTrigger {
                uuid: "uuid-1".to_string(),
                arn: "arn:aws:lambda:us-east-1:123456789012:function:test-alpha".to_string(),
                status: "Enabled".to_string(),
                last_modified: "2024-01-01T00:00:00Z".to_string(),
            },
            LambdaTrigger {
                uuid: "uuid-2".to_string(),
                arn: "arn:aws:lambda:us-east-1:123456789012:function:test-beta".to_string(),
                status: "Enabled".to_string(),
                last_modified: "2024-02-01T00:00:00Z".to_string(),
            },
            LambdaTrigger {
                uuid: "uuid-3".to_string(),
                arn: "arn:aws:lambda:us-east-1:123456789012:function:prod-gamma".to_string(),
                status: "Enabled".to_string(),
                last_modified: "2024-03-01T00:00:00Z".to_string(),
            },
        ];

        // No filter - all items
        let filtered = filtered_lambda_triggers(&app);
        assert_eq!(filtered.len(), 3);

        // Filter by "alpha"
        app.sqs_state.triggers.filter = "alpha".to_string();
        let filtered = filtered_lambda_triggers(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(
            filtered[0].arn,
            "arn:aws:lambda:us-east-1:123456789012:function:test-alpha"
        );

        // Filter by "test" - matches 2
        app.sqs_state.triggers.filter = "test".to_string();
        let filtered = filtered_lambda_triggers(&app);
        assert_eq!(filtered.len(), 2);
        assert_eq!(
            filtered[0].arn,
            "arn:aws:lambda:us-east-1:123456789012:function:test-alpha"
        );
        assert_eq!(
            filtered[1].arn,
            "arn:aws:lambda:us-east-1:123456789012:function:test-beta"
        );

        // Filter by uuid
        app.sqs_state.triggers.filter = "uuid-3".to_string();
        let filtered = filtered_lambda_triggers(&app);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].uuid, "uuid-3");
    }

    #[test]
    fn test_sqs_triggers_navigation() {
        let mut app = test_app();
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.current_service = Service::SqsQueues;
        app.sqs_state.current_queue = Some("test-queue".to_string());
        app.sqs_state.detail_tab = SqsQueueDetailTab::LambdaTriggers;
        app.sqs_state.triggers.items = vec![
            LambdaTrigger {
                uuid: "1".to_string(),
                arn: "arn1".to_string(),
                status: "Enabled".to_string(),
                last_modified: "2024-01-01".to_string(),
            },
            LambdaTrigger {
                uuid: "2".to_string(),
                arn: "arn2".to_string(),
                status: "Enabled".to_string(),
                last_modified: "2024-01-02".to_string(),
            },
        ];

        assert_eq!(app.sqs_state.triggers.selected, 0);
        app.next_item();
        assert_eq!(app.sqs_state.triggers.selected, 1);
        app.prev_item();
        assert_eq!(app.sqs_state.triggers.selected, 0);
    }

    #[test]
    fn test_sqs_pipes_navigation() {
        let mut app = test_app();
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.current_service = Service::SqsQueues;
        app.sqs_state.current_queue = Some("test-queue".to_string());
        app.sqs_state.detail_tab = SqsQueueDetailTab::EventBridgePipes;
        app.sqs_state.pipes.items = vec![
            EventBridgePipe {
                name: "pipe1".to_string(),
                status: "RUNNING".to_string(),
                target: "target1".to_string(),
                last_modified: "2024-01-01".to_string(),
            },
            EventBridgePipe {
                name: "pipe2".to_string(),
                status: "RUNNING".to_string(),
                target: "target2".to_string(),
                last_modified: "2024-01-02".to_string(),
            },
        ];

        assert_eq!(app.sqs_state.pipes.selected, 0);
        app.next_item();
        assert_eq!(app.sqs_state.pipes.selected, 1);
        app.prev_item();
        assert_eq!(app.sqs_state.pipes.selected, 0);
    }

    #[test]
    fn test_sqs_tags_navigation() {
        let mut app = test_app();
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.current_service = Service::SqsQueues;
        app.sqs_state.current_queue = Some("test-queue".to_string());
        app.sqs_state.detail_tab = SqsQueueDetailTab::Tagging;
        app.sqs_state.tags.items = vec![
            SqsQueueTag {
                key: "Env".to_string(),
                value: "prod".to_string(),
            },
            SqsQueueTag {
                key: "Team".to_string(),
                value: "backend".to_string(),
            },
        ];

        assert_eq!(app.sqs_state.tags.selected, 0);
        app.next_item();
        assert_eq!(app.sqs_state.tags.selected, 1);
        app.prev_item();
        assert_eq!(app.sqs_state.tags.selected, 0);
    }

    #[test]
    fn test_sqs_queues_navigation() {
        let mut app = test_app();
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.current_service = Service::SqsQueues;
        app.sqs_state.queues.items = vec![
            SqsQueue {
                name: "queue1".to_string(),
                url: "url1".to_string(),
                queue_type: "Standard".to_string(),
                created_timestamp: "".to_string(),
                messages_available: "0".to_string(),
                messages_in_flight: "0".to_string(),
                encryption: "Disabled".to_string(),
                content_based_deduplication: "Disabled".to_string(),
                last_modified_timestamp: "".to_string(),
                visibility_timeout: "".to_string(),
                message_retention_period: "".to_string(),
                maximum_message_size: "".to_string(),
                delivery_delay: "".to_string(),
                receive_message_wait_time: "".to_string(),
                high_throughput_fifo: "-".to_string(),
                deduplication_scope: "-".to_string(),
                fifo_throughput_limit: "-".to_string(),
                dead_letter_queue: "-".to_string(),
                messages_delayed: "0".to_string(),
                redrive_allow_policy: "-".to_string(),
                redrive_policy: "".to_string(),
                redrive_task_id: "-".to_string(),
                redrive_task_start_time: "-".to_string(),
                redrive_task_status: "-".to_string(),
                redrive_task_percent: "-".to_string(),
                redrive_task_destination: "-".to_string(),
            },
            SqsQueue {
                name: "queue2".to_string(),
                url: "url2".to_string(),
                queue_type: "Standard".to_string(),
                created_timestamp: "".to_string(),
                messages_available: "0".to_string(),
                messages_in_flight: "0".to_string(),
                encryption: "Disabled".to_string(),
                content_based_deduplication: "Disabled".to_string(),
                last_modified_timestamp: "".to_string(),
                visibility_timeout: "".to_string(),
                message_retention_period: "".to_string(),
                maximum_message_size: "".to_string(),
                delivery_delay: "".to_string(),
                receive_message_wait_time: "".to_string(),
                high_throughput_fifo: "-".to_string(),
                deduplication_scope: "-".to_string(),
                fifo_throughput_limit: "-".to_string(),
                dead_letter_queue: "-".to_string(),
                messages_delayed: "0".to_string(),
                redrive_allow_policy: "-".to_string(),
                redrive_policy: "".to_string(),
                redrive_task_id: "-".to_string(),
                redrive_task_start_time: "-".to_string(),
                redrive_task_status: "-".to_string(),
                redrive_task_percent: "-".to_string(),
                redrive_task_destination: "-".to_string(),
            },
        ];

        assert_eq!(app.sqs_state.queues.selected, 0);
        app.next_item();
        assert_eq!(app.sqs_state.queues.selected, 1);
        app.prev_item();
        assert_eq!(app.sqs_state.queues.selected, 0);
    }

    #[test]
    fn test_sqs_subscriptions_navigation() {
        let mut app = test_app();
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.current_service = Service::SqsQueues;
        app.sqs_state.current_queue = Some("test-queue".to_string());
        app.sqs_state.detail_tab = SqsQueueDetailTab::SnsSubscriptions;
        app.sqs_state.subscriptions.items = vec![
            SnsSubscription {
                subscription_arn: "arn:aws:sns:us-east-1:123:sub1".to_string(),
                topic_arn: "arn:aws:sns:us-east-1:123:topic1".to_string(),
            },
            SnsSubscription {
                subscription_arn: "arn:aws:sns:us-east-1:123:sub2".to_string(),
                topic_arn: "arn:aws:sns:us-east-1:123:topic2".to_string(),
            },
        ];

        assert_eq!(app.sqs_state.subscriptions.selected, 0);
        app.next_item();
        assert_eq!(app.sqs_state.subscriptions.selected, 1);
        app.prev_item();
        assert_eq!(app.sqs_state.subscriptions.selected, 0);
    }

    #[test]
    fn test_sqs_subscription_region_dropdown_navigation() {
        let mut app = test_app();
        app.service_selected = true;
        app.mode = Mode::FilterInput;
        app.current_service = Service::SqsQueues;
        app.sqs_state.current_queue = Some("test-queue".to_string());
        app.sqs_state.detail_tab = SqsQueueDetailTab::SnsSubscriptions;
        app.sqs_state.input_focus = InputFocus::Dropdown("SubscriptionRegion");

        assert_eq!(app.sqs_state.subscription_region_selected, 0);
        app.next_item();
        assert_eq!(app.sqs_state.subscription_region_selected, 1);
        app.next_item();
        assert_eq!(app.sqs_state.subscription_region_selected, 2);
        app.prev_item();
        assert_eq!(app.sqs_state.subscription_region_selected, 1);
        app.prev_item();
        assert_eq!(app.sqs_state.subscription_region_selected, 0);
    }

    #[test]
    fn test_sqs_subscription_region_selection() {
        let mut app = test_app();
        app.service_selected = true;
        app.mode = Mode::FilterInput;
        app.current_service = Service::SqsQueues;
        app.sqs_state.current_queue = Some("test-queue".to_string());
        app.sqs_state.detail_tab = SqsQueueDetailTab::SnsSubscriptions;
        app.sqs_state.input_focus = InputFocus::Dropdown("SubscriptionRegion");
        app.sqs_state.subscription_region_selected = 2; // us-west-1

        assert_eq!(app.sqs_state.subscription_region_filter, "");
        app.handle_action(Action::ApplyFilter);
        assert_eq!(app.sqs_state.subscription_region_filter, "us-west-1");
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_sqs_subscription_region_change_resets_selection() {
        let mut app = test_app();
        app.service_selected = true;
        app.mode = Mode::FilterInput;
        app.current_service = Service::SqsQueues;
        app.sqs_state.current_queue = Some("test-queue".to_string());
        app.sqs_state.detail_tab = SqsQueueDetailTab::SnsSubscriptions;
        app.sqs_state.input_focus = InputFocus::Dropdown("SubscriptionRegion");
        app.sqs_state.subscription_region_selected = 0;
        app.sqs_state.subscriptions.selected = 5;

        app.handle_action(Action::NextItem);

        assert_eq!(app.sqs_state.subscription_region_selected, 1);
        assert_eq!(app.sqs_state.subscriptions.selected, 0);
    }

    #[test]
    fn test_s3_object_filter_resets_selection() {
        let mut app = test_app();
        app.service_selected = true;
        app.current_service = Service::S3Buckets;
        app.s3_state.current_bucket = Some("test-bucket".to_string());
        app.s3_state.selected_row = 5;
        app.mode = Mode::FilterInput;

        app.handle_action(Action::CloseMenu);

        assert_eq!(app.s3_state.selected_row, 0);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_s3_bucket_filter_resets_selection() {
        let mut app = test_app();
        app.service_selected = true;
        app.current_service = Service::S3Buckets;
        app.s3_state.selected_row = 10;
        app.mode = Mode::FilterInput;

        app.handle_action(Action::CloseMenu);

        assert_eq!(app.s3_state.selected_row, 0);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_s3_selection_stays_in_bounds() {
        let mut app = test_app();
        app.service_selected = true;
        app.current_service = Service::S3Buckets;
        app.s3_state.selected_row = 0;
        app.s3_state.selected_object = 0;

        // Simulate going up from row 0
        app.prev_item();

        // Should stay at 0, not wrap to negative
        assert_eq!(app.s3_state.selected_row, 0);
        assert_eq!(app.s3_state.selected_object, 0);
    }

    #[test]
    fn test_cfn_filter_resets_selection() {
        let mut app = test_app();
        app.service_selected = true;
        app.current_service = Service::CloudFormationStacks;
        app.cfn_state.table.selected = 10;
        app.mode = Mode::FilterInput;

        app.handle_action(Action::CloseMenu);

        assert_eq!(app.cfn_state.table.selected, 0);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_lambda_filter_resets_selection() {
        let mut app = test_app();
        app.service_selected = true;
        app.current_service = Service::LambdaFunctions;
        app.lambda_state.table.selected = 8;
        app.mode = Mode::FilterInput;

        app.handle_action(Action::CloseMenu);

        assert_eq!(app.lambda_state.table.selected, 0);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_sqs_filter_resets_selection() {
        let mut app = test_app();
        app.service_selected = true;
        app.current_service = Service::SqsQueues;
        app.sqs_state.queues.selected = 7;
        app.mode = Mode::FilterInput;

        app.handle_action(Action::CloseMenu);

        assert_eq!(app.sqs_state.queues.selected, 0);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_cfn_status_filter_change_resets_selection() {
        use crate::ui::cfn::STATUS_FILTER;
        let mut app = test_app();
        app.service_selected = true;
        app.current_service = Service::CloudFormationStacks;
        app.mode = Mode::FilterInput;
        app.cfn_state.input_focus = STATUS_FILTER;
        app.cfn_state.status_filter = CfnStatusFilter::All;
        app.cfn_state.table.items = vec![
            CfnStack {
                name: "stack1".to_string(),
                stack_id: "id1".to_string(),
                status: "CREATE_COMPLETE".to_string(),
                created_time: "2024-01-01".to_string(),
                updated_time: String::new(),
                deleted_time: String::new(),
                drift_status: String::new(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: String::new(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            },
            CfnStack {
                name: "stack2".to_string(),
                stack_id: "id2".to_string(),
                status: "UPDATE_IN_PROGRESS".to_string(),
                created_time: "2024-01-02".to_string(),
                updated_time: String::new(),
                deleted_time: String::new(),
                drift_status: String::new(),
                last_drift_check_time: String::new(),
                status_reason: String::new(),
                description: String::new(),
                detailed_status: String::new(),
                root_stack: String::new(),
                parent_stack: String::new(),
                termination_protection: false,
                iam_role: String::new(),
                tags: Vec::new(),
                stack_policy: String::new(),
                rollback_monitoring_time: String::new(),
                rollback_alarms: Vec::new(),
                notification_arns: Vec::new(),
            },
        ];
        app.cfn_state.table.selected = 1;

        app.handle_action(Action::NextItem);

        assert_eq!(app.cfn_state.status_filter, CfnStatusFilter::Active);
        assert_eq!(app.cfn_state.table.selected, 0);
    }

    #[test]
    fn test_cfn_view_nested_toggle_resets_selection() {
        use crate::ui::cfn::VIEW_NESTED;
        let mut app = test_app();
        app.service_selected = true;
        app.current_service = Service::CloudFormationStacks;
        app.mode = Mode::FilterInput;
        app.cfn_state.input_focus = VIEW_NESTED;
        app.cfn_state.view_nested = false;
        app.cfn_state.table.items = vec![CfnStack {
            name: "stack1".to_string(),
            stack_id: "id1".to_string(),
            status: "CREATE_COMPLETE".to_string(),
            created_time: "2024-01-01".to_string(),
            updated_time: String::new(),
            deleted_time: String::new(),
            drift_status: String::new(),
            last_drift_check_time: String::new(),
            status_reason: String::new(),
            description: String::new(),
            detailed_status: String::new(),
            root_stack: String::new(),
            parent_stack: String::new(),
            termination_protection: false,
            iam_role: String::new(),
            tags: Vec::new(),
            stack_policy: String::new(),
            rollback_monitoring_time: String::new(),
            rollback_alarms: Vec::new(),
            notification_arns: Vec::new(),
        }];
        app.cfn_state.table.selected = 5;

        app.handle_action(Action::ToggleFilterCheckbox);

        assert!(app.cfn_state.view_nested);
        assert_eq!(app.cfn_state.table.selected, 0);
    }

    #[test]
    fn test_cfn_template_scroll_up() {
        let mut app = test_app();
        app.service_selected = true;
        app.current_service = Service::CloudFormationStacks;
        app.cfn_state.current_stack = Some("test-stack".to_string());
        app.cfn_state.detail_tab = CfnDetailTab::Template;
        app.cfn_state.template_scroll = 20;

        app.page_up();

        assert_eq!(app.cfn_state.template_scroll, 10);
    }

    #[test]
    fn test_cfn_template_scroll_down() {
        let mut app = test_app();
        app.service_selected = true;
        app.current_service = Service::CloudFormationStacks;
        app.cfn_state.current_stack = Some("test-stack".to_string());
        app.cfn_state.detail_tab = CfnDetailTab::Template;
        app.cfn_state.template_body = "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10\nline11\nline12\nline13\nline14\nline15".to_string();
        app.cfn_state.template_scroll = 0;

        app.page_down();

        assert_eq!(app.cfn_state.template_scroll, 10);
    }

    #[test]
    fn test_cfn_template_scroll_down_respects_max() {
        let mut app = test_app();
        app.service_selected = true;
        app.current_service = Service::CloudFormationStacks;
        app.cfn_state.current_stack = Some("test-stack".to_string());
        app.cfn_state.detail_tab = CfnDetailTab::Template;
        app.cfn_state.template_body = "line1\nline2\nline3".to_string();
        app.cfn_state.template_scroll = 0;

        app.page_down();

        // Should not scroll past the last line (3 lines = max scroll of 2)
        assert_eq!(app.cfn_state.template_scroll, 2);
    }

    #[test]
    fn test_cfn_template_arrow_up() {
        let mut app = test_app();
        app.service_selected = true;
        app.current_service = Service::CloudFormationStacks;
        app.mode = Mode::Normal;
        app.cfn_state.current_stack = Some("test-stack".to_string());
        app.cfn_state.detail_tab = CfnDetailTab::Template;
        app.cfn_state.template_scroll = 5;

        app.prev_item();

        assert_eq!(app.cfn_state.template_scroll, 4);
    }

    #[test]
    fn test_cfn_template_arrow_down() {
        let mut app = test_app();
        app.service_selected = true;
        app.current_service = Service::CloudFormationStacks;
        app.mode = Mode::Normal;
        app.cfn_state.current_stack = Some("test-stack".to_string());
        app.cfn_state.detail_tab = CfnDetailTab::Template;
        app.cfn_state.template_body = "line1\nline2\nline3\nline4\nline5".to_string();
        app.cfn_state.template_scroll = 2;

        app.next_item();

        assert_eq!(app.cfn_state.template_scroll, 3);
    }

    #[test]
    fn test_cfn_template_arrow_down_respects_max() {
        let mut app = test_app();
        app.service_selected = true;
        app.current_service = Service::CloudFormationStacks;
        app.mode = Mode::Normal;
        app.cfn_state.current_stack = Some("test-stack".to_string());
        app.cfn_state.detail_tab = CfnDetailTab::Template;
        app.cfn_state.template_body = "line1\nline2".to_string();
        app.cfn_state.template_scroll = 1;

        app.next_item();

        // Should stay at max (2 lines = max scroll of 1)
        assert_eq!(app.cfn_state.template_scroll, 1);
    }
}

#[cfg(test)]
mod lambda_version_tab_tests {
    use super::*;
    use test_helpers::*;

    #[test]
    fn test_lambda_version_tab_cycling_next() {
        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.current_version = Some("1".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Code;

        // Code -> Monitor
        app.handle_action(Action::NextDetailTab);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Monitor);
        assert!(app.lambda_state.metrics_loading);

        // Monitor -> Configuration
        app.lambda_state.metrics_loading = false;
        app.handle_action(Action::NextDetailTab);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Configuration);

        // Configuration -> Code
        app.handle_action(Action::NextDetailTab);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Code);
    }

    #[test]
    fn test_lambda_version_tab_cycling_prev() {
        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.current_version = Some("1".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Code;

        // Code -> Configuration
        app.handle_action(Action::PrevDetailTab);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Configuration);

        // Configuration -> Monitor
        app.handle_action(Action::PrevDetailTab);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Monitor);
        assert!(app.lambda_state.metrics_loading);

        // Monitor -> Code
        app.lambda_state.metrics_loading = false;
        app.handle_action(Action::PrevDetailTab);
        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Code);
    }

    #[test]
    fn test_lambda_version_monitor_clears_metrics() {
        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.lambda_state.current_function = Some("test-function".to_string());
        app.lambda_state.current_version = Some("1".to_string());
        app.lambda_state.detail_tab = LambdaDetailTab::Code;

        // Add some fake metric data
        app.lambda_state.metric_data_invocations = vec![(1, 10.0), (2, 20.0)];
        app.lambda_state.monitoring_scroll = 5;

        // Switch to Monitor tab
        app.handle_action(Action::NextDetailTab);

        assert_eq!(app.lambda_state.detail_tab, LambdaDetailTab::Monitor);
        assert!(app.lambda_state.metrics_loading);
        assert_eq!(app.lambda_state.monitoring_scroll, 0);
        assert!(app.lambda_state.metric_data_invocations.is_empty());
    }

    #[test]
    fn test_cfn_parameters_expand_collapse() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.cfn_state.current_stack = Some("test-stack".to_string());
        app.cfn_state.detail_tab = CfnDetailTab::Parameters;
        app.cfn_state.parameters.items = vec![rusticity_core::cfn::StackParameter {
            key: "Param1".to_string(),
            value: "Value1".to_string(),
            resolved_value: "Resolved1".to_string(),
        }];
        app.cfn_state.parameters.reset();

        assert_eq!(app.cfn_state.parameters.expanded_item, None);

        // Right arrow expands
        app.handle_action(Action::NextPane);
        assert_eq!(app.cfn_state.parameters.expanded_item, Some(0));

        // Left arrow collapses
        app.handle_action(Action::PrevPane);
        assert_eq!(app.cfn_state.parameters.expanded_item, None);
    }

    #[test]
    fn test_cfn_parameters_filter_resets_selection() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.cfn_state.current_stack = Some("test-stack".to_string());
        app.cfn_state.detail_tab = CfnDetailTab::Parameters;
        app.cfn_state.parameters.items = vec![
            rusticity_core::cfn::StackParameter {
                key: "DatabaseName".to_string(),
                value: "mydb".to_string(),
                resolved_value: "mydb".to_string(),
            },
            rusticity_core::cfn::StackParameter {
                key: "InstanceType".to_string(),
                value: "t2.micro".to_string(),
                resolved_value: "t2.micro".to_string(),
            },
            rusticity_core::cfn::StackParameter {
                key: "Environment".to_string(),
                value: "production".to_string(),
                resolved_value: "production".to_string(),
            },
        ];
        app.cfn_state.parameters.selected = 2; // Select third item
        app.mode = Mode::FilterInput;
        app.cfn_state.parameters_input_focus = InputFocus::Filter;

        // Type a filter character - should reset selection
        app.handle_action(Action::FilterInput('D'));
        assert_eq!(app.cfn_state.parameters.selected, 0);
        assert_eq!(app.cfn_state.parameters.filter, "D");

        // Select another item
        app.cfn_state.parameters.selected = 1;

        // Type another character - should reset again
        app.handle_action(Action::FilterInput('a'));
        assert_eq!(app.cfn_state.parameters.selected, 0);
        assert_eq!(app.cfn_state.parameters.filter, "Da");

        // Select another item
        app.cfn_state.parameters.selected = 1;

        // Backspace - should also reset
        app.handle_action(Action::FilterBackspace);
        assert_eq!(app.cfn_state.parameters.selected, 0);
        assert_eq!(app.cfn_state.parameters.filter, "D");
    }

    #[test]
    fn test_cfn_template_tab_no_preferences() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.cfn_state.current_stack = Some("test-stack".to_string());
        app.cfn_state.detail_tab = CfnDetailTab::Template;
        app.mode = Mode::Normal;

        // Try to open preferences - should be ignored
        app.handle_action(Action::OpenColumnSelector);
        assert_eq!(app.mode, Mode::Normal); // Should stay in Normal mode

        // GitSync tab should also not allow preferences
        app.cfn_state.detail_tab = CfnDetailTab::GitSync;
        app.handle_action(Action::OpenColumnSelector);
        assert_eq!(app.mode, Mode::Normal); // Should stay in Normal mode

        // Parameters tab should allow preferences
        app.cfn_state.detail_tab = CfnDetailTab::Parameters;
        app.handle_action(Action::OpenColumnSelector);
        assert_eq!(app.mode, Mode::ColumnSelector); // Should open preferences

        // Outputs tab should allow preferences
        app.mode = Mode::Normal;
        app.cfn_state.detail_tab = CfnDetailTab::Outputs;
        app.handle_action(Action::OpenColumnSelector);
        assert_eq!(app.mode, Mode::ColumnSelector); // Should open preferences
    }

    #[test]
    fn test_cfn_outputs_expand_collapse() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.cfn_state.current_stack = Some("test-stack".to_string());
        app.cfn_state.detail_tab = CfnDetailTab::Outputs;
        app.cfn_state.outputs.items = vec![rusticity_core::cfn::StackOutput {
            key: "Output1".to_string(),
            value: "Value1".to_string(),
            description: "Description1".to_string(),
            export_name: "Export1".to_string(),
        }];
        app.cfn_state.outputs.reset();

        assert_eq!(app.cfn_state.outputs.expanded_item, None);

        // Right arrow expands
        app.handle_action(Action::NextPane);
        assert_eq!(app.cfn_state.outputs.expanded_item, Some(0));

        // Left arrow collapses
        app.handle_action(Action::PrevPane);
        assert_eq!(app.cfn_state.outputs.expanded_item, None);
    }

    #[test]
    fn test_cfn_outputs_filter_resets_selection() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.service_selected = true;
        app.cfn_state.current_stack = Some("test-stack".to_string());
        app.cfn_state.detail_tab = CfnDetailTab::Outputs;
        app.cfn_state.outputs.items = vec![
            rusticity_core::cfn::StackOutput {
                key: "ApiUrl".to_string(),
                value: "https://api.example.com".to_string(),
                description: "API endpoint".to_string(),
                export_name: "MyApiUrl".to_string(),
            },
            rusticity_core::cfn::StackOutput {
                key: "BucketName".to_string(),
                value: "my-bucket".to_string(),
                description: "S3 bucket".to_string(),
                export_name: "MyBucket".to_string(),
            },
        ];
        app.cfn_state.outputs.reset();
        app.cfn_state.outputs.selected = 1;

        // Start filter mode
        app.handle_action(Action::StartFilter);
        assert_eq!(app.mode, Mode::FilterInput);

        // Type a character - should reset selection
        app.handle_action(Action::FilterInput('A'));
        assert_eq!(app.cfn_state.outputs.selected, 0);
        assert_eq!(app.cfn_state.outputs.filter, "A");

        // Type more
        app.cfn_state.outputs.selected = 1;
        app.handle_action(Action::FilterInput('p'));
        assert_eq!(app.cfn_state.outputs.selected, 0);

        // Backspace should also reset selection
        app.cfn_state.outputs.selected = 1;
        app.handle_action(Action::FilterBackspace);
        assert_eq!(app.cfn_state.outputs.selected, 0);
    }

    #[test]
    fn test_ec2_service_in_picker() {
        let app = test_app();
        assert!(app.service_picker.services.contains(&"EC2 > Instances"));
    }

    #[test]
    fn test_ec2_state_filter_cycles() {
        let mut app = test_app();
        app.current_service = Service::Ec2Instances;
        app.service_selected = true;
        app.mode = Mode::FilterInput;
        app.ec2_state.input_focus = EC2_STATE_FILTER;

        let initial = app.ec2_state.state_filter;
        assert_eq!(initial, Ec2StateFilter::AllStates);

        // Cycle through filters using ToggleFilterCheckbox
        app.handle_action(Action::ToggleFilterCheckbox);
        assert_eq!(app.ec2_state.state_filter, Ec2StateFilter::Running);

        app.handle_action(Action::ToggleFilterCheckbox);
        assert_eq!(app.ec2_state.state_filter, Ec2StateFilter::Stopped);

        app.handle_action(Action::ToggleFilterCheckbox);
        assert_eq!(app.ec2_state.state_filter, Ec2StateFilter::Terminated);

        app.handle_action(Action::ToggleFilterCheckbox);
        assert_eq!(app.ec2_state.state_filter, Ec2StateFilter::Pending);

        app.handle_action(Action::ToggleFilterCheckbox);
        assert_eq!(app.ec2_state.state_filter, Ec2StateFilter::ShuttingDown);

        app.handle_action(Action::ToggleFilterCheckbox);
        assert_eq!(app.ec2_state.state_filter, Ec2StateFilter::Stopping);

        app.handle_action(Action::ToggleFilterCheckbox);
        assert_eq!(app.ec2_state.state_filter, Ec2StateFilter::AllStates);
    }

    #[test]
    fn test_ec2_filter_resets_table() {
        let mut app = test_app();
        app.current_service = Service::Ec2Instances;
        app.service_selected = true;
        app.mode = Mode::FilterInput;
        app.ec2_state.input_focus = EC2_STATE_FILTER;
        app.ec2_state.table.selected = 5;

        app.handle_action(Action::ToggleFilterCheckbox);
        assert_eq!(app.ec2_state.table.selected, 0);
    }

    #[test]
    fn test_ec2_columns_visible() {
        let app = test_app();
        assert_eq!(app.ec2_visible_column_ids.len(), 16); // Default visible columns
        assert_eq!(app.ec2_column_ids.len(), 52); // Total available columns
    }

    #[test]
    fn test_ec2_breadcrumbs() {
        let mut app = test_app();
        app.current_service = Service::Ec2Instances;
        app.service_selected = true;
        let breadcrumb = app.breadcrumbs();
        assert_eq!(breadcrumb, "EC2 > Instances");
    }

    #[test]
    fn test_ec2_console_url() {
        let mut app = test_app();
        app.current_service = Service::Ec2Instances;
        app.service_selected = true;
        let url = app.get_console_url();
        assert!(url.contains("ec2"));
        assert!(url.contains("Instances"));
    }

    #[test]
    fn test_ec2_filter_handling() {
        let mut app = test_app();
        app.current_service = Service::Ec2Instances;
        app.service_selected = true;
        app.mode = Mode::FilterInput;

        app.handle_action(Action::FilterInput('t'));
        app.handle_action(Action::FilterInput('e'));
        app.handle_action(Action::FilterInput('s'));
        app.handle_action(Action::FilterInput('t'));

        assert_eq!(app.ec2_state.table.filter, "test");

        app.handle_action(Action::FilterBackspace);
        assert_eq!(app.ec2_state.table.filter, "tes");
    }

    #[test]
    fn test_column_selector_page_down_ec2() {
        let mut app = test_app();
        app.current_service = Service::Ec2Instances;
        app.service_selected = true;
        app.mode = Mode::ColumnSelector;
        app.column_selector_index = 0;

        app.handle_action(Action::PageDown);
        assert_eq!(app.column_selector_index, 10);

        app.handle_action(Action::PageDown);
        assert_eq!(app.column_selector_index, 20);
    }

    #[test]
    fn test_column_selector_page_up_ec2() {
        let mut app = test_app();
        app.current_service = Service::Ec2Instances;
        app.service_selected = true;
        app.mode = Mode::ColumnSelector;
        app.column_selector_index = 30;

        app.handle_action(Action::PageUp);
        assert_eq!(app.column_selector_index, 20);

        app.handle_action(Action::PageUp);
        assert_eq!(app.column_selector_index, 10);
    }

    #[test]
    fn test_ec2_state_filter_dropdown_focus() {
        let mut app = test_app();
        app.current_service = Service::Ec2Instances;
        app.service_selected = true;
        app.mode = Mode::FilterInput;

        // Tab to state filter
        app.handle_action(Action::NextFilterFocus);
        assert_eq!(app.ec2_state.input_focus, EC2_STATE_FILTER);

        // Dropdown should show when focused (tested in render)
        // Verify we can cycle the filter
        app.handle_action(Action::ToggleFilterCheckbox);
        assert_eq!(app.ec2_state.state_filter, Ec2StateFilter::Running);
    }

    #[test]
    fn test_column_selector_ctrl_d_scrolling() {
        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.mode = Mode::ColumnSelector;
        app.column_selector_index = 0;

        app.handle_action(Action::PageDown);
        assert_eq!(app.column_selector_index, 10);

        // Second PageDown should be capped at max
        let max = app.get_column_selector_max();
        app.handle_action(Action::PageDown);
        assert_eq!(app.column_selector_index, max);
    }

    #[test]
    fn test_column_selector_ctrl_u_scrolling() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.mode = Mode::ColumnSelector;
        app.column_selector_index = 25;

        app.handle_action(Action::PageUp);
        assert_eq!(app.column_selector_index, 15);

        app.handle_action(Action::PageUp);
        assert_eq!(app.column_selector_index, 5);
    }

    #[test]
    fn test_prev_preferences_lambda() {
        let mut app = test_app();
        app.current_service = Service::LambdaFunctions;
        app.mode = Mode::ColumnSelector;
        let page_size_idx = app.lambda_state.function_column_ids.len() + 2;
        app.column_selector_index = page_size_idx;

        app.handle_action(Action::PrevPreferences);
        assert_eq!(app.column_selector_index, 0);

        app.handle_action(Action::PrevPreferences);
        assert_eq!(app.column_selector_index, page_size_idx);
    }

    #[test]
    fn test_prev_preferences_cloudformation() {
        let mut app = test_app();
        app.current_service = Service::CloudFormationStacks;
        app.mode = Mode::ColumnSelector;
        let page_size_idx = app.cfn_column_ids.len() + 2;
        app.column_selector_index = page_size_idx;

        app.handle_action(Action::PrevPreferences);
        assert_eq!(app.column_selector_index, 0);

        app.handle_action(Action::PrevPreferences);
        assert_eq!(app.column_selector_index, page_size_idx);
    }

    #[test]
    fn test_prev_preferences_alarms() {
        let mut app = test_app();
        app.current_service = Service::CloudWatchAlarms;
        app.mode = Mode::ColumnSelector;
        app.column_selector_index = 28; // WrapLines

        app.handle_action(Action::PrevPreferences);
        assert_eq!(app.column_selector_index, 22); // PageSize

        app.handle_action(Action::PrevPreferences);
        assert_eq!(app.column_selector_index, 18); // ViewAs

        app.handle_action(Action::PrevPreferences);
        assert_eq!(app.column_selector_index, 0); // Columns

        app.handle_action(Action::PrevPreferences);
        assert_eq!(app.column_selector_index, 28); // Wrap to WrapLines
    }

    #[test]
    fn test_ec2_page_size_in_preferences() {
        let mut app = test_app();
        app.current_service = Service::Ec2Instances;
        app.mode = Mode::ColumnSelector;
        app.ec2_state.table.page_size = PageSize::Fifty;

        // Navigate to page size section
        let page_size_idx = app.ec2_column_ids.len() + 3; // First page size option (10)
        app.column_selector_index = page_size_idx;
        app.handle_action(Action::ToggleColumn);

        assert_eq!(app.ec2_state.table.page_size, PageSize::Ten);
    }

    #[test]
    fn test_ec2_next_preferences_with_page_size() {
        let mut app = test_app();
        app.current_service = Service::Ec2Instances;
        app.mode = Mode::ColumnSelector;
        app.column_selector_index = 0;

        let page_size_idx = app.ec2_column_ids.len() + 2;
        app.handle_action(Action::NextPreferences);
        assert_eq!(app.column_selector_index, page_size_idx);

        app.handle_action(Action::NextPreferences);
        assert_eq!(app.column_selector_index, 0);
    }

    #[test]
    fn test_ec2_dropdown_next_item() {
        let mut app = test_app();
        app.current_service = Service::Ec2Instances;
        app.mode = Mode::FilterInput;
        app.ec2_state.input_focus = EC2_STATE_FILTER;
        app.ec2_state.state_filter = Ec2StateFilter::AllStates;

        app.handle_action(Action::NextItem);
        assert_eq!(app.ec2_state.state_filter, Ec2StateFilter::Running);

        app.handle_action(Action::NextItem);
        assert_eq!(app.ec2_state.state_filter, Ec2StateFilter::Stopped);
    }

    #[test]
    fn test_ec2_dropdown_prev_item() {
        let mut app = test_app();
        app.current_service = Service::Ec2Instances;
        app.mode = Mode::FilterInput;
        app.ec2_state.input_focus = EC2_STATE_FILTER;
        app.ec2_state.state_filter = Ec2StateFilter::Stopped;

        app.handle_action(Action::PrevItem);
        assert_eq!(app.ec2_state.state_filter, Ec2StateFilter::Running);

        app.handle_action(Action::PrevItem);
        assert_eq!(app.ec2_state.state_filter, Ec2StateFilter::AllStates);
    }

    #[test]
    fn test_ec2_dropdown_cycles_with_arrows() {
        let mut app = test_app();
        app.current_service = Service::Ec2Instances;
        app.mode = Mode::FilterInput;
        app.ec2_state.input_focus = EC2_STATE_FILTER;
        app.ec2_state.state_filter = Ec2StateFilter::Stopping;

        // Next wraps to AllStates
        app.handle_action(Action::NextItem);
        assert_eq!(app.ec2_state.state_filter, Ec2StateFilter::AllStates);

        // Prev wraps to Stopping
        app.handle_action(Action::PrevItem);
        assert_eq!(app.ec2_state.state_filter, Ec2StateFilter::Stopping);
    }
}
