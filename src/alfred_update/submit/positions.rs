use crate::alfred_state::{AlfredCommandId, alfred_query_is_nuke};
use crate::app_state::TradingTerminal;
use crate::helpers::{finite_value, positive_finite_value};
use crate::message::Message;
use crate::order_execution::{
    MarketUsdSizeReference, OrderSurface, PlaceIntent, PriceSource, QuantitySource,
    ReduceOnlySource, reject_if_positions_incomplete_for_action,
};
use crate::order_update::nuke_confirmation_is_armed;
use crate::signing::ExchangeOrderKind;
use iced::Task;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Alfred Position Submission
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn submit_alfred_nuke(&mut self) -> Task<Message> {
        let Some(command) = self.alfred_command_by_id(AlfredCommandId::NukePositions) else {
            self.push_toast(
                "Type 'nuke' or 'close all' to close open positions".to_string(),
                true,
            );
            return Task::none();
        };

        if !alfred_query_is_nuke(&self.alfred.query) || !command.enabled {
            self.push_toast(
                command
                    .disabled_reason
                    .unwrap_or_else(|| "NUKE is not available".to_string()),
                true,
            );
            return Task::none();
        }

        // Route through the same two-press arming flow as the NUKE button so
        // a single Enter in the palette can never flatten every position.
        let was_armed = nuke_confirmation_is_armed(self.nuke_confirmation.as_ref(), Instant::now());
        let task = self.handle_nuke_positions();
        if was_armed && self.pending_nuke_execution.is_some() {
            // Second press: the nuke is executing; the palette's job is done.
            self.alfred.close();
        } else {
            // First press armed (or refused to arm); echo the plan where the
            // user is looking and keep the palette open for the confirm press.
            self.toast_order_status();
        }
        task
    }

    pub(super) fn submit_alfred_close_position(&mut self) -> Task<Message> {
        let Some(draft) = self.alfred_close_position_draft(&self.alfred.query) else {
            self.push_toast("Type 'close HYPE' to close a position".to_string(), true);
            return Task::none();
        };
        if !draft.can_submit() {
            self.push_toast(
                draft
                    .error
                    .unwrap_or_else(|| "Complete the close command before submitting".to_string()),
                true,
            );
            return Task::none();
        }

        let Some(coin) = draft.coin else {
            self.push_toast("Add a ticker to close".to_string(), true);
            return Task::none();
        };

        if let Some(task) = self.alfred_close_position_preflight_task(&coin, draft.fraction) {
            return task;
        }

        self.alfred.close();
        self.close_menu_coin = None;
        self.execute_close_position(&coin, draft.fraction, true)
    }

    pub(super) fn alfred_close_position_preflight_task(
        &mut self,
        coin: &str,
        fraction: f64,
    ) -> Option<Task<Message>> {
        if self.reject_if_pending_trading_request("closing positions") {
            self.toast_order_status();
            return Some(Task::none());
        }
        let Some(account_address) = self.checked_order_signing_account() else {
            self.toast_order_status();
            return Some(Task::none());
        };
        if self.account_loading {
            self.order_status = Some((
                "Account refresh in progress; wait for fresh account data before closing".into(),
                true,
            ));
            self.toast_order_status();
            return Some(Task::none());
        }
        if self.reject_if_account_reconciliation_required("closing", "account data") {
            self.toast_order_status();
            return Some(Task::none());
        }
        if let Some(task) = reject_if_positions_incomplete_for_action(self, "closing positions") {
            self.toast_order_status();
            return Some(task);
        }

        let raw_szi = {
            let Some(account_data) = self.account_data_for_order_account(&account_address) else {
                self.order_status = Some((
                    "No account data available; refresh before closing".into(),
                    true,
                ));
                self.toast_order_status();
                return Some(Task::none());
            };
            let now_ms = Self::now_ms();
            if !account_data.is_fresh_for_position_action(now_ms) {
                let age_label = account_data
                    .position_action_snapshot_age_ms(now_ms)
                    .map(|age| format!("{}s old", age.div_ceil(1000)))
                    .unwrap_or_else(|| "from the future".to_string());
                self.order_status = Some((
                    format!(
                        "Account data is stale ({age_label}); refresh before closing positions"
                    ),
                    true,
                ));
                self.toast_order_status();
                return Some(self.refresh_account_data());
            }

            let Some(position) = account_data
                .clearinghouse
                .asset_positions
                .iter()
                .find(|ap| ap.position.coin == coin)
                .map(|ap| &ap.position)
            else {
                self.order_status = Some((format!("No position found for {coin}"), true));
                self.toast_order_status();
                return Some(Task::none());
            };
            position.szi.as_str()
        };

        let Some(close_fraction) =
            positive_finite_value(fraction).filter(|fraction| *fraction <= 1.0)
        else {
            self.order_status = Some(("Close fraction is invalid".into(), true));
            self.toast_order_status();
            return Some(Task::none());
        };
        let Some(position_size) = raw_szi
            .trim()
            .parse::<f64>()
            .ok()
            .and_then(finite_value)
            .filter(|size| size.abs() > 1e-12)
        else {
            self.order_status = Some(("Position size is invalid".into(), true));
            self.toast_order_status();
            return Some(Task::none());
        };

        let intent = PlaceIntent {
            surface: OrderSurface::ClosePosition,
            symbol_key: coin.to_string(),
            is_buy: position_size < 0.0,
            order_kind: ExchangeOrderKind::Market,
            price_source: PriceSource::MarketWithSlippage {
                invalid_message: None,
                usd_size_reference: MarketUsdSizeReference::ExecutionPrice,
            },
            quantity_source: QuantitySource::CoinSize {
                size: position_size.abs() * close_fraction,
                invalid_message: "Position size is invalid",
                precision_invalid_message: "Position size is invalid",
            },
            reduce_only_source: ReduceOnlySource::Fixed(true),
        };
        if let Err(message) = self.prepare_place_order(intent) {
            self.order_status = Some((message, true));
            self.toast_order_status();
            return Some(Task::none());
        }

        None
    }
}
