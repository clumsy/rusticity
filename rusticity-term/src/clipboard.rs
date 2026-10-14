//! One cross-platform clipboard path for text yanks and rendered snapshots.
use ratatui::{buffer::Buffer, text::Span};
use std::{
    cell::RefCell,
    time::{Duration, Instant},
};

const NOTICE_DURATION: Duration = Duration::from_secs(4);

enum CopyRequest {
    Text(String),
    Snapshot,
}

pub struct CopyNotice {
    pub message: String,
    pub failed: bool,
    created: Instant,
}

#[derive(Default)]
pub struct ClipboardState {
    pending: RefCell<Option<CopyRequest>>,
    notice: Option<CopyNotice>,
}

impl ClipboardState {
    /// Interior mutability lets read-only service actions queue a copy without
    /// writing to the OS, borrowing App mutably, or accessing a global clipboard.
    pub fn request_text(&self, text: impl Into<String>) {
        self.pending.replace(Some(CopyRequest::Text(text.into())));
    }
    pub fn request_snapshot(&self) {
        self.pending.replace(Some(CopyRequest::Snapshot));
    }
    pub fn snapshot_requested(&self) -> bool {
        matches!(*self.pending.borrow(), Some(CopyRequest::Snapshot))
    }
    pub fn notice(&self) -> Option<&CopyNotice> {
        self.notice
            .as_ref()
            .filter(|notice| notice.created.elapsed() < NOTICE_DURATION)
    }
    pub fn expire_notice(&mut self) -> bool {
        if self
            .notice
            .as_ref()
            .is_some_and(|notice| notice.created.elapsed() >= NOTICE_DURATION)
        {
            self.notice = None;
            true
        } else {
            false
        }
    }
    /// The injected writer keeps request/payload/result behavior testable without
    /// modifying the developer's actual clipboard.
    pub fn complete_with(
        &mut self,
        buffer: &Buffer,
        write: impl FnOnce(String) -> Result<(), String>,
    ) -> bool {
        let Some(request) = self.pending.take() else {
            return false;
        };
        let (text, success) = match request {
            CopyRequest::Text(text) => (text, "Copied to clipboard"),
            CopyRequest::Snapshot => (snapshot_text(buffer), "Snapshot copied to clipboard"),
        };
        let (message, failed) = match write(text) {
            Ok(()) => (success.into(), false),
            Err(error) => (format!("Failed to copy: {error}"), true),
        };
        self.notice = Some(CopyNotice {
            message,
            failed,
            created: Instant::now(),
        });
        true
    }
}

/// Keep the OS backend alive for the application lifetime (important for X11
/// clipboard ownership), initializing only on the first actual copy request.
#[derive(Default)]
pub struct ClipboardService {
    backend: Option<arboard::Clipboard>,
}
impl ClipboardService {
    pub fn flush(&mut self, state: &mut ClipboardState, buffer: &Buffer) -> bool {
        state.complete_with(buffer, |text| {
            if self.backend.is_none() {
                self.backend = Some(arboard::Clipboard::new().map_err(|e| e.to_string())?);
            }
            self.backend
                .as_mut()
                .expect("clipboard initialized")
                .set_text(text)
                .map_err(|e| e.to_string())
        })
    }
}

/// Export exactly the visible characters, excluding the continuation cells of
/// wide graphemes. Supports nonzero buffer origins and trims trailing padding.
pub fn snapshot_text(buffer: &Buffer) -> String {
    let area = buffer.area;
    (area.y..area.bottom())
        .map(|y| {
            let mut line = String::new();
            let mut x = area.x;
            while x < area.right() {
                let symbol = buffer[(x, y)].symbol();
                line.push_str(symbol);
                let width = Span::raw(symbol)
                    .width()
                    .max(1)
                    .min((area.right() - x) as usize);
                x += width as u16;
            }
            line.trim_end().to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{layout::Rect, style::Style};

    #[test]
    fn snapshots_preserve_unicode_without_extra_wide_character_spaces() {
        let mut buffer = Buffer::empty(Rect::new(3, 7, 18, 3));
        buffer.set_string(3, 7, "界 e\u{301} ✅", Style::default());
        buffer.set_string(3, 8, "  indented  ", Style::default());
        assert_eq!(snapshot_text(&buffer), "界 e\u{301} ✅\n  indented\n");
        assert_eq!(snapshot_text(&Buffer::empty(Rect::new(0, 0, 0, 0))), "");
    }

    #[test]
    fn text_and_snapshots_share_the_writer_and_report_results() {
        let mut state = ClipboardState::default();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 1));
        buffer.set_string(0, 0, "screen", Style::default());
        state.request_text("arn:test");
        assert!(state.complete_with(&buffer, |text| {
            assert_eq!(text, "arn:test");
            Ok(())
        }));
        assert_eq!(state.notice().unwrap().message, "Copied to clipboard");
        assert!(!state.notice().unwrap().failed);
        assert!(!state.complete_with(&buffer, |_| panic!("No second write")));
        state.request_snapshot();
        assert!(state.snapshot_requested());
        state.complete_with(&buffer, |text| {
            assert_eq!(text, "screen");
            Ok(())
        });
        assert!(!state.snapshot_requested());
        assert_eq!(
            state.notice().unwrap().message,
            "Snapshot copied to clipboard"
        );
        state.request_text("secret-not-in-error-message");
        state.complete_with(&buffer, |_| Err("clipboard unavailable".into()));
        assert!(state.notice().unwrap().failed);
        assert_eq!(
            state.notice().unwrap().message,
            "Failed to copy: clipboard unavailable"
        );
    }

    #[test]
    fn copy_notices_expire_and_latest_request_wins() {
        let mut state = ClipboardState::default();
        let buffer = Buffer::empty(Rect::new(0, 0, 0, 0));
        state.request_snapshot();
        state.request_text("latest");
        assert!(!state.snapshot_requested());
        state.complete_with(&buffer, |text| {
            assert_eq!(text, "latest");
            Ok(())
        });
        state.notice.as_mut().unwrap().created = Instant::now() - NOTICE_DURATION;
        assert!(state.notice().is_none());
        assert!(state.expire_notice());
        assert!(!state.expire_notice());
    }
}

#[cfg(test)]
mod app_tests {
    use super::*;
    use crate::app::{App, Service};
    use crate::keymap::{Action, Mode};
    use ratatui::{backend::TestBackend, layout::Rect, Terminal};

    #[test]
    fn clipboard_service_yanks_keep_payloads_and_do_not_replace_aws_errors() {
        let mut app = App::new_without_client("test".into(), None);
        app.current_service = Service::DynamoDbTables;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.dynamodb_state.tables.items = vec![rusticity_core::dynamodb::DdbTable {
            name: "orders".into(),
            table_arn: "arn:aws:dynamodb:us-east-2:123456789012:table/orders".into(),
            ..Default::default()
        }];
        let buffer = Buffer::empty(Rect::new(0, 0, 1, 1));
        app.handle_action(Action::Yank);
        app.clipboard.complete_with(&buffer, |text| {
            assert_eq!(text, "orders");
            Ok(())
        });
        app.dynamodb_state.current_table = Some("orders".into());
        app.handle_action(Action::Yank);
        app.clipboard.complete_with(&buffer, |text| {
            assert_eq!(text, "arn:aws:dynamodb:us-east-2:123456789012:table/orders");
            Ok(())
        });
        app.mode = Mode::ErrorModal;
        app.error_message = Some("AWS access denied".into());
        app.handle_action(Action::Yank);
        app.clipboard.complete_with(&buffer, |text| {
            assert_eq!(text, "AWS access denied");
            Ok(())
        });
        assert_eq!(app.error_message.as_deref(), Some("AWS access denied"));
        assert_eq!(app.mode, Mode::ErrorModal);
        app.clipboard.request_text("retry");
        app.clipboard
            .complete_with(&buffer, |_| Err("no clipboard".into()));
        assert_eq!(app.error_message.as_deref(), Some("AWS access denied"));
        assert_eq!(app.mode, Mode::ErrorModal);
    }

    #[test]
    fn clipboard_notices_render_in_status_bar_without_opening_a_modal() {
        let mut app = App::new_without_client("test".into(), None);
        app.mode = Mode::Normal;
        let buffer = Buffer::empty(Rect::new(0, 0, 1, 1));
        for failed in [false, true] {
            app.clipboard.request_text("data");
            app.clipboard.complete_with(&buffer, |_| {
                if failed {
                    Err("unavailable".into())
                } else {
                    Ok(())
                }
            });
            let mut terminal = Terminal::new(TestBackend::new(160, 25)).unwrap();
            terminal
                .draw(|frame| crate::ui::render(frame, &app))
                .unwrap();
            let text: String = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|cell| cell.symbol())
                .collect();
            assert!(text.contains(if failed {
                "Failed to copy: unavailable"
            } else {
                "Copied to clipboard"
            }));
            assert_eq!(app.mode, Mode::Normal);
            assert!(app.error_message.is_none());
        }
    }

    #[test]
    fn clipboard_inventory_actions_and_print_use_the_same_request_state() {
        let mut app = App::new_without_client("test".into(), None);
        app.service_selected = true;
        app.current_service = Service::VpcVpcs;
        app.mode = Mode::Normal;
        app.vpc_state.vpcs.items = vec![rusticity_core::vpc::Vpc {
            vpc_id: "vpc-test".into(),
            arn: "arn:vpc:test".into(),
            ..Default::default()
        }];
        let buffer = Buffer::empty(Rect::new(0, 0, 1, 1));
        app.handle_action(Action::Yank);
        assert!(!app.clipboard.snapshot_requested());
        app.clipboard.complete_with(&buffer, |text| {
            assert_eq!(text, "arn:vpc:test");
            Ok(())
        });
        app.handle_action(Action::CopyToClipboard);
        assert!(app.clipboard.snapshot_requested());
        app.clipboard.complete_with(&buffer, |text| {
            assert_eq!(text, "");
            Ok(())
        });
        assert!(!app.clipboard.snapshot_requested());
    }
}
