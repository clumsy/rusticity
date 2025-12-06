use crate::config::AwsConfig;
use anyhow::Result;

#[derive(Clone, Debug)]
pub struct Stack {
    pub name: String,
    pub stack_id: String,
    pub status: String,
    pub created_time: String,
    pub updated_time: String,
    pub deleted_time: String,
    pub drift_status: String,
    pub last_drift_check_time: String,
    pub status_reason: String,
    pub description: String,
}

pub struct CloudFormationClient {
    config: AwsConfig,
}

impl CloudFormationClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    pub async fn list_stacks(&self, include_nested: bool) -> Result<Vec<Stack>> {
        let client = self.config.cloudformation_client().await;

        let mut stacks = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_stacks();
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(stack_summaries) = response.stack_summaries {
                for stack in stack_summaries {
                    // Skip nested stacks if not requested
                    if !include_nested {
                        if let Some(root_id) = &stack.root_id {
                            if root_id != stack.stack_id.as_deref().unwrap_or("") {
                                continue;
                            }
                        }
                    }

                    stacks.push(Stack {
                        name: stack.stack_name.unwrap_or_default(),
                        stack_id: stack.stack_id.unwrap_or_default(),
                        status: stack
                            .stack_status
                            .map(|s| s.as_str().to_string())
                            .unwrap_or_default(),
                        created_time: stack
                            .creation_time
                            .map(|dt| {
                                let timestamp = dt.secs();
                                let datetime = chrono::DateTime::from_timestamp(timestamp, 0)
                                    .unwrap_or_default();
                                datetime.format("%Y-%m-%d %H:%M:%S (UTC)").to_string()
                            })
                            .unwrap_or_default(),
                        updated_time: stack
                            .last_updated_time
                            .map(|dt| {
                                let timestamp = dt.secs();
                                let datetime = chrono::DateTime::from_timestamp(timestamp, 0)
                                    .unwrap_or_default();
                                datetime.format("%Y-%m-%d %H:%M:%S (UTC)").to_string()
                            })
                            .unwrap_or_default(),
                        deleted_time: stack
                            .deletion_time
                            .map(|dt| {
                                let timestamp = dt.secs();
                                let datetime = chrono::DateTime::from_timestamp(timestamp, 0)
                                    .unwrap_or_default();
                                datetime.format("%Y-%m-%d %H:%M:%S (UTC)").to_string()
                            })
                            .unwrap_or_default(),
                        drift_status: stack
                            .drift_information
                            .as_ref()
                            .and_then(|d| d.stack_drift_status.as_ref())
                            .map(|s| format!("{:?}", s))
                            .unwrap_or_default(),
                        last_drift_check_time: stack
                            .drift_information
                            .and_then(|d| d.last_check_timestamp)
                            .map(|dt| {
                                let timestamp = dt.secs();
                                let datetime = chrono::DateTime::from_timestamp(timestamp, 0)
                                    .unwrap_or_default();
                                datetime.format("%Y-%m-%d %H:%M:%S (UTC)").to_string()
                            })
                            .unwrap_or_default(),
                        status_reason: stack.stack_status_reason.unwrap_or_default(),
                        description: stack.template_description.unwrap_or_default(),
                    });
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(stacks)
    }

    pub async fn describe_stack(&self, stack_name: &str) -> Result<StackDetails> {
        let client = self.config.cloudformation_client().await;

        let response = client
            .describe_stacks()
            .stack_name(stack_name)
            .send()
            .await?;

        let stack = response
            .stacks()
            .first()
            .ok_or_else(|| anyhow::anyhow!("Stack not found"))?;

        Ok(StackDetails {
            detailed_status: String::new(),
            root_stack: stack.root_id().unwrap_or("").to_string(),
            parent_stack: stack.parent_id().unwrap_or("").to_string(),
            termination_protection: stack.enable_termination_protection().unwrap_or(false),
            iam_role: stack.role_arn().unwrap_or("").to_string(),
            tags: stack
                .tags()
                .iter()
                .map(|t| {
                    (
                        t.key().unwrap_or("").to_string(),
                        t.value().unwrap_or("").to_string(),
                    )
                })
                .collect(),
            stack_policy: String::new(),
            rollback_monitoring_time: String::new(),
            rollback_alarms: stack
                .rollback_configuration()
                .map(|rc| {
                    rc.rollback_triggers()
                        .iter()
                        .map(|t| t.arn().unwrap_or("").to_string())
                        .collect()
                })
                .unwrap_or_default(),
            notification_arns: stack
                .notification_arns()
                .iter()
                .map(|s| s.to_string())
                .collect(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct StackDetails {
    pub detailed_status: String,
    pub root_stack: String,
    pub parent_stack: String,
    pub termination_protection: bool,
    pub iam_role: String,
    pub tags: Vec<(String, String)>,
    pub stack_policy: String,
    pub rollback_monitoring_time: String,
    pub rollback_alarms: Vec<String>,
    pub notification_arns: Vec<String>,
}

impl CloudFormationClient {
    pub async fn get_template(&self, stack_name: &str) -> Result<String> {
        let client = self.config.cloudformation_client().await;
        let response = client.get_template().stack_name(stack_name).send().await?;

        Ok(response.template_body().unwrap_or("").to_string())
    }
}
