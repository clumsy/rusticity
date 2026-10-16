use super::*;
use crate::{ecr::image::Image, keymap::handle_key};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, buffer::Buffer, layout::Rect, Terminal};
use rusticity_core::finder::{Resource, ResourceKind};

fn app(service: Service) -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    app.current_service = service;
    app.service_selected = true;
    app.mode = Mode::Normal;
    app.config.account_id = "123456789012".into();
    app
}
fn press(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    if let Some(action) = handle_key(KeyEvent::new(key, modifiers), app.mode) {
        app.handle_action(action);
    }
}
fn copy(app: &mut App) -> String {
    press(app, KeyCode::Char('y'), KeyModifiers::CONTROL);
    let mut copied = String::new();
    assert!(app
        .clipboard
        .complete_with(&Buffer::empty(Rect::new(0, 0, 1, 1)), |text| {
            copied = text;
            Ok(())
        }));
    copied
}
fn screen(app: &App, width: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, 35)).unwrap();
    terminal.draw(|frame| ui::render(frame, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}
fn topic(name: &str) -> rusticity_core::SnsTopic {
    rusticity_core::SnsTopic::from_arn(&format!("arn:aws:sns:us-east-1:123456789012:{name}"))
        .unwrap()
}
fn image(tag: &str, digest: &str) -> Image {
    Image {
        tag: tag.into(),
        digest: digest.into(),
        uri: format!("registry/repo:{tag}"),
        artifact_type: String::new(),
        pushed_at: String::new(),
        last_pull_time: String::new(),
        size_bytes: 0,
    }
}

#[test]
fn ctrl_y_has_its_own_action_and_does_not_type_into_filters() {
    let key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL);
    assert_eq!(handle_key(key, Mode::Normal), Some(Action::CopyCliCommand));
    assert_eq!(
        handle_key(key, Mode::ResourceFinder),
        Some(Action::CopyCliCommand)
    );
    for mode in [
        Mode::FilterInput,
        Mode::ServicePicker,
        Mode::InsightsInput,
        Mode::ProfilePicker,
        Mode::ColumnSelector,
    ] {
        assert_eq!(handle_key(key, mode), None);
    }
    assert_eq!(
        handle_key(
            KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE),
            Mode::Normal
        ),
        Some(Action::Yank)
    );
}

#[test]
fn sns_table_copies_list_command_independent_of_highlighted_row() {
    let mut app = app(Service::SnsTopics);
    app.sns_state.topics.items = vec![topic("other"), topic("chosen")];
    app.sns_state.topics.filter = "CHOSEN".into();
    let before = app.sns_state.topics.items.clone();
    let text = copy(&mut app);
    assert!(text.starts_with("aws sns list-topics "));
    assert!(!text.contains("123456789012:chosen"));
    assert!(!text.contains(":other"));
    assert!(text.contains("--profile='beta'"));
    assert!(text.contains("--region='us-east-1'"));
    assert_eq!(app.sns_state.topics.items, before);
    assert_eq!(app.sns_state.topics.filter, "CHOSEN");
    assert!(!app.sns_state.topics.loading);
    assert!(!app.log_groups_state.loading);
    assert_eq!(app.mode, Mode::Normal);
    app.clipboard = Default::default(); // Dismiss the transient copy notice to inspect hints.
    assert!(screen(&app, 240).contains("^y copy CLI"));
    // Existing resource yank remains an ARN, not a CLI command.
    press(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
    app.clipboard
        .complete_with(&Buffer::empty(Rect::new(0, 0, 1, 1)), |text| {
            assert_eq!(text, "arn:aws:sns:us-east-1:123456789012:chosen");
            Ok(())
        });
}

#[test]
fn finder_copy_targets_focused_result_not_underlying_service() {
    let mut app = app(Service::SnsTopics);
    app.mode = Mode::ResourceFinder;
    app.resource_finder.focus = Focus::Table;
    app.resource_finder.rows = vec![Resource {
        kind: ResourceKind::Instance,
        id: "i-123".into(),
        name: "Friendly".into(),
        arn: String::new(),
        region: "us-east-1".into(),
    }];
    let text = copy(&mut app);
    assert!(text.starts_with("aws ec2 describe-instances "));
    assert!(text.contains("i-123"));
    assert!(app.resource_finder.pending_open.is_none());
    assert_eq!(app.mode, Mode::ResourceFinder);
    app.clipboard = Default::default(); // Dismiss the transient copy notice to inspect hints.
    assert!(screen(&app, 240).contains("^y copy CLI"));
    app.resource_finder.focus = Focus::Filter;
    assert!(!shortcut_visible(&app));
    press(&mut app, KeyCode::Char('y'), KeyModifiers::CONTROL);
    assert!(app.clipboard.notice().unwrap().failed);
    assert!(app.resource_finder.input.is_empty());
}

#[test]
fn unsupported_views_refuse_stale_commands_but_empty_tables_can_be_listed() {
    let mut app = app(Service::ApiGatewayApis);
    app.clipboard.request_text("stale request");
    press(&mut app, KeyCode::Char('y'), KeyModifiers::CONTROL);
    assert!(app.clipboard.notice().unwrap().failed);
    assert!(!app
        .clipboard
        .complete_with(&Buffer::empty(Rect::new(0, 0, 1, 1)), |_| panic!(
            "must not touch clipboard"
        )));
    app.current_service = Service::SnsTopics;
    assert!(copy(&mut app).starts_with("aws sns list-topics "));
    assert!(!app.sns_state.topics.loading);
}

#[test]
fn ecr_image_list_and_open_detail_use_immutable_digest_and_intercept_service_actions() {
    let mut app = app(Service::EcrRepositories);
    app.ecr_state.current_repository = Some("team/repo".into());
    app.ecr_state.images.items = vec![
        image("ignore", "sha256:other"),
        image("chosen", "sha256:correct"),
    ];
    app.ecr_state.images.filter = "chosen".into();
    let list = copy(&mut app);
    assert!(list.starts_with("aws ecr describe-images "));
    assert!(!list.contains("sha256:correct"));
    assert!(list.contains("team/repo"));
    assert!(!list.contains("imageIds"));
    assert!(!app.ecr_state.repositories.loading);
    press(&mut app, KeyCode::Enter, KeyModifiers::NONE);
    app.ecr_state.images.items[1].digest = "sha256:changed".into();
    for op in [
        "describe-images",
        "describe-image-scan-findings",
        "list-image-referrers",
        "describe-image-replication-status",
        "describe-image-signing-status",
    ] {
        let text = copy(&mut app);
        assert!(text.starts_with(&format!("aws ecr {op} ")));
        assert!(text.contains("sha256:correct"));
        assert!(!text.contains("sha256:changed"));
        app.clipboard = Default::default(); // Dismiss the transient copy notice to inspect hints.
        assert!(screen(&app, 100).contains("^y copy CLI"));
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
    }
}

#[test]
fn s3_objects_table_lists_prefix_independent_of_file_or_folder_selection() {
    let mut app = app(Service::S3Buckets);
    app.s3_state.current_bucket = Some("bucket".into());
    app.s3_state.object_tab = ui::s3::ObjectTab::Objects;
    app.s3_state.buckets.items = vec![crate::s3::Bucket {
        name: "bucket".into(),
        region: "eu-west-1".into(),
        creation_date: String::new(),
    }];
    app.s3_state.objects = vec![crate::s3::Object {
        key: "file://literal's $(text)".into(),
        is_prefix: false,
        size: 0,
        last_modified: String::new(),
        storage_class: String::new(),
    }];
    let text = copy(&mut app);
    assert!(text.starts_with("aws s3api list-objects-v2 "));
    assert!(text.contains("--region='eu-west-1'"));
    assert!(!text.contains("file://literal"));
    assert!(!app.s3_state.object_view.busy());
    app.clipboard = Default::default(); // Dismiss the transient copy notice to inspect hints.
    assert!(screen(&app, 100).contains("^y copy CLI"));
    app.s3_state.objects[0].is_prefix = true;
    assert_eq!(copy(&mut app), text);
}

#[test]
fn api_protocol_comes_from_selected_or_open_api() {
    let mut app = app(Service::ApiGatewayApis);
    app.apig_state.apis.items = vec![crate::apig::api::RestApi {
        id: "http-api".into(),
        name: "chosen".into(),
        protocol_type: "HTTP".into(),
        description: String::new(),
        created_date: String::new(),
        api_key_source: String::new(),
        endpoint_configuration: String::new(),
        disable_execute_api_endpoint: false,
        status: String::new(),
    }];
    assert!(invocation(&app).err().unwrap().contains("combines REST"));
    app.apig_state.current_api = Some(app.apig_state.apis.items[0].clone());
    assert!(copy(&mut app).starts_with("aws apigatewayv2 get-routes "));
    app.apig_state.current_api.as_mut().unwrap().protocol_type = "REST".into();
    app.apig_state.current_api.as_mut().unwrap().id = "rest-api".into();
    let text = copy(&mut app);
    assert!(text.starts_with("aws apigateway get-resources "));
    assert!(text.contains("rest-api"));
}

#[test]
fn current_profile_changes_are_reflected_and_foreign_finder_accounts_are_refused() {
    let mut app = app(Service::SnsTopics);
    app.sns_state.topics.items = vec![topic("chosen")];
    assert!(copy(&mut app).contains("--profile='beta'"));
    app.profile = "different".into();
    assert!(copy(&mut app).contains("--profile='different'"));
    app.mode = Mode::ResourceFinder;
    app.resource_finder.focus = Focus::Table;
    app.resource_finder.rows = vec![Resource::from_arn(&topic("chosen").arn).unwrap()];
    app.config.account_id = "999999999999".into();
    press(&mut app, KeyCode::Char('y'), KeyModifiers::CONTROL);
    assert!(app
        .clipboard
        .notice()
        .unwrap()
        .message
        .contains("switch profiles"));
}

#[test]
fn table_commands_exist_without_a_selected_row_and_copy_matches_execution() {
    let cases = [
        (
            Service::StepFunctionsStateMachines,
            "stepfunctions",
            "list-state-machines",
        ),
        (Service::SnsTopics, "sns", "list-topics"),
        (
            Service::SecretsManagerSecrets,
            "secretsmanager",
            "list-secrets",
        ),
        (Service::SqsQueues, "sqs", "list-queues"),
        (Service::RdsInstances, "rds", "describe-db-instances"),
        (Service::VpcVpcs, "ec2", "describe-vpcs"),
        (Service::Ec2Instances, "ec2", "describe-instances"),
        (Service::KmsKeys, "kms", "list-keys"),
        (Service::EfsFileSystems, "efs", "describe-file-systems"),
        (Service::FsxFileSystems, "fsx", "describe-file-systems"),
        (Service::S3Buckets, "s3api", "list-buckets"),
        (Service::EcrRepositories, "ecr", "describe-repositories"),
        (Service::CloudWatchLogGroups, "logs", "describe-log-groups"),
        (Service::CloudWatchAlarms, "cloudwatch", "describe-alarms"),
        (Service::CloudTrailEvents, "cloudtrail", "lookup-events"),
        (Service::DynamoDbTables, "dynamodb", "list-tables"),
        (Service::LambdaFunctions, "lambda", "list-functions"),
        (
            Service::CloudFormationStacks,
            "cloudformation",
            "describe-stacks",
        ),
        (Service::IamUsers, "iam", "list-users"),
        (Service::IamRoles, "iam", "list-roles"),
        (Service::IamUserGroups, "iam", "list-groups"),
    ];
    for (service, cli_service, operation) in cases {
        let mut app = app(service);
        let invocation = invocation(&app).unwrap();
        assert_eq!(
            &invocation.args[..2],
            [cli_service, operation],
            "{service:?}"
        );
        assert_eq!(copy(&mut app), invocation.display);
        press(&mut app, KeyCode::Char('e'), KeyModifiers::CONTROL);
        assert!(matches!(app.current_service, Service::CliOutput(_)));
        // Opening a tab only queues the operation; this test never polls AWS.
    }
    for service in [
        Service::ApiGatewayApis,
        Service::LambdaApplications,
        Service::CloudWatchInsights,
    ] {
        assert!(
            invocation(&app(service)).is_err(),
            "mixed/report view must not choose a row: {service:?}"
        );
    }
}

#[test]
fn sfn_inventory_cli_ignores_selection_and_express_type_until_entered() {
    let mut app = app(Service::StepFunctionsStateMachines);
    app.stepfunctions_state.machines.items = vec![rusticity_core::StateMachineMetadata {
        name: "fast".into(),
        arn: "arn:aws:states:us-east-1:123456789012:stateMachine:fast".into(),
        machine_type: "EXPRESS".into(),
        creation_date: String::new(),
    }];
    let base = copy(&mut app);
    assert!(base.starts_with("aws stepfunctions list-state-machines "));
    app.stepfunctions_state.machines.selected = usize::MAX;
    app.stepfunctions_state.machines.filter = "no rows".into();
    assert_eq!(copy(&mut app), base);
    app.stepfunctions_state.machines.filter.clear();
    app.stepfunctions_state.machines.selected = 0;
    app.handle_action(Action::Select);
    assert!(invocation(&app).err().unwrap().contains("EXPRESS"));
}

#[test]
fn sqs_nested_table_commands_use_entered_url_not_highlighted_row() {
    use crate::sqs::queue::Queue;
    use crate::ui::sqs::QueueDetailTab;
    let mut app = app(Service::SqsQueues);
    let queue = Queue {
        name: "orders".into(),
        url: "https://sqs.us-east-1.amazonaws.com/123456789012/orders".into(),
        queue_type: String::new(),
        created_timestamp: String::new(),
        messages_available: String::new(),
        messages_in_flight: String::new(),
        encryption: String::new(),
        content_based_deduplication: String::new(),
        last_modified_timestamp: String::new(),
        visibility_timeout: String::new(),
        message_retention_period: String::new(),
        maximum_message_size: String::new(),
        delivery_delay: String::new(),
        receive_message_wait_time: String::new(),
        high_throughput_fifo: String::new(),
        deduplication_scope: String::new(),
        fifo_throughput_limit: String::new(),
        dead_letter_queue: String::new(),
        messages_delayed: String::new(),
        redrive_allow_policy: String::new(),
        redrive_policy: String::new(),
        redrive_task_id: String::new(),
        redrive_task_start_time: String::new(),
        redrive_task_status: String::new(),
        redrive_task_percent: String::new(),
        redrive_task_destination: String::new(),
    };
    app.sqs_state.current_queue = Some(queue.url.clone());
    app.sqs_state.queues.items = vec![queue];
    app.sqs_state.queues.selected = usize::MAX;
    app.sqs_state.detail_tab = QueueDetailTab::Tagging;
    let invocation = invocation(&app).unwrap();
    assert_eq!(&invocation.args[..2], ["sqs", "list-queue-tags"]);
    assert!(invocation.args[3].contains("https://sqs.us-east-1.amazonaws.com/123456789012/orders"));
    app.sqs_state.detail_tab = QueueDetailTab::LambdaTriggers;
    let invocation = super::invocation(&app).unwrap();
    assert_eq!(
        &invocation.args[..2],
        ["lambda", "list-event-source-mappings"]
    );
    assert!(invocation.args[3].contains("arn:aws:sqs:us-east-1:123456789012:orders"));
    app.config.account_id = "999999999999".into();
    assert!(super::invocation(&app).is_err());
}
