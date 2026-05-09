use crate::api::UserFill;

mod builders;
mod helpers;
mod identity;
mod model;
mod position;

use builders::{apply_non_perp_fill, new_flip_trade, new_non_perp_trade, new_perp_trade};
use helpers::{add_legacy_note_id, legacy_trade_id, parse_fill_values};
use position::{
    fill_position_transition, is_non_perp_coin, resolved_start_position, signed_fill_size,
};
use std::cmp::Reverse;
use std::collections::HashMap;

pub use identity::{merge_fills, newest_fill_time, normalize_fills};
pub use model::{AggregatedTrade, AggregationDiagnostics, AggregationResult};

// ---------------------------------------------------------------------------
// Aggregation
// ---------------------------------------------------------------------------

pub fn aggregate_trades_with_diagnostics(mut fills: Vec<UserFill>) -> AggregationResult {
    normalize_fills(&mut fills);

    let mut trade_history = Vec::new();
    let mut current_trades: HashMap<String, AggregatedTrade> = HashMap::new();
    let mut spot_trades: HashMap<u64, AggregatedTrade> = HashMap::new();
    let mut tracked_positions: HashMap<String, (u64, f64)> = HashMap::new();
    let mut diagnostics = AggregationDiagnostics::default();

    for fill in fills {
        let coin = fill.coin.clone();
        let parsed = match parse_fill_values(&fill) {
            Ok(parsed) => parsed,
            Err(_) => {
                diagnostics.skipped_fill_count += 1;
                continue;
            }
        };
        let sz = parsed.sz;
        let api_start_pos = parsed.start_pos;
        let px = parsed.px;
        let fee = parsed.fee;
        let closed_pnl = parsed.closed_pnl;

        let is_settlement = fill.dir == "Settlement";
        let signed_sz = signed_fill_size(&fill.side, sz);

        // Spot and outcome trades don't have a concept of open margin positions.
        // We aggregate their executions simply by Order ID.
        if is_non_perp_coin(&coin) {
            let mut trade = spot_trades
                .remove(&fill.oid)
                .unwrap_or_else(|| new_non_perp_trade(&coin, &fill));
            apply_non_perp_fill(&mut trade, &coin, &fill, signed_sz, sz, px, fee);
            spot_trades.insert(fill.oid, trade);
            continue;
        }

        let start_pos = resolved_start_position(
            api_start_pos,
            tracked_positions.get(&coin).copied(),
            fill.time,
        );
        let transition = fill_position_transition(start_pos, signed_sz, is_settlement);
        let new_pos = transition.new_pos;

        if !is_settlement {
            tracked_positions.insert(coin.clone(), (fill.time, new_pos));
        }

        let mut trade = current_trades
            .remove(&coin)
            .unwrap_or_else(|| new_perp_trade(&coin, &fill, start_pos, new_pos));

        if trade.total_entry_size == 0.0 && !transition.is_flip {
            trade.is_long = new_pos > 0.0;
        }

        if start_pos.abs() > trade.max_position.abs() {
            trade.max_position = start_pos;
        }
        if new_pos.abs() > trade.max_position.abs() {
            trade.max_position = new_pos;
        }

        trade.fill_count += 1;
        add_legacy_note_id(&mut trade, legacy_trade_id(&coin, fill.time));

        if is_settlement {
            trade.fee += fee;
            trade.pnl += closed_pnl;
            current_trades.insert(coin, trade);
            continue;
        }

        if transition.is_flip {
            let closing_sz = start_pos.abs();
            let opening_sz = new_pos.abs();
            let total_sz = sz;

            let closing_ratio = if total_sz > 0.0 {
                closing_sz / total_sz
            } else {
                1.0
            };
            let opening_ratio = if total_sz > 0.0 {
                opening_sz / total_sz
            } else {
                0.0
            };

            trade.volume += closing_sz * px;
            trade.fee += fee * closing_ratio;
            trade.pnl += closed_pnl;

            trade.status = "CLOSED".to_string();
            trade.end_time = Some(fill.time);
            trade_history.push(trade);

            let new_trade =
                new_flip_trade(&coin, &fill, new_pos, opening_sz, px, fee, opening_ratio);
            current_trades.insert(coin, new_trade);
        } else {
            trade.volume += sz * px;
            trade.fee += fee;
            trade.pnl += closed_pnl;

            if new_pos.abs() > start_pos.abs() {
                let increase_sz = new_pos.abs() - start_pos.abs();
                trade.total_entry_size += increase_sz;
                trade.total_entry_notional += increase_sz * px;
                if trade.total_entry_size > 0.0 {
                    trade.avg_entry_price = trade.total_entry_notional / trade.total_entry_size;
                }
            }

            if transition.is_close {
                trade.status = "CLOSED".to_string();
                trade.end_time = Some(fill.time);
                trade_history.push(trade);
            } else {
                current_trades.insert(coin, trade);
            }
        }
    }

    for (_, trade) in current_trades {
        trade_history.push(trade);
    }

    for (_, trade) in spot_trades {
        trade_history.push(trade);
    }

    trade_history.sort_by_key(|trade| Reverse(trade.start_time));
    diagnostics.incomplete_trade_count = trade_history
        .iter()
        .filter(|trade| !trade.basis_complete)
        .count();

    AggregationResult {
        trades: trade_history,
        diagnostics,
    }
}
