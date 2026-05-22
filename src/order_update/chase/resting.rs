use crate::api::MarketType;
use crate::app_state::TradingTerminal;
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

impl TradingTerminal {
    pub(crate) fn handle_chase_resting_order(
        &mut self,
        coin: String,
        oid: u64,
        is_buy: bool,
        sz: f64,
        limit_px: f64,
        reduce_only: Option<bool>,
    ) -> Task<Message> {
        let key = self.wallet_key_input.trim().to_string();
        if key.is_empty() || self.connected_address.is_none() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        }
        let Some(account_address) = self.connected_address.clone() else {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        };
        if self.symbol_key_is_hidden(&coin) {
            self.order_status = Some(("Order ticker is hidden in Settings > Risk".into(), true));
            return Task::none();
        }
        if !sz.is_finite() || sz <= 0.0 {
            self.order_status = Some(("Cannot chase order with invalid size".into(), true));
            return Task::none();
        }
        if !limit_px.is_finite() || limit_px <= 0.0 {
            self.order_status = Some(("Cannot chase order with invalid price".into(), true));
            return Task::none();
        }

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
                    "Cannot start chase: maximum of {MAX_ACTIVE_ADVANCED_ORDERS} active advanced orders reached"
                ),
                true,
            ));
            return Task::none();
        }

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
        let reduce_only = match chase_resting_reduce_only(symbol.market_type, reduce_only) {
            Ok(reduce_only) => reduce_only,
            Err(message) => {
                self.order_status = Some((message.into(), true));
                return Task::none();
            }
        };
        let rounded_px = round_price(limit_px, sz_decimals, is_spot);
        if !rounded_px.is_finite() || rounded_px <= 0.0 {
            self.order_status = Some(("Cannot chase order with invalid price".into(), true));
            return Task::none();
        }
        let chase_id = self.next_chase_id();
        let started_at = std::time::Instant::now();
        let started_at_ms = Self::now_ms();

        self.chase_orders.insert(
            chase_id,
            ChaseOrder {
                id: chase_id,
                coin: coin.clone(),
                account_address,
                agent_key: key.into(),
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
