use crate::common::{
    format_bytes, format_duration_seconds, format_unix_timestamp, ColumnTrait, UTC_TIMESTAMP_WIDTH,
};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;

#[derive(Debug, Clone)]
pub struct Queue {
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Column {
    Name,
    Type,
    Created,
    MessagesAvailable,
    MessagesInFlight,
    Encryption,
    ContentBasedDeduplication,
    LastUpdated,
    VisibilityTimeout,
    MessageRetentionPeriod,
    MaximumMessageSize,
    DeliveryDelay,
    ReceiveMessageWaitTime,
    HighThroughputFifo,
    DeduplicationScope,
    FifoThroughputLimit,
}

impl Column {
    pub fn all() -> Vec<Column> {
        vec![
            Column::Name,
            Column::Type,
            Column::Created,
            Column::MessagesAvailable,
            Column::MessagesInFlight,
            Column::Encryption,
            Column::ContentBasedDeduplication,
            Column::LastUpdated,
            Column::VisibilityTimeout,
            Column::MessageRetentionPeriod,
            Column::MaximumMessageSize,
            Column::DeliveryDelay,
            Column::ReceiveMessageWaitTime,
            Column::HighThroughputFifo,
            Column::DeduplicationScope,
            Column::FifoThroughputLimit,
        ]
    }
}

impl ColumnTrait for Column {
    fn name(&self) -> &'static str {
        match self {
            Column::Name => "Name",
            Column::Type => "Type",
            Column::Created => "Created",
            Column::MessagesAvailable => "Messages available",
            Column::MessagesInFlight => "Messages in flight",
            Column::Encryption => "Encryption",
            Column::ContentBasedDeduplication => "Content-based deduplication",
            Column::LastUpdated => "Last updated",
            Column::VisibilityTimeout => "Visibility timeout",
            Column::MessageRetentionPeriod => "Message retention period",
            Column::MaximumMessageSize => "Maximum message size",
            Column::DeliveryDelay => "Delivery delay",
            Column::ReceiveMessageWaitTime => "Receive message wait time",
            Column::HighThroughputFifo => "High throughput FIFO",
            Column::DeduplicationScope => "Deduplication scope",
            Column::FifoThroughputLimit => "FIFO throughput limit",
        }
    }
}

impl TableColumn<Queue> for Column {
    fn name(&self) -> &str {
        ColumnTrait::name(self)
    }

    fn width(&self) -> u16 {
        ColumnTrait::name(self).len().max(match self {
            Column::Name => 40,
            Column::Type => 10,
            Column::Created => UTC_TIMESTAMP_WIDTH as usize,
            Column::MessagesAvailable => 20,
            Column::MessagesInFlight => 20,
            Column::Encryption => 12,
            Column::ContentBasedDeduplication => 30,
            Column::LastUpdated => UTC_TIMESTAMP_WIDTH as usize,
            Column::VisibilityTimeout => 20,
            Column::MessageRetentionPeriod => 25,
            Column::MaximumMessageSize => 22,
            Column::DeliveryDelay => 15,
            Column::ReceiveMessageWaitTime => 25,
            Column::HighThroughputFifo => 22,
            Column::DeduplicationScope => 22,
            Column::FifoThroughputLimit => 22,
        }) as u16
    }

    fn render(&self, item: &Queue) -> (String, Style) {
        let text = match self {
            Column::Name => item.name.clone(),
            Column::Type => item.queue_type.clone(),
            Column::Created => format_unix_timestamp(&item.created_timestamp),
            Column::MessagesAvailable => item.messages_available.clone(),
            Column::MessagesInFlight => item.messages_in_flight.clone(),
            Column::Encryption => item.encryption.clone(),
            Column::ContentBasedDeduplication => item.content_based_deduplication.clone(),
            Column::LastUpdated => format_unix_timestamp(&item.last_modified_timestamp),
            Column::VisibilityTimeout => {
                if let Ok(seconds) = item.visibility_timeout.parse::<i32>() {
                    format_duration_seconds(seconds)
                } else {
                    item.visibility_timeout.clone()
                }
            }
            Column::MessageRetentionPeriod => {
                if let Ok(seconds) = item.message_retention_period.parse::<i32>() {
                    format_duration_seconds(seconds)
                } else {
                    item.message_retention_period.clone()
                }
            }
            Column::MaximumMessageSize => {
                // Parse bytes from "262144 bytes" format
                if let Some(bytes_str) = item.maximum_message_size.split_whitespace().next() {
                    if let Ok(bytes) = bytes_str.parse::<i64>() {
                        format_bytes(bytes)
                    } else {
                        item.maximum_message_size.clone()
                    }
                } else {
                    item.maximum_message_size.clone()
                }
            }
            Column::DeliveryDelay => {
                if let Ok(seconds) = item.delivery_delay.parse::<i32>() {
                    format_duration_seconds(seconds)
                } else {
                    item.delivery_delay.clone()
                }
            }
            Column::ReceiveMessageWaitTime => {
                if let Ok(seconds) = item.receive_message_wait_time.parse::<i32>() {
                    format_duration_seconds(seconds)
                } else {
                    item.receive_message_wait_time.clone()
                }
            }
            Column::HighThroughputFifo => item.high_throughput_fifo.clone(),
            Column::DeduplicationScope => item.deduplication_scope.clone(),
            Column::FifoThroughputLimit => item.fifo_throughput_limit.clone(),
        };
        (text, Style::default())
    }
}

pub fn console_url_queues(region: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/sqs/v3/home?region={}#/queues",
        region, region
    )
}

pub fn console_url_queue_detail(region: &str, queue_url: &str) -> String {
    let encoded_url = urlencoding::encode(queue_url);
    format!(
        "https://{}.console.aws.amazon.com/sqs/v3/home?region={}#/queues/{}",
        region, region, encoded_url
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_url_queues() {
        assert_eq!(
            console_url_queues("us-east-1"),
            "https://us-east-1.console.aws.amazon.com/sqs/v3/home?region=us-east-1#/queues"
        );
    }

    #[test]
    fn test_console_url_queue_detail() {
        let url = "https://sqs.us-east-1.amazonaws.com/654654343159/MyTest";
        assert_eq!(
            console_url_queue_detail("us-east-1", url),
            "https://us-east-1.console.aws.amazon.com/sqs/v3/home?region=us-east-1#/queues/https%3A%2F%2Fsqs.us-east-1.amazonaws.com%2F654654343159%2FMyTest"
        );
    }

    #[test]
    fn test_column_all() {
        assert_eq!(Column::all().len(), 16);
    }

    #[test]
    fn test_maximum_message_size_formatting() {
        let queue = Queue {
            name: "test".to_string(),
            url: String::new(),
            queue_type: "Standard".to_string(),
            created_timestamp: String::new(),
            messages_available: "0".to_string(),
            messages_in_flight: "0".to_string(),
            encryption: "Disabled".to_string(),
            content_based_deduplication: "Disabled".to_string(),
            last_modified_timestamp: String::new(),
            visibility_timeout: String::new(),
            message_retention_period: String::new(),
            maximum_message_size: "262144 bytes".to_string(),
            delivery_delay: String::new(),
            receive_message_wait_time: String::new(),
            high_throughput_fifo: "N/A".to_string(),
            deduplication_scope: "N/A".to_string(),
            fifo_throughput_limit: "N/A".to_string(),
        };

        let (text, _) = Column::MaximumMessageSize.render(&queue);
        assert_eq!(text, "262.14 KB");
    }

    #[test]
    fn test_duration_formatting() {
        let queue = Queue {
            name: "test".to_string(),
            url: String::new(),
            queue_type: "Standard".to_string(),
            created_timestamp: String::new(),
            messages_available: "0".to_string(),
            messages_in_flight: "0".to_string(),
            encryption: "Disabled".to_string(),
            content_based_deduplication: "Disabled".to_string(),
            last_modified_timestamp: String::new(),
            visibility_timeout: "30".to_string(),
            message_retention_period: "345600".to_string(),
            maximum_message_size: String::new(),
            delivery_delay: "0".to_string(),
            receive_message_wait_time: "20".to_string(),
            high_throughput_fifo: "N/A".to_string(),
            deduplication_scope: "N/A".to_string(),
            fifo_throughput_limit: "N/A".to_string(),
        };

        let (text, _) = Column::VisibilityTimeout.render(&queue);
        assert_eq!(text, "30sec");

        let (text, _) = Column::MessageRetentionPeriod.render(&queue);
        assert_eq!(text, "5760min 0sec");

        let (text, _) = Column::DeliveryDelay.render(&queue);
        assert_eq!(text, "0sec");

        let (text, _) = Column::ReceiveMessageWaitTime.render(&queue);
        assert_eq!(text, "20sec");
    }

    #[test]
    fn test_timestamp_formatting() {
        let queue = Queue {
            name: "test".to_string(),
            url: String::new(),
            queue_type: "Standard".to_string(),
            created_timestamp: "1609459200".to_string(),
            messages_available: "0".to_string(),
            messages_in_flight: "0".to_string(),
            encryption: "Disabled".to_string(),
            content_based_deduplication: "Disabled".to_string(),
            last_modified_timestamp: "1609459200".to_string(),
            visibility_timeout: String::new(),
            message_retention_period: String::new(),
            maximum_message_size: String::new(),
            delivery_delay: String::new(),
            receive_message_wait_time: String::new(),
            high_throughput_fifo: "N/A".to_string(),
            deduplication_scope: "N/A".to_string(),
            fifo_throughput_limit: "N/A".to_string(),
        };

        let (text, _) = Column::Created.render(&queue);
        assert!(text.contains("2021-01-01"));
        assert!(text.contains("(UTC)"));
    }
}
