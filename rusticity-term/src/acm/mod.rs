//! Read-only ACM certificate inventory. The table opens no detail view, so
//! certificates are not bookmarkable and Ctrl+O always opens the list page.
pub mod actions;
pub mod columns;
pub mod inventory;

use crate::console::ConsoleUrl;
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for column in columns::Column::ALL {
        i18n.entry(column.id().into())
            .or_insert_with(|| column.default_name().into());
    }
}

/// The certificate list page, regardless of the highlighted row.
pub fn console_url(region: &str) -> String {
    ConsoleUrl::regional(region, "/acm/home")
        .query("region", region)
        .fragment("/certificates/list")
        .finish()
}

#[cfg(test)]
mod tests;
