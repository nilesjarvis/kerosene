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
            Message::LiquidationsDistributionLoaded(request_key, result) => {
                self.apply_liquidation_distribution_loaded(request_key, *result)
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

        let active_symbol = self.active_symbol.clone();
        let request = match self.build_liquidation_distribution_request() {
            Ok(request) => request,
            Err(error) => {
                self.liquidation_distribution.loading = false;
                self.liquidation_distribution.error = Some(error);
                self.liquidation_distribution.pending_request = None;
                self.liquidation_distribution
                    .clear_data_if_not_symbol(&active_symbol);
                return Task::none();
            }
        };

        if !self.liquidation_distribution.should_fetch(&request, force) {
            return Task::none();
        }

        let api_key = self.hyperdash_api_key.trim().to_string();
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
                Message::LiquidationsDistributionLoaded(request.key.clone(), Box::new(result))
            },
        )
    }

    fn build_liquidation_distribution_request(
        &self,
    ) -> Result<LiquidationDistributionRequest, String> {
        if self.hyperdash_api_key.trim().is_empty() {
            return Err("Add HyperDash key in Settings > Integrations".to_string());
        }
        if self.active_symbol.trim().is_empty() {
            return Err("Select a perp market to load liquidation distribution".to_string());
        }
        if self.symbol_key_is_hidden(&self.active_symbol) {
            return Err("Ticker is hidden in Settings > Risk".to_string());
        }

        let Some(coin) = self.hyperdash_coin_for_symbol(&self.active_symbol) else {
            return Err(
                "HyperDash liquidation distribution is available for perp markets only".to_string(),
            );
        };
        let Some(mark) = self.resolve_mid_for_symbol(&self.active_symbol) else {
            return Err(format!(
                "Waiting for a live mid price for {}",
                self.active_symbol_display
            ));
        };
        if !mark.is_finite() || mark <= 0.0 {
            return Err(format!(
                "Invalid live mid price for {}",
                self.active_symbol_display
            ));
        }

        let min = (mark * (1.0 - LIQUIDATION_DISTRIBUTION_DOWNSIDE_RANGE_PCT)).max(0.0);
        let max = mark * (1.0 + LIQUIDATION_DISTRIBUTION_UPSIDE_RANGE_PCT);
        let display = if self.active_symbol_display.trim().is_empty() {
            coin.clone()
        } else {
            self.active_symbol_display.clone()
        };

        Ok(LiquidationDistributionRequest::new(
            self.active_symbol.clone(),
            display,
            coin,
            mark,
            min,
            max,
            Self::now_ms() / 1_000,
        ))
    }

    fn apply_liquidation_distribution_loaded(
        &mut self,
        request_key: String,
        result: Result<crate::hyperdash_api::LiquidationLevel, String>,
    ) -> Task<Message> {
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
