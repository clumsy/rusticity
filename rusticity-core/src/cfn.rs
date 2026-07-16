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
    pub root_stack: String,
    pub parent_stack: String,
}

pub struct CloudFormationClient {
    config: AwsConfig,
}

impl CloudFormationClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    pub async fn list_stacks(&self, include_nested: bool) -> Result<Vec<Stack>> {
        let client = self.config.cloudformation_client();

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
                        root_stack: stack.root_id.unwrap_or_default(),
                        parent_stack: stack.parent_id.unwrap_or_default(),
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
        let client = self.config.cloudformation_client();

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
        let client = self.config.cloudformation_client();
        let response = client.get_template().stack_name(stack_name).send().await?;

        Ok(response.template_body().unwrap_or("").to_string())
    }

    pub async fn get_stack_parameters(&self, stack_name: &str) -> Result<Vec<StackParameter>> {
        let client = self.config.cloudformation_client();
        let response = client
            .describe_stacks()
            .stack_name(stack_name)
            .send()
            .await?;

        let stack = response
            .stacks()
            .first()
            .ok_or_else(|| anyhow::anyhow!("Stack not found"))?;

        let mut parameters = Vec::new();
        for param in stack.parameters() {
            parameters.push(StackParameter {
                key: param.parameter_key().unwrap_or("").to_string(),
                value: param.parameter_value().unwrap_or("").to_string(),
                resolved_value: param.resolved_value().unwrap_or("").to_string(),
            });
        }

        Ok(parameters)
    }

    pub async fn get_stack_outputs(&self, stack_name: &str) -> Result<Vec<StackOutput>> {
        let client = self.config.cloudformation_client();
        let response = client
            .describe_stacks()
            .stack_name(stack_name)
            .send()
            .await?;

        let stack = response
            .stacks()
            .first()
            .ok_or_else(|| anyhow::anyhow!("Stack not found"))?;

        let mut outputs = Vec::new();
        for output in stack.outputs() {
            outputs.push(StackOutput {
                key: output.output_key().unwrap_or("").to_string(),
                value: output.output_value().unwrap_or("").to_string(),
                description: output.description().unwrap_or("").to_string(),
                export_name: output.export_name().unwrap_or("").to_string(),
            });
        }

        outputs.sort_by(|a, b| a.key.cmp(&b.key));

        Ok(outputs)
    }

    pub async fn get_stack_resources(&self, stack_name: &str) -> Result<Vec<StackResource>> {
        let client = self.config.cloudformation_client();
        let response = client
            .describe_stack_resources()
            .stack_name(stack_name)
            .send()
            .await?;

        let mut resources = Vec::new();
        for resource in response.stack_resources() {
            resources.push(StackResource {
                logical_id: resource.logical_resource_id().unwrap_or("").to_string(),
                physical_id: resource.physical_resource_id().unwrap_or("").to_string(),
                resource_type: resource.resource_type().unwrap_or("").to_string(),
                status: resource
                    .resource_status()
                    .map(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
                module_info: resource
                    .module_info()
                    .and_then(|m| m.logical_id_hierarchy())
                    .unwrap_or("")
                    .to_string(),
            });
        }

        resources.sort_by(|a, b| a.logical_id.cmp(&b.logical_id));

        Ok(resources)
    }

    pub async fn list_stack_events(&self, stack_name: &str) -> Result<Vec<StackEvent>> {
        let client = self.config.cloudformation_client();
        let mut events = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = client.describe_stack_events().stack_name(stack_name);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let response = req.send().await?;

            for e in response.stack_events() {
                events.push(StackEvent {
                    event_id: e.event_id().unwrap_or("").to_string(),
                    timestamp: e
                        .timestamp()
                        .map(|t| {
                            t.fmt(aws_smithy_types::date_time::Format::DateTime)
                                .unwrap_or_default()
                        })
                        .unwrap_or_default(),
                    logical_id: e.logical_resource_id().unwrap_or("").to_string(),
                    status: e
                        .resource_status()
                        .map(|s| s.as_str())
                        .unwrap_or("")
                        .to_string(),
                    detailed_status: String::new(),
                    status_reason: e.resource_status_reason().unwrap_or("").to_string(),
                    hook_invocation_count: String::new(),
                    resource_type: e.resource_type().unwrap_or("").to_string(),
                    physical_id: e.physical_resource_id().unwrap_or("").to_string(),
                    client_request_token: e.client_request_token().unwrap_or("").to_string(),
                    operation_id: String::new(),
                });
            }

            next_token = response.next_token().map(|s| s.to_string());
            if next_token.is_none() {
                break;
            }
        }

        // Already sorted newest-first by the API, keep that order
        Ok(events)
    }

    pub async fn list_change_sets(&self, stack_name: &str) -> Result<Vec<StackChangeSet>> {
        let client = self.config.cloudformation_client();
        let mut change_sets = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut req = client.list_change_sets().stack_name(stack_name);
            if let Some(ref token) = next_token {
                req = req.next_token(token);
            }
            let response = req.send().await?;

            for cs in response.summaries() {
                let created_time = cs
                    .creation_time()
                    .map(|t| {
                        t.fmt(aws_smithy_types::date_time::Format::DateTime)
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();

                change_sets.push(StackChangeSet {
                    name: cs.change_set_name().unwrap_or("").to_string(),
                    change_set_id: cs.change_set_id().unwrap_or("").to_string(),
                    created_time,
                    status: cs.status().map(|s| s.as_str()).unwrap_or("").to_string(),
                    description: cs.description().unwrap_or("").to_string(),
                    root_change_set_id: cs.root_change_set_id().unwrap_or("").to_string(),
                    parent_change_set_id: cs.parent_change_set_id().unwrap_or("").to_string(),
                });
            }

            next_token = response.next_token().map(|s| s.to_string());
            if next_token.is_none() {
                break;
            }
        }

        Ok(change_sets)
    }
}

#[derive(Debug, Clone)]
pub struct StackParameter {
    pub key: String,
    pub value: String,
    pub resolved_value: String,
}

#[derive(Debug, Clone)]
pub struct StackOutput {
    pub key: String,
    pub value: String,
    pub description: String,
    pub export_name: String,
}

#[derive(Debug, Clone)]
pub struct StackResource {
    pub logical_id: String,
    pub physical_id: String,
    pub resource_type: String,
    pub status: String,
    pub module_info: String,
}

#[derive(Debug, Clone)]
pub struct StackEvent {
    pub event_id: String,
    pub timestamp: String,
    pub logical_id: String,
    pub status: String,
    pub detailed_status: String,
    pub status_reason: String,
    pub hook_invocation_count: String,
    pub resource_type: String,
    pub physical_id: String,
    pub client_request_token: String,
    pub operation_id: String,
}

#[derive(Debug, Clone)]
pub struct StackChangeSet {
    pub name: String,
    pub change_set_id: String,
    pub created_time: String,
    pub status: String,
    pub description: String,
    pub root_change_set_id: String,
    pub parent_change_set_id: String,
}
