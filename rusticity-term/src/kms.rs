pub mod actions;
pub mod key;

use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    key::init(i18n);
}

pub fn console_url_keys(region: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/kms/home?region={}#/kms/keys",
        region, region
    )
}

pub fn console_url_aws_managed_keys(region: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/kms/home?region={}#/kms/defaultKeys",
        region, region
    )
}

pub fn console_url_key(region: &str, key_id: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/kms/home?region={}#/kms/keys/{}",
        region, region, key_id
    )
}
