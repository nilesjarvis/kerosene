use crate::account::{OpenOrder, fetch_account_data_scoped, normalize_dex_open_order_coins};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{
    ChaseLifecycle, ChaseOrder, ChaseQueuedAction, ChaseStopPhase, ChaseVerificationReason,
};
use crate::ws::WsUserData;

use iced::Task;
use std::collections::HashSet;

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

#[derive(Debug, Clone)]
struct ChaseFillTotals {
    side: String,
    coin: String,
    filled_size: f64,
    total_notional: f64,
}

fn chase_fill_totals(fills: &[crate::account::UserFill], oids: &[u64]) -> Option<ChaseFillTotals> {
    if oids.is_empty() {
        return None;
    }

    let mut seen = HashSet::new();
    let mut side = None;
    let mut coin = None;
    let mut filled_size = 0.0;
    let mut total_notional = 0.0;
    let mut matched = false;
    for fill in fills {
        let Some(oid) = fill.oid else {
            continue;
        };
        if !oids.contains(&oid) {
            continue;
        }
        let fill_key = (
            oid,
            fill.time,
            fill.px.as_str(),
            fill.sz.as_str(),
            fill.side.as_str(),
            fill.dir.as_str(),
            fill.closed_pnl.as_str(),
            fill.fee.as_str(),
        );
        if !seen.insert(fill_key) {
            continue;
        }
        matched = true;
        side.get_or_insert_with(|| {
            if fill.side == "B" {
                "BUY".to_string()
            } else {
                "SELL".to_string()
            }
        });
        coin.get_or_insert_with(|| fill.coin.clone());
        let Ok(size) = fill.sz.parse::<f64>() else {
            continue;
        };
        let Ok(price) = fill.px.parse::<f64>() else {
            continue;
        };
        if size.is_finite() && size > 0.0 && price.is_finite() && price > 0.0 {
            filled_size += size;
            total_notional += size * price;
        }
    }

    if !matched {
        return None;
    }

    Some(ChaseFillTotals {
        side: side.unwrap_or_else(|| "BUY".to_string()),
        coin: coin.unwrap_or_else(|| "?".to_string()),
        filled_size,
        total_notional,
    })
}

fn chase_fill_summary_for_oids(fills: &[crate::account::UserFill], oids: &[u64]) -> Option<String> {
    let totals = chase_fill_totals(fills, oids)?;

    if totals.filled_size > 0.0 {
        let avg_px = totals.total_notional / totals.filled_size;
        Some(format!(
            "Chase filled: {} {} {} @ ${}",
            totals.side,
            format_chase_fill_number(totals.filled_size),
            totals.coin,
            format_chase_fill_number(avg_px)
        ))
    } else {
        Some("Chase filled".to_string())
    }
}

fn chase_fill_summary_for_chase(
    fills: &[crate::account::UserFill],
    chase: &ChaseOrder,
) -> Option<String> {
    let oids = chase.known_oids_with_current();
    let totals = chase_fill_totals(fills, &oids)?;

    if totals.filled_size > 0.0 {
        let avg_px = totals.total_notional / totals.filled_size;
        Some(format!(
            "Chase filled: {} {} {} @ ${}",
            totals.side,
            format_chase_fill_number(totals.filled_size),
            totals.coin,
            format_chase_fill_number(avg_px)
        ))
    } else {
        Some("Chase filled".to_string())
    }
}

fn chase_fill_summary(fills: &[crate::account::UserFill], oid: u64) -> Option<String> {
    chase_fill_summary_for_oids(fills, &[oid]).map(|summary| {
        if summary == "Chase filled" {
            format!("Chase filled (oid {oid})")
        } else {
            format!("{summary} (oid {oid})")
        }
    })
}

fn apply_open_order_to_chase(
    chase: &mut ChaseOrder,
    order: &OpenOrder,
) -> Result<bool, ()> {
    let sz = order.sz.parse::<f64>().map_err(|_| ())?;
    let oversized = chase.sync_open_remaining_size(sz).ok_or(())?;
    if !chase.remaining_size.is_finite() {
        return Err(());
    }

    chase.record_oid(order.oid);
    if let Ok(px) = order.limit_px.parse::<f64>()
        && let Some((rounded_px, price_wire)) = chase.rounded_price(px)
    {
        chase.current_price = rounded_px;
        chase.current_price_wire = price_wire;
        if chase
            .desired_price
            .and_then(|price| chase.rounded_price(price))
            .is_some_and(|(_, desired_wire)| desired_wire == chase.current_price_wire)
        {
            chase.desired_price = None;
        }
    }
    Ok(oversized)
}

fn first_open_chase_oid(chase: &ChaseOrder, open_orders: &[OpenOrder]) -> Option<u64> {
    chase
        .current_oid
        .filter(|oid| open_orders.iter().any(|order| order.oid == *oid))
        .or_else(|| {
            open_orders
                .iter()
                .find(|order| chase.tracks_oid(order.oid))
                .map(|order| order.oid)
        })
}

fn chase_completed_summary(
    fills: &[crate::account::UserFill],
    chase: &ChaseOrder,
    filled_size: f64,
) -> String {
    let summary = chase_fill_summary_for_chase(fills, chase)
        .unwrap_or_else(|| "Chase completed: target size filled".to_string());
    if chase.target_size.is_finite()
        && chase.target_size > 0.0
        && filled_size > chase.target_size + f64::EPSILON
    {
        let overfill = filled_size - chase.target_size;
        format!(
            "{summary}; over target by {}",
            format_chase_fill_number(overfill)
        )
    } else {
        summary
    }
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
                        let account_task =
                            Task::perform(fetch_account_data_scoped(addr, scope), move |r| {
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

    pub(crate) fn reconcile_chase_fills_from_account(&mut self) -> Task<Message> {
        let Some(data) = self.account_data.as_ref() else {
            return Task::none();
        };
        if !data.completeness.fills_complete {
            return Task::none();
        }
        let fills = data.fills.clone();
        let open_orders = data.open_orders.clone();
        let open_orders_complete = data.completeness.open_orders_complete;
        self.reconcile_chase_fills_from_snapshot(
            &fills,
            open_orders_complete.then_some(open_orders.as_slice()),
            false,
        )
    }

    fn reconcile_chase_fills_from_snapshot(
        &mut self,
        fills: &[crate::account::UserFill],
        open_orders: Option<&[OpenOrder]>,
        open_orders_authoritative: bool,
    ) -> Task<Message> {
        let chase_ids: Vec<u64> = self.chase_orders.keys().copied().collect();
        let mut remove_ids = Vec::new();
        let mut cancel_ids = Vec::new();
        let mut needs_open_order_refresh = false;
        for chase_id in chase_ids {
            let Some(chase) = self.chase_orders.get_mut(&chase_id) else {
                continue;
            };
            let oids = chase.known_oids_with_current();
            let Some(totals) = chase_fill_totals(fills, &oids) else {
                continue;
            };
            chase.set_filled_size(totals.filled_size);
            if chase.residual_size() <= f64::EPSILON {
                let summary = chase_completed_summary(fills, chase, totals.filled_size);
                let is_error = chase.target_size.is_finite()
                    && chase.target_size > 0.0
                    && totals.filled_size > chase.target_size + f64::EPSILON;
                match open_orders {
                    Some(open_orders) => {
                        if let Some(oid) = first_open_chase_oid(chase, open_orders) {
                            cancel_ids.push((chase_id, oid, summary, is_error));
                        } else if open_orders_authoritative {
                            remove_ids.push((chase_id, summary, is_error));
                        } else {
                            chase.lifecycle = ChaseLifecycle::Verifying {
                                reason: ChaseVerificationReason::MissingOrder,
                            };
                            chase.stop_reason = Some((summary.clone(), is_error));
                            needs_open_order_refresh = true;
                            self.order_status = Some((
                                "Chase target filled; refreshing open orders before closing".into(),
                                is_error,
                            ));
                        }
                    }
                    None => {
                        chase.lifecycle = ChaseLifecycle::Verifying {
                            reason: ChaseVerificationReason::MissingOrder,
                        };
                        chase.stop_reason = Some((summary.clone(), is_error));
                        needs_open_order_refresh = true;
                        self.order_status = Some((
                            "Chase target filled; verifying open orders before closing".into(),
                            is_error,
                        ));
                    }
                }
            }
        }

        for (chase_id, summary, is_error) in remove_ids {
            self.order_status = Some((summary.clone(), is_error));
            self.remove_chase_order_with_summary(chase_id, Some(summary));
        }
        let mut tasks = Vec::new();
        for (chase_id, oid, summary, is_error) in cancel_ids {
            tasks.push(self.cancel_known_chase_order_for_safety(chase_id, oid, summary, is_error));
        }
        if needs_open_order_refresh {
            tasks.push(self.refresh_account_data());
        }
        Task::batch(tasks)
    }

    fn handle_chase_order_disappearance(&mut self) -> Task<Message> {
        let mut needs_refresh = false;
        let open_orders = self
            .account_data
            .as_ref()
            .map(|data| data.open_orders.clone())
            .unwrap_or_default();
        let chase_ids: Vec<u64> = self.chase_orders.keys().copied().collect();
        let mut cancel_ids = Vec::new();

        for chase_id in chase_ids {
            let Some((oid, lifecycle, has_pending)) = self
                .chase_orders
                .get(&chase_id)
                .map(|chase| (chase.current_oid, chase.lifecycle, chase.has_pending_op()))
            else {
                continue;
            };
            let Some(oid) = oid else {
                continue;
            };
            if has_pending {
                continue;
            }
            if lifecycle.is_stopping() {
                continue;
            }
            match open_orders.iter().find(|order| order.oid == oid) {
                Some(order) => {
                    let Some(chase) = self.chase_orders.get_mut(&chase_id) else {
                        continue;
                    };
                    match apply_open_order_to_chase(chase, order) {
                        Ok(oversized) => {
                            if oversized {
                                chase.lifecycle = ChaseLifecycle::Verifying {
                                    reason: ChaseVerificationReason::SizeCorrection,
                                };
                                self.order_status = Some((
                                    "Chase verifying fills before correcting remaining size".into(),
                                    false,
                                ));
                                needs_refresh = true;
                            } else if matches!(lifecycle, ChaseLifecycle::Resting)
                                && !chase.lifecycle.is_stopping()
                            {
                                self.order_status = Some((format!("Chasing (oid {oid})..."), false));
                            }
                        }
                        Err(()) => {
                            self.order_status = Some((
                                "Chase stopped: invalid remaining size from open orders".into(),
                                true,
                            ));
                            cancel_ids.push((
                                chase_id,
                                oid,
                                "Chase stopped: invalid remaining size from open orders"
                                    .to_string(),
                                true,
                            ));
                        }
                    }
                }
                None => {
                    if matches!(lifecycle, ChaseLifecycle::Resting) {
                        if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                            chase.lifecycle = ChaseLifecycle::Verifying {
                                reason: ChaseVerificationReason::MissingOrder,
                            };
                        }
                        self.order_status = Some((
                            "Chase checking order status: open-orders stream no longer shows the order"
                                .into(),
                            false,
                        ));
                        needs_refresh = true;
                    }
                }
            }
        }
        let mut tasks = Vec::new();
        for (chase_id, oid, summary, is_error) in cancel_ids {
            tasks.push(self.cancel_known_chase_order_for_safety(chase_id, oid, summary, is_error));
        }
        if needs_refresh {
            tasks.push(self.refresh_account_data());
        }
        Task::batch(tasks)
    }

    pub(crate) fn reconcile_chase_after_account_refresh(&mut self) -> Task<Message> {
        let Some(data) = self.account_data.as_ref() else {
            return Task::none();
        };
        let open_orders = data.open_orders.clone();
        let fills = data.fills.clone();
        let open_orders_complete = data.completeness.open_orders_complete;
        let fills_complete = data.completeness.fills_complete;
        let connected_address = self.connected_address.clone();
        let mut tasks = Vec::new();
        if fills_complete {
            tasks.push(self.reconcile_chase_fills_from_snapshot(
                &fills,
                open_orders_complete.then_some(open_orders.as_slice()),
                true,
            ));
        }
        let chase_ids: Vec<u64> = self.chase_orders.keys().copied().collect();
        let mut remove_ids = Vec::new();
        let mut correction_ids = Vec::new();
        let mut replacement_ids = Vec::new();
        let mut status_check_ids = Vec::new();
        let mut cancel_ids = Vec::new();

        for chase_id in chase_ids {
            let Some(chase_snapshot) = self.chase_orders.get(&chase_id) else {
                continue;
            };
            if connected_address.as_deref() != Some(chase_snapshot.account_address.as_str())
                || chase_snapshot.has_pending_op()
            {
                continue;
            }
            if let ChaseLifecycle::Stopping {
                phase: ChaseStopPhase::VerifyingCancel { .. },
            } = chase_snapshot.lifecycle
            {
                if !open_orders_complete || !fills_complete {
                    self.order_status = Some((
                        "Chase stopping: waiting for complete account snapshot before clearing"
                            .into(),
                        true,
                    ));
                    continue;
                }
                if let Some(oid) = first_open_chase_oid(chase_snapshot, &open_orders) {
                    let (summary, is_error) = chase_snapshot
                        .stop_reason
                        .clone()
                        .unwrap_or_else(|| ("Chase stopped".to_string(), false));
                    cancel_ids.push((chase_id, oid, summary, is_error));
                } else {
                    let (summary, is_error) = chase_snapshot
                        .stop_reason
                        .clone()
                        .unwrap_or_else(|| ("Chase stopped".to_string(), false));
                    remove_ids.push((chase_id, summary, is_error));
                }
                continue;
            }
            if !chase_snapshot.needs_account_verification()
                || chase_snapshot.lifecycle.is_stopping()
            {
                continue;
            }
            let verification_reason = match chase_snapshot.lifecycle {
                ChaseLifecycle::Verifying { reason } => reason,
                _ => continue,
            };
            let wants_replacement = chase_snapshot.desired_price.is_some();
            if !open_orders_complete || !fills_complete {
                self.order_status = Some((
                    "Chase paused: account refresh was incomplete; not mutating until fills and open orders are verified"
                        .into(),
                    true,
                ));
                continue;
            }
            if chase_snapshot.residual_size() <= f64::EPSILON {
                let status = chase_fill_summary_for_chase(&fills, chase_snapshot)
                    .unwrap_or_else(|| "Chase completed: target size filled".to_string());
                let is_error = chase_snapshot.target_size.is_finite()
                    && chase_snapshot.target_size > 0.0
                    && chase_snapshot.filled_size > chase_snapshot.target_size + f64::EPSILON;
                if let Some(oid) = first_open_chase_oid(chase_snapshot, &open_orders) {
                    cancel_ids.push((chase_id, oid, status, is_error));
                } else {
                    remove_ids.push((chase_id, status, is_error));
                }
                continue;
            }

            let Some(oid) = chase_snapshot.current_oid else {
                if matches!(verification_reason, ChaseVerificationReason::Placement) {
                    self.order_status = Some((
                        "Chase placement status is still uncertain; waiting for orderStatus before placing another order"
                            .into(),
                        true,
                    ));
                    continue;
                }
                if matches!(
                    verification_reason,
                    ChaseVerificationReason::MissingOrderResolvedNoFill
                ) && wants_replacement
                {
                    replacement_ids.push(chase_id);
                } else if wants_replacement {
                    self.order_status = Some((
                        "Chase replacement blocked: previous order is still unresolved".into(),
                        true,
                    ));
                }
                continue;
            };

            let order = open_orders.iter().find(|order| order.oid == oid);
            match order {
                Some(order) => {
                    let mut stop_after_refresh = None;
                    if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                        match apply_open_order_to_chase(chase, order) {
                            Ok(oversized) => {
                                let needs_reconcile = chase.desired_price.is_some()
                                    || oversized
                                    || matches!(
                                        verification_reason,
                                        ChaseVerificationReason::SizeCorrection
                                    );
                                if chase.lifecycle.is_stopping() {
                                    stop_after_refresh = chase
                                        .stop_reason
                                        .clone()
                                        .or_else(|| Some(("Chase stopped".to_string(), false)));
                                } else if needs_reconcile {
                                    correction_ids.push(chase_id);
                                } else {
                                    chase.lifecycle = ChaseLifecycle::Resting;
                                    self.order_status =
                                        Some((format!("Chasing (oid {oid})..."), false));
                                }
                            }
                            Err(()) => {
                                self.order_status = Some((
                                    "Chase stopped: invalid remaining size from account refresh"
                                        .into(),
                                    true,
                                ));
                                cancel_ids.push((
                                    chase_id,
                                    order.oid,
                                    "Chase stopped: invalid remaining size from account refresh"
                                        .to_string(),
                                    true,
                                ));
                            }
                        }
                    }
                    if let Some((reason, is_error)) = stop_after_refresh {
                        tasks.push(self.stop_chase_by_id_with_reason(chase_id, reason, is_error));
                    }
                }
                None
                    if open_orders_complete
                        && wants_replacement
                        && matches!(
                            verification_reason,
                            ChaseVerificationReason::MissingOrderResolvedNoFill
                        ) =>
                {
                    if let Some(chase) = self.chase_orders.get_mut(&chase_id) {
                        chase.record_oid(oid);
                        chase.current_oid = None;
                        chase.lifecycle = ChaseLifecycle::Queued {
                            action: ChaseQueuedAction::Place,
                        };
                    }
                    replacement_ids.push(chase_id);
                }
                None if open_orders_complete && wants_replacement => {
                    status_check_ids.push((chase_id, oid));
                }
                None if open_orders_complete => {
                    if matches!(
                        verification_reason,
                        ChaseVerificationReason::MissingOrderResolvedNoFill
                    ) {
                        let status = chase_fill_summary_for_chase(&fills, chase_snapshot)
                            .or_else(|| chase_fill_summary(&fills, oid))
                            .unwrap_or_else(|| "Chase ended: order no longer open".to_string());
                        self.order_status = Some((status.clone(), false));
                        remove_ids.push((chase_id, status, false));
                    } else {
                        status_check_ids.push((chase_id, oid));
                    }
                }
                None => {
                    self.order_status = Some((
                        "Chase status uncertain: open orders refresh was incomplete".into(),
                        true,
                    ));
                }
            }
        }

        for (chase_id, summary, is_error) in remove_ids {
            self.order_status = Some((summary.clone(), is_error));
            self.remove_chase_order_with_summary(chase_id, Some(summary));
        }
        for (chase_id, oid, summary, is_error) in cancel_ids {
            tasks.push(self.cancel_known_chase_order_for_safety(chase_id, oid, summary, is_error));
        }
        tasks.extend(status_check_ids.into_iter().map(|(chase_id, oid)| {
            self.check_chase_order_status(
                chase_id,
                oid,
                "Chase checking order status before replacement",
            )
        }));
        tasks.extend(
            correction_ids
                .into_iter()
                .map(|chase_id| self.chase_modify_for_current_price_reconciliation(chase_id)),
        );
        let replacements: Vec<_> = replacement_ids
            .into_iter()
            .filter_map(|chase_id| {
                self.chase_orders
                    .get(&chase_id)
                    .and_then(|chase| chase.desired_price)
                    .map(|best| (chase_id, best))
            })
            .collect();
        tasks.extend(
            replacements
                .into_iter()
                .map(|(chase_id, best)| self.chase_place_at_best(chase_id, best)),
        );
        Task::batch(tasks)
    }
}
