use crate::account::fetch_account_data;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::ws::WsUserData;

use iced::Task;

#[cfg(test)]
mod tests;

fn preserve_open_order_reduce_only(
    order: &mut crate::account::OpenOrder,
    existing: &[crate::account::OpenOrder],
) {
    if order.reduce_only.is_none()
        && let Some(previous) = existing.iter().find(|previous| previous.oid == order.oid)
    {
        order.reduce_only = previous.reduce_only;
    }
}

fn should_repair_account_from_ws(
    connected_address: Option<&str>,
    has_account_data: bool,
    account_loading: bool,
) -> bool {
    connected_address.is_some() && !has_account_data && !account_loading
}

fn prepend_recent_fills(
    existing: &mut Vec<crate::account::UserFill>,
    incoming: Vec<crate::account::UserFill>,
    max_len: usize,
) {
    if max_len == 0 {
        existing.clear();
        return;
    }

    let mut updated =
        Vec::with_capacity(max_len.min(existing.len().saturating_add(incoming.len())));
    updated.extend(incoming.into_iter().take(max_len));
    let remaining = max_len.saturating_sub(updated.len());
    updated.extend(existing.drain(..).take(remaining));
    *existing = updated;
}

impl TradingTerminal {
    pub(super) fn apply_ws_user_data_update(
        &mut self,
        source_address: Option<String>,
        ws_data: WsUserData,
    ) -> Task<Message> {
        let wallet_details_update = ws_data.clone();
        if source_address.as_deref() != self.connected_address.as_deref() {
            self.apply_wallet_details_ws_update(source_address, ws_data);
            return Task::none();
        }

        let mut orders_changed = false;
        let mut fill_toast_msgs: Vec<String> = Vec::new();
        let exchange_symbols = self.exchange_symbols.clone();
        let muted_tickers = self.muted_tickers.clone();
        let is_muted = |symbol: &str| {
            Self::key_matches_muted_tickers(&exchange_symbols, &muted_tickers, symbol)
        };
        if let Some(data) = &mut self.account_data {
            match ws_data {
                WsUserData::AllDexPositions {
                    main_state,
                    all_positions,
                    position_details: _,
                } => {
                    data.clearinghouse.margin_summary = main_state.margin_summary;
                    data.clearinghouse.withdrawable = main_state.withdrawable;
                    data.clearinghouse.cross_margin_summary = main_state.cross_margin_summary;
                    data.clearinghouse.cross_maintenance_margin_used =
                        main_state.cross_maintenance_margin_used;
                    data.clearinghouse.asset_positions = all_positions
                        .into_iter()
                        .filter(|position| !is_muted(&position.position.coin))
                        .collect();
                    self.sync_all_chart_overlays();
                }
                WsUserData::OpenOrders { dex, orders } => {
                    let mut orders = orders;
                    for order in &mut orders {
                        preserve_open_order_reduce_only(order, &data.open_orders);
                    }
                    if dex.is_empty() {
                        data.open_orders.retain(|o| o.coin.contains(':'));
                    } else {
                        let prefix = format!("{dex}:");
                        data.open_orders.retain(|o| !o.coin.starts_with(&prefix));
                    }
                    data.open_orders.retain(|order| !is_muted(&order.coin));
                    data.open_orders
                        .extend(orders.into_iter().filter(|order| !is_muted(&order.coin)));
                    orders_changed = true;
                }
                WsUserData::Fills { fills, is_snapshot } => {
                    let fills: Vec<_> = fills
                        .into_iter()
                        .filter(|fill| !is_muted(&fill.coin))
                        .collect();
                    if is_snapshot {
                        data.fills = fills;
                    } else {
                        let toast_msgs: Vec<String> = fills
                            .iter()
                            .map(|fill| {
                                let side = if fill.side == "B" { "BUY" } else { "SELL" };
                                format!("Filled {side} {} {} @ ${}", fill.sz, fill.coin, fill.px)
                            })
                            .collect();
                        prepend_recent_fills(&mut data.fills, fills, 200);
                        fill_toast_msgs = toast_msgs;
                    }
                }
                WsUserData::SpotBalances(balances) => {
                    data.spot.balances = balances
                        .into_iter()
                        .filter(|balance| !is_muted(&balance.coin))
                        .collect();
                }
                WsUserData::AllMids(mids) => {
                    self.handle_mids_update(mids);
                }
            }
        } else {
            match ws_data {
                WsUserData::AllMids(mids) => {
                    self.handle_mids_update(mids);
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
                        self.apply_wallet_details_ws_update(
                            source_address.clone(),
                            wallet_details_update,
                        );
                        return Task::perform(fetch_account_data(addr), move |r| {
                            Message::AccountDataLoaded(requested_addr.clone(), Box::new(r))
                        });
                    }
                }
            }
        }

        for msg in fill_toast_msgs {
            self.push_toast(msg, false);
        }
        if orders_changed {
            self.sync_all_chart_orders();
            self.handle_chase_order_disappearance();
        }
        self.apply_wallet_details_ws_update(source_address, wallet_details_update);
        Task::none()
    }

    fn handle_chase_order_disappearance(&mut self) {
        if let Some(chase) = &mut self.active_chase
            && let Some(oid) = chase.current_oid
            && !chase.cancel_in_flight
        {
            let order_sz = self.account_data.as_ref().and_then(|data| {
                data.open_orders
                    .iter()
                    .find(|order| order.oid == oid)
                    .map(|order| order.sz.parse::<f64>())
            });
            match order_sz {
                Some(Ok(sz)) if sz.is_finite() && sz > 0.0 => {
                    chase.remaining_size = sz;
                    chase.oid_confirmed = true;
                }
                Some(_) => {
                    self.order_status = Some((
                        "Chase stopped: invalid remaining size from open orders".into(),
                        true,
                    ));
                    self.active_chase = None;
                }
                None => {
                    if chase.oid_confirmed {
                        self.order_status = Some(("Chase filled".to_string(), false));
                        self.active_chase = None;
                    }
                }
            }
        }
    }
}
