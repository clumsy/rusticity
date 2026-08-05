pub mod actions;
pub mod table;

use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    table::init(i18n);
}

pub fn console_url_tables(region: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/dynamodbv2/home?region={}#tables",
        region, region
    )
}

/// Table detail URL. `tab` is a dynamodbv2 console tab id (e.g. "overview").
pub fn console_url_table(region: &str, table_name: &str, tab: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/dynamodbv2/home?region={}#table?name={}&tab={}",
        region, region, table_name, tab
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_urls() {
        assert_eq!(
            console_url_tables("us-east-2"),
            "https://us-east-2.console.aws.amazon.com/dynamodbv2/home?region=us-east-2#tables"
        );
        assert_eq!(
            console_url_table("eu-west-1", "orders", "indexes"),
            "https://eu-west-1.console.aws.amazon.com/dynamodbv2/home?region=eu-west-1#table?name=orders&tab=indexes"
        );
    }
}
