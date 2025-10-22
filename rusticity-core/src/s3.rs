use crate::config::AwsConfig;
use anyhow::Result;

pub struct S3Client {
    config: AwsConfig,
}

impl S3Client {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    pub async fn list_buckets(&self) -> Result<Vec<(String, String, String)>> {
        let client = self.config.s3_client().await;
        let resp = client.list_buckets().send().await?;

        let buckets: Vec<(String, String, String)> = resp
            .buckets()
            .iter()
            .filter_map(|b| {
                let name = b.name()?.to_string();
                let date = b.creation_date().map(|d| d.to_string()).unwrap_or_default();
                Some((name, String::new(), date))
            })
            .collect();

        Ok(buckets)
    }

    pub async fn get_bucket_location(&self, bucket: &str) -> Result<String> {
        let client = self.config.s3_client().await;
        let region = match client.get_bucket_location().bucket(bucket).send().await {
            Ok(resp) => match resp.location_constraint() {
                Some(loc) => {
                    let loc_str = loc.as_str();
                    if loc_str.is_empty() || loc_str == "null" {
                        "us-east-1".to_string()
                    } else {
                        loc_str.to_string()
                    }
                }
                None => "us-east-1".to_string(),
            },
            Err(_) => "us-east-1".to_string(),
        };

        Ok(region)
    }

    pub async fn list_objects(
        &self,
        bucket: &str,
        bucket_region: &str,
        prefix: &str,
    ) -> Result<Vec<(String, i64, String, bool, String)>> {
        let client = self.config.s3_client_with_region(bucket_region).await;
        let mut req = client.list_objects_v2().bucket(bucket).delimiter("/");

        if !prefix.is_empty() {
            req = req.prefix(prefix);
        }

        let resp = req.send().await?;

        let mut objects = Vec::new();

        for prefix in resp.common_prefixes() {
            if let Some(p) = prefix.prefix() {
                objects.push((p.to_string(), 0, String::new(), true, String::new()));
            }
        }

        for obj in resp.contents() {
            if let Some(key) = obj.key() {
                let size = obj.size().unwrap_or(0);
                let modified = obj
                    .last_modified()
                    .map(|d| d.to_string())
                    .unwrap_or_default();
                let storage_class = obj
                    .storage_class()
                    .map(|s| s.as_str().to_string())
                    .unwrap_or_else(|| "STANDARD".to_string());
                objects.push((key.to_string(), size, modified, false, storage_class));
            }
        }

        Ok(objects)
    }
}
