use crate::common::{
    format_bytes, format_duration_seconds, format_unix_timestamp, translate_column, ColumnId,
    UTC_TIMESTAMP_WIDTH,
};
use crate::ui::table::Column as TableColumn;
use ratatui::prelude::*;
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for col in Column::all() {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
}

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
    pub dead_letter_queue: String,
    pub messages_delayed: String,
    pub redrive_allow_policy: String,
    pub redrive_policy: String,
    pub redrive_task_id: String,
    pub redrive_task_start_time: String,
    pub redrive_task_status: String,
    pub redrive_task_percent: String,
    pub redrive_task_destination: String,
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
    const ID_NAME: &'static str = "column.sqs.queue.name";
    const ID_TYPE: &'static str = "column.sqs.queue.type";
    const ID_CREATED: &'static str = "column.sqs.queue.created";
    const ID_MESSAGES_AVAILABLE: &'static str = "column.sqs.queue.messages_available";
    const ID_MESSAGES_IN_FLIGHT: &'static str = "column.sqs.queue.messages_in_flight";
    const ID_ENCRYPTION: &'static str = "column.sqs.queue.encryption";
    const ID_CONTENT_BASED_DEDUPLICATION: &'static str =
        "column.sqs.queue.content_based_deduplication";
    const ID_LAST_UPDATED: &'static str = "column.sqs.queue.last_updated";
    const ID_VISIBILITY_TIMEOUT: &'static str = "column.sqs.queue.visibility_timeout";
    const ID_MESSAGE_RETENTION_PERIOD: &'static str = "column.sqs.queue.message_retention_period";
    const ID_MAXIMUM_MESSAGE_SIZE: &'static str = "column.sqs.queue.maximum_message_size";
    const ID_DELIVERY_DELAY: &'static str = "column.sqs.queue.delivery_delay";
    const ID_RECEIVE_MESSAGE_WAIT_TIME: &'static str = "column.sqs.queue.receive_message_wait_time";
    const ID_HIGH_THROUGHPUT_FIFO: &'static str = "column.sqs.queue.high_throughput_fifo";
    const ID_DEDUPLICATION_SCOPE: &'static str = "column.sqs.queue.deduplication_scope";
    const ID_FIFO_THROUGHPUT_LIMIT: &'static str = "column.sqs.queue.fifo_throughput_limit";

    pub const fn id(&self) -> ColumnId {
        match self {
            Column::Name => Self::ID_NAME,
            Column::Type => Self::ID_TYPE,
            Column::Created => Self::ID_CREATED,
            Column::MessagesAvailable => Self::ID_MESSAGES_AVAILABLE,
            Column::MessagesInFlight => Self::ID_MESSAGES_IN_FLIGHT,
            Column::Encryption => Self::ID_ENCRYPTION,
            Column::ContentBasedDeduplication => Self::ID_CONTENT_BASED_DEDUPLICATION,
            Column::LastUpdated => Self::ID_LAST_UPDATED,
            Column::VisibilityTimeout => Self::ID_VISIBILITY_TIMEOUT,
            Column::MessageRetentionPeriod => Self::ID_MESSAGE_RETENTION_PERIOD,
            Column::MaximumMessageSize => Self::ID_MAXIMUM_MESSAGE_SIZE,
            Column::DeliveryDelay => Self::ID_DELIVERY_DELAY,
            Column::ReceiveMessageWaitTime => Self::ID_RECEIVE_MESSAGE_WAIT_TIME,
            Column::HighThroughputFifo => Self::ID_HIGH_THROUGHPUT_FIFO,
            Column::DeduplicationScope => Self::ID_DEDUPLICATION_SCOPE,
            Column::FifoThroughputLimit => Self::ID_FIFO_THROUGHPUT_LIMIT,
        }
    }

    pub const fn default_name(&self) -> &'static str {
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

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            Self::ID_NAME => Some(Column::Name),
            Self::ID_TYPE => Some(Column::Type),
            Self::ID_CREATED => Some(Column::Created),
            Self::ID_MESSAGES_AVAILABLE => Some(Column::MessagesAvailable),
            Self::ID_MESSAGES_IN_FLIGHT => Some(Column::MessagesInFlight),
            Self::ID_ENCRYPTION => Some(Column::Encryption),
            Self::ID_CONTENT_BASED_DEDUPLICATION => Some(Column::ContentBasedDeduplication),
            Self::ID_LAST_UPDATED => Some(Column::LastUpdated),
            Self::ID_VISIBILITY_TIMEOUT => Some(Column::VisibilityTimeout),
            Self::ID_MESSAGE_RETENTION_PERIOD => Some(Column::MessageRetentionPeriod),
            Self::ID_MAXIMUM_MESSAGE_SIZE => Some(Column::MaximumMessageSize),
            Self::ID_DELIVERY_DELAY => Some(Column::DeliveryDelay),
            Self::ID_RECEIVE_MESSAGE_WAIT_TIME => Some(Column::ReceiveMessageWaitTime),
            Self::ID_HIGH_THROUGHPUT_FIFO => Some(Column::HighThroughputFifo),
            Self::ID_DEDUPLICATION_SCOPE => Some(Column::DeduplicationScope),
            Self::ID_FIFO_THROUGHPUT_LIMIT => Some(Column::FifoThroughputLimit),
            _ => None,
        }
    }

    pub fn all() -> [Column; 16] {
        [
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

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }
}

impl TableColumn<Queue> for Column {
    fn name(&self) -> &str {
        Box::leak(translate_column(self.id(), self.default_name()).into_boxed_str())
    }

    fn width(&self) -> u16 {
        let translated = translate_column(self.id(), self.default_name());
        translated.len().max(match self {
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

#[cfg(test)]
mod tests {
    use super::*;

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
            high_throughput_fifo: "-".to_string(),
            deduplication_scope: "-".to_string(),
            fifo_throughput_limit: "-".to_string(),
            dead_letter_queue: "-".to_string(),
            messages_delayed: "0".to_string(),
            redrive_allow_policy: "-".to_string(),
            redrive_policy: "".to_string(),
            redrive_task_id: "-".to_string(),
            redrive_task_start_time: "-".to_string(),
            redrive_task_status: "-".to_string(),
            redrive_task_percent: "-".to_string(),
            redrive_task_destination: "-".to_string(),
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
            high_throughput_fifo: "-".to_string(),
            deduplication_scope: "-".to_string(),
            fifo_throughput_limit: "-".to_string(),
            dead_letter_queue: "-".to_string(),
            messages_delayed: "0".to_string(),
            redrive_allow_policy: "-".to_string(),
            redrive_policy: "".to_string(),
            redrive_task_id: "-".to_string(),
            redrive_task_start_time: "-".to_string(),
            redrive_task_status: "-".to_string(),
            redrive_task_percent: "-".to_string(),
            redrive_task_destination: "-".to_string(),
        };

        let (text, _) = Column::VisibilityTimeout.render(&queue);
        assert_eq!(text, "30s");

        let (text, _) = Column::MessageRetentionPeriod.render(&queue);
        assert_eq!(text, "4d");

        let (text, _) = Column::DeliveryDelay.render(&queue);
        assert_eq!(text, "0s");

        let (text, _) = Column::ReceiveMessageWaitTime.render(&queue);
        assert_eq!(text, "20s");
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
            high_throughput_fifo: "-".to_string(),
            deduplication_scope: "-".to_string(),
            fifo_throughput_limit: "-".to_string(),
            dead_letter_queue: "-".to_string(),
            messages_delayed: "0".to_string(),
            redrive_allow_policy: "-".to_string(),
            redrive_policy: "".to_string(),
            redrive_task_id: "-".to_string(),
            redrive_task_start_time: "-".to_string(),
            redrive_task_status: "-".to_string(),
            redrive_task_percent: "-".to_string(),
            redrive_task_destination: "-".to_string(),
        };

        let (text, _) = Column::Created.render(&queue);
        assert!(text.contains("2021-01-01"));
        assert!(text.contains("(UTC)"));
    }
}

#[test]
fn test_column_ids_have_correct_prefix() {
    for col in Column::all() {
        assert!(
            col.id().starts_with("column.sqs.queue."),
            "Column ID '{}' should start with 'column.sqs.queue.'",
            col.id()
        );
    }
}
