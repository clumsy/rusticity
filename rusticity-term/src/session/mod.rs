mod core;
pub mod ui;

pub use core::{Session, SessionTab};
pub use ui::render_session_picker;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_unique() {
        let session1 = Session::new(
            "profile1".to_string(),
            "us-east-1".to_string(),
            "123".to_string(),
            String::new(),
        );

        std::thread::sleep(std::time::Duration::from_millis(1100));

        let session2 = Session::new(
            "profile2".to_string(),
            "us-west-2".to_string(),
            "456".to_string(),
            String::new(),
        );

        assert_ne!(session1.id, session2.id);
    }
}
