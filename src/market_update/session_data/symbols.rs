use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::session_data_state::SessionDataId;
use iced::Task;

// ---------------------------------------------------------------------------
// Session Data Symbol Selection
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn select_session_data_symbol(
        &mut self,
        id: SessionDataId,
        symbol: String,
    ) -> Task<Message> {
        let Some(symbol) = self.resolved_session_data_symbol_key(&symbol) else {
            if let Some(instance) = self.session_data.get_mut(&id) {
                instance.symbol_picker_open = false;
                instance.error =
                    Some("Session Data is available for perp and spot candle symbols".to_string());
                instance.loading = false;
            }
            return Task::none();
        };

        if self.symbol_key_is_hidden(symbol) {
            if let Some(instance) = self.session_data.get_mut(&id) {
                instance.symbol_picker_open = false;
                instance.error = Some("Ticker is hidden in Settings > Risk".to_string());
                instance.loading = false;
            }
            return Task::none();
        }

        let symbol = symbol.to_string();
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

    pub(crate) fn reconcile_session_data_symbols(&mut self) -> Task<Message> {
        if self.session_data.is_empty() {
            return Task::none();
        }

        let ids = self.session_data.keys().copied().collect::<Vec<_>>();
        let mut refresh_ids = Vec::new();

        for id in ids {
            let Some(current_symbol) = self.session_data.get(&id).map(|inst| inst.symbol.as_str())
            else {
                continue;
            };
            let retained_symbol = self.retained_session_data_symbol(current_symbol);
            if retained_symbol == current_symbol {
                continue;
            }

            if let Some(instance) = self.session_data.get_mut(&id) {
                instance.symbol = retained_symbol;
                instance.search_query.clear();
                instance.symbol_picker_open = false;
                instance.clear_history();
                refresh_ids.push(id);
            }
        }

        if !refresh_ids.is_empty() {
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
            && !self.symbol_key_is_hidden(candidate_key)
        {
            return candidate_key.to_string();
        }

        if let Some(active_key) = self.resolved_session_data_symbol_key(&self.active_symbol)
            && !self.symbol_key_is_hidden(active_key)
        {
            return active_key.to_string();
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

    pub(crate) fn retained_session_data_symbol(&self, candidate: &str) -> String {
        let candidate = candidate.trim();
        if let Some(candidate_key) = self.resolved_session_data_symbol_key(candidate)
            && !self.is_ticker_muted(candidate_key)
        {
            return candidate_key.to_string();
        }

        self.visible_session_data_symbol("")
    }

    pub(crate) fn session_data_symbol_is_supported(&self, symbol: &str) -> bool {
        self.resolved_session_data_symbol_key(symbol).is_some()
    }

    pub(super) fn resolved_session_data_symbol_key<'a>(
        &'a self,
        symbol: &'a str,
    ) -> Option<&'a str> {
        let symbol = symbol.trim();
        if symbol.is_empty() {
            return None;
        }

        if self.exchange_symbols.is_empty() {
            return (!symbol.starts_with('#')).then_some(symbol);
        }

        self.resolve_exchange_symbol_by_key_or_ticker(symbol)
            .filter(|candidate| {
                matches!(candidate.market_type, MarketType::Perp | MarketType::Spot)
            })
            .map(|candidate| candidate.key.as_str())
    }
}
