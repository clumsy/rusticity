use ratatui::style::{Color, Modifier, Style};

use super::red_text;

// Foreground colors
pub fn red() -> Style {
    red_text()
}

pub fn green() -> Style {
    Style::default().fg(Color::Green)
}

pub fn yellow() -> Style {
    Style::default().fg(Color::Yellow)
}

pub fn dark_gray() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub fn gray() -> Style {
    Style::default().fg(Color::Gray)
}

pub fn white() -> Style {
    Style::default().fg(Color::White)
}

pub fn cyan() -> Style {
    Style::default().fg(Color::Cyan)
}

pub fn blue() -> Style {
    Style::default().fg(Color::Blue)
}

pub fn magenta() -> Style {
    Style::default().fg(Color::Magenta)
}

// Background colors
pub fn bg_dark_gray() -> Style {
    Style::default().bg(Color::DarkGray)
}

pub fn bg_black() -> Style {
    Style::default().bg(Color::Black)
}

pub fn bg_white() -> Style {
    Style::default().bg(Color::White)
}

// Modifiers
pub fn bold() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

pub fn italic() -> Style {
    Style::default().add_modifier(Modifier::ITALIC)
}

// Common combinations
pub fn cursor() -> Style {
    yellow()
}

pub fn placeholder() -> Style {
    dark_gray()
}

pub fn error() -> Style {
    red()
}

pub fn success() -> Style {
    green()
}

pub fn active_border() -> Style {
    green()
}

pub fn highlight() -> Style {
    bg_dark_gray()
}

pub fn label() -> Style {
    bold()
}
