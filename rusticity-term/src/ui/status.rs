use crate::app::{App, Service, ViewMode};
use crate::keymap::Mode;
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
    spans.extend(hint("^p", "preferences"));
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
    spans.extend(hint("^p", "preferences"));
    spans.extend(hint("^r", "refresh"));
    spans.extend(hint("^w", "close"));
    spans.extend(last_hint("q", "quit"));
    spans
}

pub fn render_bottom_bar(frame: &mut Frame, app: &App, area: Rect) {
    let mode_indicator = match app.mode {
        Mode::FilterInput | Mode::EventFilterInput | Mode::InsightsInput => " INSERT ",
        Mode::ServicePicker => " INSERT ",
        _ => " NORMAL ",
    };

    let mode_style = match app.mode {
        Mode::FilterInput | Mode::EventFilterInput | Mode::InsightsInput => {
            Style::default().bg(Color::Yellow).fg(Color::Black)
        }
        Mode::ServicePicker => Style::default().bg(Color::Yellow).fg(Color::Black),
        _ => Style::default().bg(Color::Blue).fg(Color::White),
    };

    let help = if app.mode == Mode::ColumnSelector {
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
        hints.extend(hint("p", "columns"));
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
        hints.extend(hint("p", "columns"));
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
            // In stack detail view
            common_detail_hotkeys()
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
        hints.extend(hint("^p", "preferences"));
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
        hints.extend(hint("p", "columns"));
        hints.extend(hint("^w", "close"));
        hints.extend(last_hint("q", "quit"));
        hints
    };

    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let spinner_frame = if app.log_groups_state.loading {
        SPINNER_FRAMES[(millis / 100 % SPINNER_FRAMES.len() as u128) as usize]
    } else {
        " "
    };

    let status_line_temp = if app.log_groups_state.loading {
        let max_width = area.width.saturating_sub(10) as usize;
        let msg = if app.log_groups_state.loading_message.len() > max_width {
            format!(
                "{}...",
                &app.log_groups_state.loading_message[..max_width.saturating_sub(3)]
            )
        } else {
            app.log_groups_state.loading_message.clone()
        };
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

    if let Some(line) = status_line_temp {
        let status_widget = Paragraph::new(line)
            .alignment(Alignment::Left)
            .style(Style::default().bg(Color::DarkGray).fg(Color::White));
        frame.render_widget(mode_widget, chunks[0]);
        frame.render_widget(spinner_widget, chunks[1]);
        frame.render_widget(status_widget, chunks[2]);
    } else {
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
        let version_line = Line::from(version_spans);
        let version_width = version_text.len() as u16;

        // Split status area into help (left) and version (right)
        let status_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(version_width)])
            .split(chunks[2]);

        let help_widget = Paragraph::new(Line::from(help))
            .block(Block::default())
            .alignment(Alignment::Left)
            .style(Style::default().bg(Color::DarkGray).fg(Color::White));

        let version_widget = Paragraph::new(version_line)
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
        let help_text = " ↑↓ scroll ⋮ ←→ toggle ⋮ ⏎ open ⋮ ^o console ⋮ ^p preferences ⋮ ^r refresh ⋮ ^w close ⋮ q quit ";
        let help_width: usize = help_text.len();

        let version_text = "RUSTICITY v0.1.2 (#1234567)";
        let version_width: usize = version_text.len();

        // Simulate terminal width of 200
        let status_width: usize = 200;
        let padding: usize = status_width
            .saturating_sub(help_width)
            .saturating_sub(version_width);

        // Total should equal status_width
        assert_eq!(help_width + padding + version_width, status_width);
    }
}
