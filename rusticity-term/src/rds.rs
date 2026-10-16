pub mod actions;
pub mod instance;

use crate::console::{component, ConsoleUrl};
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for column in instance::Column::ALL {
        i18n.entry(column.id().into())
            .or_insert_with(|| column.default_name().into());
    }
}

pub fn console_url_instances(region: &str) -> String {
    ConsoleUrl::regional(region, "/rds/home")
        .query("region", region)
        .fragment("databases:")
        .finish()
}

pub fn console_url_instance(region: &str, identifier: &str) -> String {
    ConsoleUrl::regional(region, "/rds/home")
        .query("region", region)
        .fragment(format!(
            "database:id={};is-cluster=false",
            component(identifier)
        ))
        .finish()
}

#[cfg(test)]
mod tests;
