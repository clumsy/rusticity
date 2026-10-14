//! Remaining service panels expressed as data; no row-index arithmetic.
use super::{PreferenceAction, PreferenceColumn, Preferences, PAGE_SIZE_OPTIONS};
use crate::app::{AlarmViewMode, App, CloudTrailDetailFocus, Service, ViewMode};
use crate::common::{ColumnId, PageSize};
use crate::ui::table::Column as TableColumn;
use crate::ui::{apig, cfn, iam, lambda, sqs};
use std::borrow::Cow;

#[derive(Clone, Copy)]
pub(super) enum Panel {
    Buckets,
    Alarms,
    Trail,
    TrailResources,
    LogGroups,
    LogStreams,
    LogEvents,
    LogTags,
    Apis,
    ApiRoutes,
    ApiResources,
    Ec2,
    Queues,
    Triggers,
    Subscriptions,
    Pipes,
    QueueTags,
    Functions,
    Versions,
    Aliases,
    Layers,
    Applications,
    AppResources,
    Deployments,
    Stacks,
    Parameters,
    Outputs,
    Resources,
    StackEvents,
    Changes,
    Users,
    Roles,
    Groups,
    Policies,
    StackTags,
    Memberships,
    LastAccessed,
    Empty,
}

impl Panel {
    pub(super) fn for_service(app: &App, service: Service) -> Self {
        match service {
            Service::S3Buckets if app.s3_state.current_bucket.is_none() => Self::Buckets,
            Service::CloudWatchAlarms => Self::Alarms,
            Service::CloudTrailEvents => {
                if app.cloudtrail_state.current_event.is_some()
                    && app.cloudtrail_state.detail_focus == CloudTrailDetailFocus::Resources
                {
                    Self::TrailResources
                } else {
                    Self::Trail
                }
            }
            Service::CloudWatchLogGroups => match app.view_mode {
                ViewMode::Events => Self::LogEvents,
                ViewMode::Detail => {
                    if app.log_groups_state.detail_tab == crate::app::DetailTab::Tags {
                        Self::LogTags
                    } else {
                        Self::LogStreams
                    }
                }
                _ => Self::LogGroups,
            },
            Service::ApiGatewayApis => {
                if let Some(api) = &app.apig_state.current_api {
                    if app.apig_state.detail_tab != apig::ApiDetailTab::Routes {
                        Self::Empty
                    } else if api.protocol_type.eq_ignore_ascii_case("REST") {
                        Self::ApiResources
                    } else {
                        Self::ApiRoutes
                    }
                } else {
                    Self::Apis
                }
            }
            Service::Ec2Instances => Self::Ec2,
            Service::SqsQueues => {
                if app.sqs_state.current_queue.is_some() {
                    match app.sqs_state.detail_tab {
                        sqs::QueueDetailTab::LambdaTriggers => Self::Triggers,
                        sqs::QueueDetailTab::SnsSubscriptions => Self::Subscriptions,
                        sqs::QueueDetailTab::EventBridgePipes => Self::Pipes,
                        sqs::QueueDetailTab::Tagging => Self::QueueTags,
                        _ => Self::Empty,
                    }
                } else {
                    Self::Queues
                }
            }
            Service::LambdaFunctions => {
                if app.lambda_state.current_version.is_some()
                    && app.lambda_state.detail_tab == lambda::DetailTab::Configuration
                {
                    Self::Aliases
                } else if app.lambda_state.current_function.is_some() {
                    match app.lambda_state.detail_tab {
                        lambda::DetailTab::Code
                            if app.lambda_state.table.items.iter().any(|function| {
                                Some(&function.name) == app.lambda_state.current_function.as_ref()
                                    && function.package_type == "Zip"
                            }) =>
                        {
                            Self::Layers
                        }
                        lambda::DetailTab::Code => Self::Empty,
                        lambda::DetailTab::Versions => Self::Versions,
                        lambda::DetailTab::Aliases => Self::Aliases,
                        _ => Self::Functions,
                    }
                } else {
                    Self::Functions
                }
            }
            Service::LambdaApplications => {
                if app.lambda_application_state.current_application.is_some() {
                    if app.lambda_application_state.detail_tab
                        == lambda::ApplicationDetailTab::Overview
                    {
                        Self::AppResources
                    } else {
                        Self::Deployments
                    }
                } else {
                    Self::Applications
                }
            }
            Service::CloudFormationStacks => {
                if app.cfn_state.current_stack.is_some() {
                    match app.cfn_state.detail_tab {
                        cfn::DetailTab::StackInfo => Self::StackTags,
                        cfn::DetailTab::Parameters => Self::Parameters,
                        cfn::DetailTab::Outputs => Self::Outputs,
                        cfn::DetailTab::Resources => Self::Resources,
                        cfn::DetailTab::Events => Self::StackEvents,
                        cfn::DetailTab::ChangeSets => Self::Changes,
                        _ => Self::Empty,
                    }
                } else {
                    Self::Stacks
                }
            }
            Service::IamUsers => {
                if app.iam_state.current_user.is_some() {
                    match app.iam_state.user_tab {
                        iam::UserTab::Permissions => Self::Policies,
                        iam::UserTab::Groups => Self::Memberships,
                        iam::UserTab::LastAccessed => Self::LastAccessed,
                        _ => Self::Empty,
                    }
                } else {
                    Self::Users
                }
            }
            Service::IamRoles => {
                if app.iam_state.current_role.is_some() {
                    match app.iam_state.role_tab {
                        iam::RoleTab::Permissions => Self::Policies,
                        iam::RoleTab::LastAccessed => Self::LastAccessed,
                        _ => Self::Empty,
                    }
                } else {
                    Self::Roles
                }
            }
            Service::IamUserGroups => Self::Groups,
            _ => Self::Empty,
        }
    }

    pub(super) fn model(self, app: &App) -> Preferences {
        let columns = match self {
            Self::Buckets => columns(
                &app.s3_bucket_column_ids,
                &app.s3_bucket_visible_column_ids,
                |id| crate::s3::BucketColumn::from_id(id).map(|column| column.name().to_string()),
                false,
            ),
            Self::Alarms => columns(
                &app.cw_alarm_column_ids,
                &app.cw_alarm_visible_column_ids,
                |id| {
                    crate::cw::alarms::AlarmColumn::from_id(id)
                        .map(|column| column.name().to_string())
                },
                false,
            ),
            Self::Trail => columns(
                &app.cloudtrail_event_column_ids,
                &app.cloudtrail_event_visible_column_ids,
                |id| {
                    crate::cloudtrail::CloudTrailEventColumn::from_id(id)
                        .map(|column| column.name().to_string())
                },
                false,
            ),
            Self::TrailResources => columns(
                &app.cloudtrail_resource_column_ids,
                &app.cloudtrail_resource_visible_column_ids,
                |id| {
                    crate::cloudtrail::EventResourceColumn::from_id(id)
                        .map(|column| column.name().to_string())
                },
                false,
            ),
            Self::LogGroups => columns(
                &app.cw_log_group_column_ids,
                &app.cw_log_group_visible_column_ids,
                |id| {
                    crate::cw::logs::LogGroupColumn::from_id(id)
                        .map(|column| column.name().to_string())
                },
                false,
            ),
            Self::LogStreams => columns(
                &app.cw_log_stream_column_ids,
                &app.cw_log_stream_visible_column_ids,
                |id| {
                    crate::cw::logs::StreamColumn::from_id(id)
                        .map(|column| column.name().to_string())
                },
                false,
            ),
            Self::LogEvents => columns(
                &app.cw_log_event_column_ids,
                &app.cw_log_event_visible_column_ids,
                |id| {
                    crate::cw::logs::EventColumn::from_id(id)
                        .map(|column| column.name().to_string())
                },
                false,
            ),
            Self::LogTags => columns(
                &app.cw_log_tag_column_ids,
                &app.cw_log_tag_visible_column_ids,
                |id| crate::cw::TagColumn::from_id(id).map(|column| column.name().to_string()),
                false,
            ),
            Self::Apis => columns(
                &app.apig_api_column_ids,
                &app.apig_api_visible_column_ids,
                |id| crate::apig::api::Column::from_id(id).map(|column| column.name().to_string()),
                false,
            ),
            Self::ApiRoutes => columns(
                &app.apig_route_column_ids,
                &app.apig_route_visible_column_ids,
                |id| {
                    crate::apig::route::Column::from_id(id).map(|column| column.name().to_string())
                },
                true,
            ),
            Self::ApiResources => columns(
                &app.apig_resource_column_ids,
                &app.apig_resource_visible_column_ids,
                |id| {
                    crate::apig::resource::Column::from_id(id)
                        .map(|column| column.name().to_string())
                },
                true,
            ),
            Self::Ec2 => columns(
                &app.ec2_column_ids,
                &app.ec2_visible_column_ids,
                |id| crate::ec2::Column::from_id(id).map(|column| column.name().to_string()),
                false,
            ),
            Self::Queues => columns(
                &app.sqs_column_ids,
                &app.sqs_visible_column_ids,
                |id| crate::sqs::queue::Column::from_id(id).map(|column| column.name().to_string()),
                false,
            ),
            Self::Triggers => columns(
                &app.sqs_state.trigger_column_ids,
                &app.sqs_state.trigger_visible_column_ids,
                |id| {
                    crate::sqs::trigger::Column::from_id(id).map(|column| column.name().to_string())
                },
                false,
            ),
            Self::Subscriptions => columns(
                &app.sqs_state.subscription_column_ids,
                &app.sqs_state.subscription_visible_column_ids,
                |id| crate::sqs::sub::Column::from_id(id).map(|column| column.name().to_string()),
                false,
            ),
            Self::Pipes => columns(
                &app.sqs_state.pipe_column_ids,
                &app.sqs_state.pipe_visible_column_ids,
                |id| crate::sqs::pipe::Column::from_id(id).map(|column| column.name().to_string()),
                false,
            ),
            Self::QueueTags => columns(
                &app.sqs_state.tag_column_ids,
                &app.sqs_state.tag_visible_column_ids,
                |id| crate::sqs::tag::Column::from_id(id).map(|column| column.name().to_string()),
                false,
            ),
            Self::Functions => columns(
                &app.lambda_state.function_column_ids,
                &app.lambda_state.function_visible_column_ids,
                |id| {
                    crate::lambda::FunctionColumn::from_id(id)
                        .map(|column| column.name().to_string())
                },
                false,
            ),
            Self::Versions => columns(
                &app.lambda_state.version_column_ids,
                &app.lambda_state.version_visible_column_ids,
                |id| Some(id.to_string()),
                false,
            ),
            Self::Aliases => columns(
                &app.lambda_state.alias_column_ids,
                &app.lambda_state.alias_visible_column_ids,
                |id| Some(id.to_string()),
                false,
            ),
            Self::Layers => columns(
                &app.lambda_state.layer_column_ids,
                &app.lambda_state.layer_visible_column_ids,
                |id| Some(id.to_string()),
                false,
            ),
            Self::Applications => columns(
                &app.lambda_application_column_ids,
                &app.lambda_application_visible_column_ids,
                |id| {
                    crate::lambda::ApplicationColumn::from_id(id)
                        .map(|column| column.name().to_string())
                },
                false,
            ),
            Self::AppResources => columns(
                &app.lambda_resource_column_ids,
                &app.lambda_resource_visible_column_ids,
                |id| {
                    crate::lambda::ResourceColumn::from_id(id)
                        .map(|column| column.name().to_string())
                },
                false,
            ),
            Self::Deployments => columns(
                &app.lambda_deployment_column_ids,
                &app.lambda_deployment_visible_column_ids,
                |id| {
                    crate::lambda::DeploymentColumn::from_id(id)
                        .map(|column| column.name().to_string())
                },
                false,
            ),
            Self::Stacks => columns(
                &app.cfn_column_ids,
                &app.cfn_visible_column_ids,
                |id| crate::cfn::Column::from_id(id).map(|column| column.name().to_string()),
                false,
            ),
            Self::Parameters => columns(
                &app.cfn_parameter_column_ids,
                &app.cfn_parameter_visible_column_ids,
                |id| {
                    crate::ui::cfn::ParameterColumn::from_id(id)
                        .map(|column| column.name().to_string())
                },
                false,
            ),
            Self::Outputs => columns(
                &app.cfn_output_column_ids,
                &app.cfn_output_visible_column_ids,
                |id| {
                    crate::ui::cfn::OutputColumn::from_id(id)
                        .map(|column| column.name().to_string())
                },
                false,
            ),
            Self::Resources => columns(
                &app.cfn_resource_column_ids,
                &app.cfn_resource_visible_column_ids,
                |id| {
                    crate::ui::cfn::ResourceColumn::from_id(id)
                        .map(|column| column.name().to_string())
                },
                false,
            ),
            Self::StackEvents => columns(
                &app.cfn_event_column_ids,
                &app.cfn_event_visible_column_ids,
                |id| {
                    crate::ui::cfn::EventColumn::from_id(id).map(|column| column.name().to_string())
                },
                false,
            ),
            Self::Changes => columns(
                &app.cfn_change_set_column_ids,
                &app.cfn_change_set_visible_column_ids,
                |id| {
                    crate::ui::cfn::ChangeSetColumn::from_id(id)
                        .map(|column| column.name().to_string())
                },
                false,
            ),
            Self::Users => columns(
                &app.iam_user_column_ids,
                &app.iam_user_visible_column_ids,
                |id| crate::iam::UserColumn::from_id(id).map(|column| column.name().to_string()),
                false,
            ),
            Self::Roles => columns(
                &app.iam_role_column_ids,
                &app.iam_role_visible_column_ids,
                |id| crate::iam::RoleColumn::from_id(id).map(|column| column.name().to_string()),
                false,
            ),
            Self::Groups => columns(
                &app.iam_group_column_ids,
                &app.iam_group_visible_column_ids,
                |id| Some(id.to_string()),
                false,
            ),
            Self::Policies => columns(
                &app.iam_policy_column_ids,
                &app.iam_policy_visible_column_ids,
                |id| Some(id.to_string()),
                false,
            ),
            Self::StackTags => fixed(&["Key", "Value"]),
            Self::Memberships => fixed(&["Group name", "Attached policies"]),
            Self::LastAccessed => fixed(&["Service", "Policies granting", "Last accessed"]),
            Self::Empty => Vec::new(),
        };
        let mut model =
            Preferences::columns_only(columns, !matches!(self, Self::LogEvents | Self::Empty));
        if matches!(self, Self::Alarms) {
            model = model.radio_group(
                "alarm-view",
                "View as",
                &[("table", "Table"), ("cards", "Cards")],
                if app.alarms_state.view_as == AlarmViewMode::Table {
                    "table"
                } else {
                    "cards"
                },
            );
        }
        if self.has_pages() {
            let sizes = if self.small_pages() {
                &PAGE_SIZE_OPTIONS[..3]
            } else {
                PAGE_SIZE_OPTIONS
            };
            model = model.with_page_sizes(self.page_size(app), sizes);
        }
        if matches!(self, Self::Alarms) {
            model = model.toggle(
                "alarm-wrap",
                "Wrap lines",
                "Wrap lines",
                app.alarms_state.wrap_lines,
            );
        }
        model
    }

    // Layers render as an unpaginated list (Lambda permits at most five).
    // Do not offer a page-size control that actually changes the parent inventory.
    fn has_pages(self) -> bool {
        !matches!(
            self,
            Self::Empty
                | Self::TrailResources
                | Self::LogEvents
                | Self::ApiRoutes
                | Self::ApiResources
                | Self::Layers
        )
    }
    fn small_pages(self) -> bool {
        matches!(
            self,
            Self::Applications
                | Self::AppResources
                | Self::Deployments
                | Self::Users
                | Self::Roles
                | Self::Groups
                | Self::Policies
                | Self::Memberships
                | Self::LastAccessed
        )
    }
    fn page_size(self, app: &App) -> Option<PageSize> {
        match self {
            Self::Buckets => Some(app.s3_state.buckets.page_size),
            Self::Alarms => Some(app.alarms_state.table.page_size),
            Self::Trail => Some(app.cloudtrail_state.table.page_size),
            Self::LogGroups => Some(app.log_groups_state.log_groups.page_size),
            Self::LogTags => Some(app.log_groups_state.tags.page_size),
            Self::Apis => Some(app.apig_state.apis.page_size),
            Self::Ec2 => Some(app.ec2_state.table.page_size),
            Self::Queues => Some(app.sqs_state.queues.page_size),
            Self::Triggers => Some(app.sqs_state.triggers.page_size),
            Self::Subscriptions => Some(app.sqs_state.subscriptions.page_size),
            Self::Pipes => Some(app.sqs_state.pipes.page_size),
            Self::QueueTags => Some(app.sqs_state.tags.page_size),
            Self::Functions => Some(app.lambda_state.table.page_size),
            Self::Versions => Some(app.lambda_state.version_table.page_size),
            Self::Aliases => Some(app.lambda_state.alias_table.page_size),
            Self::Applications => Some(app.lambda_application_state.table.page_size),
            Self::AppResources => Some(app.lambda_application_state.resources.page_size),
            Self::Deployments => Some(app.lambda_application_state.deployments.page_size),
            Self::Stacks => Some(app.cfn_state.table.page_size),
            Self::Parameters => Some(app.cfn_state.parameters.page_size),
            Self::Outputs => Some(app.cfn_state.outputs.page_size),
            Self::Resources => Some(app.cfn_state.resources.page_size),
            Self::StackEvents => Some(app.cfn_state.events.page_size),
            Self::Changes => Some(app.cfn_state.change_sets.page_size),
            Self::Users => Some(app.iam_state.users.page_size),
            Self::Roles => Some(app.iam_state.roles.page_size),
            Self::Groups => Some(app.iam_state.groups.page_size),
            Self::Policies => Some(app.iam_state.policies.page_size),
            Self::StackTags => Some(app.cfn_state.tags.page_size),
            Self::Memberships => Some(app.iam_state.user_group_memberships.page_size),
            Self::LastAccessed => Some(app.iam_state.last_accessed_services.page_size),
            Self::LogStreams => PAGE_SIZE_OPTIONS
                .iter()
                .find(|(size, _)| size.value() == app.log_groups_state.stream_page_size)
                .map(|(size, _)| *size),
            _ => None,
        }
    }
    fn set_page_size(self, app: &mut App, size: PageSize) {
        match self {
            Self::Buckets => app.s3_state.buckets.page_size = size,
            Self::Alarms => app.alarms_state.table.page_size = size,
            Self::Trail => {
                app.cloudtrail_state.table.page_size = size;
                app.cloudtrail_state.table.snap_to_page();
            }
            Self::LogGroups => app.log_groups_state.log_groups.page_size = size,
            Self::LogTags => app.log_groups_state.tags.page_size = size,
            Self::Apis => app.apig_state.apis.page_size = size,
            Self::Ec2 => app.ec2_state.table.page_size = size,
            Self::Queues => app.sqs_state.queues.page_size = size,
            Self::Triggers => app.sqs_state.triggers.page_size = size,
            Self::Subscriptions => app.sqs_state.subscriptions.page_size = size,
            Self::Pipes => app.sqs_state.pipes.page_size = size,
            Self::QueueTags => app.sqs_state.tags.page_size = size,
            Self::Functions => app.lambda_state.table.page_size = size,
            Self::Versions => app.lambda_state.version_table.page_size = size,
            Self::Aliases => app.lambda_state.alias_table.page_size = size,
            Self::Applications => app.lambda_application_state.table.page_size = size,
            Self::AppResources => app.lambda_application_state.resources.page_size = size,
            Self::Deployments => app.lambda_application_state.deployments.page_size = size,
            Self::Stacks => app.cfn_state.table.page_size = size,
            Self::Parameters => app.cfn_state.parameters.page_size = size,
            Self::Outputs => app.cfn_state.outputs.page_size = size,
            Self::Resources => app.cfn_state.resources.page_size = size,
            Self::StackEvents => app.cfn_state.events.page_size = size,
            Self::Changes => app.cfn_state.change_sets.page_size = size,
            Self::Users => app.iam_state.users.page_size = size,
            Self::Roles => app.iam_state.roles.page_size = size,
            Self::Groups => app.iam_state.groups.page_size = size,
            Self::Policies => app.iam_state.policies.page_size = size,
            Self::StackTags => app.cfn_state.tags.page_size = size,
            Self::Memberships => app.iam_state.user_group_memberships.page_size = size,
            Self::LastAccessed => app.iam_state.last_accessed_services.page_size = size,
            Self::LogStreams => {
                app.log_groups_state.stream_page_size = size.value();
                app.log_groups_state.selected_stream = 0;
                app.log_groups_state.stream_current_page = 0;
                app.log_groups_state.stream_page = 0;
                app.log_groups_state.expanded_stream = None;
            }
            _ => {}
        }
    }
    fn static_columns(self, app: &mut App) -> Option<&mut Vec<ColumnId>> {
        Some(match self {
            Self::Buckets => &mut app.s3_bucket_visible_column_ids,
            Self::Alarms => &mut app.cw_alarm_visible_column_ids,
            Self::Trail => &mut app.cloudtrail_event_visible_column_ids,
            Self::TrailResources => &mut app.cloudtrail_resource_visible_column_ids,
            Self::LogGroups => &mut app.cw_log_group_visible_column_ids,
            Self::LogStreams => &mut app.cw_log_stream_visible_column_ids,
            Self::LogEvents => &mut app.cw_log_event_visible_column_ids,
            Self::LogTags => &mut app.cw_log_tag_visible_column_ids,
            Self::Apis => &mut app.apig_api_visible_column_ids,
            Self::ApiRoutes => &mut app.apig_route_visible_column_ids,
            Self::ApiResources => &mut app.apig_resource_visible_column_ids,
            Self::Ec2 => &mut app.ec2_visible_column_ids,
            Self::Queues => &mut app.sqs_visible_column_ids,
            Self::Functions => &mut app.lambda_state.function_visible_column_ids,
            Self::Applications => &mut app.lambda_application_visible_column_ids,
            Self::AppResources => &mut app.lambda_resource_visible_column_ids,
            Self::Deployments => &mut app.lambda_deployment_visible_column_ids,
            Self::Stacks => &mut app.cfn_visible_column_ids,
            Self::Parameters => &mut app.cfn_parameter_visible_column_ids,
            Self::Outputs => &mut app.cfn_output_visible_column_ids,
            Self::Resources => &mut app.cfn_resource_visible_column_ids,
            Self::StackEvents => &mut app.cfn_event_visible_column_ids,
            Self::Changes => &mut app.cfn_change_set_visible_column_ids,
            Self::Users => &mut app.iam_user_visible_column_ids,
            Self::Roles => &mut app.iam_role_visible_column_ids,
            _ => return None,
        })
    }
    fn owned_columns(self, app: &mut App) -> Option<&mut Vec<String>> {
        Some(match self {
            Self::Triggers => &mut app.sqs_state.trigger_visible_column_ids,
            Self::Subscriptions => &mut app.sqs_state.subscription_visible_column_ids,
            Self::Pipes => &mut app.sqs_state.pipe_visible_column_ids,
            Self::QueueTags => &mut app.sqs_state.tag_visible_column_ids,
            Self::Versions => &mut app.lambda_state.version_visible_column_ids,
            Self::Aliases => &mut app.lambda_state.alias_visible_column_ids,
            Self::Layers => &mut app.lambda_state.layer_visible_column_ids,
            Self::Groups => &mut app.iam_group_visible_column_ids,
            Self::Policies => &mut app.iam_policy_visible_column_ids,
            _ => return None,
        })
    }
    pub(super) fn activate(self, app: &mut App) {
        match self.model(app).action(app.column_selector_index) {
            Some(PreferenceAction::SetColumnVisibility { id, visible }) => {
                if let Some(columns) = self.owned_columns(app) {
                    columns.retain(|column| column != id.as_ref());
                    if visible {
                        columns.push(id.into_owned());
                    }
                } else if let (Some(columns), Cow::Borrowed(id)) = (self.static_columns(app), id) {
                    columns.retain(|column| *column != id);
                    if visible {
                        columns.push(id);
                    }
                }
            }
            Some(PreferenceAction::SetPageSize(size)) => self.set_page_size(app, size),
            Some(PreferenceAction::SetRadio {
                group: "alarm-view",
                value,
            }) if matches!(self, Self::Alarms) => {
                app.alarms_state.view_as = if value == "cards" {
                    AlarmViewMode::Cards
                } else {
                    AlarmViewMode::Table
                };
            }
            Some(PreferenceAction::SetToggle {
                id: "alarm-wrap",
                enabled,
            }) if matches!(self, Self::Alarms) => app.alarms_state.wrap_lines = enabled,
            _ => {}
        }
    }
}
fn fixed(names: &[&'static str]) -> Vec<PreferenceColumn> {
    names
        .iter()
        .map(|&name| PreferenceColumn {
            id: Cow::Borrowed(name),
            label: name.into(),
            visible: true,
            locked: true,
        })
        .collect()
}

fn columns<T: Clone + PartialEq + AsRef<str> + Into<Cow<'static, str>>>(
    ids: &[T],
    visible: &[T],
    name: impl Fn(&T) -> Option<String>,
    lock_first: bool,
) -> Vec<PreferenceColumn> {
    ids.iter()
        .enumerate()
        .filter_map(|(index, id)| {
            name(id).map(|label| PreferenceColumn {
                id: id.clone().into(),
                label,
                visible: visible.contains(id),
                locked: lock_first && index == 0,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
