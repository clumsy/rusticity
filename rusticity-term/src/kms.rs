pub mod actions;
pub mod detail;
pub mod key;

use crate::console::{component, ConsoleUrl};
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    key::init(i18n);
}

pub fn console_url_keys(region: &str) -> String {
    ConsoleUrl::regional(region, "/kms/home")
        .query("region", region)
        .fragment("/kms/keys")
        .finish()
}

pub fn console_url_aws_managed_keys(region: &str) -> String {
    ConsoleUrl::regional(region, "/kms/home")
        .query("region", region)
        .fragment("/kms/defaultKeys")
        .finish()
}

pub fn console_url_key(region: &str, key_id: &str) -> String {
    ConsoleUrl::regional(region, "/kms/home")
        .query("region", region)
        .fragment(format!("/kms/keys/{}", component(key_id)))
        .finish()
}

#[cfg(test)]
mod console_tests {
    use super::*;

    #[test]
    fn kms_urls_preserve_commercial_routes_and_encode_key_id_once() {
        assert_eq!(
            console_url_keys("us-east-2"),
            "https://us-east-2.console.aws.amazon.com/kms/home?region=us-east-2#/kms/keys"
        );
        assert_eq!(
            console_url_aws_managed_keys("us-east-2"),
            "https://us-east-2.console.aws.amazon.com/kms/home?region=us-east-2#/kms/defaultKeys"
        );
        assert_eq!(
            console_url_key("eu-west-1", "1234abcd-12ab-34cd-56ef-1234567890ab"),
            "https://eu-west-1.console.aws.amazon.com/kms/home?region=eu-west-1#/kms/keys/1234abcd-12ab-34cd-56ef-1234567890ab"
        );
        let url = console_url_key("us-east-1", "alias/my key");
        assert!(url.ends_with("#/kms/keys/alias%2Fmy%20key"));
        assert!(!url.contains("%252F"));
    }

    #[test]
    fn kms_china_partition_uses_amazonaws_cn_host() {
        assert_eq!(
            console_url_keys("cn-north-1"),
            "https://cn-north-1.console.amazonaws.cn/kms/home?region=cn-north-1#/kms/keys"
        );
    }
}
