use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(super) fn update_muted_ticker_preferences(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MuteTicker => {
                let Some(ticker) = Self::normalize_muted_ticker_input(&self.muted_ticker_input)
                else {
                    self.muted_ticker_status = Some(("Enter a ticker to mute".to_string(), true));
                    return Task::none();
                };

                if !self.muted_tickers.insert(ticker.clone()) {
                    self.muted_ticker_status = Some((format!("{ticker} is already muted"), true));
                    return Task::none();
                }

                self.muted_ticker_input.clear();
                self.muted_ticker_status = Some((format!("Muted {ticker}"), false));
                self.push_toast(format!("Muted {ticker} across Kerosene"), false);

                let muted_chase_ids: Vec<u64> = self
                    .chase_orders
                    .iter()
                    .filter_map(|(id, chase)| self.is_ticker_muted(&chase.coin).then_some(*id))
                    .collect();
                let stop_chase_task = Task::batch(muted_chase_ids.into_iter().map(|id| {
                    self.stop_chase_by_id_with_reason(id, "Chase stopped: ticker was muted", false)
                }));
                let scrub_task = self.scrub_muted_ticker_state();
                self.refresh_symbol_search_results();
                self.refresh_live_watchlist_row_caches();
                self.persist_config();
                return Task::batch([
                    stop_chase_task,
                    scrub_task,
                    self.request_symbol_search_context_refresh(true),
                    self.request_live_watchlist_refresh(true),
                ]);
            }
            Message::UnmuteTicker(ticker) => {
                let Some(ticker) = Self::normalize_muted_ticker_input(&ticker) else {
                    return Task::none();
                };
                if self.muted_tickers.remove(&ticker) {
                    self.muted_ticker_status = Some((format!("Unmuted {ticker}"), false));
                    self.push_toast(format!("Unmuted {ticker}"), false);
                    self.refresh_symbol_search_results();
                    self.refresh_live_watchlist_row_caches();
                    self.persist_config();
                    return Task::batch([
                        self.request_symbol_search_context_refresh(true),
                        self.request_live_watchlist_refresh(true),
                    ]);
                }
            }
            _ => {}
        }

        Task::none()
    }
}
