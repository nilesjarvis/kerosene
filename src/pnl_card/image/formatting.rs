use chrono::Local;

// ---------------------------------------------------------------------------
// Export File Naming
// ---------------------------------------------------------------------------

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
