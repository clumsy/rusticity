pub mod actions;
pub mod columns;
pub mod inventory;
pub(crate) mod list;
pub mod security_groups;
pub mod subnets;

use crate::console::{component, ConsoleUrl};
use ratatui::style::Color;
use std::collections::HashMap;

/// A VPC or subnet state as the VPC lists show it.
pub(crate) fn state_label(raw: &str) -> (String, Color) {
    match raw.to_ascii_lowercase().as_str() {
        "available" => ("✅ Available".into(), Color::Green),
        "pending" => ("⏳ Pending".into(), Color::Yellow),
        _ => (raw.to_string(), Color::White),
    }
}

/// An internet gateway block mode as the VPC lists show it.
pub(crate) fn block_public_access_label(raw: &str) -> (String, Color) {
    match raw.to_ascii_lowercase().as_str() {
        "off" => ("❌ Off".into(), Color::White),
        "block-ingress" => ("✅ Block ingress".into(), Color::Green),
        "block-bidirectional" => ("✅ Block bidirectional".into(), Color::Green),
        _ => (raw.to_string(), Color::White),
    }
}

/// The VPC console page of one list, e.g. `subnets:`.
pub(crate) fn console_url_list(region: &str, fragment: &str) -> String {
    ConsoleUrl::regional(region, "/vpcconsole/home")
        .query("region", region)
        .fragment(fragment)
        .finish()
}

pub fn init(i18n: &mut HashMap<String, String>) {
    for column in columns::Column::ALL {
        i18n.entry(column.id().into())
            .or_insert_with(|| column.default_name().into());
    }
}

pub fn console_url_vpc_details(region: &str, vpc_id: &str) -> String {
    ConsoleUrl::regional(region, "/vpcconsole/home")
        .query("region", region)
        .fragment(format!("VpcDetails:VpcId={}", component(vpc_id)))
        .finish()
}

pub fn console_url(region: &str) -> String {
    console_url_list(region, "vpcs:")
}

#[cfg(test)]
mod list_tests;
#[cfg(test)]
mod tests;
