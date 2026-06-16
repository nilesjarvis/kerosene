use super::AggregatedTrade;
use crate::api::UserFill;

// ---------------------------------------------------------------------------
// Aggregation Helpers
// ---------------------------------------------------------------------------

pub(super) struct ParsedFillValues {
    pub(super) sz: f64,
    pub(super) start_pos: f64,
    pub(super) px: f64,
    pub(super) fee: f64,
    pub(super) closed_pnl: f64,
}

fn stable_fill_id(fill: &UserFill) -> String {
    let hash = fill.hash.strip_prefix("0x").unwrap_or(&fill.hash);
    let short_hash: String = hash.chars().take(16).collect();
    format!(
        "{}:{}:{}:{}:{}",
        fill.time, fill.tid, fill.oid, fill.side, short_hash
    )
}

pub(super) fn stable_trade_id(kind: &str, coin: &str, fill: &UserFill) -> String {
    format!("{}:{}:{}", kind, coin, stable_fill_id(fill))
}

pub(super) fn legacy_trade_id(coin: &str, time: u64) -> String {
    format!("{}_{}", coin, time)
}

pub(super) fn legacy_flip_trade_id(coin: &str, time: u64) -> String {
    format!("{}_{}_flip", coin, time)
}

pub(super) fn add_legacy_note_id(trade: &mut AggregatedTrade, id: String) {
    if !trade.legacy_note_ids.contains(&id) {
        trade.legacy_note_ids.push(id);
    }
}

/// Convert a spot/outcome fill fee into USD.
///
/// Spot fees are charged in the token received: a USDC-quoted *buy* pays its fee
/// in the base token (e.g. a `HYPE/USDC` buy is charged in HYPE), while a *sell*
/// is charged in USDC. `px` is the pair's USDC price per base unit, so a
/// non-USDC fee converts to USD by multiplying by `px`. A USDC (or empty/unknown)
/// fee token is already USD-denominated and is returned unchanged.
pub(super) fn non_perp_fee_usd(fee: f64, fee_token: &str, px: f64) -> f64 {
    if fee_token.is_empty() || fee_token.eq_ignore_ascii_case("USDC") {
        fee
    } else {
        fee * px
    }
}

pub(super) fn parse_fill_values(fill: &UserFill) -> Result<ParsedFillValues, String> {
    Ok(ParsedFillValues {
        sz: fill.sz.parse::<f64>().map_err(|_| "size".to_string())?,
        start_pos: fill
            .start_position
            .parse::<f64>()
            .map_err(|_| "start position".to_string())?,
        px: fill.px.parse::<f64>().map_err(|_| "price".to_string())?,
        fee: fill.fee.parse::<f64>().map_err(|_| "fee".to_string())?,
        closed_pnl: fill
            .closed_pnl
            .parse::<f64>()
            .map_err(|_| "closed PnL".to_string())?,
    })
}
