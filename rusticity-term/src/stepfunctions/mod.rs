pub mod actions;
pub mod columns;
pub mod executions;
use crate::console::{component, ConsoleUrl};
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for column in columns::Column::ALL {
        i18n.entry(column.id().into())
            .or_insert_with(|| column.default_name().into());
    }
    for column in executions::Column::ALL {
        i18n.entry(column.id().into())
            .or_insert_with(|| column.default_name().into());
    }
}

/// Regional Step Functions console link. A state machine ARN deep-links to the
/// machine detail route; otherwise the regional State Machines list. The ARN is
/// a raw value encoded exactly once. Account/session-specific host prefixes are
/// never part of the resource link.
pub fn console_url(region: &str, state_machine_arn: Option<&str>) -> String {
    let url = ConsoleUrl::regional(region, "/states/home").query("region", region);
    match state_machine_arn {
        Some(arn) if !arn.is_empty() => url
            .fragment(format!("/statemachines/view/{}", component(arn)))
            .finish(),
        _ => url.fragment("/statemachines").finish(),
    }
}

#[cfg(test)]
mod tests;
