use crate::common::{translate_column, ColumnId};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
use rusticity_core::vpc::Vpc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    VpcId,
    State,
    EncryptionControlId,
    EncryptionControlMode,
    BlockPublicAccess,
    Ipv4Cidrs,
    Ipv6Cidrs,
    DhcpOptions,
    MainRouteTable,
    MainNetworkAcl,
    Tenancy,
    DefaultVpc,
    Owner,
}

impl Column {
    /// The console's 13 selectable properties, in console order. Name, Region,
    /// and ARN remain metadata for filtering/copying, not additional properties.
    pub const ALL: [Self; 13] = [
        Self::VpcId,
        Self::State,
        Self::EncryptionControlId,
        Self::EncryptionControlMode,
        Self::BlockPublicAccess,
        Self::Ipv4Cidrs,
        Self::Ipv6Cidrs,
        Self::DhcpOptions,
        Self::MainRouteTable,
        Self::MainNetworkAcl,
        Self::Tenancy,
        Self::DefaultVpc,
        Self::Owner,
    ];

    fn definition(self) -> (ColumnId, &'static str, usize) {
        match self {
            Self::VpcId => ("column.vpc.vpc.id", "VPC ID", 23),
            Self::State => ("column.vpc.vpc.state", "State", 16),
            Self::EncryptionControlId => (
                "column.vpc.vpc.encryption_control_id",
                "Encryption control ID",
                25,
            ),
            Self::EncryptionControlMode => (
                "column.vpc.vpc.encryption_control_mode",
                "Encryption control mode",
                25,
            ),
            Self::BlockPublicAccess => (
                "column.vpc.vpc.block_public_access",
                "Block Public Access",
                23,
            ),
            Self::Ipv4Cidrs => ("column.vpc.vpc.ipv4", "IPv4 CIDR", 22),
            Self::Ipv6Cidrs => ("column.vpc.vpc.ipv6", "IPv6 CIDR", 45),
            Self::DhcpOptions => ("column.vpc.vpc.dhcp_options", "DHCP option set", 24),
            Self::MainRouteTable => ("column.vpc.vpc.main_route_table", "Main route table", 24),
            Self::MainNetworkAcl => ("column.vpc.vpc.main_network_acl", "Main network ACL", 24),
            Self::Tenancy => ("column.vpc.vpc.tenancy", "Tenancy", 12),
            Self::DefaultVpc => ("column.vpc.vpc.default", "Default VPC", 12),
            Self::Owner => ("column.vpc.vpc.owner", "Owner ID", 14),
        }
    }
    pub fn id(self) -> ColumnId {
        self.definition().0
    }
    pub fn default_name(self) -> &'static str {
        self.definition().1
    }
    pub fn name(self) -> String {
        translate_column(self.id(), self.default_name())
    }
    pub fn ids() -> Vec<ColumnId> {
        Self::ALL.into_iter().map(Self::id).collect()
    }
    pub fn default_visible_ids() -> Vec<ColumnId> {
        [
            Self::VpcId,
            Self::State,
            Self::Ipv4Cidrs,
            Self::Ipv6Cidrs,
            Self::Tenancy,
            Self::DefaultVpc,
            Self::Owner,
        ]
        .into_iter()
        .map(Self::id)
        .collect()
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.id() == id)
    }
    pub fn render(self, vpc: &Vpc) -> (String, Style) {
        let (value, color) = match self {
            Self::State => match vpc.state.to_ascii_lowercase().as_str() {
                "available" => ("✅ Available".into(), Color::Green),
                "pending" => ("⏳ Pending".into(), Color::Yellow),
                _ => (vpc.state.clone(), Color::White),
            },
            Self::DefaultVpc => match vpc.is_default {
                Some(true) => ("Yes".into(), Color::Green),
                Some(false) => ("No".into(), Color::White),
                None => (String::new(), Color::White),
            },
            Self::VpcId => (vpc.vpc_id.clone(), Color::White),
            Self::EncryptionControlId => (vpc.encryption_control_id.clone(), Color::White),
            Self::EncryptionControlMode => (vpc.encryption_control_mode.clone(), Color::White),
            Self::BlockPublicAccess => {
                match vpc.block_public_access.to_ascii_lowercase().as_str() {
                    "off" => ("❌ Off".into(), Color::White),
                    "block-ingress" => ("✅ Block ingress".into(), Color::Green),
                    "block-bidirectional" => ("✅ Block bidirectional".into(), Color::Green),
                    _ => (vpc.block_public_access.clone(), Color::White),
                }
            }
            Self::Ipv4Cidrs => (vpc.ipv4_cidrs.clone(), Color::White),
            Self::Ipv6Cidrs => (vpc.ipv6_cidrs.clone(), Color::White),
            Self::DhcpOptions => (vpc.dhcp_options_id.clone(), Color::White),
            Self::MainRouteTable => (vpc.main_route_table.clone(), Color::White),
            Self::MainNetworkAcl => (vpc.main_network_acl.clone(), Color::White),
            Self::Tenancy => (vpc.tenancy.clone(), Color::White),
            Self::Owner => (vpc.owner_id.clone(), Color::White),
        };
        if value.is_empty() {
            ("-".into(), Style::default().fg(Color::White))
        } else if value == "Unavailable" {
            (value, Style::default().fg(Color::Yellow))
        } else {
            (value, Style::default().fg(color))
        }
    }
    pub fn into_table_column(self) -> Box<dyn TableColumn<Vpc>> {
        Box::new(VpcColumn {
            column: self,
            label: self.name(),
        })
    }
}

struct VpcColumn {
    column: Column,
    label: String,
}
impl TableColumn<Vpc> for VpcColumn {
    fn name(&self) -> &str {
        &self.label
    }
    fn width(&self) -> u16 {
        self.label
            .chars()
            .count()
            .max(self.column.definition().2)
            .min(u16::MAX as usize) as u16
    }
    fn fit_content(&self) -> bool {
        true
    }

    fn render(&self, item: &Vpc) -> (String, Style) {
        self.column.render(item)
    }
}
