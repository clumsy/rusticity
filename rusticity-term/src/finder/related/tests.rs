#![cfg(test)]
//! Tests for `finder::related::selected` — the selected-source adapter.
//!
//! These tests exercise the key contract:
//! - `service_selected == false` → `None`
//! - Empty/stale inventory → `None`
//! - Detail parent takes precedence over subtable selection
//! - Filtered/sorted selection is used (not raw index)
//! - Region is scoped from `app.config.region`
//! - ARNs are populated from stored data where available
//! - Lambda qualifiers produce qualified ids/ARNs (alias priority)
//! - CloudTrail → None, Lambda Applications → Stack only with explicit stack_id
//! - Unsupported services return None

use super::*;
use crate::app::{App, Service, ViewMode};
use rusticity_core::finder::ResourceKind;

/// Test harness: create an app with `service_selected = true` and the given service.
fn app_for(service: Service) -> App {
    let mut app = App::new_without_client("test".into(), Some("us-east-2".into()));
    app.current_service = service;
    app.service_selected = true;
    app.view_mode = ViewMode::List;
    app
}

// ── Unsupported services return None ─────────────────────────────────────

#[test]
fn cloudtrail_events_returns_none() {
    let app = app_for(Service::CloudTrailEvents);
    assert_eq!(selected(&app), None);
}

#[test]
fn cloudwatch_insights_returns_none() {
    let app = app_for(Service::CloudWatchInsights);
    assert_eq!(selected(&app), None);
}

#[test]
fn lambda_application_without_stack_id_returns_none() {
    use crate::lambda::Application;
    let mut app = app_for(Service::LambdaApplications);
    app.lambda_application_state.table.items = vec![Application {
        name: "my-app".into(),
        arn: String::new(),
        description: String::new(),
        status: String::new(),
        last_modified: String::new(),
    }];
    app.lambda_application_state.table.selected = 0;
    assert_eq!(selected(&app), None);
}

// ── service_selected false → None ───────────────────────────────────────

#[test]
fn service_not_selected_returns_none() {
    let mut app = App::new_without_client("test".into(), Some("us-east-2".into()));
    app.current_service = Service::SqsQueues;
    app.service_selected = false;
    app.sqs_state.queues.items = vec![sqs_queue("test")];
    assert_eq!(selected(&app), None);
}

// ── SQS: filtered selection vs raw index ────────────────────────────────

#[test]
fn sqs_filtered_selection_not_raw_index() {
    let mut app = app_for(Service::SqsQueues);
    app.sqs_state.queues.items = vec![
        sqs_queue("alpha-queue"),
        sqs_queue("beta-queue"),
        sqs_queue("gamma-queue"),
    ];
    // SQS filter uses starts_with, so "beta" narrows to [beta-queue]
    app.sqs_state.queues.filter = "beta".into();
    app.sqs_state.queues.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.kind, ResourceKind::Queue);
    assert_eq!(r.id, "beta-queue");
    assert_eq!(r.arn, "");
    assert_eq!(r.region, "us-east-2");
}

#[test]
fn sqs_detail_parent_takes_precedence() {
    let mut app = app_for(Service::SqsQueues);
    app.sqs_state.queues.items = vec![sqs_queue("alpha-queue"), sqs_queue("beta-queue")];
    app.sqs_state.current_queue = Some("alpha-queue".into());
    app.sqs_state.queues.selected = 1;

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "alpha-queue");
}

#[test]
fn sqs_empty_inventory_returns_none() {
    let app = app_for(Service::SqsQueues);
    assert_eq!(selected(&app), None);
}

#[test]
fn sqs_stale_selected_returns_none() {
    let mut app = app_for(Service::SqsQueues);
    app.sqs_state.queues.items = vec![sqs_queue("only-queue")];
    app.sqs_state.queues.selected = 5;
    assert_eq!(selected(&app), None);
}

// ── Lambda: filtered selection, detail parent, qualifiers ────────────────

#[test]
fn lambda_filtered_selection_not_raw_index() {
    let mut app = app_for(Service::LambdaFunctions);
    app.lambda_state.table.items = vec![
        lambda_function("alpha-fn", "arn:aws:lambda:us-east-2:123:function:alpha-fn"),
        lambda_function("beta-fn", "arn:aws:lambda:us-east-2:123:function:beta-fn"),
        lambda_function("gamma-fn", "arn:aws:lambda:us-east-2:123:function:gamma-fn"),
    ];
    app.lambda_state.table.filter = "beta".into();
    app.lambda_state.table.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.kind, ResourceKind::Function);
    assert_eq!(r.id, "beta-fn");
    assert_eq!(r.arn, "arn:aws:lambda:us-east-2:123:function:beta-fn");
    assert_eq!(r.region, "us-east-2");
}

#[test]
fn lambda_empty_inventory_returns_none() {
    let app = app_for(Service::LambdaFunctions);
    assert_eq!(selected(&app), None);
}

#[test]
fn lambda_stale_selected_returns_none() {
    let mut app = app_for(Service::LambdaFunctions);
    app.lambda_state.table.items = [lambda_function(
        "only",
        "arn:aws:lambda:us-east-2:123:function:only",
    )]
    .into();
    app.lambda_state.table.selected = 3;
    assert_eq!(selected(&app), None);
}

#[test]
fn lambda_detail_parent_takes_precedence() {
    let mut app = app_for(Service::LambdaFunctions);
    app.lambda_state.table.items = vec![
        lambda_function("alpha-fn", "arn:aws:lambda:us-east-2:123:function:alpha-fn"),
        lambda_function("beta-fn", "arn:aws:lambda:us-east-2:123:function:beta-fn"),
    ];
    app.lambda_state.current_function = Some("alpha-fn".into());
    app.lambda_state.table.selected = 1;

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "alpha-fn");
}

#[test]
fn lambda_alias_qualifier_in_id_and_arn() {
    let mut app = app_for(Service::LambdaFunctions);
    app.lambda_state.table.items = [lambda_function(
        "my-fn",
        "arn:aws:lambda:us-east-2:123:function:my-fn",
    )]
    .into();
    app.lambda_state.current_function = Some("my-fn".into());
    app.lambda_state.current_alias = Some("prod".into());

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "my-fn:prod");
    assert_eq!(r.arn, "arn:aws:lambda:us-east-2:123:function:my-fn:prod");
}

#[test]
fn lambda_version_qualifier_in_id_and_arn() {
    let mut app = app_for(Service::LambdaFunctions);
    app.lambda_state.table.items = [lambda_function(
        "my-fn",
        "arn:aws:lambda:us-east-2:123:function:my-fn",
    )]
    .into();
    app.lambda_state.current_function = Some("my-fn".into());
    app.lambda_state.current_version = Some("3".into());

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "my-fn:3");
    assert_eq!(r.arn, "arn:aws:lambda:us-east-2:123:function:my-fn:3");
}

#[test]
fn lambda_alias_takes_priority_over_version() {
    let mut app = app_for(Service::LambdaFunctions);
    app.lambda_state.table.items = [lambda_function(
        "my-fn",
        "arn:aws:lambda:us-east-2:123:function:my-fn",
    )]
    .into();
    app.lambda_state.current_function = Some("my-fn".into());
    app.lambda_state.current_alias = Some("prod".into());
    app.lambda_state.current_version = Some("5".into());

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "my-fn:prod");
    assert_eq!(r.arn, "arn:aws:lambda:us-east-2:123:function:my-fn:prod");
}

#[test]
fn lambda_no_qualifier_returns_plain_id() {
    let mut app = app_for(Service::LambdaFunctions);
    app.lambda_state.table.items = [lambda_function(
        "my-fn",
        "arn:aws:lambda:us-east-2:123:function:my-fn",
    )]
    .into();
    app.lambda_state.current_function = Some("my-fn".into());

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "my-fn");
    assert_eq!(r.arn, "arn:aws:lambda:us-east-2:123:function:my-fn");
}

// ── EC2: filtered selection vs raw index ─────────────────────────────────

#[test]
fn ec2_filtered_selection_not_raw_index() {
    let mut app = app_for(Service::Ec2Instances);
    app.ec2_state.table.items = vec![
        ec2_instance("i-aaa", "web-server"),
        ec2_instance("i-bbb", "db-server"),
        ec2_instance("i-ccc", "cache-server"),
    ];
    app.ec2_state.table.filter = "db".into();
    app.ec2_state.table.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.kind, ResourceKind::Instance);
    assert_eq!(r.id, "i-bbb");
    assert_eq!(r.name, "db-server");
    assert_eq!(r.region, "us-east-2");
}

#[test]
fn ec2_detail_parent_takes_precedence() {
    let mut app = app_for(Service::Ec2Instances);
    app.ec2_state.table.items = vec![
        ec2_instance("i-aaa", "web-server"),
        ec2_instance("i-bbb", "db-server"),
    ];
    app.ec2_state.current_instance = Some("i-aaa".into());
    app.ec2_state.table.selected = 1;

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "i-aaa");
}

#[test]
fn ec2_empty_inventory_returns_none() {
    let app = app_for(Service::Ec2Instances);
    assert_eq!(selected(&app), None);
}

#[test]
fn ec2_stale_selected_returns_none() {
    let mut app = app_for(Service::Ec2Instances);
    app.ec2_state.table.items = vec![ec2_instance("i-aaa", "web-server")];
    app.ec2_state.table.selected = 10;
    assert_eq!(selected(&app), None);
}

// ── DynamoDB ─────────────────────────────────────────────────────────────

#[test]
fn dynamodb_arn_from_stored_data() {
    use rusticity_core::dynamodb::DdbTable;
    let mut app = app_for(Service::DynamoDbTables);
    app.dynamodb_state.tables.items = vec![DdbTable {
        name: "orders".into(),
        table_arn: "arn:aws:dynamodb:us-east-2:123:table/orders".into(),
        ..Default::default()
    }];
    app.dynamodb_state.tables.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.kind, ResourceKind::Table);
    assert_eq!(r.id, "orders");
    assert_eq!(r.arn, "arn:aws:dynamodb:us-east-2:123:table/orders");
}

#[test]
fn dynamodb_detail_parent_takes_precedence() {
    use rusticity_core::dynamodb::DdbTable;
    let mut app = app_for(Service::DynamoDbTables);
    app.dynamodb_state.tables.items = vec![
        DdbTable {
            name: "orders".into(),
            table_arn: "arn:aws:dynamodb:us-east-2:123:table/orders".into(),
            ..Default::default()
        },
        DdbTable {
            name: "products".into(),
            table_arn: "arn:aws:dynamodb:us-east-2:123:table/products".into(),
            ..Default::default()
        },
    ];
    app.dynamodb_state.current_table = Some("orders".into());
    app.dynamodb_state.tables.selected = 1;

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "orders");
}

#[test]
fn dynamodb_stale_selected_returns_none() {
    use rusticity_core::dynamodb::DdbTable;
    let mut app = app_for(Service::DynamoDbTables);
    app.dynamodb_state.tables.items = vec![DdbTable {
        name: "orders".into(),
        ..Default::default()
    }];
    app.dynamodb_state.tables.selected = 5;
    assert_eq!(selected(&app), None);
}

// ── S3: filtered selection, bucket region ────────────────────────────────

#[test]
fn s3_bucket_name_and_region() {
    use crate::s3::Bucket;
    let mut app = app_for(Service::S3Buckets);
    app.s3_state.buckets.items = vec![Bucket {
        name: "my-bucket".into(),
        region: "us-west-2".into(),
        creation_date: String::new(),
    }];
    app.s3_state.selected_row = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.kind, ResourceKind::Bucket);
    assert_eq!(r.id, "my-bucket");
    assert_eq!(r.region, "us-west-2");
}

#[test]
fn s3_filtered_selection_not_raw_index() {
    use crate::s3::Bucket;
    let mut app = app_for(Service::S3Buckets);
    app.s3_state.buckets.items = vec![
        Bucket {
            name: "alpha-bucket".into(),
            region: "us-east-1".into(),
            creation_date: String::new(),
        },
        Bucket {
            name: "beta-bucket".into(),
            region: "us-east-1".into(),
            creation_date: String::new(),
        },
        Bucket {
            name: "gamma-bucket".into(),
            region: "us-east-1".into(),
            creation_date: String::new(),
        },
    ];
    // Filter narrows to [beta-bucket]; selected_row=0 should pick beta
    app.s3_state.buckets.filter = "beta".into();
    app.s3_state.selected_row = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "beta-bucket");
}

#[test]
fn s3_detail_parent_takes_precedence() {
    use crate::s3::Bucket;
    let mut app = app_for(Service::S3Buckets);
    app.s3_state.buckets.items = vec![
        Bucket {
            name: "bucket-a".into(),
            region: "us-east-1".into(),
            creation_date: String::new(),
        },
        Bucket {
            name: "bucket-b".into(),
            region: "us-east-1".into(),
            creation_date: String::new(),
        },
    ];
    app.s3_state.current_bucket = Some("bucket-a".into());
    app.s3_state.selected_row = 1;

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "bucket-a");
}

#[test]
fn s3_empty_inventory_returns_none() {
    let app = app_for(Service::S3Buckets);
    assert_eq!(selected(&app), None);
}

#[test]
fn s3_stale_selected_row_returns_none() {
    use crate::s3::Bucket;
    let mut app = app_for(Service::S3Buckets);
    app.s3_state.buckets.items = vec![Bucket {
        name: "only".into(),
        region: "us-east-1".into(),
        creation_date: String::new(),
    }];
    app.s3_state.selected_row = 10;
    assert_eq!(selected(&app), None);
}

// ── CloudFormation ──────────────────────────────────────────────────────

#[test]
fn cfn_stack_id_as_arn() {
    use crate::cfn::Stack;
    let mut app = app_for(Service::CloudFormationStacks);
    app.cfn_state.table.items = vec![Stack {
        name: "my-stack".into(),
        stack_id: "arn:aws:cloudformation:us-east-2:123:stack/my-stack/abc123".into(),
        ..Default::default()
    }];
    app.cfn_state.table.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.kind, ResourceKind::Stack);
    assert_eq!(
        r.arn,
        "arn:aws:cloudformation:us-east-2:123:stack/my-stack/abc123"
    );
}

// ── API Gateway ─────────────────────────────────────────────────────────

#[test]
fn apig_api_id_and_name() {
    use crate::apig::api::RestApi;
    let mut app = app_for(Service::ApiGatewayApis);
    app.apig_state.apis.items = vec![RestApi {
        id: "abc123".into(),
        name: "my-api".into(),
        description: String::new(),
        created_date: String::new(),
        api_key_source: String::new(),
        endpoint_configuration: String::new(),
        protocol_type: String::new(),
        disable_execute_api_endpoint: false,
        status: String::new(),
    }];
    app.apig_state.apis.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.kind, ResourceKind::Api);
    assert_eq!(r.id, "abc123");
    assert_eq!(r.name, "my-api");
}

// ── KMS ─────────────────────────────────────────────────────────────────

#[test]
fn kms_arn_from_stored_data() {
    use crate::kms::key::Key;
    let mut app = app_for(Service::KmsKeys);
    app.kms_state.keys.items = vec![Key {
        key_id: "abc-123".into(),
        key_arn: "arn:aws:kms:us-east-2:123:key/abc-123".into(),
        alias: "alias/my-key".into(),
        description: String::new(),
        key_state: String::new(),
        key_usage: String::new(),
        key_spec: String::new(),
        key_manager: "Customer".into(),
        creation_date: String::new(),
        expiration_date: String::new(),
        deletion_date: String::new(),
        custom_key_store_id: String::new(),
        origin: String::new(),
        enabled: true,
        multi_region: false,
    }];
    app.kms_state.keys.selected = 0;
    app.kms_state.tab = crate::ui::kms::Tab::CustomerManaged;

    let r = selected(&app).unwrap();
    assert_eq!(r.kind, ResourceKind::Key);
    assert_eq!(r.arn, "arn:aws:kms:us-east-2:123:key/abc-123");
}

// ── EFS ─────────────────────────────────────────────────────────────────

#[test]
fn efs_arn_from_stored_data() {
    let mut app = app_for(Service::EfsFileSystems);
    app.efs_state.file_systems.items = vec![efs_file_system(
        "fs-12345",
        "my-efs",
        "arn:aws:elasticfilesystem:us-east-2:123:file-system/fs-12345",
    )];
    app.efs_state.file_systems.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.kind, ResourceKind::Efs);
    assert_eq!(
        r.arn,
        "arn:aws:elasticfilesystem:us-east-2:123:file-system/fs-12345"
    );
}

#[test]
fn efs_detail_parent_takes_precedence() {
    let mut app = app_for(Service::EfsFileSystems);
    app.efs_state.file_systems.items = vec![
        efs_file_system(
            "fs-aaa",
            "alpha",
            "arn:aws:elasticfilesystem:us-east-2:123:file-system/fs-aaa",
        ),
        efs_file_system(
            "fs-bbb",
            "beta",
            "arn:aws:elasticfilesystem:us-east-2:123:file-system/fs-bbb",
        ),
    ];
    app.efs_state.current_file_system = Some("fs-aaa".into());
    app.efs_state.file_systems.selected = 1;

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "fs-aaa");
}

// ── RDS ─────────────────────────────────────────────────────────────────

#[test]
fn rds_arn_from_stored_data() {
    use rusticity_core::rds::RdsInstance;
    let mut app = app_for(Service::RdsInstances);
    app.rds_state.instances.items = vec![RdsInstance {
        identifier: "db-prod-1".into(),
        arn: "arn:aws:rds:us-east-2:123:db:db-prod-1".into(),
        ..Default::default()
    }];
    app.rds_state.instances.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.kind, ResourceKind::Database);
    assert_eq!(r.arn, "arn:aws:rds:us-east-2:123:db:db-prod-1");
}

#[test]
fn rds_stale_selected_returns_none() {
    use rusticity_core::rds::RdsInstance;
    let mut app = app_for(Service::RdsInstances);
    app.rds_state.instances.items = vec![RdsInstance {
        identifier: "db-prod-1".into(),
        ..Default::default()
    }];
    app.rds_state.instances.selected = 10;
    assert_eq!(selected(&app), None);
}

// ── VPC ─────────────────────────────────────────────────────────────────

#[test]
fn vpc_arn_from_stored_data() {
    use rusticity_core::vpc::Vpc;
    let mut app = app_for(Service::VpcVpcs);
    app.vpc_state.vpcs.items = vec![Vpc {
        name: "main-vpc".into(),
        vpc_id: "vpc-12345".into(),
        arn: "arn:aws:ec2:us-east-2:123:vpc/vpc-12345".into(),
        ..Default::default()
    }];
    app.vpc_state.vpcs.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.kind, ResourceKind::Vpc);
    assert_eq!(r.arn, "arn:aws:ec2:us-east-2:123:vpc/vpc-12345");
}

// ── IAM users: global region, detail parent ────────────────────────────

#[test]
fn iam_users_global_region() {
    let mut app = app_for(Service::IamUsers);
    app.iam_state.users.items = vec![iam_user("test-user")];
    app.iam_state.users.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.kind, ResourceKind::User);
    assert_eq!(r.id, "test-user");
    assert_eq!(r.region, "global");
}

#[test]
fn iam_users_detail_parent_takes_precedence() {
    let mut app = app_for(Service::IamUsers);
    app.iam_state.users.items = vec![iam_user("alice"), iam_user("bob")];
    app.iam_state.current_user = Some("alice".into());
    app.iam_state.users.selected = 1;

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "alice");
}

#[test]
fn iam_users_arn_from_stored_data() {
    let mut app = app_for(Service::IamUsers);
    app.iam_state.users.items = vec![{
        let mut u = iam_user("test-user");
        u.arn = "arn:aws:iam::123:user/test-user".into();
        u
    }];
    app.iam_state.users.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.arn, "arn:aws:iam::123:user/test-user");
}

// ── IAM roles: ARN from stored data, detail parent ──────────────────────

#[test]
fn iam_roles_arn_from_stored_data() {
    let mut app = app_for(Service::IamRoles);
    app.iam_state.roles.items = vec![iam_role("TestRole", "arn:aws:iam::123:role/TestRole")];
    app.iam_state.roles.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.kind, ResourceKind::Role);
    assert_eq!(r.arn, "arn:aws:iam::123:role/TestRole");
    assert_eq!(r.region, "global");
}

#[test]
fn iam_roles_detail_parent_takes_precedence() {
    let mut app = app_for(Service::IamRoles);
    app.iam_state.roles.items = vec![
        iam_role("RoleA", "arn:aws:iam::123:role/RoleA"),
        iam_role("RoleB", "arn:aws:iam::123:role/RoleB"),
    ];
    app.iam_state.current_role = Some("RoleA".into());
    app.iam_state.roles.selected = 1;

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "RoleA");
}

// ── IAM groups: filter mirror, detail parent ────────────────────────────

#[test]
fn iam_groups_filtered_selection_not_raw_index() {
    let mut app = app_for(Service::IamUserGroups);
    app.iam_state.groups.items = vec![
        iam_group("alpha-group"),
        iam_group("beta-group"),
        iam_group("gamma-group"),
    ];
    app.iam_state.groups.filter = "beta".into();
    app.iam_state.groups.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.kind, ResourceKind::Group);
    assert_eq!(r.id, "beta-group");
    assert_eq!(r.region, "global");
}

#[test]
fn iam_groups_detail_parent_takes_precedence() {
    let mut app = app_for(Service::IamUserGroups);
    app.iam_state.groups.items = vec![iam_group("alpha-group"), iam_group("beta-group")];
    app.iam_state.current_group = Some("alpha-group".into());
    app.iam_state.groups.selected = 1;

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "alpha-group");
}

#[test]
fn iam_groups_stale_selected_returns_none() {
    let mut app = app_for(Service::IamUserGroups);
    app.iam_state.groups.items = vec![iam_group("only-group")];
    app.iam_state.groups.selected = 5;
    assert_eq!(selected(&app), None);
}

// ── CloudWatch Alarms: tab + name filter, detail parent ──────────────────

#[test]
fn alarm_name_selected() {
    use crate::cw::alarms::Alarm;
    let mut app = app_for(Service::CloudWatchAlarms);
    app.alarms_state.table.items = vec![Alarm {
        name: "HighCPU".into(),
        state: "OK".into(),
        ..Default::default()
    }];
    app.alarms_state.table.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.kind, ResourceKind::Alarm);
    assert_eq!(r.id, "HighCPU");
}

#[test]
fn alarm_detail_parent_takes_precedence() {
    use crate::cw::alarms::Alarm;
    let mut app = app_for(Service::CloudWatchAlarms);
    app.alarms_state.table.items = vec![
        Alarm {
            name: "AlarmA".into(),
            state: "OK".into(),
            ..Default::default()
        },
        Alarm {
            name: "AlarmB".into(),
            state: "OK".into(),
            ..Default::default()
        },
    ];
    app.alarms_state.current_alarm = Some("AlarmA".into());
    app.alarms_state.table.selected = 1;

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "AlarmA");
}

#[test]
fn alarm_in_alarm_tab_filters_to_alarmed() {
    use crate::cw::alarms::Alarm;
    let mut app = app_for(Service::CloudWatchAlarms);
    app.alarms_state.table.items = vec![
        Alarm {
            name: "OKAlarm".into(),
            state: "OK".into(),
            ..Default::default()
        },
        Alarm {
            name: "FiringAlarm".into(),
            state: "ALARM".into(),
            ..Default::default()
        },
    ];
    // InAlarm tab: only "FiringAlarm" passes. selected=0 in filtered list.
    app.alarms_state.alarm_tab = crate::app::AlarmTab::InAlarm;
    app.alarms_state.table.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "FiringAlarm");
}

#[test]
fn alarm_name_filter_narrows_selection() {
    use crate::cw::alarms::Alarm;
    let mut app = app_for(Service::CloudWatchAlarms);
    app.alarms_state.table.items = vec![
        Alarm {
            name: "HighCPU".into(),
            state: "OK".into(),
            ..Default::default()
        },
        Alarm {
            name: "HighMemory".into(),
            state: "OK".into(),
            ..Default::default()
        },
    ];
    // Filter to "memory" narrows to [HighMemory]; selected=0 picks that.
    app.alarms_state.table.filter = "memory".into();
    app.alarms_state.table.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "HighMemory");
}

#[test]
fn alarm_stale_selected_returns_none() {
    use crate::cw::alarms::Alarm;
    let mut app = app_for(Service::CloudWatchAlarms);
    app.alarms_state.table.items = vec![Alarm {
        name: "Only".into(),
        state: "OK".into(),
        ..Default::default()
    }];
    app.alarms_state.table.selected = 5;
    assert_eq!(selected(&app), None);
}

#[test]
fn alarm_arn_from_stored_data() {
    use crate::cw::alarms::Alarm;
    let mut app = app_for(Service::CloudWatchAlarms);
    app.alarms_state.table.items = vec![Alarm {
        name: "HighCPU".into(),
        state: "OK".into(),
        alarm_arn: "arn:aws:cloudwatch:us-east-2:123:alarm:HighCPU".into(),
        ..Default::default()
    }];
    app.alarms_state.table.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.arn, "arn:aws:cloudwatch:us-east-2:123:alarm:HighCPU");
}

// ── CloudWatch Log Groups: ARN, detail/events context ───────────────────

#[test]
fn log_group_arn_from_stored_data() {
    let mut app = app_for(Service::CloudWatchLogGroups);
    app.log_groups_state.log_groups.items = vec![log_group_with_arn(
        "/aws/lambda/my-fn",
        "arn:aws:logs:us-east-2:123:log-group:/aws/lambda/my-fn:*",
    )];
    app.log_groups_state.log_groups.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.kind, ResourceKind::LogGroup);
    assert_eq!(
        r.arn,
        "arn:aws:logs:us-east-2:123:log-group:/aws/lambda/my-fn:*"
    );
}

#[test]
fn log_group_detail_mode_uses_unfiltered_index() {
    // In Detail mode the action handler resets selected to the unfiltered
    // items position, so we must index into raw items, not the filtered list.
    let mut app = app_for(Service::CloudWatchLogGroups);
    app.log_groups_state.log_groups.items = vec![
        log_group_with_arn("alpha", "arn:aws:logs:us-east-2:123:log-group:alpha:*"),
        log_group_with_arn("beta", "arn:aws:logs:us-east-2:123:log-group:beta:*"),
    ];
    // Simulate entering detail on "beta" (index 1 in raw items).
    app.view_mode = ViewMode::Detail;
    app.log_groups_state.log_groups.filter = "alpha".into(); // would narrow to [alpha]
    app.log_groups_state.log_groups.selected = 1; // raw index, not filtered

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "beta");
}

#[test]
fn log_group_events_mode_uses_unfiltered_index() {
    let mut app = app_for(Service::CloudWatchLogGroups);
    app.log_groups_state.log_groups.items = vec![log_group_with_arn(
        "only-group",
        "arn:aws:logs:us-east-2:123:log-group:only-group:*",
    )];
    app.view_mode = ViewMode::Events;
    app.log_groups_state.log_groups.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "only-group");
}

#[test]
fn log_group_list_mode_uses_filtered_index() {
    let mut app = app_for(Service::CloudWatchLogGroups);
    app.log_groups_state.log_groups.items = vec![log_group("alpha"), log_group("beta")];
    app.view_mode = ViewMode::List;
    app.log_groups_state.log_groups.filter = "beta".into();
    app.log_groups_state.log_groups.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.id, "beta");
}

#[test]
fn log_group_stale_selected_returns_none() {
    let mut app = app_for(Service::CloudWatchLogGroups);
    app.log_groups_state.log_groups.items = vec![log_group("only")];
    app.log_groups_state.log_groups.selected = 10;
    assert_eq!(selected(&app), None);
}

// ── Lambda Applications: Stack only with explicit stack_id ──────────────

#[test]
fn lambda_application_with_stack_id() {
    use crate::lambda::Application;
    let mut app = app_for(Service::LambdaApplications);
    app.lambda_application_state.table.items = vec![Application {
        name: "my-app".into(),
        arn: "arn:aws:cloudformation:us-east-2:123:stack/my-app/uuid".into(),
        description: String::new(),
        status: String::new(),
        last_modified: String::new(),
    }];
    app.lambda_application_state.table.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.kind, ResourceKind::Stack);
    assert_eq!(r.id, "my-app");
    assert_eq!(
        r.arn,
        "arn:aws:cloudformation:us-east-2:123:stack/my-app/uuid"
    );
}

// ── Region scoping ───────────────────────────────────────────────────────

#[test]
fn region_is_scoped_from_config() {
    let mut app = App::new_without_client("test".into(), Some("eu-west-1".into()));
    app.current_service = Service::SqsQueues;
    app.service_selected = true;
    app.sqs_state.queues.items = vec![sqs_queue("my-queue")];
    app.sqs_state.queues.selected = 0;

    let r = selected(&app).unwrap();
    assert_eq!(r.region, "eu-west-1");
}

// ── Eight known successful sources ──────────────────────────────────────
//
// Verifies that the eight services that should return a Resource do so
// with the correct ResourceKind.  The remaining services (CloudTrail,
// CloudWatchInsights) return None and are tested above.

#[test]
fn eight_known_sources_return_resources() {
    // 1. SQS
    {
        let mut app = app_for(Service::SqsQueues);
        app.sqs_state.queues.items = vec![sqs_queue("q1")];
        app.sqs_state.queues.selected = 0;
        assert_eq!(selected(&app).unwrap().kind, ResourceKind::Queue);
    }
    // 2. Lambda
    {
        let mut app = app_for(Service::LambdaFunctions);
        app.lambda_state.table.items = [lambda_function(
            "fn1",
            "arn:aws:lambda:us-east-2:123:function:fn1",
        )]
        .into();
        app.lambda_state.table.selected = 0;
        assert_eq!(selected(&app).unwrap().kind, ResourceKind::Function);
    }
    // 3. EC2
    {
        let mut app = app_for(Service::Ec2Instances);
        app.ec2_state.table.items = vec![ec2_instance("i-1", "name1")];
        app.ec2_state.table.selected = 0;
        assert_eq!(selected(&app).unwrap().kind, ResourceKind::Instance);
    }
    // 4. DynamoDB
    {
        use rusticity_core::dynamodb::DdbTable;
        let mut app = app_for(Service::DynamoDbTables);
        app.dynamodb_state.tables.items = vec![DdbTable {
            name: "t1".into(),
            ..Default::default()
        }];
        app.dynamodb_state.tables.selected = 0;
        assert_eq!(selected(&app).unwrap().kind, ResourceKind::Table);
    }
    // 5. S3
    {
        use crate::s3::Bucket;
        let mut app = app_for(Service::S3Buckets);
        app.s3_state.buckets.items = vec![Bucket {
            name: "b1".into(),
            region: "us-east-1".into(),
            creation_date: String::new(),
        }];
        app.s3_state.selected_row = 0;
        assert_eq!(selected(&app).unwrap().kind, ResourceKind::Bucket);
    }
    // 6. IAM Users
    {
        let mut app = app_for(Service::IamUsers);
        app.iam_state.users.items = vec![iam_user("u1")];
        app.iam_state.users.selected = 0;
        assert_eq!(selected(&app).unwrap().kind, ResourceKind::User);
    }
    // 7. CloudWatch Alarms
    {
        use crate::cw::alarms::Alarm;
        let mut app = app_for(Service::CloudWatchAlarms);
        app.alarms_state.table.items = vec![Alarm {
            name: "a1".into(),
            state: "OK".into(),
            ..Default::default()
        }];
        app.alarms_state.table.selected = 0;
        assert_eq!(selected(&app).unwrap().kind, ResourceKind::Alarm);
    }
    // 8. CloudWatch Log Groups
    {
        let mut app = app_for(Service::CloudWatchLogGroups);
        app.log_groups_state.log_groups.items = vec![log_group("lg1")];
        app.log_groups_state.log_groups.selected = 0;
        assert_eq!(selected(&app).unwrap().kind, ResourceKind::LogGroup);
    }
}

// ── Construct helpers ───────────────────────────────────────────────────

fn sqs_queue(name: &str) -> crate::sqs::queue::Queue {
    crate::sqs::queue::Queue {
        name: name.to_string(),
        url: format!("https://sqs.us-east-2.amazonaws.com/123/{name}"),
        queue_type: "Standard".to_string(),
        created_timestamp: String::new(),
        messages_available: "0".to_string(),
        messages_in_flight: "0".to_string(),
        encryption: "Disabled".to_string(),
        content_based_deduplication: "Disabled".to_string(),
        last_modified_timestamp: String::new(),
        visibility_timeout: String::new(),
        message_retention_period: String::new(),
        maximum_message_size: String::new(),
        delivery_delay: String::new(),
        receive_message_wait_time: String::new(),
        high_throughput_fifo: "N/A".to_string(),
        deduplication_scope: "N/A".to_string(),
        fifo_throughput_limit: "N/A".to_string(),
        dead_letter_queue: "-".to_string(),
        messages_delayed: "0".to_string(),
        redrive_allow_policy: "-".to_string(),
        redrive_policy: String::new(),
        redrive_task_id: "-".to_string(),
        redrive_task_start_time: "-".to_string(),
        redrive_task_status: "-".to_string(),
        redrive_task_percent: "-".to_string(),
        redrive_task_destination: "-".to_string(),
    }
}

fn lambda_function(name: &str, arn: &str) -> crate::lambda::Function {
    crate::lambda::Function {
        name: name.to_string(),
        arn: arn.to_string(),
        application: None,
        description: String::new(),
        package_type: String::new(),
        runtime: String::new(),
        architecture: String::new(),
        code_size: 0,
        code_sha256: String::new(),
        memory_mb: 128,
        timeout_seconds: 30,
        last_modified: String::new(),
        layers: vec![],
    }
}

fn ec2_instance(id: &str, name: &str) -> crate::ec2::Instance {
    crate::ec2::Instance {
        instance_id: id.into(),
        name: name.into(),
        state: "running".into(),
        ..Default::default()
    }
}

fn efs_file_system(id: &str, name: &str, arn: &str) -> crate::efs::fs::FileSystem {
    crate::efs::fs::FileSystem {
        file_system_id: id.to_string(),
        file_system_arn: arn.to_string(),
        name: name.to_string(),
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
        dns_name: String::new(),
        tags: vec![],
        total_size_bytes: 0,
        size_in_standard_bytes: 0,
        size_in_ia_bytes: 0,
        size_in_archive_bytes: 0,
    }
}

fn iam_user(name: &str) -> crate::iam::IamUser {
    crate::iam::IamUser {
        user_name: name.to_string(),
        path: String::new(),
        groups: String::new(),
        last_activity: String::new(),
        mfa: String::new(),
        password_age: String::new(),
        console_last_sign_in: String::new(),
        access_key_id: String::new(),
        active_key_age: String::new(),
        access_key_last_used: String::new(),
        arn: String::new(),
        creation_time: String::new(),
        console_access: String::new(),
        signing_certs: String::new(),
    }
}

fn iam_role(name: &str, arn: &str) -> crate::iam::IamRole {
    crate::iam::IamRole {
        role_name: name.to_string(),
        path: String::new(),
        trusted_entities: String::new(),
        last_activity: String::new(),
        arn: arn.to_string(),
        creation_time: String::new(),
        description: String::new(),
        max_session_duration: None,
    }
}

fn iam_group(name: &str) -> crate::iam::IamGroup {
    crate::iam::IamGroup {
        group_name: name.to_string(),
        path: String::new(),
        users: String::new(),
        permissions: String::new(),
        creation_time: String::new(),
    }
}

fn log_group(name: &str) -> rusticity_core::LogGroup {
    rusticity_core::LogGroup {
        name: name.to_string(),
        creation_time: None,
        stored_bytes: None,
        retention_days: None,
        log_class: None,
        arn: None,
        log_group_arn: None,
        deletion_protection_enabled: None,
    }
}

fn log_group_with_arn(name: &str, arn: &str) -> rusticity_core::LogGroup {
    let mut lg = log_group(name);
    lg.log_group_arn = Some(arn.to_string());
    lg
}

// ════════════════════════════════════════════════════════════════════════
// Step 4: render / selection parity for the canonical filtered helpers.
//
// These tests drive the PRODUCTION functions — `ui::s3::filtered_buckets`,
// `ui::iam::filtered_iam_groups`, `ui::cw::alarms::filtered_alarms`,
// `finder::related::selected`, and the owned action handlers — never a
// re-derived copy of the filter formula. The contract they lock down is that
// the renderer, the selected-resource adapter, and the keyboard actions all
// index the SAME ordered filtered list, so the highlighted row and the
// resolved resource can never disagree.
// ════════════════════════════════════════════════════════════════════════

fn s3_bucket(name: &str, region: &str) -> crate::s3::Bucket {
    crate::s3::Bucket {
        name: name.to_string(),
        region: region.to_string(),
        creation_date: String::new(),
    }
}

fn s3_object(key: &str, is_prefix: bool) -> crate::app::S3Object {
    crate::s3::Object {
        key: key.to_string(),
        size: 0,
        last_modified: String::new(),
        is_prefix,
        storage_class: String::new(),
    }
}

fn alarm_named(name: &str, state: &str) -> crate::cw::alarms::Alarm {
    crate::cw::alarms::Alarm {
        name: name.to_string(),
        state: state.to_string(),
        ..Default::default()
    }
}

// ── S3: filtered_buckets helper directly ─────────────────────────────────

#[test]
fn s3_filtered_buckets_empty_filter_is_identity_order() {
    let items = vec![
        s3_bucket("gamma", "r"),
        s3_bucket("Alpha", "r"),
        s3_bucket("beta", "r"),
    ];
    let out = crate::ui::s3::filtered_buckets(&items, "");
    let names: Vec<&str> = out.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, vec!["gamma", "Alpha", "beta"]);
}

#[test]
fn s3_filtered_buckets_mixed_case_contains() {
    let items = vec![s3_bucket("Prod-Logs", "r"), s3_bucket("dev-data", "r")];
    // Query case differs from the stored name on both sides.
    let out = crate::ui::s3::filtered_buckets(&items, "PROD");
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].name, "Prod-Logs");
}

#[test]
fn s3_filtered_buckets_unicode_contains() {
    let items = vec![s3_bucket("数据-bucket", "r"), s3_bucket("plain", "r")];
    let out = crate::ui::s3::filtered_buckets(&items, "数据");
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].name, "数据-bucket");
}

/// The renderer walks `filtered_buckets` in order; `selected` must resolve the
/// same bucket for the same `selected_row`, under a mixed-case filter.
#[test]
fn s3_render_and_selected_agree_mixed_case_filter() {
    let mut app = app_for(Service::S3Buckets);
    app.s3_state.buckets.items = vec![
        s3_bucket("alpha-One", "us-east-1"),
        s3_bucket("Beta-Two", "us-east-1"),
        s3_bucket("alpha-Three", "us-east-1"),
    ];
    app.s3_state.buckets.filter = "ALPHA".into();

    let filtered =
        crate::ui::s3::filtered_buckets(&app.s3_state.buckets.items, &app.s3_state.buckets.filter);
    assert_eq!(
        filtered.len(),
        2,
        "mixed-case contains keeps both alpha rows"
    );

    // Row 0 → first filtered bucket.
    app.s3_state.selected_row = 0;
    assert_eq!(selected(&app).unwrap().id, filtered[0].name);
    // Row 1 (no expansion) → second filtered bucket.
    app.s3_state.selected_row = 1;
    assert_eq!(selected(&app).unwrap().id, filtered[1].name);
}

/// A child (object) row under an expanded bucket resolves to its OWNING bucket,
/// not the next bucket — the alias-row behaviour the renderer relies on.
#[test]
fn s3_expanded_child_row_resolves_to_owning_bucket() {
    let mut app = app_for(Service::S3Buckets);
    app.s3_state.buckets.items = vec![
        s3_bucket("first", "us-east-1"),
        s3_bucket("second", "us-east-1"),
    ];
    // Expand "first" with two visual child rows.
    app.s3_state.expanded_prefixes.insert("first".into());
    app.s3_state.bucket_preview.insert(
        "first".into(),
        vec![s3_object("a.txt", false), s3_object("b.txt", false)],
    );

    // Visual layout: row0=first, row1=a.txt, row2=b.txt, row3=second.
    for child_row in [0usize, 1, 2] {
        app.s3_state.selected_row = child_row;
        assert_eq!(
            selected(&app).unwrap().id,
            "first",
            "row {child_row} is within the expanded 'first' bucket"
        );
    }
    app.s3_state.selected_row = 3;
    assert_eq!(selected(&app).unwrap().id, "second");
}

/// Directory-bucket tab shows no buckets, so `selected` yields None even with a
/// populated GeneralPurpose list.
#[test]
fn s3_directory_tab_returns_none() {
    let mut app = app_for(Service::S3Buckets);
    app.s3_state.buckets.items = vec![s3_bucket("only", "us-east-1")];
    app.s3_state.selected_row = 0;
    app.s3_state.bucket_type = crate::ui::s3::BucketType::Directory;
    assert_eq!(selected(&app), None);
}

// ── IAM groups: filtered_iam_groups helper + render/selected parity ──────

#[test]
fn iam_filtered_groups_mixed_case_and_unicode() {
    let mut app = app_for(Service::IamUserGroups);
    app.iam_state.groups.items = vec![iam_group("Admins"), iam_group("réaders"), iam_group("devs")];

    app.iam_state.groups.filter = "ADMIN".into();
    assert_eq!(crate::ui::iam::filtered_iam_groups(&app).len(), 1);

    app.iam_state.groups.filter = "réa".into();
    let out = crate::ui::iam::filtered_iam_groups(&app);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].group_name, "réaders");

    app.iam_state.groups.filter = "".into();
    assert_eq!(crate::ui::iam::filtered_iam_groups(&app).len(), 3);
}

/// `selected` must return the SAME group the renderer would highlight at
/// `groups.selected`, i.e. index into the canonical filtered list.
#[test]
fn iam_groups_selected_matches_rendered_filtered_index() {
    let mut app = app_for(Service::IamUserGroups);
    app.iam_state.groups.items = vec![
        iam_group("alpha-grp"),
        iam_group("beta-grp"),
        iam_group("alpha-two"),
    ];
    app.iam_state.groups.filter = "alpha".into();

    app.iam_state.groups.selected = 1;
    let filtered = crate::ui::iam::filtered_iam_groups(&app);
    assert_eq!(filtered.len(), 2);
    let r = selected(&app).unwrap();
    assert_eq!(r.id, filtered[1].group_name);
    assert_eq!(r.id, "alpha-two");
}

// ── CW alarms: filtered_alarms helper + tab/selection parity ─────────────

#[test]
fn alarms_filtered_helper_tab_then_name() {
    let items = vec![
        alarm_named("CPU-High", "ALARM"),
        alarm_named("CPU-Low", "OK"),
        alarm_named("Mem-High", "alarm"), // lowercase state still counts (to_uppercase)
    ];
    use crate::app::AlarmTab;

    // AllAlarms + empty → identity.
    assert_eq!(
        crate::ui::cw::alarms::filtered_alarms(&items, AlarmTab::AllAlarms, "").len(),
        3
    );
    // InAlarm → only the two ALARM-state rows, order preserved.
    let in_alarm = crate::ui::cw::alarms::filtered_alarms(&items, AlarmTab::InAlarm, "");
    let names: Vec<&str> = in_alarm.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, vec!["CPU-High", "Mem-High"]);
    // InAlarm + mixed-case name filter.
    let narrowed = crate::ui::cw::alarms::filtered_alarms(&items, AlarmTab::InAlarm, "mem");
    assert_eq!(narrowed.len(), 1);
    assert_eq!(narrowed[0].name, "Mem-High");
    // Unicode name filter.
    let uni = vec![alarm_named("温度-alarm", "ALARM")];
    assert_eq!(
        crate::ui::cw::alarms::filtered_alarms(&uni, AlarmTab::AllAlarms, "温度").len(),
        1
    );
}

/// Under InAlarm tab + name filter, the keyboard action (`alarms_select_item`)
/// and `finder::related::selected` must open the SAME alarm — the mismatch this
/// step eliminates.
#[test]
fn alarms_action_and_selected_agree_under_tab_and_filter() {
    let mut app = app_for(Service::CloudWatchAlarms);
    app.alarms_state.table.items = vec![
        alarm_named("web-ok", "OK"),
        alarm_named("web-firing", "ALARM"),
        alarm_named("db-firing", "ALARM"),
    ];
    app.alarms_state.alarm_tab = crate::app::AlarmTab::InAlarm;
    app.alarms_state.table.filter = "db".into();
    // Filtered list is [db-firing]; selected index 0.
    app.alarms_state.table.selected = 0;

    // finder adapter:
    let via_finder = selected(&app).unwrap();
    assert_eq!(via_finder.id, "db-firing");

    // keyboard action opens the detail parent for the same alarm:
    crate::cw::actions::alarms_select_item(&mut app);
    assert_eq!(app.alarms_state.current_alarm.as_deref(), Some("db-firing"));
}

/// Regression for the pre-step4 bug: with a non-AllAlarms tab active, indexing
/// the RAW item list would select a different alarm than the rendered row.
/// The canonical helper makes the action index the filtered list instead.
#[test]
fn alarms_action_uses_filtered_not_raw_index() {
    let mut app = app_for(Service::CloudWatchAlarms);
    app.alarms_state.table.items = vec![
        alarm_named("ok-first", "OK"), // raw index 0 — filtered OUT by InAlarm
        alarm_named("firing-second", "ALARM"), // filtered index 0
    ];
    app.alarms_state.alarm_tab = crate::app::AlarmTab::InAlarm;
    app.alarms_state.table.selected = 0;

    crate::cw::actions::alarms_select_item(&mut app);
    // Must be the filtered row, NOT the raw index-0 "ok-first".
    assert_eq!(
        app.alarms_state.current_alarm.as_deref(),
        Some("firing-second")
    );
}

/// Render the actual list rather than inferring its rows from a test-side filter.
fn rendered_list(app: &App) -> String {
    use ratatui::{backend::TestBackend, Terminal};
    let mut terminal = Terminal::new(TestBackend::new(220, 40)).unwrap();
    terminal
        .draw(|frame| crate::ui::render(frame, app))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn filtered_s3_frame_and_related_selection_agree_with_expanded_child_rows() {
    let mut app = app_for(Service::S3Buckets);
    app.mode = crate::keymap::Mode::Normal;
    app.s3_state.buckets.items = vec![
        s3_bucket("hidden-bucket", "us-east-2"),
        s3_bucket("match-first", "us-east-2"),
        s3_bucket("match-second", "us-east-2"),
    ];
    app.s3_state.buckets.filter = "MATCH".into();
    app.s3_state.expanded_prefixes.insert("match-first".into());
    app.s3_state
        .bucket_preview
        .insert("match-first".into(), vec![s3_object("child.txt", false)]);
    app.s3_state.selected_row = 1;
    let text = rendered_list(&app);
    assert!(text.contains("match-first"));
    assert!(text.contains("child.txt"));
    assert!(!text.contains("hidden-bucket"));
    assert_eq!(selected(&app).unwrap().id, "match-first");
    app.s3_state.selected_row = 2;
    assert!(rendered_list(&app).contains("match-second"));
    assert_eq!(selected(&app).unwrap().id, "match-second");
}

#[test]
fn filtered_iam_frame_related_selection_and_enter_agree() {
    let mut app = app_for(Service::IamUserGroups);
    app.mode = crate::keymap::Mode::Normal;
    app.iam_state.groups.items = vec![
        iam_group("hidden-group"),
        iam_group("match-one"),
        iam_group("match-two"),
    ];
    app.iam_state.groups.filter = "MATCH".into();
    app.iam_state.groups.selected = 1;
    let text = rendered_list(&app);
    assert!(text.contains("match-two"));
    assert!(!text.contains("hidden-group"));
    assert_eq!(selected(&app).unwrap().id, "match-two");
    app.handle_action(crate::keymap::Action::Select);
    assert_eq!(app.iam_state.current_group.as_deref(), Some("match-two"));
}

#[test]
fn filtered_alarm_frame_related_selection_and_enter_agree() {
    let mut app = app_for(Service::CloudWatchAlarms);
    app.mode = crate::keymap::Mode::Normal;
    app.alarms_state.table.items = vec![
        alarm_named("hidden-ok", "OK"),
        alarm_named("other-firing", "ALARM"),
        alarm_named("match-firing", "ALARM"),
    ];
    app.alarms_state.alarm_tab = crate::app::AlarmTab::InAlarm;
    app.alarms_state.table.filter = "MATCH".into();
    let text = rendered_list(&app);
    assert!(text.contains("match-firing"));
    assert!(!text.contains("hidden-ok"));
    assert!(!text.contains("other-firing"));
    assert_eq!(selected(&app).unwrap().id, "match-firing");
    app.handle_action(crate::keymap::Action::Select);
    assert_eq!(
        app.alarms_state.current_alarm.as_deref(),
        Some("match-firing")
    );
}
