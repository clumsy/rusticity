use crate::config::AwsConfig;
use anyhow::Result;

/// An FSx file system, flattened for display.
#[derive(Clone, Debug, Default)]
pub struct FsxFileSystem {
    /// Name tag value.
    pub name: String,
    pub file_system_id: String,
    pub file_system_arn: String,
    /// WINDOWS / LUSTRE / ONTAP / OPENZFS.
    pub file_system_type: String,
    /// Lifecycle state (AVAILABLE, CREATING, ...).
    pub status: String,
    /// Deployment type (from the type-specific configuration).
    pub deployment_type: String,
    /// Storage class / type (SSD, HDD).
    pub storage_class: String,
    /// Storage capacity, e.g. "1200 GiB".
    pub storage_capacity: String,
    /// Raw storage capacity in GiB.
    pub storage_capacity_gib: i64,
    /// Throughput capacity, e.g. "512 MB/s" (or "-" when not applicable).
    pub throughput_capacity: String,
    /// Per-unit storage throughput (Lustre), MB/s/TiB. 0 when not applicable.
    pub per_unit_throughput: i64,
    /// Total throughput in MB/s (Lustre = per-unit × capacity). 0 when unknown.
    pub total_throughput_mbps: i64,
    /// Provisioned IOPS (or "-" when not applicable).
    pub provisioned_iops: String,
    pub creation_time: String,
    /// Lustre file system version (or "-" for non-Lustre).
    pub lustre_version: String,
    /// Data compression type (Lustre): NONE / LZ4.
    pub data_compression_type: String,
    /// EFA enabled (Lustre): "Enabled" / "Disabled" / "".
    pub efa: String,
    /// Root squash (Lustre): "Enabled" / "Disabled" / "".
    pub root_squash: String,
    /// Lustre mount name.
    pub mount_name: String,
    /// Comma-separated subnet IDs (network placement).
    pub subnet_ids: String,
    pub vpc_id: String,
    pub kms_key_id: String,
    /// DNS name of the file system.
    pub dns_name: String,
    /// Comma-separated network interface IDs.
    pub network_interface_ids: String,
    /// Raw weekly maintenance start ("d:HH:MM", e.g. "4:07:00").
    pub weekly_maintenance_start: String,
    /// Raw daily automatic backup start ("HH:MM").
    pub daily_automatic_backup_start: String,
    /// Automatic backup retention in days (0 = disabled/unknown).
    pub automatic_backup_retention_days: i32,
    /// Tags as (key, value) pairs.
    pub tags: Vec<(String, String)>,
    /// Administrative actions / updates on the file system.
    pub updates: Vec<FsxUpdate>,
}

/// An FSx administrative action (Updates tab).
#[derive(Clone, Debug, Default)]
pub struct FsxUpdate {
    pub update_type: String,
    pub target_value: String,
    pub status: String,
    pub progress_percent: String,
    pub request_time: String,
    /// Raw request time (RFC3339) used for sorting.
    pub request_time_raw: String,
}

/// An FSx backup (Backups tab).
#[derive(Clone, Debug, Default)]
pub struct FsxBackup {
    pub name: String,
    pub backup_id: String,
    pub file_system_type: String,
    pub lifecycle_state: String,
    pub progress_percent: String,
    pub backup_type: String,
    pub deployment_type: String,
    pub storage_class: String,
    pub storage: String,
    pub resource_id: String,
    pub resource_name: String,
    pub backup_time: String,
    pub active_directory: String,
    pub lustre_version: String,
    pub kms_key_id: String,
}

pub struct FsxClient {
    config: AwsConfig,
}

impl FsxClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    pub async fn list_file_systems(&self) -> Result<Vec<FsxFileSystem>> {
        let client = self.config.fsx_client();

        let mut file_systems: Vec<FsxFileSystem> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.describe_file_systems();
            if let Some(t) = next_token {
                request = request.next_token(t);
            }
            let response = request.send().await?;

            for fs in response.file_systems() {
                let file_system_type = fs
                    .file_system_type()
                    .map(|t| format!("{:?}", t))
                    .unwrap_or_default();

                let status = fs
                    .lifecycle()
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_default();

                let storage_class = fs
                    .storage_type()
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_default();

                let storage_capacity_gib = fs.storage_capacity().unwrap_or(0) as i64;
                let storage_capacity = if storage_capacity_gib > 0 {
                    format!("{} GiB", storage_capacity_gib)
                } else {
                    String::new()
                };

                // Deployment type, throughput, and provisioned IOPS live in the
                // type-specific configuration block. Lustre also carries per-unit
                // throughput, compression, EFA, root squash, and mount name.
                let mut per_unit_throughput: i64 = 0;
                let mut data_compression_type = String::new();
                let mut efa = String::new();
                let mut root_squash = String::new();
                let mut mount_name = String::new();
                let (deployment_type, throughput_capacity, provisioned_iops) =
                    if let Some(w) = fs.windows_configuration() {
                        (
                            w.deployment_type().map(|d| format!("{:?}", d)),
                            w.throughput_capacity().map(|t| format!("{} MB/s", t)),
                            None,
                        )
                    } else if let Some(l) = fs.lustre_configuration() {
                        per_unit_throughput = l.per_unit_storage_throughput().unwrap_or(0) as i64;
                        data_compression_type = l
                            .data_compression_type()
                            .map(|d| format!("{:?}", d))
                            .unwrap_or_default();
                        efa = match l.efa_enabled() {
                            Some(true) => "Enabled".to_string(),
                            Some(false) => "Disabled".to_string(),
                            None => String::new(),
                        };
                        root_squash =
                            match l.root_squash_configuration().and_then(|r| r.root_squash()) {
                                Some(rs) if !rs.is_empty() => "Enabled".to_string(),
                                _ => "Disabled".to_string(),
                            };
                        mount_name = l.mount_name().unwrap_or("").to_string();
                        (
                            l.deployment_type().map(|d| format!("{:?}", d)),
                            if per_unit_throughput > 0 {
                                Some(format!("{} MB/s/TiB", per_unit_throughput))
                            } else {
                                None
                            },
                            None,
                        )
                    } else if let Some(o) = fs.ontap_configuration() {
                        (
                            o.deployment_type().map(|d| format!("{:?}", d)),
                            o.throughput_capacity().map(|t| format!("{} MB/s", t)),
                            o.disk_iops_configuration()
                                .and_then(|d| d.iops())
                                .map(|i| i.to_string()),
                        )
                    } else if let Some(z) = fs.open_zfs_configuration() {
                        (
                            z.deployment_type().map(|d| format!("{:?}", d)),
                            z.throughput_capacity().map(|t| format!("{} MB/s", t)),
                            z.disk_iops_configuration()
                                .and_then(|d| d.iops())
                                .map(|i| i.to_string()),
                        )
                    } else {
                        (None, None, None)
                    };

                // Total throughput for Lustre = per-unit × capacity (in TB, /1000).
                let total_throughput_mbps = if per_unit_throughput > 0 {
                    per_unit_throughput * storage_capacity_gib / 1000
                } else {
                    0
                };

                // Maintenance / automatic-backup settings live in the type config.
                let (weekly_maintenance_start, daily_automatic_backup_start, retention_days) =
                    if let Some(w) = fs.windows_configuration() {
                        (
                            w.weekly_maintenance_start_time(),
                            w.daily_automatic_backup_start_time(),
                            w.automatic_backup_retention_days(),
                        )
                    } else if let Some(l) = fs.lustre_configuration() {
                        (
                            l.weekly_maintenance_start_time(),
                            l.daily_automatic_backup_start_time(),
                            l.automatic_backup_retention_days(),
                        )
                    } else if let Some(o) = fs.ontap_configuration() {
                        (
                            o.weekly_maintenance_start_time(),
                            o.daily_automatic_backup_start_time(),
                            o.automatic_backup_retention_days(),
                        )
                    } else if let Some(z) = fs.open_zfs_configuration() {
                        (
                            z.weekly_maintenance_start_time(),
                            z.daily_automatic_backup_start_time(),
                            z.automatic_backup_retention_days(),
                        )
                    } else {
                        (None, None, None)
                    };
                let weekly_maintenance_start = weekly_maintenance_start.unwrap_or("").to_string();
                let daily_automatic_backup_start =
                    daily_automatic_backup_start.unwrap_or("").to_string();
                let automatic_backup_retention_days = retention_days.unwrap_or(0);

                let dns_name = fs.dns_name().unwrap_or("").to_string();
                let network_interface_ids = fs.network_interface_ids().join(", ");

                // Administrative actions (Updates tab), sorted by request time ASC.
                let mut updates: Vec<FsxUpdate> = fs
                    .administrative_actions()
                    .iter()
                    .map(|a| {
                        let request_time_raw = a
                            .request_time()
                            .and_then(|t| t.fmt(aws_smithy_types::date_time::Format::DateTime).ok())
                            .unwrap_or_default();
                        let target_value = a
                            .target_file_system_values()
                            .map(|t| {
                                if t.storage_capacity().unwrap_or(0) > 0 {
                                    format!("{} GiB", t.storage_capacity().unwrap_or(0))
                                } else if let Some(l) = t.lustre_configuration() {
                                    l.per_unit_storage_throughput()
                                        .map(|v| format!("{} MB/s/TiB", v))
                                        .unwrap_or_default()
                                } else if let Some(w) = t.windows_configuration() {
                                    w.throughput_capacity()
                                        .map(|v| format!("{} MB/s", v))
                                        .unwrap_or_default()
                                } else {
                                    String::new()
                                }
                            })
                            .unwrap_or_default();
                        FsxUpdate {
                            update_type: a
                                .administrative_action_type()
                                .map(|t| format!("{:?}", t))
                                .unwrap_or_default(),
                            target_value,
                            status: a.status().map(|s| format!("{:?}", s)).unwrap_or_default(),
                            progress_percent: a
                                .progress_percent()
                                .map(|p| format!("{}%", p))
                                .unwrap_or_default(),
                            request_time: request_time_raw.clone(),
                            request_time_raw,
                        }
                    })
                    .collect();
                updates.sort_by(|a, b| a.request_time_raw.cmp(&b.request_time_raw));

                let creation_time = fs
                    .creation_time()
                    .and_then(|t| t.fmt(aws_smithy_types::date_time::Format::DateTime).ok())
                    .unwrap_or_default();

                // Lustre version only applies to Lustre file systems.
                let lustre_version =
                    if fs.file_system_type() == Some(&aws_sdk_fsx::types::FileSystemType::Lustre) {
                        fs.file_system_type_version().unwrap_or("").to_string()
                    } else {
                        String::new()
                    };

                let name = fs
                    .tags()
                    .iter()
                    .find(|t| t.key() == Some("Name"))
                    .and_then(|t| t.value())
                    .unwrap_or("")
                    .to_string();

                let subnet_ids = fs.subnet_ids().join(", ");

                let tags: Vec<(String, String)> = fs
                    .tags()
                    .iter()
                    .filter_map(|t| Some((t.key()?.to_string(), t.value()?.to_string())))
                    .collect();

                file_systems.push(FsxFileSystem {
                    name,
                    file_system_id: fs.file_system_id().unwrap_or("").to_string(),
                    file_system_arn: fs.resource_arn().unwrap_or("").to_string(),
                    file_system_type,
                    status,
                    deployment_type: deployment_type.unwrap_or_default(),
                    storage_class,
                    storage_capacity,
                    storage_capacity_gib,
                    throughput_capacity: throughput_capacity.unwrap_or_default(),
                    per_unit_throughput,
                    total_throughput_mbps,
                    provisioned_iops: provisioned_iops.unwrap_or_default(),
                    creation_time,
                    lustre_version,
                    data_compression_type,
                    efa,
                    root_squash,
                    mount_name,
                    subnet_ids,
                    vpc_id: fs.vpc_id().unwrap_or("").to_string(),
                    kms_key_id: fs.kms_key_id().unwrap_or("").to_string(),
                    dns_name,
                    network_interface_ids,
                    weekly_maintenance_start,
                    daily_automatic_backup_start,
                    automatic_backup_retention_days,
                    tags,
                    updates,
                });
            }

            next_token = response.next_token().map(|s| s.to_string());
            if next_token.is_none() {
                break;
            }
        }

        // Sort by creation_time descending.
        file_systems.sort_by(|a, b| b.creation_time.cmp(&a.creation_time));

        Ok(file_systems)
    }

    /// List backups for a specific file system (Backups tab).
    pub async fn describe_backups(&self, file_system_id: &str) -> Result<Vec<FsxBackup>> {
        use aws_sdk_fsx::types::Filter;
        let client = self.config.fsx_client();

        let mut out: Vec<FsxBackup> = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = client.describe_backups().filters(
                Filter::builder()
                    .name(aws_sdk_fsx::types::FilterName::FileSystemId)
                    .values(file_system_id)
                    .build(),
            );
            if let Some(t) = next_token {
                req = req.next_token(t);
            }
            let resp = req.send().await?;

            for b in resp.backups() {
                let name = b
                    .tags()
                    .iter()
                    .find(|t| t.key() == Some("Name"))
                    .and_then(|t| t.value())
                    .unwrap_or("")
                    .to_string();

                let fs = b.file_system();
                let file_system_type = fs
                    .and_then(|f| f.file_system_type())
                    .map(|t| format!("{:?}", t))
                    .unwrap_or_default();
                let deployment_type = fs.map(deployment_type_of).unwrap_or_default();
                let storage_class = fs
                    .and_then(|f| f.storage_type())
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_default();
                let storage = fs
                    .and_then(|f| f.storage_capacity())
                    .map(|c| format!("{} GiB", c))
                    .unwrap_or_default();
                let resource_id = fs
                    .and_then(|f| f.file_system_id())
                    .unwrap_or("")
                    .to_string();
                let resource_name = fs
                    .map(|f| {
                        f.tags()
                            .iter()
                            .find(|t| t.key() == Some("Name"))
                            .and_then(|t| t.value())
                            .unwrap_or("")
                            .to_string()
                    })
                    .unwrap_or_default();
                let lustre_version = fs
                    .filter(|f| {
                        f.file_system_type() == Some(&aws_sdk_fsx::types::FileSystemType::Lustre)
                    })
                    .and_then(|f| f.file_system_type_version())
                    .unwrap_or("")
                    .to_string();
                let active_directory = b
                    .directory_information()
                    .and_then(|d| d.active_directory_id())
                    .unwrap_or("")
                    .to_string();

                out.push(FsxBackup {
                    name,
                    backup_id: b.backup_id().unwrap_or("").to_string(),
                    file_system_type,
                    lifecycle_state: b
                        .lifecycle()
                        .map(|s| format!("{:?}", s))
                        .unwrap_or_default(),
                    progress_percent: b
                        .progress_percent()
                        .map(|p| format!("{}%", p))
                        .unwrap_or_default(),
                    backup_type: b.r#type().map(|t| format!("{:?}", t)).unwrap_or_default(),
                    deployment_type,
                    storage_class,
                    storage,
                    resource_id,
                    resource_name,
                    backup_time: b
                        .creation_time()
                        .and_then(|t| t.fmt(aws_smithy_types::date_time::Format::DateTime).ok())
                        .unwrap_or_default(),
                    active_directory,
                    lustre_version,
                    kms_key_id: b.kms_key_id().unwrap_or("").to_string(),
                });
            }

            next_token = resp.next_token().map(|s| s.to_string());
            if next_token.is_none() {
                break;
            }
        }

        // Sort by backup time descending (newest first).
        out.sort_by(|a, b| b.backup_time.cmp(&a.backup_time));
        Ok(out)
    }
}

/// Extract the deployment type from whichever type-specific config is present.
fn deployment_type_of(fs: &aws_sdk_fsx::types::FileSystem) -> String {
    if let Some(w) = fs.windows_configuration() {
        w.deployment_type()
            .map(|d| format!("{:?}", d))
            .unwrap_or_default()
    } else if let Some(l) = fs.lustre_configuration() {
        l.deployment_type()
            .map(|d| format!("{:?}", d))
            .unwrap_or_default()
    } else if let Some(o) = fs.ontap_configuration() {
        o.deployment_type()
            .map(|d| format!("{:?}", d))
            .unwrap_or_default()
    } else if let Some(z) = fs.open_zfs_configuration() {
        z.deployment_type()
            .map(|d| format!("{:?}", d))
            .unwrap_or_default()
    } else {
        String::new()
    }
}
