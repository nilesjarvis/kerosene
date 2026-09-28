use super::SessionDataId;
use super::model::{
    MarketSessionSummary, SessionDataCandles, SessionDataLookback, SessionDataRequest,
    SessionReturnBar, SessionWeekdaySummary,
};
use super::returns::{
    completed_session_return_bars, empty_market_session_summaries, empty_weekday_summaries,
    market_session_return_bars, market_session_summaries, weekday_summaries,
};
use crate::api::Candle;

#[derive(Debug, Clone)]
pub(crate) struct SessionDataInstance {
    pub(crate) id: SessionDataId,
    pub(crate) symbol: String,
    pub(crate) search_query: String,
    pub(crate) symbol_picker_open: bool,
    pub(crate) lookback: SessionDataLookback,
    pub(crate) candles: Vec<Candle>,
    pub(crate) bars: Vec<SessionReturnBar>,
    pub(crate) weekday_summaries: Vec<SessionWeekdaySummary>,
    pub(crate) session_summaries: Vec<MarketSessionSummary>,
    pub(crate) loading: bool,
    pub(crate) error: Option<String>,
    pub(crate) last_fetch_ms: Option<u64>,
    pub(crate) pending_request: Option<SessionDataRequest>,
}

impl SessionDataInstance {
    pub(crate) fn new(id: SessionDataId, symbol: String, lookback: SessionDataLookback) -> Self {
        Self {
            id,
            symbol,
            search_query: String::new(),
            symbol_picker_open: false,
            lookback,
            candles: Vec::new(),
            bars: Vec::new(),
            weekday_summaries: empty_weekday_summaries(),
            session_summaries: empty_market_session_summaries(),
            loading: false,
            error: None,
            last_fetch_ms: None,
            pending_request: None,
        }
    }

    pub(crate) fn clear_history(&mut self) {
        self.candles.clear();
        self.bars.clear();
        self.weekday_summaries = empty_weekday_summaries();
        self.session_summaries = empty_market_session_summaries();
        self.loading = false;
        self.error = None;
        self.last_fetch_ms = None;
        self.pending_request = None;
    }

    pub(crate) fn apply_candles(&mut self, candles: SessionDataCandles, completed_through_ms: u64) {
        self.bars = completed_session_return_bars(&candles.daily, completed_through_ms);
        self.weekday_summaries = weekday_summaries(&self.bars);
        let session_bars = market_session_return_bars(&candles.intraday, completed_through_ms);
        self.session_summaries = market_session_summaries(&session_bars);
        self.candles = candles.daily;
    }
}
