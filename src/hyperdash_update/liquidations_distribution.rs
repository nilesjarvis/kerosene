use crate::app_state::TradingTerminal;
use crate::hyperdash_api::fetch_liquidation_levels_at;
use crate::liquidations_distribution_state::{
    LiquidationDistributionData, LiquidationDistributionRequest,
    validate_liquidation_distribution_level,
};
use crate::message::Message;
use crate::pane_state::PaneKind;

use iced::Task;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Liquidations Distribution Requests
// ---------------------------------------------------------------------------

const LIQUIDATION_DISTRIBUTION_DOWNSIDE_RANGE_PCT: f64 = 0.75;
const LIQUIDATION_DISTRIBUTION_UPSIDE_RANGE_PCT: f64 = 0.75;

impl TradingTerminal {
    pub(in crate::hyperdash_update) fn update_liquidations_distribution(
        &mut self,
        message: Message,
    ) -> Task<Message> {
        match message {
            Message::RefreshLiquidationsDistribution => {
                self.request_liquidation_distribution_refresh(true)
            }
            Message::LiquidationsDistributionLoaded(request_key, generation, result) => {
                self.apply_liquidation_distribution_loaded(request_key, generation, *result)
            }
            Message::LiquidationsDistributionSearchChanged(query) => {
                self.liquidation_distribution.symbol_search_query = query;
                self.liquidation_distribution.symbol_picker_open = true;
                Task::none()
            }
            Message::ToggleLiquidationsDistributionSymbolPicker => {
                self.liquidation_distribution.symbol_picker_open =
                    !self.liquidation_distribution.symbol_picker_open;
                Task::none()
            }
            Message::LiquidationsDistributionSymbolSelected(symbol) => {
                self.select_liquidation_distribution_symbol(symbol)
            }
            Message::LiquidationsDistributionZoomed { factor, anchor } => {
                self.liquidation_distribution.zoom_by(factor, anchor);
                Task::none()
            }
            Message::ResetLiquidationsDistributionZoom => {
                self.liquidation_distribution.reset_zoom();
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub(crate) fn request_liquidation_distribution_refresh(
        &mut self,
        force: bool,
    ) -> Task<Message> {
        if !self.pane_is_open(|kind| matches!(kind, PaneKind::LiquidationsDistribution)) {
            return Task::none();
        }
        if self.liquidation_distribution.loading && !force {
            return Task::none();
        }
        if self.liquidation_distribution.symbol.trim().is_empty() && !force {
            return Task::none();
        }

        let selected_symbol = self.liquidation_distribution.symbol.clone();
        let request = match self.build_liquidation_distribution_request() {
            Ok(request) => request,
            Err(error) => {
                self.liquidation_distribution.loading = false;
                self.liquidation_distribution.error = Some(error);
                self.liquidation_distribution.pending_request = None;
                self.liquidation_distribution
                    .clear_data_if_not_symbol(&selected_symbol);
                return Task::none();
            }
        };

        if !self.liquidation_distribution.should_fetch(&request, force) {
            return Task::none();
        }

        let api_key = self.hyperdash_api_key.trim().to_string().into();
        let generation = self.hyperdash_key_generation;
        self.liquidation_distribution
            .clear_data_if_not_symbol(&request.symbol);
        self.liquidation_distribution.loading = true;
        self.liquidation_distribution.error = None;
        self.liquidation_distribution.pending_request = Some(request.clone());
        self.liquidation_distribution.last_request = Some(Instant::now());
        self.liquidation_distribution.last_request_symbol = Some(request.symbol.clone());

        Task::perform(
            fetch_liquidation_levels_at(
                request.coin,
                request.min,
                request.max,
                request.timestamp_secs,
                api_key,
            ),
            move |result| {
                Message::LiquidationsDistributionLoaded(
                    request.key.clone(),
                    generation,
                    Box::new(result),
                )
            },
        )
    }

    fn build_liquidation_distribution_request(
        &self,
    ) -> Result<LiquidationDistributionRequest, String> {
        if self.hyperdash_api_key.trim().is_empty() {
            return Err("Add HyperDash key in Settings > Integrations".to_string());
        }
        let symbol = self.liquidation_distribution.symbol.trim();
        if symbol.is_empty() {
            return Err("Select a perp market to load liquidation distribution".to_string());
        }
        if self.symbol_key_is_hidden(symbol) {
            return Err("Ticker is hidden in Settings > Risk".to_string());
        }

        let Some(coin) = self.hyperdash_coin_for_symbol(symbol) else {
            return Err(
                "HyperDash liquidation distribution is available for perp markets only".to_string(),
            );
        };
        let display = self.liquidation_distribution_symbol_display(symbol);
        let Some(mark) = self.resolve_mid_for_symbol(symbol) else {
            return Err(format!("Waiting for a live mid price for {display}"));
        };
        if !mark.is_finite() || mark <= 0.0 {
            return Err(format!("Invalid live mid price for {display}"));
        }

        let min = (mark * (1.0 - LIQUIDATION_DISTRIBUTION_DOWNSIDE_RANGE_PCT)).max(0.0);
        let max = mark * (1.0 + LIQUIDATION_DISTRIBUTION_UPSIDE_RANGE_PCT);

        Ok(LiquidationDistributionRequest::new(
            symbol.to_string(),
            display,
            coin,
            mark,
            min,
            max,
            Self::now_ms() / 1_000,
        ))
    }

    fn select_liquidation_distribution_symbol(&mut self, symbol: String) -> Task<Message> {
        if self.symbol_key_is_hidden(&symbol) {
            self.liquidation_distribution.error =
                Some(format!("{symbol} is hidden in Settings > Risk"));
            self.liquidation_distribution.symbol_picker_open = false;
            return Task::none();
        }
        if self.hyperdash_coin_for_symbol(&symbol).is_none() {
            self.liquidation_distribution.error = Some(
                "HyperDash liquidation distribution is available for perp markets only".into(),
            );
            self.liquidation_distribution.symbol_picker_open = false;
            return Task::none();
        }

        let display = self.liquidation_distribution_symbol_display(&symbol);
        self.liquidation_distribution.symbol = symbol.clone();
        self.liquidation_distribution.symbol_search_query = display;
        self.liquidation_distribution.symbol_picker_open = false;
        self.liquidation_distribution.error = None;
        self.liquidation_distribution
            .clear_data_if_not_symbol(&symbol);
        self.persist_config();
        self.request_liquidation_distribution_refresh(true)
    }

    pub(crate) fn liquidation_distribution_symbol_display(&self, symbol: &str) -> String {
        self.exchange_symbols
            .iter()
            .find(|candidate| candidate.key == symbol)
            .map(Self::exchange_symbol_display_name)
            .unwrap_or_else(|| symbol.to_string())
    }

    fn apply_liquidation_distribution_loaded(
        &mut self,
        request_key: String,
        generation: u64,
        result: Result<crate::hyperdash_api::LiquidationLevel, String>,
    ) -> Task<Message> {
        if !self.hyperdash_key_generation_is_current(generation) {
            return Task::none();
        }

        let Some(request) = self.liquidation_distribution.pending_request.clone() else {
            return Task::none();
        };
        if request.key != request_key {
            return Task::none();
        }

        self.liquidation_distribution.loading = false;
        self.liquidation_distribution.pending_request = None;

        match result {
            Ok(level) => {
                if let Err(error) = validate_liquidation_distribution_level(&request, &level) {
                    return self.apply_liquidation_distribution_error(
                        request,
                        format!("Liquidation distribution response rejected: {error}"),
                    );
                }
                let data = LiquidationDistributionData::from_level(request, level, Self::now_ms());
                self.liquidation_distribution.error = None;
                self.liquidation_distribution.last_fetch = Some(Instant::now());
                self.liquidation_distribution.data = Some(data);
            }
            Err(error) => {
                let message = format!("Liquidation distribution fetch failed: {error}");
                return self.apply_liquidation_distribution_error(request, message);
            }
        }

        Task::none()
    }

    fn apply_liquidation_distribution_error(
        &mut self,
        request: LiquidationDistributionRequest,
        message: String,
    ) -> Task<Message> {
        self.liquidation_distribution.error = Some(message.clone());
        self.liquidation_distribution
            .clear_data_if_not_symbol(&request.symbol);
        self.push_toast(message, true);
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::hyperdash_api::LiquidationLevel;

    fn request() -> LiquidationDistributionRequest {
        LiquidationDistributionRequest::new(
            "BTC".to_string(),
            "BTC".to_string(),
            "BTC".to_string(),
            100.0,
            0.0,
            200.0,
            1_778_357_590,
        )
    }

    fn level() -> LiquidationLevel {
        LiquidationLevel {
            coin: "BTC".to_string(),
            min: 0.0,
            max: 200.0,
            liquidations: Vec::new(),
        }
    }

    #[test]
    fn stale_hyperdash_generation_result_keeps_current_pending_request() {
        let (mut terminal, _) = TradingTerminal::boot();
        let request = request();
        terminal.hyperdash_key_generation = 2;
        terminal.liquidation_distribution.loading = true;
        terminal.liquidation_distribution.error = Some("current error".to_string());
        terminal.liquidation_distribution.pending_request = Some(request.clone());

        let _task =
            terminal.apply_liquidation_distribution_loaded(request.key.clone(), 1, Ok(level()));

        assert!(terminal.liquidation_distribution.loading);
        assert_eq!(
            terminal.liquidation_distribution.pending_request,
            Some(request)
        );
        assert_eq!(
            terminal.liquidation_distribution.error.as_deref(),
            Some("current error")
        );
        assert!(terminal.liquidation_distribution.data.is_none());
    }

    #[test]
    fn stale_hyperdash_generation_error_does_not_fail_current_pending_request() {
        let (mut terminal, _) = TradingTerminal::boot();
        let request = request();
        terminal.hyperdash_key_generation = 2;
        terminal.liquidation_distribution.loading = true;
        terminal.liquidation_distribution.pending_request = Some(request.clone());

        let _task = terminal.apply_liquidation_distribution_loaded(
            request.key.clone(),
            1,
            Err("old key rejected".to_string()),
        );

        assert!(terminal.liquidation_distribution.loading);
        assert_eq!(
            terminal.liquidation_distribution.pending_request,
            Some(request)
        );
        assert!(terminal.liquidation_distribution.error.is_none());
        assert!(terminal.toasts.is_empty());
    }

    #[test]
    fn current_hyperdash_generation_result_finishes_pending_request() {
        let (mut terminal, _) = TradingTerminal::boot();
        let request = request();
        terminal.hyperdash_key_generation = 2;
        terminal.liquidation_distribution.loading = true;
        terminal.liquidation_distribution.pending_request = Some(request.clone());

        let _task =
            terminal.apply_liquidation_distribution_loaded(request.key.clone(), 2, Ok(level()));

        assert!(!terminal.liquidation_distribution.loading);
        assert!(terminal.liquidation_distribution.pending_request.is_none());
        assert!(terminal.liquidation_distribution.error.is_none());
        assert!(terminal.liquidation_distribution.last_fetch.is_some());
        let data = terminal
            .liquidation_distribution
            .data
            .as_ref()
            .expect("current-generation result should apply");
        assert_eq!(data.request, request);
    }

    #[test]
    fn hyperdash_generation_bump_invalidates_pending_distribution_request() {
        let (mut terminal, _) = TradingTerminal::boot();
        terminal.liquidation_distribution.loading = true;
        terminal.liquidation_distribution.pending_request = Some(request());

        terminal.bump_hyperdash_key_generation();

        assert_eq!(terminal.hyperdash_key_generation, 1);
        assert!(!terminal.liquidation_distribution.loading);
        assert!(terminal.liquidation_distribution.pending_request.is_none());
    }
}
