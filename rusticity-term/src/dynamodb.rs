pub mod actions;
pub mod items;
pub mod table;

use crate::console::{component, ConsoleUrl};
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    table::init(i18n);
}

pub fn console_url_tables(region: &str) -> String {
    ConsoleUrl::regional(region, "/dynamodbv2/home")
        .query("region", region)
        .fragment("tables")
        .finish()
}

/// Table detail URL. `tab` is a dynamodbv2 console tab id (e.g. "overview").
/// `name` and `tab` live inside dynamodbv2's hash route, so they are authored
/// into the fragment; the table name is a raw value encoded exactly once.
pub fn console_url_table(region: &str, table_name: &str, tab: &str) -> String {
    ConsoleUrl::regional(region, "/dynamodbv2/home")
        .query("region", region)
        .fragment(format!("table?name={}&tab={}", component(table_name), tab))
        .finish()
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

    #[test]
    fn china_host_and_table_name_encoded_once() {
        assert_eq!(
            console_url_tables("cn-north-1"),
            "https://cn-north-1.console.amazonaws.cn/dynamodbv2/home?region=cn-north-1#tables"
        );
        let url = console_url_table("us-east-1", "orders table", "overview");
        assert!(url.ends_with("#table?name=orders%20table&tab=overview"));
        assert!(!url.contains("%2520"));
    }
}
