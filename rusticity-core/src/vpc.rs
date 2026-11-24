use crate::config::AwsConfig;
use anyhow::Result;
use aws_sdk_ec2::types::{
    AvailabilityZone, Filter, IpPermission, NetworkAcl, RouteTable,
    SecurityGroup as SdkSecurityGroup, Subnet as SdkSubnet, Tag, Vpc as SdkVpc,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Vpc {
    pub name: String,
    pub vpc_id: String,
    pub state: String,
    pub encryption_control_id: String,
    pub encryption_control_mode: String,
    pub block_public_access: String,
    pub main_route_table: String,
    pub main_network_acl: String,
    pub ipv4_cidrs: String,
    pub ipv6_cidrs: String,
    pub is_default: Option<bool>,
    pub tenancy: String,
    pub dhcp_options_id: String,
    pub owner_id: String,
    pub region: String,
    pub arn: String,
}

/// One subnet as the console lists it. Empty strings and `None` are fields
/// the API did not return; nothing is defaulted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Subnet {
    /// The `Name` tag; matched by the filter, not a console column.
    pub name: String,
    pub subnet_id: String,
    pub state: String,
    pub vpc_id: String,
    pub block_public_access: String,
    /// The Automated Internet Firewall treatment, as the API names it; only
    /// returned for a subnet in a VPC with that firewall.
    pub firewall_connectivity: String,
    pub outpost_id: String,
    pub ipv4_cidr: String,
    pub ipv6_cidrs: String,
    /// The association IDs of `ipv6_cidrs`, in the same order.
    pub ipv6_association_ids: String,
    pub available_ipv4: Option<i32>,
    pub availability_zone: String,
    /// From the Availability Zone of the subnet.
    pub network_border_group: String,
    /// The explicit association, else the VPC main route table.
    pub route_table: String,
    pub network_acl: String,
    pub default_for_az: Option<bool>,
    pub map_public_ip: Option<bool>,
    pub map_customer_owned_ip: Option<bool>,
    pub customer_owned_ipv4_pool: String,
    pub assign_ipv6: Option<bool>,
    pub owner_id: String,
    pub arn: String,
    pub dns64: Option<bool>,
}

/// One security group as the console lists it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SecurityGroup {
    /// The `Name` tag; the group name is `group_name`.
    pub name: String,
    pub group_id: String,
    pub group_name: String,
    pub vpc_id: String,
    pub description: String,
    pub owner_id: String,
    pub arn: String,
    /// Security group rules: one per source or destination of a permission.
    pub inbound_rules: usize,
    pub outbound_rules: usize,
}

pub struct VpcClient {
    config: AwsConfig,
}

impl VpcClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    /// The partition of the caller role, else of the Region.
    fn partition(&self) -> &str {
        self.config
            .role_arn
            .strip_prefix("arn:")
            .and_then(|arn| arn.split(':').next())
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| partition_for_region(&self.config.region))
    }

    /// Read every DescribeVpcs page; never mutate VPCs or their attributes.
    pub async fn list_vpcs(&self) -> Result<Vec<Vpc>> {
        let mut stream = self
            .config
            .ec2_client()
            .describe_vpcs()
            .into_paginator()
            .items()
            .send();
        let partition = self.partition();
        let mut vpcs = Vec::new();
        while let Some(vpc) = stream.next().await {
            vpcs.push(Vpc::from_sdk(&vpc?, &self.config.region, partition));
        }
        vpcs.sort_by(|a, b| a.vpc_id.cmp(&b.vpc_id));
        Ok(vpcs)
    }

    /// Load the properties shown by the VPC console. The two related inventories
    /// are optional: denied reads must not turn a real VPC into an empty list.
    pub async fn list_vpcs_with_properties(&self) -> Result<Vec<Vpc>> {
        let mut vpcs = self.list_vpcs().await?;
        if vpcs.is_empty() {
            return Ok(vpcs);
        }
        let client = self.config.ec2_client();
        let routes = client
            .describe_route_tables()
            .filters(
                Filter::builder()
                    .name("association.main")
                    .values("true")
                    .build(),
            )
            .into_paginator()
            .items()
            .send()
            .collect::<Result<Vec<_>, _>>();
        let acls = client
            .describe_network_acls()
            .filters(Filter::builder().name("default").values("true").build())
            .into_paginator()
            .items()
            .send()
            .collect::<Result<Vec<_>, _>>();
        let (routes, acls) = tokio::join!(routes, acls);
        match routes {
            Ok(routes) => apply_main_route_tables(&mut vpcs, &routes),
            Err(error) => {
                tracing::warn!("VPC main route tables unavailable: {error}");
                for vpc in &mut vpcs {
                    vpc.main_route_table = "Unavailable".into();
                }
            }
        }
        match acls {
            Ok(acls) => apply_main_network_acls(&mut vpcs, &acls),
            Err(error) => {
                tracing::warn!("VPC main network ACLs unavailable: {error}");
                for vpc in &mut vpcs {
                    vpc.main_network_acl = "Unavailable".into();
                }
            }
        }
        Ok(vpcs)
    }
    /// Read-only describe of one VPC with its console properties. The VPC
    /// must exist in the account; a missing id is an explicit error.
    pub async fn describe_vpc(&self, vpc_id: &str) -> Result<Vpc> {
        self.list_vpcs_with_properties()
            .await?
            .into_iter()
            .find(|vpc| vpc.vpc_id == vpc_id)
            .ok_or_else(|| anyhow::anyhow!("VPC {vpc_id} was not found in this Region"))
    }

    /// Tags of one VPC, read-only, sorted by key.
    pub async fn list_vpc_tags(&self, vpc_id: &str) -> Result<Vec<(String, String)>> {
        let mut stream = self
            .config
            .ec2_client()
            .describe_tags()
            .filters(Filter::builder().name("resource-id").values(vpc_id).build())
            .into_paginator()
            .items()
            .send();
        let mut tags = Vec::new();
        while let Some(tag) = stream.next().await {
            tags.push(tag_from_sdk(&tag?));
        }
        tags.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(tags)
    }

    /// Every subnet in the Region with its route table, network ACL and
    /// network border group, read-only, sorted by subnet ID. A denied related
    /// read marks its column Unavailable instead of failing the list.
    pub async fn list_subnets(&self) -> Result<Vec<Subnet>> {
        let client = self.config.ec2_client();
        let subnets = client
            .describe_subnets()
            .into_paginator()
            .items()
            .send()
            .collect::<Result<Vec<_>, _>>();
        let routes = client
            .describe_route_tables()
            .into_paginator()
            .items()
            .send()
            .collect::<Result<Vec<_>, _>>();
        let acls = client
            .describe_network_acls()
            .into_paginator()
            .items()
            .send()
            .collect::<Result<Vec<_>, _>>();
        let zones = client.describe_availability_zones().send();
        let (subnets, routes, acls, zones) = tokio::join!(subnets, routes, acls, zones);
        let partition = self.partition();
        let mut subnets: Vec<Subnet> = subnets?
            .iter()
            .map(|s| Subnet::from_sdk(s, &self.config.region, partition))
            .collect();
        match routes {
            Ok(routes) => apply_subnet_route_tables(&mut subnets, &routes),
            Err(error) => {
                tracing::warn!("Subnet route tables unavailable: {error}");
                for subnet in &mut subnets {
                    subnet.route_table = "Unavailable".into();
                }
            }
        }
        match acls {
            Ok(acls) => apply_subnet_network_acls(&mut subnets, &acls),
            Err(error) => {
                tracing::warn!("Subnet network ACLs unavailable: {error}");
                for subnet in &mut subnets {
                    subnet.network_acl = "Unavailable".into();
                }
            }
        }
        match zones {
            Ok(zones) => apply_network_border_groups(&mut subnets, zones.availability_zones()),
            Err(error) => {
                tracing::warn!("Subnet network border groups unavailable: {error}");
                for subnet in &mut subnets {
                    subnet.network_border_group = "Unavailable".into();
                }
            }
        }
        subnets.sort_by(|a, b| a.subnet_id.cmp(&b.subnet_id));
        Ok(subnets)
    }

    /// Every security group in the Region, read-only, sorted by group ID.
    pub async fn list_security_groups(&self) -> Result<Vec<SecurityGroup>> {
        let mut stream = self
            .config
            .ec2_client()
            .describe_security_groups()
            .into_paginator()
            .items()
            .send();
        let partition = self.partition();
        let mut groups = Vec::new();
        while let Some(group) = stream.next().await {
            groups.push(SecurityGroup::from_sdk(
                &group?,
                &self.config.region,
                partition,
            ));
        }
        groups.sort_by(|a, b| a.group_id.cmp(&b.group_id));
        Ok(groups)
    }
}

fn name_tag(tags: &[Tag]) -> String {
    tags.iter()
        .find(|t| t.key() == Some("Name"))
        .and_then(|t| t.value())
        .unwrap_or_default()
        .into()
}

/// `arn:{partition}:ec2:{region}:{owner}:{kind}/{id}`, or empty when a part
/// is unknown.
fn resource_arn(partition: &str, region: &str, owner: &str, kind: &str, id: &str) -> String {
    if id.is_empty() || owner.is_empty() || region.is_empty() {
        String::new()
    } else {
        format!("arn:{partition}:ec2:{region}:{owner}:{kind}/{id}")
    }
}

/// The explicit route table of each subnet, else the main one of its VPC.
fn apply_subnet_route_tables(subnets: &mut [Subnet], routes: &[RouteTable]) {
    for subnet in subnets {
        let explicit = routes.iter().find(|route| {
            route
                .associations()
                .iter()
                .any(|a| a.subnet_id() == Some(subnet.subnet_id.as_str()))
        });
        let main = || {
            routes.iter().find(|route| {
                route.vpc_id() == Some(subnet.vpc_id.as_str())
                    && route.associations().iter().any(|a| a.main() == Some(true))
            })
        };
        subnet.route_table = explicit
            .or_else(main)
            .and_then(|route| route.route_table_id())
            .unwrap_or_default()
            .into();
    }
}

fn apply_subnet_network_acls(subnets: &mut [Subnet], acls: &[NetworkAcl]) {
    for subnet in subnets {
        subnet.network_acl = acls
            .iter()
            .find(|acl| {
                acl.associations()
                    .iter()
                    .any(|a| a.subnet_id() == Some(subnet.subnet_id.as_str()))
            })
            .and_then(|acl| acl.network_acl_id())
            .unwrap_or_default()
            .into();
    }
}

/// The network border group of each subnet, by its Availability Zone name.
fn apply_network_border_groups(subnets: &mut [Subnet], zones: &[AvailabilityZone]) {
    for subnet in subnets {
        subnet.network_border_group = zones
            .iter()
            .find(|z| z.zone_name() == Some(subnet.availability_zone.as_str()))
            .and_then(|z| z.network_border_group())
            .unwrap_or_default()
            .into();
    }
}

/// One rule per IPv4 range, IPv6 range, prefix list and group of a permission.
fn rule_count(permissions: &[IpPermission]) -> usize {
    permissions
        .iter()
        .map(|p| {
            (p.ip_ranges().len()
                + p.ipv6_ranges().len()
                + p.prefix_list_ids().len()
                + p.user_id_group_pairs().len())
            .max(1)
        })
        .sum()
}

impl Subnet {
    fn from_sdk(subnet: &SdkSubnet, region: &str, partition: &str) -> Self {
        let subnet_id = subnet.subnet_id().unwrap_or_default().to_string();
        let owner_id = subnet.owner_id().unwrap_or_default().to_string();
        let (ipv6, ipv6_ids): (Vec<String>, Vec<String>) = subnet
            .ipv6_cidr_block_association_set()
            .iter()
            .filter_map(|a| {
                let cidr = cidr_label(
                    a.ipv6_cidr_block(),
                    a.ipv6_cidr_block_state()
                        .and_then(|s| s.state())
                        .map(|s| s.as_str()),
                )?;
                Some((cidr, a.association_id().unwrap_or_default().to_string()))
            })
            .unzip();
        let arn = match subnet.subnet_arn() {
            Some(arn) if !arn.is_empty() => arn.to_string(),
            _ => resource_arn(partition, region, &owner_id, "subnet", &subnet_id),
        };
        Self {
            name: name_tag(subnet.tags()),
            state: subnet
                .state()
                .map(|s| s.as_str().into())
                .unwrap_or_default(),
            vpc_id: subnet.vpc_id().unwrap_or_default().into(),
            block_public_access: subnet
                .block_public_access_states()
                .and_then(|b| b.internet_gateway_block_mode())
                .map(|m| m.as_str())
                .unwrap_or_default()
                .into(),
            firewall_connectivity: subnet
                .network_firewall_connectivity_type()
                .map(|t| t.as_str())
                .unwrap_or_default()
                .into(),
            outpost_id: subnet
                .outpost_arn()
                .and_then(|arn| arn.rsplit('/').next())
                .unwrap_or_default()
                .into(),
            ipv4_cidr: subnet.cidr_block().unwrap_or_default().into(),
            ipv6_cidrs: ipv6.join(", "),
            ipv6_association_ids: ipv6_ids
                .into_iter()
                .filter(|id| !id.is_empty())
                .collect::<Vec<_>>()
                .join(", "),
            available_ipv4: subnet.available_ip_address_count(),
            availability_zone: subnet.availability_zone().unwrap_or_default().into(),
            default_for_az: subnet.default_for_az(),
            map_public_ip: subnet.map_public_ip_on_launch(),
            map_customer_owned_ip: subnet.map_customer_owned_ip_on_launch(),
            customer_owned_ipv4_pool: subnet.customer_owned_ipv4_pool().unwrap_or_default().into(),
            assign_ipv6: subnet.assign_ipv6_address_on_creation(),
            dns64: subnet.enable_dns64(),
            subnet_id,
            owner_id,
            arn,
            ..Default::default()
        }
    }
}

impl SecurityGroup {
    fn from_sdk(group: &SdkSecurityGroup, region: &str, partition: &str) -> Self {
        let group_id = group.group_id().unwrap_or_default().to_string();
        let owner_id = group.owner_id().unwrap_or_default().to_string();
        let arn = match group.security_group_arn() {
            Some(arn) if !arn.is_empty() => arn.to_string(),
            _ => resource_arn(partition, region, &owner_id, "security-group", &group_id),
        };
        Self {
            name: name_tag(group.tags()),
            group_name: group.group_name().unwrap_or_default().into(),
            vpc_id: group.vpc_id().unwrap_or_default().into(),
            description: group.description().unwrap_or_default().into(),
            inbound_rules: rule_count(group.ip_permissions()),
            outbound_rules: rule_count(group.ip_permissions_egress()),
            group_id,
            owner_id,
            arn,
        }
    }
}

fn tag_from_sdk(tag: &aws_sdk_ec2::types::TagDescription) -> (String, String) {
    (
        tag.key().unwrap_or_default().to_string(),
        tag.value().unwrap_or_default().to_string(),
    )
}

fn apply_main_route_tables(vpcs: &mut [Vpc], routes: &[RouteTable]) {
    for vpc in vpcs {
        vpc.main_route_table = routes
            .iter()
            .find(|route| {
                route.vpc_id() == Some(vpc.vpc_id.as_str())
                    && route.associations().iter().any(|a| a.main() == Some(true))
            })
            .and_then(|route| route.route_table_id())
            .unwrap_or_default()
            .into();
    }
}

fn apply_main_network_acls(vpcs: &mut [Vpc], acls: &[NetworkAcl]) {
    for vpc in vpcs {
        vpc.main_network_acl = acls
            .iter()
            .find(|acl| acl.vpc_id() == Some(vpc.vpc_id.as_str()) && acl.is_default() == Some(true))
            .and_then(|acl| acl.network_acl_id())
            .unwrap_or_default()
            .into();
    }
}

fn partition_for_region(region: &str) -> &'static str {
    if region.starts_with("cn-") {
        "aws-cn"
    } else if region.starts_with("us-gov-") {
        "aws-us-gov"
    } else if region.starts_with("us-isob-") {
        "aws-iso-b"
    } else if region.starts_with("us-iso-") {
        "aws-iso"
    } else if region.starts_with("eu-isoe-") {
        "aws-iso-e"
    } else if region.starts_with("us-isof-") {
        "aws-iso-f"
    } else {
        "aws"
    }
}

fn cidr_label(cidr: Option<&str>, state: Option<&str>) -> Option<String> {
    let cidr = cidr.filter(|cidr| !cidr.is_empty())?;
    match state {
        Some("disassociated") => None,
        None | Some("associated") => Some(cidr.into()),
        Some(state) => Some(format!("{cidr} ({state})")),
    }
}

impl Vpc {
    fn from_sdk(vpc: &SdkVpc, region: &str, partition: &str) -> Self {
        let vpc_id = vpc.vpc_id().unwrap_or_default().to_string();
        let owner_id = vpc.owner_id().unwrap_or_default().to_string();
        let mut ipv4 = Vec::new();
        if let Some(cidr) = cidr_label(vpc.cidr_block(), None) {
            ipv4.push(cidr);
        }
        for association in vpc.cidr_block_association_set() {
            if let Some(cidr) = cidr_label(
                association.cidr_block(),
                association
                    .cidr_block_state()
                    .and_then(|s| s.state())
                    .map(|s| s.as_str()),
            ) {
                if !ipv4.contains(&cidr) {
                    ipv4.push(cidr);
                }
            }
        }
        let mut ipv6 = Vec::new();
        for association in vpc.ipv6_cidr_block_association_set() {
            if let Some(cidr) = cidr_label(
                association.ipv6_cidr_block(),
                association
                    .ipv6_cidr_block_state()
                    .and_then(|s| s.state())
                    .map(|s| s.as_str()),
            ) {
                if !ipv6.contains(&cidr) {
                    ipv6.push(cidr);
                }
            }
        }
        let arn = resource_arn(partition, region, &owner_id, "vpc", &vpc_id);
        Self {
            name: name_tag(vpc.tags()),
            vpc_id,
            owner_id,
            arn,
            region: region.into(),
            state: vpc.state().map(|s| s.as_str().into()).unwrap_or_default(),
            encryption_control_id: vpc
                .encryption_control()
                .and_then(|c| c.vpc_encryption_control_id())
                .unwrap_or_default()
                .into(),
            encryption_control_mode: vpc
                .encryption_control()
                .and_then(|c| c.mode())
                .map(|m| m.as_str())
                .unwrap_or_default()
                .into(),
            block_public_access: vpc
                .block_public_access_states()
                .and_then(|b| b.internet_gateway_block_mode())
                .map(|m| m.as_str())
                .unwrap_or_default()
                .into(),
            ipv4_cidrs: ipv4.join(", "),
            ipv6_cidrs: ipv6.join(", "),
            is_default: vpc.is_default(),
            tenancy: vpc
                .instance_tenancy()
                .map(|t| t.as_str().into())
                .unwrap_or_default(),
            dhcp_options_id: vpc.dhcp_options_id().unwrap_or_default().into(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_ec2::types::{
        Tag, Tenancy, VpcCidrBlockAssociation, VpcCidrBlockState, VpcCidrBlockStateCode,
        VpcIpv6CidrBlockAssociation, VpcState,
    };

    #[test]
    fn vpc_metadata_keeps_primary_secondary_and_active_ipv6_cidrs() {
        let sdk = SdkVpc::builder()
            .vpc_id("vpc-test")
            .owner_id("123456789012")
            .tags(Tag::builder().key("Name").value("test-network").build())
            .state(VpcState::Available)
            .cidr_block("10.0.0.0/16")
            .is_default(false)
            .instance_tenancy(Tenancy::Default)
            .dhcp_options_id("dopt-test")
            .cidr_block_association_set(
                VpcCidrBlockAssociation::builder()
                    .cidr_block("10.0.0.0/16")
                    .cidr_block_state(
                        VpcCidrBlockState::builder()
                            .state(VpcCidrBlockStateCode::Associated)
                            .build(),
                    )
                    .build(),
            )
            .cidr_block_association_set(
                VpcCidrBlockAssociation::builder()
                    .cidr_block("10.1.0.0/16")
                    .build(),
            )
            .ipv6_cidr_block_association_set(
                VpcIpv6CidrBlockAssociation::builder()
                    .ipv6_cidr_block("2001:db8::/56")
                    .ipv6_cidr_block_state(
                        VpcCidrBlockState::builder()
                            .state(VpcCidrBlockStateCode::Associated)
                            .build(),
                    )
                    .build(),
            )
            .ipv6_cidr_block_association_set(
                VpcIpv6CidrBlockAssociation::builder()
                    .ipv6_cidr_block("2001:db8:100::/56")
                    .ipv6_cidr_block_state(
                        VpcCidrBlockState::builder()
                            .state(VpcCidrBlockStateCode::Disassociated)
                            .build(),
                    )
                    .build(),
            )
            .build();
        let vpc = Vpc::from_sdk(&sdk, "us-east-2", "aws");
        assert_eq!(vpc.name, "test-network");
        assert_eq!(vpc.state, "available");
        assert_eq!(vpc.ipv4_cidrs, "10.0.0.0/16, 10.1.0.0/16");
        assert_eq!(vpc.ipv6_cidrs, "2001:db8::/56");
        assert_eq!(vpc.is_default, Some(false));
        assert_eq!(vpc.tenancy, "default");
        assert_eq!(vpc.dhcp_options_id, "dopt-test");
        assert_eq!(vpc.arn, "arn:aws:ec2:us-east-2:123456789012:vpc/vpc-test");
    }

    #[test]
    fn vpc_tag_mapping_keeps_key_and_value_verbatim() {
        let tag = aws_sdk_ec2::types::TagDescription::builder()
            .key("Team")
            .value("platform")
            .build();
        assert_eq!(tag_from_sdk(&tag), ("Team".into(), "platform".into()));
    }

    #[test]
    fn vpc_missing_fields_and_pending_associations_are_not_fabricated() {
        let vpc = Vpc::from_sdk(&SdkVpc::builder().build(), "us-east-2", "aws");
        assert!(vpc.name.is_empty() && vpc.arn.is_empty() && vpc.ipv4_cidrs.is_empty());
        assert_eq!(vpc.is_default, None);
        assert_eq!(
            cidr_label(Some("10.2.0.0/16"), Some("associating")),
            Some("10.2.0.0/16 (associating)".into())
        );
    }

    #[test]
    fn vpc_arn_uses_owner_and_partition() {
        let sdk = SdkVpc::builder()
            .vpc_id("vpc-test")
            .owner_id("123456789012")
            .build();
        let vpc = Vpc::from_sdk(&sdk, "cn-north-1", "aws-cn");
        assert_eq!(
            vpc.arn,
            "arn:aws-cn:ec2:cn-north-1:123456789012:vpc/vpc-test"
        );
        assert_eq!(partition_for_region("us-gov-west-1"), "aws-us-gov");
        assert_eq!(partition_for_region("us-isob-east-1"), "aws-iso-b");
    }
}

#[cfg(test)]
mod subnet_tests {
    use super::*;
    use aws_sdk_ec2::types::{
        BlockPublicAccessMode, BlockPublicAccessStates, IpRange, Ipv6Range, NetworkAclAssociation,
        NetworkFirewallConnectivityType, PrefixListId, RouteTableAssociation, SubnetCidrBlockState,
        SubnetCidrBlockStateCode, SubnetIpv6CidrBlockAssociation, SubnetState, UserIdGroupPair,
    };

    fn ipv6(cidr: &str, state: SubnetCidrBlockStateCode) -> SubnetIpv6CidrBlockAssociation {
        SubnetIpv6CidrBlockAssociation::builder()
            .association_id(format!("subnet-cidr-assoc-{}", &cidr[..14]))
            .ipv6_cidr_block(cidr)
            .ipv6_cidr_block_state(SubnetCidrBlockState::builder().state(state).build())
            .build()
    }

    #[test]
    fn subnet_maps_every_console_field_verbatim() {
        let sdk = SdkSubnet::builder()
            .subnet_id("subnet-a")
            .owner_id("123456789012")
            .tags(Tag::builder().key("Name").value("public-a").build())
            .state(SubnetState::Available)
            .vpc_id("vpc-a")
            .block_public_access_states(
                BlockPublicAccessStates::builder()
                    .internet_gateway_block_mode(BlockPublicAccessMode::Off)
                    .build(),
            )
            .network_firewall_connectivity_type(NetworkFirewallConnectivityType::OutboundOnly)
            .cidr_block("10.0.1.0/24")
            .ipv6_cidr_block_association_set(ipv6(
                "2001:db8:0:1::/64",
                SubnetCidrBlockStateCode::Associated,
            ))
            .ipv6_cidr_block_association_set(ipv6(
                "2001:db8:0:9::/64",
                SubnetCidrBlockStateCode::Disassociated,
            ))
            .available_ip_address_count(250)
            .availability_zone("us-east-2a")
            .availability_zone_id("use2-az1")
            .default_for_az(false)
            .map_public_ip_on_launch(true)
            .assign_ipv6_address_on_creation(false)
            .map_customer_owned_ip_on_launch(false)
            .outpost_arn("arn:aws:outposts:us-east-2:123456789012:outpost/op-0abc")
            .enable_dns64(false)
            .build();
        let subnet = Subnet::from_sdk(&sdk, "us-east-2", "aws");
        assert_eq!(subnet.name, "public-a");
        assert_eq!(subnet.state, "available");
        assert_eq!(subnet.block_public_access, "off");
        assert_eq!(subnet.ipv4_cidr, "10.0.1.0/24");
        assert_eq!(subnet.firewall_connectivity, "outbound-only");
        assert_eq!(subnet.ipv6_cidrs, "2001:db8:0:1::/64");
        assert_eq!(
            subnet.ipv6_association_ids, "subnet-cidr-assoc-2001:db8:0:1::",
            "only the associations shown in IPv6 CIDR"
        );
        assert_eq!(subnet.available_ipv4, Some(250));
        assert_eq!(subnet.availability_zone, "us-east-2a");
        assert_eq!(subnet.map_public_ip, Some(true));
        assert_eq!(subnet.outpost_id, "op-0abc");
        assert_eq!(subnet.dns64, Some(false));
        assert_eq!(
            subnet.arn,
            "arn:aws:ec2:us-east-2:123456789012:subnet/subnet-a"
        );
    }

    #[test]
    fn subnet_missing_fields_are_not_fabricated() {
        let subnet = Subnet::from_sdk(&SdkSubnet::builder().build(), "us-east-2", "aws");
        assert_eq!(subnet, Subnet::default());
        let sdk = SdkSubnet::builder()
            .subnet_id("subnet-a")
            .subnet_arn("arn:aws:ec2:us-east-2:111111111111:subnet/subnet-a")
            .build();
        let subnet = Subnet::from_sdk(&sdk, "us-east-2", "aws");
        assert_eq!(
            subnet.arn, "arn:aws:ec2:us-east-2:111111111111:subnet/subnet-a",
            "the API ARN wins"
        );
    }

    #[test]
    fn subnet_route_table_is_explicit_else_the_vpc_main_one() {
        let subnet = |id: &str, vpc: &str| Subnet {
            subnet_id: id.into(),
            vpc_id: vpc.into(),
            ..Default::default()
        };
        let mut subnets = vec![
            subnet("subnet-a", "vpc-a"),
            subnet("subnet-b", "vpc-a"),
            subnet("subnet-c", "vpc-missing"),
        ];
        let routes = [
            RouteTable::builder()
                .route_table_id("rtb-main-other")
                .vpc_id("vpc-other")
                .associations(RouteTableAssociation::builder().main(true).build())
                .build(),
            RouteTable::builder()
                .route_table_id("rtb-main")
                .vpc_id("vpc-a")
                .associations(RouteTableAssociation::builder().main(true).build())
                .build(),
            RouteTable::builder()
                .route_table_id("rtb-public")
                .vpc_id("vpc-a")
                .associations(
                    RouteTableAssociation::builder()
                        .subnet_id("subnet-a")
                        .build(),
                )
                .build(),
        ];
        let acls = [
            NetworkAcl::builder()
                .network_acl_id("acl-a")
                .associations(
                    NetworkAclAssociation::builder()
                        .subnet_id("subnet-a")
                        .build(),
                )
                .associations(
                    NetworkAclAssociation::builder()
                        .subnet_id("subnet-b")
                        .build(),
                )
                .build(),
            NetworkAcl::builder()
                .network_acl_id("acl-c")
                .associations(
                    NetworkAclAssociation::builder()
                        .subnet_id("subnet-c")
                        .build(),
                )
                .build(),
        ];
        apply_subnet_route_tables(&mut subnets, &routes);
        apply_subnet_network_acls(&mut subnets, &acls);
        subnets[0].availability_zone = "us-west-2-lax-1a".into();
        subnets[1].availability_zone = "us-west-2a".into();
        let zones = [
            AvailabilityZone::builder()
                .zone_name("us-west-2a")
                .network_border_group("us-west-2")
                .build(),
            AvailabilityZone::builder()
                .zone_name("us-west-2-lax-1a")
                .network_border_group("us-west-2-lax-1")
                .build(),
        ];
        apply_network_border_groups(&mut subnets, &zones);
        assert_eq!(subnets[0].network_border_group, "us-west-2-lax-1");
        assert_eq!(subnets[1].network_border_group, "us-west-2");
        assert!(subnets[2].network_border_group.is_empty());
        assert_eq!(subnets[0].route_table, "rtb-public");
        assert_eq!(subnets[1].route_table, "rtb-main");
        assert!(subnets[2].route_table.is_empty());
        assert_eq!(subnets[0].network_acl, "acl-a");
        assert_eq!(subnets[1].network_acl, "acl-a");
        assert_eq!(subnets[2].network_acl, "acl-c");
    }

    #[test]
    fn security_group_counts_one_rule_per_source() {
        let inbound = IpPermission::builder()
            .ip_protocol("tcp")
            .from_port(22)
            .to_port(22)
            .ip_ranges(IpRange::builder().cidr_ip("10.0.0.0/8").build())
            .ip_ranges(IpRange::builder().cidr_ip("192.168.0.0/16").build())
            .ipv6_ranges(Ipv6Range::builder().cidr_ipv6("::/0").build())
            .build();
        let from_group = IpPermission::builder()
            .ip_protocol("-1")
            .user_id_group_pairs(UserIdGroupPair::builder().group_id("sg-a").build())
            .prefix_list_ids(PrefixListId::builder().prefix_list_id("pl-a").build())
            .build();
        let sdk = SdkSecurityGroup::builder()
            .group_id("sg-a")
            .group_name("default")
            .description("default VPC security group")
            .vpc_id("vpc-a")
            .owner_id("123456789012")
            .tags(Tag::builder().key("Name").value("web").build())
            .ip_permissions(inbound)
            .ip_permissions(from_group)
            .ip_permissions_egress(
                IpPermission::builder()
                    .ip_protocol("-1")
                    .ip_ranges(IpRange::builder().cidr_ip("0.0.0.0/0").build())
                    .build(),
            )
            .build();
        let group = SecurityGroup::from_sdk(&sdk, "us-east-2", "aws");
        assert_eq!(group.name, "web");
        assert_eq!(group.group_name, "default");
        assert_eq!(group.inbound_rules, 5);
        assert_eq!(group.outbound_rules, 1);
        assert_eq!(
            group.arn,
            "arn:aws:ec2:us-east-2:123456789012:security-group/sg-a"
        );
        let empty = SecurityGroup::from_sdk(&SdkSecurityGroup::builder().build(), "r", "aws");
        assert_eq!(empty, SecurityGroup::default());
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use aws_sdk_ec2::types::{
        BlockPublicAccessMode, BlockPublicAccessStates, RouteTableAssociation,
        VpcEncryptionControl, VpcEncryptionControlMode,
    };

    #[test]
    fn vpc_maps_encryption_control_and_block_public_access_without_defaults() {
        let sdk = SdkVpc::builder()
            .encryption_control(
                VpcEncryptionControl::builder()
                    .vpc_encryption_control_id("vpc-enc-test")
                    .mode(VpcEncryptionControlMode::Monitor)
                    .build(),
            )
            .block_public_access_states(
                BlockPublicAccessStates::builder()
                    .internet_gateway_block_mode(BlockPublicAccessMode::BlockBidirectional)
                    .build(),
            )
            .build();
        let vpc = Vpc::from_sdk(&sdk, "us-east-2", "aws");
        assert_eq!(vpc.encryption_control_id, "vpc-enc-test");
        assert_eq!(vpc.encryption_control_mode, "monitor");
        assert_eq!(vpc.block_public_access, "block-bidirectional");
        let missing = Vpc::from_sdk(&SdkVpc::builder().build(), "us-east-2", "aws");
        assert!(missing.encryption_control_id.is_empty());
        assert!(missing.encryption_control_mode.is_empty());
        assert!(missing.block_public_access.is_empty());
    }

    #[test]
    fn vpc_related_properties_join_by_vpc_and_require_main_or_default() {
        let mut vpcs = vec![
            Vpc {
                vpc_id: "vpc-a".into(),
                ..Default::default()
            },
            Vpc {
                vpc_id: "vpc-b".into(),
                ..Default::default()
            },
            Vpc {
                vpc_id: "vpc-missing".into(),
                ..Default::default()
            },
        ];
        let routes = [
            RouteTable::builder()
                .route_table_id("rtb-custom")
                .vpc_id("vpc-a")
                .associations(RouteTableAssociation::builder().main(false).build())
                .build(),
            RouteTable::builder()
                .route_table_id("rtb-b")
                .vpc_id("vpc-b")
                .associations(RouteTableAssociation::builder().main(true).build())
                .build(),
            RouteTable::builder()
                .route_table_id("rtb-a")
                .vpc_id("vpc-a")
                .associations(RouteTableAssociation::builder().main(true).build())
                .build(),
        ];
        let acls = [
            NetworkAcl::builder()
                .network_acl_id("acl-custom")
                .vpc_id("vpc-a")
                .is_default(false)
                .build(),
            NetworkAcl::builder()
                .network_acl_id("acl-b")
                .vpc_id("vpc-b")
                .is_default(true)
                .build(),
            NetworkAcl::builder()
                .network_acl_id("acl-a")
                .vpc_id("vpc-a")
                .is_default(true)
                .build(),
        ];
        apply_main_route_tables(&mut vpcs, &routes);
        apply_main_network_acls(&mut vpcs, &acls);
        assert_eq!(vpcs[0].main_route_table, "rtb-a");
        assert_eq!(vpcs[0].main_network_acl, "acl-a");
        assert_eq!(vpcs[1].main_route_table, "rtb-b");
        assert_eq!(vpcs[1].main_network_acl, "acl-b");
        assert!(vpcs[2].main_route_table.is_empty());
        assert!(vpcs[2].main_network_acl.is_empty());
    }
}
