use super::numbers::parse_wallet_number;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Wallet Position Metrics
// ---------------------------------------------------------------------------

pub(in crate::wallet_views) fn wallet_position_value(
    szi: Option<f64>,
    wire_position_value: &str,
    mark_px: Option<f64>,
) -> Option<f64> {
    match (szi, mark_px) {
        (Some(szi), Some(mark_px)) => Some(szi.abs() * mark_px),
        _ => parse_wallet_number(wire_position_value).map(f64::abs),
    }
}

pub(in crate::wallet_views) fn wallet_position_upnl(
    szi: Option<f64>,
    entry_px: Option<f64>,
    wire_upnl: &str,
    mark_px: Option<f64>,
) -> Option<f64> {
    match (szi, entry_px, mark_px) {
        (Some(szi), Some(entry_px), Some(mark_px)) => Some(szi * (mark_px - entry_px)),
        _ => parse_wallet_number(wire_upnl),
    }
}
