use crate::api::{self, MarketType};
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::session_data_state::{
    SessionDataCandles, SessionDataId, SessionDataInstance, SessionDataLookback, SessionDataRequest,
};
use iced::Task;

const DAY_MS: u64 = 86_400_000;
// 30m candles align exactly with every market session boundary (all opens fall
// on :00 or :30 UTC year-round), unlike coarser intervals.
const INTRADAY_INTERVAL: &str = "30m";
const INTRADAY_CANDLE_MS: u64 = 30 * 60_000;
// Hyperliquid candleSnapshot responses cap out around 5000 candles; chunk
// requests to stay safely below that so long lookbacks keep full coverage.
const INTRADAY_MAX_CANDLES_PER_REQUEST: u64 = 4_000;

impl TradingTerminal {
    pub(super) fn update_session_data_market(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddSessionDataPane => self.add_session_data_pane(),
            Message::SessionDataSearchChanged(id, query) => {
                if let Some(instance) = self.session_data.get_mut(&id) {
                    instance.search_query = query;
                }
                Task::none()
            }
            Message::ToggleSessionDataSymbolPicker(id) => {
                if let Some(instance) = self.session_data.get_mut(&id) {
                    instance.symbol_picker_open = !instance.symbol_picker_open;
                    if instance.symbol_picker_open {
                        instance.search_query.clear();
                    }
                }
                Task::none()
            }
            Message::SessionDataSymbolSelected(id, symbol) => {
                self.select_session_data_symbol(id, symbol)
            }
            Message::SessionDataLookbackChanged(id, lookback) => {
                self.set_session_data_lookback(id, lookback)
            }
            Message::RefreshSessionData(id) => self.request_session_data_refresh(id, true),
            Message::SessionDataCandlesLoaded(request, result) => {
                self.apply_session_data_candles_loaded(request, result)
            }
            _ => Task::none(),
        }
    }

    fn add_session_data_pane(&mut self) -> Task<Message> {
        self.add_widget_menu_open = false;
        let Some(focus) = self.add_target_pane() else {
            self.push_toast(
                "Could not add Session Data: no pane is available".to_string(),
                true,
            );
            return Task::none();
        };

        let id = self.next_session_data_id;
        self.next_session_data_id = self.next_session_data_id.saturating_add(1);
        let symbol = self.visible_session_data_symbol(&self.active_symbol);
        self.session_data.insert(
            id,
            SessionDataInstance::new(id, symbol, SessionDataLookback::default()),
        );

        if self
            .add_pane_to_target(
                self.add_widget_axis(),
                focus,
                PaneKind::SessionData(id),
                "Session Data",
            )
            .is_none()
        {
            self.session_data.remove(&id);
            return Task::none();
        }

        self.request_session_data_refresh(id, true)
    }

    fn select_session_data_symbol(&mut self, id: SessionDataId, symbol: String) -> Task<Message> {
        let Some(symbol) = self.resolved_session_data_symbol_key(&symbol) else {
            if let Some(instance) = self.session_data.get_mut(&id) {
                instance.symbol_picker_open = false;
                instance.error =
                    Some("Session Data is available for perp and spot candle symbols".to_string());
                instance.loading = false;
            }
            return Task::none();
        };

        if self.symbol_key_is_hidden(&symbol) {
            if let Some(instance) = self.session_data.get_mut(&id) {
                instance.symbol_picker_open = false;
                instance.error = Some("Ticker is hidden in Settings > Risk".to_string());
                instance.loading = false;
            }
            return Task::none();
        }

        if let Some(instance) = self.session_data.get_mut(&id) {
            if instance.symbol == symbol {
                instance.search_query.clear();
                instance.symbol_picker_open = false;
                return Task::none();
            }
            instance.symbol = symbol;
            instance.search_query.clear();
            instance.symbol_picker_open = false;
            instance.clear_history();
        }
        self.persist_config();
        self.request_session_data_refresh(id, true)
    }

    fn set_session_data_lookback(
        &mut self,
        id: SessionDataId,
        lookback: SessionDataLookback,
    ) -> Task<Message> {
        if let Some(instance) = self.session_data.get_mut(&id) {
            if instance.lookback == lookback {
                return Task::none();
            }
            instance.lookback = lookback;
            instance.error = None;
        }
        self.persist_config();
        self.request_session_data_refresh(id, true)
    }

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

        let symbol = instance.symbol.clone();
        let lookback = instance.lookback;

        if symbol.trim().is_empty() {
            if let Some(instance) = self.session_data.get_mut(&id) {
                instance.loading = false;
                instance.error = Some("Select a symbol".to_string());
            }
            return Task::none();
        }

        if self.symbol_key_is_hidden(&symbol) {
            if let Some(instance) = self.session_data.get_mut(&id) {
                instance.loading = false;
                instance.error = Some("Ticker is hidden in Settings > Risk".to_string());
            }
            return Task::none();
        }

        if !self.session_data_symbol_is_supported(&symbol) {
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
                .is_some_and(|pending| pending.matches_refresh_target(id, &symbol, lookback))
        {
            return Task::none();
        }

        let request = SessionDataRequest {
            id,
            symbol,
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
            async move { fetch_session_data_candles(symbol, start_time, now_ms).await },
            move |result| Message::SessionDataCandlesLoaded(request.clone(), result),
        )
    }

    fn apply_session_data_candles_loaded(
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

    pub(crate) fn reconcile_session_data_symbols(&mut self) -> Task<Message> {
        if self.session_data.is_empty() {
            return Task::none();
        }

        let ids = self.session_data.keys().copied().collect::<Vec<_>>();
        let mut refresh_ids = Vec::new();
        let mut changed = false;

        for id in ids {
            let Some(current_symbol) = self.session_data.get(&id).map(|inst| inst.symbol.clone())
            else {
                continue;
            };
            let visible_symbol = self.visible_session_data_symbol(&current_symbol);
            if visible_symbol == current_symbol {
                continue;
            }

            if let Some(instance) = self.session_data.get_mut(&id) {
                instance.symbol = visible_symbol;
                instance.search_query.clear();
                instance.symbol_picker_open = false;
                instance.clear_history();
                refresh_ids.push(id);
                changed = true;
            }
        }

        if changed {
            self.persist_config();
        }

        Task::batch(
            refresh_ids
                .into_iter()
                .map(|id| self.request_session_data_refresh(id, true)),
        )
    }

    pub(crate) fn visible_session_data_symbol(&self, candidate: &str) -> String {
        let candidate = candidate.trim();
        if let Some(candidate_key) = self.resolved_session_data_symbol_key(candidate)
            && !self.symbol_key_is_hidden(&candidate_key)
        {
            return candidate_key;
        }

        if let Some(active_key) = self.resolved_session_data_symbol_key(&self.active_symbol)
            && !self.symbol_key_is_hidden(&active_key)
        {
            return active_key;
        }

        self.exchange_symbols
            .iter()
            .find(|symbol| {
                matches!(symbol.market_type, MarketType::Perp | MarketType::Spot)
                    && !self.exchange_symbol_is_hidden(symbol)
            })
            .map(|symbol| symbol.key.clone())
            .or_else(|| self.fallback_unmuted_symbol_key())
            .unwrap_or_else(|| "HYPE".to_string())
    }

    pub(crate) fn session_data_symbol_is_supported(&self, symbol: &str) -> bool {
        self.resolved_session_data_symbol_key(symbol).is_some()
    }

    fn resolved_session_data_symbol_key(&self, symbol: &str) -> Option<String> {
        let symbol = symbol.trim();
        if symbol.is_empty() {
            return None;
        }

        if self.exchange_symbols.is_empty() {
            return (!symbol.starts_with('#')).then(|| symbol.to_string());
        }

        self.exchange_symbol_for_key(symbol)
            .or_else(|| {
                self.exchange_symbols.iter().find(|candidate| {
                    candidate.ticker == symbol && candidate.market_type == MarketType::Perp
                })
            })
            .or_else(|| {
                self.exchange_symbols
                    .iter()
                    .find(|candidate| candidate.ticker == symbol)
            })
            .filter(|candidate| {
                matches!(candidate.market_type, MarketType::Perp | MarketType::Spot)
            })
            .map(|candidate| candidate.key.clone())
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

fn intraday_chunk_ranges(start_ms: u64, end_ms: u64) -> Vec<(u64, u64)> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ExchangeSymbol, MarketType};

    fn exchange_symbol(key: &str, ticker: &str, market_type: MarketType) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: ticker.to_string(),
            category: "crypto".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 2,
            max_leverage: 50,
            only_isolated: false,
            market_type,
            outcome: None,
        }
    }

    #[test]
    fn session_data_error_redacts_state_error() {
        let mut terminal = TradingTerminal::boot().0;
        let request = SessionDataRequest {
            id: 7,
            symbol: "BTC".to_string(),
            lookback: SessionDataLookback::FourWeeks,
            requested_at_ms: 123,
        };
        terminal.session_data.insert(
            7,
            SessionDataInstance::new(7, "BTC".to_string(), SessionDataLookback::FourWeeks),
        );
        {
            let instance = terminal.session_data.get_mut(&7).expect("session data");
            instance.loading = true;
            instance.pending_request = Some(request.clone());
        }

        let _task = terminal.apply_session_data_candles_loaded(
            request,
            Err("session fetch failed: api_key=session-secret".to_string()),
        );

        let error = terminal
            .session_data
            .get(&7)
            .and_then(|instance| instance.error.as_deref())
            .expect("state error");
        assert!(error.contains("api_key=<redacted>"));
        assert!(!error.contains("session-secret"));
    }

    #[test]
    fn reconcile_session_data_symbols_replaces_unsupported_saved_symbol() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.active_symbol = "HYPE".to_string();
        terminal.exchange_symbols = vec![
            exchange_symbol("HYPE", "HYPE", MarketType::Perp),
            exchange_symbol("BTC", "BTC", MarketType::Perp),
        ];
        terminal.session_data.insert(
            7,
            SessionDataInstance::new(
                7,
                "NOT_A_MARKET".to_string(),
                SessionDataLookback::FourWeeks,
            ),
        );
        let old_pending = SessionDataRequest {
            id: 7,
            symbol: "NOT_A_MARKET".to_string(),
            lookback: SessionDataLookback::FourWeeks,
            requested_at_ms: 123,
        };
        {
            let instance = terminal.session_data.get_mut(&7).expect("session data");
            instance.loading = true;
            instance.pending_request = Some(old_pending);
        }

        let _task = terminal.reconcile_session_data_symbols();

        let instance = terminal.session_data.get(&7).expect("session data");
        assert_eq!(instance.symbol, "HYPE");
        assert!(instance.loading);
        let request = instance.pending_request.as_ref().expect("pending request");
        assert_eq!(request.symbol, "HYPE");
        assert_ne!(request.requested_at_ms, 123);
        assert!(instance.error.is_none());
    }

    #[test]
    fn forced_refresh_coalesces_identical_pending_request() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.session_data.insert(
            7,
            SessionDataInstance::new(7, "HYPE".to_string(), SessionDataLookback::FourWeeks),
        );
        let pending = SessionDataRequest {
            id: 7,
            symbol: "HYPE".to_string(),
            lookback: SessionDataLookback::FourWeeks,
            requested_at_ms: 123,
        };
        {
            let instance = terminal.session_data.get_mut(&7).expect("session data");
            instance.loading = true;
            instance.pending_request = Some(pending.clone());
        }

        let _task = terminal.request_session_data_refresh(7, true);

        let instance = terminal.session_data.get(&7).expect("session data");
        assert!(instance.loading);
        assert_eq!(instance.pending_request.as_ref(), Some(&pending));
    }

    #[test]
    fn forced_refresh_replaces_pending_request_when_target_changes() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.session_data.insert(
            7,
            SessionDataInstance::new(7, "BTC".to_string(), SessionDataLookback::FourWeeks),
        );
        let pending = SessionDataRequest {
            id: 7,
            symbol: "HYPE".to_string(),
            lookback: SessionDataLookback::FourWeeks,
            requested_at_ms: 123,
        };
        {
            let instance = terminal.session_data.get_mut(&7).expect("session data");
            instance.loading = true;
            instance.pending_request = Some(pending);
        }

        let _task = terminal.request_session_data_refresh(7, true);

        let request = terminal
            .session_data
            .get(&7)
            .and_then(|instance| instance.pending_request.as_ref())
            .expect("replacement request");
        assert_eq!(request.id, 7);
        assert_eq!(request.symbol, "BTC");
        assert_eq!(request.lookback, SessionDataLookback::FourWeeks);
        assert_ne!(request.requested_at_ms, 123);
    }

    #[test]
    fn forced_refresh_replaces_pending_request_when_lookback_changes() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.session_data.insert(
            7,
            SessionDataInstance::new(7, "HYPE".to_string(), SessionDataLookback::EightWeeks),
        );
        let pending = SessionDataRequest {
            id: 7,
            symbol: "HYPE".to_string(),
            lookback: SessionDataLookback::FourWeeks,
            requested_at_ms: 123,
        };
        {
            let instance = terminal.session_data.get_mut(&7).expect("session data");
            instance.loading = true;
            instance.pending_request = Some(pending);
        }

        let _task = terminal.request_session_data_refresh(7, true);

        let request = terminal
            .session_data
            .get(&7)
            .and_then(|instance| instance.pending_request.as_ref())
            .expect("replacement request");
        assert_eq!(request.id, 7);
        assert_eq!(request.symbol, "HYPE");
        assert_eq!(request.lookback, SessionDataLookback::EightWeeks);
        assert_ne!(request.requested_at_ms, 123);
    }

    #[test]
    fn intraday_chunk_ranges_tile_long_lookbacks_without_gaps() {
        let start = 1_704_067_200_000;
        let end = start + 365 * DAY_MS;
        let chunk_ms = INTRADAY_CANDLE_MS * INTRADAY_MAX_CANDLES_PER_REQUEST;

        let ranges = intraday_chunk_ranges(start, end);

        assert_eq!(ranges.first().map(|range| range.0), Some(start));
        assert_eq!(ranges.last().map(|range| range.1), Some(end));
        for pair in ranges.windows(2) {
            assert_eq!(pair[0].1, pair[1].0);
        }
        for (chunk_start, chunk_end) in &ranges {
            assert!(chunk_end - chunk_start <= chunk_ms);
        }
        assert_eq!(ranges.len(), 5);
    }

    #[test]
    fn intraday_chunk_ranges_use_single_request_for_short_lookbacks() {
        let start = 1_704_067_200_000;
        let end = start + 28 * DAY_MS;

        assert_eq!(intraday_chunk_ranges(start, end), vec![(start, end)]);
        assert!(intraday_chunk_ranges(end, start).is_empty());
    }

    #[test]
    fn resolved_session_data_symbol_key_prefers_exact_key_over_shared_ticker() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![
            exchange_symbol("@107", "HYPE", MarketType::Spot),
            exchange_symbol("HYPE", "HYPE", MarketType::Perp),
        ];

        assert_eq!(
            terminal.resolved_session_data_symbol_key("HYPE").as_deref(),
            Some("HYPE")
        );
    }
}
