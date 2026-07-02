use super::sizing::order_size_from_quantity_input;
use crate::api::{MarketType, fetch_order_book};
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::order_execution::{
    AdvancedOrderKind, AdvancedOrderStartSnapshot, OrderOperation, OrderSurface,
    PendingOrderAction, validate_surface_market_type,
};
use crate::signing::{ChaseLifecycle, ChaseOrder};
use iced::Task;

mod lifecycle;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Chase Order Helpers
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn chase_owns_startup_pending_action(
        &self,
        _chase_id: u64,
        chase: &ChaseOrder,
    ) -> bool {
        if !chase.known_oids.is_empty()
            || chase.current_oid.is_some()
            || chase.place_attempt_count > 1
        {
            return false;
        }

        matches!(
            (self.pending_order_action, chase.is_buy),
            (Some(PendingOrderAction::ChaseBuy), true)
                | (Some(PendingOrderAction::ChaseSell), false)
        )
    }

    pub(crate) fn chase_place_result_owns_startup_pending(&self, chase_id: u64) -> bool {
        self.chase_orders
            .get(&chase_id)
            .is_some_and(|chase| self.chase_owns_startup_pending_action(chase_id, chase))
    }

    pub(crate) fn clear_chase_startup_pending_if_owned(&mut self, chase_id: u64) {
        if self.chase_place_result_owns_startup_pending(chase_id) {
            self.pending_order_action = None;
        }
    }

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
        let clear_startup_pending = self.chase_orders.get(&chase_id).is_some_and(|chase| {
            self.chase_owns_startup_pending_action(chase_id, chase)
                && chase.current_cloid.is_none()
                && !chase.has_pending_op()
        });
        let removed = self.chase_orders.contains_key(&chase_id);
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
        if clear_startup_pending {
            self.pending_order_action = None;
        }
        if removed {
            self.sync_all_chart_orders();
        }
    }

    pub(crate) fn chase_book_fetch_sigfigs(&self, symbol: &str) -> (Option<u8>, Option<u8>) {
        let mid = self.resolve_mid_for_symbol(symbol);
        let tick = mid.map(helpers::default_tick_for_price).unwrap_or(0.01);
        mid.map(|mid| helpers::compute_sigfigs(tick, mid))
            .unwrap_or((None, None))
    }

    pub(crate) fn start_chase_from_snapshot(
        &mut self,
        is_buy: bool,
        snapshot: AdvancedOrderStartSnapshot,
    ) -> Task<Message> {
        if self.reject_stale_advanced_order_start_snapshot(AdvancedOrderKind::Chase, &snapshot) {
            return Task::none();
        }

        self.start_chase(is_buy)
    }

    pub(crate) fn start_chase(&mut self, is_buy: bool) -> Task<Message> {
        let _theme = self.theme();
        let Some(start_context) = self.advanced_order_start_context(AdvancedOrderKind::Chase)
        else {
            self.toast_order_status();
            return Task::none();
        };
        if let Some(task) = self.stale_percentage_order_quantity_task("starting a chase") {
            self.toast_order_status();
            return task;
        }

        let raw_qty: f64 = match helpers::parse_positive_number(&self.order_quantity) {
            Some(v) => v,
            _ => {
                self.set_order_status("Invalid quantity".into(), true);
                return Task::none();
            }
        };

        let sym = self
            .exchange_symbols
            .iter()
            .find(|s| s.key == self.active_symbol)
            .cloned();
        let Some(sym) = sym else {
            self.set_order_status(format!("Symbol '{}' not found", self.active_symbol), true);
            return Task::none();
        };
        if let Err(error) = self.validate_exchange_symbol_orderable(
            &sym,
            OrderSurface::Chase.orderability_context_label(),
        ) {
            self.set_order_status(error, true);
            return Task::none();
        }
        if let Err(error) = validate_surface_market_type(
            OrderSurface::Chase,
            OrderOperation::Place,
            sym.market_type,
        ) {
            if sym.market_type == MarketType::Outcome
                && let Err(e) = self.validate_outcome_contract_size(raw_qty)
            {
                self.set_order_status(e, true);
            } else {
                self.set_order_status(error.status_text(), true);
            }
            return Task::none();
        }

        let reference_price = if self.order_quantity_is_usd {
            let Some(price) = self.resolve_mid_for_symbol(&self.active_symbol) else {
                self.set_order_status(
                    format!(
                        concat!(
                            "Cannot start USD Chase: no fresh mid price for {}. ",
                            "Wait for market data or enter size in coin units."
                        ),
                        self.active_symbol
                    ),
                    true,
                );
                return Task::none();
            };
            price
        } else {
            1.0
        };

        let Some(qty) = order_size_from_quantity_input(
            raw_qty,
            reference_price,
            self.order_quantity_is_usd,
            sym.sz_decimals,
        ) else {
            let message = "Invalid quantity for asset precision".to_string();
            self.set_order_status(message, true);
            return Task::none();
        };

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
                account_address: start_context.account_address,
                agent_key: start_context.agent_key,
                is_buy,
                target_size: qty,
                filled_size: 0.0,
                remaining_size: qty,
                known_oids: Vec::new(),
                current_cloid: None,
                place_attempt_count: 0,
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
                fill_cutoff_ms_by_oid: Vec::new(),
                reprice_count: 0,
                lifecycle: ChaseLifecycle::LoadingBook,
                last_reprice_at: None,
                desired_price: None,
                stop_reason: None,
                cancel_retries: 0,
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
