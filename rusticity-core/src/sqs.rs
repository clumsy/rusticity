use crate::config::AwsConfig;
use anyhow::Result;

#[derive(Clone, Debug)]
pub struct SqsQueue {
    pub name: String,
    pub url: String,
    pub queue_type: String,
    pub created_timestamp: String,
    pub messages_available: String,
    pub messages_in_flight: String,
    pub encryption: String,
    pub content_based_deduplication: String,
    pub last_modified_timestamp: String,
    pub visibility_timeout: String,
    pub message_retention_period: String,
    pub maximum_message_size: String,
    pub delivery_delay: String,
    pub receive_message_wait_time: String,
    pub high_throughput_fifo: String,
    pub deduplication_scope: String,
    pub fifo_throughput_limit: String,
    pub dead_letter_queue: String,
    pub messages_delayed: String,
    pub redrive_allow_policy: String,
    pub redrive_policy: String,
}

#[derive(Clone, Debug)]
pub struct LambdaTrigger {
    pub uuid: String,
    pub arn: String,
    pub status: String,
    pub last_modified: String,
}

#[derive(Clone, Debug)]
pub struct EventBridgePipe {
    pub name: String,
    pub status: String,
    pub target: String,
    pub last_modified: String,
}

#[derive(Clone, Debug)]
pub struct QueueTag {
    pub key: String,
    pub value: String,
}

pub struct SqsClient {
    config: AwsConfig,
}

impl SqsClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
    }

    pub async fn list_lambda_triggers(&self, queue_arn: &str) -> Result<Vec<LambdaTrigger>> {
        let lambda_client = self.config.lambda_client().await;

        let response = lambda_client
            .list_event_source_mappings()
            .event_source_arn(queue_arn)
            .send()
            .await?;

        let mut triggers = Vec::new();
        if let Some(mappings) = response.event_source_mappings {
            for mapping in mappings {
                triggers.push(LambdaTrigger {
                    uuid: mapping.uuid.unwrap_or_default(),
                    arn: mapping.function_arn.unwrap_or_default(),
                    status: mapping
                        .state
                        .map(|s| s.as_str().to_string())
                        .unwrap_or_else(|| "Unknown".to_string()),
                    last_modified: mapping
                        .last_modified
                        .map(|dt| dt.secs().to_string())
                        .unwrap_or_default(),
                });
            }
        }

        Ok(triggers)
    }

    pub async fn list_queues(&self, prefix: &str) -> Result<Vec<SqsQueue>> {
        let client = self.config.sqs_client().await;

        let mut request = client.list_queues();
        if !prefix.is_empty() {
            request = request.queue_name_prefix(prefix);
        }

        let response = request.send().await?;
        let mut queues = Vec::new();

        if let Some(urls) = response.queue_urls {
            for url in urls {
                let attrs_response = client
                    .get_queue_attributes()
                    .queue_url(&url)
                    .attribute_names(aws_sdk_sqs::types::QueueAttributeName::All)
                    .send()
                    .await?;

                let attrs = attrs_response.attributes.unwrap_or_default();

                let name = url.split('/').next_back().unwrap_or(&url).to_string();
                let queue_type = if name.ends_with(".fifo") {
                    "FIFO".to_string()
                } else {
                    "Standard".to_string()
                };

                queues.push(SqsQueue {
                    name,
                    url: url.clone(),
                    queue_type,
                    created_timestamp: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::CreatedTimestamp)
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    messages_available: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::ApproximateNumberOfMessages)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "0".to_string()),
                    messages_in_flight: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::ApproximateNumberOfMessagesNotVisible)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "0".to_string()),
                    encryption: if attrs.contains_key(&aws_sdk_sqs::types::QueueAttributeName::KmsMasterKeyId) {
                        "Enabled".to_string()
                    } else {
                        "Disabled".to_string()
                    },
                    content_based_deduplication: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::ContentBasedDeduplication)
                        .map(|v| if v == "true" { "Enabled" } else { "Disabled" })
                        .unwrap_or("Disabled")
                        .to_string(),
                    last_modified_timestamp: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::LastModifiedTimestamp)
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    visibility_timeout: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::VisibilityTimeout)
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    message_retention_period: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::MessageRetentionPeriod)
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    maximum_message_size: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::MaximumMessageSize)
                        .map(|v| format!("{} bytes", v))
                        .unwrap_or_default(),
                    delivery_delay: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::DelaySeconds)
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    receive_message_wait_time: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::ReceiveMessageWaitTimeSeconds)
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    high_throughput_fifo: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::FifoThroughputLimit)
                        .map(|v| if v == "perMessageGroupId" { "Disabled" } else { "Enabled" })
                        .unwrap_or("-")
                        .to_string(),
                    deduplication_scope: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::DeduplicationScope)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    fifo_throughput_limit: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::FifoThroughputLimit)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    dead_letter_queue: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::RedrivePolicy)
                        .and_then(|policy| {
                            // Parse JSON to extract deadLetterTargetArn
                            serde_json::from_str::<serde_json::Value>(policy)
                                .ok()
                                .and_then(|v| v.get("deadLetterTargetArn").and_then(|arn| arn.as_str()).map(|s| {
                                    // Extract queue name from ARN
                                    s.split(':').next_back().unwrap_or(s).to_string()
                                }))
                        })
                        .unwrap_or_else(|| "-".to_string()),
                    messages_delayed: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::ApproximateNumberOfMessagesDelayed)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "0".to_string()),
                    redrive_allow_policy: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::RedriveAllowPolicy)
                        .map(|policy| {
                            // Parse JSON to extract redrivePermission
                            serde_json::from_str::<serde_json::Value>(policy)
                                .ok()
                                .and_then(|v| v.get("redrivePermission").and_then(|p| p.as_str()).map(String::from))
                                .unwrap_or_else(|| policy.to_string())
                        })
                        .unwrap_or_else(|| "-".to_string()),
                    redrive_policy: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::RedrivePolicy)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "".to_string()),
                });
            }
        }

        queues.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(queues)
    }

    pub async fn get_queue_arn(&self, queue_url: &str) -> Result<String> {
        let client = self.config.sqs_client().await;

        let response = client
            .get_queue_attributes()
            .queue_url(queue_url)
            .attribute_names(aws_sdk_sqs::types::QueueAttributeName::QueueArn)
            .send()
            .await?;

        let arn = response
            .attributes()
            .and_then(|attrs| attrs.get(&aws_sdk_sqs::types::QueueAttributeName::QueueArn))
            .map(|v| v.to_string())
            .unwrap_or_default();

        Ok(arn)
    }

    pub async fn list_pipes(&self, queue_arn: &str) -> Result<Vec<EventBridgePipe>> {
        let pipes_client = self.config.pipes_client().await;

        let response = pipes_client
            .list_pipes()
            .source_prefix(queue_arn)
            .send()
            .await?;

        let mut pipes = Vec::new();
        if let Some(pipe_list) = response.pipes {
            for pipe in pipe_list {
                pipes.push(EventBridgePipe {
                    name: pipe.name.unwrap_or_default(),
                    status: pipe
                        .current_state
                        .map(|s| s.as_str().to_string())
                        .unwrap_or_default(),
                    target: pipe.target.unwrap_or_default(),
                    last_modified: pipe
                        .last_modified_time
                        .map(|dt| dt.secs().to_string())
                        .unwrap_or_default(),
                });
            }
        }

        Ok(pipes)
    }

    pub async fn list_tags(&self, queue_arn: &str) -> Result<Vec<QueueTag>> {
        let client = self.config.sqs_client().await;

        let response = client.list_queue_tags().queue_url(queue_arn).send().await?;

        let mut tags = Vec::new();
        if let Some(tag_map) = response.tags {
            for (key, value) in tag_map {
                tags.push(QueueTag { key, value });
            }
        }

        Ok(tags)
    }
}
