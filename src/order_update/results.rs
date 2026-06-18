use crate::api::{OrderStatusResult, fetch_order_status_by_cloid, fetch_order_status_by_oid};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::OneShotPlacementContext;
use crate::signing::ExchangeResponse;
use iced::Task;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionOutcomeKind {
    AcceptedResting,
    Filled,
    Cancelled,
    Rejected,
    Ambiguous,
    TransportUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutionOutcome {
    pub(crate) kind: ExecutionOutcomeKind,
    pub(crate) status: String,
    pub(crate) is_error: bool,
    pub(crate) refresh_account: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingOneShotStatusRequest {
    pub(crate) request_id: u64,
    account_address: String,
    cloid: String,
}

impl PendingOneShotStatusRequest {
    pub(crate) fn new(request_id: u64, context: &OneShotPlacementContext) -> Self {
        Self {
            request_id,
            account_address: context.account_address.clone(),
            cloid: context.cloid.clone(),
        }
    }

    fn matches(&self, request_id: u64, context: &OneShotPlacementContext) -> bool {
        self.request_id == request_id
            && self.account_address == context.account_address
            && self.cloid == context.cloid
    }
}

pub(crate) fn classify_execution_result(
    result: Result<ExchangeResponse, String>,
) -> ExecutionOutcome {
    match result {
        Ok(response) => {
            let status = response.summary();
            let response_is_error = response.is_error();
            let kind = if response_is_error {
                ExecutionOutcomeKind::Rejected
            } else if status == "Cancelled" {
                ExecutionOutcomeKind::Cancelled
            } else if response.is_ambiguous_order_result() {
                ExecutionOutcomeKind::Ambiguous
            } else if response.is_fully_filled() {
                ExecutionOutcomeKind::Filled
            } else {
                ExecutionOutcomeKind::AcceptedResting
            };
            let is_error = response_is_error || kind == ExecutionOutcomeKind::Ambiguous;
            ExecutionOutcome {
                kind,
                status,
                is_error,
                refresh_account: !response_is_error,
            }
        }
        Err(error) => ExecutionOutcome {
            kind: ExecutionOutcomeKind::TransportUnknown,
            status: error,
            is_error: true,
            refresh_account: true,
        },
    }
}

impl TradingTerminal {
    fn remove_local_open_order(&mut self, account_address: &str, oid: u64, symbol: &str) {
        let Some(data) = self.account_data_for_order_account_mut(account_address) else {
            return;
        };
        let before = data.open_orders.len();
        data.open_orders
            .retain(|order| order.oid != oid || order.coin != symbol);
        if data.open_orders.len() != before {
            self.sync_all_chart_orders();
        }
    }

    fn cancel_order_status_task(
        account_address: String,
        oid: u64,
        symbol: String,
    ) -> Task<Message> {
        Task::perform(
            fetch_order_status_by_oid(account_address.clone(), oid),
            move |result| Message::CancelOrderStatusLoaded {
                account_address,
                oid,
                symbol,
                result: Box::new(result),
            },
        )
    }

    fn set_unexpected_one_shot_resting_status(
        &mut self,
        context: &OneShotPlacementContext,
        summary: &str,
    ) {
        let display = self.display_name_for_symbol(&context.symbol_key);
        self.set_order_status(
            format!(
                "{} {} order unexpectedly rested for {}: {}; refreshing account data, cancel {} if it is still open",
                context.placement_label(),
                context.order_kind.label(),
                display,
                summary,
                context.cloid
            ),
            true,
        );
    }

    fn handle_unexpected_one_shot_resting_order(
        &mut self,
        context: &OneShotPlacementContext,
        summary: &str,
    ) -> Task<Message> {
        self.set_unexpected_one_shot_resting_status(context, summary);
        self.refresh_account_data()
    }

    pub(crate) fn handle_order_result(
        &mut self,
        pending_indicator_id: Option<u64>,
        context: OneShotPlacementContext,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        self.pending_order_action = None;
        self.clear_pending_order_indicator(pending_indicator_id);
        let outcome = classify_execution_result(result);
        self.apply_one_shot_placement_outcome(context, outcome)
    }

    pub(crate) fn handle_cancel_result(
        &mut self,
        account_address: String,
        pending_indicator_id: Option<u64>,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        let cancelled_order = self.pending_cancel_indicator_order(pending_indicator_id);
        self.clear_pending_order_indicator(pending_indicator_id);
        if !self.connected_order_account_matches(&account_address) {
            return Task::none();
        }
        let cancelled_oid = cancelled_order.as_ref().map(|(oid, _)| *oid);
        let outcome = classify_execution_result(result);
        if matches!(
            outcome.kind,
            ExecutionOutcomeKind::Ambiguous | ExecutionOutcomeKind::TransportUnknown
        ) {
            let status_task = cancelled_order
                .clone()
                .map_or_else(Task::none, |(oid, symbol)| {
                    Self::cancel_order_status_task(account_address.clone(), oid, symbol)
                });
            let order_label = cancelled_oid
                .map(|oid| format!(" for order {oid}"))
                .unwrap_or_default();
            self.set_order_status(
                format!(
                    "Cancel status unknown{order_label}: {}; checking orderStatus and refreshing account data",
                    outcome.status
                ),
                true,
            );
            return Task::batch([self.refresh_account_data(), status_task]);
        }
        // Drop the order from the local snapshot on a confirmed cancel so the
        // ack does not resurrect an interactive line for an order the exchange
        // has already removed; the next authoritative update wins regardless.
        if outcome.kind == ExecutionOutcomeKind::Cancelled
            && let Some((oid, symbol)) = cancelled_order
        {
            self.remove_local_open_order(&account_address, oid, &symbol);
        }
        self.apply_execution_outcome(outcome)
    }

    pub(crate) fn handle_cancel_order_status_result(
        &mut self,
        account_address: String,
        oid: u64,
        symbol: String,
        result: Result<OrderStatusResult, String>,
    ) -> Task<Message> {
        if !self.connected_order_account_matches(&account_address) {
            return Task::none();
        }

        match result {
            Ok(status) if status.is_open() => {
                self.set_order_status(
                    format!(
                        "Cancel status still uncertain for order {oid}: orderStatus reports open ({}); refreshing account data",
                        status.raw_summary
                    ),
                    true,
                );
            }
            Ok(status) if status.is_filled() => {
                self.remove_local_open_order(&account_address, oid, &symbol);
                self.set_order_status(
                    format!(
                        "Cancel did not prevent fill for order {oid}: {}; refreshing account data",
                        status.raw_summary
                    ),
                    true,
                );
            }
            Ok(status) if status.is_no_fill_terminal() => {
                self.remove_local_open_order(&account_address, oid, &symbol);
                self.set_order_status(
                    format!(
                        "Cancel resolved for order {oid}: orderStatus reports {}; refreshing account data",
                        status.raw_summary
                    ),
                    false,
                );
            }
            Ok(status) if status.is_missing() => {
                self.set_order_status(
                    format!(
                        "Cancel status still uncertain for order {oid}: {}; refreshing account data",
                        status.raw_summary
                    ),
                    true,
                );
            }
            Ok(status) => {
                self.set_order_status(
                    format!(
                        "Cancel status still uncertain for order {oid}: orderStatus returned {}; refreshing account data",
                        status.raw_summary
                    ),
                    true,
                );
            }
            Err(error) => {
                self.set_order_status(
                    format!(
                        "Cancel status still uncertain for order {oid}: {error}; refreshing account data"
                    ),
                    true,
                );
            }
        }

        self.refresh_account_data()
    }

    pub(crate) fn handle_close_position_result(
        &mut self,
        pending_indicator_id: Option<u64>,
        context: OneShotPlacementContext,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        self.pending_order_action = None;
        self.clear_pending_order_indicator(pending_indicator_id);
        let outcome = classify_execution_result(result);
        self.apply_one_shot_placement_outcome(context, outcome)
    }

    pub(crate) fn handle_nuke_result(
        &mut self,
        execution_id: u64,
        context: OneShotPlacementContext,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        if !self.one_shot_context_matches_current_account(&context) {
            self.clear_nuke_execution_if_current(execution_id);
            return Task::none();
        }

        let outcome = classify_execution_result(result);
        if matches!(
            outcome.kind,
            ExecutionOutcomeKind::Ambiguous | ExecutionOutcomeKind::TransportUnknown
        ) {
            let display = self.display_name_for_symbol(&context.symbol_key);
            self.set_order_status(
                format!(
                    "NUKE placement status unknown for {}: {}; checking {}",
                    display, outcome.status, context.cloid
                ),
                true,
            );
            let request_context = context.clone();
            return Task::perform(
                fetch_order_status_by_cloid(context.account_address.clone(), context.cloid.clone()),
                move |result| Message::NukePlacementStatusLoaded {
                    execution_id,
                    context: request_context,
                    result: Box::new(result),
                },
            );
        }

        if outcome.kind == ExecutionOutcomeKind::AcceptedResting
            && !context.order_kind.allows_resting_response()
        {
            let nuke_task = self.record_nuke_child_uncertain(execution_id);
            self.set_unexpected_one_shot_resting_status(&context, &outcome.status);
            return Task::batch([nuke_task, self.refresh_account_data()]);
        }

        let confirmed = matches!(
            outcome.kind,
            ExecutionOutcomeKind::AcceptedResting | ExecutionOutcomeKind::Filled
        );
        self.record_nuke_child_outcome(execution_id, confirmed, outcome.refresh_account)
    }

    pub(crate) fn toggle_close_menu(&mut self, coin: String) {
        if self.close_menu_coin.as_deref() == Some(&coin) {
            self.close_menu_coin = None;
        } else {
            self.close_menu_coin = Some(coin);
        }
    }

    pub(crate) fn clear_transient_order_ui(&mut self) {
        for instance in self.charts.values_mut() {
            instance.clear_quick_order();
            instance.editor_open = false;
            instance.editor_search_query.clear();
            instance.editor_selected_index = None;
            instance.secondary_editor_open = false;
            instance.secondary_editor_search_query.clear();
            instance.secondary_editor_selected_index = None;
            instance.chart.active_tool = None;
        }
        self.chart_quick_order_surface.clear();
        self.chart_surface_active_tools.clear();
    }

    pub(crate) fn apply_execution_outcome(&mut self, outcome: ExecutionOutcome) -> Task<Message> {
        self.set_order_status(outcome.status, outcome.is_error);
        if outcome.refresh_account {
            self.refresh_account_data()
        } else {
            Task::none()
        }
    }

    fn one_shot_context_matches_current_account(&self, context: &OneShotPlacementContext) -> bool {
        self.connected_order_account_matches(&context.account_address)
    }

    fn begin_one_shot_status_request(&mut self, context: &OneShotPlacementContext) -> u64 {
        let request_id = self.next_one_shot_status_request_id;
        self.next_one_shot_status_request_id = self.next_one_shot_status_request_id.wrapping_add(1);
        self.pending_one_shot_status_request =
            Some(PendingOneShotStatusRequest::new(request_id, context));
        request_id
    }

    fn clear_nuke_execution_if_current(&mut self, execution_id: u64) {
        if self
            .pending_nuke_execution
            .as_ref()
            .is_some_and(|execution| execution.id == execution_id)
        {
            self.pending_nuke_execution = None;
        }
    }

    pub(crate) fn apply_one_shot_placement_outcome(
        &mut self,
        context: OneShotPlacementContext,
        outcome: ExecutionOutcome,
    ) -> Task<Message> {
        if !self.one_shot_context_matches_current_account(&context) {
            return Task::none();
        }
        self.pending_one_shot_status_request = None;

        if matches!(
            outcome.kind,
            ExecutionOutcomeKind::Ambiguous | ExecutionOutcomeKind::TransportUnknown
        ) {
            let display = self.display_name_for_symbol(&context.symbol_key);
            self.set_order_status(
                format!(
                    "{} placement status unknown for {}: {}; checking {}",
                    context.placement_label(),
                    display,
                    outcome.status,
                    context.cloid
                ),
                true,
            );
            let request_context = context.clone();
            let request_id = self.begin_one_shot_status_request(&context);
            let status_task = Task::perform(
                fetch_order_status_by_cloid(context.account_address.clone(), context.cloid.clone()),
                move |result| Message::OneShotPlacementStatusLoaded {
                    request_id,
                    context: request_context,
                    result: Box::new(result),
                },
            );
            return if outcome.refresh_account {
                Task::batch([self.refresh_account_data(), status_task])
            } else {
                status_task
            };
        }

        if outcome.kind == ExecutionOutcomeKind::AcceptedResting
            && !context.order_kind.allows_resting_response()
        {
            return self.handle_unexpected_one_shot_resting_order(&context, &outcome.status);
        }

        self.apply_execution_outcome(outcome)
    }

    pub(crate) fn handle_one_shot_placement_status_result(
        &mut self,
        request_id: u64,
        context: OneShotPlacementContext,
        result: Result<OrderStatusResult, String>,
    ) -> Task<Message> {
        let request_matches = self
            .pending_one_shot_status_request
            .as_ref()
            .is_some_and(|pending| pending.matches(request_id, &context));
        if !request_matches {
            return Task::none();
        }
        self.pending_one_shot_status_request = None;

        if !self.one_shot_context_matches_current_account(&context) {
            return Task::none();
        }

        let display = self.display_name_for_symbol(&context.symbol_key);
        match result {
            Ok(status) if status.is_open() && context.order_kind.allows_resting_response() => {
                self.set_order_status(
                    format!(
                        "{} placement confirmed by orderStatus for {}: {}",
                        context.placement_label(),
                        display,
                        status.raw_summary
                    ),
                    false,
                );
            }
            Ok(status) if status.is_open() => {
                return self
                    .handle_unexpected_one_shot_resting_order(&context, &status.raw_summary);
            }
            Ok(status) if status.is_filled() => {
                self.set_order_status(
                    format!(
                        "{} placement filled according to orderStatus for {}: {}",
                        context.placement_label(),
                        display,
                        status.raw_summary
                    ),
                    false,
                );
            }
            Ok(status) if status.is_definitive_no_fill_terminal() => {
                self.set_order_status(
                    format!(
                        "{} placement rejected according to orderStatus for {}: {}",
                        context.placement_label(),
                        display,
                        status.raw_summary
                    ),
                    true,
                );
            }
            Ok(status) if status.is_no_fill_terminal() => {
                self.set_order_status(
                    format!(
                        "{} placement resolved without fill for {}: {}",
                        context.placement_label(),
                        display,
                        status.raw_summary
                    ),
                    false,
                );
            }
            Ok(status) if status.is_missing() => {
                self.set_order_status(
                    format!(
                        "{} placement status still uncertain for {} ({}): {}",
                        context.placement_label(),
                        display,
                        context.cloid,
                        status.raw_summary
                    ),
                    true,
                );
            }
            Ok(status) => {
                self.set_order_status(
                    format!(
                        "{} placement status for {} ({}) was {}",
                        context.placement_label(),
                        display,
                        context.cloid,
                        status.raw_summary
                    ),
                    true,
                );
            }
            Err(error) => {
                self.set_order_status(
                    format!(
                        "{} placement status still uncertain for {} ({}): {}",
                        context.placement_label(),
                        display,
                        context.cloid,
                        error
                    ),
                    true,
                );
            }
        }

        self.refresh_account_data()
    }

    pub(crate) fn handle_nuke_placement_status_result(
        &mut self,
        execution_id: u64,
        context: OneShotPlacementContext,
        result: Result<OrderStatusResult, String>,
    ) -> Task<Message> {
        if !self.one_shot_context_matches_current_account(&context) {
            self.clear_nuke_execution_if_current(execution_id);
            return Task::none();
        }

        match result {
            Ok(status) if status.is_open() && !context.order_kind.allows_resting_response() => {
                let nuke_task = self.record_nuke_child_uncertain(execution_id);
                self.set_unexpected_one_shot_resting_status(&context, &status.raw_summary);
                Task::batch([nuke_task, self.refresh_account_data()])
            }
            Ok(status) if status.is_open() || status.is_filled() => {
                self.record_nuke_child_outcome(execution_id, true, true)
            }
            Ok(status) if status.is_definitive_no_fill_terminal() => {
                self.record_nuke_child_outcome(execution_id, false, false)
            }
            Ok(status) if status.is_no_fill_terminal() => {
                self.record_nuke_child_outcome(execution_id, false, true)
            }
            Ok(_) | Err(_) => self.record_nuke_child_uncertain(execution_id),
        }
    }

    fn record_nuke_child_outcome(
        &mut self,
        execution_id: u64,
        confirmed: bool,
        refresh_needed: bool,
    ) -> Task<Message> {
        let Some(execution) = self
            .pending_nuke_execution
            .as_mut()
            .filter(|execution| execution.id == execution_id)
        else {
            return Task::none();
        };

        if confirmed {
            execution.record_confirmed(refresh_needed);
        } else {
            execution.record_failed(refresh_needed);
        }
        self.finish_or_update_nuke_execution()
    }

    fn record_nuke_child_uncertain(&mut self, execution_id: u64) -> Task<Message> {
        let Some(execution) = self
            .pending_nuke_execution
            .as_mut()
            .filter(|execution| execution.id == execution_id)
        else {
            return Task::none();
        };

        execution.record_uncertain();
        self.finish_or_update_nuke_execution()
    }

    fn finish_or_update_nuke_execution(&mut self) -> Task<Message> {
        let Some(execution) = self.pending_nuke_execution.as_ref() else {
            return Task::none();
        };
        let status = execution.status_text();
        let is_error = execution.has_problem();
        let is_complete = execution.is_complete();
        let refresh_needed = execution.refresh_needed();
        self.set_order_status(status, is_error);

        if !is_complete {
            return Task::none();
        }
        self.pending_nuke_execution = None;
        if refresh_needed {
            self.refresh_account_data()
        } else {
            Task::none()
        }
    }
}

pub(in crate::order_update) fn result_requires_account_refresh(
    result: &Result<ExchangeResponse, String>,
) -> bool {
    match result {
        Ok(response) => !response.is_error(),
        // Signed exchange requests can fail locally after the exchange has
        // already accepted the action. Reconcile account state on transport,
        // response-body, or parse failures so basic order paths fail closed
        // instead of leaving open orders/positions stale.
        Err(_) => true,
    }
}
