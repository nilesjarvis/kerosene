use crate::api::{self, MarketType};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;
use std::collections::{BTreeSet, HashMap};

impl TradingTerminal {
    pub(super) fn request_outcome_volume_refresh(&mut self) -> Task<Message> {
        let symbols: Vec<String> = self
            .exchange_symbols
            .iter()
            .filter(|symbol| symbol.market_type == MarketType::Outcome)
            .filter(|symbol| symbol.is_user_selectable_market())
            .filter(|symbol| !self.exchange_symbol_is_hidden(symbol))
            .map(|symbol| symbol.key.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        self.outcome_volumes_error = None;
        if symbols.is_empty() {
            self.outcome_volumes_24h.clear();
            self.outcome_volumes_loading = false;
            return Task::none();
        }

        self.outcome_volumes_loading = true;
        Task::perform(
            api::fetch_outcome_volumes_24h(symbols),
            Message::OutcomeVolumesLoaded,
        )
    }

    pub(super) fn apply_outcome_volumes_loaded(
        &mut self,
        result: Result<HashMap<String, f64>, String>,
    ) -> Task<Message> {
        self.outcome_volumes_loading = false;
        match result {
            Ok(volumes) => {
                self.outcome_volumes_24h = volumes;
                self.outcome_volumes_error = None;
            }
            Err(error) => {
                self.outcome_volumes_error = Some(error);
            }
        }
        Task::none()
    }
}
