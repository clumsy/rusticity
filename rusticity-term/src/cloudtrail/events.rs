use crate::common::translate_column;
use crate::common::{ColumnId, UTC_TIMESTAMP_WIDTH};
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    for col in CloudTrailEventColumn::all() {
        i18n.entry(col.id().to_string())
            .or_insert_with(|| col.default_name().to_string());
    }
}

#[derive(Debug, Clone)]
pub struct CloudTrailEvent {
    pub event_time: String,
    pub event_name: String,
    pub username: String,
    pub event_source: String,
    pub resource_type: String,
    pub resource_name: String,
    pub read_only: String,
    pub aws_region: String,
    pub event_id: String,
    pub access_key_id: String,
    pub source_ip_address: String,
    pub error_code: String,
    pub request_id: String,
    pub event_type: String,
    pub cloud_trail_event_json: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CloudTrailEventColumn {
    EventName,
    EventTime,
    Username,
    EventSource,
    ResourceType,
    ResourceName,
    ReadOnly,
    AwsRegion,
    EventId,
    AccessKeyId,
    SourceIpAddress,
    ErrorCode,
    RequestId,
    EventType,
}

impl CloudTrailEventColumn {
    const ID_EVENT_NAME: &'static str = "column.cloudtrail.event.event_name";
    const ID_EVENT_TIME: &'static str = "column.cloudtrail.event.event_time";
    const ID_USERNAME: &'static str = "column.cloudtrail.event.username";
    const ID_EVENT_SOURCE: &'static str = "column.cloudtrail.event.event_source";
    const ID_RESOURCE_TYPE: &'static str = "column.cloudtrail.event.resource_type";
    const ID_RESOURCE_NAME: &'static str = "column.cloudtrail.event.resource_name";
    const ID_READ_ONLY: &'static str = "column.cloudtrail.event.read_only";
    const ID_AWS_REGION: &'static str = "column.cloudtrail.event.aws_region";
    const ID_EVENT_ID: &'static str = "column.cloudtrail.event.event_id";
    const ID_ACCESS_KEY_ID: &'static str = "column.cloudtrail.event.access_key_id";
    const ID_SOURCE_IP_ADDRESS: &'static str = "column.cloudtrail.event.source_ip_address";
    const ID_ERROR_CODE: &'static str = "column.cloudtrail.event.error_code";
    const ID_REQUEST_ID: &'static str = "column.cloudtrail.event.request_id";
    const ID_EVENT_TYPE: &'static str = "column.cloudtrail.event.event_type";

    pub const fn id(&self) -> &'static str {
        match self {
            CloudTrailEventColumn::EventName => Self::ID_EVENT_NAME,
            CloudTrailEventColumn::EventTime => Self::ID_EVENT_TIME,
            CloudTrailEventColumn::Username => Self::ID_USERNAME,
            CloudTrailEventColumn::EventSource => Self::ID_EVENT_SOURCE,
            CloudTrailEventColumn::ResourceType => Self::ID_RESOURCE_TYPE,
            CloudTrailEventColumn::ResourceName => Self::ID_RESOURCE_NAME,
            CloudTrailEventColumn::ReadOnly => Self::ID_READ_ONLY,
            CloudTrailEventColumn::AwsRegion => Self::ID_AWS_REGION,
            CloudTrailEventColumn::EventId => Self::ID_EVENT_ID,
            CloudTrailEventColumn::AccessKeyId => Self::ID_ACCESS_KEY_ID,
            CloudTrailEventColumn::SourceIpAddress => Self::ID_SOURCE_IP_ADDRESS,
            CloudTrailEventColumn::ErrorCode => Self::ID_ERROR_CODE,
            CloudTrailEventColumn::RequestId => Self::ID_REQUEST_ID,
            CloudTrailEventColumn::EventType => Self::ID_EVENT_TYPE,
        }
    }

    pub const fn default_name(&self) -> &'static str {
        match self {
            CloudTrailEventColumn::EventName => "Event name",
            CloudTrailEventColumn::EventTime => "Event time",
            CloudTrailEventColumn::Username => "User name",
            CloudTrailEventColumn::EventSource => "Event source",
            CloudTrailEventColumn::ResourceType => "Resource type",
            CloudTrailEventColumn::ResourceName => "Resource name",
            CloudTrailEventColumn::ReadOnly => "Read-only",
            CloudTrailEventColumn::AwsRegion => "AWS region",
            CloudTrailEventColumn::EventId => "Event ID",
            CloudTrailEventColumn::AccessKeyId => "AWS access key",
            CloudTrailEventColumn::SourceIpAddress => "Source IP address",
            CloudTrailEventColumn::ErrorCode => "Error code",
            CloudTrailEventColumn::RequestId => "Request ID",
            CloudTrailEventColumn::EventType => "Event type",
        }
    }

    pub fn name(&self) -> String {
        translate_column(self.id(), self.default_name())
    }

    pub fn width(&self) -> u16 {
        match self {
            CloudTrailEventColumn::EventName => 35,
            CloudTrailEventColumn::EventTime => UTC_TIMESTAMP_WIDTH,
            CloudTrailEventColumn::Username => 25,
            CloudTrailEventColumn::EventSource => 30,
            CloudTrailEventColumn::ResourceType => 25,
            CloudTrailEventColumn::ResourceName => 40,
            CloudTrailEventColumn::ReadOnly => 12,
            CloudTrailEventColumn::AwsRegion => 15,
            CloudTrailEventColumn::EventId => 40,
            CloudTrailEventColumn::AccessKeyId => 25,
            CloudTrailEventColumn::SourceIpAddress => 18,
            CloudTrailEventColumn::ErrorCode => 20,
            CloudTrailEventColumn::RequestId => 40,
            CloudTrailEventColumn::EventType => 20,
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            Self::ID_EVENT_NAME => Some(CloudTrailEventColumn::EventName),
            Self::ID_EVENT_TIME => Some(CloudTrailEventColumn::EventTime),
            Self::ID_USERNAME => Some(CloudTrailEventColumn::Username),
            Self::ID_EVENT_SOURCE => Some(CloudTrailEventColumn::EventSource),
            Self::ID_RESOURCE_TYPE => Some(CloudTrailEventColumn::ResourceType),
            Self::ID_RESOURCE_NAME => Some(CloudTrailEventColumn::ResourceName),
            Self::ID_READ_ONLY => Some(CloudTrailEventColumn::ReadOnly),
            Self::ID_AWS_REGION => Some(CloudTrailEventColumn::AwsRegion),
            Self::ID_EVENT_ID => Some(CloudTrailEventColumn::EventId),
            Self::ID_ACCESS_KEY_ID => Some(CloudTrailEventColumn::AccessKeyId),
            Self::ID_SOURCE_IP_ADDRESS => Some(CloudTrailEventColumn::SourceIpAddress),
            Self::ID_ERROR_CODE => Some(CloudTrailEventColumn::ErrorCode),
            Self::ID_REQUEST_ID => Some(CloudTrailEventColumn::RequestId),
            Self::ID_EVENT_TYPE => Some(CloudTrailEventColumn::EventType),
            _ => None,
        }
    }

    pub fn all() -> [CloudTrailEventColumn; 14] {
        [
            CloudTrailEventColumn::EventName,
            CloudTrailEventColumn::EventTime,
            CloudTrailEventColumn::Username,
            CloudTrailEventColumn::EventSource,
            CloudTrailEventColumn::ResourceType,
            CloudTrailEventColumn::ResourceName,
            CloudTrailEventColumn::ReadOnly,
            CloudTrailEventColumn::AwsRegion,
            CloudTrailEventColumn::EventId,
            CloudTrailEventColumn::AccessKeyId,
            CloudTrailEventColumn::SourceIpAddress,
            CloudTrailEventColumn::ErrorCode,
            CloudTrailEventColumn::RequestId,
            CloudTrailEventColumn::EventType,
        ]
    }

    pub fn ids() -> Vec<ColumnId> {
        Self::all().iter().map(|c| c.id()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_ids_have_correct_prefix() {
        for col in CloudTrailEventColumn::all() {
            assert!(
                col.id().starts_with("column.cloudtrail.event."),
                "CloudTrailEventColumn ID '{}' should start with 'column.cloudtrail.event.'",
                col.id()
            );
        }
    }
}
