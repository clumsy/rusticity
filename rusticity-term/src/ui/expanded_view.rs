use super::labeled_field;

/// Format multiple fields as lines
pub fn format_fields(fields: &[(&str, &str)]) -> Vec<ratatui::text::Line<'static>> {
    fields
        .iter()
        .map(|(label, value)| labeled_field(&format!("{}: ", label), *value))
        .collect()
}

/// Format key-value pairs for expansion (used in table rows)
pub fn format_expansion_text(fields: &[(&str, String)]) -> Vec<(String, ratatui::prelude::Style)> {
    fields
        .iter()
        .map(|(label, value)| {
            let display_value = if value.is_empty() { "-" } else { value };
            (
                format!("{}: {}", label, display_value),
                ratatui::prelude::Style::default(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_labeled_field() {
        assert_eq!(labeled_field("Name", "test-value").spans.len(), 2);
    }

    #[test]
    fn test_labeled_field_empty() {
        let text: String = labeled_field("Name", "")
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(text.contains("-"));
    }

    #[test]
    fn test_format_fields() {
        assert_eq!(
            format_fields(&[("Name", "test"), ("Status", "active")]).len(),
            2
        );
    }

    #[test]
    fn test_format_expansion_text() {
        let lines = format_expansion_text(&[
            ("Name", "test".to_string()),
            ("Status", "active".to_string()),
        ]);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].0, "Name: test");
        assert_eq!(lines[1].0, "Status: active");
    }

    #[test]
    fn test_format_expansion_text_empty() {
        let lines = format_expansion_text(&[("Name", String::new())]);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].0, "Name: -");
    }
}
