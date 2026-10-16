pub mod access_point;
pub mod actions;
pub mod fs;
pub mod mount_target;

use crate::console::{component, ConsoleUrl};
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    fs::init(i18n);
    access_point::init(i18n);
    mount_target::init(i18n);
}

pub fn console_url_file_systems(region: &str) -> String {
    ConsoleUrl::regional(region, "/efs/home")
        .query("region", region)
        .fragment("/file-systems")
        .finish()
}

pub fn console_url_file_system(region: &str, fs_id: &str) -> String {
    ConsoleUrl::regional(region, "/efs/home")
        .query("region", region)
        .fragment(format!("/file-systems/{}", component(fs_id)))
        .finish()
}

/// Console URL for a file system's detail page, deep-linked to a specific tab
/// via the `tabId` query parameter (e.g. `tabId=fileSystemPolicy`). The `tabId`
/// lives inside the EFS hash route, not the top-level query.
pub fn console_url_file_system_with_tab(
    region: &str,
    fs_id: &str,
    tab: crate::ui::efs::DetailTab,
) -> String {
    use crate::ui::efs::DetailTab;
    let tab_id = match tab {
        DetailTab::MeteredSize => Some("meteredSize"),
        DetailTab::Monitoring => Some("monitoring"),
        DetailTab::Tags => Some("tags"),
        DetailTab::FileSystemPolicy => Some("fileSystemPolicy"),
        DetailTab::AccessPoints => Some("accessPoints"),
        DetailTab::Network => Some("network"),
        DetailTab::Replication => Some("replication"),
    };
    match tab_id {
        Some(id) => ConsoleUrl::regional(region, "/efs/home")
            .query("region", region)
            .fragment(format!("/file-systems/{}?tabId={}", component(fs_id), id))
            .finish(),
        None => console_url_file_system(region, fs_id),
    }
}

#[cfg(test)]
mod console_tests {
    use super::*;
    use crate::ui::efs::DetailTab;

    #[test]
    fn efs_urls_preserve_commercial_route_and_encode_fs_id_once() {
        assert_eq!(
            console_url_file_systems("us-east-2"),
            "https://us-east-2.console.aws.amazon.com/efs/home?region=us-east-2#/file-systems"
        );
        assert_eq!(
            console_url_file_system("eu-west-1", "fs-0a1b2c3d"),
            "https://eu-west-1.console.aws.amazon.com/efs/home?region=eu-west-1#/file-systems/fs-0a1b2c3d"
        );
        assert_eq!(
            console_url_file_system_with_tab("us-east-1", "fs-0a1b2c3d", DetailTab::Tags),
            "https://us-east-1.console.aws.amazon.com/efs/home?region=us-east-1#/file-systems/fs-0a1b2c3d?tabId=tags"
        );
        // A '/' in the id is percent-encoded exactly once inside the fragment.
        let url = console_url_file_system("us-east-2", "fs/weird");
        assert!(url.ends_with("#/file-systems/fs%2Fweird"));
        assert!(!url.contains("%252F"));
    }

    #[test]
    fn efs_china_partition_uses_amazonaws_cn_host() {
        assert_eq!(
            console_url_file_systems("cn-north-1"),
            "https://cn-north-1.console.amazonaws.cn/efs/home?region=cn-north-1#/file-systems"
        );
    }
}
