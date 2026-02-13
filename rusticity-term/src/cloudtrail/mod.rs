pub mod events;
pub mod resources;

pub use events::{CloudTrailEvent, CloudTrailEventColumn};
pub use resources::{EventResource, EventResourceColumn};

use std::collections::HashMap;

pub fn init(i18n: &mut HashMap<String, String>) {
    events::init(i18n);
}
