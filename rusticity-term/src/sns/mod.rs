pub mod actions;
pub mod columns;
pub mod subscription;
pub mod subscriptions;
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

pub fn console_url(region: &str, topic_arn: Option<&str>) -> String {
    // Use the canonical regional hostname, never a browser-session-specific prefix.
    let domain = if region.starts_with("cn-") {
        "amazonaws.cn"
    } else {
        "aws.amazon.com"
    };
    let base = format!(
        "https://{region}.console.{domain}/sns/v3/home?region={}",
        urlencoding::encode(region)
    );
    match topic_arn {
        Some(arn) if !arn.is_empty() => format!("{base}#/topic/{}", urlencoding::encode(arn)),
        _ => format!("{base}#/topics"),
    }
}

#[cfg(test)]
mod tests;
