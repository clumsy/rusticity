use super::*;
use aws_sdk_rds::types::{
    AvailabilityZone, DbSubnetGroup, Endpoint, Subnet, Tag, VpcSecurityGroupMembership,
};
use aws_smithy_http_client::test_util::infallible_client_fn;
use std::sync::{Arc, Mutex};

const ARN: &str = "arn:aws:rds:us-east-2:123456789012:db:orders";

fn instance_xml(arn: &str) -> String {
    format!(
        r#"<DescribeDBInstancesResponse xmlns="http://rds.amazonaws.com/doc/2014-10-31/">
  <DescribeDBInstancesResult><DBInstances><DBInstance>
    <DBInstanceIdentifier>orders</DBInstanceIdentifier>
    <DBInstanceArn>{arn}</DBInstanceArn>
    <DBInstanceStatus>available</DBInstanceStatus>
    <Engine>postgres</Engine>
    <MasterUsername>admin</MasterUsername>
    <DeletionProtection>true</DeletionProtection>
    <TagList><Tag><Key>team</Key><Value>data</Value></Tag></TagList>
  </DBInstance></DBInstances></DescribeDBInstancesResult>
</DescribeDBInstancesResponse>"#
    )
}

/// Replays one canned response to every request and records each body.
fn config(status: u16, body: String) -> (AwsConfig, Arc<Mutex<Vec<String>>>) {
    routed(move |_| (status, body.clone()))
}

/// Answers each request from `route(action)` and records each request as
/// `<uri> <form body>`. CloudWatch requests are not form encoded, so their
/// action is empty.
fn routed(
    route: impl Fn(&str) -> (u16, String) + Send + Sync + 'static,
) -> (AwsConfig, Arc<Mutex<Vec<String>>>) {
    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let record = seen.clone();
    let http = infallible_client_fn(move |request| {
        let sent = request
            .body()
            .bytes()
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        let action = sent
            .split('&')
            .find_map(|pair| pair.strip_prefix("Action="))
            .unwrap_or_default()
            .to_string();
        record
            .lock()
            .unwrap()
            .push(format!("{} {sent}", request.uri()));
        let (status, body) = route(&action);
        http::Response::builder()
            .status(status)
            .body(body.into_bytes())
            .unwrap()
    });
    let sdk_config = aws_config::SdkConfig::builder()
        .behavior_version(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-2"))
        .credentials_provider(aws_sdk_rds::config::SharedCredentialsProvider::new(
            aws_sdk_rds::config::Credentials::new("test", "test", None, None, "test"),
        ))
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(http)
        .build();
    (
        AwsConfig::from_sdk_config(sdk_config, "us-east-2", "123456789012"),
        seen,
    )
}

const DENIED: &str = r#"<ErrorResponse><Error><Type>Sender</Type><Code>AccessDenied</Code><Message>no</Message></Error><RequestId>r</RequestId></ErrorResponse>"#;

fn rds_response(action: &str, result: &str) -> String {
    format!(
        r#"<{action}Response xmlns="http://rds.amazonaws.com/doc/2014-10-31/"><{action}Result>{result}</{action}Result></{action}Response>"#
    )
}

#[tokio::test]
async fn one_describe_call_scoped_by_full_arn_with_tags() {
    let (config, seen) = config(200, instance_xml(ARN));
    let detail = instance_details(&config, ARN).await.unwrap();
    assert_eq!(detail.identifier, "orders");
    assert_eq!(detail.master_username, "admin");
    assert_eq!(detail.deletion_protection, Some(true));
    assert_eq!(detail.tags, [("team".to_string(), "data".to_string())]);
    let seen = seen.lock().unwrap();
    assert!(
        seen[0].contains("Action=DescribeDBInstances"),
        "{}",
        seen[0]
    );
    assert_eq!(
        seen.iter()
            .filter(|r| r.contains("Action=DescribeDBInstances"))
            .count(),
        1,
        "exactly one instance read"
    );
    assert!(
        seen[0].contains(&format!("DBInstanceIdentifier={}", ARN.replace(':', "%3A"))),
        "{}",
        seen[0]
    );
}

#[tokio::test]
async fn mismatched_identity_is_rejected() {
    let other = "arn:aws:rds:us-east-2:123456789012:db:other";
    let (config, _) = config(200, instance_xml(other));
    let error = instance_details(&config, ARN).await.unwrap_err();
    assert!(error.to_string().contains("mismatched identity"));
}

#[tokio::test]
async fn not_found_is_an_error() {
    let (config, _) = config(
        404,
        r#"<ErrorResponse><Error><Type>Sender</Type><Code>DBInstanceNotFound</Code><Message>gone</Message></Error><RequestId>r</RequestId></ErrorResponse>"#.into(),
    );
    let error = instance_details(&config, ARN).await.unwrap_err();
    assert!(format!("{error:#}").contains("DescribeDBInstances"));
}

#[tokio::test]
async fn foreign_or_malformed_arns_are_rejected_before_any_request() {
    let (config, seen) = config(200, instance_xml(ARN));
    for arn in [
        "arn:aws:rds:eu-west-1:123456789012:db:orders",
        "arn:aws:rds:us-east-2:999999999999:db:orders",
        "arn:aws:rds:us-east-2:123456789012:cluster:orders",
        "arn:aws:rds:us-east-2:123456789012:db:",
        "orders",
    ] {
        assert!(instance_details(&config, arn).await.is_err(), "{arn}");
    }
    assert!(seen.lock().unwrap().is_empty());
}

#[test]
fn maps_every_tab_field_and_keeps_absent_values_absent() {
    let sdk = DbInstance::builder()
        .db_instance_identifier("orders")
        .db_instance_arn(ARN)
        .dbi_resource_id("db-ABC")
        .db_instance_status("available")
        .engine("postgres")
        .engine_version("17.1")
        .db_instance_class("db.t4g.micro")
        .availability_zone("us-east-2a")
        .db_cluster_identifier("cluster-1")
        .endpoint(
            Endpoint::builder()
                .address("orders.example")
                .port(5432)
                .build(),
        )
        .db_subnet_group(
            DbSubnetGroup::builder()
                .db_subnet_group_name("private")
                .vpc_id("vpc-1")
                .subnets(
                    Subnet::builder()
                        .subnet_identifier("subnet-b")
                        .subnet_availability_zone(
                            AvailabilityZone::builder().name("us-east-2b").build(),
                        )
                        .build(),
                )
                .build(),
        )
        .vpc_security_groups(
            VpcSecurityGroupMembership::builder()
                .vpc_security_group_id("sg-1")
                .status("active")
                .build(),
        )
        .publicly_accessible(false)
        .ca_certificate_identifier("rds-ca-rsa2048-g1")
        .max_allocated_storage(100)
        .backup_retention_period(0)
        .preferred_backup_window("03:00-04:00")
        .tag_list(Tag::builder().key("z").value("2").build())
        .tag_list(Tag::builder().key("a").value("1").build())
        .build();
    let d = map(&sdk, "us-east-2");
    assert_eq!(d.role, "Cluster member (cluster-1)");
    assert_eq!(d.resource_id, "db-ABC");
    assert_eq!(d.port, Some(5432));
    assert_eq!(d.subnets, ["subnet-b"]);
    assert_eq!(d.security_groups, ["sg-1 (active)"]);
    assert_eq!(d.publicly_accessible, Some(false));
    assert_eq!(d.max_storage_gib, Some(100));
    assert_eq!(
        d.backup_retention_days,
        Some(0),
        "zero is kept, not dropped"
    );
    assert_eq!(d.tags[0].0, "a", "tags are sorted");
    assert_eq!(d.deletion_protection, None);
    assert!(d.kms_key_id.is_empty() && d.latest_restorable_time.is_empty());
}

#[tokio::test]
async fn sections_read_this_instance_and_fail_independently() {
    let rid = "db-ABC";
    let xml = instance_xml(ARN).replace(
        "<Engine>postgres</Engine>",
        &format!("<Engine>postgres</Engine><DbiResourceId>{rid}</DbiResourceId>"),
    );
    let (config, seen) = routed(move |action| {
        match action {
        "DescribeDBInstances" => (200, xml.clone()),
        "DescribeEvents" => (
            200,
            rds_response(
                "DescribeEvents",
                "<Events>\
                 <Event><SourceIdentifier>orders</SourceIdentifier><Message>old</Message><Date>2026-01-01T00:00:00Z</Date></Event>\
                 <Event><SourceIdentifier>orders</SourceIdentifier><Message>new</Message><Date>2026-01-02T00:00:00Z</Date></Event>\
                 <Event><SourceIdentifier>other</SourceIdentifier><Message>foreign</Message></Event>\
                 </Events>",
            ),
        ),
        "DescribeDBLogFiles" => (
            200,
            rds_response(
                "DescribeDBLogFiles",
                "<DescribeDBLogFiles><DescribeDBLogFilesDetails>\
                 <LogFileName>error/postgres.log</LogFileName><LastWritten>1767225600000</LastWritten><Size>200</Size>\
                 </DescribeDBLogFilesDetails></DescribeDBLogFiles>",
            ),
        ),
        "DescribeIntegrations" => (
            200,
            rds_response(
                "DescribeIntegrations",
                &format!(
                    "<Integrations>\
                     <Integration><IntegrationName>mine</IntegrationName><SourceArn>{ARN}</SourceArn><TargetArn>arn:aws:redshift:t</TargetArn><Status>active</Status></Integration>\
                     <Integration><IntegrationName>theirs</IntegrationName><SourceArn>arn:aws:rds:us-east-2:123456789012:db:other</SourceArn></Integration>\
                     </Integrations>"
                ),
            ),
        ),
        "DescribeDBRecommendations" => (
            200,
            rds_response(
                "DescribeDBRecommendations",
                &format!(
                    "<DBRecommendations>\
                     <member><ResourceArn>{ARN}</ResourceArn><Status>active</Status><Severity>low</Severity><Detection>d1</Detection></member>\
                     <member><ResourceArn>{ARN}</ResourceArn><Status>resolved</Status><Detection>d2</Detection></member>\
                     <member><ResourceArn>arn:aws:rds:us-east-2:123456789012:db:other</ResourceArn><Status>active</Status></member>\
                     </DBRecommendations>"
                ),
            ),
        ),
        // CloudWatch (metrics, alarms) is denied.
        _ => (403, DENIED.into()),
    }
    });
    let detail = instance_details(&config, ARN).await.unwrap();
    let s = &detail.sections;
    let events = s.events.as_ref().unwrap();
    assert_eq!(
        events
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>(),
        ["new", "old"],
        "own events only, newest first"
    );
    let logs = s.logs.as_ref().unwrap();
    assert_eq!(logs[0].name, "error/postgres.log");
    assert_eq!(logs[0].size, Some(200));
    assert_eq!(logs[0].last_written, "2026-01-01T00:00:00Z");
    let integrations = s.integrations.as_ref().unwrap();
    assert_eq!(integrations.len(), 1, "other sources are dropped");
    assert_eq!(integrations[0].name, "mine");
    assert_eq!(integrations[0].status, "active");
    assert_eq!(integrations[0].source, "orders");
    assert_eq!(integrations[0].source_arn, ARN);
    let recommendations = s.recommendations.as_ref().unwrap();
    assert_eq!(recommendations.len(), 2, "other resources are dropped");
    assert_eq!(recommendations[0].resource_identifier, "orders");
    // DMS is denied too: only Data migrations is unavailable.
    assert!(s
        .migrations
        .as_ref()
        .unwrap_err()
        .contains("Data migrations"));
    assert_eq!(detail.recommendations, "1", "only active ones are counted");
    assert!(s
        .monitoring
        .as_ref()
        .unwrap_err()
        .contains("CloudWatch metrics"));
    assert!(s.alarms.as_ref().unwrap_err().contains("CloudWatch alarms"));
    assert_eq!(detail.cpu, "Unavailable");
    let seen = seen.lock().unwrap();
    let filtered = seen
        .iter()
        .find(|r| r.contains("Action=DescribeDBRecommendations"))
        .unwrap();
    assert!(
        filtered.contains("dbi-resource-id") && filtered.contains(rid),
        "{filtered}"
    );
    let events = seen
        .iter()
        .find(|r| r.contains("Action=DescribeEvents"))
        .unwrap();
    assert!(
        events.contains("SourceType=db-instance")
            && events.contains("SourceIdentifier=orders")
            && events.contains(&format!("Duration={}", sections::EVENT_MINUTES)),
        "{events}"
    );
    assert!(
        !seen.iter().any(|r| r.contains("Action=Modify")
            || r.contains("Action=Delete")
            || r.contains("Action=Download")),
        "read-only"
    );
}

#[tokio::test]
async fn failed_sections_never_fail_the_detail() {
    let (config, _) = routed(|action| match action {
        "DescribeDBInstances" => (200, instance_xml(ARN)),
        _ => (403, DENIED.into()),
    });
    let detail = instance_details(&config, ARN).await.unwrap();
    let s = &detail.sections;
    assert!(s.events.is_err() && s.logs.is_err() && s.integrations.is_err());
    // No resource ID in the fixture: no unscoped recommendations read.
    assert!(s
        .recommendations
        .as_ref()
        .unwrap_err()
        .contains("no resource ID"));
    assert_eq!(detail.recommendations, "Unavailable");
}

#[test]
fn alarms_match_only_this_instance_on_rds_metrics() {
    use aws_sdk_cloudwatch::types::{ComparisonOperator, Dimension, MetricAlarm, StateValue};
    let build = |namespace: &str, id: &str| {
        MetricAlarm::builder()
            .alarm_name("cpu-high")
            .namespace(namespace)
            .metric_name("CPUUtilization")
            .comparison_operator(ComparisonOperator::GreaterThanThreshold)
            .threshold(80.0)
            .state_value(StateValue::Ok)
            .dimensions(
                Dimension::builder()
                    .name("DBInstanceIdentifier")
                    .value(id)
                    .build(),
            )
            .build()
    };
    assert!(sections::watches(&build("AWS/RDS", "orders"), "orders"));
    assert!(!sections::watches(&build("AWS/RDS", "orders-2"), "orders"));
    assert!(!sections::watches(&build("AWS/EC2", "orders"), "orders"));
    let row = sections::alarm(&build("AWS/RDS", "orders"));
    assert_eq!(row.state, "OK");
    assert_eq!(row.condition, "CPUUtilization > 80");
}

#[test]
fn integration_destination_fields_come_from_the_target_arn() {
    let i = aws_sdk_rds::types::Integration::builder()
        .integration_name("orders-to-redshift")
        .integration_arn("arn:aws:rds:us-east-2:123456789012:integration:abc")
        .source_arn(ARN)
        .target_arn("arn:aws:redshift-serverless:us-east-2:210987654321:namespace/n")
        .build();
    let row = sections::integration(&i);
    assert_eq!(row.source, "orders");
    assert_eq!(row.destination_account, "210987654321");
    assert_eq!(row.destination_type, "Amazon Redshift Serverless");
    assert_eq!(
        row.arn,
        "arn:aws:rds:us-east-2:123456789012:integration:abc"
    );
    assert_eq!(sections::destination_type("redshift"), "Amazon Redshift");
    assert_eq!(sections::destination_type("other"), "other");
}

#[test]
fn recommendation_end_time_comes_from_the_issue() {
    use aws_sdk_rds::types::{IssueDetails, PerformanceIssueDetails};
    let row = DbRecommendationFixture::with_end(Some(1_767_225_600));
    assert_eq!(row[0].end_time, "2026-01-01T00:00:00Z");
    assert!(DbRecommendationFixture::with_end(None)[0]
        .end_time
        .is_empty());
    struct DbRecommendationFixture;
    impl DbRecommendationFixture {
        fn with_end(secs: Option<i64>) -> Vec<sections::RdsRecommendation> {
            let mut b = aws_sdk_rds::types::DbRecommendation::builder().resource_arn(ARN);
            if let Some(secs) = secs {
                b = b.issue_details(
                    IssueDetails::builder()
                        .performance_issue_details(
                            PerformanceIssueDetails::builder()
                                .end_time(aws_smithy_types::DateTime::from_secs(secs))
                                .build(),
                        )
                        .build(),
                );
            }
            sections::own_recommendations(&[b.build()], ARN)
        }
    }
}

#[test]
fn memory_is_shown_in_gigabytes() {
    assert_eq!(gigabytes(1024), "1 GB");
    assert_eq!(gigabytes(512), "0.5 GB");
    assert_eq!(gigabytes(16384), "16 GB");
    assert_eq!(gigabytes(1536), "1.5 GB");
}

#[test]
fn data_migrations_match_this_endpoint_through_project_and_provider() {
    use aws_sdk_databasemigration::types::{
        DataMigration, DataMigrationStatistics, DataProviderDescriptor, MigrationProject,
    };
    use std::collections::HashMap;
    let mut own = HashMap::new();
    own.insert(
        "arn:dp:mine".to_string(),
        ("orders.example".to_string(), Some(5432)),
    );
    let project = |arn: &str, provider: &str| {
        MigrationProject::builder()
            .migration_project_arn(arn)
            .target_data_provider_descriptors(
                DataProviderDescriptor::builder()
                    .data_provider_arn(provider)
                    .build(),
            )
            .build()
    };
    let projects = [
        project("arn:mp:mine", "arn:dp:mine"),
        project("arn:mp:other", "arn:dp:other"),
    ];
    let servers = migrations::project_servers(&projects, &own);
    assert_eq!(servers.len(), 1);
    let migration = |name: &str, project: &str| {
        DataMigration::builder()
            .data_migration_name(name)
            .migration_project_arn(project)
            .data_migration_status("RUNNING")
            .data_migration_statistics(
                DataMigrationStatistics::builder()
                    .full_load_percentage(45)
                    .tables_loaded(3)
                    .elapsed_time_millis(1000)
                    .build(),
            )
            .build()
    };
    let rows = migrations::own_migrations(
        &[
            migration("b-mine", "arn:mp:mine"),
            migration("theirs", "arn:mp:other"),
            migration("a-mine", "arn:mp:mine"),
        ],
        &servers,
    );
    let names: Vec<_> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["a-mine", "b-mine"]);
    assert_eq!(rows[0].server_name, "orders.example");
    assert_eq!(rows[0].port, Some(5432));
    assert_eq!(rows[0].progress, "45%");
    assert_eq!(rows[0].tables_loaded, Some(3));
}

#[test]
fn maps_configuration_fields() {
    use aws_sdk_rds::types::{DbParameterGroupStatus, MasterUserSecret, OptionGroupMembership};
    let sdk = DbInstance::builder()
        .db_instance_identifier("orders")
        .db_instance_arn(ARN)
        .engine_lifecycle_support("open-source-rds-extended-support")
        .option_group_memberships(
            OptionGroupMembership::builder()
                .option_group_name("default:postgres-18")
                .status("in-sync")
                .build(),
        )
        .db_parameter_groups(
            DbParameterGroupStatus::builder()
                .db_parameter_group_name("default.postgres18")
                .parameter_apply_status("in-sync")
                .build(),
        )
        .master_user_secret(
            MasterUserSecret::builder()
                .secret_arn("arn:secret")
                .secret_status("active")
                .kms_key_id("arn:key")
                .build(),
        )
        .is_storage_config_upgrade_available(false)
        .database_insights_mode(aws_sdk_rds::types::DatabaseInsightsMode::Standard)
        .monitoring_interval(0)
        .build();
    let d = map(&sdk, "us-east-2");
    assert_eq!(
        d.engine_lifecycle_support,
        "open-source-rds-extended-support"
    );
    assert_eq!(
        d.option_groups,
        [("default:postgres-18".to_string(), "in-sync".to_string())]
    );
    assert_eq!(d.parameter_groups[0].1, "in-sync");
    assert_eq!(d.master_secret_arn, "arn:secret");
    assert_eq!(d.master_secret_status, "active");
    assert_eq!(d.master_secret_kms_key, "arn:key");
    assert_eq!(d.storage_upgrade_available, Some(false));
    assert_eq!(d.database_insights_mode, "standard");
    assert_eq!(d.monitoring_interval, Some(0));
    assert_eq!(d.multi_tenant, None);
}

mod connectivity_tables {
    use super::*;
    use aws_sdk_ec2::types::{ReferencedSecurityGroup, SecurityGroup, SecurityGroupRule};

    fn group(id: &str, name: &str) -> SecurityGroup {
        SecurityGroup::builder()
            .group_id(id)
            .group_name(name)
            .build()
    }

    fn inbound_from(group: &str, from: &str) -> SecurityGroupRule {
        SecurityGroupRule::builder()
            .group_id(group)
            .is_egress(false)
            .referenced_group_info(ReferencedSecurityGroup::builder().group_id(from).build())
            .build()
    }

    #[test]
    fn rules_follow_console_types_and_names() {
        let rules = [
            SecurityGroupRule::builder()
                .group_id("sg-1")
                .is_egress(true)
                .cidr_ipv4("0.0.0.0/0")
                .build(),
            inbound_from("sg-1", "sg-1"),
            SecurityGroupRule::builder()
                .group_id("sg-1")
                .is_egress(false)
                .prefix_list_id("pl-1")
                .build(),
        ];
        let rows = connectivity::rule_rows(&[group("sg-1", "default")], &rules);
        let view: Vec<_> = rows
            .iter()
            .map(|r| {
                (
                    r.security_group.as_str(),
                    r.rule_type.as_str(),
                    r.rule.as_str(),
                )
            })
            .collect();
        assert_eq!(
            view,
            [
                ("default (sg-1)", "CIDR/IP - Outbound", "0.0.0.0/0"),
                ("default (sg-1)", "EC2 Security Group - Inbound", "sg-1"),
                ("default (sg-1)", "Prefix list - Inbound", "pl-1"),
            ]
        );
    }

    #[test]
    fn compute_pairs_need_the_console_group_names() {
        let db = [group("sg-db", "rds-ec2-1"), group("sg-default", "default")];
        let referenced = [
            group("sg-ec2", "ec2-rds-1"),
            group("sg-other", "web"),
            group("sg-fn", "lambda-rds-1"),
        ];
        let rules = [
            inbound_from("sg-db", "sg-ec2"),
            inbound_from("sg-db", "sg-other"),
            inbound_from("sg-default", "sg-ec2"),
            inbound_from("sg-db", "sg-fn"),
        ];
        let pairs = connectivity::pairs(&db, &rules, &referenced);
        assert_eq!(
            pairs,
            [("sg-db".to_string(), "sg-ec2".to_string(), "EC2 instance")],
            "only rds-ec2 to ec2-rds"
        );
    }

    #[test]
    fn group_ids_drop_the_status() {
        assert_eq!(
            connectivity::group_ids(&["sg-1 (active)".into(), "sg-2".into()]),
            ["sg-1", "sg-2"]
        );
    }

    #[test]
    fn iam_roles_come_from_the_instance() {
        let sdk = DbInstance::builder()
            .associated_roles(
                aws_sdk_rds::types::DbInstanceRole::builder()
                    .role_arn("arn:aws:iam::1:role/r")
                    .feature_name("s3Import")
                    .status("ACTIVE")
                    .build(),
            )
            .build();
        let d = map(&sdk, "us-east-2");
        assert_eq!(d.iam_roles[0].feature, "s3Import");
        assert_eq!(d.iam_roles[0].status, "ACTIVE");
    }

    #[tokio::test]
    async fn a_standalone_instance_is_its_own_replication_row() {
        let (config, _) = config(200, instance_xml(ARN));
        let detail = instance_details(&config, ARN).await.unwrap();
        let rows = detail.sections.replication.as_ref().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].identifier, "orders");
        assert_eq!(rows[0].role, "Instance");
        assert!(rows[0].lag.is_empty(), "no lag read for a primary");
        assert_eq!(detail.sections.compute, Ok(vec![]), "no groups, no reads");
    }
}
