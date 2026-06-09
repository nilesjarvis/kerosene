mod chase;
mod fills;
mod orders;

use fills::apply_fills_update;
use orders::preserve_open_order_reduce_only;

use crate::account::{fetch_account_data_scoped_with_provider, normalize_dex_open_order_coins};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws::WsUserData;

use iced::Task;

#[cfg(test)]
use fills::{chase_fill_summary, prepend_recent_fills};
#[cfg(test)]
use orders::apply_open_order_to_chase;

#[cfg(test)]
mod tests;

fn should_repair_account_from_ws(
    connected_address: Option<&str>,
    has_account_data: bool,
    account_loading: bool,
) -> bool {
    connected_address.is_some() && !has_account_data && !account_loading
}

impl TradingTerminal {
    pub(super) fn apply_ws_user_data_update(
        &mut self,
        source_address: Option<String>,
        ws_data: WsUserData,
    ) -> Task<Message> {
        if source_address.as_deref() != self.connected_address.as_deref() {
            return self.apply_wallet_details_ws_update(source_address, ws_data);
        }

        // Broadcast fanout fell behind — at least `skipped` order / fill /
        // position updates were dropped before this consumer caught up.
        // Local state is now potentially stale relative to the exchange;
        // force a full account refresh rather than risk firing chase or
        // TWAP logic off a state snapshot that's missing fills. Use the
        // shared force-refresh path so trading handlers see `account_loading`
        // immediately and fail closed until the replacement snapshot arrives.
        if let WsUserData::Lagged { skipped } = &ws_data {
            let toast = format!(
                "WS user-data stream lagged ({} update{} dropped); refreshing account...",
                skipped,
                if *skipped == 1 { "" } else { "s" }
            );
            self.push_toast(toast, true);
            if let Some(addr) = self.connected_address.clone() {
                return self.force_refresh_account_data_for_reconciliation(addr);
            }
            return Task::none();
        }
        let wallet_details_update = ws_data.clone();

        let mut orders_changed = false;
        let mut fills_changed = false;
        let mut positions_changed = false;
        let mut mids_task = Task::none();
        let mut fill_toast_msgs: Vec<String> = Vec::new();
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
        let is_muted = |symbol: &str| {
            Self::key_matches_muted_tickers(&exchange_symbols, &muted_tickers, symbol)
        };
        if let Some(data) = &mut self.account_data {
            match ws_data {
                WsUserData::AllDexPositions {
                    main_state,
                    states_by_dex,
                    all_positions,
                    position_details: _,
                } => {
                    let mut states_by_dex = states_by_dex;
                    for state in states_by_dex.values_mut() {
                        state
                            .asset_positions
                            .retain(|position| !is_hidden(&position.position.coin));
                    }
                    data.fetched_at_ms = Self::now_ms();
                    data.clearinghouse.margin_summary = main_state.margin_summary;
                    data.clearinghouse.withdrawable = main_state.withdrawable;
                    data.clearinghouse.cross_margin_summary = main_state.cross_margin_summary;
                    data.clearinghouse.cross_maintenance_margin_used =
                        main_state.cross_maintenance_margin_used;
                    data.clearinghouse.asset_positions = all_positions
                        .into_iter()
                        .filter(|position| !is_hidden(&position.position.coin))
                        .collect();
                    data.clearinghouses_by_dex = states_by_dex;
                    positions_changed = true;
                }
                WsUserData::OpenOrders { dex, orders } => {
                    let mut orders = orders;
                    normalize_dex_open_order_coins(&dex, &mut orders);
                    for order in &mut orders {
                        preserve_open_order_reduce_only(order, &data.open_orders);
                    }
                    if dex.is_empty() {
                        data.open_orders.retain(|o| o.coin.contains(':'));
                    } else {
                        let prefix = format!("{dex}:");
                        data.open_orders.retain(|o| !o.coin.starts_with(&prefix));
                    }
                    data.open_orders.retain(|order| !is_hidden(&order.coin));
                    data.open_orders
                        .extend(orders.into_iter().filter(|order| !is_hidden(&order.coin)));
                    orders_changed = true;
                }
                WsUserData::Fills { fills, is_snapshot } => {
                    fill_toast_msgs =
                        apply_fills_update(&mut data.fills, fills, is_snapshot, is_hidden);
                    fills_changed = true;
                }
                WsUserData::SpotBalances(balances) => {
                    data.spot.balances = balances
                        .into_iter()
                        .filter(|balance| !is_muted(&balance.coin))
                        .collect();
                }
                WsUserData::AllMids(mids) => {
                    mids_task = self.handle_mids_update(mids);
                }
                // Lagged is handled by the early-return at the top of the
                // method; this arm exists only for match exhaustiveness.
                WsUserData::Lagged { .. } => {}
            }
        } else {
            match ws_data {
                WsUserData::AllMids(mids) => {
                    mids_task = self.handle_mids_update(mids);
                }
                _ => {
                    if should_repair_account_from_ws(
                        self.connected_address.as_deref(),
                        self.account_data.is_some(),
                        self.account_loading,
                    ) && let Some(addr) = &self.connected_address
                    {
                        let addr = addr.clone();
                        let requested_addr = addr.clone();
                        self.account_loading = true;
                        self.account_error = None;
                        let wallet_task = self.apply_wallet_details_ws_update(
                            source_address.clone(),
                            wallet_details_update,
                        );
                        let scope = self.account_data_fetch_scope();
                        let provider = self.read_data_provider;
                        let hydromancer_key = self.hydromancer_api_key.trim().to_string();
                        let account_task = Task::perform(
                            fetch_account_data_scoped_with_provider(
                                addr,
                                scope,
                                provider,
                                hydromancer_key,
                            ),
                            move |r| {
                                Message::AccountDataLoaded(requested_addr.clone(), Box::new(r))
                            },
                        );
                        return Task::batch([wallet_task, account_task]);
                    }
                }
            }
        }

        for msg in fill_toast_msgs {
            self.push_toast(msg, false);
        }
        if positions_changed {
            self.sync_all_chart_overlays();
        } else if orders_changed {
            self.sync_all_chart_orders();
        }
        if fills_changed && !positions_changed {
            self.sync_all_chart_trade_markers();
        }
        if fills_changed {
            self.reconcile_twap_fills_from_account();
        }
        let fill_reconcile_task = if fills_changed {
            self.reconcile_chase_fills_from_account()
        } else {
            Task::none()
        };
        let chase_task = if orders_changed {
            self.handle_chase_order_disappearance()
        } else {
            Task::none()
        };
        let wallet_task = if matches!(wallet_details_update, WsUserData::AllMids(_)) {
            Task::none()
        } else {
            self.apply_wallet_details_ws_update(source_address, wallet_details_update)
        };
        Task::batch([fill_reconcile_task, chase_task, mids_task, wallet_task])
    }
}
