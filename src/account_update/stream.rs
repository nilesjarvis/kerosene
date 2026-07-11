mod chase;
mod fills;
mod orders;

use fills::{apply_fills_update, fill_toast_message};
use orders::preserve_open_order_reduce_only;

use crate::account::{UserFill, normalize_dex_open_order_coins};
use crate::api::{MarketType, spot_symbol_for_indexed_key};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws::WsUserData;

use std::collections::HashSet;

use iced::Task;

#[cfg(test)]
use fills::{chase_fill_summary, prepend_recent_fills};
#[cfg(test)]
use orders::{apply_open_order_to_chase, first_open_chase_oid};

#[cfg(test)]
mod tests;

fn should_repair_account_from_ws(
    connected_address: Option<&str>,
    has_account_data: bool,
    account_loading: bool,
) -> bool {
    connected_address.is_some() && !has_account_data && !account_loading
}

fn fill_is_spot(fill: &UserFill, symbols: &[crate::api::ExchangeSymbol]) -> bool {
    if fill.coin.starts_with('@') || fill.coin.contains('/') {
        return true;
    }
    symbols
        .iter()
        .find(|symbol| symbol.key == fill.coin)
        .or_else(|| spot_symbol_for_indexed_key(symbols, &fill.coin))
        .is_some_and(|symbol| symbol.market_type == MarketType::Spot)
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
                if self.account_loading {
                    self.account_refresh_followup_pending = true;
                    self.account_reconciliation_required = true;
                    return Task::none();
                }
                return self.force_refresh_account_data_for_reconciliation(addr);
            }
            return Task::none();
        }
        let wallet_details_update = ws_data.clone();

        let mut account_data_changed = false;
        let mut orders_changed = false;
        let mut orders_updated_dex = None;
        let mut fills_changed = false;
        let mut positions_changed = false;
        let mut spot_balances_changed = false;
        let mut mids_task = Task::none();
        let mut fill_toast_fills: Vec<UserFill> = Vec::new();
        let mut fresh_fills: Vec<UserFill> = Vec::new();
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
        let account_snapshot_matches_source = source_address
            .as_deref()
            .and_then(|address| self.account_data_for_order_account(address))
            .is_some();
        if self.account_data.is_some()
            && !account_snapshot_matches_source
            && !matches!(ws_data, WsUserData::AllMids(_))
        {
            self.bump_account_data_revision();
            self.account_data = None;
            self.account_data_address = None;
        }
        if account_snapshot_matches_source {
            let source_account = source_address.as_deref().unwrap_or_default();
            let Some(data) = self.account_data_for_order_account_mut(source_account) else {
                return Task::none();
            };
            match ws_data {
                WsUserData::AllDexPositions {
                    main_state,
                    states_by_dex,
                    all_positions,
                    position_details: _,
                } => {
                    data.mark_positions_fetched_at(Self::now_ms());
                    data.clearinghouse.margin_summary = main_state.margin_summary;
                    data.clearinghouse.withdrawable = main_state.withdrawable;
                    data.clearinghouse.cross_margin_summary = main_state.cross_margin_summary;
                    data.clearinghouse.cross_maintenance_margin_used =
                        main_state.cross_maintenance_margin_used;
                    data.clearinghouse.asset_positions = all_positions;
                    data.clearinghouses_by_dex = states_by_dex;
                    account_data_changed = true;
                    positions_changed = true;
                }
                WsUserData::OpenOrders { dex, orders } => {
                    orders_updated_dex = Some(dex.clone());
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
                    data.open_orders.extend(orders);
                    data.mark_open_orders_fetched_at_for_dex(&dex, Self::now_ms());
                    account_data_changed = true;
                    orders_changed = true;
                }
                WsUserData::Fills { fills, is_snapshot } => {
                    if !is_snapshot {
                        let seen: HashSet<String> =
                            data.fills.iter().map(UserFill::dedup_key).collect();
                        fresh_fills = fills
                            .iter()
                            .filter(|fill| !seen.contains(&fill.dedup_key()))
                            .cloned()
                            .collect();
                    }
                    if fresh_fills
                        .iter()
                        .any(|fill| fill_is_spot(fill, &exchange_symbols))
                    {
                        // Fills and spotState are separate websocket lanes. A
                        // fill can arrive first, so the last balance snapshot
                        // is no longer safe for percentage sizing until a new
                        // spotState or full account refresh reconciles it.
                        data.completeness.spot_balances_complete = false;
                        spot_balances_changed = true;
                    }
                    fill_toast_fills =
                        apply_fills_update(&mut data.fills, fills, is_snapshot, is_hidden);
                    account_data_changed = true;
                    fills_changed = true;
                }
                WsUserData::SpotBalances(balances) => {
                    data.spot.balances = balances;
                    data.mark_spot_balances_fetched_at(Self::now_ms());
                    account_data_changed = true;
                    spot_balances_changed = true;
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
                    if self.connected_address.is_some()
                        && self.account_data.is_none()
                        && self.account_loading
                    {
                        // The initial REST fetch may have captured its snapshot
                        // before this frame, but there is no loaded base to
                        // merge the frame into. Let that fetch populate display
                        // state, then require one post-frame snapshot before
                        // order reconciliation.
                        self.account_refresh_followup_pending = true;
                        self.account_reconciliation_required = true;
                    }
                    if should_repair_account_from_ws(
                        self.connected_address.as_deref(),
                        self.account_data.is_some(),
                        self.account_loading,
                    ) && let Some(addr) = &self.connected_address
                    {
                        let addr = addr.clone();
                        let wallet_task = self.apply_wallet_details_ws_update(
                            source_address.clone(),
                            wallet_details_update,
                        );
                        let account_task = self.force_refresh_account_data_for_reconciliation(addr);
                        return Task::batch([wallet_task, account_task]);
                    }
                }
            }
        }

        if account_data_changed {
            self.bump_account_data_revision();
        }
        if spot_balances_changed {
            self.bump_spot_balances_revision();
        }
        if !fresh_fills.is_empty() {
            self.consume_pending_market_order_fills(&fresh_fills);
        }
        for fill in &fill_toast_fills {
            let coin_label = self.display_coin_for_journal(&fill.coin);
            let size_label = self.fill_toast_size_label(fill);
            self.push_toast(fill_toast_message(fill, &coin_label, &size_label), false);
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
            orders_updated_dex
                .as_deref()
                .map(|dex| self.handle_chase_order_disappearance(dex))
                .unwrap_or_else(Task::none)
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

    /// Outcome fills toast in whole contracts; everything else keeps the wire
    /// size string.
    fn fill_toast_size_label(&self, fill: &UserFill) -> String {
        match fill.sz.parse::<f64>() {
            Ok(size) if self.is_outcome_coin(&fill.coin) => {
                self.display_size_for_symbol(&fill.coin, size)
            }
            _ => fill.sz.clone(),
        }
    }
}
