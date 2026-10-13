pub mod actions;
pub mod instance;

use std::collections::HashMap;
use urlencoding::encode;

pub fn init(i18n: &mut HashMap<String, String>) {
    for column in instance::Column::ALL {
        i18n.entry(column.id().into())
            .or_insert_with(|| column.default_name().into());
    }
}

pub fn console_url_instances(region: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/rds/home?region={}#databases:",
        region,
        encode(region)
    )
}

pub fn console_url_instance(region: &str, identifier: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/rds/home?region={}#database:id={};is-cluster=false",
        region,
        encode(region),
        encode(identifier)
    )
}

#[cfg(test)]
mod tests;
