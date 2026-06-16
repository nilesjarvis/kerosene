use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::{
    OrderSurface, PendingNukeExecution, PreparedExchangeOrder, place_order_task,
};
use crate::signing::ExchangeOrderKind;

use super::reject_if_positions_incomplete_for_action;

use iced::Task;

mod planning;

#[cfg(test)]
pub(crate) use planning::NukeSkipReason;
pub(crate) use planning::{NukePlan, NukePositionOrder};
use planning::{NukePositionInput, NukeSymbolInfo, plan_nuke_positions_from_inputs};

#[cfg(test)]
mod tests;

fn nuke_prepared_order(coin: String, order: NukePositionOrder) -> PreparedExchangeOrder {
    PreparedExchangeOrder {
        surface: OrderSurface::Nuke,
        symbol_key: coin,
        asset: order.asset,
        is_buy: order.is_buy,
        price: order.price,
        size: order.size,
        order_kind: ExchangeOrderKind::Market,
        reduce_only: true,
        market_type: MarketType::Perp,
    }
}

impl TradingTerminal {
    /// Plan a NUKE: classify every active position in the account snapshot into
    /// `(coin, order)` for submission or `(coin, reason)` for skip.
    /// Returns `Err` only when a position's `szi` field cannot be parsed
    /// (malformed account data), in which case the whole action is aborted
    /// rather than partially submitted.
    pub(crate) fn plan_nuke_positions(&self) -> Result<NukePlan, String> {
        let positions = self
            .connected_order_account_snapshot()
            .map(|(_, data)| data)
            .map(|d| d.clearinghouse.asset_positions.clone())
            .unwrap_or_default();

        let slippage = self.market_slippage_fraction();
        let inputs = positions.into_iter().map(|ap| {
            let coin = ap.position.coin;
            let is_hidden = self.symbol_key_is_hidden(&coin) || self.position_is_hidden(&coin);
            let sym = self
                .exchange_symbols
                .iter()
                .find(|s| s.key == coin)
                .map(|s| NukeSymbolInfo {
                    asset_index: s.asset_index,
                    sz_decimals: s.sz_decimals,
                    market_type: s.market_type,
                });
            let mid = self.resolve_mid_for_symbol(&coin);

            NukePositionInput {
                coin,
                raw_size: ap.position.szi,
                is_hidden,
                sym,
                mid,
            }
        });

        plan_nuke_positions_from_inputs(inputs, slippage)
    }

    pub(crate) fn execute_nuke_positions(&mut self) -> Task<Message> {
        let _theme = self.theme();
        if self.pending_nuke_execution.is_some() {
            self.order_status = Some(("NUKE already in progress".into(), true));
            return Task::none();
        }
        if self.reject_if_pending_trading_request("NUKE") {
            return Task::none();
        }

        let Some((key, account_address)) = self.order_signing_context() else {
            return Task::none();
        };

        if self.account_loading {
            self.order_status = Some((
                "Account refresh in progress; wait for fresh account data before NUKE".into(),
                true,
            ));
            return Task::none();
        }
        if self.reject_if_account_reconciliation_required("NUKE", "account data") {
            return Task::none();
        }
        if let Some(task) = reject_if_positions_incomplete_for_action(self, "NUKE") {
            return task;
        }
        let Some(account_data) = self.account_data_for_order_account(&account_address) else {
            self.order_status = Some((
                "No account data available; refresh before NUKE".into(),
                true,
            ));
            return Task::none();
        };
        let now_ms = Self::now_ms();
        if !account_data.is_fresh_for_position_action(now_ms) {
            let age_label = account_data
                .position_action_snapshot_age_ms(now_ms)
                .map(|age| format!("{}s old", age.div_ceil(1000)))
                .unwrap_or_else(|| "from the future".to_string());
            self.order_status = Some((
                format!("Account data is stale ({age_label}); refresh before NUKE"),
                true,
            ));
            return self.refresh_account_data();
        }

        let plan = match self.plan_nuke_positions() {
            Ok(plan) => plan,
            Err(e) => {
                self.order_status = Some((e, true));
                return Task::none();
            }
        };

        if plan.is_empty() {
            self.order_status = Some(("No positions to close".into(), true));
            return Task::none();
        }
        if !plan.hidden_skipped.is_empty() {
            self.order_status = Some((
                format!(
                    "NUKE aborted: hidden exposure could not be routed. Hidden skipped: {}",
                    plan.format_hidden_skip_list()
                ),
                true,
            ));
            return Task::none();
        }
        if plan.ready.is_empty() {
            // Every active position is unrouteable. Refuse to fire — surface
            // why so the user can address it (subscribe to mids, switch
            // symbol search filters, etc.) rather than seeing a silent no-op.
            self.order_status = Some((
                format!(
                    "NUKE aborted: no positions could be routed. Skipped: {}",
                    plan.format_skip_list()
                ),
                true,
            ));
            return Task::none();
        }
        self.nuke_confirmation = None;

        let ready_count = plan.ready.len();
        let skipped_count = plan.skipped.len();
        // Format the skip list before consuming `ready` in the loop below.
        let skip_summary = plan.format_skip_list();
        let NukePlan { ready, .. } = plan;

        let execution_id = self.next_nuke_execution_id;
        self.next_nuke_execution_id = self.next_nuke_execution_id.saturating_add(1);
        self.pending_nuke_execution = Some(PendingNukeExecution::new(
            execution_id,
            ready_count,
            skipped_count,
        ));
        let mut tasks = Vec::with_capacity(ready_count);
        for (coin, order) in ready {
            let k = key.clone();
            let prepared = nuke_prepared_order(coin, order);
            let (request, context) = prepared.place_request_with_context(&account_address);
            tasks.push(place_order_task(k, request, move |r| Message::NukeResult {
                execution_id,
                context,
                result: Box::new(r),
            }));
        }

        let total = ready_count + skipped_count;
        let status = if skipped_count == 0 {
            format!(
                "Nuking {} position{}...",
                ready_count,
                if ready_count == 1 { "" } else { "s" }
            )
        } else {
            format!(
                "Nuking {} of {} position{}; skipped: {}",
                ready_count,
                total,
                if total == 1 { "" } else { "s" },
                skip_summary
            )
        };
        self.order_status = Some((status, false));
        Task::batch(tasks)
    }
}
