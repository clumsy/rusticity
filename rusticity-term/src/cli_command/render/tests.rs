use super::*;
fn context() -> Context<'static> {
    Context {
        profile: "beta",
        region: "us-east-1",
        account_id: "123456789012",
    }
}
fn resource(kind: ResourceKind) -> Resource {
    Resource {
        kind,
        id: "selected".into(),
        name: "Display name".into(),
        arn: String::new(),
        region: "us-east-1".into(),
    }
}
/// Decode only POSIX single-quoted words, independently of the encoder.
fn words(text: &str) -> Vec<String> {
    let mut words = vec![];
    let mut word = String::new();
    let mut quoted = false;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '\'' => quoted = !quoted,
            '\\' if !quoted => word.push(chars.next().expect("escape character")),
            ' ' if !quoted => {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
            }
            _ => word.push(c),
        }
    }
    assert!(!quoted);
    if !word.is_empty() {
        words.push(word);
    }
    words
}
fn input(command: &str) -> Value {
    let words = words(command);
    let raw = words
        .windows(2)
        .find(|pair| pair[0] == "--cli-input-json")
        .expect("CLI JSON must be supplied as a separate argument pair");
    serde_json::from_str(&raw[1]).unwrap()
}

#[test]
fn every_supported_resource_kind_maps_to_one_read_only_operation() {
    use ResourceKind::*;
    for (kind, op, key, value) in [
        (
            Instance,
            "ec2 describe-instances",
            "InstanceIds",
            json!(["selected"]),
        ),
        (Vpc, "ec2 describe-vpcs", "VpcIds", json!(["selected"])),
        (
            Table,
            "dynamodb describe-table",
            "TableName",
            json!("selected"),
        ),
        (User, "iam get-user", "UserName", json!("selected")),
        (Role, "iam get-role", "RoleName", json!("selected")),
        (Group, "iam get-group", "GroupName", json!("selected")),
        (
            Repository,
            "ecr describe-repositories",
            "repositoryNames",
            json!(["selected"]),
        ),
        (
            PublicRepository,
            "ecr-public describe-repositories",
            "repositoryNames",
            json!(["selected"]),
        ),
        (
            Efs,
            "efs describe-file-systems",
            "FileSystemId",
            json!("selected"),
        ),
        (
            Fsx,
            "fsx describe-file-systems",
            "FileSystemIds",
            json!(["selected"]),
        ),
        (
            Stack,
            "cloudformation describe-stacks",
            "StackName",
            json!("selected"),
        ),
        (
            Alarm,
            "cloudwatch describe-alarms",
            "AlarmNames",
            json!(["selected"]),
        ),
        (
            LogGroup,
            "logs describe-log-groups",
            "logGroupIdentifiers",
            json!(["selected"]),
        ),
        (Key, "kms describe-key", "KeyId", json!("selected")),
        (
            Function,
            "lambda get-function-configuration",
            "FunctionName",
            json!("selected"),
        ),
        (
            Database,
            "rds describe-db-instances",
            "DBInstanceIdentifier",
            json!("selected"),
        ),
        (
            Bucket,
            "s3api get-bucket-location",
            "Bucket",
            json!("selected"),
        ),
        (Queue, "sqs get-queue-url", "QueueName", json!("selected")),
    ] {
        let command = command(&Target::Resource(resource(kind)), context()).unwrap();
        assert!(command.starts_with(&format!("aws {op} ")), "{command}");
        assert_eq!(input(&command)[key], value);
        assert!(words(&command).contains(&"--profile=beta".into()));
    }
}

#[test]
fn regional_global_public_and_bucket_region_policies() {
    for kind in [ResourceKind::User, ResourceKind::Role, ResourceKind::Group] {
        let mut r = resource(kind);
        r.region = "global".into();
        assert!(!command(&Target::Resource(r), context())
            .unwrap()
            .contains("--region"));
    }
    let cmd = command(
        &Target::Resource(resource(ResourceKind::PublicRepository)),
        Context {
            region: "eu-west-1",
            ..context()
        },
    )
    .unwrap();
    assert!(cmd.contains("--region='us-east-1'"));
    let mut bucket = resource(ResourceKind::Bucket);
    bucket.region = "eu-west-1".into();
    assert!(command(&Target::Resource(bucket), context())
        .unwrap()
        .contains("--region='eu-west-1'"));
    let mut r = resource(ResourceKind::Instance);
    r.region.clear();
    assert!(command(
        &Target::Resource(r),
        Context {
            region: "eu-west-1",
            ..context()
        }
    )
    .unwrap()
    .contains("--region='eu-west-1'"));
}

#[test]
fn arn_account_and_region_mismatches_refuse_to_copy() {
    let mut r = resource(ResourceKind::Function);
    r.arn = "arn:aws:lambda:us-east-1:999999999999:function:test".into();
    assert!(command(&Target::Resource(r.clone()), context())
        .unwrap_err()
        .contains("switch profiles"));
    r.arn = "arn:aws:lambda:eu-west-1:123456789012:function:test".into();
    assert!(command(&Target::Resource(r.clone()), context())
        .unwrap_err()
        .contains("switch regions"));
    r.arn.clear();
    r.region = "eu-west-1".into();
    assert!(command(&Target::Resource(r), context())
        .unwrap_err()
        .contains("switch regions"));
}

#[test]
fn prefers_actual_arns_and_keeps_lambda_qualifiers() {
    for (kind, arn, key) in [
        (
            ResourceKind::Function,
            "arn:aws:lambda:us-east-1:123456789012:function:test:PROD",
            "FunctionName",
        ),
        (
            ResourceKind::Key,
            "arn:aws:kms:us-east-1:123456789012:key/abc",
            "KeyId",
        ),
        (
            ResourceKind::Stack,
            "arn:aws:cloudformation:us-east-1:123456789012:stack/test/uuid",
            "StackName",
        ),
        (
            ResourceKind::Topic,
            "arn:aws:sns:us-east-1:123456789012:orders.fifo",
            "TopicArn",
        ),
    ] {
        let mut r = resource(kind);
        r.arn = arn.into();
        assert_eq!(
            input(&command(&Target::Resource(r), context()).unwrap())[key],
            arn
        );
    }
    let mut r = resource(ResourceKind::LogGroup);
    r.arn = "arn:aws:logs:us-east-1:123456789012:log-group:/test:*".into();
    assert_eq!(
        input(&command(&Target::Resource(r), context()).unwrap())["logGroupIdentifiers"],
        json!(["arn:aws:logs:us-east-1:123456789012:log-group:/test"])
    );
}

#[test]
fn sqs_includes_actual_owner_when_known() {
    let mut r = resource(ResourceKind::Queue);
    r.arn = "arn:aws:sqs:us-east-1:123456789012:selected".into();
    assert_eq!(
        input(&command(&Target::Resource(r), context()).unwrap())["QueueOwnerAWSAccountId"],
        "123456789012"
    );
}

#[test]
fn image_digest_and_protocol_specific_api_parameters() {
    let cmd = command(
        &Target::EcrImage {
            repository: "team/image".into(),
            digest: "sha256:immutable".into(),
        },
        context(),
    )
    .unwrap();
    assert_eq!(
        input(&cmd),
        json!({"repositoryName":"team/image","imageIds":[{"imageDigest":"sha256:immutable"}]})
    );
    for (protocol, op, key) in [
        ("REST", "apigateway get-rest-api", "restApiId"),
        ("HTTP", "apigatewayv2 get-api", "ApiId"),
        ("WEBSOCKET", "apigatewayv2 get-api", "ApiId"),
    ] {
        let cmd = command(
            &Target::Api {
                id: "abc".into(),
                protocol: protocol.into(),
            },
            context(),
        )
        .unwrap();
        assert!(cmd.starts_with(&format!("aws {op} ")));
        assert_eq!(input(&cmd)[key], "abc");
    }
}

#[test]
fn literal_hostile_values_survive_shell_and_json_without_file_expansion() {
    for key in [
        "it's a file",
        "$(touch bad); echo pwned",
        "file:///etc/passwd",
        "fileb://secret",
        "--profile=evil",
        "[one,two]",
        "a,b=c",
        "a\\b\"c",
        "資源",
    ] {
        let cmd = command(
            &Target::S3Object {
                bucket: "bucket".into(),
                key: key.into(),
                region: "us-east-1".into(),
            },
            context(),
        )
        .unwrap();
        assert_eq!(input(&cmd)["Key"], key);
        assert_eq!(words(&cmd).len(), 7); // aws, service, operation, JSON option/value, Region, profile
        assert!(cmd.starts_with("aws s3api head-object "));
    }
    assert_eq!(shell_quote("it's"), "'it'\\''s'");
}

#[test]
fn missing_identifiers_and_control_characters_are_errors_not_broad_queries() {
    for kind in ResourceKind::ALL {
        let mut r = resource(kind);
        r.id.clear();
        assert!(
            command(&Target::Resource(r), context()).is_err(),
            "{kind:?}"
        );
    }
    for id in ["line\nbreak", "nul\0value", "escape\x1bvalue"] {
        let mut r = resource(ResourceKind::Table);
        r.id = id.into();
        assert!(command(&Target::Resource(r), context())
            .unwrap_err()
            .contains("control character"));
    }
    for target in [
        Target::EcrImage {
            repository: "repo".into(),
            digest: "".into(),
        },
        Target::S3Object {
            bucket: "bucket".into(),
            key: "".into(),
            region: "us-east-1".into(),
        },
        Target::Api {
            id: "".into(),
            protocol: "REST".into(),
        },
    ] {
        assert!(command(&target, context()).is_err());
    }
}

#[test]
fn unsupported_types_unknown_protocol_and_missing_sns_arn_are_explicit() {
    for kind in [
        ResourceKind::Reference,
        ResourceKind::Api,
        ResourceKind::Topic,
    ] {
        assert!(command(&Target::Resource(resource(kind)), context()).is_err());
    }
    assert!(command(
        &Target::Api {
            id: "abc".into(),
            protocol: "UNKNOWN".into()
        },
        context()
    )
    .is_err());
}

#[test]
fn unset_profile_uses_provider_chain_but_regional_commands_require_region() {
    let mut r = resource(ResourceKind::Table);
    r.region.clear();
    let cmd = command(
        &Target::Resource(r.clone()),
        Context {
            profile: "",
            ..context()
        },
    )
    .unwrap();
    assert!(!cmd.contains("--profile"));
    assert!(command(
        &Target::Resource(r),
        Context {
            region: "",
            ..context()
        }
    )
    .is_err());
    let cmd = command(
        &Target::Resource(resource(ResourceKind::Table)),
        Context {
            profile: "dev's $(x)",
            ..context()
        },
    )
    .unwrap();
    assert!(words(&cmd).contains(&"--profile=dev's $(x)".into()));
    assert!(command(
        &Target::Resource(resource(ResourceKind::Table)),
        Context {
            profile: "bad\nprofile",
            ..context()
        }
    )
    .is_err());
}

mod contracts;

#[test]
fn sns_command_supplies_json_as_separate_argument_for_required_topic_arn() {
    let mut r = resource(ResourceKind::Topic);
    r.id = "test".into();
    r.arn = "arn:aws:sns:us-east-1:123456789012:test".into();
    assert_eq!(command(&Target::Resource(r), context()).unwrap(),
        "aws sns get-topic-attributes --cli-input-json '{\"TopicArn\":\"arn:aws:sns:us-east-1:123456789012:test\"}' --region='us-east-1' --profile='beta'");
}
