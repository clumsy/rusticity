//! A request state replaces data in the affected pane. It never overlays stale
//! rows or claims an empty result before a successful read.
use crate::async_read::ReadStatus;
use ratatui::{
    prelude::*,
    widgets::{Paragraph, Wrap},
};

pub(crate) fn marker(status: ReadStatus) -> Option<Span<'static>> {
    match status {
        ReadStatus::Idle => None,
        ReadStatus::Loading { started } => {
            let frames = super::status::SPINNER_FRAMES;
            let index = (started.elapsed().as_millis() / 100) as usize % frames.len();
            Some(Span::raw(frames[index]))
        }
        ReadStatus::Ready => Some(Span::raw("✓")),
        ReadStatus::Failed => Some(Span::raw("!")),
    }
}

/// Returns true if this state consumed the pane; false permits rendering fresh
/// data (including a successfully empty collection). Resource identity and
/// navigation controls may be rendered outside this area.
pub(crate) fn render(
    frame: &mut Frame,
    area: Rect,
    status: ReadStatus,
    _label: &str,
    error: Option<&str>,
) -> bool {
    let (text, style) = match status {
        ReadStatus::Ready => return false,
        ReadStatus::Idle => (error.unwrap_or("Not loaded").into(), Style::default()),
        // The tab owns the spinner; the status bar owns elapsed time and hints.
        ReadStatus::Loading { .. } => ("Loading…".into(), Style::default()),
        ReadStatus::Failed => {
            let error: String = error
                .unwrap_or("The read did not complete successfully")
                .chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect();
            (
                format!("Read failed: {error}\nCtrl+R: retry  Esc: back"),
                Style::default().fg(Color::Red),
            )
        }
    };
    frame.render_widget(
        Paragraph::new(text).style(style).wrap(Wrap { trim: false }),
        area,
    );
    true
}

/// Keep the normal table border and column headers while replacing only its
/// rows. Returns true unless fresh rows (or a successful empty result) may show.
pub(crate) fn render_table<T>(
    frame: &mut Frame,
    mut config: super::table::TableConfig<T>,
    status: ReadStatus,
    label: &str,
    error: Option<&str>,
) -> bool {
    let area = config.area;
    if status != ReadStatus::Ready {
        config.items.clear();
        config.expanded_index = None;
        config.title = super::format_title(label);
    }
    super::table::render_table(frame, config);
    if area.width > 2 && area.height > 3 {
        render(
            frame,
            Rect {
                x: area.x + 1,
                y: area.y + 2,
                width: area.width - 2,
                height: area.height - 3,
            },
            status,
            label,
            error,
        );
    }
    status != ReadStatus::Ready
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use std::time::{Duration, Instant};

    #[test]
    fn indicators_distinguish_idle_loading_success_and_failure() {
        assert!(marker(ReadStatus::Idle).is_none());
        assert_eq!(marker(ReadStatus::Ready).unwrap().content, "✓");
        assert_eq!(marker(ReadStatus::Failed).unwrap().content, "!");
        let now = Instant::now();
        let first = marker(ReadStatus::Loading { started: now }).unwrap();
        let next = marker(ReadStatus::Loading {
            started: now - Duration::from_millis(110),
        })
        .unwrap();
        assert_ne!(first.content, next.content);
    }

    #[test]
    fn kms_keeps_filter_and_table_with_a_readable_tab_indicator() {
        use crate::{
            app::{App, Service, Tab},
            keymap::Mode,
            tab_runtime::TabId,
        };

        let mut app = App::new_without_client("test".into(), Some("us-east-1".into()));
        app.current_service = Service::KmsKeys;
        app.service_selected = true;
        app.mode = Mode::Normal;
        app.tabs.push(Tab {
            id: TabId::new(),
            service: Service::KmsKeys,
            title: "KMS › Managed Keys".into(),
            breadcrumb: "KMS › Managed Keys".into(),
        });
        crate::tab_runtime::sync_active(&mut app);
        crate::kms::inventory::request(&mut app);

        let mut terminal = Terminal::new(TestBackend::new(120, 12)).unwrap();
        terminal.draw(|f| crate::ui::render(f, &app)).unwrap();
        let buffer = terminal.backend().buffer();
        let tab_line: String = buffer.content()[120..240]
            .iter()
            .map(|c| c.symbol())
            .collect();
        let prefix = "KMS › Managed Keys ";
        let glyph = tab_line.strip_prefix(prefix).unwrap().trim_end();
        assert!(super::super::SPINNER_FRAMES.contains(&glyph));
        let indicator = &buffer[(prefix.chars().count() as u16, 1)];
        assert_eq!(indicator.fg, buffer[(0, 1)].fg);
        assert_eq!(indicator.bg, buffer[(0, 1)].bg);
        assert_ne!(indicator.fg, indicator.bg);
        let text: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert_eq!(
            buffer
                .content()
                .iter()
                .filter(|c| super::super::SPINNER_FRAMES.contains(&c.symbol()))
                .count(),
            1,
            "The spinner must not be duplicated in the content or footer"
        );
        assert!(text.contains("AWS managed keys") && text.contains("Customer managed keys"));
        let footer: String = buffer.content()[1320..1440]
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(footer.contains("Loading KMS keys…") && footer.contains("s ⋮ Esc: cancel / back"));
        let content: String = buffer.content()[360..1320]
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(content.contains("Search by alias, key ID, or description"));
        assert!(content.contains("Alias") && content.contains("Key ID"));
        assert!(content.contains("Loading…"));
        assert!(!content.contains("Esc:"), "Hints stay in the status bar");
        assert!(!content.contains("AWS managed keys (0)"));
        assert_eq!(
            content.matches('╭').count(),
            2,
            "Only the filter and table borders"
        );

        // Completion uses the same cell and readable tab text color.
        for (success, glyph) in [(true, "✓"), (false, "!")] {
            app.kms_state.inventory = crate::async_read::AsyncRead::resolved(
                crate::async_read::ReadScope::from_app(&app, "kms-inventory"),
                success,
            );
            terminal.draw(|f| crate::ui::render(f, &app)).unwrap();
            let cell = &terminal.backend().buffer()[(prefix.chars().count() as u16, 1)];
            assert_eq!(cell.symbol(), glyph);
            assert_eq!(cell.fg, terminal.backend().buffer()[(0, 1)].fg);
            assert_eq!(cell.bg, terminal.backend().buffer()[(0, 1)].bg);
            assert_ne!(cell.fg, cell.bg);
        }
    }

    #[test]
    fn table_status_keeps_headers_and_never_renders_old_rows() {
        let topic =
            rusticity_core::SnsTopic::from_arn("arn:aws:sns:us-east-1:123456789012:old-row")
                .unwrap();
        let columns = vec![crate::sns::columns::Column::Name.into_table_column()];
        for (width, height) in [(120, 8), (2, 2), (1, 1)] {
            for status in [
                ReadStatus::Loading {
                    started: Instant::now(),
                },
                ReadStatus::Failed,
                ReadStatus::Idle,
            ] {
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                terminal
                    .draw(|f| {
                        assert!(render_table(
                            f,
                            super::super::table::TableConfig {
                                items: vec![&topic],
                                selected_index: 0,
                                expanded_index: Some(0),
                                columns: &columns,
                                sort_column: "Name",
                                sort_direction: crate::common::SortDirection::Asc,
                                title: "Topics (1)".into(),
                                area: f.area(),
                                get_expanded_content: None,
                                is_active: true,
                            },
                            status,
                            "Topics",
                            Some("AccessDenied"),
                        ));
                    })
                    .unwrap();
                let text: String = terminal
                    .backend()
                    .buffer()
                    .content()
                    .iter()
                    .map(|c| c.symbol())
                    .collect();
                assert!(!text.contains("old-row") && !text.contains("Topics (1)"));
                if width == 120 {
                    assert!(text.contains("Topics") && text.contains("Name"));
                    assert_eq!(text.matches('╭').count(), 1);
                    assert!(
                        text.contains(if matches!(status, ReadStatus::Loading { .. }) {
                            "Loading…"
                        } else {
                            "AccessDenied"
                        })
                    );
                }
            }
        }
    }

    #[test]
    fn loading_and_error_consume_the_pane_but_ready_allows_fresh_rows() {
        for width in [1, 30, 100] {
            let mut terminal = Terminal::new(TestBackend::new(width, 8)).unwrap();
            terminal
                .draw(|f| {
                    assert!(render(
                        f,
                        f.area(),
                        ReadStatus::Loading {
                            started: Instant::now() - Duration::from_secs(3)
                        },
                        "executions",
                        None
                    ));
                })
                .unwrap();
            if width == 100 {
                let text: String = terminal
                    .backend()
                    .buffer()
                    .content()
                    .iter()
                    .map(|c| c.symbol())
                    .collect();
                assert_eq!(text.trim(), "Loading…");
                assert!(!text.contains("No executions"));
                assert!(!super::super::SPINNER_FRAMES
                    .iter()
                    .any(|glyph| text.contains(glyph)));
                assert!(!text.chars().any(|c| ('\u{2500}'..='\u{257f}').contains(&c)));
            }
            terminal
                .draw(|f| {
                    assert!(render(
                        f,
                        f.area(),
                        ReadStatus::Failed,
                        "executions",
                        Some("AccessDenied\x1b")
                    ));
                })
                .unwrap();
            if width == 100 {
                let text: String = terminal
                    .backend()
                    .buffer()
                    .content()
                    .iter()
                    .map(|c| c.symbol())
                    .collect();
                assert!(text.contains("AccessDenied") && text.contains("Ctrl+R"));
                assert!(!text.contains('\x1b'));
            }
            terminal
                .draw(|f| assert!(!render(f, f.area(), ReadStatus::Ready, "executions", None)))
                .unwrap();
        }
    }
}
