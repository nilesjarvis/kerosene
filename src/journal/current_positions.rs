use crate::account::AssetPosition;
use crate::helpers::parse_finite_number;

use super::AggregatedTrade;

const CURRENT_POSITION_TRADE_PREFIX: &str = "position";
const POSITION_EPSILON: f64 = 1e-9;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct JournalPositionReconciliation {
    pub added_open_positions: usize,
    pub removed_stale_positions: usize,
}

pub fn reconcile_current_position_trades(
    trades: &mut Vec<AggregatedTrade>,
    positions: &[AssetPosition],
    snapshot_time_ms: u64,
) -> JournalPositionReconciliation {
    let before_len = trades.len();
    trades.retain(|trade| !is_current_position_trade(&trade.id));
    let removed_stale_positions = before_len.saturating_sub(trades.len());

    let mut added_open_positions = 0;
    for position in positions {
        let Some(trade) = current_position_trade(position, snapshot_time_ms) else {
            continue;
        };

        if trades.iter().any(|existing| {
            existing.coin == trade.coin
                && existing.status == "OPEN"
                && !is_current_position_trade(&existing.id)
        }) {
            continue;
        }

        trades.push(trade);
        added_open_positions += 1;
    }

    if added_open_positions > 0 || removed_stale_positions > 0 {
        trades.sort_by_key(|trade| std::cmp::Reverse(trade.start_time));
    }

    JournalPositionReconciliation {
        added_open_positions,
        removed_stale_positions,
    }
}

pub fn current_position_fallback_warning(count: usize) -> String {
    format!(
        "{} open position{} shown from current account state because opening fills were not \
         present in loaded fill history.",
        count,
        if count == 1 { " is" } else { "s are" }
    )
}

fn is_current_position_trade(id: &str) -> bool {
    id.starts_with("position:")
}

fn current_position_trade(
    asset_position: &AssetPosition,
    snapshot_time_ms: u64,
) -> Option<AggregatedTrade> {
    let position = &asset_position.position;
    let szi = parse_finite_number(&position.szi)?;
    if szi.abs() <= POSITION_EPSILON {
        return None;
    }

    let entry_px = parse_finite_number(&position.entry_px).unwrap_or(0.0);
    let size = szi.abs();
    let total_entry_notional = size * entry_px;

    Some(AggregatedTrade {
        id: format!("{CURRENT_POSITION_TRADE_PREFIX}:{}", position.coin),
        legacy_note_ids: vec![format!("{}_current_position", position.coin)],
        coin: position.coin.clone(),
        start_time: snapshot_time_ms,
        end_time: None,
        max_position: szi,
        volume: total_entry_notional,
        fee: 0.0,
        pnl: 0.0,
        status: "OPEN".to_string(),
        fill_count: 0,
        avg_entry_price: entry_px,
        total_entry_notional,
        total_entry_size: size,
        is_long: szi > 0.0,
        basis_complete: false,
    })
}

#[cfg(test)]
mod tests;
