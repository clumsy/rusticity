use crate::common::{translate_column, ColumnId, UTC_TIMESTAMP_WIDTH};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    Name,
    InstanceId,
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
}

impl Column {
    pub fn id(&self) -> &'static str {
        match self {
            Column::Name => "column.ec2.instance.name",
            Column::InstanceId => "column.ec2.instance.instance_id",
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
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            Column::Name => "Name",
            Column::InstanceId => "Instance ID",
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
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "column.ec2.instance.name" => Some(Column::Name),
            "column.ec2.instance.instance_id" => Some(Column::InstanceId),
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
            _ => None,
        }
    }

    pub fn all() -> [Column; 16] {
        [
            Column::Name,
            Column::InstanceId,
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
                    Column::Name => 30,
                    Column::InstanceId => 20,
                    Column::InstanceState => 15,
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
                }) as u16
            }

            fn render(&self, item: &Instance) -> (String, Style) {
                match self.variant {
                    Column::Name => (item.name.clone(), Style::default()),
                    Column::InstanceId => (item.instance_id.clone(), Style::default()),
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
                }
            }
        }

        Box::new(InstanceColumn { variant: *self })
    }
}

pub fn format_state(state: &str) -> (String, ratatui::style::Color) {
    match state {
        "running" => ("running".to_string(), ratatui::style::Color::Green),
        "stopped" => ("stopped".to_string(), ratatui::style::Color::Red),
        "terminated" => ("terminated".to_string(), ratatui::style::Color::DarkGray),
        "pending" => ("pending".to_string(), ratatui::style::Color::Yellow),
        "shutting-down" => ("shutting-down".to_string(), ratatui::style::Color::Yellow),
        "stopping" => ("stopping".to_string(), ratatui::style::Color::Yellow),
        _ => (state.to_string(), ratatui::style::Color::White),
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
        assert_eq!(columns.len(), 16);
        assert_eq!(columns[0], Column::Name.id());
    }

    #[test]
    fn test_format_state() {
        let (formatted, color) = format_state("running");
        assert_eq!(formatted, "running");
        assert_eq!(color, ratatui::style::Color::Green);

        let (formatted, color) = format_state("stopped");
        assert_eq!(formatted, "stopped");
        assert_eq!(color, ratatui::style::Color::Red);

        let (formatted, color) = format_state("terminated");
        assert_eq!(formatted, "terminated");
        assert_eq!(color, ratatui::style::Color::DarkGray);

        let (formatted, color) = format_state("pending");
        assert_eq!(formatted, "pending");
        assert_eq!(color, ratatui::style::Color::Yellow);

        let (formatted, color) = format_state("shutting-down");
        assert_eq!(formatted, "shutting-down");
        assert_eq!(color, ratatui::style::Color::Yellow);

        let (formatted, color) = format_state("stopping");
        assert_eq!(formatted, "stopping");
        assert_eq!(color, ratatui::style::Color::Yellow);
    }

    #[test]
    fn test_column_from_id() {
        assert_eq!(Column::from_id("column.ec2.instance.name"), Some(Column::Name));
        assert_eq!(Column::from_id("column.ec2.instance.instance_id"), Some(Column::InstanceId));
        assert_eq!(Column::from_id("column.ec2.instance.state"), Some(Column::InstanceState));
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
}
