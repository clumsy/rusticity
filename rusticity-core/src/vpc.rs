use crate::config::AwsConfig;
use anyhow::Result;
use aws_sdk_ec2::types::{Filter, NetworkAcl, RouteTable, Vpc as SdkVpc};

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

pub struct VpcClient {
    config: AwsConfig,
}

impl VpcClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
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
        let partition = self
            .config
            .role_arn
            .strip_prefix("arn:")
            .and_then(|arn| arn.split(':').next())
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| partition_for_region(&self.config.region));
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
        let arn = if vpc_id.is_empty() || owner_id.is_empty() || region.is_empty() {
            String::new()
        } else {
            format!("arn:{partition}:ec2:{region}:{owner_id}:vpc/{vpc_id}")
        };
        Self {
            name: vpc
                .tags()
                .iter()
                .find(|t| t.key() == Some("Name"))
                .and_then(|t| t.value())
                .unwrap_or_default()
                .into(),
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
