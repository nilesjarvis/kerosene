use crate::api::{MarketType, fetch_order_book};
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::order_execution::PendingOrderAction;
use crate::signing::ChaseOrder;
use crate::twap_state::MAX_ACTIVE_ADVANCED_ORDERS;
use iced::Task;

mod lifecycle;

// ---------------------------------------------------------------------------
// Chase Order Helpers
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn selected_chase_id(&self) -> Option<u64> {
        self.selected_chase_id
            .filter(|id| self.chase_orders.contains_key(id))
            .or_else(|| self.chase_orders.keys().next_back().copied())
    }

    pub(crate) fn selected_chase(&self) -> Option<&ChaseOrder> {
        let id = self.selected_chase_id()?;
        self.chase_orders.get(&id)
    }

    pub(crate) fn remove_chase_order(&mut self, chase_id: u64) {
        self.remove_chase_order_with_summary(chase_id, None);
    }

    pub(crate) fn remove_chase_order_with_summary(
        &mut self,
        chase_id: u64,
        summary: Option<String>,
    ) {
        if let Some(chase) = self.chase_orders.remove(&chase_id) {
            let summary = summary.unwrap_or_else(|| {
                chase
                    .stop_reason
                    .as_ref()
                    .map(|(reason, _)| reason.clone())
                    .unwrap_or_else(|| "Chase completed or no longer open".to_string())
            });
            self.archive_chase_order(&chase, summary);
        }
        if self.selected_chase_id == Some(chase_id) {
            self.selected_chase_id = self.chase_orders.keys().next_back().copied();
        }
    }

    pub(crate) fn chase_book_fetch_sigfigs(&self, symbol: &str) -> (Option<u8>, Option<u8>) {
        let mid = self.resolve_mid_for_symbol(symbol);
        let tick = mid.map(helpers::default_tick_for_price).unwrap_or(0.01);
        mid.map(|mid| helpers::compute_sigfigs(tick, mid))
            .unwrap_or((None, None))
    }

    pub(crate) fn start_chase(&mut self, is_buy: bool) -> Task<Message> {
        let _theme = self.theme();
        if self.active_advanced_order_count() >= MAX_ACTIVE_ADVANCED_ORDERS {
            self.order_status = Some((
                format!(
                    "Cannot start chase: maximum of {MAX_ACTIVE_ADVANCED_ORDERS} active advanced orders reached"
                ),
                true,
            ));
            return Task::none();
        }
        if self.pending_order_action.is_some() {
            self.order_status = Some(("Wait for the pending order action to finish".into(), true));
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

        let asset = sym.asset_index;
        let sz_decimals = sym.sz_decimals;
        let is_spot = sym.market_type == MarketType::Spot;
        let started_at = std::time::Instant::now();
        let started_at_ms = Self::now_ms();
        let reduce_only = if is_spot {
            false
        } else {
            self.order_reduce_only
        };
        let chase_id = self.next_chase_id();

        self.chase_orders.insert(
            chase_id,
            ChaseOrder {
                id: chase_id,
                coin: self.active_symbol.clone(),
                account_address,
                agent_key: key.clone().into(),
                is_buy,
                target_size: qty,
                remaining_size: qty,
                asset,
                sz_decimals,
                is_spot,
                reduce_only,
                current_oid: None,
                current_price: 0.0,
                current_price_wire: String::new(),
                initial_price: 0.0,
                started_at,
                started_at_ms,
                reprice_count: 0,
                pending_op: None,
                last_reprice_at: None,
                pending_best_price: None,
                stop_requested: false,
                stop_reason: None,
                cancel_retries: 0,
                oid_confirmed: false,
                missing_open_order_refresh_requested: false,
            },
        );
        self.selected_chase_id = Some(chase_id);

        let side_str = if is_buy { "BUY" } else { "SELL" };
        self.order_status = Some((
            format!(
                "Chase {side_str} {qty} {}: loading book...",
                self.active_symbol_display
            ),
            false,
        ));
        self.pending_order_action = Some(if is_buy {
            PendingOrderAction::ChaseBuy
        } else {
            PendingOrderAction::ChaseSell
        });

        let symbol = self.active_symbol.clone();
        let sigfigs = self.chase_book_fetch_sigfigs(&symbol);
        Task::perform(fetch_order_book(symbol, sigfigs), move |result| {
            Message::ChaseInitialBookLoaded {
                chase_id,
                result: Box::new(result),
            }
        })
    }
}
