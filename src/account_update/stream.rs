use crate::account::{OpenOrder, fetch_account_data, normalize_dex_open_order_coins};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::ChaseOrder;
use crate::ws::WsUserData;

use iced::Task;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChaseOpenOrderPriceSync {
    Trust,
    PreserveExpectedIfUnconfirmed,
}

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

fn apply_fills_update<F>(
    existing: &mut Vec<crate::account::UserFill>,
    fills: Vec<crate::account::UserFill>,
    is_snapshot: bool,
    is_muted: F,
) -> Vec<String>
where
    F: Fn(&str) -> bool,
{
    let fills: Vec<_> = fills
        .into_iter()
        .filter(|fill| !is_muted(&fill.coin))
        .collect();
    if is_snapshot {
        *existing = fills;
        Vec::new()
    } else {
        let toast_msgs: Vec<String> = fills
            .iter()
            .map(|fill| {
                let side = if fill.side == "B" { "BUY" } else { "SELL" };
                format!("Filled {side} {} {} @ ${}", fill.sz, fill.coin, fill.px)
            })
            .collect();
        prepend_recent_fills(existing, fills, 200);
        toast_msgs
    }
}

fn chase_fill_summary(fills: &[crate::account::UserFill], oid: u64) -> Option<String> {
    let matching: Vec<_> = fills.iter().filter(|fill| fill.oid == Some(oid)).collect();
    if matching.is_empty() {
        return None;
    }

    let first = matching[0];
    let side = if first.side == "B" { "BUY" } else { "SELL" };
    let coin = first.coin.as_str();
    let mut total_size = 0.0;
    let mut total_notional = 0.0;
    for fill in matching {
        let Ok(size) = fill.sz.parse::<f64>() else {
            continue;
        };
        let Ok(price) = fill.px.parse::<f64>() else {
            continue;
        };
        if size.is_finite() && size > 0.0 && price.is_finite() && price > 0.0 {
            total_size += size;
            total_notional += size * price;
        }
    }

    if total_size > 0.0 {
        let avg_px = total_notional / total_size;
        Some(format!(
            "Chase filled: {side} {} {coin} @ ${} (oid {oid})",
            format_chase_fill_number(total_size),
            format_chase_fill_number(avg_px)
        ))
    } else {
        Some(format!("Chase filled (oid {oid})"))
    }
}

fn apply_open_order_to_chase(
    chase: &mut ChaseOrder,
    order: &OpenOrder,
    price_sync: ChaseOpenOrderPriceSync,
) -> Result<(), ()> {
    let sz = order.sz.parse::<f64>().map_err(|_| ())?;
    if !sz.is_finite() || sz <= 0.0 {
        return Err(());
    }

    chase.remaining_size = sz;
    if let Ok(px) = order.limit_px.parse::<f64>()
        && let Some((rounded_px, price_wire)) = chase.rounded_price(px)
    {
        let preserve_expected = matches!(
            price_sync,
            ChaseOpenOrderPriceSync::PreserveExpectedIfUnconfirmed
        ) && !chase.oid_confirmed
            && price_wire != chase.current_price_wire;
        if !preserve_expected {
            chase.current_price = rounded_px;
            chase.current_price_wire = price_wire;
        }
    }
    chase.oid_confirmed = true;
    chase.missing_open_order_refresh_requested = false;
    Ok(())
}

fn format_chase_fill_number(value: f64) -> String {
    let formatted = format!("{value:.8}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
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
        let mut mids_task = Task::none();
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
                    data.fetched_at_ms = Self::now_ms();
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
                    data.open_orders.retain(|order| !is_muted(&order.coin));
                    data.open_orders
                        .extend(orders.into_iter().filter(|order| !is_muted(&order.coin)));
                    orders_changed = true;
                }
                WsUserData::Fills { fills, is_snapshot } => {
                    fill_toast_msgs =
                        apply_fills_update(&mut data.fills, fills, is_snapshot, is_muted);
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
                        let account_task = Task::perform(fetch_account_data(addr), move |r| {
                            Message::AccountDataLoaded(requested_addr.clone(), Box::new(r))
                        });
                        return Task::batch([wallet_task, account_task]);
                    }
                }
            }
        }

        for msg in fill_toast_msgs {
            self.push_toast(msg, false);
        }
        if orders_changed {
            self.sync_all_chart_orders();
        }
        if fills_changed {
            self.sync_all_chart_trade_markers();
            self.reconcile_twap_fills_from_account();
        }
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
        Task::batch([chase_task, mids_task, wallet_task])
    }

    fn handle_chase_order_disappearance(&mut self) -> Task<Message> {
        let mut refresh_for_missing_order = false;
        let open_orders = self
            .account_data
            .as_ref()
            .map(|data| data.open_orders.clone())
            .unwrap_or_default();
        let chase_ids: Vec<u64> = self.chase_orders.keys().copied().collect();
        let mut remove_ids = Vec::new();

        for chase_id in chase_ids {
            let Some((oid, confirmed, refresh_requested, has_pending)) =
                self.chase_orders.get(&chase_id).map(|chase| {
                    (
                        chase.current_oid,
                        chase.oid_confirmed,
                        chase.missing_open_order_refresh_requested,
                        chase.has_pending_op(),
                    )
                })
            else {
                continue;
            };
            let Some(oid) = oid else {
                continue;
            };
            if has_pending {
                continue;
            }
            match open_orders.iter().find(|order| order.oid == oid) {
                Some(order) => {
                    if let Some(chase) = self.chase_orders.get_mut(&chase_id)
                        && apply_open_order_to_chase(
                            chase,
                            order,
                            ChaseOpenOrderPriceSync::PreserveExpectedIfUnconfirmed,
                        )
                        .is_err()
                    {
                        self.order_status = Some((
                            "Chase stopped: invalid remaining size from open orders".into(),
                            true,
                        ));
                        remove_ids.push((
                            chase_id,
                            "Chase stopped: invalid remaining size from open orders".to_string(),
                        ));
                    }
                }
                None => {
                    if confirmed && !refresh_requested {
                        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                            chase.missing_open_order_refresh_requested = true;
                        }
                        self.order_status = Some((
                            "Chase checking order status: open-orders stream no longer shows the order"
                                .into(),
                            false,
                        ));
                        refresh_for_missing_order = true;
                    }
                }
            }
        }
        for (chase_id, summary) in remove_ids {
            self.remove_chase_order_with_summary(chase_id, Some(summary));
        }
        if refresh_for_missing_order {
            return self.refresh_account_data();
        }
        Task::none()
    }

    pub(crate) fn reconcile_chase_after_account_refresh(&mut self) -> Task<Message> {
        let Some(data) = self.account_data.as_ref() else {
            return Task::none();
        };
        let open_orders = data.open_orders.clone();
        let fills = data.fills.clone();
        let open_orders_complete = data.completeness.open_orders_complete;
        let connected_address = self.connected_address.clone();
        let chase_ids: Vec<u64> = self.chase_orders.keys().copied().collect();
        let mut tasks = Vec::new();
        let mut remove_ids = Vec::new();

        for chase_id in chase_ids {
            let Some(chase_snapshot) = self.chase_orders.get(&chase_id) else {
                continue;
            };
            if connected_address.as_deref() != Some(chase_snapshot.account_address.as_str())
                || !chase_snapshot.missing_open_order_refresh_requested
                || chase_snapshot.has_pending_op()
            {
                continue;
            }
            let Some(oid) = chase_snapshot.current_oid else {
                continue;
            };

            let order = open_orders.iter().find(|order| order.oid == oid);
            match order {
                Some(order) => {
                    let mut stop_after_refresh = None;
                    if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                        if apply_open_order_to_chase(chase, order, ChaseOpenOrderPriceSync::Trust)
                            .is_err()
                        {
                            self.order_status = Some((
                                "Chase stopped: invalid remaining size from account refresh".into(),
                                true,
                            ));
                            remove_ids.push((
                                chase_id,
                                "Chase stopped: invalid remaining size from account refresh"
                                    .to_string(),
                            ));
                        } else if chase.stop_requested {
                            stop_after_refresh = chase
                                .stop_reason
                                .clone()
                                .or_else(|| Some(("Chase stopped".to_string(), false)));
                        } else {
                            self.order_status = Some((format!("Chasing (oid {oid})..."), false));
                        }
                    }
                    if let Some((reason, is_error)) = stop_after_refresh {
                        tasks.push(self.stop_chase_by_id_with_reason(chase_id, reason, is_error));
                    }
                }
                None if open_orders_complete => {
                    let status = chase_fill_summary(&fills, oid)
                        .unwrap_or_else(|| "Chase ended: order no longer open".to_string());
                    self.order_status = Some((status.clone(), false));
                    remove_ids.push((chase_id, status));
                }
                None => {
                    self.order_status = Some((
                        "Chase status uncertain: open orders refresh was incomplete".into(),
                        true,
                    ));
                }
            }
        }

        for (chase_id, summary) in remove_ids {
            self.remove_chase_order_with_summary(chase_id, Some(summary));
        }
        Task::batch(tasks)
    }
}
