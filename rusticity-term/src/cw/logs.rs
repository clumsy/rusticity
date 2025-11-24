use crate::common::{format_bytes, format_timestamp, ColumnId, UTC_TIMESTAMP_WIDTH};
use crate::ui::table;
use ratatui::prelude::*;
use rusticity_core::{LogEvent, LogGroup, LogStream};
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for col in [
        StreamColumn::LogStream,
        StreamColumn::ARN,
        StreamColumn::CreationTime,
        StreamColumn::FirstEventTime,
        StreamColumn::LastEventTime,
        StreamColumn::LastIngestionTime,
        StreamColumn::UploadSequenceToken,
    ] {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }

    for col in [EventColumn::Timestamp, EventColumn::Message] {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }

    for col in [
        LogGroupColumn::LogGroup,
        LogGroupColumn::LogClass,
        LogGroupColumn::Retention,
        LogGroupColumn::StoredBytes,
        LogGroupColumn::CreationTime,
        LogGroupColumn::ARN,
    ] {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
}

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

// Log Group Columns
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogGroupColumn {
    LogGroup,
    LogClass,
    Retention,
    StoredBytes,
    CreationTime,
    ARN,
}

impl LogGroupColumn {
    pub fn id(&self) -> &'static str {
        match self {
            LogGroupColumn::LogGroup => "column.cw.group.log_group",
            LogGroupColumn::LogClass => "column.cw.group.log_class",
            LogGroupColumn::Retention => "column.cw.group.retention",
            LogGroupColumn::StoredBytes => "column.cw.group.stored_bytes",
            LogGroupColumn::CreationTime => "column.cw.group.creation_time",
            LogGroupColumn::ARN => "column.cw.group.arn",
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            LogGroupColumn::LogGroup => "Log group",
            LogGroupColumn::LogClass => "Log class",
            LogGroupColumn::Retention => "Retention",
            LogGroupColumn::StoredBytes => "Stored bytes",
            LogGroupColumn::CreationTime => "Creation time",
            LogGroupColumn::ARN => "ARN",
        }
    }

    pub fn from_id(id: ColumnId) -> Option<Self> {
        Self::try_from(id).ok()
    }

    pub fn all() -> [LogGroupColumn; 6] {
        [
            LogGroupColumn::LogGroup,
            LogGroupColumn::LogClass,
            LogGroupColumn::Retention,
            LogGroupColumn::StoredBytes,
            LogGroupColumn::CreationTime,
            LogGroupColumn::ARN,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }

    pub fn default_visible() -> Vec<ColumnId> {
        vec![
            LogGroupColumn::LogGroup.id(),
            LogGroupColumn::StoredBytes.id(),
        ]
    }
}

impl TryFrom<ColumnId> for LogGroupColumn {
    type Error = ();

    fn try_from(id: ColumnId) -> Result<Self, Self::Error> {
        Self::all().into_iter().find(|c| c.id() == id).ok_or(())
    }
}

impl table::Column<LogGroup> for LogGroupColumn {
    fn id(&self) -> &'static str {
        Self::id(self)
    }

    fn default_name(&self) -> &'static str {
        Self::default_name(self)
    }

    fn width(&self) -> u16 {
        match self {
            Self::LogGroup => 50,
            Self::LogClass => 15,
            Self::Retention => 10,
            Self::StoredBytes => 15,
            Self::CreationTime => UTC_TIMESTAMP_WIDTH,
            Self::ARN => 80,
        }
    }

    fn render(&self, item: &LogGroup) -> (String, Style) {
        let text = match self {
            Self::LogGroup => item.name.clone(),
            Self::LogClass => item.log_class.clone().unwrap_or_else(|| "-".to_string()),
            Self::Retention => item
                .retention_days
                .map(|d| d.to_string())
                .unwrap_or_else(|| "Never".to_string()),
            Self::StoredBytes => format_bytes(item.stored_bytes.unwrap_or(0)),
            Self::CreationTime => item
                .creation_time
                .map(|t| format_timestamp(&t))
                .unwrap_or_else(|| "-".to_string()),
            Self::ARN => item.arn.clone().unwrap_or_else(|| "-".to_string()),
        };
        (text, Style::default())
    }
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
    pub fn id(&self) -> &'static str {
        match self {
            StreamColumn::LogStream => "column.cw.stream.log_stream",
            StreamColumn::ARN => "column.cw.stream.arn",
            StreamColumn::CreationTime => "column.cw.stream.creation_time",
            StreamColumn::FirstEventTime => "column.cw.stream.first_event_time",
            StreamColumn::LastEventTime => "column.cw.stream.last_event_time",
            StreamColumn::LastIngestionTime => "column.cw.stream.last_ingestion_time",
            StreamColumn::UploadSequenceToken => "column.cw.stream.upload_sequence_token",
        }
    }

    pub fn default_name(&self) -> &'static str {
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

    pub fn from_id(id: ColumnId) -> Option<Self> {
        Self::try_from(id).ok()
    }

    pub fn all() -> [StreamColumn; 7] {
        [
            StreamColumn::LogStream,
            StreamColumn::ARN,
            StreamColumn::CreationTime,
            StreamColumn::FirstEventTime,
            StreamColumn::LastEventTime,
            StreamColumn::LastIngestionTime,
            StreamColumn::UploadSequenceToken,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }

    pub fn default_visible() -> Vec<ColumnId> {
        [
            StreamColumn::LogStream,
            StreamColumn::CreationTime,
            StreamColumn::LastEventTime,
        ]
        .iter()
        .map(|c| c.id())
        .collect()
    }
}

impl TryFrom<ColumnId> for StreamColumn {
    type Error = ();

    fn try_from(id: ColumnId) -> Result<Self, Self::Error> {
        Self::all().into_iter().find(|c| c.id() == id).ok_or(())
    }
}

impl table::Column<LogStream> for StreamColumn {
    fn id(&self) -> &'static str {
        Self::id(self)
    }

    fn default_name(&self) -> &'static str {
        Self::default_name(self)
    }

    fn width(&self) -> u16 {
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

    fn render(&self, item: &LogStream) -> (String, Style) {
        let text = match self {
            StreamColumn::LogStream => item.name.clone(),
            StreamColumn::ARN => "-".to_string(),
            StreamColumn::CreationTime => item
                .creation_time
                .map(|t| format_timestamp(&t))
                .unwrap_or_else(|| "-".to_string()),
            StreamColumn::FirstEventTime => "-".to_string(),
            StreamColumn::LastEventTime => item
                .last_event_time
                .map(|t| format_timestamp(&t))
                .unwrap_or_else(|| "-".to_string()),
            StreamColumn::LastIngestionTime => "-".to_string(),
            StreamColumn::UploadSequenceToken => "-".to_string(),
        };
        (text, Style::default())
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
    pub fn id(&self) -> &'static str {
        match self {
            EventColumn::Timestamp => "column.cw.event.timestamp",
            EventColumn::IngestionTime => "column.cw.event.ingestion_time",
            EventColumn::Message => "column.cw.event.message",
            EventColumn::EventId => "column.cw.event.event_id",
            EventColumn::LogStreamName => "column.cw.event.log_stream_name",
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            EventColumn::Timestamp => "Timestamp",
            EventColumn::IngestionTime => "Ingestion time",
            EventColumn::Message => "Message",
            EventColumn::EventId => "Event ID",
            EventColumn::LogStreamName => "Log stream name",
        }
    }

    pub fn from_id(id: ColumnId) -> Option<Self> {
        Self::try_from(id).ok()
    }

    pub fn all() -> [EventColumn; 5] {
        [
            EventColumn::Timestamp,
            EventColumn::IngestionTime,
            EventColumn::Message,
            EventColumn::EventId,
            EventColumn::LogStreamName,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }

    pub fn default_visible() -> Vec<ColumnId> {
        [EventColumn::Timestamp, EventColumn::Message]
            .iter()
            .map(|c| c.id())
            .collect()
    }
}

impl TryFrom<ColumnId> for EventColumn {
    type Error = ();

    fn try_from(id: ColumnId) -> Result<Self, Self::Error> {
        Self::all().into_iter().find(|c| c.id() == id).ok_or(())
    }
}

impl table::Column<LogEvent> for EventColumn {
    fn id(&self) -> &'static str {
        Self::id(self)
    }

    fn default_name(&self) -> &'static str {
        Self::default_name(self)
    }

    fn width(&self) -> u16 {
        match self {
            EventColumn::Timestamp => UTC_TIMESTAMP_WIDTH,
            EventColumn::IngestionTime => UTC_TIMESTAMP_WIDTH,
            EventColumn::Message => 100,
            EventColumn::EventId => 30,
            EventColumn::LogStreamName => 50,
        }
    }

    fn render(&self, item: &LogEvent) -> (String, Style) {
        let text = match self {
            EventColumn::Timestamp => format_timestamp(&item.timestamp),
            EventColumn::IngestionTime => "-".to_string(),
            EventColumn::Message => item.message.clone(),
            EventColumn::EventId => "-".to_string(),
            EventColumn::LogStreamName => "-".to_string(),
        };
        (text, Style::default())
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

    #[test]
    fn test_log_group_column_id_returns_full_key() {
        let id = LogGroupColumn::LogGroup.id();
        assert_eq!(id, "column.cw.group.log_group");
        assert!(id.starts_with("column."));
    }

    #[test]
    fn test_stream_column_id_returns_full_key() {
        let id = StreamColumn::LogStream.id();
        assert_eq!(id, "column.cw.stream.log_stream");
        assert!(id.starts_with("column."));
    }

    #[test]
    fn test_event_column_id_returns_full_key() {
        let id = EventColumn::Timestamp.id();
        assert_eq!(id, "column.cw.event.timestamp");
        assert!(id.starts_with("column."));
    }
}
