use std::fmt;

#[derive(Clone, PartialEq)]
pub(super) struct ParsedClosePositionIntent {
    pub(super) symbol: Option<String>,
    pub(super) fraction: Option<f64>,
    pub(super) error: Option<String>,
}

impl fmt::Debug for ParsedClosePositionIntent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParsedClosePositionIntent")
            .field("has_symbol", &self.symbol.is_some())
            .field("fraction", &self.fraction.as_ref().map(|_| "<redacted>"))
            .field("has_error", &self.error.is_some())
            .finish()
    }
}

pub(super) fn parse_close_position_intent(query: &str) -> Option<ParsedClosePositionIntent> {
    let tokens = close_tokens(query);
    let close = tokens.first()?;
    if !close.eq_ignore_ascii_case("close") {
        return None;
    }

    let mut symbol = None;
    let mut fraction = None;
    let mut error = None;

    for token in tokens.iter().skip(1) {
        if is_close_filler(token) || is_close_percent_label(token) {
            continue;
        }

        if is_close_fraction_candidate(token) {
            if fraction.is_some() {
                error = Some("Use one close percentage".to_string());
                continue;
            }
            match parse_close_fraction(token) {
                Some(value) => fraction = Some(value),
                None => error = Some("Use a close percentage from 1 to 100".to_string()),
            }
            continue;
        }

        if symbol.is_none() {
            symbol = Some(token.clone());
        } else if error.is_none() {
            error = Some("Use one ticker to close".to_string());
        }
    }

    Some(ParsedClosePositionIntent {
        symbol,
        fraction,
        error,
    })
}

fn close_tokens(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(trim_close_token)
        .filter(|token| !token.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn trim_close_token(token: &str) -> &str {
    token.trim_matches(|ch: char| {
        matches!(
            ch,
            '\'' | '"' | '(' | ')' | '[' | ']' | '{' | '}' | ';' | ','
        )
    })
}

fn parse_close_fraction(token: &str) -> Option<f64> {
    let value = token.trim().trim_end_matches('%').parse::<f64>().ok()?;
    (value.is_finite() && value > 0.0 && value <= 100.0).then_some(value / 100.0)
}

fn is_close_fraction_candidate(token: &str) -> bool {
    let token = token.trim();
    token.ends_with('%') || token.parse::<f64>().is_ok()
}

fn is_close_filler(token: &str) -> bool {
    matches!(token.to_ascii_lowercase().as_str(), "of" | "position")
}

fn is_close_percent_label(token: &str) -> bool {
    matches!(
        token.to_ascii_lowercase().as_str(),
        "percent" | "pct" | "percentage"
    )
}
