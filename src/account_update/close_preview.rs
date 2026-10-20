use crate::account_state::BottomTab;
use crate::api::OrderBook;
use crate::app_state::TradingTerminal;
use crate::helpers::parse_finite_number;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::read_data_provider::MarketDataSourceContext;
use iced::Task;
use std::time::Instant;

impl TradingTerminal {
    pub(crate) fn position_close_preview_is_visible(&self, coin: &str) -> bool {
        !self.hide_pnl
            && self.pane_is_open(|kind| {
                matches!(
                    kind,
                    PaneKind::BottomTabs {
                        active_tab: BottomTab::Positions
                    }
                )
            })
            && !self.symbol_key_is_hidden(coin)
            && (!self.position_is_hidden(coin) || self.show_hidden_positions)
            && self.account_positions_with_outcomes().iter().any(|ap| {
                ap.position.coin == coin
                    && parse_finite_number(&ap.position.szi).is_some_and(|size| size.abs() > 1e-12)
            })
    }

    pub(super) fn hover_position_close_preview(
        &mut self,
        coin: String,
        hovered: bool,
    ) -> Task<Message> {
        if !hovered {
            if self.position_close_preview.coin.as_deref() == Some(coin.as_str()) {
                self.position_close_preview.clear();
            }
            return Task::none();
        }
        self.position_close_preview.clear();
        if !self.position_close_preview_is_visible(&coin) {
            return Task::none();
        }
        self.position_close_preview.coin = Some(coin.clone());
        self.position_close_preview.now_ms = Self::now_ms();
        let generation = self.position_close_preview.generation;
        if self.is_outcome_coin(&coin) {
            self.position_close_preview.fee_multiplier =
                Some(Err("Outcome closing fee estimate unavailable".into()));
        } else if let Some(symbol) = self.exchange_symbol_for_key(&coin).cloned() {
            let stable_pair = symbol.market_type == crate::api::MarketType::Spot
                && crate::helpers::is_usd_stable_fee_token(&symbol.ticker);
            if crate::helpers::hip3_dex(&coin).is_none() && symbol.collateral_token == Some(0) {
                self.position_close_preview.fee_multiplier =
                    Some(Ok(if stable_pair { 0.2 } else { 1.0 }));
                return Task::none();
            }
            return Task::perform(
                fees::fetch_market_fee_multiplier(
                    coin.clone(),
                    symbol.collateral_token,
                    stable_pair,
                ),
                move |result| Message::PositionCloseFeeLoaded {
                    generation,
                    coin: coin.clone(),
                    result,
                },
            );
        } else {
            self.position_close_preview.fee_multiplier =
                Some(Err("Market fee terms unavailable".into()));
        }
        Task::none()
    }

    pub(super) fn apply_position_close_book(
        &mut self,
        generation: u64,
        coin: String,
        source_context: MarketDataSourceContext,
        book: Option<OrderBook>,
    ) -> Task<Message> {
        if generation != self.position_close_preview.generation
            || self.position_close_preview.coin.as_deref() != Some(coin.as_str())
            || !self.market_stream_source_is_current(source_context)
            || !self.position_close_preview_is_visible(&coin)
        {
            return Task::none();
        }
        self.position_close_preview.book = book.map(|book| (book, source_context, Instant::now()));
        self.position_close_preview.now_ms = Self::now_ms();
        Task::none()
    }

    pub(super) fn apply_position_close_fee(
        &mut self,
        generation: u64,
        coin: String,
        result: Result<f64, String>,
    ) -> Task<Message> {
        if generation == self.position_close_preview.generation
            && self.position_close_preview.coin.as_deref() == Some(coin.as_str())
            && self.position_close_preview_is_visible(&coin)
        {
            self.position_close_preview.fee_multiplier = Some(result);
        }
        Task::none()
    }
}

mod fees;
#[cfg(test)]
mod tests;
