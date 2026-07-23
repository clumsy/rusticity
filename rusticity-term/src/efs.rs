pub mod access_point;
pub mod actions;
pub mod fs;
pub mod mount_target;

use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    fs::init(i18n);
    access_point::init(i18n);
    mount_target::init(i18n);
}

pub fn console_url_file_systems(region: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/efs/home?region={}#/file-systems",
        region, region
    )
}

pub fn console_url_file_system(region: &str, fs_id: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/efs/home?region={}#/file-systems/{}",
        region, region, fs_id
    )
}

/// Console URL for a file system's detail page, deep-linked to a specific tab
/// via the `tabId` query parameter (e.g. `tabId=fileSystemPolicy`). The General
/// tab is the default view and carries no `tabId`.
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
        Some(id) => format!(
            "https://{}.console.aws.amazon.com/efs/home?region={}#/file-systems/{}?tabId={}",
            region, region, fs_id, id
        ),
        None => console_url_file_system(region, fs_id),
    }
}
