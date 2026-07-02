use crate::account::{position_notional_from_mark_or_wire, position_upnl_from_mark_or_wire};

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
    position_notional_from_mark_or_wire(szi, parse_wallet_number(wire_position_value), mark_px)
}

pub(in crate::wallet_views) fn wallet_position_upnl(
    szi: Option<f64>,
    entry_px: Option<f64>,
    wire_upnl: &str,
    mark_px: Option<f64>,
) -> Option<f64> {
    position_upnl_from_mark_or_wire(szi, entry_px, parse_wallet_number(wire_upnl), mark_px)
}

/// Whether a missing metric on a synthesized spot/outcome row is merely
/// unavailable rather than invalid: balances without a derivable cost basis
/// (e.g. transferred-in tokens) carry empty wire strings for entry, value,
/// and PnL. Such rows render as "-" and must not poison summary totals,
/// matching the main positions table's unavailable-vs-invalid distinction.
pub(in crate::wallet_views) fn wallet_spot_value_unavailable(
    spot_like: bool,
    parsed: Option<f64>,
    wire: &str,
) -> bool {
    spot_like && parsed.is_none() && wire.trim().is_empty()
}
