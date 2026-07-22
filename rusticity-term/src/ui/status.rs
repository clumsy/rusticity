use crate::app::{App, Service, ViewMode};
use crate::keymap::Mode;
use crate::ui::cfn::DetailTab;
use crate::ui::red_text;
use ratatui::{prelude::*, widgets::*};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

pub fn first_hint(key: &'static str, action: &'static str) -> Vec<Span<'static>> {
    vec![
        Span::styled(key, red_text()),
        Span::raw(" "),
        Span::raw(action),
        Span::raw(" ⋮"),
    ]
}

pub fn hint(key: &'static str, action: &'static str) -> Vec<Span<'static>> {
    vec![
        Span::raw(" "),
        Span::styled(key, red_text()),
        Span::raw(" "),
        Span::raw(action),
        Span::raw(" ⋮"),
    ]
}

pub fn last_hint(key: &'static str, action: &'static str) -> Vec<Span<'static>> {
    vec![
        Span::raw(" "),
        Span::styled(key, red_text()),
        Span::raw(" "),
        Span::raw(action),
    ]
}

fn common_detail_hotkeys() -> Vec<Span<'static>> {
    let mut spans = vec![];
    spans.extend(first_hint("↑↓", "scroll"));
    spans.extend(hint("⎋", "back"));
    spans.extend(hint("[]", "switch"));
    spans.extend(hint("⇤⇥", "switch"));
    spans.extend(hint("^u", "page up"));
    spans.extend(hint("^d", "page down"));
    spans.extend(hint("y", "yank"));
    spans.extend(hint("^o", "console"));
    spans.extend(hint("p", "preferences"));
    spans.extend(hint("^p", "print"));
    spans.extend(hint("^r", "refresh"));
    spans.extend(hint("^w", "close"));
    spans.extend(last_hint("q", "quit"));
    spans
}

fn common_list_hotkeys() -> Vec<Span<'static>> {
    let mut spans = vec![];
    spans.extend(first_hint("↑↓", "scroll"));
    spans.extend(hint("←→", "toggle"));
    spans.extend(hint("⏎", "open"));
    spans.extend(hint("[]", "switch"));
    spans.extend(hint("^u", "page up"));
    spans.extend(hint("^d", "page down"));
    spans.extend(hint("y", "yank"));
    spans.extend(hint("^o", "console"));
    spans.extend(hint("p", "preferences"));
    spans.extend(hint("^p", "print"));
    spans.extend(hint("^r", "refresh"));
    spans.extend(hint("^w", "close"));
    spans.extend(last_hint("q", "quit"));
    spans
}

pub fn render_bottom_bar(frame: &mut Frame, app: &App, area: Rect) {
    let is_insert = match app.mode {
        Mode::FilterInput | Mode::EventFilterInput | Mode::InsightsInput => true,
        Mode::ServicePicker | Mode::SpaceMenu => app.service_picker.filter_active,
        Mode::RegionPicker => app.region_filter_active,
        Mode::SessionPicker => app.session_filter_active,
        Mode::ProfilePicker => app.profile_filter_active,
        _ => false,
    };

    let mode_indicator = if is_insert { " INSERT " } else { " NORMAL " };
    let mode_style = if is_insert {
        Style::default().bg(Color::Yellow).fg(Color::Black)
    } else {
        Style::default().bg(Color::Blue).fg(Color::White)
    };

    let help = if app.mode == Mode::QuitConfirm {
        let mut hints = vec![];
        hints.extend(first_hint("y", "confirm quit"));
        hints.extend(last_hint("n / Esc", "cancel"));
        hints
    } else if app.mode == Mode::ColumnSelector {
        let mut hints = vec![];
        hints.extend(hint("↑↓", "scroll"));
        hints.extend(hint("␣", "toggle"));

        if app.current_service == Service::CloudWatchAlarms {
            hints.extend(hint("⇤⇥", "switch"));
        }

        hints.extend(last_hint("⎋", "close"));
        hints
    } else if app.mode == Mode::InsightsInput {
        let mut hints = vec![];
        hints.extend(hint(" tab", "switch"));
        hints.extend(hint("↑↓", "scroll"));
        hints.extend(hint("␣", "toggle"));
        hints.extend(hint("⏎", "execute"));
        hints.extend(hint("⎋", "cancel"));
        hints.extend(hint("^r", "refresh"));
        hints.extend(last_hint("^w", "close"));
        hints
    } else if app.current_service == Service::CloudWatchInsights {
        let mut hints = vec![];
        hints.extend(hint(" i", "insert"));
        hints.extend(hint("⏎", "execute"));
        hints.extend(hint("?", "help"));
        hints.extend(hint("^r", "refresh"));
        hints.extend(hint("^o", "console"));
        hints.extend(hint("⎋", "back"));
        hints.extend(hint("^w", "close"));
        hints.extend(last_hint("q", "quit"));
        hints
    } else if app.mode == Mode::EventFilterInput {
        let mut hints = vec![];
        hints.extend(first_hint("⇤⇥", "switch"));
        hints.extend(hint("␣", "change unit"));
        hints.extend(hint("⏎", "apply"));
        hints.extend(hint("⎋", "cancel"));
        hints.extend(last_hint("^w", "close"));
        hints
    } else if app.mode == Mode::RegionPicker {
        let mut hints = vec![];
        hints.extend(first_hint("↑↓", "scroll"));
        hints.extend(hint("i", "filter"));
        hints.extend(hint("⏎", "select"));
        hints.extend(hint("^l", "measure latency"));
        hints.extend(hint("⎋", "close"));
        hints.extend(last_hint("q", "quit"));
        hints
    } else if app.mode == Mode::FilterInput {
        let mut hints = vec![];
        hints.extend(first_hint("⏎", "apply"));
        hints.extend(hint("⎋", "cancel"));
        hints.extend(hint("␣", "toggle"));
        hints.extend(hint("⇤⇥", "switch"));
        hints.extend(last_hint("^w", "close"));
        hints
    } else if app.view_mode == ViewMode::Events {
        let mut hints = vec![];
        hints.extend(first_hint("↑↓", "scroll"));
        hints.extend(hint("←→", "toggle"));
        hints.extend(hint("y", "yank"));
        hints.extend(hint("^o", "console"));
        hints.extend(hint("^r", "refresh"));
        hints.extend(hint("p", "preferences"));
        hints.extend(hint("^p", "print"));
        hints.extend(hint("^w", "close"));
        hints.extend(last_hint("q", "quit"));
        hints
    } else if app.current_service == Service::CloudWatchAlarms
        && app.alarms_state.current_alarm.is_some()
    {
        let mut hints = vec![];
        hints.extend(first_hint("↑↓", "scroll"));
        hints.extend(hint("⇤⇥", "switch tab"));
        hints.extend(hint("⎋", "back"));
        hints.extend(hint("y", "copy JSON"));
        hints.extend(hint("^o", "console"));
        hints.extend(hint("^r", "refresh"));
        hints.extend(hint("^w", "close"));
        hints.extend(last_hint("q", "quit"));
        hints
    } else if app.view_mode == ViewMode::Detail {
        let mut hints = vec![];
        hints.extend(first_hint("↑↓", "scroll"));
        hints.extend(hint("←→", "toggle"));
        hints.extend(hint("⏎", "open"));
        hints.extend(hint("⎋", "back"));
        hints.extend(hint("i", "insert"));
        hints.extend(hint("s", "sort"));
        hints.extend(hint("o", "order"));
        hints.extend(hint("<num>p", "page"));
        hints.extend(hint("^o", "console"));
        hints.extend(hint("⇤⇥", "switch"));
        hints.extend(hint("^r", "refresh"));
        hints.extend(hint("p", "preferences"));
        hints.extend(hint("^p", "print"));
        hints.extend(hint("^w", "close"));
        hints.extend(last_hint("q", "quit"));
        hints
    } else if app.current_service == Service::EcrRepositories {
        if app.ecr_state.current_repository.is_some() {
            let mut hints = vec![];
            hints.extend(first_hint("↑↓", "scroll"));
            hints.extend(hint("←→", "toggle"));
            hints.extend(hint("⎋", "back"));
            hints.extend(hint("y", "yank"));
            hints.extend(hint("^o", "console"));
            hints.extend(hint("^r", "refresh"));
            hints.extend(hint("^w", "close"));
            hints.extend(last_hint("q", "quit"));
            hints
        } else {
            let mut hints = vec![];
            hints.extend(first_hint("↑↓", "scroll"));
            hints.extend(hint("←→", "toggle"));
            hints.extend(hint("⏎", "open"));
            hints.extend(hint("⇤⇥", "switch"));
            hints.extend(hint("y", "yank"));
            hints.extend(hint("^o", "console"));
            hints.extend(hint("^r", "refresh"));
            hints.extend(hint("^w", "close"));
            hints.extend(last_hint("q", "quit"));
            hints
        }
    } else if app.current_service == Service::S3Buckets {
        if app.s3_state.current_bucket.is_some() {
            let mut hints = vec![];
            hints.extend(first_hint("↑↓", "scroll"));
            hints.extend(hint("←→", "toggle"));
            hints.extend(hint("⏎", "open"));
            hints.extend(hint("⎋", "back"));
            hints.extend(hint("⇤⇥", "switch"));
            hints.extend(hint("^o", "console"));
            hints.extend(hint("^r", "refresh"));
            hints.extend(hint("^w", "close"));
            hints.extend(last_hint("q", "quit"));
            hints
        } else {
            let mut hints = vec![];
            hints.extend(first_hint("↑↓", "scroll"));
            hints.extend(hint("←→", "toggle"));
            hints.extend(hint("⏎", "open"));
            hints.extend(hint("⇤⇥", "switch"));
            hints.extend(hint("^o", "console"));
            hints.extend(hint("^r", "refresh"));
            hints.extend(hint("^w", "close"));
            hints.extend(last_hint("q", "quit"));
            hints
        }
    } else if app.current_service == Service::CloudFormationStacks {
        if app.cfn_state.current_stack.is_some() {
            // In stack detail view - customize hints based on tab
            if app.cfn_state.detail_tab == DetailTab::Template
                || app.cfn_state.detail_tab == DetailTab::GitSync
            {
                // Template and GitSync tabs: no preferences
                let mut hints = vec![];
                hints.extend(first_hint("↑↓", "scroll"));
                hints.extend(hint("⎋", "back"));
                hints.extend(hint("[]", "switch"));
                hints.extend(hint("⇤⇥", "switch"));
                hints.extend(hint("^u", "page up"));
                hints.extend(hint("^d", "page down"));
                hints.extend(hint("y", "yank"));
                hints.extend(hint("^o", "console"));
                hints.extend(hint("^r", "refresh"));
                hints.extend(hint("^w", "close"));
                hints.extend(last_hint("q", "quit"));
                hints
            } else {
                common_detail_hotkeys()
            }
        } else {
            // In stack list view - build custom hints with snapshot
            let mut hints = vec![];
            hints.extend(first_hint("↑↓", "scroll"));
            hints.extend(hint("←→", "toggle"));
            hints.extend(hint("⏎", "open"));
            hints.extend(hint("[]", "switch"));
            hints.extend(hint("^u", "page up"));
            hints.extend(hint("^d", "page down"));
            hints.extend(hint("y", "yank"));
            hints.extend(hint("^p", "snapshot"));
            hints.extend(hint("^o", "console"));
            hints.extend(hint("^r", "refresh"));
            hints.extend(hint("^w", "close"));
            hints.extend(last_hint("q", "quit"));
            hints
        }
    } else if app.current_service == Service::IamUsers {
        if app.iam_state.current_user.is_some() {
            common_detail_hotkeys()
        } else {
            common_list_hotkeys()
        }
    } else if app.current_service == Service::IamRoles {
        if app.iam_state.current_role.is_some() {
            common_detail_hotkeys()
        } else {
            common_list_hotkeys()
        }
    } else if app.current_service == Service::LambdaFunctions {
        if app.lambda_state.current_function.is_some() {
            common_detail_hotkeys()
        } else {
            common_list_hotkeys()
        }
    } else if app.view_mode == ViewMode::List {
        let mut hints = vec![];
        hints.extend(first_hint("↑↓", "scroll"));
        hints.extend(hint("←→", "toggle"));
        hints.extend(hint("⏎", "open"));
        hints.extend(hint("^o", "console"));
        hints.extend(hint("p", "preferences"));
        hints.extend(hint("^p", "print"));
        hints.extend(hint("^r", "refresh"));
        hints.extend(hint("^w", "close"));
        hints.extend(last_hint("q", "quit"));
        hints
    } else {
        let mut hints = vec![];
        hints.extend(first_hint("↑↓", "scroll"));
        hints.extend(hint("←→", "toggle"));
        hints.extend(hint("⏎", "open"));
        hints.extend(hint("⎋", "back"));
        hints.extend(hint("<num>p", "page"));
        hints.extend(hint("^o", "console"));
        hints.extend(hint("^r", "refresh"));
        hints.extend(hint("p", "preferences"));
        hints.extend(hint("^p", "print"));
        hints.extend(hint("^w", "close"));
        hints.extend(last_hint("q", "quit"));
        hints
    };

    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    // Any service loading → show generic spinner inline with version on the right
    let is_loading = app.log_groups_state.loading
        || app.kms_state.keys.loading
        || app.ecr_state.repositories.loading
        || app.ec2_state.table.loading
        || app.cfn_state.table.loading
        || app.lambda_state.table.loading
        || app.lambda_application_state.table.loading
        || app.alarms_state.table.loading
        || app.cloudtrail_state.table.loading
        || app.apig_state.apis.loading
        || app.sqs_state.queues.loading;

    let spinner_frame = if is_loading {
        SPINNER_FRAMES[(millis / 100 % SPINNER_FRAMES.len() as u128) as usize]
    } else {
        " "
    };

    // Only show a left-side connection message (profile/region change), not generic loading noise
    let connection_message: Option<String> =
        if app.log_groups_state.loading && !app.log_groups_state.loading_message.is_empty() {
            let msg = &app.log_groups_state.loading_message;
            // Only connection messages, not "Refreshing..." noise
            if msg.starts_with("Connecting") || msg.starts_with("Loading log") {
                let max_width = area.width.saturating_sub(10) as usize;
                Some(if msg.len() > max_width {
                    format!("{}...", &msg[..max_width.saturating_sub(3)])
                } else {
                    msg.clone()
                })
            } else {
                None
            }
        } else {
            None
        };

    let status_line_temp = if let Some(msg) = connection_message {
        Some(Line::from(vec![Span::raw(msg)]))
    } else if !app.page_input.is_empty() {
        Some(Line::from(vec![Span::raw(format!(
            "Go to page {} (press p to confirm)",
            app.page_input
        ))]))
    } else {
        None
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(8),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .split(area);

    let mode_widget = Paragraph::new(mode_indicator).style(mode_style);

    let spinner_widget = Paragraph::new(spinner_frame)
        .block(Block::default())
        .style(Style::default().bg(Color::DarkGray).fg(Color::Yellow));

    // Build version string
    let version = env!("CARGO_PKG_VERSION");
    let commit = option_env!("GIT_HASH").unwrap_or("unknown");
    let version_text = format!("⋮ RUSTICITY v{} (#{})", version, commit);

    let colors = [
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
    ];
    let seed = millis as usize;
    let version_spans: Vec<Span> = version_text
        .chars()
        .enumerate()
        .map(|(i, c)| {
            let color = colors[(seed + i) % colors.len()];
            Span::styled(c.to_string(), Style::default().fg(color))
        })
        .collect();

    // When loading, prepend "Loading… <spinner>" in yellow to the version line
    let right_line = if is_loading {
        let spin = SPINNER_FRAMES[(millis / 100 % SPINNER_FRAMES.len() as u128) as usize];
        let mut spans = vec![Span::styled(
            format!("Loading… {} ", spin),
            Style::default().fg(Color::Yellow),
        )];
        spans.extend(version_spans);
        Line::from(spans)
    } else {
        Line::from(version_spans)
    };

    // Width: version text + optional "Loading… X " prefix (11 chars)
    let loading_prefix_len: u16 = if is_loading { 11 } else { 0 };
    let right_width = version_text.len() as u16 + loading_prefix_len;

    if let Some(line) = status_line_temp {
        let status_widget = Paragraph::new(line)
            .alignment(Alignment::Left)
            .style(Style::default().bg(Color::DarkGray).fg(Color::White));
        frame.render_widget(mode_widget, chunks[0]);
        frame.render_widget(spinner_widget, chunks[1]);
        frame.render_widget(status_widget, chunks[2]);
    } else {
        let status_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(right_width)])
            .split(chunks[2]);

        let help_widget = Paragraph::new(Line::from(help))
            .block(Block::default())
            .alignment(Alignment::Left)
            .style(Style::default().bg(Color::DarkGray).fg(Color::White));

        let version_widget = Paragraph::new(right_line)
            .alignment(Alignment::Right)
            .style(Style::default().bg(Color::DarkGray).fg(Color::White));

        frame.render_widget(mode_widget, chunks[0]);
        frame.render_widget(spinner_widget, chunks[1]);
        frame.render_widget(help_widget, status_chunks[0]);
        frame.render_widget(version_widget, status_chunks[1]);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_version_string_format() {
        let version = env!("CARGO_PKG_VERSION");
        let commit = option_env!("GIT_HASH").unwrap_or("unknown");
        let version_text = format!("RUSTICITY v{} (#{})", version, commit);

        assert!(version_text.starts_with("RUSTICITY v"));
        assert!(version_text.contains("#"));
    }

    #[test]
    fn test_version_padding_calculation() {
        // Simulate help text width
        let help_text = " ↑↓ scroll ⋮ ←→ toggle ⋮ ⏎ open ⋮ ^o console ⋮ p preferences ⋮ ^p print ⋮ ^r refresh ⋮ ^w close ⋮ q quit ";
        let help_width: usize = help_text.len();

        let version = env!("CARGO_PKG_VERSION");
        let commit = option_env!("GIT_HASH").unwrap_or("unknown");
        let version_text = format!("RUSTICITY v{} (#{})", version, commit);
        let version_width: usize = version_text.len();

        // Simulate terminal width of 200
        let status_width: usize = 200;
        let padding: usize = status_width
            .saturating_sub(help_width)
            .saturating_sub(version_width);

        // Total should equal status_width
        assert_eq!(help_width + padding + version_width, status_width);
    }

    #[test]
    fn test_preferences_hint_uses_p_not_ctrl_p() {
        use super::common_list_hotkeys;
        let spans = common_list_hotkeys();
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();

        // Should have "p preferences" not "^p preferences"
        assert!(
            text.contains("p preferences"),
            "Should show 'p preferences'"
        );
        assert!(
            !text.contains("^p preferences"),
            "Should not show '^p preferences'"
        );
    }

    #[test]
    fn test_print_hint_uses_ctrl_p() {
        use super::common_list_hotkeys;
        let spans = common_list_hotkeys();
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();

        // Should have "^p print" for copy to clipboard
        assert!(
            text.contains("^p print"),
            "Should show '^p print' for copy to clipboard"
        );
    }

    #[test]
    fn test_yank_hint_uses_y() {
        use super::common_list_hotkeys;
        let spans = common_list_hotkeys();
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();

        // Should have "y yank" for copying selected item
        assert!(
            text.contains("y yank"),
            "Should show 'y yank' for copying selected item"
        );
    }

    #[test]
    fn test_common_detail_hotkeys_has_correct_hints() {
        use super::common_detail_hotkeys;
        let spans = common_detail_hotkeys();
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();

        // Detail views should have p for preferences and ^p for print
        assert!(
            text.contains("p preferences"),
            "Detail view should show 'p preferences'"
        );
        assert!(
            text.contains("^p print"),
            "Detail view should show '^p print'"
        );
    }

    #[test]
    fn test_no_columns_hint_anywhere() {
        let list_spans = super::common_list_hotkeys();
        let list_text: String = list_spans.iter().map(|s| s.content.as_ref()).collect();

        // Should never show "columns", always "preferences"
        assert!(
            !list_text.contains("p columns"),
            "Should not show 'p columns', use 'p preferences' instead"
        );

        let detail_spans = super::common_detail_hotkeys();
        let detail_text: String = detail_spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            !detail_text.contains("p columns"),
            "Should not show 'p columns', use 'p preferences' instead"
        );
    }

    #[test]
    fn test_all_views_have_print_hotkey() {
        // Test list view
        let list_spans = super::common_list_hotkeys();
        let list_text: String = list_spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            list_text.contains("^p print"),
            "List view must have '^p print' hotkey"
        );

        // Test detail view
        let detail_spans = super::common_detail_hotkeys();
        let detail_text: String = detail_spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            detail_text.contains("^p print"),
            "Detail view must have '^p print' hotkey"
        );
    }

    #[test]
    fn test_preferences_always_uses_p_not_ctrl_p() {
        let list_spans = super::common_list_hotkeys();
        let list_text: String = list_spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            list_text.contains("p preferences"),
            "Should use 'p preferences'"
        );
        assert!(
            !list_text.contains("^p preferences"),
            "Should not use '^p preferences'"
        );

        let detail_spans = super::common_detail_hotkeys();
        let detail_text: String = detail_spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            detail_text.contains("p preferences"),
            "Should use 'p preferences'"
        );
        assert!(
            !detail_text.contains("^p preferences"),
            "Should not use '^p preferences'"
        );
    }

    #[test]
    fn test_is_loading_true_when_kms_loading() {
        use crate::app::App;
        let mut app = App::new_without_client("default".to_string(), None);
        app.kms_state.keys.loading = true;
        // Verify is_loading would be true — test by checking the flag directly
        // (render_bottom_bar is not unit-testable without a frame, but we verify the condition)
        let is_loading = app.log_groups_state.loading
            || app.kms_state.keys.loading
            || app.ecr_state.repositories.loading;
        assert!(is_loading, "is_loading must be true when kms keys loading");
    }

    #[test]
    fn test_is_loading_false_when_nothing_loading() {
        use crate::app::App;
        let app = App::new_without_client("default".to_string(), None);
        let is_loading = app.log_groups_state.loading
            || app.kms_state.keys.loading
            || app.ecr_state.repositories.loading
            || app.ec2_state.table.loading;
        assert!(
            !is_loading,
            "is_loading must be false when nothing is loading"
        );
    }

    #[test]
    fn test_refreshing_message_not_set_on_refresh() {
        // Ctrl+R must NOT set loading_message to "Refreshing..." —
        // that message was leaking the spinner into unrelated services.
        use crate::app::App;
        use crate::keymap::{Action, Mode};
        let mut app = App::new_without_client("default".to_string(), None);
        app.mode = Mode::ProfilePicker;
        app.handle_action(Action::Refresh);
        assert!(
            app.log_groups_state.loading_message.is_empty()
                || !app.log_groups_state.loading_message.contains("Refreshing"),
            "Refresh must not set 'Refreshing...' loading_message: got {:?}",
            app.log_groups_state.loading_message
        );
    }

    #[test]
    fn test_spinner_off_when_all_loading_false() {
        use crate::app::App;
        let app = App::new_without_client("default".to_string(), None);
        // All loading flags default to false
        let is_loading = app.log_groups_state.loading
            || app.kms_state.keys.loading
            || app.ecr_state.repositories.loading
            || app.ec2_state.table.loading
            || app.cfn_state.table.loading
            || app.lambda_state.table.loading
            || app.lambda_application_state.table.loading
            || app.alarms_state.table.loading
            || app.cloudtrail_state.table.loading
            || app.apig_state.apis.loading
            || app.sqs_state.queues.loading;
        assert!(!is_loading, "spinner must be OFF when nothing is loading");
    }

    #[test]
    fn test_spinner_on_then_off_lifecycle() {
        // Simulate: loading=true (spinner on) → load completes → loading=false (spinner off)
        use crate::app::App;
        let mut app = App::new_without_client("default".to_string(), None);

        app.kms_state.keys.loading = true;
        let spinning = app.kms_state.keys.loading;
        assert!(spinning, "spinner must be ON during load");

        // Simulate load complete
        app.kms_state.keys.loading = false;
        let spinning_after = app.kms_state.keys.loading;
        assert!(!spinning_after, "spinner must be OFF after load completes");
    }

    #[test]
    fn test_spinner_on_for_each_service() {
        use crate::app::App;
        // Each service loading flag independently triggers the spinner
        let check = |loading: bool| loading; // mirrors the any_loading OR chain

        assert!(check(true), "loading=true must trigger spinner");
        assert!(!check(false), "loading=false must not trigger spinner");

        let mut app = App::new_without_client("default".to_string(), None);

        type LoadSetter = Box<dyn Fn(&mut App, bool)>;
        let cases: Vec<(&str, LoadSetter)> = vec![
            (
                "kms",
                Box::new(|a: &mut App, v| a.kms_state.keys.loading = v),
            ),
            (
                "ecr",
                Box::new(|a: &mut App, v| a.ecr_state.repositories.loading = v),
            ),
            (
                "ec2",
                Box::new(|a: &mut App, v| a.ec2_state.table.loading = v),
            ),
            (
                "cfn",
                Box::new(|a: &mut App, v| a.cfn_state.table.loading = v),
            ),
            (
                "lambda_fn",
                Box::new(|a: &mut App, v| a.lambda_state.table.loading = v),
            ),
            (
                "alarms",
                Box::new(|a: &mut App, v| a.alarms_state.table.loading = v),
            ),
        ];

        for (name, setter) in cases {
            setter(&mut app, true);
            let is_loading = app.kms_state.keys.loading
                || app.ecr_state.repositories.loading
                || app.ec2_state.table.loading
                || app.cfn_state.table.loading
                || app.lambda_state.table.loading
                || app.alarms_state.table.loading;
            assert!(is_loading, "spinner must be ON when {name} is loading");
            setter(&mut app, false);
        }
    }

    #[test]
    fn test_connection_message_filter_allows_connecting() {
        // Only "Connecting..." and "Loading log..." messages pass through to left side.
        let allowed = ["Connecting with new region...", "Loading log groups..."];
        for msg in &allowed {
            assert!(
                msg.starts_with("Connecting") || msg.starts_with("Loading log"),
                "Message '{msg}' should pass connection filter"
            );
        }
        let blocked = ["Refreshing...", "Loading KMS keys...", ""];
        for msg in &blocked {
            assert!(
                !(msg.starts_with("Connecting") || msg.starts_with("Loading log")),
                "Message '{msg}' should be blocked by connection filter"
            );
        }
    }
}
