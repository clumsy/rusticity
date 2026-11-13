//! Every table that offers the sort chooser, and where each keeps its
//! order. Preferences reads `extend` to draw "Sort by" and calls `choose`
//! when Space is pressed on one of its rows.
//!
//! Two kinds of table: the five lists with a fixed default column (RDS, EC2,
//! VPC, Secrets Manager, ACM) keep a `SortOrder`; every other table keeps a
//! `Choice`, whose "Default order" row is the order it already had. S3
//! buckets (a prefix tree), CloudTrail events (time-ordered pages) and the
//! CloudFormation nested view keep their own structure and are not offered.
use super::{Choice, SortOrder, DEFAULT_VALUE};
use crate::app::{App, Service};
use crate::common::ColumnId;
use crate::preferences::Preferences;

/// The visible columns of a table, in column order, with their labels.
fn columns(
    all: &[ColumnId],
    visible: &[ColumnId],
    label: impl Fn(ColumnId) -> Option<&'static str>,
) -> Vec<(ColumnId, &'static str)> {
    all.iter()
        .filter(|id| visible.contains(id))
        .filter_map(|id| label(id).map(|l| (*id, l)))
        .collect()
}

/// How a table keeps its order.
#[derive(Clone, Copy)]
enum Kind {
    Order(SortOrder, SortOrder),
    Choice(Choice),
}

/// One sortable table on screen.
struct Described {
    columns: Vec<(ColumnId, &'static str)>,
    visible: Vec<ColumnId>,
    kind: Kind,
}

impl Described {
    fn order(
        columns: Vec<(ColumnId, &'static str)>,
        visible: &[ColumnId],
        order: SortOrder,
        default: SortOrder,
    ) -> Self {
        Self {
            columns,
            visible: visible.to_vec(),
            kind: Kind::Order(order, default),
        }
    }

    fn choice(
        columns: Vec<(ColumnId, &'static str)>,
        visible: &[ColumnId],
        choice: Choice,
    ) -> Self {
        Self {
            columns,
            visible: visible.to_vec(),
            kind: Kind::Choice(choice),
        }
    }
}

fn choice_of<T: 'static>(t: &crate::eventbridge::Table<T>) -> Described {
    let all: Vec<ColumnId> = t.spec().iter().map(|(id, ..)| *id).collect();
    Described::choice(columns(&all, &t.visible, Some), &t.visible, t.sort)
}

/// The sortable table on screen for `service`, if any.
fn describe(app: &App, service: Service) -> Option<Described> {
    use crate::eventbridge::{Level, Tab};
    Some(match service {
        Service::RdsInstances if crate::rds::detail::active(app) => {
            let view = app.rds_state.detail.view()?;
            view.slot()?;
            let table = view.active_table();
            let all: Vec<ColumnId> = view.active_columns().iter().map(|(id, _)| *id).collect();
            Described::choice(
                columns(&all, &table.visible_column_ids, Some),
                &table.visible_column_ids,
                table.sort,
            )
        }
        Service::RdsInstances => {
            let s = &app.rds_state;
            use crate::rds::instance::Column;
            Described::order(
                columns(&s.column_ids, &s.visible_column_ids, |id| {
                    Column::from_id(id).map(|c| c.default_name())
                }),
                &s.visible_column_ids,
                s.sort,
                crate::ui::rds::default_sort(),
            )
        }
        Service::VpcVpcs => {
            let s = &app.vpc_state;
            use crate::vpc::columns::Column;
            Described::order(
                columns(&s.column_ids, &s.visible_column_ids, |id| {
                    Column::from_id(id).map(|c| c.default_name())
                }),
                &s.visible_column_ids,
                s.sort,
                crate::ui::vpc::default_sort(),
            )
        }
        Service::SecretsManagerSecrets if !crate::secretsmanager::detail::active(app) => {
            let s = &app.secretsmanager_state;
            use crate::secretsmanager::columns::Column;
            Described::order(
                columns(&s.column_ids, &s.visible_column_ids, |id| {
                    Column::from_id(id).map(|c| c.default_name())
                }),
                &s.visible_column_ids,
                s.sort,
                crate::ui::secretsmanager::default_sort(),
            )
        }
        Service::AcmCertificates => {
            let s = &app.acm_state;
            use crate::acm::columns::Column;
            Described::order(
                columns(&s.column_ids, &s.visible_column_ids, |id| {
                    Column::from_id(id).map(|c| c.default_name())
                }),
                &s.visible_column_ids,
                s.sort,
                crate::ui::acm::default_sort(),
            )
        }
        Service::Ec2Instances if app.ec2_state.current_instance.is_none() => Described::order(
            columns(&app.ec2_column_ids, &app.ec2_visible_column_ids, |id| {
                crate::ec2::Column::from_id(id).map(|c| c.default_name())
            }),
            &app.ec2_visible_column_ids,
            app.ec2_state.sort,
            crate::ui::ec2::default_sort(),
        ),
        Service::EcrRepositories if app.ecr_state.current_repository.is_none() => {
            Described::choice(
                columns(
                    &app.ecr_repo_column_ids,
                    &app.ecr_repo_visible_column_ids,
                    |id| crate::ecr::repo::Column::from_id(id).map(|c| c.default_name()),
                ),
                &app.ecr_repo_visible_column_ids,
                app.ecr_state.sort,
            )
        }
        Service::EfsFileSystems if app.efs_state.current_file_system.is_none() => {
            Described::choice(
                columns(&app.efs_column_ids, &app.efs_visible_column_ids, |id| {
                    crate::efs::fs::Column::from_id(id).map(|c| c.default_name())
                }),
                &app.efs_visible_column_ids,
                app.efs_state.sort,
            )
        }
        Service::FsxFileSystems if app.fsx_state.current_file_system.is_none() => {
            Described::choice(
                columns(&app.fsx_column_ids, &app.fsx_visible_column_ids, |id| {
                    crate::fsx::fs::Column::from_id(id).map(|c| c.default_name())
                }),
                &app.fsx_visible_column_ids,
                app.fsx_state.sort,
            )
        }
        Service::CloudWatchLogGroups if app.view_mode == crate::app::ViewMode::List => {
            Described::choice(
                columns(
                    &app.cw_log_group_column_ids,
                    &app.cw_log_group_visible_column_ids,
                    |id| crate::cw::logs::LogGroupColumn::from_id(id).map(|c| c.default_name()),
                ),
                &app.cw_log_group_visible_column_ids,
                app.log_groups_state.sort,
            )
        }
        Service::ApiGatewayApis if app.apig_state.current_api.is_none() => Described::choice(
            columns(
                &app.apig_api_column_ids,
                &app.apig_api_visible_column_ids,
                |id| crate::apig::api::Column::from_id(id).map(|c| c.default_name()),
            ),
            &app.apig_api_visible_column_ids,
            app.apig_state.sort,
        ),
        Service::CloudFormationStacks
            if app.cfn_state.current_stack.is_none() && !app.cfn_state.view_nested =>
        {
            Described::choice(
                columns(&app.cfn_column_ids, &app.cfn_visible_column_ids, |id| {
                    crate::cfn::Column::from_id(id).map(|c| c.default_name())
                }),
                &app.cfn_visible_column_ids,
                app.cfn_state.sort,
            )
        }
        Service::IamUsers if app.iam_state.current_user.is_none() => Described::choice(
            columns(
                &app.iam_user_column_ids,
                &app.iam_user_visible_column_ids,
                |id| crate::iam::UserColumn::from_id(id).map(|c| c.default_name()),
            ),
            &app.iam_user_visible_column_ids,
            app.iam_state.users_sort,
        ),
        Service::IamRoles if app.iam_state.current_role.is_none() => Described::choice(
            columns(
                &app.iam_role_column_ids,
                &app.iam_role_visible_column_ids,
                |id| crate::iam::RoleColumn::from_id(id).map(|c| c.default_name()),
            ),
            &app.iam_role_visible_column_ids,
            app.iam_state.roles_sort,
        ),
        Service::SqsQueues if app.sqs_state.current_queue.is_none() => Described::choice(
            columns(&app.sqs_column_ids, &app.sqs_visible_column_ids, |id| {
                crate::sqs::queue::Column::from_id(id).map(|c| c.default_name())
            }),
            &app.sqs_visible_column_ids,
            app.sqs_state.sort,
        ),
        Service::LambdaFunctions if app.lambda_state.current_function.is_none() => {
            let s = &app.lambda_state;
            Described::choice(
                columns(
                    &s.function_column_ids,
                    &s.function_visible_column_ids,
                    |id| crate::lambda::FunctionColumn::from_id(id).map(|c| c.default_name()),
                ),
                &s.function_visible_column_ids,
                s.sort,
            )
        }
        Service::LambdaApplications
            if app.lambda_application_state.current_application.is_none() =>
        {
            Described::choice(
                columns(
                    &app.lambda_application_column_ids,
                    &app.lambda_application_visible_column_ids,
                    |id| crate::lambda::ApplicationColumn::from_id(id).map(|c| c.default_name()),
                ),
                &app.lambda_application_visible_column_ids,
                app.lambda_application_state.sort,
            )
        }
        Service::DynamoDbTables if app.dynamodb_state.current_table.is_none() => Described::choice(
            columns(
                &app.dynamodb_column_ids,
                &app.dynamodb_visible_column_ids,
                |id| crate::dynamodb::table::Column::from_id(id).map(|c| c.default_name()),
            ),
            &app.dynamodb_visible_column_ids,
            app.dynamodb_state.sort,
        ),
        Service::KmsKeys if !crate::kms::detail::active(app) => {
            let s = &app.kms_state;
            Described::choice(
                columns(
                    &crate::kms::key::Column::ids(),
                    s.visible_column_ids(),
                    |id| crate::kms::key::Column::from_id(id).map(|c| c.default_name()),
                ),
                s.visible_column_ids(),
                s.sort,
            )
        }
        Service::SnsTopics if app.sns_state.subscriptions.topic.is_none() => {
            let s = &app.sns_state;
            Described::choice(
                columns(&s.column_ids, &s.visible_column_ids, |id| {
                    crate::sns::columns::Column::from_id(id).map(|c| c.default_name())
                }),
                &s.visible_column_ids,
                s.sort,
            )
        }
        Service::StepFunctionsStateMachines
            if app.stepfunctions_state.executions.machine.is_none() =>
        {
            let s = &app.stepfunctions_state;
            Described::choice(
                columns(&s.column_ids, &s.visible_column_ids, |id| {
                    crate::stepfunctions::columns::Column::from_id(id).map(|c| c.default_name())
                }),
                &s.visible_column_ids,
                s.sort,
            )
        }
        Service::EventBridgeEventBuses => {
            let state = &app.eventbridge_state;
            match (crate::eventbridge::level(app), &state.bus) {
                (Level::Buses, _) => choice_of(&state.buses),
                (Level::Rules, Some(bus)) => choice_of(&bus.rules),
                (Level::Rule(Tab::Targets), Some(bus)) => choice_of(&bus.rule.as_ref()?.targets),
                (Level::Rule(Tab::Tags), Some(bus)) => choice_of(&bus.rule.as_ref()?.tags),
                _ => return None,
            }
        }
        _ => return None,
    })
}

/// Add "Sort by" to an enabled table's preferences.
pub(crate) fn extend(app: &App, service: Service, prefs: Preferences) -> Preferences {
    if !prefs.is_enabled() {
        return prefs;
    }
    let Some(table) = describe(app, service) else {
        return prefs;
    };
    if table.columns.is_empty() {
        return prefs;
    }
    match table.kind {
        Kind::Order(order, default) => super::with_sort(
            prefs,
            &table.columns,
            order.effective(default, &table.visible),
        ),
        Kind::Choice(choice) => super::with_choice(prefs, &table.columns, choice, &table.visible),
    }
}

/// Space on a "Sort by" row of the table on screen; shows the first row of
/// the new order. Returns false when `service` has no sortable table here.
pub(crate) fn choose(app: &mut App, service: Service, value: &'static str) -> bool {
    let Some(table) = describe(app, service) else {
        return false;
    };
    let visible = table.visible;
    let toggle_order = |order: &mut SortOrder, default: SortOrder| {
        order.toggle(value, default, &visible);
    };
    let toggle_choice = |choice: &mut Choice| choice.toggle(value, &visible);
    if value == DEFAULT_VALUE && matches!(table.kind, Kind::Order(..)) {
        return false;
    }
    use crate::eventbridge::{Level, Tab};
    let level = crate::eventbridge::level(app);
    match service {
        Service::RdsInstances if crate::rds::detail::active(app) => {
            let t = app.rds_state.detail.view_mut().unwrap().active_table_mut();
            toggle_choice(&mut t.sort);
            t.rows.reset();
        }
        Service::RdsInstances => {
            toggle_order(&mut app.rds_state.sort, crate::ui::rds::default_sort());
            app.rds_state.instances.reset();
        }
        Service::VpcVpcs => {
            toggle_order(&mut app.vpc_state.sort, crate::ui::vpc::default_sort());
            app.vpc_state.vpcs.reset();
        }
        Service::SecretsManagerSecrets => {
            toggle_order(
                &mut app.secretsmanager_state.sort,
                crate::ui::secretsmanager::default_sort(),
            );
            app.secretsmanager_state.secrets.reset();
        }
        Service::AcmCertificates => {
            toggle_order(&mut app.acm_state.sort, crate::ui::acm::default_sort());
            app.acm_state.certificates.reset();
        }
        Service::Ec2Instances => {
            toggle_order(&mut app.ec2_state.sort, crate::ui::ec2::default_sort());
            app.ec2_state.table.reset();
        }
        Service::EcrRepositories => {
            toggle_choice(&mut app.ecr_state.sort);
            app.ecr_state.repositories.reset();
        }
        Service::EfsFileSystems => {
            toggle_choice(&mut app.efs_state.sort);
            app.efs_state.file_systems.reset();
        }
        Service::FsxFileSystems => {
            toggle_choice(&mut app.fsx_state.sort);
            app.fsx_state.file_systems.reset();
        }
        Service::CloudWatchLogGroups => {
            toggle_choice(&mut app.log_groups_state.sort);
            app.log_groups_state.log_groups.reset();
        }
        Service::ApiGatewayApis => {
            toggle_choice(&mut app.apig_state.sort);
            app.apig_state.apis.reset();
        }
        Service::CloudFormationStacks => {
            toggle_choice(&mut app.cfn_state.sort);
            app.cfn_state.table.reset();
        }
        Service::IamUsers => {
            toggle_choice(&mut app.iam_state.users_sort);
            app.iam_state.users.reset();
        }
        Service::IamRoles => {
            toggle_choice(&mut app.iam_state.roles_sort);
            app.iam_state.roles.reset();
        }
        Service::SqsQueues => {
            toggle_choice(&mut app.sqs_state.sort);
            app.sqs_state.queues.reset();
        }
        Service::LambdaFunctions => {
            toggle_choice(&mut app.lambda_state.sort);
            app.lambda_state.table.reset();
        }
        Service::LambdaApplications => {
            toggle_choice(&mut app.lambda_application_state.sort);
            app.lambda_application_state.table.reset();
        }
        Service::DynamoDbTables => {
            toggle_choice(&mut app.dynamodb_state.sort);
            app.dynamodb_state.tables.reset();
        }
        Service::KmsKeys => {
            toggle_choice(&mut app.kms_state.sort);
            app.kms_state.keys.reset();
        }
        Service::SnsTopics => {
            toggle_choice(&mut app.sns_state.sort);
            app.sns_state.topics.reset();
        }
        Service::StepFunctionsStateMachines => {
            toggle_choice(&mut app.stepfunctions_state.sort);
            app.stepfunctions_state.machines.reset();
        }
        Service::EventBridgeEventBuses => {
            let state = &mut app.eventbridge_state;
            let bus = state.bus.as_mut();
            macro_rules! pick {
                ($t:expr) => {{
                    let t = $t;
                    toggle_choice(&mut t.sort);
                    t.rows.reset();
                }};
            }
            match (level, bus) {
                (Level::Buses, _) => pick!(&mut state.buses),
                (Level::Rules, Some(bus)) => pick!(&mut bus.rules),
                (Level::Rule(Tab::Targets), Some(bus)) => {
                    pick!(&mut bus.rule.as_mut().unwrap().targets)
                }
                (Level::Rule(Tab::Tags), Some(bus)) => pick!(&mut bus.rule.as_mut().unwrap().tags),
                _ => return false,
            }
        }
        _ => return false,
    }
    true
}
