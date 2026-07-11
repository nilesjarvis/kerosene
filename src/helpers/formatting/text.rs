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
    redact_sensitive_text(text, 40)
}

/// Redact external order-lifecycle text, including 128-bit client order IDs.
pub fn redact_sensitive_order_text(text: &str) -> String {
    redact_sensitive_text(text, 32)
}

fn redact_sensitive_text(text: &str, minimum_hex_run: usize) -> String {
    redact_hex_tokens(
        &redact_bearer_phrases(&redact_sensitive_key_values(text)),
        minimum_hex_run,
    )
}

pub fn redact_wallet_address_debug_value(value: &str) -> &str {
    let trimmed = value.trim();
    let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    else {
        return value;
    };
    if hex.len() == 40 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        "<redacted>"
    } else {
        value
    }
}

pub fn path_neutral_io_error_detail(error: &std::io::Error) -> String {
    let kind = match error.kind() {
        std::io::ErrorKind::NotFound => "not found",
        std::io::ErrorKind::PermissionDenied => "permission denied",
        std::io::ErrorKind::AlreadyExists => "already exists",
        std::io::ErrorKind::InvalidInput => "invalid input",
        std::io::ErrorKind::InvalidData => "invalid data",
        std::io::ErrorKind::Interrupted => "interrupted",
        std::io::ErrorKind::UnexpectedEof => "unexpected EOF",
        std::io::ErrorKind::WriteZero => "write failed",
        _ => "I/O error",
    };

    match error.raw_os_error() {
        Some(code) => format!("{kind} (os error {code})"),
        None => kind.to_string(),
    }
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
        "client_secret",
        "clientsecret",
        "client-secret",
        "client_id",
        "clientid",
        "client-id",
        "x_oauth_client_id",
        "xoauthclientid",
        "x-oauth-client-id",
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
        "set-cookie",
        "cookie",
        "session_token",
        "sessiontoken",
        "session-token",
        "csrf_token",
        "csrftoken",
        "csrf-token",
        "jwt",
        "signature",
        "phone_code_hash",
        "phonecodehash",
        "phone-code-hash",
        "phone_code",
        "phonecode",
        "phone-code",
        "code_hash",
        "codehash",
        "code-hash",
        "auth_token",
        "authtoken",
        "auth-token",
        "access_token",
        "accesstoken",
        "refresh_token",
        "refreshtoken",
        "id_token",
        "idtoken",
        "id-token",
        "bearer_token",
        "bearertoken",
        "token",
        "sig",
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
        if text.as_bytes().get(value_start) == Some(&b'\\')
            && text.as_bytes().get(value_start + 1) == Some(&b'"')
        {
            redacted.push_str("\\\"<redacted>");
            if text.as_bytes().get(value_end.saturating_sub(2)) == Some(&b'\\')
                && text.as_bytes().get(value_end.saturating_sub(1)) == Some(&b'"')
            {
                redacted.push_str("\\\"");
            }
        } else if text.as_bytes().get(value_start) == Some(&b'"') {
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

    if text[key_start..key_end].eq_ignore_ascii_case("authorization") {
        cursor = authorization_credential_start(bytes, cursor);
    }

    let value_start = cursor;
    if bytes.get(cursor) == Some(&b'\\')
        && matches!(bytes.get(cursor + 1), Some(b'"') | Some(b'\''))
    {
        let quote = bytes[cursor + 1];
        cursor += 2;
        while cursor + 1 < bytes.len() {
            if bytes[cursor] == b'\\' && bytes[cursor + 1] == quote {
                return Some((value_start, cursor + 2));
            }
            cursor += 1;
        }
        return Some((value_start, bytes.len()));
    }

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

    if sensitive_key_is_cookie_header(text, key_start, key_end) {
        return Some((value_start, cookie_header_value_end(bytes, cursor)));
    }

    while bytes.get(cursor).is_some_and(|byte| {
        !byte.is_ascii_whitespace()
            && !matches!(byte, b',' | b'&' | b';' | b'}' | b']' | b'"' | b'\'')
    }) {
        cursor += 1;
    }
    Some((value_start, cursor))
}

fn sensitive_key_is_cookie_header(text: &str, key_start: usize, key_end: usize) -> bool {
    let key = &text[key_start..key_end];
    key.eq_ignore_ascii_case("cookie") || key.eq_ignore_ascii_case("set-cookie")
}

fn cookie_header_value_end(bytes: &[u8], mut cursor: usize) -> usize {
    while bytes
        .get(cursor)
        .is_some_and(|byte| !matches!(byte, b'\n' | b'\r'))
    {
        cursor += 1;
    }
    cursor
}

fn authorization_credential_start(bytes: &[u8], mut cursor: usize) -> usize {
    if matches!(bytes.get(cursor), Some(b'"') | Some(b'\'')) {
        return cursor;
    }
    if bytes.get(cursor) == Some(&b'\\')
        && matches!(bytes.get(cursor + 1), Some(b'"') | Some(b'\''))
    {
        return cursor;
    }

    let scheme_start = cursor;
    while bytes
        .get(cursor)
        .is_some_and(|byte| !byte.is_ascii_whitespace() && !matches!(byte, b',' | b';'))
    {
        cursor += 1;
    }

    if cursor == scheme_start {
        return scheme_start;
    }

    let mut value_start = cursor;
    while bytes
        .get(value_start)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        value_start += 1;
    }

    if value_start == cursor {
        scheme_start
    } else {
        value_start
    }
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

fn redact_hex_tokens(text: &str, minimum_run: usize) -> String {
    let mut redacted = String::with_capacity(text.len());
    let mut run_start = None;
    let mut run_len = 0_usize;

    for (index, ch) in text.char_indices() {
        if ch.is_ascii_hexdigit() {
            if run_start.is_none() {
                run_start = Some(index);
            }
            run_len += 1;
            continue;
        }

        if let Some(start) = run_start.take() {
            if run_len >= minimum_run {
                redacted.push_str("<redacted-hex>");
            } else {
                redacted.push_str(&text[start..index]);
            }
            run_len = 0;
        }
        redacted.push(ch);
    }

    if let Some(start) = run_start {
        if run_len >= minimum_run {
            redacted.push_str("<redacted-hex>");
        } else {
            redacted.push_str(&text[start..]);
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
    fn sensitive_order_text_redacts_128_bit_cloid_but_preserves_short_oid() {
        let cloid = "0x1234567890abcdef1234567890abcdef";
        let rendered = redact_sensitive_order_text(&format!("cloid={cloid} oid=42"));

        assert!(rendered.contains("<redacted-hex>"));
        assert!(rendered.contains("oid=42"));
        assert!(!rendered.contains(cloid));
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
    fn sensitive_response_excerpt_redacts_non_bearer_authorization_credentials() {
        let text = concat!(
            "Authorization: Basic base64-secret\n",
            "Proxy-Authorization: Token proxy-secret\n",
            "api_key=\"later-secret\""
        );
        let rendered = sensitive_response_excerpt(text, 240);

        assert!(rendered.contains("Authorization: Basic <redacted>"));
        assert!(rendered.contains("Proxy-Authorization: Token <redacted>"));
        assert!(rendered.contains("api_key=\"<redacted>\""));
        for secret in ["base64-secret", "proxy-secret", "later-secret"] {
            assert!(!rendered.contains(secret), "excerpt leaked {secret}");
        }
    }

    #[test]
    fn sensitive_response_excerpt_redacts_camel_case_token_aliases() {
        let text = r#"{"authToken":"auth-secret","accessToken":"access-secret","refreshToken":"refresh-secret","idToken":"id-secret","bearerToken":"bearer-secret"}"#;
        let rendered = sensitive_response_excerpt(text, 240);

        assert_eq!(rendered.matches("<redacted>").count(), 5);
        for secret in [
            "auth-secret",
            "access-secret",
            "refresh-secret",
            "id-secret",
            "bearer-secret",
        ] {
            assert!(!rendered.contains(secret), "excerpt leaked {secret}");
        }
    }

    #[test]
    fn sensitive_response_excerpt_redacts_client_secret_and_signatures() {
        let text = concat!(
            r#"{"client_secret":"client-secret","clientSecret":"camel-client-secret","client_id":"client-id-secret","clientId":"camel-client-id-secret"}"#,
            " signature=signature-secret sig=sig-secret"
        );
        let rendered = sensitive_response_excerpt(text, 360);

        assert_eq!(rendered.matches("<redacted>").count(), 6);
        for secret in [
            "client-secret",
            "camel-client-secret",
            "client-id-secret",
            "camel-client-id-secret",
            "signature-secret",
            "sig-secret",
        ] {
            assert!(!rendered.contains(secret), "excerpt leaked {secret}");
        }
    }

    #[test]
    fn sensitive_response_excerpt_redacts_cookie_headers() {
        let text = concat!(
            "Set-Cookie: sid=session-secret; Path=/; HttpOnly\n",
            "Cookie: a=first-cookie-secret; b=second-cookie-secret\n",
            "session_token=session-secret csrf-token=csrf-secret jwt=jwt-secret ok=true",
        );
        let rendered = sensitive_response_excerpt(text, 512);

        assert!(rendered.contains("Set-Cookie: <redacted>"));
        assert!(rendered.contains("Cookie: <redacted>"));
        assert!(rendered.contains("ok=true"));
        for secret in [
            "session-secret",
            "first-cookie-secret",
            "second-cookie-secret",
            "csrf-secret",
            "jwt-secret",
        ] {
            assert!(!rendered.contains(secret), "excerpt leaked {secret}");
        }
    }

    #[test]
    fn sensitive_response_excerpt_preserves_unicode_while_redacting_hex() {
        let text = "é€ 0x0123456789abcdef0123456789abcdef01234567";
        let rendered = sensitive_response_excerpt(text, 240);

        assert!(rendered.starts_with("é€ 0x<redacted-hex>"));
        assert!(!rendered.contains("0123456789abcdef0123456789abcdef01234567"));
    }

    #[test]
    fn wallet_address_debug_value_redacts_full_hex_addresses() {
        let address = "  0xAbC0000000000000000000000000000000000000  ";

        assert_eq!(redact_wallet_address_debug_value(address), "<redacted>");
    }

    #[test]
    fn wallet_address_debug_value_preserves_non_addresses() {
        assert_eq!(redact_wallet_address_debug_value("Whale"), "Whale");
        assert_eq!(
            redact_wallet_address_debug_value("0xabc0...0000"),
            "0xabc0...0000"
        );
    }

    #[test]
    fn path_neutral_io_error_detail_omits_custom_payload() {
        let error = std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "denied /home/alice/secret-path api_key=io-secret",
        );

        let rendered = path_neutral_io_error_detail(&error);

        assert_eq!(rendered, "permission denied");
        assert!(!rendered.contains("/home/alice"));
        assert!(!rendered.contains("io-secret"));
    }

    #[test]
    fn sensitive_response_excerpt_redacts_escaped_quoted_key_values() {
        let text = r#"{"error":"upstream echoed api_key=\"escaped-secret\""}"#;
        let rendered = sensitive_response_excerpt(text, 240);

        assert!(rendered.contains(r#"api_key=\"<redacted>\""#));
        assert!(!rendered.contains("escaped-secret"));
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
