use crate::api::UserFill;

mod builders;
mod helpers;
mod identity;
mod model;
mod position;

use builders::{apply_non_perp_fill, new_flip_trade, new_non_perp_trade, new_perp_trade};
use helpers::{
    add_legacy_note_id, legacy_trade_id, non_perp_fee_usd, parse_fill_values, stable_trade_id,
};
use position::{
    POSITION_EPSILON, fill_position_transition, is_non_perp_coin, resolved_start_position,
    signed_fill_size,
};
use std::cmp::Reverse;
use std::collections::HashMap;

pub use identity::{FillIdentity, merge_fills, newest_fill_time, normalize_fills};
pub use model::{
    AggregatedTrade, AggregationDiagnostics, AggregationResult, JournalAttributedFill,
    JournalAttributedFillRole, JournalTradeDetails,
};

// ---------------------------------------------------------------------------
// Aggregation
// ---------------------------------------------------------------------------

pub fn aggregate_trades_with_diagnostics(mut fills: Vec<UserFill>) -> AggregationResult {
    normalize_fills(&mut fills);

    // Per coin, the earliest fill that opens from flat (`startPosition ≈ 0`).
    // A trade's basis is reconstructible whenever such an open exists at or
    // before its own opening fill — see `coin_first_flat_open_times`.
    let coin_first_flat_open = coin_first_flat_open_times(&fills);

    let mut trade_history = Vec::new();
    let mut current_trades: HashMap<String, AggregatedTrade> = HashMap::new();
    let mut spot_trades: HashMap<u64, AggregatedTrade> = HashMap::new();
    let mut trade_details: HashMap<String, JournalTradeDetails> = HashMap::new();
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
            // Non-USDC spot fees (base-token buy fees) are converted to USD so the
            // trade card, per-asset table, and totals stay consistently USD-denominated.
            let fee_usd = non_perp_fee_usd(fee, &fill.fee_token, px);
            let mut trade = spot_trades
                .remove(&fill.oid)
                .unwrap_or_else(|| new_non_perp_trade(&coin, &fill));
            apply_non_perp_fill(
                &mut trade, &coin, &fill, signed_sz, sz, px, fee_usd, closed_pnl,
            );
            record_attributed_fill(
                &mut trade_details,
                &trade.id,
                &coin,
                &fill,
                px,
                sz,
                sz,
                if signed_sz >= 0.0 {
                    JournalAttributedFillRole::Increase
                } else {
                    JournalAttributedFillRole::Reduce
                },
                fee_usd,
                closed_pnl,
            );
            spot_trades.insert(fill.oid, trade);
            continue;
        }

        let resolved_start = resolved_start_position(
            api_start_pos,
            tracked_positions.get(&coin).copied(),
            fill.time,
        );
        if resolved_start.same_timestamp_mismatch {
            diagnostics.same_timestamp_position_mismatch_count += 1;
        }
        let start_pos = resolved_start.start_pos;
        let transition = fill_position_transition(start_pos, signed_sz, is_settlement);
        let new_pos = transition.new_pos;

        if !is_settlement {
            tracked_positions.insert(coin.clone(), (fill.time, new_pos));
        }

        let mut trade = current_trades.remove(&coin).unwrap_or_else(|| {
            // The trade's opening basis is reconstructible if the coin opened
            // from flat at or before this fill within the loaded history, even
            // when same-timestamp ordering or a dust residual made this chain
            // begin on a reducing fill rather than the true open.
            let basis_complete = coin_first_flat_open
                .get(&coin)
                .is_some_and(|&flat_time| flat_time <= fill.time);
            new_perp_trade(&coin, &fill, start_pos, new_pos, basis_complete)
        });

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
        add_stable_note_ids_for_fill(&mut trade, &coin, &fill);

        if is_settlement {
            trade.fee += fee;
            trade.pnl += closed_pnl;
            record_attributed_fill(
                &mut trade_details,
                &trade.id,
                &coin,
                &fill,
                px,
                sz,
                0.0,
                JournalAttributedFillRole::Settlement,
                fee,
                closed_pnl,
            );
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
            record_attributed_fill(
                &mut trade_details,
                &trade.id,
                &coin,
                &fill,
                px,
                sz,
                closing_sz,
                JournalAttributedFillRole::FlipClose,
                fee * closing_ratio,
                closed_pnl,
            );

            trade.status = "CLOSED".to_string();
            trade.end_time = Some(fill.time);
            trade_history.push(trade);

            let mut new_trade =
                new_flip_trade(&coin, &fill, new_pos, opening_sz, px, fee, opening_ratio);
            add_stable_note_ids_for_fill(&mut new_trade, &coin, &fill);
            record_attributed_fill(
                &mut trade_details,
                &new_trade.id,
                &coin,
                &fill,
                px,
                sz,
                opening_sz,
                JournalAttributedFillRole::FlipOpen,
                fee * opening_ratio,
                0.0,
            );
            current_trades.insert(coin, new_trade);
        } else {
            trade.volume += sz * px;
            trade.fee += fee;
            trade.pnl += closed_pnl;

            let exposure_increased = new_pos.abs() > start_pos.abs();
            let role = if exposure_increased {
                JournalAttributedFillRole::Increase
            } else {
                JournalAttributedFillRole::Reduce
            };
            let attributed_size = if exposure_increased {
                new_pos.abs() - start_pos.abs()
            } else {
                (start_pos.abs() - new_pos.abs()).max(0.0)
            };
            record_attributed_fill(
                &mut trade_details,
                &trade.id,
                &coin,
                &fill,
                px,
                sz,
                attributed_size,
                role,
                fee,
                closed_pnl,
            );

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
        trade_details,
        diagnostics,
    }
}

/// Earliest fill time, per perp coin, at which a fill reports a flat opening
/// position (`startPosition ≈ 0`).
///
/// Hyperliquid's per-fill `startPosition` is authoritative, so a coin with such
/// a fill demonstrably opened from flat within the loaded history. Because fills
/// are fetched contiguously, every trade for that coin that opens at or after
/// that time has a reconstructible basis — even when same-timestamp fill
/// ordering or a sub-epsilon dust residual causes the trade's chain to begin on
/// a reducing ("Close") fill instead of the true open. Relying only on the
/// chain head's `startPosition` (the previous behavior) mis-flagged those trades
/// as "partial" despite the open being present in the data.
fn coin_first_flat_open_times(fills: &[UserFill]) -> HashMap<String, u64> {
    let mut earliest: HashMap<String, u64> = HashMap::new();
    for fill in fills {
        if is_non_perp_coin(&fill.coin) || fill.dir == "Settlement" {
            continue;
        }
        let Ok(parsed) = parse_fill_values(fill) else {
            continue;
        };
        if parsed.start_pos.abs() <= POSITION_EPSILON {
            earliest
                .entry(fill.coin.clone())
                .and_modify(|time| *time = (*time).min(fill.time))
                .or_insert(fill.time);
        }
    }
    earliest
}

fn add_stable_note_ids_for_fill(trade: &mut AggregatedTrade, coin: &str, fill: &UserFill) {
    add_legacy_note_id(trade, stable_trade_id("perp", coin, fill));
    add_legacy_note_id(trade, stable_trade_id("perp-flip", coin, fill));
}

#[allow(clippy::too_many_arguments)]
fn record_attributed_fill(
    trade_details: &mut HashMap<String, JournalTradeDetails>,
    trade_id: &str,
    coin: &str,
    fill: &UserFill,
    price: f64,
    raw_size: f64,
    attributed_size: f64,
    role: JournalAttributedFillRole,
    fee: f64,
    closed_pnl: f64,
) {
    trade_details
        .entry(trade_id.to_string())
        .or_insert_with(|| JournalTradeDetails {
            trade_id: trade_id.to_string(),
            coin: coin.to_string(),
            attributed_fills: Vec::new(),
        })
        .attributed_fills
        .push(JournalAttributedFill {
            identity: FillIdentity::from(fill),
            time_ms: fill.time,
            price,
            raw_size,
            attributed_size,
            side: fill.side.clone(),
            role,
            fee,
            closed_pnl,
        });
}
