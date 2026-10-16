pub mod actions;
pub mod columns;

use crate::console::ConsoleUrl;
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for column in columns::Column::ALL {
        i18n.entry(column.id().into())
            .or_insert_with(|| column.default_name().into());
    }
}

pub fn console_url(region: &str) -> String {
    ConsoleUrl::regional(region, "/vpcconsole/home")
        .query("region", region)
        .fragment("vpcs:")
        .finish()
}

#[cfg(test)]
mod tests;
