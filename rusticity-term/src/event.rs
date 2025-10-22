use crossterm::event::{self, Event};
use std::time::Duration;

pub struct EventHandler;

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn next(&self) -> anyhow::Result<Option<Event>> {
        if event::poll(Duration::from_millis(100))? {
            Ok(Some(event::read()?))
        } else {
            Ok(None)
        }
    }
}
