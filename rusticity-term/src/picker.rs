//! The one popup-picker contract: list focus first, `i` (or any typed
//! character) moves focus to the filter, arrows and Enter belong to the list
//! until then, Esc leaves the filter before it closes the picker, and the
//! filter keeps rows whose text holds every typed word in any case. Service,
//! tab, Region, profile and session pickers all route through these helpers so
//! their key handling and filtering cannot drift apart again.
use ratatui::style::{Color, Style};
use ratatui::text::Span;

/// A character typed while the picker is open.
pub(crate) fn typed(filter: &mut String, active: &mut bool, selected: &mut usize, c: char) {
    if *active || c != 'i' {
        filter.push(c);
        *selected = 0;
    }
    *active = true;
}

pub(crate) fn backspace(filter: &mut String, active: &mut bool, selected: &mut usize) {
    if *active {
        filter.pop();
        *selected = 0;
    }
}

/// `i` on the list: move focus to the filter without typing.
pub(crate) fn focus(active: &mut bool) {
    *active = true;
}

/// Esc: leave the filter first. Returns true when it did; false means the
/// filter already had focus and the caller may close the picker.
pub(crate) fn exit_filter(active: &mut bool) -> bool {
    if *active {
        *active = false;
        true
    } else {
        false
    }
}

/// Arrows and Enter act on the list only while the filter lacks focus.
pub(crate) fn can_navigate(active: bool) -> bool {
    !active
}

/// Word filter: every whitespace-separated word must appear, in any case.
pub(crate) fn matches(text: &str, filter: &str) -> bool {
    let haystack = text.to_lowercase();
    filter
        .to_lowercase()
        .split_whitespace()
        .all(|word| haystack.contains(word))
}

/// Filter line contents: typed text with a cursor block when focused, else the
/// placeholder (if any) in gray.
pub(crate) fn filter_line<'a>(
    filter: &'a str,
    active: bool,
    placeholder: Option<&'a str>,
) -> Vec<Span<'a>> {
    if active {
        vec![
            Span::raw(filter),
            Span::styled("█", Style::default().fg(Color::Green)),
        ]
    } else if filter.is_empty() {
        match placeholder {
            Some(text) => vec![Span::styled(text, Style::default().fg(Color::DarkGray))],
            None => Vec::new(),
        }
    } else {
        vec![Span::raw(filter)]
    }
}

/// Border of the filter box: green while it holds focus.
pub(crate) fn filter_border(active: bool) -> Style {
    Style::default().fg(if active { Color::Green } else { Color::White })
}

/// Border of the list box: green while the list holds focus.
pub(crate) fn list_border(filter_active: bool) -> Style {
    Style::default().fg(if filter_active {
        Color::White
    } else {
        Color::Green
    })
}
