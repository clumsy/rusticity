pub mod actions;
pub mod columns;

use std::collections::HashMap;
use urlencoding::encode;

pub fn init(i18n: &mut HashMap<String, String>) {
    for column in columns::Column::ALL {
        i18n.entry(column.id().into())
            .or_insert_with(|| column.default_name().into());
    }
}

pub fn console_url(region: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/vpcconsole/home?region={}#vpcs:",
        region,
        encode(region)
    )
}

#[cfg(test)]
mod tests;
