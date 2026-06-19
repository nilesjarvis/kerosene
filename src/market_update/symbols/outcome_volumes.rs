use crate::api::{self, MarketType};
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use iced::Task;
use std::collections::{BTreeSet, HashMap, HashSet};

impl TradingTerminal {
    pub(super) fn request_outcome_volume_refresh(&mut self) -> Task<Message> {
        let symbols = self.current_outcome_volume_symbols();
        self.outcome_volumes_request_id = self.outcome_volumes_request_id.saturating_add(1);
        let request_id = self.outcome_volumes_request_id;

        self.outcome_volumes_error = None;
        if symbols.is_empty() {
            self.outcome_volumes_24h.clear();
            self.outcome_volumes_loading = false;
            return Task::none();
        }

        self.outcome_volumes_loading = true;
        let requested_symbols = symbols.clone();
        Task::perform(api::fetch_outcome_volumes_24h(symbols), move |result| {
            Message::OutcomeVolumesLoaded(request_id, requested_symbols.clone(), result)
        })
    }

    fn current_outcome_volume_symbols(&self) -> Vec<String> {
        self.exchange_symbols
            .iter()
            .filter(|symbol| symbol.market_type == MarketType::Outcome)
            .filter(|symbol| symbol.is_user_selectable_market())
            .filter(|symbol| !self.exchange_symbol_is_hidden(symbol))
            .map(|symbol| symbol.key.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub(super) fn apply_outcome_volumes_loaded(
        &mut self,
        request_id: u64,
        requested_symbols: Vec<String>,
        result: Result<HashMap<String, api::OutcomeVolume24h>, String>,
    ) -> Task<Message> {
        if request_id != self.outcome_volumes_request_id {
            return Task::none();
        }

        self.outcome_volumes_request_id = self.outcome_volumes_request_id.saturating_add(1);
        self.outcome_volumes_loading = false;
        match result {
            Ok(mut volumes) => {
                let requested_symbols: HashSet<String> = requested_symbols.into_iter().collect();
                let current_symbols: HashSet<String> =
                    self.current_outcome_volume_symbols().into_iter().collect();
                volumes.retain(|symbol, _| {
                    requested_symbols.contains(symbol) && current_symbols.contains(symbol)
                });
                self.outcome_volumes_24h = volumes;
                self.outcome_volumes_error = None;
            }
            Err(error) => {
                self.outcome_volumes_error = Some(redact_sensitive_response_text(&error));
            }
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ExchangeSymbol, OutcomeSymbolInfo, OutcomeVolume24h};

    #[test]
    fn stale_outcome_volume_result_after_newer_request_is_ignored() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![outcome_symbol("#1"), outcome_symbol("#2")];
        let _ = terminal.request_outcome_volume_refresh();
        let stale_request_id = terminal.outcome_volumes_request_id;
        let _ = terminal.request_outcome_volume_refresh();
        let current_request_id = terminal.outcome_volumes_request_id;

        let _ = terminal.apply_outcome_volumes_loaded(
            stale_request_id,
            vec!["#1".to_string(), "#2".to_string()],
            Ok(HashMap::from([("#1".to_string(), volume(1.0))])),
        );

        assert!(terminal.outcome_volumes_loading);
        assert!(terminal.outcome_volumes_24h.is_empty());

        let _ = terminal.apply_outcome_volumes_loaded(
            current_request_id,
            vec!["#1".to_string(), "#2".to_string()],
            Ok(HashMap::from([("#2".to_string(), volume(2.0))])),
        );

        assert!(!terminal.outcome_volumes_loading);
        assert_eq!(
            terminal
                .outcome_volumes_24h
                .get("#2")
                .map(|volume| volume.contract),
            Some(2.0)
        );
        assert!(!terminal.outcome_volumes_24h.contains_key("#1"));
    }

    #[test]
    fn empty_outcome_universe_invalidates_in_flight_volume_request() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![outcome_symbol("#1")];
        let _ = terminal.request_outcome_volume_refresh();
        let stale_request_id = terminal.outcome_volumes_request_id;

        terminal.exchange_symbols.clear();
        let _ = terminal.request_outcome_volume_refresh();
        let _ = terminal.apply_outcome_volumes_loaded(
            stale_request_id,
            vec!["#1".to_string()],
            Ok(HashMap::from([("#1".to_string(), volume(1.0))])),
        );

        assert!(!terminal.outcome_volumes_loading);
        assert!(terminal.outcome_volumes_24h.is_empty());
    }

    #[test]
    fn outcome_volume_result_keeps_only_requested_current_symbols() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![outcome_symbol("#1"), outcome_symbol("#2")];
        let _ = terminal.request_outcome_volume_refresh();
        let request_id = terminal.outcome_volumes_request_id;

        let _ = terminal.apply_outcome_volumes_loaded(
            request_id,
            vec!["#1".to_string()],
            Ok(HashMap::from([
                ("#1".to_string(), volume(1.0)),
                ("#2".to_string(), volume(2.0)),
                ("#3".to_string(), volume(3.0)),
            ])),
        );

        assert_eq!(terminal.outcome_volumes_24h.len(), 1);
        assert_eq!(
            terminal
                .outcome_volumes_24h
                .get("#1")
                .map(|volume| volume.contract),
            Some(1.0)
        );
    }

    #[test]
    fn outcome_volume_error_redacts_sensitive_text() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.exchange_symbols = vec![outcome_symbol("#1")];
        let _ = terminal.request_outcome_volume_refresh();
        let request_id = terminal.outcome_volumes_request_id;

        let _ = terminal.apply_outcome_volumes_loaded(
            request_id,
            vec!["#1".to_string()],
            Err("outcome volume fetch failed: api_key=super-secret".to_string()),
        );

        assert!(!terminal.outcome_volumes_loading);
        let error = terminal.outcome_volumes_error.as_ref().expect("error");
        assert!(error.contains("api_key=<redacted>"));
        assert!(!error.contains("super-secret"));
    }

    fn outcome_symbol(key: &str) -> ExchangeSymbol {
        ExchangeSymbol {
            key: key.to_string(),
            ticker: key.to_string(),
            category: "outcome".to_string(),
            display_name: None,
            keywords: Vec::new(),
            asset_index: 0,
            collateral_token: None,
            sz_decimals: 0,
            max_leverage: 1,
            only_isolated: false,
            market_type: MarketType::Outcome,
            outcome: Some(OutcomeSymbolInfo {
                outcome_id: 1,
                question_id: None,
                question_name: None,
                question_description: None,
                question_class: None,
                question_underlying: None,
                question_expiry: None,
                question_price_thresholds: Vec::new(),
                question_period: None,
                question_named_outcomes: Vec::new(),
                question_settled_named_outcomes: Vec::new(),
                question_fallback_outcome: None,
                bucket_index: None,
                is_question_fallback: false,
                side_index: 0,
                side_name: "Yes".to_string(),
                outcome_name: "Yes".to_string(),
                description: "Outcome".to_string(),
                class: None,
                underlying: None,
                expiry: None,
                target_price: None,
                period: None,
                quote_symbol: "USDC".to_string(),
                quote_token_index: None,
                encoding: 0,
            }),
        }
    }

    fn volume(contract: f64) -> OutcomeVolume24h {
        OutcomeVolume24h {
            contract,
            notional: contract * 2.0,
        }
    }
}
