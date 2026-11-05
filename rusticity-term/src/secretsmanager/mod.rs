pub mod actions;
pub mod columns;
pub mod detail;
use crate::console::ConsoleUrl;
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for column in columns::Column::ALL {
        i18n.entry(column.id().into())
            .or_insert_with(|| column.default_name().into());
    }
}
/// The inventory table always opens the list page, never the highlighted row.
/// Browser-session-specific hostname prefixes are deliberately not hardcoded.
pub fn console_url(region: &str) -> String {
    ConsoleUrl::regional(region, "/secretsmanager/listsecrets")
        .query("region", region)
        .finish()
}
/// An entered secret detail opens that secret, identified by its name.
pub fn console_url_secret(region: &str, name: &str) -> String {
    ConsoleUrl::regional(region, "/secretsmanager/secret")
        .query("name", name)
        .query("region", region)
        .finish()
}
#[cfg(test)]
mod tests;
