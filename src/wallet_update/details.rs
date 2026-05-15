use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(super) fn update_wallet_details(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenWalletDetailsWindow(address) => {
                return self.open_wallet_details_window(address);
            }
            Message::RefreshWalletDetails(window_id) => {
                return self.refresh_wallet_details_window(window_id);
            }
            Message::WalletDetailsLoaded(window_id, address, result) => {
                let exchange_symbols = self.exchange_symbols.clone();
                let muted_tickers = self.muted_tickers.clone();
                let market_universe = self.market_universe.clone();
                let Some(state) = self.wallet_detail_windows.get_mut(&window_id) else {
                    return Task::none();
                };
                if state.address != address {
                    return Task::none();
                }
                state.loading = false;
                match *result {
                    Ok(data) => {
                        let data = Self::filter_wallet_details_for_hidden_symbols_with(
                            &exchange_symbols,
                            &muted_tickers,
                            &market_universe,
                            data,
                        );
                        state.last_refresh_ms = Some(data.fetched_at_ms);
                        state.data = Some(data);
                        state.error = None;
                    }
                    Err(e) => {
                        state.error = Some(e);
                    }
                }
            }
            Message::WalletDetailsWsUpdate(source_address, data) => {
                return self.apply_wallet_details_ws_update(source_address, *data);
            }
            _ => {}
        }

        Task::none()
    }
}
