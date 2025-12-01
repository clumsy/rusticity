pub mod alarms;
pub mod insights;
pub mod logs;

// Re-export types
pub use alarms::{Alarm, AlarmColumn};
pub use logs::{EventColumn, LogGroupColumn, StreamColumn};
use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    alarms::init(i18n);
    logs::init(i18n);
}
