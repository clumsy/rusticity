//! View-scoped CLI resolution for the services whose detail views are built
//! from NESTED tables (a table the user ENTERED a parent to reach) rather than
//! a single highlighted row.
//!
//! Every command produced here is the fixed, read-only collection API for the
//! table the user is *currently looking at*, scoped to the parent they entered
//! — never a describe/drilldown for the highlighted child row. This mirrors the
//! user's correction: `Ctrl+Y`/`Ctrl+E` on a table copies the command that
//! *produced that table*, scoped to its entered parent.
//!
//! No AWS requests, subprocesses, credential reads, or automatic execution: the
//! operation names are static string literals and every identifier travels
//! inside the JSON input, exactly as the running backend built its own request.
//!
//! A `None` result means "this view is not one of the nested tables owned here"
//! — the caller falls through to its other resolvers. An `Err` means "this view
//! IS owned, but its command cannot be reproduced safely" (e.g. a Scan that
//! would be billed without the user's key/query, or a tab that fans out across
//! more than one API). We never silently substitute a broader or different
//! collection. No API mutations or automatic executions occur here; explicit
//! read commands can still incur normal AWS read charges.

use crate::app::{App, Service, ViewMode};
use crate::ui::cw::alarms::{AlarmDetailTab, AlarmTab};
use crate::ui::cw::logs::DetailTab as LogsDetailTab;
use crate::ui::dynamodb::{self, DetailTab as DdbDetailTab, ItemOperation, ItemProjection};
use crate::ui::iam::{GroupTab, RoleTab, UserTab};
use crate::ui::lambda::DetailTab as LambdaDetailTab;
use rusticity_core::finder::{Resource, ResourceKind};
use serde_json::{json, Value};

use super::render::Target;

/// Build a `None`-region (current-region) view command with no parent scope.
fn view(operation: &'static str, input: Value) -> Target {
    Target::View {
        operation,
        input,
        region: None,
        scope: None,
    }
}

/// Build a view command scoped to a parent resource. The parent is validated by
/// the formatter via the existing `resource_command` path (account/region/ARN
/// checks), so a foreign-account or cross-region parent is refused there.
fn scoped(operation: &'static str, input: Value, scope: Resource) -> Target {
    Target::View {
        operation,
        input,
        region: None,
        scope: Some(scope),
    }
}

/// The IAM resource-kind view command, which is global (region `None`) and
/// carries no parent scope — IAM list/get commands take no `--region`.
fn global(operation: &'static str, input: Value) -> Target {
    Target::View {
        operation,
        input,
        region: None,
        scope: None,
    }
}

/// Resolve the current view to its collection command, if this view is one of
/// the nested-table views owned by this module.
pub(super) fn target(app: &App) -> Result<Option<Target>, String> {
    match app.current_service {
        Service::CloudWatchLogGroups => logs(app),
        Service::CloudWatchInsights => insights(),
        Service::CloudWatchAlarms => alarms(app),
        Service::CloudTrailEvents => cloudtrail(app),
        Service::DynamoDbTables => dynamodb_view(app),
        Service::LambdaFunctions => lambda_functions(app),
        Service::LambdaApplications => lambda_applications(),
        Service::CloudFormationStacks => cloudformation(app),
        Service::IamUsers => iam_users(app),
        Service::IamRoles => iam_roles(app),
        Service::IamUserGroups => iam_groups(app),
        Service::ApiGatewayApis => apigateway(app),
        _ => Ok(None),
    }
}

// ── CloudWatch Logs ──────────────────────────────────────────────────────────

fn log_group_scope(app: &App) -> Option<Resource> {
    let group = crate::ui::cw::logs::selected_log_group(app)?;
    // The log-group ARN (without the trailing ":*") scopes the parent so the
    // formatter's account check runs; the describe commands themselves address
    // the group by name, which is what the backend sends.
    let arn = group
        .log_group_arn
        .clone()
        .or_else(|| group.arn.clone())
        .map(|arn| arn.trim_end_matches(":*").to_string())
        .unwrap_or_default();
    Some(Resource {
        kind: ResourceKind::LogGroup,
        id: group.name.clone(),
        name: group.name.clone(),
        arn,
        region: app.config.region.clone(),
    })
}

/// Start/end times for the events view, computed exactly as `load_log_events`:
/// from the relative amount/unit when the amount parses, else unbounded.
fn log_event_time_range(app: &App) -> Result<(Option<i64>, Option<i64>), String> {
    use crate::cw::insights::TimeUnit;
    let state = &app.log_groups_state;
    if state.relative_amount.is_empty() {
        return Ok((None, None));
    }
    let amount = state
        .relative_amount
        .parse::<i64>()
        .map_err(|_| "Invalid log time range")?;
    if amount < 0 {
        return Err("Log time range cannot be negative".into());
    }
    let unit_ms = match state.relative_unit {
        TimeUnit::Minutes => 60_000,
        TimeUnit::Hours => 3_600_000,
        TimeUnit::Days => 86_400_000,
        TimeUnit::Weeks => 604_800_000,
    };
    let now = chrono::Utc::now().timestamp_millis();
    let duration = amount
        .checked_mul(unit_ms)
        .ok_or("Log time range is too large")?;
    let start = now
        .checked_sub(duration)
        .ok_or("Log time range is too large")?;
    Ok((Some(start), Some(now)))
}

fn logs(app: &App) -> Result<Option<Target>, String> {
    let state = &app.log_groups_state;
    match app.view_mode {
        // Top inventory: the log-groups list.
        ViewMode::List => Ok(Some(view("logs describe-log-groups", json!({})))),
        // Entered a group: the Log streams nested table.
        ViewMode::Detail if state.detail_tab == LogsDetailTab::LogStreams => {
            let Some(scope) = log_group_scope(app) else {
                return Err("Enter a log group before copying its streams command".into());
            };
            Ok(Some(scoped(
                "logs describe-log-streams",
                json!({ "logGroupName": &scope.id }),
                scope,
            )))
        }
        ViewMode::Detail if state.detail_tab == LogsDetailTab::Tags => {
            let scope = log_group_scope(app).ok_or("Enter a log group first")?;
            if scope.arn.is_empty() {
                return Err("Refresh the log group to load its ARN".into());
            }
            Ok(Some(scoped(
                "logs list-tags-for-resource",
                json!({"resourceArn":scope.arn}),
                scope,
            )))
        }
        // Entered a stream: the events nested table for the CURRENT stream.
        ViewMode::Events => {
            let Some(scope) = log_group_scope(app) else {
                return Err("Enter a log group before copying its events command".into());
            };
            let stream = state
                .log_streams
                .get(state.selected_stream)
                .ok_or("No log stream is open; enter a stream first")?;
            let (start, end) = log_event_time_range(app)?;
            // Event text filtering is client-side; it is NOT CloudWatch filter
            // pattern syntax. Preserve the entered stream rather than inventing
            // a filter-log-events request with different matching semantics.
            let operation = "logs get-log-events";
            let mut input = json!({
                "logGroupName": &scope.id,
                "logStreamName": &stream.name,
                "startFromHead": false,
            });
            if let Some(start) = start {
                input["startTime"] = json!(start);
            }
            if let Some(end) = end {
                input["endTime"] = json!(end);
            }
            Ok(Some(scoped(operation, input, scope)))
        }
        // Tags and any other detail tab are not nested COLLECTION tables with a
        // single fixed read command owned here.
        _ => Ok(None),
    }
}

// ── CloudWatch Logs Insights ───────────────────────────────────────────────

fn insights() -> Result<Option<Target>, String> {
    // Results come from GetQueryResults, which needs the query ID returned by a
    // prior StartQuery. The UI keeps no query ID (it discards it after polling),
    // so there is nothing to scope a GetQueryResults to — and StartQuery is
    // never emitted because it creates a query and bills the scanned bytes.
    Err(
        "Logs Insights results are not reproducible as a read-only command: GetQueryResults needs \
         a query ID the view does not retain, and StartQuery would create and bill a new scan"
            .into(),
    )
}

// ── CloudWatch Alarms ────────────────────────────────────────────────────────

fn alarms(app: &App) -> Result<Option<Target>, String> {
    let state = &app.alarms_state;
    // A detail view is open: only a tab backed by exactly one read-only API is
    // reproducible. Tags/History are single-alarm reads; the rest are not fixed
    // collection tables and fall through.
    if let Some(alarm) = &state.current_alarm {
        return match state.detail_tab {
            AlarmDetailTab::Tags => {
                let parent = crate::finder::related::selected(app).ok_or("The entered alarm identity is unavailable")?;
                if parent.arn.is_empty() { return Err("Refresh the alarm to load its ARN".into()); }
                Ok(Some(scoped("cloudwatch list-tags-for-resource", json!({"ResourceARN":parent.arn}), parent)))
            }
            AlarmDetailTab::Actions | AlarmDetailTab::ParentAlarms | AlarmDetailTab::MuteRules => Err("This alarm table is assembled from parent metadata or multiple reads; no single scoped collection command is available".into()),
            AlarmDetailTab::History => Ok(Some(view(
                "cloudwatch describe-alarm-history",
                json!({ "AlarmName": alarm }),
            ))),
            _ => Ok(None),
        };
    }
    // Top inventory: the alarms table. The "In alarm" tab is the same
    // describe-alarms narrowed by state; keep that server-side scope.
    let mut input = json!({});
    if state.alarm_tab == AlarmTab::InAlarm {
        input["StateValue"] = json!("ALARM");
    }
    Ok(Some(view("cloudwatch describe-alarms", input)))
}

// ── CloudTrail ───────────────────────────────────────────────────────────────

fn cloudtrail(app: &App) -> Result<Option<Target>, String> {
    let state = &app.cloudtrail_state;
    // An entered event's JSON/detail is a single record, not a collection — the
    // view has no selected-row command and no collection command of its own.
    if state.current_event.is_some() {
        return Ok(None);
    }
    // The event-history table: lookup-events with the EventName filter the view
    // currently has applied (set on Enter), and nothing else selected.
    let mut input = json!({});
    if let Some(name) = &state.active_event_name_filter {
        if !name.is_empty() {
            input["LookupAttributes"] = json!([{
                "AttributeKey": "EventName",
                "AttributeValue": name,
            }]);
        }
    }
    Ok(Some(view("cloudtrail lookup-events", input)))
}

// ── DynamoDB ─────────────────────────────────────────────────────────────────

fn table_scope(app: &App, table: &rusticity_core::dynamodb::DdbTable) -> Resource {
    Resource {
        kind: ResourceKind::Table,
        id: table.name.clone(),
        name: table.name.clone(),
        arn: table.table_arn.clone(),
        region: app.config.region.clone(),
    }
}

fn dynamodb_view(app: &App) -> Result<Option<Target>, String> {
    let state = &app.dynamodb_state;
    // Top inventory: the tables list.
    if state.current_table.is_none() {
        return Ok(Some(view("dynamodb list-tables", json!({}))));
    }
    let table = dynamodb::current_table(app)
        .ok_or("Refresh the entered table to load its metadata before copying this command")?;
    let scope = table_scope(app, table);
    match state.detail_tab {
        // Items tab: reproduce the EXACT Scan/Query the view is running — same
        // index, projection, key type, partition value. Never scan blindly.
        DdbDetailTab::Items => {
            let index = dynamodb::selected_item_index(table, state.item_index);
            let keys = dynamodb::selected_key_attributes(table, state.item_index);
            let projection = match state.item_projection {
                ItemProjection::AllAttributes => None,
                ItemProjection::KeysOnly => Some(keys.as_slice()),
            };
            match state.item_operation {
                ItemOperation::Scan => {
                    let mut input = json!({
                        "TableName": table.name,
                        "Limit": dynamodb::ITEM_LIMIT,
                    });
                    if let Some(index) = &index {
                        input["IndexName"] = json!(index);
                    }
                    apply_projection(&mut input, projection);
                    Ok(Some(scoped("dynamodb scan", input, scope)))
                }
                ItemOperation::Query => {
                    let value = state.item_filter.as_str();
                    if value.is_empty() {
                        return Err(
                            "Enter the partition-key value in the Items tab before copying the \
                             Query command"
                                .into(),
                        );
                    }
                    let Some(pk) = keys.first() else {
                        return Err(
                            "The selected table or index has no partition key to query".into()
                        );
                    };
                    let partition_type = dynamodb::key_attribute_type(
                        &dynamodb::selected_partition_key_label(table, state.item_index),
                    );
                    let value_node = match partition_type.as_str() {
                        "N" => json!({ "N": value }),
                        _ => json!({ "S": value }),
                    };
                    let mut input = json!({
                        "TableName": table.name,
                        "Limit": dynamodb::ITEM_LIMIT,
                        "KeyConditionExpression": "#pk = :pk",
                        "ExpressionAttributeNames": { "#pk": pk },
                        "ExpressionAttributeValues": { ":pk": value_node },
                    });
                    if let Some(index) = &index {
                        input["IndexName"] = json!(index);
                    }
                    apply_projection(&mut input, projection);
                    Ok(Some(scoped("dynamodb query", input, scope)))
                }
            }
        }
        // Indexes and Global tables (replicas) are rendered from DescribeTable;
        // Target::View has no JMESPath/query support yet, so the whole-table
        // describe is the faithful collection read — NOT a describe of the
        // highlighted index/replica row.
        DdbDetailTab::Indexes | DdbDetailTab::GlobalTables => Ok(Some(scoped(
            "dynamodb describe-table",
            json!({ "TableName": table.name }),
            scope,
        ))),
        // Backups is a distinct list API scoped to the table.
        DdbDetailTab::Backups => Ok(Some(scoped(
            "dynamodb list-backups",
            json!({ "TableName": table.name }),
            scope,
        ))),
        // Tags is a resource-scoped list keyed by the table ARN.
        DdbDetailTab::Tags => {
            if table.table_arn.is_empty() {
                return Err(
                    "The table ARN is not loaded; refresh the table before copying its \
                            tags command"
                        .into(),
                );
            }
            Ok(Some(scoped(
                "dynamodb list-tags-of-resource",
                json!({ "ResourceArn": table.table_arn }),
                scope,
            )))
        }
        // Settings / Monitor / Exports&streams / Additional settings aren't a
        // single fixed collection read.
        _ => Ok(None),
    }
}

fn apply_projection(input: &mut Value, projection: Option<&[String]>) {
    let Some(attrs) = projection else {
        return;
    };
    let placeholders: Vec<String> = (0..attrs.len()).map(|i| format!("#p{i}")).collect();
    input["ProjectionExpression"] = json!(placeholders.join(", "));
    let mut names = serde_json::Map::new();
    for (placeholder, attr) in placeholders.iter().zip(attrs.iter()) {
        names.insert(placeholder.clone(), json!(attr));
    }
    // Query already carries a #pk name entry; merge rather than overwrite.
    if let Some(Value::Object(existing)) = input.get_mut("ExpressionAttributeNames") {
        for (k, v) in names {
            existing.insert(k, v);
        }
    } else {
        input["ExpressionAttributeNames"] = Value::Object(names);
    }
}

// ── Lambda ───────────────────────────────────────────────────────────────────

fn function_scope(app: &App, name: &str) -> Resource {
    Resource {
        kind: ResourceKind::Function,
        id: name.to_string(),
        name: name.to_string(),
        arn: String::new(),
        region: app.config.region.clone(),
    }
}

fn lambda_functions(app: &App) -> Result<Option<Target>, String> {
    let state = &app.lambda_state;
    let Some(function) = &state.current_function else {
        // Top inventory: the functions list.
        return Ok(Some(view("lambda list-functions", json!({}))));
    };
    let scope = function_scope(app, function);
    match state.detail_tab {
        // Versions / Aliases are list APIs scoped to the entered function.
        LambdaDetailTab::Versions => Ok(Some(scoped(
            "lambda list-versions-by-function",
            json!({ "FunctionName": function }),
            scope,
        ))),
        LambdaDetailTab::Aliases => Ok(Some(scoped(
            "lambda list-aliases",
            json!({ "FunctionName": function }),
            scope,
        ))),
        // Code / Configuration / Monitor are single-function detail, not nested
        // collection tables; the legacy per-resource describe path owns them.
        _ => Ok(None),
    }
}

fn lambda_applications() -> Result<Option<Target>, String> {
    // The Applications view aggregates a CloudFormation stack plus per-resource
    // Lambda lookups; no single read-only command reproduces the table, so we
    // refuse explicitly rather than pick one of the several APIs it fans out to.
    Err(
        "The Lambda Applications view combines CloudFormation and Lambda APIs; no single \
         read-only command reproduces it"
            .into(),
    )
}

// ── CloudFormation ─────────────────────────────────────────────────────────

fn stack_scope(app: &App, name: &str) -> Resource {
    Resource {
        kind: ResourceKind::Stack,
        id: name.to_string(),
        name: name.to_string(),
        arn: String::new(),
        region: app.config.region.clone(),
    }
}

fn cloudformation(app: &App) -> Result<Option<Target>, String> {
    use crate::ui::cfn::DetailTab as CfnDetailTab;
    let state = &app.cfn_state;
    let Some(stack) = &state.current_stack else {
        // Top inventory: the stacks list.
        return Ok(Some(view("cloudformation describe-stacks", json!({}))));
    };
    let scope = stack_scope(app, stack);
    match state.detail_tab {
        // Resources / Events / Change sets are list APIs scoped to the stack.
        CfnDetailTab::Resources => Ok(Some(scoped(
            "cloudformation describe-stack-resources",
            json!({ "StackName": stack }),
            scope,
        ))),
        CfnDetailTab::Events => Ok(Some(scoped(
            "cloudformation describe-stack-events",
            json!({ "StackName": stack }),
            scope,
        ))),
        CfnDetailTab::ChangeSets => Ok(Some(scoped(
            "cloudformation list-change-sets",
            json!({ "StackName": stack }),
            scope,
        ))),
        // Parameters and Outputs are fields of DescribeStacks on the PARENT
        // stack — the whole-stack describe, never a selected parameter/output.
        CfnDetailTab::Parameters | CfnDetailTab::Outputs => Ok(Some(scoped(
            "cloudformation describe-stacks",
            json!({ "StackName": stack }),
            scope,
        ))),
        // StackInfo / Template / GitSync are not fixed collection tables here.
        _ => Ok(None),
    }
}

// ── IAM ──────────────────────────────────────────────────────────────────────

fn iam_users(app: &App) -> Result<Option<Target>, String> {
    let state = &app.iam_state;
    let Some(user) = &state.current_user else {
        return Ok(Some(global("iam list-users", json!({}))));
    };
    match state.user_tab {
        UserTab::Groups => Ok(Some(global(
            "iam list-groups-for-user",
            json!({ "UserName": user }),
        ))),
        UserTab::Tags => Ok(Some(global(
            "iam list-user-tags",
            json!({ "UserName": user }),
        ))),
        // Permissions combines attached (managed) and inline policies — two
        // APIs. Refuse rather than silently emit only one.
        UserTab::Permissions => Err(
            "The Permissions tab combines managed and inline policies from two IAM APIs; no single \
             read-only command reproduces it"
                .into(),
        ),
        UserTab::LastAccessed => Err("Last accessed requires an existing IAM report ID, which this view does not retain".into()),
        // Security credentials is a detail pane.
        _ => Ok(None),
    }
}

fn iam_roles(app: &App) -> Result<Option<Target>, String> {
    let state = &app.iam_state;
    let Some(role) = &state.current_role else {
        return Ok(Some(global("iam list-roles", json!({}))));
    };
    match state.role_tab {
        RoleTab::Tags => Ok(Some(global(
            "iam list-role-tags",
            json!({ "RoleName": role }),
        ))),
        RoleTab::Permissions => Err(
            "The Permissions tab combines managed and inline policies from two IAM APIs; no single \
             read-only command reproduces it"
                .into(),
        ),
        RoleTab::LastAccessed => Err("Last accessed requires an existing IAM report ID, which this view does not retain".into()),
        // Trust relationships and revoke sessions are detail panes.
        _ => Ok(None),
    }
}

fn iam_groups(app: &App) -> Result<Option<Target>, String> {
    let state = &app.iam_state;
    let Some(group) = &state.current_group else {
        return Ok(Some(global("iam list-groups", json!({}))));
    };
    match state.group_tab {
        // The Users tab IS get-group: it returns the group plus its members,
        // scoped to the entered group.
        GroupTab::Users => Ok(Some(global(
            "iam get-group",
            json!({ "GroupName": group }),
        ))),
        GroupTab::Permissions => Err(
            "The Permissions tab combines managed and inline policies from two IAM APIs; no single \
             read-only command reproduces it"
                .into(),
        ),
        _ => Err("Access Advisor requires an existing IAM report ID, which this view does not retain".into()),
    }
}

// ── API Gateway ──────────────────────────────────────────────────────────────

fn apigateway(app: &App) -> Result<Option<Target>, String> {
    let state = &app.apig_state;
    let Some(api) = &state.current_api else {
        // The top inventory combines REST (v1), HTTP and WebSocket (v2) APIs
        // from different services — no single list command produces that table.
        return Err(
            "The API Gateway inventory combines REST, HTTP and WebSocket APIs across two services; \
             no single read-only command reproduces it"
                .into(),
        );
    };
    // Entered an API: its resources (REST) or routes (HTTP/WebSocket), scoped to
    // the whole API by id — never the highlighted child resource/route.
    match api.protocol_type.to_ascii_uppercase().as_str() {
        "REST" => Ok(Some(view(
            "apigateway get-resources",
            json!({"restApiId":api.id}),
        ))),
        "HTTP" | "WEBSOCKET" => Ok(Some(view(
            "apigatewayv2 get-routes",
            json!({"ApiId":api.id}),
        ))),
        _ => Err("The entered API protocol is unknown; refresh it first".into()),
    }
}

#[cfg(test)]
mod tests;
