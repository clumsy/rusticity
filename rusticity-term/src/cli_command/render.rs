//! Pure command formatting. No process execution, AWS calls, or credential reads.
use rusticity_core::finder::{Resource, ResourceKind};
use serde_json::{json, Value};

#[derive(Debug)]
pub(super) enum Target {
    /// A fixed read-only API for the current collection, never a selected row.
    View {
        operation: &'static str,
        input: Value,
        region: Option<String>,
        scope: Option<Resource>,
    },
    Resource(Resource),
    TopicSubscriptions(Resource),
    EcrImage {
        repository: String,
        digest: String,
    },
    S3Object {
        bucket: String,
        key: String,
        region: String,
    },
    Api {
        id: String,
        protocol: String,
    },
}
#[derive(Clone, Copy)]
pub(super) struct Context<'a> {
    pub profile: &'a str,
    pub region: &'a str,
    pub account_id: &'a str,
}

pub(super) fn command(target: &Target, context: Context<'_>) -> Result<String, String> {
    invocation(target, context).map(|invocation| invocation.display)
}

pub(super) fn invocation(
    target: &Target,
    context: Context<'_>,
) -> Result<super::Invocation, String> {
    validate("Profile", context.profile, false)?;
    validate("Region", context.region, false)?;
    validate("Account", context.account_id, false)?;
    let (operation, input, region) = match target {
        Target::View {
            operation,
            input,
            region,
            scope,
        } => {
            if !VIEW_OPERATIONS.contains(operation) {
                return Err("This view has no allowlisted read-only CLI operation".into());
            }
            if !input.is_object() {
                return Err("View command input must be a JSON object".into());
            }
            validate_json(input)?;
            if let Some(parent) = scope {
                resource_command(parent, context)?;
            }
            let selected_region = if operation.starts_with("iam ") {
                None
            } else if operation.starts_with("ecr-public ") {
                Some("us-east-1".into())
            } else {
                Some(regional(region.as_deref().unwrap_or(context.region))?)
            };
            for key in [
                "stateMachineArn",
                "TopicArn",
                "ResourceArn",
                "resourceArn",
                "resourceARN",
                "KeyId",
                "StackName",
                "FunctionName",
                "ResourceARN",
                "SourceArn",
                "EventSourceArn",
            ] {
                if let Some(value) = input.get(key).and_then(Value::as_str) {
                    validate(key, value, true)?;
                    if value.starts_with("arn:") {
                        validate_parent_arn(value, context, selected_region.as_deref())?;
                    }
                }
            }
            (*operation, input.clone(), selected_region)
        }
        Target::Resource(resource) => resource_command(resource, context)?,
        Target::TopicSubscriptions(resource) => {
            if resource.kind != ResourceKind::Topic {
                return Err("Expected an SNS topic".into());
            }
            let (_, input, region) = resource_command(resource, context)?;
            ("sns list-subscriptions-by-topic", input, region)
        }
        Target::EcrImage { repository, digest } => {
            validate("Repository", repository, true)?;
            validate("Image digest", digest, true)?;
            (
                "ecr describe-images",
                json!({"repositoryName":repository,"imageIds":[{"imageDigest":digest}]}),
                Some(regional(context.region)?),
            )
        }
        Target::S3Object {
            bucket,
            key,
            region,
        } => {
            validate("Bucket", bucket, true)?;
            validate("Object key", key, true)?;
            let region = if region.is_empty() || region == "global" {
                context.region
            } else {
                region
            };
            (
                "s3api head-object",
                json!({"Bucket":bucket,"Key":key}),
                Some(regional(region)?),
            )
        }
        Target::Api { id, protocol } => {
            validate("API ID", id, true)?;
            let (op, input) = match protocol.to_ascii_uppercase().as_str() {
                "REST" => ("apigateway get-rest-api", json!({"restApiId":id})),
                "HTTP" | "WEBSOCKET" => ("apigatewayv2 get-api", json!({"ApiId":id})),
                _ => {
                    return Err(
                        "API protocol is unknown; select a REST, HTTP or WebSocket API".into(),
                    )
                }
            };
            (op, input, Some(regional(context.region)?))
        }
    };
    // All resource parameters travel INSIDE JSON. AWS CLI parameter-file and
    // shorthand expansion therefore cannot reinterpret a literal object key or
    // identifier beginning with file://, fileb://, [, a dash, or a comma.
    // This global option MUST be a separate argv pair: CLI v2's required-
    // argument parser does not recognize --cli-input-json=... as supplying
    // the operation's required parameters.
    let mut text = format!(
        "aws {operation} --cli-input-json {}",
        shell_quote(&input.to_string())
    );
    let mut args: Vec<String> = operation.split_whitespace().map(str::to_owned).collect();
    args.extend(["--cli-input-json".into(), input.to_string()]);
    if let Some(region) = region {
        text.push_str(&format!(" --region={}", shell_quote(&region)));
        args.push(format!("--region={region}"));
    }
    if !context.profile.is_empty() {
        text.push_str(&format!(" --profile={}", shell_quote(context.profile)));
        args.push(format!("--profile={}", context.profile));
    }
    Ok(super::Invocation {
        args,
        display: text,
    })
}

// Only these fixed collection reads may pass the view-command boundary.
const VIEW_OPERATIONS: &[&str] = &[
    "sqs list-queues",
    "sqs list-queue-tags",
    "sqs list-message-move-tasks",
    "lambda list-event-source-mappings",
    "stepfunctions list-state-machines",
    "stepfunctions list-executions",
    "sns list-topics",
    "sns list-subscriptions-by-topic",
    "secretsmanager list-secrets",
    "rds describe-db-instances",
    "ec2 describe-vpcs",
    "ec2 describe-instances",
    "ec2 describe-tags",
    "kms list-keys",
    "kms list-aliases",
    "kms list-resource-tags",
    "efs describe-file-systems",
    "efs describe-access-points",
    "efs describe-mount-targets",
    "efs list-tags-for-resource",
    "fsx describe-file-systems",
    "fsx describe-backups",
    "fsx list-tags-for-resource",
    "s3api list-buckets",
    "s3api list-objects-v2",
    "ecr describe-repositories",
    "ecr-public describe-repositories",
    "ecr describe-images",
    "ecr-public describe-images",
    "ecr describe-image-scan-findings",
    "ecr list-image-referrers",
    "ecr describe-image-replication-status",
    "ecr describe-image-signing-status",
    "logs describe-log-groups",
    "logs describe-log-streams",
    "logs get-log-events",
    "logs list-tags-for-resource",
    "cloudwatch describe-alarms",
    "cloudwatch describe-alarm-history",
    "cloudwatch list-tags-for-resource",
    "cloudtrail lookup-events",
    "dynamodb list-tables",
    "dynamodb describe-table",
    "dynamodb scan",
    "dynamodb query",
    "dynamodb list-backups",
    "dynamodb list-tags-of-resource",
    "lambda list-functions",
    "lambda list-versions-by-function",
    "lambda list-aliases",
    "cloudformation describe-stacks",
    "cloudformation describe-stack-resources",
    "cloudformation describe-stack-events",
    "cloudformation list-change-sets",
    "iam list-users",
    "iam list-roles",
    "iam list-groups",
    "iam list-groups-for-user",
    "iam list-user-tags",
    "iam list-role-tags",
    "iam get-group",
    "apigateway get-resources",
    "apigatewayv2 get-routes",
];

fn validate_json(value: &Value) -> Result<(), String> {
    match value {
        Value::String(text) => validate("View parameter", text, false),
        Value::Array(values) => values.iter().try_for_each(validate_json),
        Value::Object(values) => values.iter().try_for_each(|(key, value)| {
            validate("View parameter name", key, false)?;
            validate_json(value)
        }),
        _ => Ok(()),
    }
}

fn validate_parent_arn(
    arn: &str,
    context: Context<'_>,
    region: Option<&str>,
) -> Result<(), String> {
    let parts: Vec<_> = arn.splitn(6, ':').collect();
    if parts.len() != 6 || parts[1].is_empty() || parts[2].is_empty() || parts[5].is_empty() {
        return Err("Parent ARN is malformed".into());
    }
    if !parts[4].is_empty() && !context.account_id.is_empty() && parts[4] != context.account_id {
        return Err("Parent belongs to another account; switch profiles first".into());
    }
    if !parts[3].is_empty() && region.is_some_and(|region| region != parts[3]) {
        return Err("Parent is in another Region; switch regions first".into());
    }
    Ok(())
}

fn resource_command(
    resource: &Resource,
    context: Context<'_>,
) -> Result<(&'static str, Value, Option<String>), String> {
    use ResourceKind::*;
    if resource.kind == Reference {
        return Err("This resource type has no supported read-only CLI command".into());
    }
    if resource.kind == Api {
        return Err(
            "Open this API's service view first so its REST/HTTP/WebSocket protocol is known"
                .into(),
        );
    }
    validate("Resource ID", &resource.id, true)?;
    validate("Resource ARN", &resource.arn, false)?;
    validate("Resource Region", &resource.region, false)?;
    let global = matches!(resource.kind, User | Role | Group);
    let mut arn_region = "";
    let mut owner = "";
    if !resource.arn.is_empty() {
        let parts: Vec<_> = resource.arn.splitn(6, ':').collect();
        if parts.len() != 6
            || parts[0] != "arn"
            || parts[1].is_empty()
            || parts[2].is_empty()
            || parts[5].is_empty()
        {
            return Err("Resource ARN is malformed".into());
        }
        owner = parts[4];
        arn_region = parts[3];
        if !owner.is_empty() && !context.account_id.is_empty() && owner != context.account_id {
            return Err(
                "Resource belongs to another account; switch profiles before copying its command"
                    .into(),
            );
        }
    }
    let region = if global {
        None
    } else if resource.kind == PublicRepository {
        Some("us-east-1".into())
    } else if resource.kind == Bucket {
        Some(regional(
            if resource.region.is_empty() || resource.region == "global" {
                context.region
            } else {
                &resource.region
            },
        )?)
    } else {
        for region in [resource.region.as_str(), arn_region] {
            if !region.is_empty() && !context.region.is_empty() && region != context.region {
                return Err(
                    "Resource is in another Region; switch regions before copying its command"
                        .into(),
                );
            }
        }
        let region = if !resource.region.is_empty() {
            resource.region.as_str()
        } else if !arn_region.is_empty() {
            arn_region
        } else {
            context.region
        };
        Some(regional(region)?)
    };
    let id = resource.id.as_str();
    let arn_or_id = if resource.arn.is_empty() {
        id
    } else {
        &resource.arn
    };
    let (operation, input) = match resource.kind {
        Instance => ("ec2 describe-instances", json!({"InstanceIds":[id]})),
        Vpc => ("ec2 describe-vpcs", json!({"VpcIds":[id]})),
        Table => ("dynamodb describe-table", json!({"TableName":id})),
        User => ("iam get-user", json!({"UserName":id})),
        Role => ("iam get-role", json!({"RoleName":id})),
        Group => ("iam get-group", json!({"GroupName":id})),
        Repository => ("ecr describe-repositories", json!({"repositoryNames":[id]})),
        PublicRepository => (
            "ecr-public describe-repositories",
            json!({"repositoryNames":[id]}),
        ),
        Efs => ("efs describe-file-systems", json!({"FileSystemId":id})),
        Fsx => ("fsx describe-file-systems", json!({"FileSystemIds":[id]})),
        Stack => (
            "cloudformation describe-stacks",
            json!({"StackName":arn_or_id}),
        ),
        Alarm => ("cloudwatch describe-alarms", json!({"AlarmNames":[id]})),
        LogGroup => (
            "logs describe-log-groups",
            json!({"logGroupIdentifiers":[arn_or_id.strip_suffix(":*").unwrap_or(arn_or_id)]}),
        ),
        Key => ("kms describe-key", json!({"KeyId":arn_or_id})),
        Function => (
            "lambda get-function-configuration",
            json!({"FunctionName":arn_or_id}),
        ),
        Database => (
            "rds describe-db-instances",
            json!({"DBInstanceIdentifier":id}),
        ),
        Bucket => ("s3api get-bucket-location", json!({"Bucket":id})),
        StateMachine => {
            if resource.arn.is_empty()
                || !Resource::from_arn(&resource.arn).is_some_and(|r| r.kind == StateMachine)
            {
                return Err("A full Step Functions state machine ARN is required".into());
            }
            (
                "stepfunctions list-executions",
                json!({"stateMachineArn":resource.arn}),
            )
        }
        Secret => {
            if resource.arn.is_empty()
                || !Resource::from_arn(&resource.arn).is_some_and(|r| r.kind == Secret)
            {
                return Err(
                    "Secret metadata requires the full Secrets Manager ARN; refresh the inventory"
                        .into(),
                );
            }
            (
                "secretsmanager describe-secret",
                json!({"SecretId":resource.arn}),
            )
        }
        Topic => {
            if resource.arn.is_empty() || rusticity_core::SnsTopic::from_arn(&resource.arn).is_err()
            {
                return Err("SNS topic requires a valid full ARN; refresh its inventory".into());
            }
            ("sns get-topic-attributes", json!({"TopicArn":resource.arn}))
        }
        Queue => {
            let mut input = json!({"QueueName":id});
            if !owner.is_empty() {
                input["QueueOwnerAWSAccountId"] = json!(owner);
            }
            ("sqs get-queue-url", input)
        }
        Api | Reference => unreachable!("unsupported types rejected before dispatch"),
    };
    Ok((operation, input, region))
}
fn regional(region: &str) -> Result<String, String> {
    validate("Region", region, true)?;
    if region == "global" {
        return Err("A regional command needs a concrete AWS Region".into());
    }
    Ok(region.into())
}
fn validate(field: &str, value: &str, required: bool) -> Result<(), String> {
    if required && value.is_empty() {
        return Err(format!(
            "{field} is missing; select a resource or refresh the view"
        ));
    }
    if value.chars().any(char::is_control) {
        return Err(format!(
            "{field} contains a control character; no command copied"
        ));
    }
    Ok(())
}
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests;
