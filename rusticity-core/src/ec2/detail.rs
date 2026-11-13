//! Read-only detail for one EC2 instance. One DescribeInstances call scoped to
//! the instance ID fills the summary and most tab fields, tags and network
//! interfaces included; the returned identity is verified against the
//! requested ID. The rest comes from bounded parallel reads, each failing on
//! its own: the AMI, the termination and stop protection attributes, the
//! credit specification, status checks, CloudWatch alarms and metrics,
//! security group rules and the attached volumes. Never a mutating call.
use super::{utc, Ec2Client};
use crate::config::AwsConfig;
use anyhow::{anyhow, bail, Context, Result};
use aws_sdk_ec2::types::{
    Filter, Instance, InstanceAttributeName, InstanceStatus, Reservation, SecurityGroupRule, Volume,
};
use std::future::Future;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(30);
/// Bound on each optional read after the instance read succeeded.
const ENRICH_TIMEOUT: Duration = Duration::from_secs(15);

/// One section of a detail tab: its rows, or why they are unavailable.
pub type Section<T> = Result<Vec<T>, String>;

/// One CloudWatch alarm on the instance, as the RDS detail shows them.
pub type Ec2Alarm = crate::rds::detail::sections::RdsAlarm;

/// Monitoring charts, in tab order. Each is the 5-minute average over the
/// last 3 hours.
pub const METRICS: [&str; 6] = [
    "CPUUtilization",
    "NetworkIn",
    "NetworkOut",
    "NetworkPacketsIn",
    "NetworkPacketsOut",
    "MetadataNoToken",
];

#[derive(Clone, Debug, PartialEq)]
pub struct Ec2Metric {
    pub name: &'static str,
    pub points: Vec<(i64, f64)>,
}

/// Raw `SummaryStatus` per check: `ok`, `impaired`, `initializing`,
/// `insufficient-data` or `not-applicable`; empty when not reported.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatusChecks {
    pub system: String,
    pub instance: String,
    pub attached_ebs: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ec2SecurityGroupRule {
    /// The rule `Name` tag.
    pub name: String,
    pub rule_id: String,
    pub port_range: String,
    pub protocol: String,
    /// Source of an inbound rule, destination of an outbound one.
    pub peer: String,
    /// Name of the group that holds the rule.
    pub security_group: String,
    pub description: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ec2BlockDevice {
    pub volume_id: String,
    pub device_name: String,
    pub size_gib: Option<i32>,
    pub attachment_status: String,
    pub attach_time: String,
    pub encrypted: Option<bool>,
    pub kms_key_id: String,
    pub delete_on_termination: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ec2NetworkInterface {
    pub id: String,
    pub description: String,
    pub ipv4_prefixes: Vec<String>,
    pub ipv6_prefixes: Vec<String>,
    pub public_ipv4: String,
    pub private_ipv4: String,
    pub private_dns: String,
    pub ipv6_addresses: Vec<String>,
    pub attach_time: String,
    pub attachment_status: String,
    pub vpc_id: String,
    pub subnet_id: String,
    pub delete_on_termination: Option<bool>,
    /// Group names.
    pub security_groups: Vec<String>,
    pub interface_type: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sections {
    pub status_checks: Result<StatusChecks, String>,
    pub alarms: Section<Ec2Alarm>,
    pub inbound_rules: Section<Ec2SecurityGroupRule>,
    pub outbound_rules: Section<Ec2SecurityGroupRule>,
    pub block_devices: Section<Ec2BlockDevice>,
    pub monitoring: Section<Ec2Metric>,
}

impl Default for Sections {
    fn default() -> Self {
        Self {
            status_checks: Ok(StatusChecks::default()),
            alarms: Ok(Vec::new()),
            inbound_rules: Ok(Vec::new()),
            outbound_rules: Ok(Vec::new()),
            block_devices: Ok(Vec::new()),
            monitoring: Ok(Vec::new()),
        }
    }
}

/// Fresh metadata. Absent values stay empty or `None` rather than invented;
/// raw API values are kept for the view to name.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ec2InstanceDetail {
    pub instance_id: String,
    pub name: String,
    pub state: String,
    pub instance_type: String,
    pub public_ipv4: String,
    /// Every private IPv4 address across the interfaces, primary first.
    pub private_ipv4s: Vec<String>,
    pub secondary_private_ipv4s: Vec<String>,
    pub ipv6_addresses: Vec<String>,
    pub public_dns: String,
    pub private_dns: String,
    /// `ip-name` or `resource-name`.
    pub hostname_type: String,
    pub resource_name_a_record: Option<bool>,
    /// Public addresses the account owns.
    pub elastic_ips: Vec<String>,
    /// The Amazon-owned public address, if any.
    pub auto_assigned_ip: String,
    pub carrier_ips: Vec<String>,
    pub vpc_id: String,
    pub subnet_id: String,
    pub availability_zone: String,
    pub outpost_arn: String,
    pub iam_instance_profile_arn: String,
    /// `required` or `optional`.
    pub http_tokens: String,
    /// `enabled` or `disabled`.
    pub metadata_tags: String,
    pub managed: Option<bool>,
    pub operator: String,
    // Instance details
    pub image_id: String,
    /// `enabled` or `disabled`.
    pub monitoring: String,
    pub platform_details: String,
    pub launch_time: String,
    pub reboot_migration: String,
    pub auto_recovery: String,
    /// `spot`, `scheduled` or `capacity-block`; empty for on-demand.
    pub lifecycle: String,
    pub hibernation: Option<bool>,
    pub ami_launch_index: Option<i32>,
    pub key_name: String,
    pub state_transition_reason: String,
    pub state_transition_message: String,
    pub kernel_id: String,
    pub ramdisk_id: String,
    pub usage_operation: String,
    pub owner_id: String,
    pub enclaves: Option<bool>,
    pub boot_mode: String,
    pub current_boot_mode: String,
    // Host and placement group
    pub host_id: String,
    pub affinity: String,
    pub placement_group: String,
    pub placement_group_id: String,
    pub host_resource_group: String,
    pub tenancy: String,
    pub virtualization_type: String,
    pub reservation_id: String,
    pub partition_number: Option<i32>,
    pub vcpus: Option<i32>,
    // Capacity reservation
    pub capacity_reservation_id: String,
    pub capacity_reservation_preference: String,
    // Storage
    pub root_device_name: String,
    pub root_device_type: String,
    pub ebs_optimized: Option<bool>,
    /// (ID, name) per security group.
    pub security_groups: Vec<(String, String)>,
    pub tags: Vec<(String, String)>,
    pub network_interfaces: Vec<Ec2NetworkInterface>,
    // From the other reads: "Unavailable" when that read fails or times out.
    pub ami_name: String,
    pub ami_location: String,
    /// "Yes", "No", or empty when the AMI does not report it.
    pub allowed_image: String,
    /// "Enabled" or "Disabled".
    pub termination_protection: String,
    pub stop_protection: String,
    /// `standard` or `unlimited` for a burstable type; empty otherwise.
    pub credit_specification: String,
    /// Status and alarms, Security rules, Storage volumes and Monitoring.
    pub sections: Sections,
}

/// `i-` and 8 or 17 lowercase hex digits.
pub(crate) fn validate_instance_id(id: &str) -> Result<()> {
    let hex = id.strip_prefix("i-").unwrap_or_default();
    if !matches!(hex.len(), 8 | 17) || !hex.bytes().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f'))
    {
        bail!("not an EC2 instance ID: {id:?}");
    }
    Ok(())
}

pub(crate) async fn instance_details(config: &AwsConfig, id: &str) -> Result<Ec2InstanceDetail> {
    validate_instance_id(id)?;
    let out = tokio::time::timeout(
        TIMEOUT,
        config
            .ec2_client()
            .describe_instances()
            .instance_ids(id)
            .send(),
    )
    .await
    .map_err(|_| anyhow!("EC2 DescribeInstances timed out after {TIMEOUT:?}"))?
    .context("EC2 DescribeInstances request failed")?;
    let found: Vec<_> = out
        .reservations()
        .iter()
        .flat_map(|r| r.instances().iter().map(move |i| (r, i)))
        .collect();
    let [(reservation, instance)] = found.as_slice() else {
        bail!(
            "EC2 DescribeInstances returned {} instances for {id:?}; expected exactly one",
            found.len()
        );
    };
    if instance.instance_id() != Some(id) {
        bail!(
            "EC2 DescribeInstances returned {:?} for requested instance {id:?}; refusing to \
             present a mismatched identity",
            instance.instance_id().unwrap_or_default()
        );
    }
    let mut detail = map(instance, reservation);
    let group_ids: Vec<String> = detail
        .security_groups
        .iter()
        .map(|(id, _)| id.clone())
        .collect();
    let volume_ids: Vec<String> = instance
        .block_device_mappings()
        .iter()
        .filter_map(|m| m.ebs()?.volume_id())
        .map(str::to_string)
        .collect();
    let client = Ec2Client::new(config.clone());
    let (
        (ami_name, ami_location, allowed_image),
        termination_protection,
        stop_protection,
        credit_specification,
        status_checks,
        alarms,
        rules,
        volumes,
        monitoring,
    ) = tokio::join!(
        image(config, &detail.image_id),
        protection(config, id, InstanceAttributeName::DisableApiTermination),
        protection(config, id, InstanceAttributeName::DisableApiStop),
        credits(config, id, &detail.instance_type),
        bounded("Status checks", status_checks(config, id)),
        bounded("CloudWatch alarms", alarms(config, id)),
        bounded("Security group rules", rules(config, &group_ids)),
        bounded("Block devices", volumes(config, &volume_ids)),
        bounded("CloudWatch metrics", monitoring(&client, id)),
    );
    detail.ami_name = ami_name;
    detail.ami_location = ami_location;
    detail.allowed_image = allowed_image;
    detail.termination_protection = termination_protection;
    detail.stop_protection = stop_protection;
    detail.credit_specification = credit_specification;
    let (inbound_rules, outbound_rules) = match rules {
        Ok(rules) => {
            let (inbound, outbound) = rule_rows(&rules, &detail.security_groups);
            (Ok(inbound), Ok(outbound))
        }
        Err(error) => (Err(error.clone()), Err(error)),
    };
    detail.sections = Sections {
        status_checks,
        alarms,
        inbound_rules,
        outbound_rules,
        block_devices: volumes.map(|volumes| block_devices(instance, &volumes)),
        monitoring,
    };
    Ok(detail)
}

async fn bounded<T>(what: &str, read: impl Future<Output = Result<T>>) -> Result<T, String> {
    match tokio::time::timeout(ENRICH_TIMEOUT, read).await {
        Ok(Ok(rows)) => Ok(rows),
        Ok(Err(error)) => {
            tracing::warn!("EC2 {what} unavailable: {error:#}");
            Err(format!("{what} unavailable: {error:#}"))
        }
        Err(_) => Err(format!(
            "{what} unavailable: timed out after {ENRICH_TIMEOUT:?}"
        )),
    }
}

/// A scalar field from an optional read: the value, or "Unavailable".
async fn field<T>(what: &str, read: impl Future<Output = Result<T>>) -> Option<T> {
    match tokio::time::timeout(ENRICH_TIMEOUT, read).await {
        Ok(Ok(value)) => Some(value),
        Ok(Err(error)) => {
            tracing::warn!("EC2 {what} unavailable: {error:#}");
            None
        }
        Err(_) => None,
    }
}

const UNAVAILABLE: &str = "Unavailable";

/// AMI name, AMI location and Allowed image. An AMI the account can no
/// longer see leaves them empty.
async fn image(config: &AwsConfig, image_id: &str) -> (String, String, String) {
    if image_id.is_empty() {
        return Default::default();
    }
    let read = async {
        let out = config
            .ec2_client()
            .describe_images()
            .image_ids(image_id)
            .send()
            .await?;
        Ok::<_, anyhow::Error>(
            out.images()
                .iter()
                .find(|i| i.image_id() == Some(image_id))
                .map(|i| {
                    (
                        i.name().unwrap_or_default().to_string(),
                        i.image_location().unwrap_or_default().to_string(),
                        match i.image_allowed() {
                            Some(true) => "Yes".into(),
                            Some(false) => "No".into(),
                            None => String::new(),
                        },
                    )
                })
                .unwrap_or_default(),
        )
    };
    field("DescribeImages", read)
        .await
        .unwrap_or_else(|| (UNAVAILABLE.into(), UNAVAILABLE.into(), UNAVAILABLE.into()))
}

/// Termination or stop protection: "Enabled", "Disabled" or "Unavailable".
async fn protection(config: &AwsConfig, id: &str, attribute: InstanceAttributeName) -> String {
    let read = async {
        let out = config
            .ec2_client()
            .describe_instance_attribute()
            .instance_id(id)
            .attribute(attribute.clone())
            .send()
            .await?;
        if out.instance_id() != Some(id) {
            bail!("DescribeInstanceAttribute answered for another instance");
        }
        let value = match attribute {
            InstanceAttributeName::DisableApiStop => out.disable_api_stop(),
            _ => out.disable_api_termination(),
        };
        Ok(value.and_then(|v| v.value()))
    };
    match field("DescribeInstanceAttribute", read).await {
        Some(Some(true)) => "Enabled".into(),
        Some(_) => "Disabled".into(),
        None => UNAVAILABLE.into(),
    }
}

/// The CPU credit option of a burstable (T) instance; empty for other types.
async fn credits(config: &AwsConfig, id: &str, instance_type: &str) -> String {
    if !instance_type.starts_with('t') {
        return String::new();
    }
    let read = async {
        let out = config
            .ec2_client()
            .describe_instance_credit_specifications()
            .instance_ids(id)
            .send()
            .await?;
        Ok::<_, anyhow::Error>(
            out.instance_credit_specifications()
                .iter()
                .find(|c| c.instance_id() == Some(id))
                .and_then(|c| c.cpu_credits())
                .unwrap_or_default()
                .to_string(),
        )
    };
    field("DescribeInstanceCreditSpecifications", read)
        .await
        .unwrap_or_else(|| UNAVAILABLE.into())
}

/// The checks of this instance; all empty when not reported.
async fn status_checks(config: &AwsConfig, id: &str) -> Result<StatusChecks> {
    let out = config
        .ec2_client()
        .describe_instance_status()
        .instance_ids(id)
        .include_all_instances(true)
        .send()
        .await?;
    Ok(out
        .instance_statuses()
        .iter()
        .find(|s| s.instance_id() == Some(id))
        .map(checks)
        .unwrap_or_default())
}

pub(crate) fn checks(s: &InstanceStatus) -> StatusChecks {
    let raw = |status: Option<&aws_sdk_ec2::types::SummaryStatus>| {
        status.map(|s| s.as_str().to_string()).unwrap_or_default()
    };
    StatusChecks {
        system: raw(s.system_status().and_then(|v| v.status())),
        instance: raw(s.instance_status().and_then(|v| v.status())),
        attached_ebs: raw(s.attached_ebs_status().and_then(|v| v.status())),
    }
}

/// Alarms in any namespace whose `InstanceId` dimension is this instance.
async fn alarms(config: &AwsConfig, id: &str) -> Result<Vec<Ec2Alarm>> {
    let pages = config
        .cloudwatch_client()
        .describe_alarms()
        .into_paginator()
        .send()
        .collect::<Result<Vec<_>, _>>()
        .await?;
    let mut rows: Vec<_> = pages
        .iter()
        .flat_map(|page| page.metric_alarms())
        .filter(|a| {
            a.dimensions()
                .iter()
                .any(|d| d.name() == Some("InstanceId") && d.value() == Some(id))
        })
        .map(crate::rds::detail::sections::alarm)
        .collect();
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(rows)
}

async fn rules(config: &AwsConfig, group_ids: &[String]) -> Result<Vec<SecurityGroupRule>> {
    if group_ids.is_empty() {
        return Ok(Vec::new());
    }
    let pages = config
        .ec2_client()
        .describe_security_group_rules()
        .filters(
            Filter::builder()
                .name("group-id")
                .set_values(Some(group_ids.to_vec()))
                .build(),
        )
        .into_paginator()
        .send()
        .collect::<Result<Vec<_>, _>>()
        .await?;
    Ok(pages
        .iter()
        .flat_map(|p| p.security_group_rules().to_vec())
        .collect())
}

/// `tcp` -> `TCP`, `-1` -> `All`.
fn protocol(raw: &str) -> String {
    match raw {
        "-1" => "All".into(),
        "icmpv6" => "ICMPv6".into(),
        other => other.to_uppercase(),
    }
}

/// `22`, `0 - 65535`, or `All` for every port.
fn port_range(rule: &SecurityGroupRule) -> String {
    match (rule.from_port(), rule.to_port()) {
        _ if rule.ip_protocol() == Some("-1") => "All".into(),
        (Some(-1), _) | (None, _) => "All".into(),
        (Some(from), Some(to)) if from == to => from.to_string(),
        (Some(from), Some(to)) => format!("{from} - {to}"),
        (Some(from), None) => from.to_string(),
    }
}

/// Inbound and outbound rows for the instance groups, only those groups.
pub(crate) fn rule_rows(
    rules: &[SecurityGroupRule],
    groups: &[(String, String)],
) -> (Vec<Ec2SecurityGroupRule>, Vec<Ec2SecurityGroupRule>) {
    let text = |v: Option<&str>| v.unwrap_or_default().to_string();
    let (mut inbound, mut outbound) = (Vec::new(), Vec::new());
    for r in rules {
        let Some((_, group_name)) = groups
            .iter()
            .find(|(id, _)| Some(id.as_str()) == r.group_id())
        else {
            continue;
        };
        let peer = r
            .cidr_ipv4()
            .or(r.cidr_ipv6())
            .or(r.prefix_list_id())
            .or(r.referenced_group_info().and_then(|g| g.group_id()));
        let row = Ec2SecurityGroupRule {
            name: r
                .tags()
                .iter()
                .find(|t| t.key() == Some("Name"))
                .map(|t| text(t.value()))
                .unwrap_or_default(),
            rule_id: text(r.security_group_rule_id()),
            port_range: port_range(r),
            protocol: protocol(r.ip_protocol().unwrap_or_default()),
            peer: text(peer),
            security_group: group_name.clone(),
            description: text(r.description()),
        };
        if r.is_egress() == Some(true) {
            outbound.push(row);
        } else {
            inbound.push(row);
        }
    }
    for rows in [&mut inbound, &mut outbound] {
        rows.sort_by(|a, b| (&a.security_group, &a.rule_id).cmp(&(&b.security_group, &b.rule_id)));
    }
    (inbound, outbound)
}

async fn volumes(config: &AwsConfig, ids: &[String]) -> Result<Vec<Volume>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(config
        .ec2_client()
        .describe_volumes()
        .set_volume_ids(Some(ids.to_vec()))
        .send()
        .await?
        .volumes()
        .to_vec())
}

/// One row per EBS mapping, in mapping order, with its volume details.
pub(crate) fn block_devices(instance: &Instance, volumes: &[Volume]) -> Vec<Ec2BlockDevice> {
    let text = |v: Option<&str>| v.unwrap_or_default().to_string();
    instance
        .block_device_mappings()
        .iter()
        .filter_map(|m| {
            let ebs = m.ebs()?;
            let volume = volumes.iter().find(|v| v.volume_id() == ebs.volume_id());
            Some(Ec2BlockDevice {
                volume_id: text(ebs.volume_id()),
                device_name: text(m.device_name()),
                size_gib: volume.and_then(|v| v.size()),
                attachment_status: ebs
                    .status()
                    .map(|s| s.as_str().to_string())
                    .unwrap_or_default(),
                attach_time: utc(ebs.attach_time()),
                encrypted: volume.and_then(|v| v.encrypted()),
                kms_key_id: text(volume.and_then(|v| v.kms_key_id())),
                delete_on_termination: ebs.delete_on_termination(),
            })
        })
        .collect()
}

async fn monitoring(client: &Ec2Client, id: &str) -> Result<Vec<Ec2Metric>> {
    let (cpu, network_in, network_out, packets_in, packets_out, no_token) = tokio::try_join!(
        client.get_cpu_metrics(id),
        client.get_network_in_metrics(id),
        client.get_network_out_metrics(id),
        client.get_network_packets_in_metrics(id),
        client.get_network_packets_out_metrics(id),
        client.get_metadata_no_token_metrics(id),
    )?;
    Ok(METRICS
        .into_iter()
        .zip([
            cpu,
            network_in,
            network_out,
            packets_in,
            packets_out,
            no_token,
        ])
        .map(|(name, points)| Ec2Metric { name, points })
        .collect())
}

fn interface(n: &aws_sdk_ec2::types::InstanceNetworkInterface) -> Ec2NetworkInterface {
    let text = |v: Option<&str>| v.unwrap_or_default().to_string();
    let attachment = n.attachment();
    Ec2NetworkInterface {
        id: text(n.network_interface_id()),
        description: text(n.description()),
        ipv4_prefixes: n
            .ipv4_prefixes()
            .iter()
            .filter_map(|p| p.ipv4_prefix())
            .map(str::to_string)
            .collect(),
        ipv6_prefixes: n
            .ipv6_prefixes()
            .iter()
            .filter_map(|p| p.ipv6_prefix())
            .map(str::to_string)
            .collect(),
        public_ipv4: text(n.association().and_then(|a| a.public_ip())),
        private_ipv4: text(n.private_ip_address()),
        private_dns: text(n.private_dns_name()),
        ipv6_addresses: n
            .ipv6_addresses()
            .iter()
            .filter_map(|a| a.ipv6_address())
            .map(str::to_string)
            .collect(),
        attach_time: utc(attachment.and_then(|a| a.attach_time())),
        attachment_status: attachment
            .and_then(|a| a.status())
            .map(|s| s.as_str().to_string())
            .unwrap_or_default(),
        vpc_id: text(n.vpc_id()),
        subnet_id: text(n.subnet_id()),
        delete_on_termination: attachment.and_then(|a| a.delete_on_termination()),
        security_groups: n
            .groups()
            .iter()
            .filter_map(|g| g.group_name())
            .map(str::to_string)
            .collect(),
        interface_type: text(n.interface_type()),
    }
}

pub(crate) fn map(i: &Instance, reservation: &Reservation) -> Ec2InstanceDetail {
    let text = |v: Option<&str>| v.unwrap_or_default().to_string();
    let tags: Vec<(String, String)> = i
        .tags()
        .iter()
        .map(|t| (text(t.key()), text(t.value())))
        .collect();
    let interfaces = i.network_interfaces();
    // Private addresses, primary first; and the public address behind each.
    let mut private_ipv4s = Vec::new();
    let mut secondary_private_ipv4s = Vec::new();
    let (mut elastic_ips, mut auto_assigned, mut carrier_ips) =
        (Vec::new(), Vec::new(), Vec::new());
    for n in interfaces {
        for address in n.private_ip_addresses() {
            let Some(ip) = address.private_ip_address() else {
                continue;
            };
            if address.primary() == Some(true) {
                private_ipv4s.insert(0, ip.to_string());
            } else {
                private_ipv4s.push(ip.to_string());
                secondary_private_ipv4s.push(ip.to_string());
            }
            if let Some(association) = address.association() {
                if let Some(public) = association.public_ip() {
                    if association.ip_owner_id() == Some("amazon") {
                        auto_assigned.push(public.to_string());
                    } else {
                        elastic_ips.push(public.to_string());
                    }
                }
                if let Some(carrier) = association.carrier_ip() {
                    carrier_ips.push(carrier.to_string());
                }
            }
        }
    }
    if private_ipv4s.is_empty() {
        private_ipv4s.extend(i.private_ip_address().map(str::to_string));
    }
    let placement = i.placement();
    let dns = i.private_dns_name_options();
    let metadata = i.metadata_options();
    let maintenance = i.maintenance_options();
    let cpu = i.cpu_options();
    Ec2InstanceDetail {
        instance_id: text(i.instance_id()),
        name: tags
            .iter()
            .find(|(k, _)| k == "Name")
            .map(|(_, v)| v.clone())
            .unwrap_or_default(),
        state: i
            .state()
            .and_then(|s| s.name())
            .map(|n| n.as_str().to_string())
            .unwrap_or_default(),
        instance_type: i
            .instance_type()
            .map(|t| t.as_str().to_string())
            .unwrap_or_default(),
        public_ipv4: text(i.public_ip_address()),
        private_ipv4s,
        secondary_private_ipv4s,
        ipv6_addresses: interfaces
            .iter()
            .flat_map(|n| n.ipv6_addresses())
            .filter_map(|a| a.ipv6_address())
            .map(str::to_string)
            .collect(),
        public_dns: text(i.public_dns_name()),
        private_dns: text(i.private_dns_name()),
        hostname_type: dns
            .and_then(|d| d.hostname_type())
            .map(|h| h.as_str().to_string())
            .unwrap_or_default(),
        resource_name_a_record: dns.and_then(|d| d.enable_resource_name_dns_a_record()),
        elastic_ips,
        auto_assigned_ip: auto_assigned.join(", "),
        carrier_ips,
        vpc_id: text(i.vpc_id()),
        subnet_id: text(i.subnet_id()),
        availability_zone: text(placement.and_then(|p| p.availability_zone())),
        outpost_arn: text(i.outpost_arn()),
        iam_instance_profile_arn: text(i.iam_instance_profile().and_then(|p| p.arn())),
        http_tokens: metadata
            .and_then(|m| m.http_tokens())
            .map(|t| t.as_str().to_string())
            .unwrap_or_default(),
        metadata_tags: metadata
            .and_then(|m| m.instance_metadata_tags())
            .map(|t| t.as_str().to_string())
            .unwrap_or_default(),
        managed: i.operator().and_then(|o| o.managed()),
        operator: text(i.operator().and_then(|o| o.principal())),
        image_id: text(i.image_id()),
        monitoring: i
            .monitoring()
            .and_then(|m| m.state())
            .map(|s| s.as_str().to_string())
            .unwrap_or_default(),
        platform_details: text(i.platform_details()),
        launch_time: utc(i.launch_time()),
        reboot_migration: maintenance
            .and_then(|m| m.reboot_migration())
            .map(|v| v.as_str().to_string())
            .unwrap_or_default(),
        auto_recovery: maintenance
            .and_then(|m| m.auto_recovery())
            .map(|v| v.as_str().to_string())
            .unwrap_or_default(),
        lifecycle: i
            .instance_lifecycle()
            .map(|l| l.as_str().to_string())
            .unwrap_or_default(),
        hibernation: i.hibernation_options().and_then(|h| h.configured()),
        ami_launch_index: i.ami_launch_index(),
        key_name: text(i.key_name()),
        state_transition_reason: text(i.state_transition_reason()),
        state_transition_message: text(i.state_reason().and_then(|r| r.message())),
        kernel_id: text(i.kernel_id()),
        ramdisk_id: text(i.ramdisk_id()),
        usage_operation: text(i.usage_operation()),
        owner_id: text(reservation.owner_id()),
        enclaves: i.enclave_options().and_then(|e| e.enabled()),
        boot_mode: i
            .boot_mode()
            .map(|b| b.as_str().to_string())
            .unwrap_or_default(),
        current_boot_mode: i
            .current_instance_boot_mode()
            .map(|b| b.as_str().to_string())
            .unwrap_or_default(),
        host_id: text(placement.and_then(|p| p.host_id())),
        affinity: text(placement.and_then(|p| p.affinity())),
        placement_group: text(placement.and_then(|p| p.group_name())),
        placement_group_id: text(placement.and_then(|p| p.group_id())),
        host_resource_group: placement
            .and_then(|p| p.host_resource_group_arn())
            .and_then(|arn| arn.rsplit('/').next())
            .unwrap_or_default()
            .to_string(),
        tenancy: placement
            .and_then(|p| p.tenancy())
            .map(|t| t.as_str().to_string())
            .unwrap_or_default(),
        virtualization_type: i
            .virtualization_type()
            .map(|v| v.as_str().to_string())
            .unwrap_or_default(),
        reservation_id: text(reservation.reservation_id()),
        partition_number: placement.and_then(|p| p.partition_number()),
        vcpus: cpu.and_then(|c| Some(c.core_count()? * c.threads_per_core()?)),
        capacity_reservation_id: text(i.capacity_reservation_id()),
        capacity_reservation_preference: i
            .capacity_reservation_specification()
            .and_then(|c| c.capacity_reservation_preference())
            .map(|p| p.as_str().to_string())
            .unwrap_or_default(),
        root_device_name: text(i.root_device_name()),
        root_device_type: i
            .root_device_type()
            .map(|t| t.as_str().to_string())
            .unwrap_or_default(),
        ebs_optimized: i.ebs_optimized(),
        security_groups: i
            .security_groups()
            .iter()
            .filter_map(|g| Some((g.group_id()?.to_string(), text(g.group_name()))))
            .collect(),
        tags,
        network_interfaces: interfaces.iter().map(interface).collect(),
        ami_name: String::new(),
        ami_location: String::new(),
        allowed_image: String::new(),
        termination_protection: String::new(),
        stop_protection: String::new(),
        credit_specification: String::new(),
        sections: Sections::default(),
    }
}

#[cfg(test)]
mod tests;
