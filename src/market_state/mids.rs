mod dexes;
mod parsing;
mod updates;

use crate::account::{HIP3_DEXES, fetch_all_mids};
use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::OrderKind;

use self::dexes::known_mids_dexes;
use self::parsing::parse_mids_response;
use self::updates::apply_mids_update;

use iced::Task;
use std::collections::HashMap;

impl TradingTerminal {
    pub(crate) fn handle_mids_update(&mut self, mids: HashMap<String, f64>) -> Task<Message> {
        let exchange_symbols = self.exchange_symbols.clone();
        let muted_tickers = self.muted_tickers.clone();
        let market_universe = self.market_universe.clone();
        let is_hidden = |symbol: &str| {
            Self::symbol_key_is_hidden_with(
                &exchange_symbols,
                &muted_tickers,
                &market_universe,
                symbol,
            )
        };

        apply_mids_update(
            &mut self.all_mids,
            &mut self.all_mids_updated_at_ms,
            &mut self.live_watchlist_flashes,
            mids,
            Self::now_ms(),
            is_hidden,
        );
        if matches!(self.order_kind, OrderKind::Limit | OrderKind::Chase)
            && self.order_price.trim().is_empty()
        {
            let active_symbol = self.active_symbol.clone();
            self.refresh_order_price_for_symbol(&active_symbol);
        }
        self.refresh_live_watchlist_row_caches();
        self.order_book_precision_refresh_task()
    }

    pub(crate) fn fetch_mids_task_for_dex(dex: &str) -> Task<Message> {
        let dex_name = dex.to_string();
        Task::perform(fetch_all_mids(dex_name.clone()), move |result| {
            let parsed = result.map(parse_mids_response);
            Message::AllMidsBootstrapLoaded(dex_name.clone(), parsed)
        })
    }

    pub(crate) fn known_mids_dexes_from_symbols(symbols: &[ExchangeSymbol]) -> Vec<String> {
        known_mids_dexes(symbols, HIP3_DEXES)
    }

    pub(crate) fn visible_mids_dexes(&self) -> Vec<String> {
        self.market_universe
            .selected_hip3_dex()
            .map(|dex| vec![dex.to_string()])
            .unwrap_or_else(|| Self::known_mids_dexes_from_symbols(&self.exchange_symbols))
    }

    pub(crate) fn mids_bootstrap_tasks(&self) -> Vec<Task<Message>> {
        let dexes = self.visible_mids_dexes();
        let mut tasks = Vec::with_capacity(dexes.len());
        for dex in dexes {
            tasks.push(Self::fetch_mids_task_for_dex(&dex));
        }
        tasks
    }
}
