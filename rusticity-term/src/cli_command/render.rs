//! Pure command formatting. No process execution, AWS calls, or credential reads.
use rusticity_core::finder::{Resource, ResourceKind};
use serde_json::{json, Value};

pub(super) enum Target {
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
