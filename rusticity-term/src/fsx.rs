pub mod actions;
pub mod fs;

use crate::console::{component, ConsoleUrl};
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    fs::init(i18n);
}

pub fn console_url_file_systems(region: &str) -> String {
    ConsoleUrl::regional(region, "/fsx/home")
        .query("region", region)
        .fragment("file-systems")
        .finish()
}

pub fn console_url_file_system(region: &str, fs_id: &str) -> String {
    ConsoleUrl::regional(region, "/fsx/home")
        .query("region", region)
        .fragment(format!("file-system-details/{}", component(fs_id)))
        .finish()
}

#[cfg(test)]
mod console_tests {
    use super::*;

    #[test]
    fn fsx_urls_preserve_commercial_route_and_encode_fs_id_once() {
        assert_eq!(
            console_url_file_systems("us-east-2"),
            "https://us-east-2.console.aws.amazon.com/fsx/home?region=us-east-2#file-systems"
        );
        assert_eq!(
            console_url_file_system("eu-west-1", "fs-0a1b2c3d"),
            "https://eu-west-1.console.aws.amazon.com/fsx/home?region=eu-west-1#file-system-details/fs-0a1b2c3d"
        );
        let url = console_url_file_system("us-east-2", "fs/weird");
        assert!(url.ends_with("#file-system-details/fs%2Fweird"));
        assert!(!url.contains("%252F"));
    }

    #[test]
    fn fsx_china_partition_uses_amazonaws_cn_host() {
        assert_eq!(
            console_url_file_systems("cn-northwest-1"),
            "https://cn-northwest-1.console.amazonaws.cn/fsx/home?region=cn-northwest-1#file-systems"
        );
    }
}
