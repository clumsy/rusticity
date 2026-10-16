//! View-scoped resolver tests: every assertion is about the OPERATION and the
//! JSON COLLECTION SCOPE the current view resolves to — never a highlighted
//! child row — plus the empty-selection, active-filter and out-of-bounds edge
//! cases. No command is executed and no AWS client is created.
use super::*;
use crate::app::{App, Service, ViewMode};
use crate::ui::cw::alarms::{AlarmDetailTab, AlarmTab};
use crate::ui::cw::logs::DetailTab as LogsDetailTab;
use crate::ui::dynamodb::{DetailTab as DdbDetailTab, ItemOperation, ItemProjection};
use crate::ui::iam::{GroupTab, RoleTab, UserTab};
use crate::ui::lambda::DetailTab as LambdaDetailTab;
use rusticity_core::dynamodb::{DdbIndex, DdbTable};
use rusticity_core::{LogGroup, LogStream};

fn app(service: Service) -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    app.current_service = service;
    app.service_selected = true;
    app.config.account_id = "123456789012".into();
    app
}

/// Resolve and require a `Target::View`, returning its parts for assertions.
/// `Target` is not `Debug`, so the failure arms describe the shape in words.
fn resolved(app: &App) -> (&'static str, Value, Option<String>, Option<Resource>) {
    match target(app) {
        Ok(Some(Target::View {
            operation,
            input,
            region,
            scope,
        })) => (operation, input, region, scope),
        Ok(Some(_)) => panic!("expected a Target::View, got a different Target variant"),
        Ok(None) => panic!("expected a Target::View, got None (view not owned)"),
        Err(e) => panic!("expected a Target::View, got Err: {e}"),
    }
}

/// True when the view is explicitly not owned here (`Ok(None)`).
fn is_unowned(app: &App) -> bool {
    matches!(target(app), Ok(None))
}

fn log_group(name: &str) -> LogGroup {
    LogGroup {
        name: name.into(),
        creation_time: None,
        stored_bytes: None,
        retention_days: None,
        log_class: None,
        arn: Some(format!(
            "arn:aws:logs:us-east-1:123456789012:log-group:{name}:*"
        )),
        log_group_arn: None,
        deletion_protection_enabled: None,
    }
}

fn log_stream(name: &str) -> LogStream {
    LogStream {
        name: name.into(),
        creation_time: None,
        last_event_time: None,
    }
}

fn ddb_table(name: &str) -> DdbTable {
    DdbTable {
        name: name.into(),
        table_arn: format!("arn:aws:dynamodb:us-east-1:123456789012:table/{name}"),
        partition_key: "pk (S)".into(),
        sort_key: "sk (N)".into(),
        ..Default::default()
    }
}

// ── CloudWatch Logs ──────────────────────────────────────────────────────────

#[test]
fn logs_list_view_is_describe_log_groups_unscoped() {
    let mut app = self::app(Service::CloudWatchLogGroups);
    app.view_mode = ViewMode::List;
    let (op, input, region, scope) = resolved(&app);
    assert_eq!(op, "logs describe-log-groups");
    assert_eq!(input, json!({}));
    assert_eq!(region, None);
    assert!(scope.is_none());
}

#[test]
fn logs_streams_table_describes_streams_of_entered_group_not_selected_stream() {
    let mut app = self::app(Service::CloudWatchLogGroups);
    app.view_mode = ViewMode::Detail;
    app.log_groups_state.detail_tab = LogsDetailTab::LogStreams;
    app.log_groups_state.log_groups.items = vec![log_group("/aws/lambda/a"), log_group("entered")];
    app.log_groups_state.log_groups.selected = 1;
    // A highlighted stream must NOT leak into the command.
    app.log_groups_state.log_streams = vec![log_stream("ignored-stream")];
    app.log_groups_state.selected_stream = 0;
    let (op, input, _, scope) = resolved(&app);
    assert_eq!(op, "logs describe-log-streams");
    assert_eq!(input["logGroupName"], json!("entered"));
    assert!(!input.to_string().contains("ignored-stream"));
    assert_eq!(scope.unwrap().kind, ResourceKind::LogGroup);
}

#[test]
fn logs_events_table_uses_current_stream_and_honors_time_and_filter() {
    let mut app = self::app(Service::CloudWatchLogGroups);
    app.view_mode = ViewMode::Events;
    app.log_groups_state.log_groups.items = vec![log_group("grp")];
    app.log_groups_state.log_groups.selected = 0;
    app.log_groups_state.log_streams = vec![log_stream("stream-a"), log_stream("stream-b")];
    app.log_groups_state.selected_stream = 1;
    app.log_groups_state.relative_amount = "1".into();
    // No filter → get-log-events for the current stream, bounded by the range.
    let (op, input, _, scope) = resolved(&app);
    assert_eq!(op, "logs get-log-events");
    assert_eq!(input["logGroupName"], json!("grp"));
    assert_eq!(input["logStreamName"], json!("stream-b"));
    assert!(input.get("startTime").is_some());
    assert!(input.get("endTime").is_some());
    assert_eq!(scope.unwrap().id, "grp");

    // The text filter is client-side, not CloudWatch filter-pattern syntax.
    app.log_groups_state.event_filter = "ERROR".into();
    let (op, input, _, _) = resolved(&app);
    assert_eq!(op, "logs get-log-events");
    assert_eq!(input["logStreamName"], json!("stream-b"));
    assert!(input.get("filterPattern").is_none());
}

#[test]
fn logs_events_without_a_stream_is_an_explicit_error() {
    let mut app = self::app(Service::CloudWatchLogGroups);
    app.view_mode = ViewMode::Events;
    app.log_groups_state.log_groups.items = vec![log_group("grp")];
    // selected_stream points past an empty stream list.
    app.log_groups_state.log_streams = vec![];
    app.log_groups_state.selected_stream = 3;
    assert!(target(&app).is_err());
}

#[test]
fn logs_tags_table_lists_tags_for_entered_group() {
    let mut app = self::app(Service::CloudWatchLogGroups);
    app.view_mode = ViewMode::Detail;
    app.log_groups_state.detail_tab = LogsDetailTab::Tags;
    app.log_groups_state.log_groups.items = vec![log_group("grp")];
    let (op, input, _, _) = resolved(&app);
    assert_eq!(op, "logs list-tags-for-resource");
    assert!(input["resourceArn"]
        .as_str()
        .unwrap()
        .contains(":log-group:grp"));
}

// ── CloudWatch Logs Insights ───────────────────────────────────────────────

#[test]
fn insights_never_emits_start_query_and_refuses() {
    let app = app(Service::CloudWatchInsights);
    let err = target(&app).unwrap_err();
    assert!(err.contains("StartQuery"));
    // Belt and suspenders: the refusal must not smuggle a runnable op name.
    assert!(!err.contains("get-query-results"));
}

// ── CloudWatch Alarms ────────────────────────────────────────────────────────

#[test]
fn alarms_all_and_in_alarm_tabs_scope_describe_alarms_server_side() {
    let mut app = self::app(Service::CloudWatchAlarms);
    app.alarms_state.alarm_tab = AlarmTab::AllAlarms;
    let (op, input, region, scope) = resolved(&app);
    assert_eq!(op, "cloudwatch describe-alarms");
    assert_eq!(input, json!({}));
    assert_eq!(region, None);
    assert!(scope.is_none());

    app.alarms_state.alarm_tab = AlarmTab::InAlarm;
    let (_, input, _, _) = resolved(&app);
    assert_eq!(input["StateValue"], json!("ALARM"));
}

#[test]
fn alarms_detail_history_is_scoped_other_detail_tabs_are_not_owned() {
    let mut app = self::app(Service::CloudWatchAlarms);
    app.alarms_state.current_alarm = Some("my-alarm".into());
    app.alarms_state.detail_tab = AlarmDetailTab::History;
    let (op, input, _, _) = resolved(&app);
    assert_eq!(op, "cloudwatch describe-alarm-history");
    assert_eq!(input["AlarmName"], json!("my-alarm"));

    app.alarms_state.detail_tab = AlarmDetailTab::Details;
    assert!(is_unowned(&app));
}

// ── CloudTrail ───────────────────────────────────────────────────────────────

#[test]
fn cloudtrail_table_lookup_events_applies_active_filter_only() {
    let mut app = self::app(Service::CloudTrailEvents);
    // No filter applied → bare lookup-events, no selected row.
    let (op, input, region, scope) = resolved(&app);
    assert_eq!(op, "cloudtrail lookup-events");
    assert_eq!(input, json!({}));
    assert_eq!(region, None);
    assert!(scope.is_none());

    app.cloudtrail_state.active_event_name_filter = Some("RunInstances".into());
    let (_, input, _, _) = resolved(&app);
    assert_eq!(
        input["LookupAttributes"],
        json!([{"AttributeKey": "EventName", "AttributeValue": "RunInstances"}])
    );
}

#[test]
fn cloudtrail_entered_event_detail_has_no_collection_command() {
    let mut app = self::app(Service::CloudTrailEvents);
    app.cloudtrail_state.current_event = Some(crate::cloudtrail::events::CloudTrailEvent {
        event_time: String::new(),
        event_name: "RunInstances".into(),
        username: String::new(),
        event_source: String::new(),
        resource_type: String::new(),
        resource_name: String::new(),
        read_only: String::new(),
        aws_region: String::new(),
        event_id: String::new(),
        access_key_id: String::new(),
        source_ip_address: String::new(),
        error_code: String::new(),
        request_id: String::new(),
        event_type: String::new(),
        cloud_trail_event_json: String::new(),
    });
    assert!(is_unowned(&app));
}

// ── DynamoDB ─────────────────────────────────────────────────────────────────

#[test]
fn dynamodb_top_inventory_is_list_tables() {
    let app = app(Service::DynamoDbTables);
    let (op, input, _, scope) = resolved(&app);
    assert_eq!(op, "dynamodb list-tables");
    assert_eq!(input, json!({}));
    assert!(scope.is_none());
}

#[test]
fn dynamodb_items_scan_mirrors_index_and_projection_never_blind() {
    let mut app = self::app(Service::DynamoDbTables);
    let mut table = ddb_table("orders");
    table.global_secondary_indexes = vec![DdbIndex {
        name: "by-customer".into(),
        partition_key: "customer_id (S)".into(),
        ..Default::default()
    }];
    app.dynamodb_state.tables.items = vec![table];
    app.dynamodb_state.current_table = Some("orders".into());
    app.dynamodb_state.detail_tab = DdbDetailTab::Items;
    app.dynamodb_state.item_operation = ItemOperation::Scan;
    // Base table, all attributes → no projection, no index.
    let (op, input, _, scope) = resolved(&app);
    assert_eq!(op, "dynamodb scan");
    assert_eq!(input["TableName"], json!("orders"));
    assert_eq!(input["Limit"], json!(100));
    assert!(input.get("IndexName").is_none());
    assert!(input.get("ProjectionExpression").is_none());
    assert_eq!(scope.unwrap().kind, ResourceKind::Table);

    // Index 1 + Keys only → IndexName and a key-only projection.
    app.dynamodb_state.item_index = 1;
    app.dynamodb_state.item_projection = ItemProjection::KeysOnly;
    let (_, input, _, _) = resolved(&app);
    assert_eq!(input["IndexName"], json!("by-customer"));
    assert_eq!(input["ProjectionExpression"], json!("#p0"));
    assert_eq!(
        input["ExpressionAttributeNames"],
        json!({"#p0": "customer_id"})
    );
}

#[test]
fn dynamodb_items_query_requires_a_partition_value_and_types_it() {
    let mut app = self::app(Service::DynamoDbTables);
    app.dynamodb_state.tables.items = vec![ddb_table("orders")];
    app.dynamodb_state.current_table = Some("orders".into());
    app.dynamodb_state.detail_tab = DdbDetailTab::Items;
    app.dynamodb_state.item_operation = ItemOperation::Query;
    // No partition value entered → explicit error, never an unconditional scan.
    app.dynamodb_state.item_filter.clear();
    assert!(target(&app).is_err());
    app.dynamodb_state.item_filter = "   ".into();
    let (_, input, _, _) = resolved(&app);
    assert_eq!(input["ExpressionAttributeValues"][":pk"]["S"], "   ");

    app.dynamodb_state.item_filter = "cust-1".into();
    let (op, input, _, _) = resolved(&app);
    assert_eq!(op, "dynamodb query");
    assert_eq!(input["KeyConditionExpression"], json!("#pk = :pk"));
    assert_eq!(input["ExpressionAttributeNames"], json!({"#pk": "pk"}));
    assert_eq!(
        input["ExpressionAttributeValues"],
        json!({":pk": {"S": "cust-1"}})
    );
}

#[test]
fn dynamodb_index_and_replica_tables_describe_the_whole_parent_not_a_row() {
    let mut app = self::app(Service::DynamoDbTables);
    app.dynamodb_state.tables.items = vec![ddb_table("orders")];
    app.dynamodb_state.current_table = Some("orders".into());
    for tab in [DdbDetailTab::Indexes, DdbDetailTab::GlobalTables] {
        app.dynamodb_state.detail_tab = tab;
        let (op, input, _, scope) = resolved(&app);
        assert_eq!(op, "dynamodb describe-table");
        assert_eq!(input, json!({"TableName": "orders"}));
        assert_eq!(scope.unwrap().id, "orders");
    }
}

#[test]
fn dynamodb_tags_and_backups_are_scoped_lists() {
    let mut app = self::app(Service::DynamoDbTables);
    app.dynamodb_state.tables.items = vec![ddb_table("orders")];
    app.dynamodb_state.current_table = Some("orders".into());

    app.dynamodb_state.detail_tab = DdbDetailTab::Backups;
    let (op, input, _, _) = resolved(&app);
    assert_eq!(op, "dynamodb list-backups");
    assert_eq!(input["TableName"], json!("orders"));

    app.dynamodb_state.detail_tab = DdbDetailTab::Tags;
    let (op, input, _, _) = resolved(&app);
    assert_eq!(op, "dynamodb list-tags-of-resource");
    assert_eq!(
        input["ResourceArn"],
        json!("arn:aws:dynamodb:us-east-1:123456789012:table/orders")
    );
}

// ── Lambda ───────────────────────────────────────────────────────────────────

#[test]
fn lambda_list_versions_and_aliases_scope_to_entered_function() {
    let mut app = self::app(Service::LambdaFunctions);
    // Top inventory first.
    let (op, _, _, _) = resolved(&app);
    assert_eq!(op, "lambda list-functions");

    app.lambda_state.current_function = Some("my-fn".into());
    app.lambda_state.detail_tab = LambdaDetailTab::Versions;
    let (op, input, _, scope) = resolved(&app);
    assert_eq!(op, "lambda list-versions-by-function");
    assert_eq!(input["FunctionName"], json!("my-fn"));
    assert_eq!(scope.unwrap().kind, ResourceKind::Function);

    app.lambda_state.detail_tab = LambdaDetailTab::Aliases;
    let (op, input, _, _) = resolved(&app);
    assert_eq!(op, "lambda list-aliases");
    assert_eq!(input["FunctionName"], json!("my-fn"));

    app.lambda_state.detail_tab = LambdaDetailTab::Configuration;
    assert!(is_unowned(&app));
}

#[test]
fn lambda_applications_refuses_multi_api_aggregation() {
    let app = app(Service::LambdaApplications);
    assert!(target(&app).is_err());
}

// ── CloudFormation ─────────────────────────────────────────────────────────

#[test]
fn cloudformation_inventory_and_nested_tables_scope_to_the_stack() {
    use crate::ui::cfn::DetailTab as CfnDetailTab;
    let mut app = self::app(Service::CloudFormationStacks);
    let (op, _, _, _) = resolved(&app);
    assert_eq!(op, "cloudformation describe-stacks");

    app.cfn_state.current_stack = Some("prod".into());
    for (tab, expected) in [
        (
            CfnDetailTab::Resources,
            "cloudformation describe-stack-resources",
        ),
        (CfnDetailTab::Events, "cloudformation describe-stack-events"),
        (CfnDetailTab::ChangeSets, "cloudformation list-change-sets"),
    ] {
        app.cfn_state.detail_tab = tab;
        let (op, input, _, scope) = resolved(&app);
        assert_eq!(op, expected);
        assert_eq!(input["StackName"], json!("prod"));
        assert_eq!(scope.unwrap().kind, ResourceKind::Stack);
    }
}

#[test]
fn cloudformation_parameters_and_outputs_describe_the_parent_stack() {
    use crate::ui::cfn::DetailTab as CfnDetailTab;
    let mut app = self::app(Service::CloudFormationStacks);
    app.cfn_state.current_stack = Some("prod".into());
    for tab in [CfnDetailTab::Parameters, CfnDetailTab::Outputs] {
        app.cfn_state.detail_tab = tab;
        let (op, input, _, _) = resolved(&app);
        assert_eq!(op, "cloudformation describe-stacks");
        assert_eq!(input, json!({"StackName": "prod"}));
    }
}

// ── IAM ──────────────────────────────────────────────────────────────────────

#[test]
fn iam_inventories_are_global_lists() {
    for (service, op) in [
        (Service::IamUsers, "iam list-users"),
        (Service::IamRoles, "iam list-roles"),
        (Service::IamUserGroups, "iam list-groups"),
    ] {
        let app = app(service);
        let (resolved_op, input, region, scope) = resolved(&app);
        assert_eq!(resolved_op, op);
        assert_eq!(input, json!({}));
        assert_eq!(region, None, "IAM commands take no region");
        assert!(scope.is_none());
    }
}

#[test]
fn iam_group_users_tab_is_get_group_permissions_refuse() {
    let mut app = self::app(Service::IamUserGroups);
    app.iam_state.current_group = Some("admins".into());
    app.iam_state.group_tab = GroupTab::Users;
    let (op, input, region, _) = resolved(&app);
    assert_eq!(op, "iam get-group");
    assert_eq!(input["GroupName"], json!("admins"));
    assert_eq!(region, None);

    app.iam_state.group_tab = GroupTab::Permissions;
    assert!(target(&app).is_err());
}

#[test]
fn iam_user_and_role_permissions_refuse_but_tags_resolve() {
    let mut app = self::app(Service::IamUsers);
    app.iam_state.current_user = Some("alice".into());
    app.iam_state.user_tab = UserTab::Permissions;
    assert!(target(&app).is_err());
    app.iam_state.user_tab = UserTab::Tags;
    let (op, input, _, _) = resolved(&app);
    assert_eq!(op, "iam list-user-tags");
    assert_eq!(input["UserName"], json!("alice"));

    let mut app = self::app(Service::IamRoles);
    app.iam_state.current_role = Some("svc".into());
    app.iam_state.role_tab = RoleTab::Permissions;
    assert!(target(&app).is_err());
    app.iam_state.role_tab = RoleTab::Tags;
    let (op, input, _, _) = resolved(&app);
    assert_eq!(op, "iam list-role-tags");
    assert_eq!(input["RoleName"], json!("svc"));
}

// ── API Gateway ──────────────────────────────────────────────────────────────

#[test]
fn apigateway_combined_inventory_is_unsupported() {
    let app = app(Service::ApiGatewayApis);
    assert!(target(&app).is_err());
}

#[test]
fn apigateway_entered_api_lists_whole_collection_by_protocol() {
    let mut app = self::app(Service::ApiGatewayApis);
    app.apig_state.current_api = Some(crate::apig::api::RestApi {
        id: "rest-1".into(),
        name: "svc".into(),
        protocol_type: "REST".into(),
        description: String::new(),
        created_date: String::new(),
        api_key_source: String::new(),
        endpoint_configuration: String::new(),
        disable_execute_api_endpoint: false,
        status: String::new(),
    });
    let (op, input, _, _) = resolved(&app);
    assert_eq!(op, "apigateway get-resources");
    assert_eq!(input["restApiId"], json!("rest-1"));

    app.apig_state.current_api.as_mut().unwrap().protocol_type = "HTTP".into();
    app.apig_state.current_api.as_mut().unwrap().id = "http-1".into();
    let (op, input, _, _) = resolved(&app);
    assert_eq!(op, "apigatewayv2 get-routes");
    assert_eq!(input["ApiId"], json!("http-1"));
}
