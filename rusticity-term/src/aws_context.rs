//! The AWS context: the profile, the resolved Region, account and role, and
//! the service clients built for them, as one value.
//!
//! Its parts never change one at a time. A new profile or Region connects a
//! new context off-screen ([`AwsContext::connect`], bounded), and only a
//! successful connection replaces the current one, whole, through
//! [`crate::app::App::replace_context`]. A failed or slow connection leaves the
//! current context, and everything shown for it, untouched.
use anyhow::{anyhow, Result};
use rusticity_core::{
    AlarmsClient, ApiGatewayClient, AwsConfig, CloudFormationClient, CloudTrailClient,
    CloudWatchClient, DynamoDbClient, Ec2Client, EcrClient, EfsClient, FsxClient, IamClient,
    KmsClient, LambdaClient, RdsClient, S3Client, SecretsManagerClient, SnsClient, SqsClient,
    StepFunctionsClient, VpcClient,
};
use std::time::Duration;

/// Bound on signing in and resolving the identity of a new context.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

pub struct AwsContext {
    pub profile: String,
    /// The resolved Region, account and role.
    pub config: AwsConfig,
    pub cloudwatch_client: CloudWatchClient,
    pub cloudtrail_client: CloudTrailClient,
    pub s3_client: S3Client,
    pub sqs_client: SqsClient,
    pub sns_client: SnsClient,
    pub secretsmanager_client: SecretsManagerClient,
    pub stepfunctions_client: StepFunctionsClient,
    pub alarms_client: AlarmsClient,
    pub ec2_client: Ec2Client,
    pub ecr_client: EcrClient,
    pub kms_client: KmsClient,
    pub efs_client: EfsClient,
    pub fsx_client: FsxClient,
    pub rds_client: RdsClient,
    pub vpc_client: VpcClient,
    pub dynamodb_client: DynamoDbClient,
    pub apig_client: ApiGatewayClient,
    pub iam_client: IamClient,
    pub lambda_client: LambdaClient,
    pub cloudformation_client: CloudFormationClient,
}

/// A profile and Region to connect; `None` uses the environment and the
/// profile default, as at startup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextRequest {
    pub profile: Option<String>,
    pub region: Option<String>,
}

impl AwsContext {
    /// Sign in with `profile` in `region` and build every client. Unbounded;
    /// see [`AwsContext::connect_bounded`].
    pub async fn connect(profile: Option<String>, region: Option<String>) -> Result<Self> {
        let profile = profile
            .or_else(|| std::env::var("AWS_PROFILE").ok())
            .ok_or_else(|| {
                anyhow!(
                    "No AWS profile specified. Set AWS_PROFILE environment variable or select a profile."
                )
            })?;
        // The SDK credential chain reads the profile from the environment.
        std::env::set_var("AWS_PROFILE", &profile);
        let config = AwsConfig::new(region).await?;
        let cloudwatch_client = CloudWatchClient::new(config.clone()).await?;
        Ok(Self::with_clients(profile, config, cloudwatch_client))
    }

    /// [`AwsContext::connect`] bounded by [`CONNECT_TIMEOUT`].
    pub async fn connect_bounded(request: ContextRequest) -> Result<Self> {
        tokio::time::timeout(
            CONNECT_TIMEOUT,
            Self::connect(request.profile, request.region),
        )
        .await
        .map_err(|_| anyhow!("Connecting to AWS timed out after {CONNECT_TIMEOUT:?}"))?
    }

    /// A context that never signs in: no credentials, no account. For a
    /// startup without credentials and for tests.
    pub fn offline(profile: String, region: Option<String>) -> Self {
        let config = AwsConfig::dummy(region);
        let cloudwatch_client = CloudWatchClient::dummy(config.clone());
        Self::with_clients(profile, config, cloudwatch_client)
    }

    fn with_clients(
        profile: String,
        config: AwsConfig,
        cloudwatch_client: CloudWatchClient,
    ) -> Self {
        Self {
            profile,
            cloudwatch_client,
            cloudtrail_client: CloudTrailClient::new(config.clone()),
            s3_client: S3Client::new(config.clone()),
            sqs_client: SqsClient::new(config.clone()),
            sns_client: SnsClient::new(config.clone()),
            secretsmanager_client: SecretsManagerClient::new(config.clone()),
            stepfunctions_client: StepFunctionsClient::new(config.clone()),
            alarms_client: AlarmsClient::new(config.clone()),
            ec2_client: Ec2Client::new(config.clone()),
            ecr_client: EcrClient::new(config.clone()),
            kms_client: KmsClient::new(config.clone()),
            efs_client: EfsClient::new(config.clone()),
            fsx_client: FsxClient::new(config.clone()),
            rds_client: RdsClient::new(config.clone()),
            vpc_client: VpcClient::new(config.clone()),
            dynamodb_client: DynamoDbClient::new(config.clone()),
            apig_client: ApiGatewayClient::new(config.clone()),
            iam_client: IamClient::new(config.clone()),
            lambda_client: LambdaClient::new(config.clone()),
            cloudformation_client: CloudFormationClient::new(config.clone()),
            config,
        }
    }

    /// This context with the same profile and Region, for a reconnect.
    pub fn request(&self) -> ContextRequest {
        ContextRequest {
            profile: Some(self.profile.clone()).filter(|p| !p.is_empty()),
            region: Some(self.config.region.clone()).filter(|r| !r.is_empty()),
        }
    }
}

#[cfg(test)]
mod tests;
