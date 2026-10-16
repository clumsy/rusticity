pub mod actions;
pub mod columns;
pub mod subscription;
pub mod subscriptions;
use crate::console::{component, ConsoleUrl};
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for column in subscription::Column::ALL {
        i18n.entry(column.id().into())
            .or_insert_with(|| column.default_name().into());
    }
    for column in columns::Column::ALL {
        i18n.entry(column.id().into())
            .or_insert_with(|| column.default_name().into());
    }
}

/// Regional SNS console link. A topic ARN deep-links to the topic detail route;
/// otherwise the topics list. The ARN is a raw value encoded exactly once.
/// Account/session-specific host prefixes are never part of the resource link.
pub fn console_url(region: &str, topic_arn: Option<&str>) -> String {
    let url = ConsoleUrl::regional(region, "/sns/v3/home").query("region", region);
    match topic_arn {
        Some(arn) if !arn.is_empty() => url.fragment(format!("/topic/{}", component(arn))).finish(),
        _ => url.fragment("/topics").finish(),
    }
}

#[cfg(test)]
mod tests;
