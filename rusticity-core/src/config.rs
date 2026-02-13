use anyhow::Result;

#[derive(Clone, Debug)]
pub struct AwsConfig {
    pub region: String,
    pub account_id: String,
    pub role_arn: String,
    pub region_auto_detected: bool,
}

impl AwsConfig {
    pub async fn new(region: Option<String>) -> Result<Self> {
        Self::new_with_timeout(region, std::time::Duration::from_secs(10)).await
    }

    pub async fn new_with_timeout(
        region: Option<String>,
        timeout: std::time::Duration,
    ) -> Result<Self> {
        // Check for region early to avoid IMDS timeout
        if region.is_none()
            && std::env::var("AWS_REGION").is_err()
            && std::env::var("AWS_DEFAULT_REGION").is_err()
        {
            return Err(anyhow::anyhow!("Missing Region"));
        }

        let mut config_loader = aws_config::defaults(aws_config::BehaviorVersion::latest());

        // Use profile from AWS_PROFILE env var if set
        let profile_to_use = std::env::var("AWS_PROFILE").ok();
        if let Some(ref profile) = profile_to_use {
            if !profile.is_empty() {
                tracing::info!("Using AWS profile: {}", profile);
                config_loader = config_loader.profile_name(profile);
            }
        }

        if let Some(r) = &region {
            config_loader = config_loader.region(aws_config::Region::new(r.clone()));
        }

        // Load config with timeout
        let config = tokio::time::timeout(timeout, config_loader.load())
            .await
            .map_err(|_| anyhow::anyhow!("Timeout loading AWS config"))?;

        // Double-check region is set
        if config.region().is_none() {
            return Err(anyhow::anyhow!("Missing Region"));
        }

        // Try to get identity with timeout
        let (account_id, role_arn) =
            match tokio::time::timeout(timeout, Self::try_get_identity(&config)).await {
                Ok(Ok((acc, role))) => {
                    tracing::info!("Loaded identity: account={}, role={}", acc, role);
                    (acc, role)
                }
                Ok(Err(e)) => {
                    tracing::error!("Failed to get identity: {}", e);
                    return Err(e);
                }
                Err(_) => return Err(anyhow::anyhow!("Timeout getting AWS identity")),
            };

        let (region_str, auto_detected) = match config.region() {
            Some(r) => (r.as_ref().to_string(), false),
            None => {
                let fastest = Self::find_fastest_region().await?;
                (fastest, true)
            }
        };

        Ok(Self {
            region: region_str,
            account_id,
            role_arn,
            region_auto_detected: auto_detected,
        })
    }

    async fn try_get_identity(config: &aws_config::SdkConfig) -> Result<(String, String)> {
        let sts_client = aws_sdk_sts::Client::new(config);
        let identity = sts_client.get_caller_identity().send().await?;
        let account_id = identity.account().unwrap_or("").to_string();
        let role_arn = identity.arn().unwrap_or("").to_string();
        Ok((account_id, role_arn))
    }

    async fn find_fastest_region() -> Result<String> {
        use std::time::Instant;

        let regions = [
            "us-east-1",
            "us-east-2",
            "us-west-1",
            "us-west-2",
            "af-south-1",
            "ap-east-1",
            "ap-south-1",
            "ap-south-2",
            "ap-northeast-1",
            "ap-northeast-2",
            "ap-northeast-3",
            "ap-southeast-1",
            "ap-southeast-2",
            "ap-southeast-3",
            "ap-southeast-4",
            "ca-central-1",
            "ca-west-1",
            "eu-central-1",
            "eu-central-2",
            "eu-west-1",
            "eu-west-2",
            "eu-west-3",
            "eu-north-1",
            "eu-south-1",
            "eu-south-2",
            "il-central-1",
            "me-central-1",
            "me-south-1",
            "sa-east-1",
        ];

        let mut tasks = Vec::new();

        for &region in &regions {
            let region_name = region.to_string();
            tasks.push(tokio::spawn(async move {
                let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                    .region(aws_config::Region::new(region_name.clone()))
                    .load()
                    .await;
                let s3 = aws_sdk_s3::Client::new(&config);
                let start = Instant::now();
                match tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    s3.list_buckets().send(),
                )
                .await
                {
                    Ok(Ok(_)) => Some((region_name, start.elapsed())),
                    _ => Some((region_name, std::time::Duration::from_secs(9999))),
                }
            }));
        }

        let results = futures::future::join_all(tasks).await;
        let mut latencies: Vec<(String, std::time::Duration)> = results
            .into_iter()
            .filter_map(|r| r.ok().flatten())
            .collect();

        latencies.sort_by_key(|(_, d)| *d);

        latencies
            .first()
            .map(|(r, _)| r.clone())
            .ok_or_else(|| anyhow::anyhow!("Could not determine fastest region"))
    }

    pub fn dummy(region: Option<String>) -> Self {
        Self {
            region: region.unwrap_or_default(),
            account_id: "".to_string(),
            role_arn: "".to_string(),
            region_auto_detected: false,
        }
    }

    pub async fn get_account_for_profile(profile: &str, region: &str) -> Result<String> {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .profile_name(profile)
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        let sts_client = aws_sdk_sts::Client::new(&config);
        let identity = sts_client.get_caller_identity().send().await?;
        Ok(identity.account().unwrap_or("").to_string())
    }

    pub async fn s3_client(&self) -> aws_sdk_s3::Client {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(self.region.clone()))
            .load()
            .await;
        aws_sdk_s3::Client::new(&config)
    }

    pub async fn s3_client_with_region(&self, region: &str) -> aws_sdk_s3::Client {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;
        aws_sdk_s3::Client::new(&config)
    }

    pub async fn cloudformation_client(&self) -> aws_sdk_cloudformation::Client {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(self.region.clone()))
            .load()
            .await;
        aws_sdk_cloudformation::Client::new(&config)
    }

    pub async fn cloudtrail_client(&self) -> aws_sdk_cloudtrail::Client {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(self.region.clone()))
            .load()
            .await;
        aws_sdk_cloudtrail::Client::new(&config)
    }

    pub async fn lambda_client(&self) -> aws_sdk_lambda::Client {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(self.region.clone()))
            .load()
            .await;
        aws_sdk_lambda::Client::new(&config)
    }

    pub async fn iam_client(&self) -> aws_sdk_iam::Client {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(self.region.clone()))
            .load()
            .await;
        aws_sdk_iam::Client::new(&config)
    }

    pub async fn ecr_client(&self) -> aws_sdk_ecr::Client {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(self.region.clone()))
            .load()
            .await;
        aws_sdk_ecr::Client::new(&config)
    }

    pub async fn ecr_public_client(&self) -> aws_sdk_ecrpublic::Client {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(self.region.clone()))
            .load()
            .await;
        aws_sdk_ecrpublic::Client::new(&config)
    }

    pub async fn cloudwatch_client(&self) -> aws_sdk_cloudwatch::Client {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(self.region.clone()))
            .load()
            .await;
        aws_sdk_cloudwatch::Client::new(&config)
    }

    pub async fn sqs_client(&self) -> aws_sdk_sqs::Client {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(self.region.clone()))
            .load()
            .await;
        aws_sdk_sqs::Client::new(&config)
    }

    pub async fn cloudwatch_logs_client(&self) -> aws_sdk_cloudwatchlogs::Client {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(self.region.clone()))
            .load()
            .await;
        aws_sdk_cloudwatchlogs::Client::new(&config)
    }

    pub async fn pipes_client(&self) -> aws_sdk_pipes::Client {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(self.region.clone()))
            .load()
            .await;
        aws_sdk_pipes::Client::new(&config)
    }

    pub async fn ec2_client(&self) -> aws_sdk_ec2::Client {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(self.region.clone()))
            .load()
            .await;
        aws_sdk_ec2::Client::new(&config)
    }

    pub async fn apigateway_client(&self) -> aws_sdk_apigateway::Client {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(self.region.clone()))
            .load()
            .await;
        aws_sdk_apigateway::Client::new(&config)
    }

    pub async fn apigatewayv2_client(&self) -> aws_sdk_apigatewayv2::Client {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(self.region.clone()))
            .load()
            .await;
        aws_sdk_apigatewayv2::Client::new(&config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dummy_config_with_region() {
        let region = "us-west-2";
        let config = AwsConfig::dummy(Some(region.to_string()));
        assert_eq!(config.region, region);
        assert_eq!(config.account_id, "");
        assert!(!config.region_auto_detected);
    }

    #[test]
    fn test_dummy_config_without_region() {
        let config = AwsConfig::dummy(None);
        assert_eq!(config.region, "");
        assert_eq!(config.account_id, "");
        assert!(!config.region_auto_detected);
    }

    #[tokio::test]
    async fn test_new_fails_without_credentials() {
        // Clear AWS env vars to simulate no credentials
        std::env::remove_var("AWS_ACCESS_KEY_ID");
        std::env::remove_var("AWS_SECRET_ACCESS_KEY");
        std::env::remove_var("AWS_SESSION_TOKEN");
        std::env::set_var("AWS_PROFILE", "nonexistent-profile-test");

        let result = AwsConfig::new(Some("us-east-1".to_string())).await;

        // Should fail with credentials error before attempting region detection
        assert!(result.is_err());
        let err_str = format!("{}", result.unwrap_err());
        // Can be credentials error or dispatch failure (both indicate auth issues)
        assert!(
            err_str.contains("credentials")
                || err_str.contains("profile")
                || err_str.contains("dispatch"),
            "Expected auth error, got: {}",
            err_str
        );
    }

    #[tokio::test]
    async fn test_timeout_is_configurable() {
        std::env::remove_var("AWS_ACCESS_KEY_ID");
        std::env::remove_var("AWS_SECRET_ACCESS_KEY");
        std::env::set_var("AWS_PROFILE", "nonexistent-profile-test");

        // Test with very short timeout
        let result = AwsConfig::new_with_timeout(
            Some("us-east-1".to_string()),
            std::time::Duration::from_millis(100),
        )
        .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_dummy_config_preserves_values() {
        let region = "eu-west-1";
        let config = AwsConfig::dummy(Some(region.to_string()));
        assert_eq!(config.region, region);
        assert_eq!(config.account_id, "");
        assert_eq!(config.role_arn, "");
        assert!(!config.region_auto_detected);
    }
}
