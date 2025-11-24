pub mod alarms;
pub mod insights;
pub mod logs;

// Re-export types
pub use alarms::{Alarm, AlarmColumn};
pub use logs::{EventColumn, StreamColumn};

pub fn init() {
    alarms::init();
    logs::init();
}
