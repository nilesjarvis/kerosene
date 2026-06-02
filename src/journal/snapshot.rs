use super::{AggregatedTrade, JournalAttributedFillRole, JournalTradeDetails};
use crate::api::Candle;
use crate::chart::TradeMarker;
use crate::config::ChartBackfillSource;
use crate::timeframe::Timeframe;

const SNAPSHOT_MAX_CANDLES: u64 = 260;
const MIN_PADDING_MS: u64 = 60 * 60 * 1000;
const SNAPSHOT_LADDER: &[Timeframe] = &[
    Timeframe::M1,
    Timeframe::M3,
    Timeframe::M5,
    Timeframe::M15,
    Timeframe::M30,
    Timeframe::H1,
    Timeframe::H2,
    Timeframe::H4,
    Timeframe::H8,
    Timeframe::H12,
    Timeframe::D1,
    Timeframe::D3,
    Timeframe::W1,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalTradeSnapshotRequest {
    pub account_key: Option<String>,
    pub address: String,
    pub trade_id: String,
    pub coin: String,
    pub source: ChartBackfillSource,
    pub timeframe: Timeframe,
    pub ladder_index: usize,
    pub trade_start_ms: u64,
    pub trade_end_ms: u64,
    pub is_open: bool,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct JournalTradeSnapshot {
    pub trade_id: String,
    pub coin: String,
    pub source: ChartBackfillSource,
    pub timeframe: Timeframe,
    pub trade_start_ms: u64,
    pub trade_end_ms: u64,
    pub is_open: bool,
    pub start_ms: u64,
    pub end_ms: u64,
    pub candles: Vec<Candle>,
    pub markers: Vec<TradeMarker>,
    pub metrics: JournalTradeSnapshotMetrics,
    pub status: JournalTradeSnapshotStatus,
}

#[derive(Debug, Clone)]
pub struct JournalTradeSnapshotMetrics {
    pub timeframe: Timeframe,
    pub candle_count: usize,
    pub entry_price: f64,
    pub exit_price: f64,
    pub raw_asset_move: f64,
    pub directional_move: f64,
    pub max_adverse_excursion: f64,
    pub max_favorable_excursion: f64,
    pub asset_drawdown: f64,
}

#[derive(Debug, Clone)]
pub enum JournalTradeSnapshotStatus {
    Loaded,
    Unavailable(String),
}

pub fn initial_snapshot_request(
    account_key: Option<String>,
    address: String,
    trade: &AggregatedTrade,
    source: ChartBackfillSource,
    now_ms: u64,
) -> Result<JournalTradeSnapshotRequest, String> {
    if trade.coin.starts_with('@') || trade.coin.starts_with('#') {
        return Err("Chart snapshots are currently available for perp trades only.".to_string());
    }
    if !trade.basis_complete {
        return Err(
            "Snapshot unavailable because opening fills are outside loaded history.".to_string(),
        );
    }

    let trade_end_ms = trade.end_time.unwrap_or(now_ms).max(trade.start_time);
    let ladder_index = initial_ladder_index(trade.start_time, trade_end_ms);
    snapshot_request_for_ladder_index(
        account_key,
        address,
        trade,
        source,
        trade_end_ms,
        trade.end_time.is_none(),
        ladder_index,
    )
}

pub fn next_snapshot_request(
    request: &JournalTradeSnapshotRequest,
) -> Option<JournalTradeSnapshotRequest> {
    let next_ladder_index = request.ladder_index.saturating_add(1);
    let timeframe = *SNAPSHOT_LADDER.get(next_ladder_index)?;
    let duration = request.trade_end_ms.saturating_sub(request.trade_start_ms);
    let padding = snapshot_padding_ms(duration, timeframe);

    Some(JournalTradeSnapshotRequest {
        account_key: request.account_key.clone(),
        address: request.address.clone(),
        trade_id: request.trade_id.clone(),
        coin: request.coin.clone(),
        source: request.source,
        timeframe,
        ladder_index: next_ladder_index,
        trade_start_ms: request.trade_start_ms,
        trade_end_ms: request.trade_end_ms,
        is_open: request.is_open,
        start_ms: request.trade_start_ms.saturating_sub(padding),
        end_ms: if request.is_open {
            request.trade_end_ms
        } else {
            request.trade_end_ms.saturating_add(padding)
        },
    })
}

pub fn build_journal_trade_snapshot(
    request: &JournalTradeSnapshotRequest,
    trade: &AggregatedTrade,
    details: &JournalTradeDetails,
    candles: Vec<Candle>,
) -> Result<JournalTradeSnapshot, String> {
    let metrics = journal_snapshot_metrics(request, trade, details, &candles)?;
    let markers = snapshot_markers_for_details(details);

    Ok(JournalTradeSnapshot {
        trade_id: request.trade_id.clone(),
        coin: request.coin.clone(),
        source: request.source,
        timeframe: request.timeframe,
        trade_start_ms: request.trade_start_ms,
        trade_end_ms: request.trade_end_ms,
        is_open: request.is_open,
        start_ms: request.start_ms,
        end_ms: request.end_ms,
        candles,
        markers,
        metrics,
        status: JournalTradeSnapshotStatus::Loaded,
    })
}

pub fn unavailable_snapshot(
    trade: &AggregatedTrade,
    source: ChartBackfillSource,
    now_ms: u64,
    reason: String,
) -> JournalTradeSnapshot {
    let trade_end_ms = trade.end_time.unwrap_or(now_ms).max(trade.start_time);
    JournalTradeSnapshot {
        trade_id: trade.id.clone(),
        coin: trade.coin.clone(),
        source,
        timeframe: Timeframe::M1,
        trade_start_ms: trade.start_time,
        trade_end_ms,
        is_open: trade.end_time.is_none(),
        start_ms: trade.start_time,
        end_ms: trade_end_ms,
        candles: Vec::new(),
        markers: Vec::new(),
        metrics: JournalTradeSnapshotMetrics {
            timeframe: Timeframe::M1,
            candle_count: 0,
            entry_price: trade.avg_entry_price,
            exit_price: trade.avg_entry_price,
            raw_asset_move: 0.0,
            directional_move: 0.0,
            max_adverse_excursion: 0.0,
            max_favorable_excursion: 0.0,
            asset_drawdown: 0.0,
        },
        status: JournalTradeSnapshotStatus::Unavailable(reason),
    }
}

pub fn snapshot_markers_for_details(details: &JournalTradeDetails) -> Vec<TradeMarker> {
    let mut markers: Vec<_> = details
        .attributed_fills
        .iter()
        .filter(|fill| fill.role != JournalAttributedFillRole::Settlement)
        .filter(|fill| fill.price.is_finite() && fill.price > 0.0)
        .filter(|fill| fill.attributed_size.is_finite() && fill.attributed_size > 0.0)
        .filter_map(|fill| {
            let is_buy = match fill.side.as_str() {
                "B" => true,
                "A" => false,
                _ => return None,
            };
            Some(TradeMarker {
                time_ms: fill.time_ms,
                price: fill.price,
                size: fill.attributed_size,
                is_buy,
            })
        })
        .collect();
    markers.sort_by_key(|marker| marker.time_ms);
    markers
}

fn snapshot_request_for_ladder_index(
    account_key: Option<String>,
    address: String,
    trade: &AggregatedTrade,
    source: ChartBackfillSource,
    trade_end_ms: u64,
    is_open: bool,
    ladder_index: usize,
) -> Result<JournalTradeSnapshotRequest, String> {
    let timeframe = *SNAPSHOT_LADDER
        .get(ladder_index)
        .ok_or_else(|| "No candle timeframe available for snapshot.".to_string())?;
    let duration = trade_end_ms.saturating_sub(trade.start_time);
    let padding = snapshot_padding_ms(duration, timeframe);

    Ok(JournalTradeSnapshotRequest {
        account_key,
        address,
        trade_id: trade.id.clone(),
        coin: trade.coin.clone(),
        source,
        timeframe,
        ladder_index,
        trade_start_ms: trade.start_time,
        trade_end_ms,
        is_open,
        start_ms: trade.start_time.saturating_sub(padding),
        end_ms: if is_open {
            trade_end_ms
        } else {
            trade_end_ms.saturating_add(padding)
        },
    })
}

fn initial_ladder_index(trade_start_ms: u64, trade_end_ms: u64) -> usize {
    let duration = trade_end_ms.saturating_sub(trade_start_ms);
    SNAPSHOT_LADDER
        .iter()
        .position(|timeframe| {
            let padding = snapshot_padding_ms(duration, *timeframe);
            let padded = duration.saturating_add(padding.saturating_mul(2));
            padded.div_ceil(timeframe.duration_ms().max(1)) <= SNAPSHOT_MAX_CANDLES
        })
        .unwrap_or(SNAPSHOT_LADDER.len().saturating_sub(1))
}

fn snapshot_padding_ms(duration_ms: u64, timeframe: Timeframe) -> u64 {
    (duration_ms.saturating_mul(3) / 4)
        .max(timeframe.duration_ms().saturating_mul(12))
        .max(MIN_PADDING_MS)
}

fn journal_snapshot_metrics(
    request: &JournalTradeSnapshotRequest,
    trade: &AggregatedTrade,
    details: &JournalTradeDetails,
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
        fill_vwap(
            details,
            &[
                JournalAttributedFillRole::Increase,
                JournalAttributedFillRole::FlipOpen,
            ],
        )
        .ok_or_else(|| "Could not derive entry price from fills.".to_string())?
    };

    let exit_price = if trade.end_time.is_some() {
        fill_vwap(
            details,
            &[
                JournalAttributedFillRole::Reduce,
                JournalAttributedFillRole::FlipClose,
            ],
        )
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
mod tests {
    use super::*;
    use crate::journal::{FillIdentity, JournalAttributedFill};

    fn candle(open_time: u64, close_time: u64, low: f64, high: f64, close: f64) -> Candle {
        Candle {
            open_time,
            close_time,
            open: close,
            high,
            low,
            close,
            volume: 1.0,
        }
    }

    fn trade(is_long: bool) -> AggregatedTrade {
        AggregatedTrade {
            id: "perp:BTC:test".to_string(),
            legacy_note_ids: Vec::new(),
            coin: "BTC".to_string(),
            start_time: 1_000,
            end_time: Some(2_000),
            max_position: 1.0,
            volume: 0.0,
            fee: 0.0,
            pnl: 0.0,
            status: "CLOSED".to_string(),
            fill_count: 2,
            avg_entry_price: 100.0,
            total_entry_notional: 100.0,
            total_entry_size: 1.0,
            is_long,
            basis_complete: true,
        }
    }

    fn details() -> JournalTradeDetails {
        JournalTradeDetails {
            trade_id: "perp:BTC:test".to_string(),
            coin: "BTC".to_string(),
            attributed_fills: vec![JournalAttributedFill {
                identity: FillIdentity {
                    time: 2_000,
                    tid: 1,
                    oid: 1,
                    hash: "0x1".to_string(),
                    coin: "BTC".to_string(),
                    side: "A".to_string(),
                    px: "110".to_string(),
                    sz: "1".to_string(),
                },
                time_ms: 2_000,
                price: 110.0,
                raw_size: 1.0,
                attributed_size: 1.0,
                side: "A".to_string(),
                role: JournalAttributedFillRole::Reduce,
                fee: 0.0,
                closed_pnl: 10.0,
            }],
        }
    }

    fn request() -> JournalTradeSnapshotRequest {
        JournalTradeSnapshotRequest {
            account_key: Some("acct".to_string()),
            address: "0xabc".to_string(),
            trade_id: "perp:BTC:test".to_string(),
            coin: "BTC".to_string(),
            source: ChartBackfillSource::Hyperliquid,
            timeframe: Timeframe::M1,
            ladder_index: 0,
            trade_start_ms: 1_000,
            trade_end_ms: 2_000,
            is_open: false,
            start_ms: 0,
            end_ms: 3_000,
        }
    }

    #[test]
    fn overlapping_candles_include_candle_spanning_short_trade() {
        let metrics = journal_snapshot_metrics(
            &request(),
            &trade(true),
            &details(),
            &[candle(0, 60_000, 95.0, 115.0, 110.0)],
        )
        .expect("metrics");

        assert_eq!(metrics.candle_count, 1);
        assert!((metrics.raw_asset_move - 0.10).abs() <= 1e-9);
    }

    #[test]
    fn short_directional_metrics_are_inverted() {
        let metrics = journal_snapshot_metrics(
            &request(),
            &trade(false),
            &details(),
            &[candle(0, 60_000, 95.0, 115.0, 110.0)],
        )
        .expect("metrics");

        assert!((metrics.directional_move + 0.10).abs() <= 1e-9);
        assert!((metrics.max_adverse_excursion + 0.15).abs() <= 1e-9);
        assert!((metrics.max_favorable_excursion - 0.05).abs() <= 1e-9);
    }

    #[test]
    fn planner_chooses_one_minute_for_short_trades() {
        let idx = initial_ladder_index(1_000, 61_000);
        assert_eq!(SNAPSHOT_LADDER[idx], Timeframe::M1);
    }

    #[test]
    fn empty_retry_advances_to_next_timeframe() {
        let next = next_snapshot_request(&request()).expect("next request");
        assert_eq!(next.timeframe, Timeframe::M3);
        assert_eq!(next.ladder_index, 1);
    }
}
