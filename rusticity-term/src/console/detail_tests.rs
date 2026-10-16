//! End-to-end `App::get_console_url` coverage for the EXPLICIT detail-routing
//! branches across services. These drive real `App` state (the `current_*`
//! selection, the active detail tab, the view mode) and assert the literal URL
//! the dispatcher hands back — never comparing the dispatcher's output to a
//! second call of the same builder. Per-builder encoding is proven in each
//! service's own `console_url_tests`; here we prove the ROUTING picks the right
//! builder and falls back to the list route when the selection is absent.
//!
//! No browser is opened and no AWS request is made: `get_console_url` only
//! assembles a string. Top-level inventories always resolve to their list route
//! (the detail link lives behind an entered `current_*` selection).

use crate::app::{App, Service, ViewMode};

const REGION: &str = "eu-west-1";

fn app() -> App {
    let mut app = App::new_without_client("test".into(), Some(REGION.into()));
    app.config.account_id = "123456789012".into();
    app
}

fn url(app: &App) -> String {
    app.get_console_url()
}

// ── IAM ───────────────────────────────────────────────────────────────────────

mod iam {
    use super::*;
    use crate::ui::iam::{RoleTab, UserTab};

    fn users_app() -> App {
        let mut app = app();
        app.current_service = Service::IamUsers;
        app
    }

    fn roles_app() -> App {
        let mut app = app();
        app.current_service = Service::IamRoles;
        app
    }

    #[test]
    fn users_list_when_no_user_entered() {
        assert_eq!(
            url(&users_app()),
            "https://console.aws.amazon.com/iam/home#/users"
        );
    }

    #[test]
    fn user_detail_links_the_active_tab_section() {
        // Every UserTab maps to its console `section=` fragment value.
        let cases = [
            (UserTab::Permissions, "permissions"),
            (UserTab::Groups, "groups"),
            (UserTab::Tags, "tags"),
            (UserTab::SecurityCredentials, "security_credentials"),
            (UserTab::LastAccessed, "access_advisor"),
        ];
        for (tab, section) in cases {
            let mut app = users_app();
            app.iam_state.current_user = Some("alice".into());
            app.iam_state.user_tab = tab;
            assert_eq!(
                url(&app),
                format!(
                    "https://eu-west-1.console.aws.amazon.com/iam/home?region=eu-west-1#/users/details/alice?section={section}"
                ),
                "user tab {tab:?} must deep-link section={section}"
            );
        }
    }

    #[test]
    fn roles_list_when_no_role_entered() {
        assert_eq!(
            url(&roles_app()),
            "https://console.aws.amazon.com/iam/home#/roles"
        );
    }

    #[test]
    fn role_detail_links_the_active_tab_section() {
        let cases = [
            (RoleTab::Permissions, "permissions"),
            (RoleTab::TrustRelationships, "trust_relationships"),
            (RoleTab::Tags, "tags"),
            (RoleTab::LastAccessed, "access_advisor"),
            (RoleTab::RevokeSessions, "revoke_sessions"),
        ];
        for (tab, section) in cases {
            let mut app = roles_app();
            app.iam_state.current_role = Some("admin".into());
            app.iam_state.role_tab = tab;
            assert_eq!(
                url(&app),
                format!(
                    "https://eu-west-1.console.aws.amazon.com/iam/home?region=eu-west-1#/roles/details/admin?section={section}"
                ),
                "role tab {tab:?} must deep-link section={section}"
            );
        }
    }

    #[test]
    fn role_policy_takes_precedence_over_role_detail_tab() {
        // When a policy is open inside a role, the policy editor route wins over
        // the active role tab.
        let mut app = roles_app();
        app.iam_state.current_role = Some("admin".into());
        app.iam_state.role_tab = RoleTab::Tags;
        app.iam_state.current_policy = Some("AmazonS3FullAccess".into());
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/iam/home?region=eu-west-1#/roles/details/admin/editPolicy/AmazonS3FullAccess?step=addPermissions"
        );
    }

    #[test]
    fn policy_without_role_falls_back_to_roles_list() {
        // A dangling `current_policy` with no `current_role` cannot build a
        // policy link and must fall back to the global roles list, not panic or
        // emit a malformed detail route.
        let mut app = roles_app();
        app.iam_state.current_policy = Some("Orphaned".into());
        assert_eq!(url(&app), "https://console.aws.amazon.com/iam/home#/roles");
    }
}

// ── Lambda ──────────────────────────────────────────────────────────────────

mod lambda {
    use super::*;
    use crate::ui::lambda::DetailTab;

    fn functions_app() -> App {
        let mut app = app();
        app.current_service = Service::LambdaFunctions;
        app
    }

    #[test]
    fn functions_list_when_none_entered() {
        assert_eq!(
            url(&functions_app()),
            "https://eu-west-1.console.aws.amazon.com/lambda/home?region=eu-west-1#/functions"
        );
    }

    #[test]
    fn function_detail_when_entered_without_version() {
        let mut app = functions_app();
        app.lambda_state.current_function = Some("my-fn".into());
        app.lambda_state.detail_tab = DetailTab::Configuration;
        // Without a pinned version the route is the plain function detail; the
        // active tab only matters once a version is pinned.
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/lambda/home?region=eu-west-1#/functions/my-fn"
        );
    }

    #[test]
    fn function_version_links_tab_mapping() {
        // Only Code/Configuration have distinct console ids; the rest collapse
        // to `code`.
        let cases = [
            (DetailTab::Code, "code"),
            (DetailTab::Configuration, "configure"),
            (DetailTab::Monitor, "code"),
            (DetailTab::Aliases, "code"),
            (DetailTab::Versions, "code"),
        ];
        for (tab, want) in cases {
            let mut app = functions_app();
            app.lambda_state.current_function = Some("my-fn".into());
            app.lambda_state.current_version = Some("3".into());
            app.lambda_state.detail_tab = tab;
            assert_eq!(
                url(&app),
                format!(
                    "https://eu-west-1.console.aws.amazon.com/lambda/home?region=eu-west-1#/functions/my-fn/versions/3?tab={want}"
                ),
                "version tab {tab:?} must map to tab={want}"
            );
        }
    }

    #[test]
    fn applications_list_route() {
        let mut app = app();
        app.current_service = Service::LambdaApplications;
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/lambda/home?region=eu-west-1#/applications"
        );
    }
}

// ── CloudFormation ────────────────────────────────────────────────────────────

mod cfn {
    use super::*;
    use crate::cfn::Stack;
    use crate::ui::cfn::DetailTab;

    fn stack(name: &str, stack_id: &str) -> Stack {
        Stack {
            name: name.into(),
            stack_id: stack_id.into(),
            status: String::new(),
            created_time: String::new(),
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
        }
    }

    fn stacks_app() -> App {
        let mut app = app();
        app.current_service = Service::CloudFormationStacks;
        app
    }

    #[test]
    fn stacks_list_when_no_stack_entered() {
        assert_eq!(
            url(&stacks_app()),
            "https://eu-west-1.console.aws.amazon.com/cloudformation/home?region=eu-west-1#/stacks"
        );
    }

    #[test]
    fn stack_detail_links_every_tab() {
        // The detail route keys off `stack_id` (not the name) and carries the
        // tab path as the fragment's leading segment; ChangeSets alone appends
        // `&eventsView=graph`.
        let id = "arn:aws:cloudformation:eu-west-1:123456789012:stack/demo/abc";
        let enc = "arn%3Aaws%3Acloudformation%3Aeu-west-1%3A123456789012%3Astack%2Fdemo%2Fabc";
        let cases = [
            (DetailTab::StackInfo, "stackinfo", ""),
            (DetailTab::Events, "events", ""),
            (DetailTab::Resources, "resources", ""),
            (DetailTab::Outputs, "outputs", ""),
            (DetailTab::Parameters, "parameters", ""),
            (DetailTab::Template, "template", ""),
            (DetailTab::ChangeSets, "changesets", "&eventsView=graph"),
            (DetailTab::GitSync, "gitsync", ""),
        ];
        for (tab, path, extra) in cases {
            let mut app = stacks_app();
            app.cfn_state.current_stack = Some("demo".into());
            app.cfn_state.table.items = vec![stack("demo", id)];
            app.cfn_state.detail_tab = tab;
            assert_eq!(
                url(&app),
                format!(
                    "https://eu-west-1.console.aws.amazon.com/cloudformation/home?region=eu-west-1#/stacks/{path}?filteringText=&filteringStatus=active&viewNested=true&stackId={enc}{extra}"
                ),
                "cfn tab {tab:?} must route to /stacks/{path}"
            );
        }
    }

    #[test]
    fn missing_stack_row_falls_back_to_list() {
        // `current_stack` names a stack that is not in the table (e.g. filtered
        // out): the lookup misses and the route falls back to the list.
        let mut app = stacks_app();
        app.cfn_state.current_stack = Some("ghost".into());
        app.cfn_state.table.items = vec![stack("demo", "id-1")];
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/cloudformation/home?region=eu-west-1#/stacks"
        );
    }
}

// ── S3 / SQS ──────────────────────────────────────────────────────────────────

mod s3_sqs {
    use super::*;

    #[test]
    fn s3_bucket_detail_appends_joined_prefix() {
        let mut app = app();
        app.current_service = Service::S3Buckets;
        app.s3_state.current_bucket = Some("my-bucket".into());
        app.s3_state.prefix_stack = vec!["logs/".into(), "2026/".into()];
        // The bucket route uses S3's own console host; the joined prefix stack
        // ("logs/" + "2026/") is a raw query value, so its slashes encode once.
        assert_eq!(
            url(&app),
            "https://s3.console.aws.amazon.com/s3/buckets/my-bucket?region=eu-west-1&prefix=logs%2F2026%2F"
        );
    }

    #[test]
    fn s3_list_when_no_bucket_entered() {
        let mut app = app();
        app.current_service = Service::S3Buckets;
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/s3/buckets?region=eu-west-1"
        );
    }

    #[test]
    fn sqs_queue_detail_encodes_queue_url_once() {
        let mut app = app();
        app.current_service = Service::SqsQueues;
        app.sqs_state.current_queue =
            Some("https://sqs.eu-west-1.amazonaws.com/123456789012/MyQ".into());
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/sqs/v3/home?region=eu-west-1#/queues/https%3A%2F%2Fsqs.eu-west-1.amazonaws.com%2F123456789012%2FMyQ"
        );
    }

    #[test]
    fn sqs_list_when_no_queue_entered() {
        let mut app = app();
        app.current_service = Service::SqsQueues;
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/sqs/v3/home?region=eu-west-1#/queues"
        );
    }
}

// ── EFS / FsX / DynamoDB ──────────────────────────────────────────────────────

mod storage {
    use super::*;
    use crate::ui::dynamodb::DetailTab as DdbTab;
    use crate::ui::efs::DetailTab as EfsTab;

    #[test]
    fn efs_detail_links_every_tab_id() {
        let cases = [
            (EfsTab::MeteredSize, "meteredSize"),
            (EfsTab::Monitoring, "monitoring"),
            (EfsTab::Tags, "tags"),
            (EfsTab::FileSystemPolicy, "fileSystemPolicy"),
            (EfsTab::AccessPoints, "accessPoints"),
            (EfsTab::Network, "network"),
            (EfsTab::Replication, "replication"),
        ];
        for (tab, tab_id) in cases {
            let mut app = app();
            app.current_service = Service::EfsFileSystems;
            app.efs_state.current_file_system = Some("fs-0a1b".into());
            app.efs_state.detail_tab = tab;
            assert_eq!(
                url(&app),
                format!(
                    "https://eu-west-1.console.aws.amazon.com/efs/home?region=eu-west-1#/file-systems/fs-0a1b?tabId={tab_id}"
                ),
                "efs tab {tab:?} must carry tabId={tab_id}"
            );
        }
    }

    #[test]
    fn efs_list_when_none_entered() {
        let mut app = app();
        app.current_service = Service::EfsFileSystems;
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/efs/home?region=eu-west-1#/file-systems"
        );
    }

    #[test]
    fn fsx_detail_uses_file_system_details_route() {
        // FsX has no tab deep-link; any detail tab resolves to the same route.
        let mut app = app();
        app.current_service = Service::FsxFileSystems;
        app.fsx_state.current_file_system = Some("fs-xyz".into());
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/fsx/home?region=eu-west-1#file-system-details/fs-xyz"
        );
    }

    #[test]
    fn fsx_list_when_none_entered() {
        let mut app = app();
        app.current_service = Service::FsxFileSystems;
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/fsx/home?region=eu-west-1#file-systems"
        );
    }

    #[test]
    fn dynamodb_detail_links_every_console_tab() {
        let cases = [
            (DdbTab::Items, "item-explorer"),
            (DdbTab::Settings, "overview"),
            (DdbTab::Indexes, "indexes"),
            (DdbTab::Monitor, "monitoring"),
            (DdbTab::GlobalTables, "globaltables"),
            (DdbTab::Backups, "backups"),
            (DdbTab::ExportsStreams, "exportsandstreams"),
            (DdbTab::AdditionalSettings, "additionalsettings"),
            (DdbTab::Tags, "tags"),
        ];
        for (tab, console_tab) in cases {
            let mut app = app();
            app.current_service = Service::DynamoDbTables;
            app.dynamodb_state.current_table = Some("orders".into());
            app.dynamodb_state.detail_tab = tab;
            assert_eq!(
                url(&app),
                format!(
                    "https://eu-west-1.console.aws.amazon.com/dynamodbv2/home?region=eu-west-1#table?name=orders&tab={console_tab}"
                ),
                "ddb tab {tab:?} must map to tab={console_tab}"
            );
        }
    }

    #[test]
    fn dynamodb_list_when_none_entered() {
        let mut app = app();
        app.current_service = Service::DynamoDbTables;
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/dynamodbv2/home?region=eu-west-1#tables"
        );
    }
}

// ── ECR ─────────────────────────────────────────────────────────────────────

mod ecr {
    use super::*;

    // NOTE: the image-digest deep link and the public-repository branch are
    // exercised end-to-end (through `handle_action`, which builds the private
    // `scan::View`) in `ecr/actions.rs`'s `console_tests`. Here we only cover
    // the two routing branches reachable from plain `App` state: the list
    // fallback and the private-repository detail (no open image view).

    #[test]
    fn repositories_list_when_none_entered() {
        let mut app = app();
        app.current_service = Service::EcrRepositories;
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/ecr/private-registry/repositories?region=eu-west-1"
        );
    }

    #[test]
    fn private_repository_detail_without_open_image() {
        // A repository is entered but no image view is open and the tab is
        // private: the route is the private-repository detail, built from the
        // active account id and region.
        let mut app = app();
        app.current_service = Service::EcrRepositories;
        app.ecr_state.current_repository = Some("team/image".into());
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/ecr/repositories/private/123456789012/team%2Fimage?region=eu-west-1"
        );
    }
}

// ── CloudWatch Logs ───────────────────────────────────────────────────────────

mod cw_logs {
    use super::*;
    use rusticity_core::{LogGroup, LogStream};

    fn log_group(name: &str) -> LogGroup {
        LogGroup {
            name: name.into(),
            creation_time: None,
            stored_bytes: None,
            retention_days: None,
            log_class: None,
            arn: None,
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

    fn logs_app() -> App {
        let mut app = app();
        app.current_service = Service::CloudWatchLogGroups;
        app
    }

    #[test]
    fn list_in_list_mode() {
        let mut app = logs_app();
        app.view_mode = ViewMode::List;
        app.log_groups_state.log_groups.items = vec![log_group("/aws/lambda/a")];
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/cloudwatch/home?region=eu-west-1#logsV2:log-groups"
        );
    }

    #[test]
    fn detail_mode_links_selected_group() {
        let mut app = logs_app();
        app.view_mode = ViewMode::Detail;
        app.log_groups_state.log_groups.items = vec![log_group("/aws/lambda/a")];
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/cloudwatch/home?region=eu-west-1#logsV2:log-groups/log-group/%2Faws%2Flambda%2Fa"
        );
    }

    #[test]
    fn detail_mode_without_group_falls_back_to_list() {
        // Detail view but no selected group (empty inventory) cannot build the
        // group route and falls back to the list.
        let mut app = logs_app();
        app.view_mode = ViewMode::Detail;
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/cloudwatch/home?region=eu-west-1#logsV2:log-groups"
        );
    }

    #[test]
    fn events_mode_links_group_and_stream() {
        let mut app = logs_app();
        app.view_mode = ViewMode::Events;
        app.log_groups_state.log_groups.items = vec![log_group("/aws/lambda/a")];
        app.log_groups_state.log_streams = vec![log_stream("2026/10/16/[$LATEST]abc")];
        app.log_groups_state.selected_stream = 0;
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/cloudwatch/home?region=eu-west-1#logsV2:log-groups/log-group/%2Faws%2Flambda%2Fa/log-events/2026%2F10%2F16%2F%5B%24LATEST%5Dabc"
        );
    }

    #[test]
    fn events_mode_without_stream_falls_back_to_list() {
        // Events view with a group but no loaded stream falls through to the
        // list route rather than emitting a stream route with a missing stream.
        let mut app = logs_app();
        app.view_mode = ViewMode::Events;
        app.log_groups_state.log_groups.items = vec![log_group("/aws/lambda/a")];
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/cloudwatch/home?region=eu-west-1#logsV2:log-groups"
        );
    }
}

// ── EC2 / CloudTrail ──────────────────────────────────────────────────────────

mod ec2_cloudtrail {
    use super::*;
    use crate::cloudtrail::CloudTrailEvent;

    fn event(id: &str, name: &str) -> CloudTrailEvent {
        CloudTrailEvent {
            event_id: id.into(),
            event_name: name.into(),
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
        }
    }

    #[test]
    fn ec2_list_then_instance_detail() {
        let mut app = app();
        app.current_service = Service::Ec2Instances;
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/ec2/home?region=eu-west-1#Instances:"
        );
        app.ec2_state.current_instance = Some("i-0abc".into());
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/ec2/home?region=eu-west-1#InstanceDetails:instanceId=i-0abc"
        );
    }

    #[test]
    fn cloudtrail_list_then_event_detail() {
        let mut app = app();
        app.current_service = Service::CloudTrailEvents;
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/cloudtrail/home?region=eu-west-1#/events"
        );
        // The event route switches to the `cloudtrailv2` host and encodes the
        // raw event id once.
        app.cloudtrail_state.current_event = Some(event("id/with+raw%?#", "RunInstances"));
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/cloudtrailv2/home?region=eu-west-1#/events/id%2Fwith%2Braw%25%3F%23"
        );
    }
}

// ── SNS ─────────────────────────────────────────────────────────────────────

mod sns {
    use super::*;
    use rusticity_core::sns::SnsSubscription;
    use rusticity_core::SnsTopic;

    const TOPIC_ARN: &str = "arn:aws:sns:eu-west-1:123456789012:my-topic";

    fn topic() -> SnsTopic {
        SnsTopic {
            name: "my-topic".into(),
            arn: TOPIC_ARN.into(),
            topic_type: String::new(),
        }
    }

    fn subscription(arn: &str, status: &str) -> SnsSubscription {
        SnsSubscription {
            subscription_arn: arn.into(),
            topic_arn: TOPIC_ARN.into(),
            protocol: "email".into(),
            endpoint: "a@example.com".into(),
            owner: "123456789012".into(),
            status: status.into(),
        }
    }

    fn sns_app() -> App {
        let mut app = app();
        app.current_service = Service::SnsTopics;
        app.service_selected = true;
        app
    }

    #[test]
    fn topics_list_when_no_topic_entered() {
        // Esc from the subscriptions view clears `topic`; the inventory then
        // always opens the topics list regardless of the highlighted row.
        assert_eq!(
            url(&sns_app()),
            "https://eu-west-1.console.aws.amazon.com/sns/v3/home?region=eu-west-1#/topics"
        );
    }

    #[test]
    fn confirmed_subscription_deep_links_subscription_arn() {
        let mut app = sns_app();
        app.sns_state.subscriptions.topic = Some(topic());
        let sub_arn = format!("{TOPIC_ARN}:1234abcd-5678");
        app.sns_state.subscriptions.table.items = vec![subscription(&sub_arn, "Confirmed")];
        app.sns_state.subscriptions.table.selected = 0;
        assert_eq!(
            url(&app),
            format!(
                "https://eu-west-1.console.aws.amazon.com/sns/v3/home?region=eu-west-1#/subscription/{}",
                "arn%3Aaws%3Asns%3Aeu-west-1%3A123456789012%3Amy-topic%3A1234abcd-5678"
            )
        );
    }

    #[test]
    fn pending_subscription_links_the_topic_not_the_subscription() {
        // A non-Confirmed selection is never presented as a subscription deep
        // link; the route is the enclosing topic detail.
        let mut app = sns_app();
        app.sns_state.subscriptions.topic = Some(topic());
        app.sns_state.subscriptions.table.items =
            vec![subscription("PendingConfirmation", "PendingConfirmation")];
        app.sns_state.subscriptions.table.selected = 0;
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/sns/v3/home?region=eu-west-1#/topic/arn%3Aaws%3Asns%3Aeu-west-1%3A123456789012%3Amy-topic"
        );
    }

    #[test]
    fn empty_subscription_list_links_the_topic() {
        // Entered a topic whose subscriptions are empty: no selection to deep
        // link, so the topic detail route is used.
        let mut app = sns_app();
        app.sns_state.subscriptions.topic = Some(topic());
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/sns/v3/home?region=eu-west-1#/topic/arn%3Aaws%3Asns%3Aeu-west-1%3A123456789012%3Amy-topic"
        );
    }

    #[test]
    fn out_of_range_selection_links_the_topic() {
        // A selected index past the end of the list resolves to no selected
        // subscription and must fall back to the topic detail, not panic.
        let mut app = sns_app();
        app.sns_state.subscriptions.topic = Some(topic());
        let sub_arn = format!("{TOPIC_ARN}:confirmed-1");
        app.sns_state.subscriptions.table.items = vec![subscription(&sub_arn, "Confirmed")];
        app.sns_state.subscriptions.table.selected = 7;
        assert_eq!(
            url(&app),
            "https://eu-west-1.console.aws.amazon.com/sns/v3/home?region=eu-west-1#/topic/arn%3Aaws%3Asns%3Aeu-west-1%3A123456789012%3Amy-topic"
        );
    }
}
