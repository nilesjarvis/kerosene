use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::PendingOrderAction;
use crate::signing::{ChaseOrder, float_to_wire, round_price};
use iced::Task;

mod lifecycle;

// ---------------------------------------------------------------------------
// Chase Order Helpers
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn start_chase(&mut self, is_buy: bool) -> Task<Message> {
        let _theme = self.theme();
        if self.active_chase.is_some() {
            self.order_status =
                Some(("Stop the active chase before starting another".into(), true));
            return Task::none();
        }

        let key = self.wallet_key_input.trim().to_string();
        if key.is_empty() || self.connected_address.is_none() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        }
        let Some(account_address) = self.connected_address.clone() else {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        };
        if self.is_ticker_muted(&self.active_symbol) {
            self.order_status = Some(("Active ticker is muted in Settings > Risk".into(), true));
            return Task::none();
        }

        let qty: f64 = match self.order_quantity.parse::<f64>() {
            Ok(v) if v.is_finite() && v > 0.0 => v,
            _ => {
                self.order_status = Some(("Invalid quantity".into(), true));
                return Task::none();
            }
        };

        let sym = self
            .exchange_symbols
            .iter()
            .find(|s| s.key == self.active_symbol);
        let Some(sym) = sym else {
            self.order_status = Some((format!("Symbol '{}' not found", self.active_symbol), true));
            return Task::none();
        };
        if sym.market_type == MarketType::Outcome {
            if let Err(e) = self.validate_outcome_contract_size(qty) {
                self.order_status = Some((e, true));
            } else {
                self.outcome_read_only_status("chase trading");
            }
            return Task::none();
        }

        let best_px = self.best_chase_price(&self.active_symbol, is_buy);
        let Some(best) = best_px else {
            self.order_status = Some(("No order book data to chase".into(), true));
            return Task::none();
        };

        let asset = sym.asset_index;
        let sz_decimals = sym.sz_decimals;
        let is_spot = sym.market_type == MarketType::Spot;
        let reduce_only = if is_spot {
            false
        } else {
            self.order_reduce_only
        };
        let rounded_best = round_price(best, sz_decimals, is_spot);
        if !rounded_best.is_finite() || rounded_best <= 0.0 {
            self.order_status = Some(("Invalid chase price".into(), true));
            return Task::none();
        }
        let price_wire = float_to_wire(rounded_best);
        let chase_id = self.next_chase_id();

        self.active_chase = Some(ChaseOrder {
            id: chase_id,
            coin: self.active_symbol.clone(),
            account_address,
            agent_key: key.clone().into(),
            is_buy,
            remaining_size: qty,
            asset,
            sz_decimals,
            is_spot,
            reduce_only,
            current_oid: None,
            current_price: rounded_best,
            current_price_wire: price_wire,
            initial_price: rounded_best,
            started_at: std::time::Instant::now(),
            reprice_count: 0,
            pending_op: None,
            last_reprice_at: None,
            stop_requested: false,
            stop_reason: None,
            cancel_retries: 0,
            oid_confirmed: false,
            missing_open_order_refresh_requested: false,
        });

        let side_str = if is_buy { "BUY" } else { "SELL" };
        self.order_status = Some((format!("Chase {side_str} {qty} @ best ({best})..."), false));
        self.pending_order_action = Some(if is_buy {
            PendingOrderAction::ChaseBuy
        } else {
            PendingOrderAction::ChaseSell
        });

        self.chase_place_at_best()
    }
}
