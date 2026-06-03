const RESPONSE_SNIPPET_CHARS: usize = 200;

pub fn text_excerpt(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

pub fn ellipsized_text(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let preview: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_none() {
        return preview;
    }

    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }

    let prefix: String = preview.chars().take(max_chars - 3).collect();
    format!("{prefix}...")
}

pub fn response_excerpt(text: &str) -> String {
    text_excerpt(text, RESPONSE_SNIPPET_CHARS)
}

pub fn response_snippet(text: &str) -> String {
    let mut chars = text.chars();
    let mut snippet: String = chars.by_ref().take(RESPONSE_SNIPPET_CHARS).collect();
    if chars.next().is_some() {
        snippet.push_str("...");
    }
    snippet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_snippet_marks_truncated_text() {
        let text = "a".repeat(RESPONSE_SNIPPET_CHARS + 1);

        assert_eq!(
            response_snippet(&text),
            format!("{}...", "a".repeat(RESPONSE_SNIPPET_CHARS))
        );
    }

    #[test]
    fn response_excerpt_preserves_existing_plain_truncation() {
        let text = "b".repeat(RESPONSE_SNIPPET_CHARS + 1);

        assert_eq!(response_excerpt(&text), "b".repeat(RESPONSE_SNIPPET_CHARS));
    }

    #[test]
    fn ellipsized_text_keeps_suffix_inside_max_length() {
        let text = "c".repeat(141);

        assert_eq!(
            ellipsized_text(&text, 140),
            format!("{}...", "c".repeat(137))
        );
    }

    #[test]
    fn ellipsized_text_leaves_short_text_unchanged() {
        assert_eq!(ellipsized_text("short", 140), "short");
    }

    #[test]
    fn snippets_truncate_by_char_boundary() {
        let text = format!("{}z", "é".repeat(RESPONSE_SNIPPET_CHARS));

        assert_eq!(response_excerpt(&text), "é".repeat(RESPONSE_SNIPPET_CHARS));
        assert_eq!(
            response_snippet(&text),
            format!("{}...", "é".repeat(RESPONSE_SNIPPET_CHARS))
        );
    }
}
