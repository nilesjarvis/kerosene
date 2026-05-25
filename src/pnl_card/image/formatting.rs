use chrono::Local;

// ---------------------------------------------------------------------------
// Export Text Formatting
// ---------------------------------------------------------------------------

pub(in crate::pnl_card) fn export_text(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            let upper = ch.to_ascii_uppercase();
            if matches!(
                upper,
                'A'..='Z'
                    | '0'..='9'
                    | '/'
                    | ':'
                    | '-'
                    | '_'
                    | '.'
                    | ','
                    | '+'
                    | '$'
                    | '%'
                    | '*'
                    | ' '
            ) {
                upper
            } else if ch.is_whitespace() {
                ' '
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

pub(in crate::pnl_card) fn pnl_card_filename(ticker: &str) -> String {
    let safe_ticker = ticker
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    let safe_ticker = if safe_ticker.is_empty() {
        "pnl-card".to_string()
    } else {
        safe_ticker
    };
    format!(
        "kerosene-{safe_ticker}-pnl-card-{}.png",
        Local::now().format("%Y%m%d-%H%M%S")
    )
}
