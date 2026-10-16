use super::{component, ConsoleUrl};
use crate::{
    apig, cfn, cw, dynamodb, ecr, efs, fsx, iam, kms, lambda, rds, s3, secretsmanager, sns, sqs,
    vpc,
};

#[test]
fn explicit_host_modes_preserve_global_and_s3_routes() {
    for (region, domain) in [
        ("eu-west-1", "aws.amazon.com"),
        ("cn-north-1", "amazonaws.cn"),
    ] {
        assert_eq!(
            ConsoleUrl::regional(region, "/service/home").finish(),
            format!("https://{region}.console.{domain}/service/home")
        );
        assert_eq!(
            ConsoleUrl::global(region, "/iam/home")
                .fragment("/users")
                .finish(),
            format!("https://console.{domain}/iam/home#/users")
        );
        assert_eq!(
            ConsoleUrl::s3(region, "/s3/buckets/example")
                .query("region", region)
                .finish(),
            format!("https://s3.console.{domain}/s3/buckets/example?region={region}")
        );
    }
}

#[test]
fn query_values_encode_once_in_order_and_always_precede_fragment() {
    let url = ConsoleUrl::regional("us-east-1", "/service/home")
        .fragment("/route?tab=details&view=graph")
        .query("name", "a /+%2F?#&=雪")
        .query("empty", "")
        .query("region", "us-east-1")
        .finish();
    assert_eq!(url, "https://us-east-1.console.aws.amazon.com/service/home?name=a%20%2F%2B%252F%3F%23%26%3D%E9%9B%AA&empty=&region=us-east-1#/route?tab=details&view=graph");
    // A literal %2F must remain a literal %2F after one URL decode.
    assert_eq!(urlencoding::decode(&component("%2F")).unwrap(), "%2F");
}

#[test]
fn region_cannot_supply_url_authority_delimiters() {
    let url = ConsoleUrl::regional("us-east-1/@example.invalid?#", "/service")
        .query("region", "us-east-1/@example.invalid?#")
        .finish();
    let authority = url
        .strip_prefix("https://")
        .unwrap()
        .split('/')
        .next()
        .unwrap();
    assert_eq!(
        authority,
        "us-east-1%2F%40example.invalid%3F%23.console.aws.amazon.com"
    );
    assert!(!authority.contains('@'));
    assert!(url.ends_with("?region=us-east-1%2F%40example.invalid%3F%23"));
}

#[test]
fn all_regional_list_routes_share_host_and_keep_service_fragments() {
    type Case = (fn(&str) -> String, &'static str, &'static str);
    let cases: &[Case] = &[
        (apig::console_url_apis, "/apigateway/main/apis", ""),
        (cfn::console_url_stacks, "/cloudformation/home", "#/stacks"),
        (
            cw::logs::console_url_list,
            "/cloudwatch/home",
            "#logsV2:log-groups",
        ),
        (dynamodb::console_url_tables, "/dynamodbv2/home", "#tables"),
        (
            ecr::console_url_repositories,
            "/ecr/private-registry/repositories",
            "",
        ),
        (efs::console_url_file_systems, "/efs/home", "#/file-systems"),
        (fsx::console_url_file_systems, "/fsx/home", "#file-systems"),
        (iam::console_url_groups, "/iam/home", "#/groups"),
        (kms::console_url_keys, "/kms/home", "#/kms/keys"),
        (
            kms::console_url_aws_managed_keys,
            "/kms/home",
            "#/kms/defaultKeys",
        ),
        (lambda::console_url_functions, "/lambda/home", "#/functions"),
        (
            lambda::console_url_applications,
            "/lambda/home",
            "#/applications",
        ),
        (rds::console_url_instances, "/rds/home", "#databases:"),
        (s3::console_url_buckets, "/s3/buckets", ""),
        (
            secretsmanager::console_url,
            "/secretsmanager/listsecrets",
            "",
        ),
        (|r| sns::console_url(r, None), "/sns/v3/home", "#/topics"),
        (sqs::console_url_queues, "/sqs/v3/home", "#/queues"),
        (vpc::console_url, "/vpcconsole/home", "#vpcs:"),
    ];
    for (region, domain) in [
        ("us-east-1", "aws.amazon.com"),
        ("eu-west-1", "aws.amazon.com"),
        ("cn-north-1", "amazonaws.cn"),
    ] {
        for (build, path, fragment) in cases {
            assert_eq!(
                build(region),
                format!("https://{region}.console.{domain}{path}?region={region}{fragment}")
            );
        }
    }
}

#[test]
fn cloudwatch_logs_and_alarm_fragments_are_not_reencoded() {
    assert_eq!(cw::logs::console_url_stream("eu-west-1", "/aws/lambda/a%name", "2026/10/[v1]a+b"), "https://eu-west-1.console.aws.amazon.com/cloudwatch/home?region=eu-west-1#logsV2:log-groups/log-group/%2Faws%2Flambda%2Fa%25name/log-events/2026%2F10%2F%5Bv1%5Da%2Bb");
    assert_eq!(cw::logs::console_url_detail("us-east-1", "group?#"), "https://us-east-1.console.aws.amazon.com/cloudwatch/home?region=us-east-1#logsV2:log-groups/log-group/group%3F%23");
    let tail = "?~(sortingColumn~'Last*2520state*2520update~sortingDirection~'ASC~timeRange~(startDate~'PT3H~endDate~'PT0S))";
    assert_eq!(cw::alarms::console_url("us-east-1", Some("a /+%#")), format!("https://us-east-1.console.aws.amazon.com/cloudwatch/home?region=us-east-1#alarmsV2:alarm/a%20%2F%2B%25%23{tail}"));
    assert_eq!(cw::alarms::console_url("us-east-1", None), format!("https://us-east-1.console.aws.amazon.com/cloudwatch/home?region=us-east-1#alarmsV2:alarms{tail}"));
}

#[test]
fn insights_query_and_source_encoding_matches_jsurl_grammar() {
    let url = cw::insights::console_url(
        "eu-west-1",
        "123456789012",
        "fields @message, @timestamp\n| limit 5",
        &["/aws/lambda/demo".into(), "second:group".into()],
    );
    assert_eq!(url, "https://eu-west-1.console.aws.amazon.com/cloudwatch/home?region=eu-west-1#logsV2:logs-insights$3FqueryDetail$3D~(end~0~start~-3600~timeType~'RELATIVE~tz~'UTC~unit~'seconds~editorString~'fields*20*40message*2c*20*40timestamp*0a*7c*20limit*205~source~(~'arn*3aaws*3alogs*3aeu-west-1*3a123456789012*3alog-group*3a*2faws*2flambda*2fdemo~'arn*3aaws*3alogs*3aeu-west-1*3a123456789012*3alog-group*3asecond*3agroup)~lang~'CWLI)$26tab$3Dlogs");
    let default = cw::insights::console_url("us-east-1", "123456789012", "", &[]);
    assert!(default.contains("~editorString~'fields*20*40timestamp*2c*20*40message"));
    assert!(default.ends_with("~source~(~)~lang~'CWLI)$26tab$3Dlogs"));
}

#[test]
fn action_routes_keep_list_detail_and_context_behavior_without_opening_browser() {
    use crate::app::{App, Service};
    let mut app = App::new_without_client("test".into(), Some("eu-west-1".into()));
    app.current_service = Service::Ec2Instances;
    assert_eq!(
        app.get_console_url(),
        "https://eu-west-1.console.aws.amazon.com/ec2/home?region=eu-west-1#Instances:"
    );
    app.ec2_state.current_instance = Some("i-123".into());
    assert!(app
        .get_console_url()
        .ends_with("#InstanceDetails:instanceId=i-123"));
    app.config.region = "cn-north-1".into();
    assert_eq!(app.get_console_url(), "https://cn-north-1.console.amazonaws.cn/ec2/home?region=cn-north-1#InstanceDetails:instanceId=i-123");
    app.current_service = Service::SecretsManagerSecrets;
    app.secretsmanager_state.secrets.items = vec![rusticity_core::SecretMetadata {
        name: "selected/secret".into(),
        ..Default::default()
    }];
    assert_eq!(
        app.get_console_url(),
        "https://cn-north-1.console.amazonaws.cn/secretsmanager/listsecrets?region=cn-north-1"
    );
    app.current_service = Service::CliOutput(1);
    assert_eq!(app.get_console_url(), "");
    app.current_service = Service::CloudTrailEvents;
    assert_eq!(
        app.get_console_url(),
        "https://cn-north-1.console.amazonaws.cn/cloudtrail/home?region=cn-north-1#/events"
    );
    app.cloudtrail_state.current_event = Some(crate::cloudtrail::CloudTrailEvent {
        event_id: "event/id+%?#".into(),
        event_name: "Example".into(),
        event_time: String::new(),
        username: String::new(),
        event_source: String::new(),
        resource_type: String::new(),
        resource_name: String::new(),
        read_only: String::new(),
        aws_region: String::new(),
        access_key_id: String::new(),
        source_ip_address: String::new(),
        error_code: String::new(),
        request_id: String::new(),
        event_type: String::new(),
        cloud_trail_event_json: String::new(),
    });
    assert_eq!(app.get_console_url(), "https://cn-north-1.console.amazonaws.cn/cloudtrailv2/home?region=cn-north-1#/events/event%2Fid%2B%25%3F%23");
}

#[test]
fn govcloud_routes_never_use_the_commercial_or_china_console_host() {
    // AWS documents https://console.amazonaws-us-gov.com as the console entry
    // point. Preserve the selected region in the query instead of inventing a
    // regional/service hostname within this partition.
    for region in ["us-gov-east-1", "us-gov-west-1"] {
        assert_eq!(
            ConsoleUrl::regional(region, "/service/home")
                .query("region", region)
                .finish(),
            format!("https://console.amazonaws-us-gov.com/service/home?region={region}")
        );
        assert_eq!(
            iam::console_url_users(region),
            "https://console.amazonaws-us-gov.com/iam/home#/users"
        );
        assert_eq!(s3::console_url_bucket(region, "bucket", "logs/"), format!("https://console.amazonaws-us-gov.com/s3/buckets/bucket?region={region}&prefix=logs%2F"));
        assert_eq!(
            sns::console_url(region, None),
            format!("https://console.amazonaws-us-gov.com/sns/v3/home?region={region}#/topics")
        );
        assert_eq!(
            secretsmanager::console_url(region),
            format!(
                "https://console.amazonaws-us-gov.com/secretsmanager/listsecrets?region={region}"
            )
        );
    }
}
