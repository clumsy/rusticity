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
}

pub struct SqsClient {
    config: AwsConfig,
}

impl SqsClient {
    pub fn new(config: AwsConfig) -> Self {
        Self { config }
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
                        .map(|v| format!("{} seconds", v))
                        .unwrap_or_default(),
                    message_retention_period: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::MessageRetentionPeriod)
                        .map(|v| format!("{} seconds", v))
                        .unwrap_or_default(),
                    maximum_message_size: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::MaximumMessageSize)
                        .map(|v| format!("{} bytes", v))
                        .unwrap_or_default(),
                    delivery_delay: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::DelaySeconds)
                        .map(|v| format!("{} seconds", v))
                        .unwrap_or_default(),
                    receive_message_wait_time: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::ReceiveMessageWaitTimeSeconds)
                        .map(|v| format!("{} seconds", v))
                        .unwrap_or_default(),
                    high_throughput_fifo: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::FifoThroughputLimit)
                        .map(|v| if v == "perMessageGroupId" { "Disabled" } else { "Enabled" })
                        .unwrap_or("N/A")
                        .to_string(),
                    deduplication_scope: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::DeduplicationScope)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "N/A".to_string()),
                    fifo_throughput_limit: attrs
                        .get(&aws_sdk_sqs::types::QueueAttributeName::FifoThroughputLimit)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "N/A".to_string()),
                });
            }
        }

        queues.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(queues)
    }
}
