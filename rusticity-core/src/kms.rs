use crate::config::AwsConfig;
use anyhow::Result;

#[derive(Clone, Debug)]
pub struct KmsKey {
    pub key_id: String,
    pub key_arn: String,
    pub description: String,
    pub key_state: String,
    pub key_usage: String,
    pub key_spec: String,
    pub key_manager: String, // "AWS" | "CUSTOMER"
    pub creation_date: String,
    pub expiration_date: String, // for EXTERNAL origin keys
    pub deletion_date: String,   // when pending deletion
    pub custom_key_store_id: String,
    pub origin: String,
    pub enabled: bool,
    pub alias: String,
    pub multi_region: bool,
}

pub struct KmsClient {
    config: AwsConfig,
}

impl KmsClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    pub async fn list_keys(&self) -> Result<Vec<KmsKey>> {
        let client = self.config.kms_client();

        // Step 1: list all key IDs with pagination
        let mut key_ids: Vec<String> = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut request = client.list_keys();
            if let Some(m) = marker {
                request = request.marker(m);
            }

            let response = request.send().await?;

            if let Some(keys) = response.keys {
                for entry in keys {
                    if let Some(id) = entry.key_id {
                        key_ids.push(id);
                    }
                }
            }

            if !response.truncated {
                break;
            }
            marker = response.next_marker;
            if marker.is_none() {
                break;
            }
        }

        // Step 2: fetch aliases once to build a key_id → alias map
        let mut alias_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let mut alias_marker: Option<String> = None;

        loop {
            let mut request = client.list_aliases();
            if let Some(m) = alias_marker {
                request = request.marker(m);
            }
            let response = request.send().await?;

            if let Some(aliases) = response.aliases {
                for a in aliases {
                    if let (Some(name), Some(target_id)) = (a.alias_name, a.target_key_id) {
                        let display = name.strip_prefix("alias/").unwrap_or(&name).to_string();
                        alias_map
                            .entry(target_id)
                            .and_modify(|v| {
                                v.push_str(", ");
                                v.push_str(&display);
                            })
                            .or_insert(display);
                    }
                }
            }

            if !response.truncated {
                break;
            }
            alias_marker = response.next_marker;
            if alias_marker.is_none() {
                break;
            }
        }

        // Step 3: describe each key in parallel (10 concurrent to avoid throttling)
        use futures::stream::{self, StreamExt};

        let mut result: Vec<KmsKey> = stream::iter(key_ids)
            .map(|key_id| {
                let client = self.config.kms_client();
                let alias = alias_map.get(&key_id).cloned().unwrap_or_default();
                async move {
                    let resp = client.describe_key().key_id(&key_id).send().await.ok()?;
                    let meta = resp.key_metadata?;

                    let key_manager = meta
                        .key_manager
                        .as_ref()
                        .map(|m| format!("{:?}", m))
                        .unwrap_or_default();
                    let key_state = meta
                        .key_state
                        .as_ref()
                        .map(|s| format!("{:?}", s))
                        .unwrap_or_default();
                    let key_usage = meta
                        .key_usage
                        .as_ref()
                        .map(|u| format!("{:?}", u))
                        .unwrap_or_default();
                    let key_spec = meta
                        .key_spec
                        .as_ref()
                        .map(|s| format!("{:?}", s))
                        .unwrap_or_default();
                    let origin = meta
                        .origin
                        .as_ref()
                        .map(|o| format!("{:?}", o))
                        .unwrap_or_default();

                    let fmt_dt = |dt: aws_smithy_types::DateTime| {
                        dt.fmt(aws_smithy_types::date_time::Format::DateTime)
                            .unwrap_or_default()
                    };

                    let creation_date = meta.creation_date.map(fmt_dt).unwrap_or_default();
                    let expiration_date = meta.valid_to.map(fmt_dt).unwrap_or_default();
                    let deletion_date = meta.deletion_date.map(fmt_dt).unwrap_or_default();
                    let custom_key_store_id = meta.custom_key_store_id.unwrap_or_default();

                    Some(KmsKey {
                        key_id: meta.key_id,
                        key_arn: meta.arn.unwrap_or_default(),
                        description: meta.description.unwrap_or_default(),
                        enabled: meta.enabled,
                        alias,
                        key_state,
                        key_usage,
                        key_spec,
                        key_manager,
                        creation_date,
                        expiration_date,
                        deletion_date,
                        custom_key_store_id,
                        origin,
                        multi_region: meta.multi_region.unwrap_or(false),
                    })
                }
            })
            .buffer_unordered(10)
            .filter_map(|k| async move { k })
            .collect()
            .await;

        result.sort_by(|a, b| a.alias.cmp(&b.alias).then_with(|| a.key_id.cmp(&b.key_id)));

        Ok(result)
    }
}
