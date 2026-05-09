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
    if !trade.legacy_note_ids.iter().any(|existing| existing == &id) {
        trade.legacy_note_ids.push(id);
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
