use super::model::{
    WALLET_DETAILS_DEFAULT_HEIGHT, WALLET_DETAILS_DEFAULT_WIDTH, WalletDetailsWindowState,
};
use crate::account::{WalletOpenOrderDetail, fetch_wallet_details};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws::WsUserData;

use iced::{Size, Task, window};

impl TradingTerminal {
    pub(crate) fn wallet_details_fetch_task(
        window_id: window::Id,
        address: String,
    ) -> Task<Message> {
        Task::perform(fetch_wallet_details(address.clone()), move |r| {
            Message::WalletDetailsLoaded(window_id, address.clone(), Box::new(r))
        })
    }

    pub(crate) fn open_wallet_details_window(&mut self, address: String) -> Task<Message> {
        let Some(address) = Self::normalize_wallet_address(&address) else {
            self.push_toast("Invalid wallet address".to_string(), true);
            return Task::none();
        };

        if let Some((&window_id, _)) = self
            .wallet_detail_windows
            .iter()
            .find(|(_, state)| state.address == address)
        {
            return window::gain_focus(window_id);
        }

        let settings = window::Settings {
            size: Size::new(WALLET_DETAILS_DEFAULT_WIDTH, WALLET_DETAILS_DEFAULT_HEIGHT),
            ..window::Settings::default()
        };
        let (window_id, open_task) = window::open(settings);
        self.wallet_detail_windows
            .insert(window_id, WalletDetailsWindowState::new(address.clone()));

        Task::batch([
            open_task.map(Message::WindowOpened),
            Self::wallet_details_fetch_task(window_id, address),
        ])
    }

    pub(crate) fn refresh_wallet_details_window(&mut self, window_id: window::Id) -> Task<Message> {
        let Some(state) = self.wallet_detail_windows.get_mut(&window_id) else {
            return Task::none();
        };
        if state.loading {
            return Task::none();
        }
        state.loading = true;
        state.error = None;
        Self::wallet_details_fetch_task(window_id, state.address.clone())
    }

    pub(crate) fn apply_wallet_details_ws_update(
        &mut self,
        address: Option<String>,
        data: WsUserData,
    ) {
        let Some(address) = address.as_deref().and_then(Self::normalize_wallet_address) else {
            if let WsUserData::AllMids(mids) = data {
                self.handle_mids_update(mids);
            }
            return;
        };

        let now_ms = Self::now_ms();
        let exchange_symbols = self.exchange_symbols.clone();
        let muted_tickers = self.muted_tickers.clone();
        let is_muted = |symbol: &str| {
            Self::key_matches_muted_tickers(&exchange_symbols, &muted_tickers, symbol)
        };
        match data {
            WsUserData::AllDexPositions {
                main_state,
                all_positions,
                position_details,
            } => {
                let all_positions: Vec<_> = all_positions
                    .into_iter()
                    .filter(|position| !is_muted(&position.position.coin))
                    .collect();
                let position_details: Vec<_> = position_details
                    .into_iter()
                    .filter(|position| !is_muted(&position.asset_position.position.coin))
                    .collect();
                for state in self
                    .wallet_detail_windows
                    .values_mut()
                    .filter(|state| state.address == address)
                {
                    if let Some(details) = state.data.as_mut() {
                        details.clearinghouse.margin_summary = main_state.margin_summary.clone();
                        details.clearinghouse.withdrawable = main_state.withdrawable.clone();
                        details.clearinghouse.cross_margin_summary =
                            main_state.cross_margin_summary.clone();
                        details.clearinghouse.cross_maintenance_margin_used =
                            main_state.cross_maintenance_margin_used.clone();
                        details.clearinghouse.asset_positions = all_positions.clone();
                        details.positions = position_details.clone();
                        details.fetched_at_ms = now_ms;
                    }
                    state.last_refresh_ms = Some(now_ms);
                    state.error = None;
                }
            }
            WsUserData::OpenOrders { dex, orders } => {
                let orders: Vec<_> = orders
                    .into_iter()
                    .filter(|order| !is_muted(&order.coin))
                    .collect();
                for state in self
                    .wallet_detail_windows
                    .values_mut()
                    .filter(|state| state.address == address)
                {
                    if let Some(details) = state.data.as_mut() {
                        details
                            .open_orders
                            .retain(|order| order.dex != dex && !is_muted(&order.order.coin));
                        details
                            .open_orders
                            .extend(orders.iter().cloned().map(|order| WalletOpenOrderDetail {
                                dex: dex.clone(),
                                order,
                            }));
                        details.fetched_at_ms = now_ms;
                    }
                    state.last_refresh_ms = Some(now_ms);
                    state.error = None;
                }
            }
            WsUserData::SpotBalances(balances) => {
                let balances: Vec<_> = balances
                    .into_iter()
                    .filter(|balance| !is_muted(&balance.coin))
                    .collect();
                for state in self
                    .wallet_detail_windows
                    .values_mut()
                    .filter(|state| state.address == address)
                {
                    if let Some(details) = state.data.as_mut() {
                        details.spot.balances = balances.clone();
                        details.fetched_at_ms = now_ms;
                    }
                    state.last_refresh_ms = Some(now_ms);
                    state.error = None;
                }
            }
            WsUserData::AllMids(mids) => {
                self.handle_mids_update(mids);
            }
            WsUserData::Fills { .. } => {}
        }
    }
}
