use crate::app_time::DisplayTimeZone;

// ---------------------------------------------------------------------------
// Export File Naming
// ---------------------------------------------------------------------------

pub(in crate::pnl_card) fn pnl_card_filename(ticker: &str, zone: DisplayTimeZone) -> String {
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
        chrono::Utc::now()
            .with_timezone(&zone.0)
            .format("%Y%m%d-%H%M%S")
    )
}
