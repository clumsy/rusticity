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
/// This service currently shows an inventory table, so Ctrl+O always opens the
/// list page — there is no per-secret route. Browser-session-specific hostname
/// prefixes are deliberately not hardcoded.
pub fn console_url(region: &str) -> String {
    ConsoleUrl::regional(region, "/secretsmanager/listsecrets")
        .query("region", region)
        .finish()
}
#[cfg(test)]
mod tests;
