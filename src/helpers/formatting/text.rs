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

pub fn fallback_initials(primary: &str, fallback: &str) -> String {
    let mut initials = primary
        .split_whitespace()
        .filter_map(|part| part.chars().find(|ch| ch.is_ascii_alphanumeric()))
        .take(2)
        .map(|ch| ch.to_ascii_uppercase())
        .collect::<String>();
    if initials.is_empty() {
        initials = fallback
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .take(2)
            .map(|ch| ch.to_ascii_uppercase())
            .collect();
    }
    if initials.is_empty() {
        "?".to_string()
    } else {
        initials
    }
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

pub fn sensitive_response_excerpt(text: &str, max_chars: usize) -> String {
    text_excerpt(&redact_sensitive_response_text(text), max_chars)
}

pub fn sensitive_response_snippet(text: &str) -> String {
    response_snippet(&redact_sensitive_response_text(text))
}

pub fn redact_sensitive_response_text(text: &str) -> String {
    redact_long_hex_tokens(&redact_bearer_phrases(&redact_sensitive_key_values(text)))
}

fn redact_sensitive_key_values(text: &str) -> String {
    const KEYS: &[&str] = &[
        "authorization",
        "api_key",
        "apikey",
        "api-key",
        "api_secret",
        "apisecret",
        "api-secret",
        "api_hash",
        "apihash",
        "api-hash",
        "private_key",
        "privatekey",
        "secret_key",
        "secretkey",
        "secret-key",
        "agent_key",
        "agentkey",
        "session_id",
        "sessionid",
        "session-id",
        "cursor",
        "password",
        "passcode",
        "phone_code_hash",
        "phonecodehash",
        "phone-code-hash",
        "phone_code",
        "phonecode",
        "phone-code",
        "code_hash",
        "codehash",
        "code-hash",
        "access_token",
        "refresh_token",
        "bearer_token",
        "token",
    ];

    let lower = text.to_ascii_lowercase();
    let mut redacted = String::with_capacity(text.len());
    let mut index = 0;

    while index < text.len() {
        let Some((key_start, key_len)) = find_next_key(&lower, index, KEYS) else {
            redacted.push_str(&text[index..]);
            break;
        };

        let key_end = key_start + key_len;
        let Some((value_start, value_end)) = sensitive_value_bounds(text, key_start, key_end)
        else {
            redacted.push_str(&text[index..key_end]);
            index = key_end;
            continue;
        };

        redacted.push_str(&text[index..value_start]);
        if text.as_bytes().get(value_start) == Some(&b'"') {
            redacted.push('"');
            redacted.push_str("<redacted>");
            if text.as_bytes().get(value_end.saturating_sub(1)) == Some(&b'"') {
                redacted.push('"');
            }
        } else {
            redacted.push_str("<redacted>");
        }
        index = value_end;
    }

    redacted
}

fn find_next_key(lower: &str, start: usize, keys: &[&str]) -> Option<(usize, usize)> {
    keys.iter()
        .filter_map(|key| {
            lower[start..]
                .find(key)
                .map(|offset| (start + offset, key.len()))
        })
        .min_by_key(|(index, _)| *index)
}

fn sensitive_value_bounds(text: &str, key_start: usize, key_end: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut cursor = key_end;
    if bytes.get(cursor) == Some(&b'"') || bytes.get(cursor) == Some(&b'\'') {
        cursor += 1;
    }
    while bytes
        .get(cursor)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        cursor += 1;
    }
    if !matches!(bytes.get(cursor), Some(b':') | Some(b'=')) {
        return None;
    }
    cursor += 1;
    while bytes
        .get(cursor)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        cursor += 1;
    }

    let is_authorization = text[key_start..key_end].eq_ignore_ascii_case("authorization");
    if is_authorization && text[cursor..].to_ascii_lowercase().starts_with("bearer ") {
        cursor += "bearer ".len();
    }

    let value_start = cursor;
    if matches!(bytes.get(cursor), Some(b'"') | Some(b'\'')) {
        let quote = bytes[cursor];
        cursor += 1;
        while cursor < bytes.len() {
            if bytes[cursor] == quote && bytes.get(cursor.saturating_sub(1)) != Some(&b'\\') {
                return Some((value_start, cursor + 1));
            }
            cursor += 1;
        }
        return Some((value_start, cursor));
    }

    while bytes.get(cursor).is_some_and(|byte| {
        !byte.is_ascii_whitespace()
            && !matches!(byte, b',' | b'&' | b';' | b'}' | b']' | b'"' | b'\'')
    }) {
        cursor += 1;
    }
    Some((value_start, cursor))
}

fn redact_bearer_phrases(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let mut redacted = String::with_capacity(text.len());
    let mut index = 0;

    while let Some(offset) = lower[index..].find("bearer ") {
        let start = index + offset;
        let value_start = start + "bearer ".len();
        let value_end = text.as_bytes()[value_start..]
            .iter()
            .position(|byte| {
                byte.is_ascii_whitespace() || matches!(byte, b',' | b'&' | b';' | b'"' | b'\'')
            })
            .map(|offset| value_start + offset)
            .unwrap_or(text.len());
        redacted.push_str(&text[index..value_start]);
        redacted.push_str("<redacted>");
        index = value_end;
    }

    redacted.push_str(&text[index..]);
    redacted
}

fn redact_long_hex_tokens(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut redacted = String::with_capacity(text.len());
    let mut index = 0;

    while index < bytes.len() {
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_hexdigit() {
            index += 1;
        }
        if index.saturating_sub(start) >= 40 {
            redacted.push_str("<redacted-hex>");
        } else {
            redacted.push_str(&text[start..index]);
        }
        if index < bytes.len() {
            redacted.push(bytes[index] as char);
            index += 1;
        }
    }

    redacted
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
    fn sensitive_response_snippet_redacts_common_secret_shapes() {
        let text = concat!(
            "Authorization: Bearer header-token\n",
            r#"{"api_key":"json-key","apiKey":"camel-key","token":"json-token"}"#,
            " url?access_token=query-token&ok=true ",
            "0x0123456789abcdef0123456789abcdef01234567"
        );
        let rendered = sensitive_response_snippet(text);

        assert!(rendered.contains("<redacted>"));
        assert!(rendered.contains("<redacted-hex>"));
        for secret in [
            "header-token",
            "json-key",
            "camel-key",
            "json-token",
            "query-token",
            "0123456789abcdef0123456789abcdef01234567",
        ] {
            assert!(!rendered.contains(secret), "snippet leaked {secret}");
        }
    }

    #[test]
    fn sensitive_response_excerpt_redacts_secret_key_aliases() {
        let text = concat!(
            r#"{"agent_key":"agent-secret","privateKey":"private-secret","#,
            r#""secret-key":"signing-secret","api_secret":"api-secret","#,
            r#""api_hash":"telegram-api-hash","password":"password-secret","#,
            r#""phone_code":"telegram-code","phone_code_hash":"telegram-hash","#,
            r#""sessionId":"session-secret","cursor":"cursor-secret"}"#
        );
        let rendered = sensitive_response_excerpt(text, 512);

        for secret in [
            "agent-secret",
            "private-secret",
            "signing-secret",
            "api-secret",
            "telegram-api-hash",
            "password-secret",
            "telegram-code",
            "telegram-hash",
            "session-secret",
            "cursor-secret",
        ] {
            assert!(!rendered.contains(secret), "excerpt leaked {secret}");
        }
        assert_eq!(rendered.matches("<redacted>").count(), 10);
    }

    #[test]
    fn sensitive_response_excerpt_redacts_authorization_without_hiding_later_keys() {
        let text = "Authorization: Bearer x-secret-token api_key=\"abc123\" trace=0123456789abcdef0123456789abcdef01234567";
        let rendered = sensitive_response_excerpt(text, 240);

        assert!(rendered.contains("<redacted>"));
        assert!(rendered.contains("<redacted-hex>"));
        for secret in [
            "x-secret-token",
            "abc123",
            "0123456789abcdef0123456789abcdef01234567",
        ] {
            assert!(!rendered.contains(secret), "excerpt leaked {secret}");
        }
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
