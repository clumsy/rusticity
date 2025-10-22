use crate::common::UTC_TIMESTAMP_WIDTH;

pub fn console_url_list(region: &str) -> String {
    format!(
        "https://{}.console.aws.amazon.com/cloudwatch/home?region={}#logsV2:log-groups",
        region, region
    )
}

pub fn console_url_detail(region: &str, group_name: &str) -> String {
    let encoded_group = urlencoding::encode(group_name);
    format!(
        "https://{}.console.aws.amazon.com/cloudwatch/home?region={}#logsV2:log-groups/log-group/{}",
        region, region, encoded_group
    )
}

pub fn console_url_stream(region: &str, group_name: &str, stream_name: &str) -> String {
    let encoded_group = urlencoding::encode(group_name);
    let encoded_stream = urlencoding::encode(stream_name);
    format!(
        "https://{}.console.aws.amazon.com/cloudwatch/home?region={}#logsV2:log-groups/log-group/{}/log-events/{}",
        region, region, encoded_group, encoded_stream
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StreamColumn {
    LogStream,
    ARN,
    CreationTime,
    FirstEventTime,
    LastEventTime,
    LastIngestionTime,
    UploadSequenceToken,
}

impl StreamColumn {
    pub fn name(&self) -> &'static str {
        match self {
            StreamColumn::LogStream => "Log stream",
            StreamColumn::ARN => "ARN",
            StreamColumn::CreationTime => "Creation time",
            StreamColumn::FirstEventTime => "First event time",
            StreamColumn::LastEventTime => "Last event time",
            StreamColumn::LastIngestionTime => "Last ingestion time",
            StreamColumn::UploadSequenceToken => "Upload sequence token",
        }
    }

    pub fn width(&self) -> u16 {
        match self {
            StreamColumn::LogStream => 50,
            StreamColumn::ARN => 80,
            StreamColumn::CreationTime => UTC_TIMESTAMP_WIDTH,
            StreamColumn::FirstEventTime => UTC_TIMESTAMP_WIDTH,
            StreamColumn::LastEventTime => UTC_TIMESTAMP_WIDTH,
            StreamColumn::LastIngestionTime => UTC_TIMESTAMP_WIDTH,
            StreamColumn::UploadSequenceToken => 30,
        }
    }

    pub fn all() -> Vec<StreamColumn> {
        vec![
            StreamColumn::LogStream,
            StreamColumn::ARN,
            StreamColumn::CreationTime,
            StreamColumn::FirstEventTime,
            StreamColumn::LastEventTime,
            StreamColumn::LastIngestionTime,
            StreamColumn::UploadSequenceToken,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventColumn {
    Timestamp,
    IngestionTime,
    Message,
    EventId,
    LogStreamName,
}

impl EventColumn {
    pub fn name(&self) -> &'static str {
        match self {
            EventColumn::Timestamp => "Timestamp",
            EventColumn::IngestionTime => "Ingestion time",
            EventColumn::Message => "Message",
            EventColumn::EventId => "Event ID",
            EventColumn::LogStreamName => "Log stream name",
        }
    }

    pub fn width(&self) -> u16 {
        match self {
            EventColumn::Timestamp => UTC_TIMESTAMP_WIDTH,
            EventColumn::IngestionTime => UTC_TIMESTAMP_WIDTH,
            EventColumn::Message => 0, // Min constraint
            EventColumn::EventId => 40,
            EventColumn::LogStreamName => 50,
        }
    }

    pub fn all() -> Vec<EventColumn> {
        vec![
            EventColumn::Timestamp,
            EventColumn::IngestionTime,
            EventColumn::Message,
            EventColumn::EventId,
            EventColumn::LogStreamName,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_column_all_returns_seven_columns() {
        let columns = StreamColumn::all();
        assert_eq!(columns.len(), 7);
    }

    #[test]
    fn test_event_column_all_returns_five_columns() {
        let columns = EventColumn::all();
        assert_eq!(columns.len(), 5);
    }
}
