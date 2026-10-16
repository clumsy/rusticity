//! Regression tests for `App::get_console_url` at the top-level inventory level.
//!
//! Invariant under test: for a top-level inventory view (no resource "entered"),
//! Ctrl+O opens that service's regional *list* page, and that URL is independent
//! of which row is highlighted, what filter string is active, whether a row is
//! expanded, whether the table is empty, and whether `selected` is out of range.
//!
//! The six services the parent fixed (CloudWatch Alarms, DynamoDB, EFS, FSx, KMS,
//! RDS) are exercised with non-empty fixture rows plus selection/filter/expansion
//! mutations; SNS was fixed earlier and is covered the same way. Every other
//! selectable `Service` variant is covered for list routing as well, so adding a
//! new inventory that mis-routes Ctrl+O will trip a test here.
//!
//! URLs are asserted as *literals*, not by re-calling the same builder the
//! production code calls — a builder bug would otherwise be invisible.

use crate::app::{App, Service, ViewMode};
use crate::table::TableState;

const R1: &str = "us-east-1"; // default region under test
const R2: &str = "eu-west-1"; // non-default region, to catch region hardcoding

fn app(region: &str) -> App {
    App::new_without_client("test".to_string(), Some(region.to_string()))
}

/// Replace a table's rows, drop `selected` somewhere inside (or past) them, apply
/// a filter string and optionally expand a row. This is the exact state a user
/// can be in when they press Ctrl+O over a populated, filtered, expanded list.
fn seed<T>(
    table: &mut TableState<T>,
    rows: Vec<T>,
    selected: usize,
    filter: &str,
    expanded: Option<usize>,
) {
    table.items = rows;
    table.selected = selected;
    table.filter = filter.to_string();
    table.expanded_item = expanded;
}

// ── Minimal row fixtures for the two resource types lacking `Default` ──────────

fn efs_row(id: &str) -> crate::efs::fs::FileSystem {
    crate::efs::fs::FileSystem {
        file_system_id: id.to_string(),
        file_system_arn: String::new(),
        name: String::new(),
        creation_token: String::new(),
        encrypted: String::new(),
        kms_key_id: String::new(),
        total_size: String::new(),
        size_in_standard: String::new(),
        size_in_ia: String::new(),
        size_in_archive: String::new(),
        provisioned_throughput: String::new(),
        throughput_mode: String::new(),
        life_cycle_state: String::new(),
        number_of_mount_targets: String::new(),
        owner_id: String::new(),
        creation_time: String::new(),
        performance_mode: String::new(),
        availability_zone: String::new(),
        replication_overwrite_protection: String::new(),
        total_size_bytes: 0,
        size_in_standard_bytes: 0,
        size_in_ia_bytes: 0,
        size_in_archive_bytes: 0,
        dns_name: String::new(),
        tags: Vec::new(),
    }
}

fn ecr_row(name: &str) -> crate::ecr::repo::Repository {
    crate::ecr::repo::Repository {
        name: name.to_string(),
        uri: String::new(),
        created_at: String::new(),
        tag_immutability: String::new(),
        encryption_type: String::new(),
    }
}

fn kms_row(id: &str, manager: &str) -> crate::kms::key::Key {
    crate::kms::key::Key {
        key_id: id.to_string(),
        key_arn: String::new(),
        description: String::new(),
        key_state: String::new(),
        key_usage: String::new(),
        key_spec: String::new(),
        key_manager: manager.to_string(),
        creation_date: String::new(),
        expiration_date: String::new(),
        deletion_date: String::new(),
        custom_key_store_id: String::new(),
        origin: String::new(),
        enabled: true,
        alias: String::new(),
        multi_region: false,
    }
}

// ── Expected literal list URLs, parameterized by region ────────────────────────

fn expected_list_url(service: Service, region: &str) -> String {
    match service {
        Service::CloudWatchLogGroups => format!(
            "https://{region}.console.aws.amazon.com/cloudwatch/home?region={region}#logsV2:log-groups"
        ),
        Service::CloudWatchAlarms => format!(
            "https://{region}.console.aws.amazon.com/cloudwatch/home?region={region}#alarmsV2:alarms?~(sortingColumn~'Last*2520state*2520update~sortingDirection~'ASC~timeRange~(startDate~'PT3H~endDate~'PT0S))"
        ),
        Service::RdsInstances => format!(
            "https://{region}.console.aws.amazon.com/rds/home?region={region}#databases:"
        ),
        Service::S3Buckets => format!(
            "https://{region}.console.aws.amazon.com/s3/buckets?region={region}"
        ),
        Service::SnsTopics => format!(
            "https://{region}.console.aws.amazon.com/sns/v3/home?region={region}#/topics"
        ),
        Service::StepFunctionsStateMachines => format!("https://{region}.console.aws.amazon.com/states/home?region={region}#/statemachines"),
        Service::SecretsManagerSecrets => format!(
            "https://{region}.console.aws.amazon.com/secretsmanager/listsecrets?region={region}"
        ),
        Service::SqsQueues => format!(
            "https://{region}.console.aws.amazon.com/sqs/v3/home?region={region}#/queues"
        ),
        Service::VpcVpcs => format!(
            "https://{region}.console.aws.amazon.com/vpcconsole/home?region={region}#vpcs:"
        ),
        Service::EcrRepositories => format!(
            "https://{region}.console.aws.amazon.com/ecr/private-registry/repositories?region={region}"
        ),
        Service::EfsFileSystems => format!(
            "https://{region}.console.aws.amazon.com/efs/home?region={region}#/file-systems"
        ),
        Service::FsxFileSystems => format!(
            "https://{region}.console.aws.amazon.com/fsx/home?region={region}#file-systems"
        ),
        Service::DynamoDbTables => format!(
            "https://{region}.console.aws.amazon.com/dynamodbv2/home?region={region}#tables"
        ),
        Service::LambdaFunctions => format!(
            "https://{region}.console.aws.amazon.com/lambda/home?region={region}#/functions"
        ),
        Service::LambdaApplications => format!(
            "https://{region}.console.aws.amazon.com/lambda/home?region={region}#/applications"
        ),
        Service::CloudFormationStacks => format!(
            "https://{region}.console.aws.amazon.com/cloudformation/home?region={region}#/stacks"
        ),
        Service::ApiGatewayApis => format!(
            "https://{region}.console.aws.amazon.com/apigateway/main/apis?region={region}"
        ),
        // IAM Users/Roles list routes are global: no regional host prefix and
        // no region query. Groups is the exception — it is a regional route.
        Service::IamUsers => "https://console.aws.amazon.com/iam/home#/users".to_string(),
        Service::IamRoles => "https://console.aws.amazon.com/iam/home#/roles".to_string(),
        Service::IamUserGroups => format!(
            "https://{region}.console.aws.amazon.com/iam/home?region={region}#/groups"
        ),
        Service::Ec2Instances => format!("https://{region}.console.aws.amazon.com/ec2/home?region={region}#Instances:"),
        Service::CloudTrailEvents => format!("https://{region}.console.aws.amazon.com/cloudtrail/home?region={region}#/events"),
        Service::KmsKeys => format!("https://{region}.console.aws.amazon.com/kms/home?region={region}#/kms/keys"),
        Service::CliOutput(_) => String::new(),
        // This isn't an inventory list; its query/source fragment has dedicated tests.
        Service::CloudWatchInsights => panic!("Insights uses the dedicated fragment tests"),
    }
}

/// Services whose top-level `get_console_url` has *no* detail/entered guard at
/// all: the dispatcher returns the list URL unconditionally, so seeding rows and
/// mutating selection/filter can only confirm it. These are driven generically.
fn drive_simple(service: Service, region: &str) {
    let mut app = app(region);
    app.current_service = service;
    app.service_selected = true;
    let expected = expected_list_url(service, region);

    // Empty + default selection.
    assert_eq!(app.get_console_url(), expected, "{service:?} empty");

    // Out-of-range selection on an empty table.
    app.set_selected_generic(service, 999);
    assert_eq!(
        app.get_console_url(),
        expected,
        "{service:?} empty, selected OOB"
    );
}

// Helper on App to mutate `selected` for services driven generically. Kept here
// (test-only) rather than reaching into each state module from the match arms.
impl App {
    fn set_selected_generic(&mut self, service: Service, selected: usize) {
        match service {
            Service::RdsInstances => self.rds_state.instances.selected = selected,
            Service::VpcVpcs => self.vpc_state.vpcs.selected = selected,
            Service::SecretsManagerSecrets => self.secretsmanager_state.secrets.selected = selected,
            Service::SqsQueues => self.sqs_state.queues.selected = selected,
            Service::S3Buckets => self.s3_state.buckets.selected = selected,
            Service::LambdaFunctions => self.lambda_state.table.selected = selected,
            Service::LambdaApplications => self.lambda_application_state.table.selected = selected,
            Service::CloudFormationStacks => self.cfn_state.table.selected = selected,
            Service::ApiGatewayApis => self.apig_state.apis.selected = selected,
            Service::IamUsers => self.iam_state.users.selected = selected,
            Service::IamRoles => self.iam_state.roles.selected = selected,
            Service::IamUserGroups => self.iam_state.groups.selected = selected,
            Service::CloudWatchLogGroups => self.log_groups_state.log_groups.selected = selected,
            _ => {}
        }
    }
}

// ── The six offenders + SNS: seed rows, then mutate selection/filter/expansion ──

#[test]
fn alarms_list_url_is_invariant_to_row_state() {
    for region in [R1, R2] {
        let mut app = app(region);
        app.current_service = Service::CloudWatchAlarms;
        app.service_selected = true;
        app.view_mode = ViewMode::List;
        // current_alarm is the only thing the dispatcher reads; leave it None.
        assert!(app.alarms_state.current_alarm.is_none());
        let expected = expected_list_url(Service::CloudWatchAlarms, region);

        assert_eq!(app.get_console_url(), expected, "alarms empty");

        let rows = vec![
            crate::cw::alarms::Alarm {
                name: "a".into(),
                ..Default::default()
            },
            crate::cw::alarms::Alarm {
                name: "b".into(),
                ..Default::default()
            },
            crate::cw::alarms::Alarm {
                name: "c".into(),
                ..Default::default()
            },
        ];
        seed(&mut app.alarms_state.table, rows, 1, "b", Some(1));
        assert_eq!(
            app.get_console_url(),
            expected,
            "alarms selected=1 filter=b expanded=1"
        );

        app.alarms_state.table.selected = 999; // out of range
        app.alarms_state.table.expanded_item = None;
        app.alarms_state.table.filter = String::new();
        assert_eq!(app.get_console_url(), expected, "alarms selected OOB");
    }
}

#[test]
fn dynamodb_list_url_is_invariant_to_row_state() {
    for region in [R1, R2] {
        let mut app = app(region);
        app.current_service = Service::DynamoDbTables;
        app.service_selected = true;
        assert!(app.dynamodb_state.current_table.is_none());
        let expected = expected_list_url(Service::DynamoDbTables, region);

        assert_eq!(app.get_console_url(), expected, "ddb empty");

        let rows = vec![
            rusticity_core::dynamodb::DdbTable {
                name: "orders".into(),
                ..Default::default()
            },
            rusticity_core::dynamodb::DdbTable {
                name: "users".into(),
                ..Default::default()
            },
        ];
        seed(&mut app.dynamodb_state.tables, rows, 1, "users", Some(0));
        assert_eq!(
            app.get_console_url(),
            expected,
            "ddb selected=1 filter=users expanded=0"
        );

        app.dynamodb_state.tables.selected = 42; // out of range
        assert_eq!(app.get_console_url(), expected, "ddb selected OOB");
    }
}

#[test]
fn efs_list_url_is_invariant_to_row_state() {
    for region in [R1, R2] {
        let mut app = app(region);
        app.current_service = Service::EfsFileSystems;
        app.service_selected = true;
        assert!(app.efs_state.current_file_system.is_none());
        let expected = expected_list_url(Service::EfsFileSystems, region);

        assert_eq!(app.get_console_url(), expected, "efs empty");

        let rows = vec![efs_row("fs-1"), efs_row("fs-2"), efs_row("fs-3")];
        seed(&mut app.efs_state.file_systems, rows, 2, "fs-", Some(2));
        assert_eq!(
            app.get_console_url(),
            expected,
            "efs selected=2 filter expanded=2"
        );

        app.efs_state.file_systems.selected = 100; // out of range
        assert_eq!(app.get_console_url(), expected, "efs selected OOB");
    }
}

#[test]
fn fsx_list_url_is_invariant_to_row_state() {
    for region in [R1, R2] {
        let mut app = app(region);
        app.current_service = Service::FsxFileSystems;
        app.service_selected = true;
        assert!(app.fsx_state.current_file_system.is_none());
        let expected = expected_list_url(Service::FsxFileSystems, region);

        assert_eq!(app.get_console_url(), expected, "fsx empty");

        let rows = vec![
            rusticity_core::fsx::FsxFileSystem {
                file_system_id: "fs-a".into(),
                ..Default::default()
            },
            rusticity_core::fsx::FsxFileSystem {
                file_system_id: "fs-b".into(),
                ..Default::default()
            },
        ];
        seed(&mut app.fsx_state.file_systems, rows, 1, "fs-b", Some(1));
        assert_eq!(
            app.get_console_url(),
            expected,
            "fsx selected=1 filter expanded=1"
        );

        app.fsx_state.file_systems.selected = 77; // out of range
        assert_eq!(app.get_console_url(), expected, "fsx selected OOB");
    }
}

#[test]
fn kms_list_url_is_invariant_to_row_state_and_differs_per_tab() {
    use crate::app::KmsTab;
    for region in [R1, R2] {
        let mut app = app(region);
        app.current_service = Service::KmsKeys;
        app.service_selected = true;

        let rows = vec![kms_row("aws-key", "AWS"), kms_row("cust-key", "CUSTOMER")];
        seed(&mut app.kms_state.keys, rows, 1, "key", Some(0));

        // Customer-managed tab → #/kms/keys
        app.kms_state.tab = KmsTab::CustomerManaged;
        let customer =
            format!("https://{region}.console.aws.amazon.com/kms/home?region={region}#/kms/keys");
        assert_eq!(app.get_console_url(), customer, "kms customer list");

        // AWS-managed tab → #/kms/defaultKeys (a DIFFERENT list route)
        app.kms_state.tab = KmsTab::AwsManaged;
        let aws_managed = format!(
            "https://{region}.console.aws.amazon.com/kms/home?region={region}#/kms/defaultKeys"
        );
        assert_eq!(app.get_console_url(), aws_managed, "kms aws-managed list");
        assert_ne!(
            customer, aws_managed,
            "KMS tabs must route to distinct list pages"
        );

        // Out-of-range selection does not change the tab's list URL.
        app.kms_state.keys.selected = 999;
        assert_eq!(
            app.get_console_url(),
            aws_managed,
            "kms selected OOB (aws tab)"
        );
    }
}

#[test]
fn rds_list_url_is_invariant_to_row_state() {
    for region in [R1, R2] {
        let mut app = app(region);
        app.current_service = Service::RdsInstances;
        app.service_selected = true;
        let expected = expected_list_url(Service::RdsInstances, region);

        assert_eq!(app.get_console_url(), expected, "rds empty");

        let rows = vec![
            rusticity_core::rds::RdsInstance {
                identifier: "db-1".into(),
                ..Default::default()
            },
            rusticity_core::rds::RdsInstance {
                identifier: "db-2".into(),
                ..Default::default()
            },
        ];
        seed(&mut app.rds_state.instances, rows, 1, "db-2", Some(1));
        assert_eq!(
            app.get_console_url(),
            expected,
            "rds selected=1 filter expanded=1"
        );

        app.rds_state.instances.selected = 55; // out of range
        assert_eq!(app.get_console_url(), expected, "rds selected OOB");
    }
}

#[test]
fn sns_list_url_is_invariant_to_row_state() {
    for region in [R1, R2] {
        let mut app = app(region);
        app.current_service = Service::SnsTopics;
        app.service_selected = true;
        // Entering a topic sets subscriptions.topic; leave it None for the list.
        assert!(app.sns_state.subscriptions.topic.is_none());
        let expected = expected_list_url(Service::SnsTopics, region);

        assert_eq!(app.get_console_url(), expected, "sns empty");

        app.sns_state.topics.selected = 3;
        app.sns_state.topics.filter = "prod".to_string();
        app.sns_state.topics.expanded_item = Some(0);
        assert_eq!(
            app.get_console_url(),
            expected,
            "sns selected/filter/expanded"
        );

        app.sns_state.topics.selected = 999; // out of range
        assert_eq!(app.get_console_url(), expected, "sns selected OOB");
    }
}

// ── VPC / Secrets: no detail guard, exercise rows + mutation explicitly ─────────

#[test]
fn vpc_list_url_is_invariant_to_row_state() {
    for region in [R1, R2] {
        let mut app = app(region);
        app.current_service = Service::VpcVpcs;
        app.service_selected = true;
        let expected = expected_list_url(Service::VpcVpcs, region);

        let rows = vec![
            rusticity_core::vpc::Vpc {
                vpc_id: "vpc-1".into(),
                ..Default::default()
            },
            rusticity_core::vpc::Vpc {
                vpc_id: "vpc-2".into(),
                ..Default::default()
            },
        ];
        seed(&mut app.vpc_state.vpcs, rows, 1, "vpc-2", Some(1));
        assert_eq!(
            app.get_console_url(),
            expected,
            "vpc selected/filter/expanded"
        );

        app.vpc_state.vpcs.selected = 99;
        assert_eq!(app.get_console_url(), expected, "vpc selected OOB");
    }
}

#[test]
fn secrets_list_url_is_invariant_to_row_state() {
    for region in [R1, R2] {
        let mut app = app(region);
        app.current_service = Service::SecretsManagerSecrets;
        app.service_selected = true;
        let expected = expected_list_url(Service::SecretsManagerSecrets, region);

        let rows = vec![
            rusticity_core::SecretMetadata {
                name: "db/pw".into(),
                ..Default::default()
            },
            rusticity_core::SecretMetadata {
                name: "api/key".into(),
                ..Default::default()
            },
        ];
        seed(
            &mut app.secretsmanager_state.secrets,
            rows,
            1,
            "api",
            Some(0),
        );
        assert_eq!(
            app.get_console_url(),
            expected,
            "secrets selected/filter/expanded"
        );

        app.secretsmanager_state.secrets.selected = 88;
        assert_eq!(app.get_console_url(), expected, "secrets selected OOB");
    }
}

// ── ECR: public inventory must not force the private registry section ───────────

#[test]
fn ecr_top_level_list_url_follows_tab_not_selected_row() {
    use crate::app::EcrTab;
    for region in [R1, R2] {
        let mut app = app(region);
        app.current_service = Service::EcrRepositories;
        app.service_selected = true;
        assert!(app.ecr_state.current_repository.is_none());
        let expected = expected_list_url(Service::EcrRepositories, region);

        let rows = vec![ecr_row("repo-a"), ecr_row("repo-b")];
        seed(&mut app.ecr_state.repositories, rows, 1, "repo", Some(1));
        assert_eq!(app.get_console_url(), expected, "ecr private list, seeded");

        app.ecr_state.tab = EcrTab::Public;
        let expected =
            format!("https://{region}.console.aws.amazon.com/ecr/repositories?region={region}");
        assert_eq!(
            app.get_console_url(),
            expected,
            "public inventory uses the documented ECR repository entry point"
        );
        app.ecr_state.repositories.selected = 123;
        assert_eq!(app.get_console_url(), expected, "ecr selected OOB");
    }
}

// ── Every remaining selectable inventory: list routing, both regions ────────────

#[test]
fn simple_inventories_route_to_list_in_both_regions() {
    let services = [
        Service::CloudWatchLogGroups,
        Service::S3Buckets,
        Service::SqsQueues,
        Service::LambdaFunctions,
        Service::LambdaApplications,
        Service::CloudFormationStacks,
        Service::ApiGatewayApis,
        Service::IamUsers,
        Service::IamRoles,
        Service::IamUserGroups,
    ];
    for service in services {
        for region in [R1, R2] {
            drive_simple(service, region);
        }
    }
}

// ── CloudWatch Logs: list only when not in Detail/Events view ───────────────────

#[test]
fn cloudwatch_logs_list_url_when_not_in_detail_view() {
    for region in [R1, R2] {
        let mut app = app(region);
        app.current_service = Service::CloudWatchLogGroups;
        app.service_selected = true;
        app.view_mode = ViewMode::List;
        let expected = expected_list_url(Service::CloudWatchLogGroups, region);

        assert_eq!(app.get_console_url(), expected, "cw logs list, empty");

        app.log_groups_state.log_groups.selected = 5;
        app.log_groups_state.log_groups.filter = "/aws/lambda".to_string();
        app.log_groups_state.log_groups.expanded_item = Some(0);
        assert_eq!(
            app.get_console_url(),
            expected,
            "cw logs list, selected/filter/expanded"
        );
    }
}

// ── CloudWatch Insights: always its bespoke fragment; assert prefix + suffix ────

#[test]
fn cloudwatch_insights_url_has_expected_fragment_shape() {
    for region in [R1, R2] {
        let mut app = app(region);
        app.current_service = Service::CloudWatchInsights;
        app.service_selected = true;
        let url = app.get_console_url();

        let host_prefix = format!(
            "https://{region}.console.aws.amazon.com/cloudwatch/home?region={region}#logsV2:logs-insights"
        );
        assert!(
            url.starts_with(&host_prefix),
            "insights host/route prefix: {url}"
        );
        // Default (empty query) editor string and trailing tab marker.
        assert!(
            url.contains("editorString~'fields"),
            "insights default editor string: {url}"
        );
        assert!(
            url.ends_with("~lang~'CWLI)$26tab$3Dlogs"),
            "insights trailing fragment: {url}"
        );
    }
}

// ── CLI output tab: never emits a console URL ──────────────────────────────────

#[test]
fn cli_output_has_no_console_url() {
    let mut app = app(R1);
    app.current_service = Service::CliOutput(7);
    assert_eq!(app.get_console_url(), "", "CLI output must produce no URL");
}

#[test]
fn list_route_matrix_covers_every_service_picker_entry() {
    // Keep this match exhaustive above and this roster checked against the
    // picker: a newly added service cannot silently escape routing coverage.
    let services = [
        Service::ApiGatewayApis,
        Service::CloudWatchLogGroups,
        Service::CloudWatchInsights,
        Service::CloudWatchAlarms,
        Service::CloudTrailEvents,
        Service::RdsInstances,
        Service::S3Buckets,
        Service::SnsTopics,
        Service::SecretsManagerSecrets,
        Service::StepFunctionsStateMachines,
        Service::SqsQueues,
        Service::VpcVpcs,
        Service::Ec2Instances,
        Service::EcrRepositories,
        Service::KmsKeys,
        Service::EfsFileSystems,
        Service::FsxFileSystems,
        Service::DynamoDbTables,
        Service::LambdaFunctions,
        Service::LambdaApplications,
        Service::CloudFormationStacks,
        Service::IamUsers,
        Service::IamRoles,
        Service::IamUserGroups,
    ];
    let mut names = services
        .iter()
        .map(|service| service.name())
        .collect::<Vec<_>>();
    names.sort();
    let app = app(R1);
    let mut picker = app.service_picker.services.clone();
    picker.sort();
    assert_eq!(names, picker);
    for region in [R1, R2] {
        let mut app = self::app(region);
        app.kms_state.tab = crate::ui::kms::Tab::CustomerManaged;
        for service in services {
            app.current_service = service;
            if service == Service::CloudWatchInsights {
                assert!(app.get_console_url().contains("#logsV2:logs-insights"));
            } else {
                assert_eq!(
                    app.get_console_url(),
                    expected_list_url(service, region),
                    "{service:?}"
                );
            }
        }
    }
}
