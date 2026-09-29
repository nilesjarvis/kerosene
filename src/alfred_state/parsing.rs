pub(super) fn command_tokens(query: &str) -> impl Iterator<Item = &str> {
    query
        .split_whitespace()
        .map(trim_token)
        .filter(|token| !token.is_empty())
}

fn trim_token(token: &str) -> &str {
    token.trim_matches(|ch: char| {
        matches!(
            ch,
            '\'' | '"' | '(' | ')' | '[' | ']' | '{' | '}' | ';' | ','
        )
    })
}

pub(super) fn normalize_symbol_input(symbol: &str) -> String {
    if symbol.starts_with('@') || symbol.starts_with('#') || symbol.starts_with('+') {
        return symbol.to_string();
    }

    if let Some((dex, ticker)) = symbol.split_once(':') {
        format!(
            "{}:{}",
            dex.to_ascii_lowercase(),
            ticker.to_ascii_uppercase()
        )
    } else {
        symbol.to_ascii_uppercase()
    }
}
