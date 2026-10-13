use super::*;
use aws_sdk_rds::types::{
    CertificateDetails, DbCluster, DbClusterMember, DbInstanceStatusInfo, DbParameterGroupStatus,
    DbSubnetGroup, Endpoint, OptionGroupMembership, UpgradeRolloutOrder,
    VpcSecurityGroupMembership,
};

#[test]
fn rds_maps_requested_metadata_and_preserves_zero_and_false() {
    let sdk = DbInstance::builder()
        .db_instance_identifier("orders")
        .db_instance_arn("arn:aws:rds:us-east-2:123456789012:db:orders")
        .db_instance_status("available")
        .engine("postgres")
        .engine_version("17.11")
        .db_instance_class("db.t4g.micro")
        .availability_zone("us-east-2a")
        .endpoint(
            Endpoint::builder()
                .address("orders.example.invalid")
                .port(5432)
                .build(),
        )
        .upgrade_rollout_order(UpgradeRolloutOrder::First)
        .engine_lifecycle_support("open-source-rds-extended-support-disabled")
        .db_subnet_group(
            DbSubnetGroup::builder()
                .db_subnet_group_name("private")
                .vpc_id("vpc-test")
                .build(),
        )
        .multi_az(false)
        .storage_type("gp3")
        .allocated_storage(20)
        .iops(3000)
        .storage_throughput(125)
        .vpc_security_groups(
            VpcSecurityGroupMembership::builder()
                .vpc_security_group_id("sg-test")
                .build(),
        )
        .certificate_details(
            CertificateDetails::builder()
                .valid_till(DateTime::from_secs(1800000000))
                .build(),
        )
        .pending_modified_values(
            PendingModifiedValues::builder()
                .engine_version("18")
                .multi_az(false)
                .master_user_password("must-never-be-displayed")
                .build(),
        )
        .character_set_name("UTF8")
        .option_group_memberships(
            OptionGroupMembership::builder()
                .option_group_name("default:postgres-17")
                .build(),
        )
        .instance_create_time(DateTime::from_secs(1700000000))
        .storage_encrypted(true)
        .promotion_tier(0)
        .status_infos(
            DbInstanceStatusInfo::builder()
                .status_type("read replication")
                .status("replicating")
                .build(),
        )
        .db_parameter_groups(
            DbParameterGroupStatus::builder()
                .db_parameter_group_name("default.postgres17")
                .build(),
        )
        .read_replica_source_db_instance_identifier("source")
        .build();
    let i = RdsInstance::from_sdk(&sdk, "us-east-2");
    assert_eq!(i.identifier, "orders");
    assert_eq!(i.region, "us-east-2");
    assert_eq!(i.endpoint, "orders.example.invalid");
    assert_eq!(i.port, Some(5432));
    assert_eq!(i.role, "Read replica");
    assert_eq!(i.upgrade_rollout_order, "first");
    assert_eq!(i.extended_support, "Disabled");
    assert_eq!(i.vpc, "vpc-test");
    assert_eq!(i.subnet_group, "private");
    assert_eq!(i.multi_az, Some(false));
    assert_eq!(i.storage_type, "gp3");
    assert_eq!(i.storage_gib, Some(20));
    assert_eq!(i.iops, Some(3000));
    assert_eq!(i.storage_throughput, Some(125));
    assert_eq!(i.security_groups, "sg-test");
    assert_eq!(
        i.certificate_expiry,
        timestamp(Some(&DateTime::from_secs(1800000000)))
    );
    assert_eq!(
        i.created_time,
        timestamp(Some(&DateTime::from_secs(1700000000)))
    );
    assert!(i.pending_changes.contains("Engine version"));
    assert!(i.pending_changes.contains("Multi-AZ"));
    assert!(i.pending_changes.contains("Password"));
    assert!(!i.pending_changes.contains("must-never"));
    assert_eq!(i.character_set, "UTF8");
    assert_eq!(i.option_group, "default:postgres-17");
    assert_eq!(i.encrypted, Some(true));
    assert_eq!(i.priority_tier, Some(0));
    assert_eq!(i.replica_state, "replicating");
    assert_eq!(i.parameter_group, "default.postgres17");
    assert!(i.cpu.is_empty());
    assert!(i.recommendations.is_empty());
}

#[test]
fn rds_missing_optional_fields_do_not_become_false_or_zero() {
    let sdk = DbInstance::builder().db_instance_status("creating").build();
    let i = RdsInstance::from_sdk(&sdk, "eu-west-1");
    assert_eq!(i.status, "creating");
    assert_eq!(i.region, "eu-west-1");
    assert!(i.endpoint.is_empty());
    assert!(i.role.is_empty());
    assert_eq!(i.port, None);
    assert_eq!(i.multi_az, None);
    assert_eq!(i.encrypted, None);
    assert_eq!(i.storage_gib, None);
    assert!(i.pending_changes.is_empty());
}

#[test]
fn rds_cluster_metadata_determines_role_priority_and_clone_group() {
    let mut instances = vec![
        RdsInstance {
            identifier: "reader".into(),
            cluster_identifier: "cluster".into(),
            ..Default::default()
        },
        RdsInstance {
            identifier: "writer".into(),
            cluster_identifier: "cluster".into(),
            ..Default::default()
        },
    ];
    let clusters = [DbCluster::builder()
        .db_cluster_identifier("cluster")
        .clone_group_id("clone-id")
        .db_cluster_members(
            DbClusterMember::builder()
                .db_instance_identifier("reader")
                .is_cluster_writer(false)
                .promotion_tier(3)
                .build(),
        )
        .db_cluster_members(
            DbClusterMember::builder()
                .db_instance_identifier("writer")
                .is_cluster_writer(true)
                .build(),
        )
        .build()];
    enrichment::apply_clusters(&mut instances, &clusters);
    assert_eq!(instances[0].role, "Reader");
    assert_eq!(instances[0].priority_tier, Some(3));
    assert_eq!(instances[0].clone_group_id, "clone-id");
    assert!(instances[0].is_replica);
    assert_eq!(instances[1].role, "Writer");
    assert!(!instances[1].is_replica);
}

#[test]
fn rds_optional_metadata_matches_resource_and_does_not_count_resolved_recommendations() {
    use aws_sdk_rds::types::{
        DbRecommendation, PendingMaintenanceAction, ResourcePendingMaintenanceActions,
    };
    let mut instances = vec![
        RdsInstance {
            arn: "arn:test:db:one".into(),
            ..Default::default()
        },
        RdsInstance {
            arn: "arn:test:db:two".into(),
            ..Default::default()
        },
    ];
    let recommendations = [
        DbRecommendation::builder()
            .resource_arn("arn:test:db:one")
            .status("active")
            .build(),
        DbRecommendation::builder()
            .resource_arn("arn:test:db:one")
            .status("resolved")
            .build(),
        DbRecommendation::builder()
            .resource_arn("arn:test:db:other")
            .status("active")
            .build(),
    ];
    enrichment::apply_recommendations(&mut instances, &recommendations);
    assert_eq!(instances[0].recommendations, "1");
    assert_eq!(instances[1].recommendations, "0");
    let maintenance = [ResourcePendingMaintenanceActions::builder()
        .resource_identifier("arn:test:db:one")
        .pending_maintenance_action_details(
            PendingMaintenanceAction::builder()
                .action("system-update")
                .build(),
        )
        .build()];
    enrichment::apply_maintenance(&mut instances, &maintenance);
    assert_eq!(instances[0].maintenance, "system-update");
    assert_eq!(instances[1].maintenance, "None");
}
