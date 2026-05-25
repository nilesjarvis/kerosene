use super::*;
use crate::account::AssetContext;
use crate::config;
use crate::denomination::DisplayDenominationContext;
use crate::hyperdash_api::{PerpDeltaEntry, TickerPositionEntry};
use crate::positioning_state::PositioningInfoChangeSortField;
use crate::wallet_state::address_book::WalletDisplay;

mod change;
mod columns;
mod formatting;
mod live;

fn sample_position() -> TickerPositionEntry {
    TickerPositionEntry {
        address: "0xabc0000000000000000000000000000000001234".to_string(),
        display_name: None,
        label: Some("Desk A".to_string()),
        tag: Some("macro".to_string()),
        verified: Some(true),
        copy_score: Some(42.0),
        size: 10.0,
        notional_size: 1000.0,
        entry_price: 25.0,
        liquidation_price: None,
        unrealized_pnl: 15.0,
        funding_pnl: -1.0,
        account_value: 5000.0,
    }
}

fn asset_ctx(mark_px: Option<&str>, mid_px: Option<&str>) -> AssetContext {
    AssetContext {
        funding: None,
        open_interest: None,
        oracle_px: None,
        mark_px: mark_px.map(str::to_string),
        mid_px: mid_px.map(str::to_string),
        prev_day_px: None,
        day_ntl_vlm: None,
        impact_pxs: None,
    }
}

fn delta(address: &str, current: f64, change: f64) -> PerpDeltaEntry {
    PerpDeltaEntry {
        address: address.to_string(),
        current,
        delta: change,
    }
}
