use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogGroup {
    pub name: String,
    pub creation_time: Option<DateTime<Utc>>,
    pub stored_bytes: Option<i64>,
    pub retention_days: Option<i32>,
    pub log_class: Option<String>,
    pub arn: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogStream {
    pub name: String,
    pub creation_time: Option<DateTime<Utc>>,
    pub last_event_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub timestamp: DateTime<Utc>,
    pub message: String,
}
