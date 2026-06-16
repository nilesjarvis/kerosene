use super::helpers::{add_legacy_note_id, legacy_flip_trade_id, legacy_trade_id, stable_trade_id};
use super::model::AggregatedTrade;
use crate::api::UserFill;

pub(super) fn new_non_perp_trade(coin: &str, fill: &UserFill) -> AggregatedTrade {
    let non_perp_prefix = if coin.starts_with('#') {
        "outcome"
    } else {
        "spot"
    };

    AggregatedTrade {
        id: format!("{}:{}:{}", non_perp_prefix, coin, fill.oid),
        legacy_note_ids: vec![format!("{}_{}", coin, fill.oid)],
        coin: coin.to_string(),
        start_time: fill.time,
        end_time: Some(fill.time),
        max_position: 0.0,
        volume: 0.0,
        fee: 0.0,
        pnl: 0.0,
        status: "FILLED".to_string(),
        fill_count: 0,
        avg_entry_price: 0.0,
        total_entry_notional: 0.0,
        total_entry_size: 0.0,
        is_long: true,
        basis_complete: true,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_non_perp_fill(
    trade: &mut AggregatedTrade,
    coin: &str,
    fill: &UserFill,
    signed_sz: f64,
    sz: f64,
    px: f64,
    fee: f64,
    closed_pnl: f64,
) {
    add_legacy_note_id(trade, format!("{}_{}", coin, fill.oid));
    trade.end_time = Some(fill.time);
    trade.max_position += signed_sz;
    trade.volume += sz * px;
    trade.fee += fee;
    // Hyperliquid reports realized PnL on closing (sell) spot fills, mirroring the
    // perp paths' `trade.pnl += closed_pnl`. Buy fills carry ~0 closedPnl.
    trade.pnl += closed_pnl;
    trade.fill_count += 1;

    trade.total_entry_size += sz;
    trade.total_entry_notional += sz * px;
    if trade.total_entry_size > 0.0 {
        trade.avg_entry_price = trade.total_entry_notional / trade.total_entry_size;
    }
}

pub(super) fn new_perp_trade(
    coin: &str,
    fill: &UserFill,
    start_pos: f64,
    new_pos: f64,
) -> AggregatedTrade {
    AggregatedTrade {
        id: stable_trade_id("perp", coin, fill),
        legacy_note_ids: vec![legacy_trade_id(coin, fill.time)],
        coin: coin.to_string(),
        start_time: fill.time,
        end_time: None,
        max_position: start_pos,
        volume: 0.0,
        fee: 0.0,
        pnl: 0.0,
        status: "OPEN".to_string(),
        fill_count: 0,
        avg_entry_price: 0.0,
        total_entry_notional: 0.0,
        total_entry_size: 0.0,
        is_long: new_pos > 0.0 || (new_pos == 0.0 && start_pos > 0.0),
        basis_complete: start_pos.abs() <= 1e-6,
    }
}

pub(super) fn new_flip_trade(
    coin: &str,
    fill: &UserFill,
    new_pos: f64,
    opening_sz: f64,
    px: f64,
    fee: f64,
    opening_ratio: f64,
) -> AggregatedTrade {
    AggregatedTrade {
        id: stable_trade_id("perp-flip", coin, fill),
        legacy_note_ids: vec![legacy_flip_trade_id(coin, fill.time)],
        coin: coin.to_string(),
        start_time: fill.time,
        end_time: None,
        max_position: new_pos,
        volume: opening_sz * px,
        fee: fee * opening_ratio,
        pnl: 0.0,
        status: "OPEN".to_string(),
        fill_count: 1,
        avg_entry_price: px,
        total_entry_notional: opening_sz * px,
        total_entry_size: opening_sz,
        is_long: new_pos > 0.0,
        basis_complete: true,
    }
}
