use super::{
    AggregatedTrade, JournalAttributedFillRole, JournalTradeDetails, JournalTradeSnapshotMetrics,
    JournalTradeSnapshotRequest,
};
use crate::api::Candle;

pub(super) fn journal_snapshot_metrics(
    request: &JournalTradeSnapshotRequest,
    trade: &AggregatedTrade,
    details: Option<&JournalTradeDetails>,
    candles: &[Candle],
) -> Result<JournalTradeSnapshotMetrics, String> {
    let overlapping: Vec<&Candle> = candles
        .iter()
        .filter(|candle| {
            candle.close_time >= request.trade_start_ms && candle.open_time <= request.trade_end_ms
        })
        .collect();
    if overlapping.is_empty() {
        return Err("No candles overlap the trade window.".to_string());
    }

    let entry_price = if trade.avg_entry_price.is_finite() && trade.avg_entry_price > 0.0 {
        trade.avg_entry_price
    } else {
        details
            .and_then(|details| {
                fill_vwap(
                    details,
                    &[
                        JournalAttributedFillRole::Increase,
                        JournalAttributedFillRole::FlipOpen,
                    ],
                )
            })
            .ok_or_else(|| "Could not derive entry price from fills.".to_string())?
    };

    let exit_price = if trade.end_time.is_some() {
        details
            .and_then(|details| {
                fill_vwap(
                    details,
                    &[
                        JournalAttributedFillRole::Reduce,
                        JournalAttributedFillRole::FlipClose,
                    ],
                )
            })
            .or_else(|| overlapping.last().map(|candle| candle.close))
    } else {
        overlapping.last().map(|candle| candle.close)
    }
    .filter(|price| price.is_finite() && *price > 0.0)
    .ok_or_else(|| "Could not derive exit/reference price.".to_string())?;

    let (lowest_low, highest_high) = overlapping
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), candle| {
            (low.min(candle.low), high.max(candle.high))
        });
    if !lowest_low.is_finite() || !highest_high.is_finite() || entry_price <= 0.0 {
        return Err("Candle price range is invalid.".to_string());
    }

    let raw_asset_move = (exit_price - entry_price) / entry_price;
    let directional_move = if trade.is_long {
        raw_asset_move
    } else {
        -raw_asset_move
    };
    let max_adverse_excursion = if trade.is_long {
        (lowest_low - entry_price) / entry_price
    } else {
        (entry_price - highest_high) / entry_price
    };
    let max_favorable_excursion = if trade.is_long {
        (highest_high - entry_price) / entry_price
    } else {
        (entry_price - lowest_low) / entry_price
    };

    Ok(JournalTradeSnapshotMetrics {
        timeframe: request.timeframe,
        candle_count: overlapping.len(),
        entry_price,
        exit_price,
        raw_asset_move,
        directional_move,
        max_adverse_excursion,
        max_favorable_excursion,
        asset_drawdown: peak_to_trough_drawdown(&overlapping),
    })
}

fn fill_vwap(details: &JournalTradeDetails, roles: &[JournalAttributedFillRole]) -> Option<f64> {
    let (weighted_sum, size_sum) = details
        .attributed_fills
        .iter()
        .filter(|fill| roles.contains(&fill.role))
        .filter(|fill| {
            fill.price.is_finite()
                && fill.price > 0.0
                && fill.attributed_size.is_finite()
                && fill.attributed_size > 0.0
        })
        .fold((0.0, 0.0), |(weighted_sum, size_sum), fill| {
            (
                weighted_sum + fill.price * fill.attributed_size,
                size_sum + fill.attributed_size,
            )
        });

    (size_sum > 0.0).then_some(weighted_sum / size_sum)
}

fn peak_to_trough_drawdown(candles: &[&Candle]) -> f64 {
    let mut peak = f64::NEG_INFINITY;
    let mut worst = 0.0_f64;

    for candle in candles {
        if candle.high.is_finite() && candle.high > peak {
            peak = candle.high;
        }
        if peak.is_finite() && peak > 0.0 && candle.low.is_finite() {
            worst = worst.min((candle.low - peak) / peak);
        }
    }

    worst
}

#[cfg(test)]
mod tests;
