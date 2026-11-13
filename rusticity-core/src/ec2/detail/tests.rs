use super::*;
use aws_sdk_ec2::types::{
    AttachmentStatus, CpuOptions, EbsInstanceBlockDevice, GroupIdentifier, HostnameType,
    HttpTokensState, InstanceBlockDeviceMapping, InstanceNetworkInterface,
    InstanceNetworkInterfaceAssociation, InstanceNetworkInterfaceAttachment,
    InstancePrivateIpAddress, InstanceState, InstanceStateName, InstanceStatusSummary, Placement,
    PrivateDnsNameOptionsResponse, ReferencedSecurityGroup, SummaryStatus, Tag,
};
use aws_smithy_http_client::test_util::infallible_client_fn;
use std::sync::{Arc, Mutex};

const ID: &str = "i-0123456789abcdef0";

fn instance_xml(id: &str) -> String {
    format!(
        r#"<DescribeInstancesResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
  <requestId>r</requestId>
  <reservationSet><item>
    <reservationId>r-1</reservationId><ownerId>123456789012</ownerId>
    <instancesSet><item>
      <instanceId>{id}</instanceId>
      <imageId>ami-1</imageId>
      <instanceState><code>16</code><name>running</name></instanceState>
      <instanceType>m5.large</instanceType>
      <groupSet><item><groupId>sg-1</groupId><groupName>web</groupName></item></groupSet>
      <tagSet><item><key>Name</key><value>web-1</value></item></tagSet>
      <blockDeviceMapping><item><deviceName>/dev/xvda</deviceName>
        <ebs><volumeId>vol-1</volumeId><status>attached</status>
        <deleteOnTermination>true</deleteOnTermination></ebs></item></blockDeviceMapping>
    </item></instancesSet>
  </item></reservationSet>
</DescribeInstancesResponse>"#
    )
}

const DENIED: &str = r#"<Response><Errors><Error><Code>UnauthorizedOperation</Code><Message>no</Message></Error></Errors><RequestID>r</RequestID></Response>"#;

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
        .credentials_provider(aws_sdk_ec2::config::SharedCredentialsProvider::new(
            aws_sdk_ec2::config::Credentials::new("test", "test", None, None, "test"),
        ))
        .retry_config(aws_config::retry::RetryConfig::disabled())
        .http_client(http)
        .build();
    (
        AwsConfig::from_sdk_config(sdk_config, "us-east-2", "123456789012"),
        seen,
    )
}

/// The instance read succeeds; every other read is denied.
fn instance_only(id: &'static str) -> (AwsConfig, Arc<Mutex<Vec<String>>>) {
    routed(move |action| match action {
        "DescribeInstances" => (200, instance_xml(id)),
        _ => (403, DENIED.into()),
    })
}

#[tokio::test]
async fn one_describe_call_scoped_by_instance_id() {
    let (config, seen) = instance_only(ID);
    let d = instance_details(&config, ID).await.unwrap();
    assert_eq!(d.instance_id, ID);
    assert_eq!(d.name, "web-1");
    assert_eq!(d.owner_id, "123456789012");
    assert_eq!(d.reservation_id, "r-1");
    assert_eq!(d.security_groups, [("sg-1".into(), "web".into())]);
    let seen = seen.lock().unwrap();
    let reads: Vec<_> = seen
        .iter()
        .filter(|r| r.contains("Action=DescribeInstances&"))
        .collect();
    assert_eq!(reads.len(), 1, "exactly one instance read");
    assert!(
        reads[0].contains(&format!("InstanceId.1={ID}")),
        "{}",
        reads[0]
    );
}

#[tokio::test]
async fn failed_optional_reads_mark_only_their_fields_and_sections() {
    let (config, seen) = instance_only(ID);
    let d = instance_details(&config, ID).await.unwrap();
    assert_eq!(d.ami_name, "Unavailable");
    assert_eq!(d.termination_protection, "Unavailable");
    assert_eq!(d.stop_protection, "Unavailable");
    // m5 is not burstable: no credit read at all.
    assert_eq!(d.credit_specification, "");
    let s = &d.sections;
    assert!(s
        .status_checks
        .as_ref()
        .unwrap_err()
        .starts_with("Status checks unavailable"));
    assert!(s.inbound_rules.is_err() && s.outbound_rules.is_err());
    assert!(s
        .block_devices
        .as_ref()
        .unwrap_err()
        .starts_with("Block devices unavailable"));
    assert!(s.alarms.is_err());
    assert!(s.monitoring.is_err());
    let seen = seen.lock().unwrap();
    for action in [
        "DescribeImages",
        "DescribeInstanceAttribute",
        "DescribeInstanceStatus",
        "DescribeSecurityGroupRules",
        "DescribeVolumes",
    ] {
        assert!(
            seen.iter()
                .any(|r| r.contains(&format!("Action={action}&"))),
            "{action}"
        );
    }
    assert!(!seen
        .iter()
        .any(|r| r.contains("DescribeInstanceCreditSpecifications")));
}

#[tokio::test]
async fn mismatched_identity_is_rejected() {
    let (config, _) = instance_only("i-0fffffffffffffff0");
    let error = instance_details(&config, ID).await.unwrap_err();
    assert!(error.to_string().contains("mismatched identity"), "{error}");
}

#[tokio::test]
async fn no_instance_is_an_error() {
    let (config, _) = routed(|_| {
        (
            200,
            r#"<DescribeInstancesResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/"><reservationSet/></DescribeInstancesResponse>"#.into(),
        )
    });
    let error = instance_details(&config, ID).await.unwrap_err();
    assert!(
        error.to_string().contains("returned 0 instances"),
        "{error}"
    );
}

#[tokio::test]
async fn malformed_ids_are_rejected_before_any_request() {
    let (config, seen) = instance_only(ID);
    for id in [
        "",
        "i-",
        "i-123",
        "i-0123456789ABCDEF0",
        "vol-0123456789abcdef0",
        "i-0123456789abcdef0 ",
    ] {
        assert!(instance_details(&config, id).await.is_err(), "{id:?}");
    }
    assert!(seen.lock().unwrap().is_empty());
    assert!(validate_instance_id("i-12345678").is_ok());
}

#[tokio::test]
async fn protection_attributes_and_the_ami_are_read_when_allowed() {
    let (config, seen) = routed(|action| {
        match action {
        "DescribeInstances" => (200, instance_xml(ID)),
        "DescribeImages" => (
            200,
            r#"<DescribeImagesResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/"><imagesSet><item>
                <imageId>ami-1</imageId><name>al2023</name><imageLocation>amazon/al2023</imageLocation>
                <imageAllowed>true</imageAllowed></item></imagesSet></DescribeImagesResponse>"#
                .into(),
        ),
        "DescribeInstanceAttribute" => (
            200,
            format!(
                r#"<DescribeInstanceAttributeResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/"><instanceId>{ID}</instanceId><disableApiTermination><value>true</value></disableApiTermination><disableApiStop><value>false</value></disableApiStop></DescribeInstanceAttributeResponse>"#
            ),
        ),
        _ => (403, DENIED.into()),
    }
    });
    let d = instance_details(&config, ID).await.unwrap();
    assert_eq!(
        (
            d.ami_name.as_str(),
            d.ami_location.as_str(),
            d.allowed_image.as_str()
        ),
        ("al2023", "amazon/al2023", "Yes")
    );
    assert_eq!(d.termination_protection, "Enabled");
    assert_eq!(d.stop_protection, "Disabled");
    let seen = seen.lock().unwrap();
    assert!(seen
        .iter()
        .any(|r| r.contains("Attribute=disableApiTermination")));
    assert!(seen.iter().any(|r| r.contains("Attribute=disableApiStop")));
}

#[tokio::test]
async fn an_attribute_answer_for_another_instance_is_unavailable() {
    let (config, _) = routed(|action| {
        match action {
        "DescribeInstances" => (200, instance_xml(ID)),
        "DescribeInstanceAttribute" => (
            200,
            r#"<DescribeInstanceAttributeResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/"><instanceId>i-0fffffffffffffff0</instanceId><disableApiTermination><value>true</value></disableApiTermination></DescribeInstanceAttributeResponse>"#.into(),
        ),
        _ => (403, DENIED.into()),
    }
    });
    let d = instance_details(&config, ID).await.unwrap();
    assert_eq!(d.termination_protection, "Unavailable");
}

fn sdk_instance() -> Instance {
    let association = |ip: &str, owner: &str| {
        InstanceNetworkInterfaceAssociation::builder()
            .public_ip(ip)
            .ip_owner_id(owner)
            .build()
    };
    Instance::builder()
        .instance_id(ID)
        .state(
            InstanceState::builder()
                .name(InstanceStateName::Stopped)
                .build(),
        )
        .instance_type(aws_sdk_ec2::types::InstanceType::T3Micro)
        .placement(
            Placement::builder()
                .availability_zone("us-east-2a")
                .host_resource_group_arn(
                    "arn:aws:resource-groups:us-east-2:123456789012:group/hosts",
                )
                .partition_number(2)
                .build(),
        )
        .cpu_options(
            CpuOptions::builder()
                .core_count(2)
                .threads_per_core(2)
                .build(),
        )
        .private_dns_name_options(
            PrivateDnsNameOptionsResponse::builder()
                .hostname_type(HostnameType::ResourceName)
                .enable_resource_name_dns_a_record(true)
                .build(),
        )
        .metadata_options(
            aws_sdk_ec2::types::InstanceMetadataOptionsResponse::builder()
                .http_tokens(HttpTokensState::Required)
                .build(),
        )
        .private_ip_address("10.0.0.5")
        .network_interfaces(
            InstanceNetworkInterface::builder()
                .network_interface_id("eni-1")
                .private_ip_address("10.0.0.5")
                .private_ip_addresses(
                    InstancePrivateIpAddress::builder()
                        .private_ip_address("10.0.0.9")
                        .primary(false)
                        .association(association("52.0.0.1", "123456789012"))
                        .build(),
                )
                .private_ip_addresses(
                    InstancePrivateIpAddress::builder()
                        .private_ip_address("10.0.0.5")
                        .primary(true)
                        .association(association("3.0.0.1", "amazon"))
                        .build(),
                )
                .attachment(
                    InstanceNetworkInterfaceAttachment::builder()
                        .status(AttachmentStatus::Attached)
                        .delete_on_termination(true)
                        .build(),
                )
                .groups(
                    GroupIdentifier::builder()
                        .group_id("sg-1")
                        .group_name("web")
                        .build(),
                )
                .interface_type("interface")
                .build(),
        )
        .block_device_mappings(
            InstanceBlockDeviceMapping::builder()
                .device_name("/dev/xvda")
                .ebs(
                    EbsInstanceBlockDevice::builder()
                        .volume_id("vol-1")
                        .status(AttachmentStatus::Attached)
                        .delete_on_termination(true)
                        .build(),
                )
                .build(),
        )
        .block_device_mappings(
            InstanceBlockDeviceMapping::builder()
                .device_name("/dev/sdf")
                .ebs(EbsInstanceBlockDevice::builder().volume_id("vol-2").build())
                .build(),
        )
        .tags(Tag::builder().key("Name").value("web-1").build())
        .build()
}

#[test]
fn maps_addresses_placement_and_options_and_keeps_absent_values_absent() {
    let reservation = Reservation::builder()
        .owner_id("123456789012")
        .reservation_id("r-1")
        .build();
    let d = map(&sdk_instance(), &reservation);
    assert_eq!(d.state, "stopped");
    assert_eq!(d.instance_type, "t3.micro");
    assert_eq!(d.private_ipv4s, ["10.0.0.5", "10.0.0.9"]);
    assert_eq!(d.secondary_private_ipv4s, ["10.0.0.9"]);
    assert_eq!(d.elastic_ips, ["52.0.0.1"]);
    assert_eq!(d.auto_assigned_ip, "3.0.0.1");
    assert_eq!(d.hostname_type, "resource-name");
    assert_eq!(d.resource_name_a_record, Some(true));
    assert_eq!(d.http_tokens, "required");
    assert_eq!(d.host_resource_group, "hosts");
    assert_eq!(d.partition_number, Some(2));
    assert_eq!(d.vcpus, Some(4));
    assert_eq!(d.name, "web-1");
    assert_eq!(d.owner_id, "123456789012");
    // Absent stays absent.
    assert_eq!(d.public_ipv4, "");
    assert_eq!(d.lifecycle, "");
    assert_eq!(d.ebs_optimized, None);
    assert_eq!(d.managed, None);
    let n = &d.network_interfaces[0];
    assert_eq!(n.id, "eni-1");
    assert_eq!(n.attachment_status, "attached");
    assert_eq!(n.delete_on_termination, Some(true));
    assert_eq!(n.security_groups, ["web"]);
}

#[test]
fn block_devices_follow_the_mappings_with_their_volumes() {
    let volume = Volume::builder()
        .volume_id("vol-1")
        .size(8)
        .encrypted(true)
        .kms_key_id("arn:aws:kms:us-east-2:123456789012:key/k")
        .build();
    let rows = block_devices(&sdk_instance(), &[volume]);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].device_name, "/dev/xvda");
    assert_eq!(rows[0].size_gib, Some(8));
    assert_eq!(rows[0].encrypted, Some(true));
    assert_eq!(rows[0].attachment_status, "attached");
    // A volume the read did not return keeps its mapping, nothing invented.
    assert_eq!(rows[1].volume_id, "vol-2");
    assert_eq!(rows[1].size_gib, None);
    assert_eq!(rows[1].encrypted, None);
}

#[test]
fn rules_split_by_direction_for_the_instance_groups_only() {
    let rule = |id: &str, group: &str, egress: bool| {
        SecurityGroupRule::builder()
            .security_group_rule_id(id)
            .group_id(group)
            .is_egress(egress)
    };
    let rules = [
        rule("sgr-2", "sg-1", false)
            .ip_protocol("tcp")
            .from_port(443)
            .to_port(443)
            .cidr_ipv4("0.0.0.0/0")
            .tags(Tag::builder().key("Name").value("https").build())
            .build(),
        rule("sgr-1", "sg-1", false)
            .ip_protocol("tcp")
            .from_port(1024)
            .to_port(2048)
            .referenced_group_info(ReferencedSecurityGroup::builder().group_id("sg-9").build())
            .build(),
        rule("sgr-3", "sg-1", true)
            .ip_protocol("-1")
            .from_port(-1)
            .to_port(-1)
            .cidr_ipv6("::/0")
            .build(),
        rule("sgr-4", "sg-other", false).ip_protocol("udp").build(),
    ];
    let (inbound, outbound) = rule_rows(&rules, &[("sg-1".into(), "web".into())]);
    let ids: Vec<_> = inbound.iter().map(|r| r.rule_id.as_str()).collect();
    assert_eq!(ids, ["sgr-1", "sgr-2"]);
    assert_eq!(inbound[0].port_range, "1024 - 2048");
    assert_eq!(inbound[0].peer, "sg-9");
    assert_eq!(inbound[1].name, "https");
    assert_eq!(inbound[1].port_range, "443");
    assert_eq!(inbound[1].protocol, "TCP");
    assert_eq!(inbound[1].security_group, "web");
    assert_eq!(outbound.len(), 1);
    assert_eq!(
        (
            outbound[0].port_range.as_str(),
            outbound[0].protocol.as_str()
        ),
        ("All", "All")
    );
    assert_eq!(outbound[0].peer, "::/0");
}

#[test]
fn status_checks_keep_raw_values() {
    let summary = |status| InstanceStatusSummary::builder().status(status).build();
    let s = InstanceStatus::builder()
        .instance_id(ID)
        .system_status(summary(SummaryStatus::Ok))
        .instance_status(summary(SummaryStatus::Impaired))
        .build();
    let c = checks(&s);
    assert_eq!(c.system, "ok");
    assert_eq!(c.instance, "impaired");
    assert_eq!(c.attached_ebs, "");
}

#[test]
fn utc_matches_the_list_format() {
    let time = aws_smithy_types::DateTime::from_secs(1_700_000_000);
    assert_eq!(utc(Some(&time)), "2023-11-14 22:13:20 (UTC)");
    assert_eq!(utc(None), "");
}
