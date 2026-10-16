//! View-scoped resolver tests. Each asserts the OPERATION and raw JSON INPUT the
//! current table view resolves to, that the result is invariant to the selected
//! row and to clearing the filter (the command follows the VIEW, never the row),
//! and that detail panes resolve to `None` while unrepresentable aggregate tables
//! resolve to `Err`. These inspect `Target::View` fields directly; full `App`
//! wiring (region/profile rendering) is the parent's concern.
use super::*;
use crate::app::{App, Service};
use crate::keymap::Action;
use rusticity_core::finder::ResourceKind;

fn app(service: Service) -> App {
    let mut app = App::new_without_client("beta".into(), Some("us-east-1".into()));
    app.current_service = service;
    app.service_selected = true;
    app.mode = crate::keymap::Mode::Normal;
    app.config.account_id = "123456789012".into();
    app
}

/// Unwrap a `Some(Target::View { .. })` into its `(operation, input)` for
/// assertions. Panics on `None`/`Err`/other variants — the caller wants a view.
/// Describe a result without requiring `Target: Debug` (the enum intentionally
/// derives none). Used only for panic messages on an unexpected shape.
fn shape(result: &Result<Option<Target>, String>) -> String {
    match result {
        Ok(Some(Target::View { .. })) => "Ok(Some(View))".into(),
        Ok(Some(_)) => "Ok(Some(non-View Target))".into(),
        Ok(None) => "Ok(None)".into(),
        Err(e) => format!("Err({e})"),
    }
}

fn view_of(result: &Result<Option<Target>, String>) -> (&'static str, serde_json::Value) {
    match result {
        Ok(Some(Target::View {
            operation, input, ..
        })) => (operation, input.clone()),
        other => panic!("expected Target::View, got {}", shape(other)),
    }
}

fn region_of(result: &Result<Option<Target>, String>) -> Option<String> {
    match result {
        Ok(Some(Target::View { region, .. })) => region.clone(),
        other => panic!("expected Target::View, got {}", shape(other)),
    }
}

fn scope_of(result: &Result<Option<Target>, String>) -> Option<rusticity_core::finder::Resource> {
    match result {
        Ok(Some(Target::View { scope, .. })) => scope.clone(),
        other => panic!("expected Target::View, got {}", shape(other)),
    }
}

// ── Step Functions ──────────────────────────────────────────────────────────

fn machine(name: &str, machine_type: &str) -> rusticity_core::StateMachineMetadata {
    rusticity_core::StateMachineMetadata {
        name: name.into(),
        arn: format!("arn:aws:states:us-east-1:123456789012:stateMachine:{name}"),
        machine_type: machine_type.into(),
        creation_date: String::new(),
    }
}

#[test]
fn sfn_top_inventory_lists_state_machines_regardless_of_selection_or_filter() {
    let mut app = app(Service::StepFunctionsStateMachines);
    app.stepfunctions_state.machines.items =
        vec![machine("alpha", "STANDARD"), machine("beta", "EXPRESS")];
    let baseline = target(&app);
    let (op, input) = view_of(&baseline);
    assert_eq!(op, "stepfunctions list-state-machines");
    assert_eq!(input, serde_json::json!({}));
    assert_eq!(region_of(&baseline), None);

    // Selecting a different row (incl. an EXPRESS one) and clearing the filter
    // must not change the collection command.
    app.stepfunctions_state.machines.selected = 1;
    app.stepfunctions_state.machines.filter = "nomatch".into();
    let (op2, input2) = {
        let r = target(&app);
        view_of(&r)
    };
    assert_eq!(op2, "stepfunctions list-state-machines");
    assert_eq!(input2, serde_json::json!({}));
    app.stepfunctions_state.machines.filter.clear();
    assert_eq!(
        view_of(&target(&app)).0,
        "stepfunctions list-state-machines"
    );
}

#[test]
fn sfn_entered_standard_machine_lists_executions_scoped_to_parent_arn() {
    let mut app = app(Service::StepFunctionsStateMachines);
    app.stepfunctions_state.machines.items = vec![machine("orders", "STANDARD")];
    app.handle_action(Action::Select);
    let result = target(&app);
    let (op, input) = view_of(&result);
    assert_eq!(op, "stepfunctions list-executions");
    assert_eq!(
        input,
        serde_json::json!({
            "stateMachineArn": "arn:aws:states:us-east-1:123456789012:stateMachine:orders"
        })
    );
    let scope = scope_of(&result).expect("parent machine scope");
    assert_eq!(scope.kind, ResourceKind::StateMachine);
    assert_eq!(
        scope.arn,
        "arn:aws:states:us-east-1:123456789012:stateMachine:orders"
    );
}

#[test]
fn sfn_entered_express_machine_is_an_explicit_error_not_a_row_fallthrough() {
    let mut app = app(Service::StepFunctionsStateMachines);
    app.stepfunctions_state.machines.items = vec![machine("stream", "EXPRESS")];
    app.handle_action(Action::Select);
    let err = target(&app).unwrap_err();
    assert!(err.contains("EXPRESS"), "message: {err}");
}

// ── SNS ───────────────────────────────────────────────────────────────────────

fn topic(name: &str) -> rusticity_core::SnsTopic {
    rusticity_core::SnsTopic::from_arn(&format!("arn:aws:sns:us-east-1:123456789012:{name}"))
        .unwrap()
}

#[test]
fn sns_top_inventory_lists_topics() {
    let mut app = app(Service::SnsTopics);
    app.sns_state.topics.items = vec![topic("a"), topic("b")];
    app.sns_state.topics.filter = "zzz".into();
    app.sns_state.topics.selected = 1;
    let (op, input) = view_of(&target(&app));
    assert_eq!(op, "sns list-topics");
    assert_eq!(input, serde_json::json!({}));
}

#[test]
fn sns_entered_topic_lists_subscriptions_by_topic_arn() {
    let mut app = app(Service::SnsTopics);
    app.sns_state.topics.items = vec![topic("events")];
    app.handle_action(Action::Select);
    let result = target(&app);
    let (op, input) = view_of(&result);
    assert_eq!(op, "sns list-subscriptions-by-topic");
    assert_eq!(
        input,
        serde_json::json!({ "TopicArn": "arn:aws:sns:us-east-1:123456789012:events" })
    );
    assert_eq!(scope_of(&result).unwrap().kind, ResourceKind::Topic);
}

// ── Secrets Manager / RDS / VPC ────────────────────────────────────────────────

#[test]
fn secrets_lists_secrets() {
    let app = app(Service::SecretsManagerSecrets);
    let (op, input) = view_of(&target(&app));
    assert_eq!(op, "secretsmanager list-secrets");
    assert_eq!(input, serde_json::json!({}));
}

#[test]
fn rds_describes_all_db_instances_unscoped() {
    let app = app(Service::RdsInstances);
    let (op, input) = view_of(&target(&app));
    assert_eq!(op, "rds describe-db-instances");
    assert_eq!(input, serde_json::json!({}));
}

#[test]
fn vpc_describes_all_vpcs_unscoped() {
    let app = app(Service::VpcVpcs);
    let (op, input) = view_of(&target(&app));
    assert_eq!(op, "ec2 describe-vpcs");
    assert_eq!(input, serde_json::json!({}));
}

// ── EC2 ─────────────────────────────────────────────────────────────────────

#[test]
fn ec2_describes_all_instances_without_client_state_filter() {
    let mut app = app(Service::Ec2Instances);
    // A client-side state filter must NOT become an AWS --filters argument.
    app.ec2_state.state_filter = crate::ui::ec2::StateFilter::Stopped;
    let (op, input) = view_of(&target(&app));
    assert_eq!(op, "ec2 describe-instances");
    assert_eq!(input, serde_json::json!({}));
}

#[test]
fn ec2_entered_instance_detail_is_none() {
    let mut app = app(Service::Ec2Instances);
    app.ec2_state.current_instance = Some("i-123".into());
    assert!(matches!(target(&app), Ok(None)));
}

// ── KMS ─────────────────────────────────────────────────────────────────────

fn kms_key() -> crate::kms::key::Key {
    crate::kms::key::Key {
        key_id: "1234abcd-12ab-34cd-56ef-1234567890ab".into(),
        key_arn: "arn:aws:kms:us-east-1:123456789012:key/1234abcd-12ab-34cd-56ef-1234567890ab"
            .into(),
        alias: String::new(),
        description: String::new(),
        key_state: "Enabled".into(),
        key_usage: String::new(),
        key_spec: String::new(),
        key_manager: "Customer".into(),
        creation_date: String::new(),
        expiration_date: String::new(),
        deletion_date: String::new(),
        custom_key_store_id: String::new(),
        origin: String::new(),
        multi_region: false,
        enabled: true,
    }
}

#[test]
fn kms_top_inventory_lists_keys() {
    let mut app = app(Service::KmsKeys);
    app.kms_state.tab = crate::ui::kms::Tab::CustomerManaged;
    app.kms_state.keys.items = vec![kms_key()];
    let (op, input) = view_of(&target(&app));
    assert_eq!(op, "kms list-keys");
    assert_eq!(input, serde_json::json!({}));
}

#[test]
fn kms_entered_detail_aliases_and_tags_tabs_scope_by_key_arn_others_are_none() {
    let mut app = app(Service::KmsKeys);
    app.kms_state.tab = crate::ui::kms::Tab::CustomerManaged;
    app.kms_state.keys.items = vec![kms_key()];
    app.handle_action(Action::Select); // open detail, tab = General
    let arn = "arn:aws:kms:us-east-1:123456789012:key/1234abcd-12ab-34cd-56ef-1234567890ab";

    // General configuration is a summary pane.
    assert!(matches!(target(&app), Ok(None)));

    // General → Aliases.
    app.handle_action(Action::NextDetailTab);
    let result = target(&app);
    let (op, input) = view_of(&result);
    assert_eq!(op, "kms list-aliases");
    assert_eq!(input, serde_json::json!({ "KeyId": arn }));
    assert_eq!(scope_of(&result).unwrap().kind, ResourceKind::Key);

    // Aliases → Rotation (summary).
    app.handle_action(Action::NextDetailTab);
    assert!(matches!(target(&app), Ok(None)));

    // Rotation → Policy (summary).
    app.handle_action(Action::NextDetailTab);
    assert!(matches!(target(&app), Ok(None)));

    // Policy → Tags.
    app.handle_action(Action::NextDetailTab);
    let (op, input) = view_of(&target(&app));
    assert_eq!(op, "kms list-resource-tags");
    assert_eq!(input, serde_json::json!({ "KeyId": arn }));
}

// ── EFS ─────────────────────────────────────────────────────────────────────

fn efs_fs(id: &str) -> crate::efs::fs::FileSystem {
    let mut fs = crate::efs::fs::FileSystem {
        file_system_id: id.into(),
        file_system_arn: format!(
            "arn:aws:elasticfilesystem:us-east-1:123456789012:file-system/{id}"
        ),
        name: "fs".into(),
        creation_token: String::new(),
        encrypted: "Yes".into(),
        kms_key_id: String::new(),
        total_size: String::new(),
        size_in_standard: String::new(),
        size_in_ia: String::new(),
        size_in_archive: String::new(),
        provisioned_throughput: String::new(),
        throughput_mode: String::new(),
        life_cycle_state: "available".into(),
        number_of_mount_targets: "0".into(),
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
        tags: vec![],
    };
    fs.name = "prod".into();
    fs
}

#[test]
fn efs_top_inventory_describes_file_systems() {
    let app = app(Service::EfsFileSystems);
    let (op, input) = view_of(&target(&app));
    assert_eq!(op, "efs describe-file-systems");
    assert_eq!(input, serde_json::json!({}));
}

#[test]
fn efs_entered_access_points_network_and_tags_map_to_exact_ops_others_none() {
    use crate::ui::efs::DetailTab;
    let mut app = app(Service::EfsFileSystems);
    app.efs_state.file_systems.items = vec![efs_fs("fs-001")];
    app.efs_state.current_file_system = Some("fs-001".into());

    app.efs_state.detail_tab = DetailTab::AccessPoints;
    let (op, input) = view_of(&target(&app));
    assert_eq!(op, "efs describe-access-points");
    assert_eq!(input, serde_json::json!({ "FileSystemId": "fs-001" }));

    app.efs_state.detail_tab = DetailTab::Network;
    let (op, input) = view_of(&target(&app));
    assert_eq!(op, "efs describe-mount-targets");
    assert_eq!(input, serde_json::json!({ "FileSystemId": "fs-001" }));

    app.efs_state.detail_tab = DetailTab::Tags;
    let result = target(&app);
    let (op, input) = view_of(&result);
    assert_eq!(op, "efs list-tags-for-resource");
    assert_eq!(input, serde_json::json!({ "ResourceId": "fs-001" }));
    assert_eq!(scope_of(&result).unwrap().kind, ResourceKind::Efs);

    for detail in [
        DetailTab::MeteredSize,
        DetailTab::Monitoring,
        DetailTab::FileSystemPolicy,
        DetailTab::Replication,
    ] {
        app.efs_state.detail_tab = detail;
        assert!(
            matches!(target(&app), Ok(None)),
            "{detail:?} should be a detail pane"
        );
    }
}

// ── FSx ─────────────────────────────────────────────────────────────────────

#[test]
fn fsx_top_inventory_describes_file_systems() {
    let app = app(Service::FsxFileSystems);
    let (op, input) = view_of(&target(&app));
    assert_eq!(op, "fsx describe-file-systems");
    assert_eq!(input, serde_json::json!({}));
}

#[test]
fn fsx_nested_tables_scope_backups_tags_and_updates_to_parent() {
    use crate::ui::fsx::DetailTab;
    let mut app = app(Service::FsxFileSystems);
    let mut fs = rusticity_core::fsx::FsxFileSystem {
        file_system_id: "fs-fsx1".into(),
        file_system_arn: "arn:aws:fsx:us-east-1:123456789012:file-system/fs-fsx1".into(),
        ..Default::default()
    };
    fs.name = "data".into();
    app.fsx_state.file_systems.items = vec![fs];
    app.fsx_state.current_file_system = Some("fs-fsx1".into());

    app.fsx_state.detail_tab = DetailTab::Backups;
    let (op, input) = view_of(&target(&app));
    assert_eq!(op, "fsx describe-backups");
    assert_eq!(
        input,
        serde_json::json!({
            "Filters": [{ "Name": "file-system-id", "Values": ["fs-fsx1"] }]
        })
    );

    app.fsx_state.detail_tab = DetailTab::Tags;
    let (op, input) = view_of(&target(&app));
    assert_eq!(op, "fsx list-tags-for-resource");
    assert_eq!(
        input["ResourceARN"],
        "arn:aws:fsx:us-east-1:123456789012:file-system/fs-fsx1"
    );
    app.fsx_state.detail_tab = DetailTab::Updates;
    let (op, input) = view_of(&target(&app));
    assert_eq!(op, "fsx describe-file-systems");
    assert_eq!(input["FileSystemIds"], serde_json::json!(["fs-fsx1"]));

    for detail in [
        DetailTab::NetworkSecurity,
        DetailTab::MonitoringPerformance,
        DetailTab::Administration,
        DetailTab::DataRepository,
    ] {
        app.fsx_state.detail_tab = detail;
        assert!(matches!(target(&app), Ok(None)), "{detail:?} is a pane");
    }
}

// ── S3 ──────────────────────────────────────────────────────────────────────

#[test]
fn s3_top_inventory_lists_buckets_region_none() {
    let app = app(Service::S3Buckets);
    let result = target(&app);
    let (op, input) = view_of(&result);
    assert_eq!(op, "s3api list-buckets");
    assert_eq!(input, serde_json::json!({}));
    assert_eq!(region_of(&result), None);
}

#[test]
fn s3_entered_objects_tab_lists_objects_v2_with_bucket_region_and_navigated_prefix() {
    use crate::ui::s3::ObjectTab;
    let mut app = app(Service::S3Buckets);
    app.s3_state.buckets.items = vec![crate::s3::Bucket {
        name: "bucket".into(),
        region: "eu-west-1".into(),
        creation_date: String::new(),
    }];
    app.s3_state.current_bucket = Some("bucket".into());
    app.s3_state.object_tab = ObjectTab::Objects;
    app.s3_state.objects = vec![crate::s3::Object {
        key: "a/file.txt".into(),
        is_prefix: false,
        size: 0,
        last_modified: String::new(),
        storage_class: String::new(),
    }];

    // Root of the bucket: Bucket + Delimiter, no Prefix. Bucket's own region.
    let result = target(&app);
    let (op, input) = view_of(&result);
    assert_eq!(op, "s3api list-objects-v2");
    assert_eq!(
        input,
        serde_json::json!({ "Bucket": "bucket", "Delimiter": "/" })
    );
    assert_eq!(region_of(&result), Some("eu-west-1".into()));

    // A client text FILTER must not leak into the request; only a navigated
    // prefix scopes it.
    app.s3_state.object_filter = "needle".into();
    app.s3_state.prefix_stack = vec!["logs/".into()];
    let (_, input) = view_of(&target(&app));
    assert_eq!(
        input,
        serde_json::json!({ "Bucket": "bucket", "Delimiter": "/", "Prefix": "logs/" })
    );

    // An open single-object preview is a detail view → None (preserve head-object).
    app.s3_state.object_view.preview = Some(crate::s3::object::Preview {
        key: "a/file.txt".into(),
        content: rusticity_core::s3::object::TextPreview {
            text: "hello".into(),
            truncated: false,
            format: rusticity_core::s3::object::TextFormat::Plain,
        },
        scroll: 0,
    });
    assert!(matches!(target(&app), Ok(None)));
}

#[test]
fn s3_non_objects_tab_is_none() {
    use crate::ui::s3::ObjectTab;
    let mut app = app(Service::S3Buckets);
    app.s3_state.current_bucket = Some("bucket".into());
    app.s3_state.object_tab = ObjectTab::Properties;
    assert!(matches!(target(&app), Ok(None)));
}

// ── ECR ─────────────────────────────────────────────────────────────────────

fn ecr_image(tag: &str, digest: &str) -> crate::ecr::image::Image {
    crate::ecr::image::Image {
        tag: tag.into(),
        digest: digest.into(),
        uri: format!("registry/repo:{tag}"),
        artifact_type: "Image".into(),
        pushed_at: String::new(),
        last_pull_time: String::new(),
        size_bytes: 0,
    }
}

#[test]
fn ecr_top_repository_list_private_and_public_tabs() {
    use crate::ui::ecr::Tab;
    let mut app = app(Service::EcrRepositories);

    app.ecr_state.tab = Tab::Private;
    let result = target(&app);
    let (op, input) = view_of(&result);
    assert_eq!(op, "ecr describe-repositories");
    assert_eq!(input, serde_json::json!({}));
    assert_eq!(region_of(&result), None);

    app.ecr_state.tab = Tab::Public;
    let result = target(&app);
    let (op, _) = view_of(&result);
    assert_eq!(op, "ecr-public describe-repositories");
    assert_eq!(region_of(&result), Some("us-east-1".into()));
}

#[test]
fn ecr_entered_private_images_table_describes_images_without_image_ids() {
    use crate::ui::ecr::Tab;
    let mut app = app(Service::EcrRepositories);
    app.ecr_state.tab = Tab::Private;
    app.ecr_state.current_repository = Some("team/repo".into());
    app.ecr_state.images.items = vec![ecr_image("v1", "sha256:aaa"), ecr_image("v2", "sha256:bbb")];
    // Selecting a different image must not change the images-table command.
    app.ecr_state.images.selected = 1;
    let (op, input) = view_of(&target(&app));
    assert_eq!(op, "ecr describe-images");
    assert_eq!(input, serde_json::json!({ "repositoryName": "team/repo" }));
}

#[test]
fn ecr_entered_public_repository_lists_images() {
    use crate::ui::ecr::Tab;
    let mut app = app(Service::EcrRepositories);
    app.ecr_state.tab = Tab::Public;
    app.ecr_state.current_repository = Some("public/repo".into());
    let (op, input) = view_of(&target(&app));
    assert_eq!(op, "ecr-public describe-images");
    assert_eq!(input, serde_json::json!({"repositoryName":"public/repo"}));
}

#[test]
fn ecr_image_scan_detail_tabs_map_to_digest_scoped_apis_general_is_none() {
    use crate::ui::ecr::Tab;
    let mut app = app(Service::EcrRepositories);
    app.ecr_state.tab = Tab::Private;
    app.ecr_state.current_repository = Some("team/repo".into());
    app.ecr_state.images.items = vec![ecr_image("v1", "sha256:deadbeef")];
    app.handle_action(Action::Select); // open scan view, tab = General

    // General information is an image summary pane.
    assert!(matches!(target(&app), Ok(None)));

    let image_id = serde_json::json!({ "imageDigest": "sha256:deadbeef" });

    // General → Scanning.
    app.handle_action(Action::NextDetailTab);
    let (op, input) = view_of(&target(&app));
    assert_eq!(op, "ecr describe-image-scan-findings");
    assert_eq!(
        input,
        serde_json::json!({ "repositoryName": "team/repo", "imageId": image_id })
    );

    // Scanning → Referrers.
    app.handle_action(Action::NextDetailTab);
    let (op, input) = view_of(&target(&app));
    assert_eq!(op, "ecr list-image-referrers");
    assert_eq!(input["subjectId"], image_id);
    assert!(input.get("imageId").is_none());

    // Referrers → Replication.
    app.handle_action(Action::NextDetailTab);
    let (op, input) = view_of(&target(&app));
    assert_eq!(op, "ecr describe-image-replication-status");
    assert_eq!(
        input,
        serde_json::json!({ "repositoryName": "team/repo", "imageId": image_id })
    );

    // Replication → Signing.
    app.handle_action(Action::NextDetailTab);
    let (op, _) = view_of(&target(&app));
    assert_eq!(op, "ecr describe-image-signing-status");
}

// ── Non-owned services ────────────────────────────────────────────────────────

#[test]
fn non_owned_services_defer_to_the_caller_with_none() {
    for service in [
        Service::ApiGatewayApis,
        Service::CloudWatchLogGroups,
        Service::CloudWatchAlarms,
        Service::CloudTrailEvents,
        Service::DynamoDbTables,
        Service::LambdaFunctions,
        Service::CloudFormationStacks,
        Service::IamUsers,
        Service::IamRoles,
        Service::IamUserGroups,
    ] {
        let app = app(service);
        assert!(
            matches!(target(&app), Ok(None)),
            "{service:?} is not owned here"
        );
    }
}
