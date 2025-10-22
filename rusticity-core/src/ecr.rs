use crate::config::AwsConfig;
use anyhow::Result;

#[derive(Clone, Debug)]
pub struct EcrRepository {
    pub name: String,
    pub uri: String,
    pub created_at: String,
    pub tag_immutability: String,
    pub encryption_type: String,
}

#[derive(Clone, Debug)]
pub struct EcrImage {
    pub tag: String,
    pub artifact_type: String,
    pub pushed_at: String,
    pub size_bytes: i64,
    pub uri: String,
    pub digest: String,
    pub last_pull_time: String,
}

pub struct EcrClient {
    config: AwsConfig,
}

impl EcrClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    pub async fn list_private_repositories(&self) -> Result<Vec<EcrRepository>> {
        let client = self.config.ecr_client().await;

        let mut repositories = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.describe_repositories();
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(repos) = response.repositories {
                for repo in repos {
                    repositories.push(EcrRepository {
                        name: repo.repository_name.unwrap_or_default(),
                        uri: repo.repository_uri.unwrap_or_default(),
                        created_at: repo
                            .created_at
                            .map(|dt| {
                                dt.fmt(aws_smithy_types::date_time::Format::DateTime)
                                    .unwrap_or_default()
                            })
                            .unwrap_or_default(),
                        tag_immutability: repo
                            .image_tag_mutability
                            .map(|m| format!("{:?}", m))
                            .unwrap_or_default(),
                        encryption_type: repo
                            .encryption_configuration
                            .map(|e| {
                                let enc_type = format!("{:?}", e.encryption_type);
                                match enc_type.as_str() {
                                    "Aes256" => "AES256".to_string(),
                                    _ => enc_type,
                                }
                            })
                            .unwrap_or_default(),
                    });
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(repositories)
    }

    pub async fn list_public_repositories(&self) -> Result<Vec<EcrRepository>> {
        let client = self.config.ecr_public_client().await;

        let mut repositories = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.describe_repositories();
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(repos) = response.repositories {
                for repo in repos {
                    repositories.push(EcrRepository {
                        name: repo.repository_name.unwrap_or_default(),
                        uri: repo.repository_uri.unwrap_or_default(),
                        created_at: repo
                            .created_at
                            .map(|dt| {
                                dt.fmt(aws_smithy_types::date_time::Format::DateTime)
                                    .unwrap_or_default()
                            })
                            .unwrap_or_default(),
                        tag_immutability: String::new(),
                        encryption_type: String::new(),
                    });
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(repositories)
    }

    pub async fn list_images(
        &self,
        repository_name: &str,
        repository_uri: &str,
    ) -> Result<Vec<EcrImage>> {
        let client = self.config.ecr_client().await;

        let mut images = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.describe_images().repository_name(repository_name);
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(image_details) = response.image_details {
                for image in image_details {
                    let tags = image.image_tags.unwrap_or_default();
                    let tag = tags
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "<untagged>".to_string());

                    let size_bytes = image.image_size_in_bytes.unwrap_or(0);

                    let uri = format!("{}:{}", repository_uri, tag);

                    images.push(EcrImage {
                        tag,
                        artifact_type: image.artifact_media_type.unwrap_or_default(),
                        pushed_at: image
                            .image_pushed_at
                            .map(|dt| {
                                dt.fmt(aws_smithy_types::date_time::Format::DateTime)
                                    .unwrap_or_default()
                            })
                            .unwrap_or_default(),
                        size_bytes,
                        uri,
                        digest: image.image_digest.unwrap_or_default(),
                        last_pull_time: image
                            .last_recorded_pull_time
                            .map(|dt| {
                                dt.fmt(aws_smithy_types::date_time::Format::DateTime)
                                    .unwrap_or_default()
                            })
                            .unwrap_or_default(),
                    });
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(images)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_ecr_image_uri_format() {
        let repository_uri = "123456789012.dkr.ecr.us-east-1.amazonaws.com/my-repo";
        let tag = "v1.0.0";
        let expected_uri = "123456789012.dkr.ecr.us-east-1.amazonaws.com/my-repo:v1.0.0";

        let uri = format!("{}:{}", repository_uri, tag);
        assert_eq!(uri, expected_uri);
    }

    #[test]
    fn test_ecr_image_uri_with_untagged() {
        let repository_uri = "123456789012.dkr.ecr.us-east-1.amazonaws.com/my-repo";
        let tag = "<untagged>";
        let expected_uri = "123456789012.dkr.ecr.us-east-1.amazonaws.com/my-repo:<untagged>";

        let uri = format!("{}:{}", repository_uri, tag);
        assert_eq!(uri, expected_uri);
    }

    #[test]
    fn test_encryption_type_aes256_formatting() {
        let enc_type = "Aes256";
        let formatted = match enc_type {
            "Aes256" => "AES256".to_string(),
            _ => enc_type.to_string(),
        };
        assert_eq!(formatted, "AES256");
    }

    #[test]
    fn test_encryption_type_kms_unchanged() {
        let enc_type = "Kms";
        let formatted = match enc_type {
            "Aes256" => "AES256".to_string(),
            _ => enc_type.to_string(),
        };
        assert_eq!(formatted, "Kms");
    }
}
