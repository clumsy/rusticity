use crate::common::{translate_column, ColumnId, UTC_TIMESTAMP_WIDTH};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
use ratatui::style::Color;
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for col in Column::all() {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
}

#[derive(Debug, Clone)]
pub struct Instance {
    pub instance_id: String,
    pub name: String,
    pub state: String,
    pub instance_type: String,
    pub availability_zone: String,
    pub public_ipv4_dns: String,
    pub public_ipv4_address: String,
    pub elastic_ip: String,
    pub ipv6_ips: String,
    pub monitoring: String,
    pub security_groups: String,
    pub key_name: String,
    pub launch_time: String,
    pub platform_details: String,
    pub status_checks: String,
    pub alarm_status: String,
    pub private_dns_name: String,
    pub private_ip_address: String,
    pub security_group_ids: String,
    pub owner_id: String,
    pub volume_id: String,
    pub root_device_name: String,
    pub root_device_type: String,
    pub ebs_optimized: String,
    pub image_id: String,
    pub kernel_id: String,
    pub ramdisk_id: String,
    pub ami_launch_index: String,
    pub reservation_id: String,
    pub vpc_id: String,
    pub subnet_ids: String,
    pub instance_lifecycle: String,
    pub architecture: String,
    pub virtualization_type: String,
    pub platform: String,
    pub iam_instance_profile_arn: String,
    pub tenancy: String,
    pub affinity: String,
    pub host_id: String,
    pub placement_group: String,
    pub partition_number: String,
    pub capacity_reservation_id: String,
    pub state_transition_reason_code: String,
    pub state_transition_reason_message: String,
    pub stop_hibernation_behavior: String,
    pub outpost_arn: String,
    pub product_codes: String,
    pub availability_zone_id: String,
    pub imdsv2: String,
    pub usage_operation: String,
    pub managed: String,
    pub operator: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    InstanceId,
    Name,
    InstanceState,
    InstanceType,
    StatusCheck,
    AlarmStatus,
    AvailabilityZone,
    PublicIpv4Dns,
    PublicIpv4Address,
    ElasticIp,
    Ipv6Ips,
    Monitoring,
    SecurityGroupName,
    KeyName,
    LaunchTime,
    PlatformDetails,
    PrivateDnsName,
    PrivateIpAddress,
    SecurityGroupIds,
    OwnerId,
    VolumeId,
    RootDeviceName,
    RootDeviceType,
    EbsOptimized,
    ImageId,
    KernelId,
    RamdiskId,
    AmiLaunchIndex,
    ReservationId,
    VpcId,
    SubnetIds,
    InstanceLifecycle,
    Architecture,
    VirtualizationType,
    Platform,
    IamInstanceProfileArn,
    Tenancy,
    Affinity,
    HostId,
    PlacementGroup,
    PartitionNumber,
    CapacityReservationId,
    StateTransitionReasonCode,
    StateTransitionReasonMessage,
    StopHibernationBehavior,
    OutpostArn,
    ProductCodes,
    AvailabilityZoneId,
    Imdsv2,
    UsageOperation,
    Managed,
    Operator,
}

impl Column {
    pub fn id(&self) -> &'static str {
        match self {
            Column::InstanceId => "column.ec2.instance.instance_id",
            Column::Name => "column.ec2.instance.name",
            Column::InstanceState => "column.ec2.instance.state",
            Column::InstanceType => "column.ec2.instance.instance_type",
            Column::StatusCheck => "column.ec2.instance.status_check",
            Column::AlarmStatus => "column.ec2.instance.alarm_status",
            Column::AvailabilityZone => "column.ec2.instance.availability_zone",
            Column::PublicIpv4Dns => "column.ec2.instance.public_ipv4_dns",
            Column::PublicIpv4Address => "column.ec2.instance.public_ipv4_address",
            Column::ElasticIp => "column.ec2.instance.elastic_ip",
            Column::Ipv6Ips => "column.ec2.instance.ipv6_ips",
            Column::Monitoring => "column.ec2.instance.monitoring",
            Column::SecurityGroupName => "column.ec2.instance.security_group_name",
            Column::KeyName => "column.ec2.instance.key_name",
            Column::LaunchTime => "column.ec2.instance.launch_time",
            Column::PlatformDetails => "column.ec2.instance.platform_details",
            Column::PrivateDnsName => "column.ec2.instance.private_dns_name",
            Column::PrivateIpAddress => "column.ec2.instance.private_ip_address",
            Column::SecurityGroupIds => "column.ec2.instance.security_group_ids",
            Column::OwnerId => "column.ec2.instance.owner_id",
            Column::VolumeId => "column.ec2.instance.volume_id",
            Column::RootDeviceName => "column.ec2.instance.root_device_name",
            Column::RootDeviceType => "column.ec2.instance.root_device_type",
            Column::EbsOptimized => "column.ec2.instance.ebs_optimized",
            Column::ImageId => "column.ec2.instance.image_id",
            Column::KernelId => "column.ec2.instance.kernel_id",
            Column::RamdiskId => "column.ec2.instance.ramdisk_id",
            Column::AmiLaunchIndex => "column.ec2.instance.ami_launch_index",
            Column::ReservationId => "column.ec2.instance.reservation_id",
            Column::VpcId => "column.ec2.instance.vpc_id",
            Column::SubnetIds => "column.ec2.instance.subnet_ids",
            Column::InstanceLifecycle => "column.ec2.instance.instance_lifecycle",
            Column::Architecture => "column.ec2.instance.architecture",
            Column::VirtualizationType => "column.ec2.instance.virtualization_type",
            Column::Platform => "column.ec2.instance.platform",
            Column::IamInstanceProfileArn => "column.ec2.instance.iam_instance_profile_arn",
            Column::Tenancy => "column.ec2.instance.tenancy",
            Column::Affinity => "column.ec2.instance.affinity",
            Column::HostId => "column.ec2.instance.host_id",
            Column::PlacementGroup => "column.ec2.instance.placement_group",
            Column::PartitionNumber => "column.ec2.instance.partition_number",
            Column::CapacityReservationId => "column.ec2.instance.capacity_reservation_id",
            Column::StateTransitionReasonCode => "column.ec2.instance.state_transition_reason_code",
            Column::StateTransitionReasonMessage => {
                "column.ec2.instance.state_transition_reason_message"
            }
            Column::StopHibernationBehavior => "column.ec2.instance.stop_hibernation_behavior",
            Column::OutpostArn => "column.ec2.instance.outpost_arn",
            Column::ProductCodes => "column.ec2.instance.product_codes",
            Column::AvailabilityZoneId => "column.ec2.instance.availability_zone_id",
            Column::Imdsv2 => "column.ec2.instance.imdsv2",
            Column::UsageOperation => "column.ec2.instance.usage_operation",
            Column::Managed => "column.ec2.instance.managed",
            Column::Operator => "column.ec2.instance.operator",
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            Column::InstanceId => "Instance ID",
            Column::Name => "Name",
            Column::InstanceState => "Instance state",
            Column::InstanceType => "Instance type",
            Column::StatusCheck => "Status check",
            Column::AlarmStatus => "Alarm status",
            Column::AvailabilityZone => "Availability Zone",
            Column::PublicIpv4Dns => "Public IPv4 DNS",
            Column::PublicIpv4Address => "Public IPv4 address",
            Column::ElasticIp => "Elastic IP",
            Column::Ipv6Ips => "IPv6 IPs",
            Column::Monitoring => "Monitoring",
            Column::SecurityGroupName => "Security group name",
            Column::KeyName => "Key name",
            Column::LaunchTime => "Launch time",
            Column::PlatformDetails => "Platform details",
            Column::PrivateDnsName => "Private DNS name",
            Column::PrivateIpAddress => "Private IP address",
            Column::SecurityGroupIds => "Security group IDs",
            Column::OwnerId => "Owner ID",
            Column::VolumeId => "Volume ID",
            Column::RootDeviceName => "Root device name",
            Column::RootDeviceType => "Root device type",
            Column::EbsOptimized => "EBS optimized",
            Column::ImageId => "Image ID",
            Column::KernelId => "Kernel ID",
            Column::RamdiskId => "RAM disk ID",
            Column::AmiLaunchIndex => "AMI launch index",
            Column::ReservationId => "Reservation ID",
            Column::VpcId => "VPC ID",
            Column::SubnetIds => "Subnet IDs",
            Column::InstanceLifecycle => "Instance lifecycle",
            Column::Architecture => "Architecture",
            Column::VirtualizationType => "Virtualization type",
            Column::Platform => "Platform",
            Column::IamInstanceProfileArn => "IAM instance profile ARN",
            Column::Tenancy => "Tenancy",
            Column::Affinity => "Affinity",
            Column::HostId => "Host ID",
            Column::PlacementGroup => "Placement group",
            Column::PartitionNumber => "Partition number",
            Column::CapacityReservationId => "Capacity Reservation ID",
            Column::StateTransitionReasonCode => "State transition reason code",
            Column::StateTransitionReasonMessage => "State transition reason message",
            Column::StopHibernationBehavior => "Stop-hibernation behavior",
            Column::OutpostArn => "Outpost ARN",
            Column::ProductCodes => "Product codes",
            Column::AvailabilityZoneId => "Availability Zone ID",
            Column::Imdsv2 => "IMDSv2",
            Column::UsageOperation => "Usage operation",
            Column::Managed => "Managed",
            Column::Operator => "Operator",
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "column.ec2.instance.instance_id" => Some(Column::InstanceId),
            "column.ec2.instance.name" => Some(Column::Name),
            "column.ec2.instance.state" => Some(Column::InstanceState),
            "column.ec2.instance.instance_type" => Some(Column::InstanceType),
            "column.ec2.instance.status_check" => Some(Column::StatusCheck),
            "column.ec2.instance.alarm_status" => Some(Column::AlarmStatus),
            "column.ec2.instance.availability_zone" => Some(Column::AvailabilityZone),
            "column.ec2.instance.public_ipv4_dns" => Some(Column::PublicIpv4Dns),
            "column.ec2.instance.public_ipv4_address" => Some(Column::PublicIpv4Address),
            "column.ec2.instance.elastic_ip" => Some(Column::ElasticIp),
            "column.ec2.instance.ipv6_ips" => Some(Column::Ipv6Ips),
            "column.ec2.instance.monitoring" => Some(Column::Monitoring),
            "column.ec2.instance.security_group_name" => Some(Column::SecurityGroupName),
            "column.ec2.instance.key_name" => Some(Column::KeyName),
            "column.ec2.instance.launch_time" => Some(Column::LaunchTime),
            "column.ec2.instance.platform_details" => Some(Column::PlatformDetails),
            "column.ec2.instance.private_dns_name" => Some(Column::PrivateDnsName),
            "column.ec2.instance.private_ip_address" => Some(Column::PrivateIpAddress),
            "column.ec2.instance.security_group_ids" => Some(Column::SecurityGroupIds),
            "column.ec2.instance.owner_id" => Some(Column::OwnerId),
            "column.ec2.instance.volume_id" => Some(Column::VolumeId),
            "column.ec2.instance.root_device_name" => Some(Column::RootDeviceName),
            "column.ec2.instance.root_device_type" => Some(Column::RootDeviceType),
            "column.ec2.instance.ebs_optimized" => Some(Column::EbsOptimized),
            "column.ec2.instance.image_id" => Some(Column::ImageId),
            "column.ec2.instance.kernel_id" => Some(Column::KernelId),
            "column.ec2.instance.ramdisk_id" => Some(Column::RamdiskId),
            "column.ec2.instance.ami_launch_index" => Some(Column::AmiLaunchIndex),
            "column.ec2.instance.reservation_id" => Some(Column::ReservationId),
            "column.ec2.instance.vpc_id" => Some(Column::VpcId),
            "column.ec2.instance.subnet_ids" => Some(Column::SubnetIds),
            "column.ec2.instance.instance_lifecycle" => Some(Column::InstanceLifecycle),
            "column.ec2.instance.architecture" => Some(Column::Architecture),
            "column.ec2.instance.virtualization_type" => Some(Column::VirtualizationType),
            "column.ec2.instance.platform" => Some(Column::Platform),
            "column.ec2.instance.iam_instance_profile_arn" => Some(Column::IamInstanceProfileArn),
            "column.ec2.instance.tenancy" => Some(Column::Tenancy),
            "column.ec2.instance.affinity" => Some(Column::Affinity),
            "column.ec2.instance.host_id" => Some(Column::HostId),
            "column.ec2.instance.placement_group" => Some(Column::PlacementGroup),
            "column.ec2.instance.partition_number" => Some(Column::PartitionNumber),
            "column.ec2.instance.capacity_reservation_id" => Some(Column::CapacityReservationId),
            "column.ec2.instance.state_transition_reason_code" => {
                Some(Column::StateTransitionReasonCode)
            }
            "column.ec2.instance.state_transition_reason_message" => {
                Some(Column::StateTransitionReasonMessage)
            }
            "column.ec2.instance.stop_hibernation_behavior" => {
                Some(Column::StopHibernationBehavior)
            }
            "column.ec2.instance.outpost_arn" => Some(Column::OutpostArn),
            "column.ec2.instance.product_codes" => Some(Column::ProductCodes),
            "column.ec2.instance.availability_zone_id" => Some(Column::AvailabilityZoneId),
            "column.ec2.instance.imdsv2" => Some(Column::Imdsv2),
            "column.ec2.instance.usage_operation" => Some(Column::UsageOperation),
            "column.ec2.instance.managed" => Some(Column::Managed),
            "column.ec2.instance.operator" => Some(Column::Operator),
            _ => None,
        }
    }

    pub fn all() -> [Column; 52] {
        [
            Column::InstanceId,
            Column::Name,
            Column::InstanceState,
            Column::InstanceType,
            Column::StatusCheck,
            Column::AlarmStatus,
            Column::AvailabilityZone,
            Column::PublicIpv4Dns,
            Column::PublicIpv4Address,
            Column::ElasticIp,
            Column::Ipv6Ips,
            Column::Monitoring,
            Column::SecurityGroupName,
            Column::KeyName,
            Column::LaunchTime,
            Column::PlatformDetails,
            Column::PrivateDnsName,
            Column::PrivateIpAddress,
            Column::SecurityGroupIds,
            Column::OwnerId,
            Column::VolumeId,
            Column::RootDeviceName,
            Column::RootDeviceType,
            Column::EbsOptimized,
            Column::ImageId,
            Column::KernelId,
            Column::RamdiskId,
            Column::AmiLaunchIndex,
            Column::ReservationId,
            Column::VpcId,
            Column::SubnetIds,
            Column::InstanceLifecycle,
            Column::Architecture,
            Column::VirtualizationType,
            Column::Platform,
            Column::IamInstanceProfileArn,
            Column::Tenancy,
            Column::Affinity,
            Column::HostId,
            Column::PlacementGroup,
            Column::PartitionNumber,
            Column::CapacityReservationId,
            Column::StateTransitionReasonCode,
            Column::StateTransitionReasonMessage,
            Column::StopHibernationBehavior,
            Column::OutpostArn,
            Column::ProductCodes,
            Column::AvailabilityZoneId,
            Column::Imdsv2,
            Column::UsageOperation,
            Column::Managed,
            Column::Operator,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }

    pub fn to_column(&self) -> Box<dyn TableColumn<Instance>> {
        struct InstanceColumn {
            variant: Column,
        }

        impl TableColumn<Instance> for InstanceColumn {
            fn name(&self) -> &str {
                Box::leak(self.variant.name().into_boxed_str())
            }

            fn width(&self) -> u16 {
                let translated = translate_column(self.variant.id(), self.variant.default_name());
                translated.len().max(match self.variant {
                    Column::InstanceId => 20,
                    Column::Name => 30,
                    Column::InstanceState => 18,
                    Column::InstanceType => 15,
                    Column::StatusCheck => 15,
                    Column::AlarmStatus => 15,
                    Column::AvailabilityZone => 20,
                    Column::PublicIpv4Dns => 40,
                    Column::PublicIpv4Address => 20,
                    Column::ElasticIp => 20,
                    Column::Ipv6Ips => 30,
                    Column::Monitoring => 15,
                    Column::SecurityGroupName => 30,
                    Column::KeyName => 20,
                    Column::LaunchTime => UTC_TIMESTAMP_WIDTH as usize,
                    Column::PlatformDetails => 30,
                    Column::PrivateDnsName => 40,
                    Column::PrivateIpAddress => 20,
                    Column::SecurityGroupIds => 30,
                    Column::OwnerId => 15,
                    Column::VolumeId => 25,
                    Column::RootDeviceName => 20,
                    Column::RootDeviceType => 18,
                    Column::EbsOptimized => 15,
                    Column::ImageId => 25,
                    Column::KernelId => 25,
                    Column::RamdiskId => 25,
                    Column::AmiLaunchIndex => 18,
                    Column::ReservationId => 20,
                    Column::VpcId => 25,
                    Column::SubnetIds => 30,
                    Column::InstanceLifecycle => 20,
                    Column::Architecture => 15,
                    Column::VirtualizationType => 20,
                    Column::Platform => 15,
                    Column::IamInstanceProfileArn => 50,
                    Column::Tenancy => 15,
                    Column::Affinity => 15,
                    Column::HostId => 25,
                    Column::PlacementGroup => 25,
                    Column::PartitionNumber => 18,
                    Column::CapacityReservationId => 30,
                    Column::StateTransitionReasonCode => 30,
                    Column::StateTransitionReasonMessage => 50,
                    Column::StopHibernationBehavior => 28,
                    Column::OutpostArn => 40,
                    Column::ProductCodes => 30,
                    Column::AvailabilityZoneId => 20,
                    Column::Imdsv2 => 15,
                    Column::UsageOperation => 25,
                    Column::Managed => 15,
                    Column::Operator => 15,
                }) as u16
            }

            fn render(&self, item: &Instance) -> (String, Style) {
                match self.variant {
                    Column::InstanceId => (item.instance_id.clone(), Style::default()),
                    Column::Name => (item.name.clone(), Style::default()),
                    Column::InstanceState => {
                        let (formatted, color) = format_state(&item.state);
                        (formatted, Style::default().fg(color))
                    }
                    Column::InstanceType => (item.instance_type.clone(), Style::default()),
                    Column::StatusCheck => (item.status_checks.clone(), Style::default()),
                    Column::AlarmStatus => (item.alarm_status.clone(), Style::default()),
                    Column::AvailabilityZone => (item.availability_zone.clone(), Style::default()),
                    Column::PublicIpv4Dns => (item.public_ipv4_dns.clone(), Style::default()),
                    Column::PublicIpv4Address => {
                        (item.public_ipv4_address.clone(), Style::default())
                    }
                    Column::ElasticIp => (item.elastic_ip.clone(), Style::default()),
                    Column::Ipv6Ips => (item.ipv6_ips.clone(), Style::default()),
                    Column::Monitoring => (item.monitoring.clone(), Style::default()),
                    Column::SecurityGroupName => (item.security_groups.clone(), Style::default()),
                    Column::KeyName => (item.key_name.clone(), Style::default()),
                    Column::LaunchTime => (item.launch_time.clone(), Style::default()),
                    Column::PlatformDetails => (item.platform_details.clone(), Style::default()),
                    Column::PrivateDnsName => (item.private_dns_name.clone(), Style::default()),
                    Column::PrivateIpAddress => (item.private_ip_address.clone(), Style::default()),
                    Column::SecurityGroupIds => (item.security_group_ids.clone(), Style::default()),
                    Column::OwnerId => (item.owner_id.clone(), Style::default()),
                    Column::VolumeId => (item.volume_id.clone(), Style::default()),
                    Column::RootDeviceName => (item.root_device_name.clone(), Style::default()),
                    Column::RootDeviceType => (item.root_device_type.clone(), Style::default()),
                    Column::EbsOptimized => (item.ebs_optimized.clone(), Style::default()),
                    Column::ImageId => (item.image_id.clone(), Style::default()),
                    Column::KernelId => (item.kernel_id.clone(), Style::default()),
                    Column::RamdiskId => (item.ramdisk_id.clone(), Style::default()),
                    Column::AmiLaunchIndex => (item.ami_launch_index.clone(), Style::default()),
                    Column::ReservationId => (item.reservation_id.clone(), Style::default()),
                    Column::VpcId => (item.vpc_id.clone(), Style::default()),
                    Column::SubnetIds => (item.subnet_ids.clone(), Style::default()),
                    Column::InstanceLifecycle => {
                        (item.instance_lifecycle.clone(), Style::default())
                    }
                    Column::Architecture => (item.architecture.clone(), Style::default()),
                    Column::VirtualizationType => {
                        (item.virtualization_type.clone(), Style::default())
                    }
                    Column::Platform => (item.platform.clone(), Style::default()),
                    Column::IamInstanceProfileArn => {
                        (item.iam_instance_profile_arn.clone(), Style::default())
                    }
                    Column::Tenancy => (item.tenancy.clone(), Style::default()),
                    Column::Affinity => (item.affinity.clone(), Style::default()),
                    Column::HostId => (item.host_id.clone(), Style::default()),
                    Column::PlacementGroup => (item.placement_group.clone(), Style::default()),
                    Column::PartitionNumber => (item.partition_number.clone(), Style::default()),
                    Column::CapacityReservationId => {
                        (item.capacity_reservation_id.clone(), Style::default())
                    }
                    Column::StateTransitionReasonCode => {
                        (item.state_transition_reason_code.clone(), Style::default())
                    }
                    Column::StateTransitionReasonMessage => (
                        item.state_transition_reason_message.clone(),
                        Style::default(),
                    ),
                    Column::StopHibernationBehavior => {
                        (item.stop_hibernation_behavior.clone(), Style::default())
                    }
                    Column::OutpostArn => (item.outpost_arn.clone(), Style::default()),
                    Column::ProductCodes => (item.product_codes.clone(), Style::default()),
                    Column::AvailabilityZoneId => {
                        (item.availability_zone_id.clone(), Style::default())
                    }
                    Column::Imdsv2 => (item.imdsv2.clone(), Style::default()),
                    Column::UsageOperation => (item.usage_operation.clone(), Style::default()),
                    Column::Managed => (item.managed.clone(), Style::default()),
                    Column::Operator => (item.operator.clone(), Style::default()),
                }
            }
        }

        Box::new(InstanceColumn { variant: *self })
    }
}

pub fn format_state(state: &str) -> (String, Color) {
    match state {
        "running" => ("✅ Running".to_string(), Color::Green),
        "stopped" => ("🛑 Stopped".to_string(), Color::Red),
        "terminated" => ("❌ Terminated".to_string(), Color::DarkGray),
        "pending" => ("❎ Pending".to_string(), Color::Yellow),
        "shutting-down" => ("🔴 Shutting-down".to_string(), Color::Yellow),
        "stopping" => ("🚫 Stopping".to_string(), Color::Yellow),
        _ => (state.to_string(), Color::White),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_names() {
        assert_eq!(Column::Name.name(), "Name");
        assert_eq!(Column::InstanceId.name(), "Instance ID");
        assert_eq!(Column::InstanceState.name(), "Instance state");
        assert_eq!(Column::InstanceType.name(), "Instance type");
    }

    #[test]
    fn test_column_all() {
        let columns = Column::ids();
        assert_eq!(columns.len(), 52);
        assert_eq!(columns[0], Column::InstanceId.id());
    }

    #[test]
    fn test_format_state() {
        let (formatted, color) = format_state("running");
        assert_eq!(formatted, "✅ Running");
        assert_eq!(color, Color::Green);

        let (formatted, color) = format_state("stopped");
        assert_eq!(formatted, "🛑 Stopped");
        assert_eq!(color, Color::Red);

        let (formatted, color) = format_state("terminated");
        assert_eq!(formatted, "❌ Terminated");
        assert_eq!(color, Color::DarkGray);

        let (formatted, color) = format_state("pending");
        assert_eq!(formatted, "❎ Pending");
        assert_eq!(color, Color::Yellow);

        let (formatted, color) = format_state("shutting-down");
        assert_eq!(formatted, "🔴 Shutting-down");
        assert_eq!(color, Color::Yellow);

        let (formatted, color) = format_state("stopping");
        assert_eq!(formatted, "🚫 Stopping");
        assert_eq!(color, Color::Yellow);
    }

    #[test]
    fn test_column_from_id() {
        assert_eq!(
            Column::from_id("column.ec2.instance.name"),
            Some(Column::Name)
        );
        assert_eq!(
            Column::from_id("column.ec2.instance.instance_id"),
            Some(Column::InstanceId)
        );
        assert_eq!(
            Column::from_id("column.ec2.instance.state"),
            Some(Column::InstanceState)
        );
        assert_eq!(Column::from_id("invalid"), None);
    }

    #[test]
    fn test_column_ids_unique() {
        let ids = Column::ids();
        let mut seen = std::collections::HashSet::new();
        for id in ids {
            assert!(seen.insert(id), "Duplicate column ID: {}", id);
        }
    }

    #[test]
    fn test_column_ids_have_correct_prefix() {
        for col in Column::all() {
            assert!(
                col.id().starts_with("column.ec2.instance."),
                "Column ID '{}' should start with 'column.ec2.instance.'",
                col.id()
            );
        }
    }

    #[test]
    fn test_launch_time_uses_utc_timestamp_width() {
        let col = Column::LaunchTime.to_column();
        assert_eq!(col.width(), UTC_TIMESTAMP_WIDTH);
    }

    #[test]
    fn test_utc_timestamp_width_constant_is_27() {
        assert_eq!(UTC_TIMESTAMP_WIDTH, 27);
    }
}
