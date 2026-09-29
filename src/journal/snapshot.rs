use super::{AggregatedTrade, JournalAttributedFillRole, JournalTradeDetails};
use crate::api::Candle;
use crate::chart::TradeMarker;
use crate::config::ChartBackfillSource;
use crate::timeframe::Timeframe;
use std::fmt;

mod metrics;
mod requests;

use metrics::journal_snapshot_metrics;
pub use requests::{
    initial_snapshot_request, live_position_snapshot_request,
    live_position_snapshot_request_for_timeframe, next_snapshot_request,
    snapshot_request_for_timeframe,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JournalSnapshotCoverage {
    #[default]
    TwoX,
    FourX,
    EightX,
}

impl JournalSnapshotCoverage {
    pub const OPTIONS: [Self; 3] = [Self::TwoX, Self::FourX, Self::EightX];

    pub fn label(self) -> &'static str {
        match self {
            Self::TwoX => "2x",
            Self::FourX => "4x",
            Self::EightX => "8x",
        }
    }

    fn padding_multiplier(self) -> u64 {
        match self {
            Self::TwoX => 2,
            Self::FourX => 4,
            Self::EightX => 8,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct JournalTradeSnapshotRequest {
    pub account_key: Option<String>,
    pub address: String,
    pub trade_id: String,
    pub coin: String,
    pub source: ChartBackfillSource,
    pub read_data_provider_generation: u64,
    pub hydromancer_key_generation: u64,
    pub coverage: JournalSnapshotCoverage,
    pub timeframe: Timeframe,
    pub ladder_index: usize,
    pub trade_start_ms: u64,
    pub trade_end_ms: u64,
    pub is_open: bool,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Clone)]
pub struct JournalSnapshotRequestSettings {
    pub account_key: Option<String>,
    pub address: String,
    pub source: ChartBackfillSource,
    pub read_data_provider_generation: u64,
    pub hydromancer_key_generation: u64,
    pub coverage: JournalSnapshotCoverage,
    pub now_ms: u64,
}

impl fmt::Debug for JournalTradeSnapshotRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JournalTradeSnapshotRequest")
            .field(
                "account_key",
                &self.account_key.as_ref().map(|_| "<redacted>"),
            )
            .field("address", &format_args!("<redacted>"))
            .field("trade_id", &self.trade_id)
            .field("coin", &self.coin)
            .field("source", &self.source)
            .field(
                "read_data_provider_generation",
                &self.read_data_provider_generation,
            )
            .field(
                "hydromancer_key_generation",
                &self.hydromancer_key_generation,
            )
            .field("coverage", &self.coverage)
            .field("timeframe", &self.timeframe)
            .field("ladder_index", &self.ladder_index)
            .field("trade_start_ms", &self.trade_start_ms)
            .field("trade_end_ms", &self.trade_end_ms)
            .field("is_open", &self.is_open)
            .field("start_ms", &self.start_ms)
            .field("end_ms", &self.end_ms)
            .finish()
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct JournalTradeSnapshot {
    pub trade_id: String,
    pub coin: String,
    pub source: ChartBackfillSource,
    pub coverage: JournalSnapshotCoverage,
    pub timeframe: Timeframe,
    pub trade_start_ms: u64,
    pub trade_end_ms: u64,
    pub is_open: bool,
    /// An open position charted against its entry level over a recent window
    /// because its opening fills are unavailable (no fill markers, no open
    /// boundary; a horizontal entry guide is drawn instead).
    pub live_position: bool,
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

pub fn build_journal_trade_snapshot(
    request: &JournalTradeSnapshotRequest,
    trade: &AggregatedTrade,
    details: Option<&JournalTradeDetails>,
    candles: Vec<Candle>,
) -> Result<JournalTradeSnapshot, String> {
    let metrics = journal_snapshot_metrics(request, trade, details, &candles)?;
    let markers = details
        .map(snapshot_markers_for_details)
        .unwrap_or_default();
    // An open position with no usable opening fills is charted against its
    // entry level over a recent window rather than as an entry → exit span.
    let live_position = trade.end_time.is_none()
        && details.is_none_or(|details| details.attributed_fills.is_empty());

    Ok(JournalTradeSnapshot {
        trade_id: request.trade_id.clone(),
        coin: request.coin.clone(),
        source: request.source,
        coverage: request.coverage,
        timeframe: request.timeframe,
        trade_start_ms: request.trade_start_ms,
        trade_end_ms: request.trade_end_ms,
        is_open: request.is_open,
        live_position,
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
        coverage: JournalSnapshotCoverage::default(),
        timeframe: Timeframe::M1,
        trade_start_ms: trade.start_time,
        trade_end_ms,
        is_open: trade.end_time.is_none(),
        live_position: false,
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

#[cfg(test)]
mod tests;
