use super::{
    AggregatedTrade, JournalSnapshotCoverage, JournalSnapshotRequestSettings,
    JournalTradeSnapshotRequest,
};
use crate::config::ChartBackfillSource;
use crate::journal::is_non_perp_coin;
use crate::timeframe::Timeframe;

const SNAPSHOT_MAX_CANDLES: u64 = 260;
const MIN_PADDING_MS: u64 = 60 * 60 * 1000;
/// Recent-history window shown for an open position whose opening fills are not
/// in the loaded history (carried-in positions and synthetic current-position
/// trades). The true open time is unknown, so the chart shows recent price
/// action with the entry level marked rather than an entry → exit window.
const LIVE_POSITION_LOOKBACK_MS: u64 = 7 * 24 * 60 * 60 * 1000;
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

impl JournalSnapshotRequestSettings {
    fn into_context(
        self,
        trade_start_ms: u64,
        trade_end_ms: u64,
        is_open: bool,
    ) -> SnapshotRequestContext {
        SnapshotRequestContext {
            account_key: self.account_key,
            address: self.address,
            source: self.source,
            read_data_provider_generation: self.read_data_provider_generation,
            hydromancer_key_generation: self.hydromancer_key_generation,
            coverage: self.coverage,
            trade_start_ms,
            trade_end_ms,
            is_open,
        }
    }
}

pub fn initial_snapshot_request(
    settings: JournalSnapshotRequestSettings,
    trade: &AggregatedTrade,
) -> Result<JournalTradeSnapshotRequest, String> {
    validate_trade_history(trade)?;

    let coverage = settings.coverage;
    let now_ms = settings.now_ms;
    let trade_end_ms = trade.end_time.unwrap_or(now_ms).max(trade.start_time);
    let ladder_index = initial_ladder_index(trade.start_time, trade_end_ms, coverage);
    snapshot_request_for_ladder_index(
        settings.into_context(trade.start_time, trade_end_ms, trade.end_time.is_none()),
        trade,
        ladder_index,
    )
}

fn validate_trade_history(trade: &AggregatedTrade) -> Result<(), String> {
    if is_non_perp_coin(&trade.coin) {
        return Err("Chart snapshots are currently available for perp trades only.".to_string());
    }
    if !trade.basis_complete {
        return Err(
            "Snapshot unavailable because opening fills are outside loaded history.".to_string(),
        );
    }

    Ok(())
}

/// Validate that a trade can be charted as a live position: an open perp with a
/// known entry price but no usable opening fills.
fn validate_live_position(trade: &AggregatedTrade) -> Result<(), String> {
    if is_non_perp_coin(&trade.coin) {
        return Err("Chart snapshots are currently available for perp trades only.".to_string());
    }
    if trade.end_time.is_some() {
        return Err(
            "Live-position snapshots are only available while a position is open.".to_string(),
        );
    }
    if !(trade.avg_entry_price.is_finite() && trade.avg_entry_price > 0.0) {
        return Err(
            "Snapshot unavailable because no entry price is available for this position."
                .to_string(),
        );
    }
    Ok(())
}

/// Build a recent-history snapshot request for an open position whose opening
/// fills are unavailable (carried in from before the loaded history, or a
/// synthetic current-position trade). The true open time is unknown, so a fixed
/// recent window is charted with the entry level marked.
pub fn live_position_snapshot_request(
    settings: JournalSnapshotRequestSettings,
    trade: &AggregatedTrade,
) -> Result<JournalTradeSnapshotRequest, String> {
    validate_live_position(trade)?;
    let coverage = settings.coverage;
    let now_ms = settings.now_ms;
    let trade_start_ms = now_ms.saturating_sub(LIVE_POSITION_LOOKBACK_MS);
    let ladder_index = initial_ladder_index(trade_start_ms, now_ms, coverage);
    snapshot_request_for_ladder_index(
        settings.into_context(trade_start_ms, now_ms, true),
        trade,
        ladder_index,
    )
}

/// Live-position variant pinned to a specific timeframe (detail-view selector).
pub fn live_position_snapshot_request_for_timeframe(
    settings: JournalSnapshotRequestSettings,
    trade: &AggregatedTrade,
    timeframe: Timeframe,
) -> Result<JournalTradeSnapshotRequest, String> {
    validate_live_position(trade)?;
    let ladder_index = SNAPSHOT_LADDER
        .iter()
        .position(|candidate| *candidate == timeframe)
        .ok_or_else(|| "Unsupported snapshot timeframe.".to_string())?;
    // Cap the window so a fine timeframe (1m/5m) over the fixed lookback can't
    // request tens of thousands of candles; coarse timeframes still span the
    // full lookback. (The auto-ladder request is already bounded by
    // initial_ladder_index, but an explicitly pinned timeframe is not.)
    let now_ms = settings.now_ms;
    let lookback =
        LIVE_POSITION_LOOKBACK_MS.min(timeframe.duration_ms().saturating_mul(SNAPSHOT_MAX_CANDLES));
    let trade_start_ms = now_ms.saturating_sub(lookback);
    snapshot_request_for_ladder_index(
        settings.into_context(trade_start_ms, now_ms, true),
        trade,
        ladder_index,
    )
}

/// Build a snapshot request pinned to a specific timeframe (for the detail-view
/// 1m / 5m / 1h selector), rather than the auto-selected ladder rung.
pub fn snapshot_request_for_timeframe(
    settings: JournalSnapshotRequestSettings,
    trade: &AggregatedTrade,
    timeframe: Timeframe,
) -> Result<JournalTradeSnapshotRequest, String> {
    validate_trade_history(trade)?;

    let ladder_index = SNAPSHOT_LADDER
        .iter()
        .position(|candidate| *candidate == timeframe)
        .ok_or_else(|| "Unsupported snapshot timeframe.".to_string())?;
    let now_ms = settings.now_ms;
    let trade_end_ms = trade.end_time.unwrap_or(now_ms).max(trade.start_time);
    snapshot_request_for_ladder_index(
        settings.into_context(trade.start_time, trade_end_ms, trade.end_time.is_none()),
        trade,
        ladder_index,
    )
}

struct SnapshotRequestContext {
    account_key: Option<String>,
    address: String,
    source: ChartBackfillSource,
    read_data_provider_generation: u64,
    hydromancer_key_generation: u64,
    coverage: JournalSnapshotCoverage,
    trade_start_ms: u64,
    trade_end_ms: u64,
    is_open: bool,
}

fn snapshot_request_for_ladder_index(
    context: SnapshotRequestContext,
    trade: &AggregatedTrade,
    ladder_index: usize,
) -> Result<JournalTradeSnapshotRequest, String> {
    let timeframe = *SNAPSHOT_LADDER
        .get(ladder_index)
        .ok_or_else(|| "No candle timeframe available for snapshot.".to_string())?;
    let (start_ms, end_ms) = snapshot_bounds(
        context.trade_start_ms,
        context.trade_end_ms,
        context.is_open,
        timeframe,
        context.coverage,
    );

    Ok(JournalTradeSnapshotRequest {
        account_key: context.account_key,
        address: context.address,
        trade_id: trade.id.clone(),
        coin: trade.coin.clone(),
        source: context.source,
        read_data_provider_generation: context.read_data_provider_generation,
        hydromancer_key_generation: context.hydromancer_key_generation,
        coverage: context.coverage,
        timeframe,
        ladder_index,
        trade_start_ms: context.trade_start_ms,
        trade_end_ms: context.trade_end_ms,
        is_open: context.is_open,
        start_ms,
        end_ms,
    })
}

pub fn next_snapshot_request(
    request: &JournalTradeSnapshotRequest,
) -> Option<JournalTradeSnapshotRequest> {
    let next_ladder_index = request.ladder_index.saturating_add(1);
    let timeframe = *SNAPSHOT_LADDER.get(next_ladder_index)?;
    let (start_ms, end_ms) = snapshot_bounds(
        request.trade_start_ms,
        request.trade_end_ms,
        request.is_open,
        timeframe,
        request.coverage,
    );

    Some(JournalTradeSnapshotRequest {
        timeframe,
        ladder_index: next_ladder_index,
        start_ms,
        end_ms,
        ..request.clone()
    })
}

fn snapshot_bounds(
    trade_start_ms: u64,
    trade_end_ms: u64,
    is_open: bool,
    timeframe: Timeframe,
    coverage: JournalSnapshotCoverage,
) -> (u64, u64) {
    let duration = trade_end_ms.saturating_sub(trade_start_ms);
    let padding = snapshot_padding_ms(duration, timeframe, coverage);
    let start_ms = trade_start_ms.saturating_sub(padding);
    let end_ms = if is_open {
        trade_end_ms
    } else {
        trade_end_ms.saturating_add(padding)
    };
    (start_ms, end_ms)
}

fn initial_ladder_index(
    trade_start_ms: u64,
    trade_end_ms: u64,
    coverage: JournalSnapshotCoverage,
) -> usize {
    let duration = trade_end_ms.saturating_sub(trade_start_ms);
    SNAPSHOT_LADDER
        .iter()
        .position(|timeframe| {
            let padding = snapshot_padding_ms(duration, *timeframe, coverage);
            let padded = duration.saturating_add(padding.saturating_mul(2));
            padded.div_ceil(timeframe.duration_ms().max(1)) <= SNAPSHOT_MAX_CANDLES
        })
        .unwrap_or(SNAPSHOT_LADDER.len().saturating_sub(1))
}

fn snapshot_padding_ms(
    duration_ms: u64,
    timeframe: Timeframe,
    coverage: JournalSnapshotCoverage,
) -> u64 {
    (duration_ms.saturating_mul(3) / 4)
        .max(timeframe.duration_ms().saturating_mul(12))
        .max(MIN_PADDING_MS)
        .saturating_mul(coverage.padding_multiplier())
}

#[cfg(test)]
mod tests;
