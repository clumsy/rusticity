pub mod actions;
pub mod fs;

use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    fs::init(i18n);
}

pub fn console_url_file_systems(region: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/fsx/home?region={}#file-systems",
        region, region
    )
}

pub fn console_url_file_system(region: &str, fs_id: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/fsx/home?region={}#file-system-details/{}",
        region, region, fs_id
    )
}
