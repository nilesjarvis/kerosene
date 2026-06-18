use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Task;

impl TradingTerminal {
    pub(super) fn update_wallet_details(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenWalletDetailsWindow(address) => {
                return self.open_wallet_details_window(address.into_string());
            }
            Message::RefreshWalletDetails(window_id) => {
                return self.refresh_wallet_details_window(window_id);
            }
            Message::WalletDetailsLoaded(window_id, address, context, result) => {
                let address = address.into_string();
                let context_is_current = self.read_data_request_context_is_current(context);
                let exchange_symbols = self.exchange_symbols.clone();
                let muted_tickers = self.muted_tickers.clone();
                let market_universe = self.market_universe.clone();
                let Some(state) = self.wallet_detail_windows.get_mut(&window_id) else {
                    return Task::none();
                };
                if state.address != address {
                    return Task::none();
                }
                if !context_is_current {
                    if state.loading && state.loading_context == Some(context) {
                        state.loading = false;
                        state.loading_context = None;
                    }
                    return Task::none();
                }
                state.loading = false;
                state.loading_context = None;
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
                return self.apply_wallet_details_ws_update(
                    source_address.map(|address| address.into_string()),
                    *data,
                );
            }
            _ => {}
        }

        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ReadDataProvider;
    use crate::read_data_provider::ReadDataRequestContext;
    use crate::wallet_state::WalletDetailsWindowState;
    use crate::ws::WsUserData;

    const TEST_ADDRESS: &str = "0xabc0000000000000000000000000000000000000";

    #[test]
    fn stale_hydromancer_context_clears_matching_wallet_details_loading() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_key_generation = 2;
        let window_id = iced::window::Id::unique();
        let mut state = WalletDetailsWindowState::new(TEST_ADDRESS.to_string());
        let stale_context = ReadDataRequestContext {
            provider: ReadDataProvider::Hydromancer,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: 1,
        };
        state.loading_context = Some(stale_context);
        state.error = Some("old error".to_string());
        terminal.wallet_detail_windows.insert(window_id, state);

        let _task = terminal.update_wallet_details(Message::WalletDetailsLoaded(
            window_id,
            TEST_ADDRESS.to_string().into(),
            stale_context,
            Box::new(Err("new error".to_string())),
        ));

        let state = terminal
            .wallet_detail_windows
            .get(&window_id)
            .expect("details window");
        assert!(!state.loading);
        assert_eq!(state.loading_context, None);
        assert_eq!(state.error.as_deref(), Some("old error"));
    }

    #[test]
    fn stale_hydromancer_context_does_not_clear_newer_wallet_details_request() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.read_data_provider = ReadDataProvider::Hydromancer;
        terminal.hydromancer_key_generation = 2;
        let window_id = iced::window::Id::unique();
        let stale_context = ReadDataRequestContext {
            provider: ReadDataProvider::Hydromancer,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: 1,
        };
        let current_context = ReadDataRequestContext {
            provider: ReadDataProvider::Hydromancer,
            read_data_provider_generation: terminal.read_data_provider_generation,
            hydromancer_key_generation: 2,
        };
        let mut state = WalletDetailsWindowState::new(TEST_ADDRESS.to_string());
        state.loading_context = Some(current_context);
        terminal.wallet_detail_windows.insert(window_id, state);

        let _task = terminal.update_wallet_details(Message::WalletDetailsLoaded(
            window_id,
            TEST_ADDRESS.to_string().into(),
            stale_context,
            Box::new(Err("old request failed".to_string())),
        ));

        let state = terminal
            .wallet_detail_windows
            .get(&window_id)
            .expect("details window");
        assert!(state.loading);
        assert_eq!(state.loading_context, Some(current_context));
        assert_eq!(state.error, None);
    }

    #[test]
    fn wallet_detail_stream_lag_marks_matching_window_stale_and_refreshing() {
        let mut terminal = TradingTerminal::boot().0;
        let window_id = iced::window::Id::unique();
        let mut state = WalletDetailsWindowState::new(TEST_ADDRESS.to_string());
        state.loading = false;
        state.error = None;
        terminal.wallet_detail_windows.insert(window_id, state);

        let _task = terminal.update_wallet_details(Message::WalletDetailsWsUpdate(
            Some(TEST_ADDRESS.to_string().into()),
            Box::new(WsUserData::Lagged { skipped: 7 }),
        ));

        let state = terminal
            .wallet_detail_windows
            .get(&window_id)
            .expect("details window");
        assert!(state.loading);
        assert!(
            state
                .error
                .as_deref()
                .is_some_and(|error| error.contains("stream lagged"))
        );
    }

    #[test]
    fn wallet_detail_stream_lag_does_not_mark_other_wallet_windows() {
        let mut terminal = TradingTerminal::boot().0;
        let window_id = iced::window::Id::unique();
        let mut state =
            WalletDetailsWindowState::new("0xdef0000000000000000000000000000000000000".to_string());
        state.loading = false;
        state.error = None;
        terminal.wallet_detail_windows.insert(window_id, state);

        let _task = terminal.update_wallet_details(Message::WalletDetailsWsUpdate(
            Some(TEST_ADDRESS.to_string().into()),
            Box::new(WsUserData::Lagged { skipped: 7 }),
        ));

        let state = terminal
            .wallet_detail_windows
            .get(&window_id)
            .expect("details window");
        assert!(!state.loading);
        assert_eq!(state.error, None);
    }
}
