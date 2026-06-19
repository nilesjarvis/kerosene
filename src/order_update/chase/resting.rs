use crate::account::OpenOrder;
use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers::{parse_positive_finite_number, positive_finite_value};
use crate::message::Message;
use crate::signing::{ChaseLifecycle, ChaseOrder, float_to_wire, round_price};
use crate::twap_state::MAX_ACTIVE_ADVANCED_ORDERS;

use iced::Task;

#[cfg(test)]
mod tests;

fn chase_resting_reduce_only(
    market_type: MarketType,
    reduce_only: Option<bool>,
) -> Result<bool, &'static str> {
    if market_type == MarketType::Spot {
        return Ok(false);
    }
    reduce_only.ok_or(
        "Cannot chase order: reduce-only metadata is unavailable; refresh account data first",
    )
}

fn chase_resting_order_is_buy(side: &str) -> Option<bool> {
    match side {
        "B" => Some(true),
        "A" => Some(false),
        _ => None,
    }
}

fn chase_resting_order_wire_is_supported(order: &OpenOrder) -> Result<(), &'static str> {
    if order.is_trigger == Some(true)
        || order
            .trigger_px
            .as_deref()
            .and_then(parse_positive_finite_number)
            .is_some()
    {
        return Err("Cannot chase order: trigger orders cannot be chased safely yet");
    }
    if order
        .order_type
        .as_deref()
        .is_some_and(|kind| !kind.eq_ignore_ascii_case("limit"))
    {
        return Err("Cannot chase order: order type cannot be chased safely yet");
    }
    if order
        .tif
        .as_deref()
        .is_some_and(|tif| !tif.eq_ignore_ascii_case("Gtc"))
    {
        return Err("Cannot chase order: non-GTC orders cannot be chased safely yet");
    }
    Ok(())
}

impl TradingTerminal {
    pub(crate) fn handle_chase_resting_order(&mut self, coin: String, oid: u64) -> Task<Message> {
        if self.has_pending_cancel_indicator(oid) {
            self.order_status = Some((
                format!("Wait for pending cancel of order {oid} before starting a Chase"),
                true,
            ));
            return Task::none();
        }
        if self.reject_if_pending_trading_request("starting a Chase") {
            return Task::none();
        }

        let Some((key, account_address)) = self.captured_order_signing_context() else {
            return Task::none();
        };
        if self.symbol_key_is_hidden(&coin) {
            self.order_status = Some(("Order ticker is hidden in Settings > Risk".into(), true));
            return Task::none();
        }

        if self.account_loading {
            self.order_status = Some((
                "Account refresh in progress; wait for fresh open orders before starting chase"
                    .into(),
                true,
            ));
            return Task::none();
        }
        if self.reject_if_account_reconciliation_required("starting chase", "open orders") {
            return Task::none();
        }
        let order = {
            let Some(account_data) = self.account_data_for_order_account(&account_address) else {
                self.order_status = Some((
                    "No account data available; refresh before starting chase".into(),
                    true,
                ));
                return Task::none();
            };
            let now_ms = Self::now_ms();
            if !account_data.completeness.open_orders_complete {
                self.order_status = Some((
                    "Open orders are incomplete; refresh before starting chase".into(),
                    true,
                ));
                return self.refresh_account_data();
            }
            if !account_data.is_fresh_for_open_order_action_for_symbol(&coin, now_ms) {
                let age_label = account_data
                    .open_order_action_snapshot_age_ms_for_symbol(&coin, now_ms)
                    .map(|age| format!("{}s old", age.div_ceil(1000)))
                    .unwrap_or_else(|| "from the future".to_string());
                self.order_status = Some((
                    format!("Open orders are stale ({age_label}); refresh before starting chase"),
                    true,
                ));
                return self.refresh_account_data();
            }
            let Some(order) = account_data
                .open_orders
                .iter()
                .find(|order| order.oid == oid && order.coin == coin)
                .cloned()
            else {
                self.order_status = Some(("Order no longer exists".into(), true));
                return Task::none();
            };
            order
        };

        if self
            .chase_orders
            .values()
            .any(|chase| chase.current_oid == Some(oid))
        {
            return Task::none();
        }
        if self.active_advanced_order_count() >= MAX_ACTIVE_ADVANCED_ORDERS {
            self.order_status = Some((
                format!(
                    concat!(
                        "Cannot start chase: maximum of ",
                        "{} active advanced orders reached"
                    ),
                    MAX_ACTIVE_ADVANCED_ORDERS
                ),
                true,
            ));
            return Task::none();
        }

        if let Err(message) = chase_resting_order_wire_is_supported(&order) {
            self.order_status = Some((message.into(), true));
            return Task::none();
        }
        let Some(is_buy) = chase_resting_order_is_buy(&order.side) else {
            self.order_status = Some((
                "Cannot chase order: open order has invalid side".into(),
                true,
            ));
            return Task::none();
        };
        let Some(sz) = order.sz.parse::<f64>().ok().and_then(positive_finite_value) else {
            self.order_status = Some(("Cannot chase order with invalid size".into(), true));
            return Task::none();
        };
        let Some(limit_px) = order
            .limit_px
            .parse::<f64>()
            .ok()
            .and_then(positive_finite_value)
        else {
            self.order_status = Some(("Cannot chase order with invalid price".into(), true));
            return Task::none();
        };

        let symbol = self.exchange_symbols.iter().find(|s| s.key == coin);
        let Some(symbol) = symbol else {
            self.order_status = Some((format!("Symbol '{coin}' not found"), true));
            return Task::none();
        };
        if symbol.market_type == MarketType::Outcome {
            self.outcome_read_only_status("chase trading");
            return Task::none();
        }

        let asset = symbol.asset_index;
        let sz_decimals = symbol.sz_decimals;
        let is_spot = symbol.market_type == MarketType::Spot;
        let reduce_only = match chase_resting_reduce_only(symbol.market_type, order.reduce_only) {
            Ok(reduce_only) => reduce_only,
            Err(message) => {
                self.order_status = Some((message.into(), true));
                return Task::none();
            }
        };
        let rounded_px = round_price(limit_px, sz_decimals, is_spot);
        let Some(rounded_px) = positive_finite_value(rounded_px) else {
            self.order_status = Some(("Cannot chase order with invalid price".into(), true));
            return Task::none();
        };
        let chase_id = self.next_chase_id();
        let started_at = std::time::Instant::now();
        let started_at_ms = Self::now_ms();

        self.chase_orders.insert(
            chase_id,
            ChaseOrder {
                id: chase_id,
                coin: coin.clone(),
                account_address,
                agent_key: key,
                is_buy,
                target_size: sz,
                filled_size: 0.0,
                remaining_size: sz,
                known_oids: vec![oid],
                current_cloid: None,
                place_attempt_count: 0,
                asset,
                sz_decimals,
                is_spot,
                reduce_only,
                current_oid: Some(oid),
                current_price: rounded_px,
                current_price_wire: float_to_wire(rounded_px),
                initial_price: rounded_px,
                started_at,
                started_at_ms,
                fill_cutoff_ms_by_oid: vec![(
                    oid,
                    ChaseOrder::adopted_fill_cutoff_ms(started_at_ms),
                )],
                reprice_count: 0,
                lifecycle: ChaseLifecycle::Resting,
                last_reprice_at: None,
                desired_price: None,
                stop_reason: None,
                cancel_retries: 0,
            },
        );
        self.selected_chase_id = Some(chase_id);

        self.order_status = Some((format!("Chasing resting order {oid}..."), false));
        Task::none()
    }
}
