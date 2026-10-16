pub mod actions;
pub mod columns;
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for column in columns::Column::ALL {
        i18n.entry(column.id().into())
            .or_insert_with(|| column.default_name().into());
    }
}
/// This service currently shows an inventory table, so Ctrl+O opens the list.
/// Browser-session-specific hostname prefixes are deliberately not hardcoded.
pub fn console_url(region: &str) -> String {
    let domain = if region.starts_with("cn-") {
        "amazonaws.cn"
    } else {
        "aws.amazon.com"
    };
    format!(
        "https://{region}.console.{domain}/secretsmanager/listsecrets?region={}",
        urlencoding::encode(region)
    )
}
#[cfg(test)]
mod tests;
