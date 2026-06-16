use super::model::{
    WALLET_DETAILS_DEFAULT_HEIGHT, WALLET_DETAILS_DEFAULT_WIDTH, WalletDetailsWindowState,
};
use crate::account::{
    AccountDataFetchScope, WalletOpenOrderDetail, fetch_wallet_details_scoped_with_provider,
    normalize_dex_open_order_coins,
};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::read_data_provider::ReadDataRequestContext;
use crate::ws::WsUserData;

use iced::{Size, Task, window};

impl TradingTerminal {
    pub(crate) fn wallet_details_fetch_task(
        &self,
        window_id: window::Id,
        address: String,
        scope: AccountDataFetchScope,
        read_context: ReadDataRequestContext,
    ) -> Task<Message> {
        let provider = self.read_data_provider;
        let hydromancer_key = self.hydromancer_api_key_for_task();
        Task::perform(
            fetch_wallet_details_scoped_with_provider(
                address.clone(),
                scope,
                provider,
                hydromancer_key,
            ),
            move |r| {
                Message::WalletDetailsLoaded(window_id, address.clone(), read_context, Box::new(r))
            },
        )
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
            ..crate::window_chrome::settings(self.custom_window_chrome_active)
        };
        let (window_id, open_task) = window::open(settings);
        let read_context = self.read_data_request_context();
        let mut state = WalletDetailsWindowState::new(address.clone());
        state.loading_context = Some(read_context);
        self.wallet_detail_windows.insert(window_id, state);

        let scope = self.account_data_fetch_scope();
        Task::batch([
            open_task.map(Message::WindowOpened),
            self.wallet_details_fetch_task(window_id, address, scope, read_context),
        ])
    }

    pub(crate) fn refresh_wallet_details_window(&mut self, window_id: window::Id) -> Task<Message> {
        let read_context = self.read_data_request_context();
        let Some(state) = self.wallet_detail_windows.get_mut(&window_id) else {
            return Task::none();
        };
        if state.loading {
            return Task::none();
        }
        state.loading = true;
        state.loading_context = Some(read_context);
        state.error = None;
        let address = state.address.clone();
        let scope = self.account_data_fetch_scope();
        self.wallet_details_fetch_task(window_id, address, scope, read_context)
    }

    pub(crate) fn apply_wallet_details_ws_update(
        &mut self,
        address: Option<String>,
        data: WsUserData,
    ) -> Task<Message> {
        let Some(address) = address.as_deref().and_then(Self::normalize_wallet_address) else {
            if let WsUserData::AllMids(mids) = data {
                return self.handle_mids_update(mids);
            }
            return Task::none();
        };

        let now_ms = Self::now_ms();
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
        match data {
            WsUserData::AllDexPositions {
                main_state,
                states_by_dex: _,
                all_positions,
                position_details,
            } => {
                let all_positions: Vec<_> = all_positions
                    .into_iter()
                    .filter(|position| !is_hidden(&position.position.coin))
                    .collect();
                let position_details: Vec<_> = position_details
                    .into_iter()
                    .filter(|position| !is_hidden(&position.asset_position.position.coin))
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
                let mut orders = orders;
                normalize_dex_open_order_coins(&dex, &mut orders);
                let orders: Vec<_> = orders
                    .into_iter()
                    .filter(|order| !is_hidden(&order.coin))
                    .collect();
                for state in self
                    .wallet_detail_windows
                    .values_mut()
                    .filter(|state| state.address == address)
                {
                    if let Some(details) = state.data.as_mut() {
                        details
                            .open_orders
                            .retain(|order| order.dex != dex && !is_hidden(&order.order.coin));
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
                    .filter(|balance| !is_hidden(&balance.coin))
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
                return self.handle_mids_update(mids);
            }
            WsUserData::Fills { .. } => {}
            WsUserData::Lagged { skipped } => {
                let mut refreshes = Vec::new();
                let read_context = self.read_data_request_context();
                for (window_id, state) in self
                    .wallet_detail_windows
                    .iter_mut()
                    .filter(|(_, state)| state.address == address)
                {
                    state.error = Some(format!(
                        "Wallet detail stream lagged ({skipped} updates skipped); refreshing \
                         snapshot"
                    ));
                    if !state.loading {
                        state.loading = true;
                        state.loading_context = Some(read_context);
                        refreshes.push((*window_id, state.address.clone()));
                    }
                }

                if !refreshes.is_empty() {
                    let scope = self.account_data_fetch_scope();
                    let tasks: Vec<_> = refreshes
                        .into_iter()
                        .map(|(window_id, address)| {
                            self.wallet_details_fetch_task(
                                window_id,
                                address,
                                scope.clone(),
                                read_context,
                            )
                        })
                        .collect();
                    return Task::batch(tasks);
                }
            }
        }
        Task::none()
    }
}
