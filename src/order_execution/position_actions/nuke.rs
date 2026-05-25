use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{OrderKind, place_order};

use iced::Task;

mod planning;

pub(crate) use planning::NukePlan;
use planning::{NukePositionInput, NukeSymbolInfo, plan_nuke_positions_from_inputs};
#[cfg(test)]
pub(crate) use planning::{NukePositionOrder, NukeSkipReason};

#[cfg(test)]
mod tests;

impl TradingTerminal {
    /// Plan a NUKE: classify every active visible non-muted position into
    /// `(coin, order)` for submission or `(coin, reason)` for skip.
    /// Returns `Err` only when a position's `szi` field cannot be parsed
    /// (malformed account data), in which case the whole action is aborted
    /// rather than partially submitted.
    pub(crate) fn plan_nuke_positions(&self) -> Result<NukePlan, String> {
        let positions = self
            .account_data
            .as_ref()
            .map(|d| d.clearinghouse.asset_positions.clone())
            .unwrap_or_default();

        let slippage = self.market_slippage_fraction();
        let inputs = positions.into_iter().map(|ap| {
            let coin = ap.position.coin;
            let is_visible = !self.symbol_key_is_hidden(&coin) && !self.position_is_hidden(&coin);
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
                is_visible,
                sym,
                mid,
            }
        });

        plan_nuke_positions_from_inputs(inputs, slippage)
    }

    pub(crate) fn execute_nuke_positions(&mut self) -> Task<Message> {
        let _theme = self.theme();
        let key = self.wallet_key_input.trim().to_string();
        if key.is_empty() || self.connected_address.is_none() {
            self.order_status = Some(("Connect wallet and enter agent key first".into(), true));
            return Task::none();
        }

        if self.account_loading {
            self.order_status = Some((
                "Account refresh in progress; wait for fresh account data before NUKE".into(),
                true,
            ));
            return Task::none();
        }
        let Some(account_data) = self.account_data.as_ref() else {
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

        let mut tasks = Vec::with_capacity(ready_count);
        for (_coin, order) in ready {
            let k = key.clone();
            tasks.push(Task::perform(
                place_order(
                    k.into(),
                    order.asset,
                    order.is_buy,
                    order.price,
                    order.size,
                    OrderKind::Market,
                    true,
                ),
                |r| Message::NukeResult(Box::new(r)),
            ));
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
