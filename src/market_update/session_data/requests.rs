use crate::api;
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use crate::session_data_state::{SessionDataCandles, SessionDataId, SessionDataRequest};
use iced::Task;

pub(super) const DAY_MS: u64 = 86_400_000;
// 30m candles align exactly with every market session boundary (all opens fall
// on :00 or :30 UTC year-round), unlike coarser intervals.
const INTRADAY_INTERVAL: &str = "30m";
pub(super) const INTRADAY_CANDLE_MS: u64 = 30 * 60_000;
// Hyperliquid candleSnapshot responses cap out around 5000 candles; chunk
// requests to stay safely below that so long lookbacks keep full coverage.
pub(super) const INTRADAY_MAX_CANDLES_PER_REQUEST: u64 = 4_000;

impl TradingTerminal {
    pub(crate) fn request_session_data_refresh_all(&mut self, force: bool) -> Task<Message> {
        let ids = self.session_data.keys().copied().collect::<Vec<_>>();
        Task::batch(
            ids.into_iter()
                .map(|id| self.request_session_data_refresh(id, force)),
        )
    }

    pub(crate) fn request_session_data_refresh(
        &mut self,
        id: SessionDataId,
        force: bool,
    ) -> Task<Message> {
        let now_ms = Self::now_ms();
        let Some(instance) = self.session_data.get(&id) else {
            return Task::none();
        };

        if instance.loading && !force {
            return Task::none();
        }

        let symbol = instance.symbol.as_str();
        let lookback = instance.lookback;

        if symbol.trim().is_empty() {
            if let Some(instance) = self.session_data.get_mut(&id) {
                instance.loading = false;
                instance.error = Some("Select a symbol".to_string());
            }
            return Task::none();
        }

        if self.symbol_key_is_hidden(symbol) {
            if let Some(instance) = self.session_data.get_mut(&id) {
                instance.loading = false;
                instance.error = Some("Ticker is hidden in Settings > Risk".to_string());
            }
            return Task::none();
        }

        if !self.session_data_symbol_is_supported(symbol) {
            if let Some(instance) = self.session_data.get_mut(&id) {
                instance.loading = false;
                instance.error =
                    Some("Session Data is available for perp and spot candle symbols".to_string());
            }
            return Task::none();
        }

        if instance.loading
            && instance
                .pending_request
                .as_ref()
                .is_some_and(|pending| pending.matches_refresh_target(id, symbol, lookback))
        {
            return Task::none();
        }

        let request = SessionDataRequest {
            id,
            symbol: symbol.to_string(),
            lookback,
            requested_at_ms: now_ms,
        };
        if let Some(instance) = self.session_data.get_mut(&id) {
            instance.loading = true;
            instance.error = None;
            instance.pending_request = Some(request.clone());
        }

        Self::fetch_session_data_task(request, now_ms)
    }

    fn fetch_session_data_task(request: SessionDataRequest, now_ms: u64) -> Task<Message> {
        let start_time = now_ms.saturating_sub(request.lookback.days().saturating_mul(DAY_MS));
        let symbol = request.symbol.clone();
        Task::perform(
            fetch_session_data_candles(symbol, start_time, now_ms),
            move |result| Message::SessionDataCandlesLoaded(request, result),
        )
    }

    pub(super) fn apply_session_data_candles_loaded(
        &mut self,
        request: SessionDataRequest,
        result: Result<SessionDataCandles, String>,
    ) -> Task<Message> {
        let Some(instance) = self.session_data.get_mut(&request.id) else {
            return Task::none();
        };

        let is_current = instance
            .pending_request
            .as_ref()
            .is_some_and(|pending| pending == &request);
        if !is_current {
            return Task::none();
        }

        instance.loading = false;
        instance.pending_request = None;
        match result {
            Ok(candles) => {
                let completed_through_ms = Self::now_ms();
                instance.last_fetch_ms = Some(completed_through_ms);
                instance.apply_candles(candles, completed_through_ms);
                instance.error = if instance.bars.is_empty() {
                    Some("No completed session history available for this symbol".to_string())
                } else {
                    None
                };
            }
            Err(error) => {
                instance.error = Some(redact_sensitive_response_text(&error));
            }
        }
        Task::none()
    }
}

async fn fetch_session_data_candles(
    symbol: String,
    start_time: u64,
    end_time: u64,
) -> Result<SessionDataCandles, String> {
    let daily = api::fetch_candles(symbol.clone(), "1d".to_string(), start_time, end_time).await?;
    let mut intraday = Vec::new();
    for (chunk_start, chunk_end) in intraday_chunk_ranges(start_time, end_time) {
        let chunk = api::fetch_candles(
            symbol.clone(),
            INTRADAY_INTERVAL.to_string(),
            chunk_start,
            chunk_end,
        )
        .await?;
        intraday.extend(chunk);
    }
    Ok(SessionDataCandles { daily, intraday })
}

pub(super) fn intraday_chunk_ranges(start_ms: u64, end_ms: u64) -> Vec<(u64, u64)> {
    if end_ms <= start_ms {
        return Vec::new();
    }
    let chunk_ms = INTRADAY_CANDLE_MS.saturating_mul(INTRADAY_MAX_CANDLES_PER_REQUEST);
    let mut ranges = Vec::new();
    let mut chunk_start = start_ms;
    while chunk_start < end_ms {
        let chunk_end = chunk_start.saturating_add(chunk_ms).min(end_ms);
        ranges.push((chunk_start, chunk_end));
        chunk_start = chunk_end;
    }
    ranges
}
