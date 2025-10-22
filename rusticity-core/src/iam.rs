use crate::config::AwsConfig;

pub struct IamClient {
    config: AwsConfig,
}

impl IamClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    pub async fn list_users(&self) -> Result<Vec<aws_sdk_iam::types::User>, String> {
        let client = self.config.iam_client().await;

        let mut users = Vec::new();
        let mut marker = None;

        loop {
            let mut request = client.list_users();
            if let Some(m) = marker {
                request = request.marker(m);
            }

            let response = request
                .send()
                .await
                .map_err(|e| format!("Failed to list users: {}", e))?;

            let is_truncated = response.is_truncated();
            marker = response.marker;
            users.extend(response.users);

            if !is_truncated {
                break;
            }
        }

        Ok(users)
    }

    pub async fn list_roles(&self) -> Result<Vec<aws_sdk_iam::types::Role>, String> {
        let client = self.config.iam_client().await;

        let mut roles = Vec::new();
        let mut marker = None;

        loop {
            let mut request = client.list_roles();
            if let Some(m) = marker {
                request = request.marker(m);
            }

            let response = request
                .send()
                .await
                .map_err(|e| format!("Failed to list roles: {}", e))?;

            let is_truncated = response.is_truncated();
            marker = response.marker;
            roles.extend(response.roles);

            if !is_truncated {
                break;
            }
        }

        Ok(roles)
    }

    pub async fn list_groups(&self) -> Result<Vec<aws_sdk_iam::types::Group>, String> {
        let client = self.config.iam_client().await;

        let mut groups = Vec::new();
        let mut marker = None;

        loop {
            let mut request = client.list_groups();
            if let Some(m) = marker {
                request = request.marker(m);
            }

            let response = request
                .send()
                .await
                .map_err(|e| format!("Failed to list groups: {}", e))?;

            let is_truncated = response.is_truncated();
            marker = response.marker;
            groups.extend(response.groups);

            if !is_truncated {
                break;
            }
        }

        Ok(groups)
    }

    pub async fn list_attached_role_policies(
        &self,
        role_name: &str,
    ) -> Result<Vec<aws_sdk_iam::types::AttachedPolicy>, String> {
        let client = self.config.iam_client().await;

        let response = client
            .list_attached_role_policies()
            .role_name(role_name)
            .send()
            .await
            .map_err(|e| format!("Failed to list attached role policies: {}", e))?;

        Ok(response.attached_policies.unwrap_or_default())
    }

    pub async fn list_attached_group_policies(
        &self,
        group_name: &str,
    ) -> Result<Vec<aws_sdk_iam::types::AttachedPolicy>, String> {
        let client = self.config.iam_client().await;

        let response = client
            .list_attached_group_policies()
            .group_name(group_name)
            .send()
            .await
            .map_err(|e| format!("Failed to list attached group policies: {}", e))?;

        Ok(response.attached_policies.unwrap_or_default())
    }

    pub async fn list_role_policies(&self, role_name: &str) -> Result<Vec<String>, String> {
        let client = self.config.iam_client().await;

        let response = client
            .list_role_policies()
            .role_name(role_name)
            .send()
            .await
            .map_err(|e| format!("Failed to list role policies: {}", e))?;

        Ok(response.policy_names)
    }

    pub async fn get_group(&self, group_name: &str) -> Result<usize, String> {
        let client = self.config.iam_client().await;

        let response = client
            .get_group()
            .group_name(group_name)
            .send()
            .await
            .map_err(|e| format!("Failed to get group: {}", e))?;

        Ok(response.users.len())
    }

    pub async fn get_group_users(
        &self,
        group_name: &str,
    ) -> Result<Vec<aws_sdk_iam::types::User>, String> {
        let client = self.config.iam_client().await;

        let response = client
            .get_group()
            .group_name(group_name)
            .send()
            .await
            .map_err(|e| format!("Failed to get group users: {}", e))?;

        Ok(response.users)
    }

    pub async fn list_group_policies(&self, group_name: &str) -> Result<Vec<String>, String> {
        let client = self.config.iam_client().await;

        let response = client
            .list_group_policies()
            .group_name(group_name)
            .send()
            .await
            .map_err(|e| format!("Failed to list group policies: {}", e))?;

        Ok(response.policy_names)
    }

    pub async fn get_role_policy(
        &self,
        role_name: &str,
        policy_name: &str,
    ) -> Result<String, String> {
        let client = self.config.iam_client().await;

        let response = client
            .get_role_policy()
            .role_name(role_name)
            .policy_name(policy_name)
            .send()
            .await
            .map_err(|e| format!("Failed to get role policy: {}", e))?;

        let policy_document = response.policy_document();

        // URL decode and pretty print JSON
        let decoded = percent_encoding::percent_decode_str(policy_document)
            .decode_utf8()
            .map_err(|e| format!("Failed to decode policy: {}", e))?;

        let json: serde_json::Value = serde_json::from_str(&decoded)
            .map_err(|e| format!("Failed to parse policy JSON: {}", e))?;

        serde_json::to_string_pretty(&json)
            .map_err(|e| format!("Failed to format policy JSON: {}", e))
    }

    pub async fn get_policy_version(&self, policy_arn: &str) -> Result<String, String> {
        let client = self.config.iam_client().await;

        // Get the policy to find the default version
        let policy_response = client
            .get_policy()
            .policy_arn(policy_arn)
            .send()
            .await
            .map_err(|e| format!("Failed to get policy: {}", e))?;

        let default_version = policy_response
            .policy()
            .and_then(|p| p.default_version_id())
            .ok_or_else(|| "No default version found".to_string())?;

        // Get the policy version document
        let version_response = client
            .get_policy_version()
            .policy_arn(policy_arn)
            .version_id(default_version)
            .send()
            .await
            .map_err(|e| format!("Failed to get policy version: {}", e))?;

        let policy_document = version_response
            .policy_version()
            .and_then(|v| v.document())
            .ok_or_else(|| "No policy document found".to_string())?;

        // URL decode and pretty print JSON
        let decoded = percent_encoding::percent_decode_str(policy_document)
            .decode_utf8()
            .map_err(|e| format!("Failed to decode policy: {}", e))?;

        let json: serde_json::Value = serde_json::from_str(&decoded)
            .map_err(|e| format!("Failed to parse policy JSON: {}", e))?;

        serde_json::to_string_pretty(&json)
            .map_err(|e| format!("Failed to format policy JSON: {}", e))
    }

    pub async fn get_role(&self, role_name: &str) -> Result<String, String> {
        let client = self.config.iam_client().await;

        let response = client
            .get_role()
            .role_name(role_name)
            .send()
            .await
            .map_err(|e| format!("Failed to get role: {}", e))?;

        let assume_role_policy = response
            .role()
            .and_then(|r| r.assume_role_policy_document())
            .ok_or_else(|| "No assume role policy document found".to_string())?;

        // URL decode and pretty print JSON
        let decoded = percent_encoding::percent_decode_str(assume_role_policy)
            .decode_utf8()
            .map_err(|e| format!("Failed to decode policy: {}", e))?;

        let json: serde_json::Value = serde_json::from_str(&decoded)
            .map_err(|e| format!("Failed to parse policy JSON: {}", e))?;

        serde_json::to_string_pretty(&json)
            .map_err(|e| format!("Failed to format policy JSON: {}", e))
    }

    pub async fn list_role_tags(&self, role_name: &str) -> Result<Vec<(String, String)>, String> {
        let client = self.config.iam_client().await;

        let response = client
            .list_role_tags()
            .role_name(role_name)
            .send()
            .await
            .map_err(|e| format!("Failed to list role tags: {}", e))?;

        Ok(response
            .tags()
            .iter()
            .map(|t| (t.key().to_string(), t.value().to_string()))
            .collect())
    }

    pub async fn list_user_tags(&self, user_name: &str) -> Result<Vec<(String, String)>, String> {
        let client = self.config.iam_client().await;

        let response = client
            .list_user_tags()
            .user_name(user_name)
            .send()
            .await
            .map_err(|e| format!("Failed to list user tags: {}", e))?;

        Ok(response
            .tags()
            .iter()
            .map(|t| (t.key().to_string(), t.value().to_string()))
            .collect())
    }

    pub async fn get_login_profile(&self, user_name: &str) -> Result<bool, String> {
        let client = self.config.iam_client().await;

        match client.get_login_profile().user_name(user_name).send().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub async fn list_access_keys(&self, user_name: &str) -> Result<usize, String> {
        let client = self.config.iam_client().await;

        let response = client
            .list_access_keys()
            .user_name(user_name)
            .send()
            .await
            .map_err(|e| format!("Failed to list access keys: {}", e))?;

        Ok(response.access_key_metadata().len())
    }
}
