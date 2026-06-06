use crate::api::{OrderStatusResult, fetch_order_status_by_cloid};
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

pub(crate) fn classify_execution_result(
    result: Result<ExchangeResponse, String>,
) -> ExecutionOutcome {
    match result {
        Ok(response) => {
            let status = response.summary();
            let is_error = response.is_error();
            let kind = if is_error {
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
            ExecutionOutcome {
                kind,
                status,
                is_error,
                refresh_account: !is_error,
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
        pending_indicator_id: Option<u64>,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        self.clear_pending_order_indicator(pending_indicator_id);
        let outcome = classify_execution_result(result);
        self.apply_execution_outcome(outcome)
    }

    pub(crate) fn handle_close_position_result(
        &mut self,
        context: OneShotPlacementContext,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
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
            self.set_order_status(
                format!(
                    "NUKE placement status unknown for {}: {}; checking {}",
                    context.symbol_key, outcome.status, context.cloid
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
        self.connected_address.as_deref() == Some(context.account_address.as_str())
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

        if matches!(
            outcome.kind,
            ExecutionOutcomeKind::Ambiguous | ExecutionOutcomeKind::TransportUnknown
        ) {
            self.set_order_status(
                format!(
                    "{} placement status unknown for {}: {}; checking {}",
                    context.placement_label(),
                    context.symbol_key,
                    outcome.status,
                    context.cloid
                ),
                true,
            );
            let request_context = context.clone();
            let status_task = Task::perform(
                fetch_order_status_by_cloid(context.account_address.clone(), context.cloid.clone()),
                move |result| Message::OneShotPlacementStatusLoaded {
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

        self.apply_execution_outcome(outcome)
    }

    pub(crate) fn handle_one_shot_placement_status_result(
        &mut self,
        context: OneShotPlacementContext,
        result: Result<OrderStatusResult, String>,
    ) -> Task<Message> {
        if !self.one_shot_context_matches_current_account(&context) {
            return Task::none();
        }

        match result {
            Ok(status) if status.is_open() => {
                self.set_order_status(
                    format!(
                        "{} placement confirmed by orderStatus for {}: {}",
                        context.placement_label(),
                        context.symbol_key,
                        status.raw_summary
                    ),
                    false,
                );
            }
            Ok(status) if status.is_filled() => {
                self.set_order_status(
                    format!(
                        "{} placement filled according to orderStatus for {}: {}",
                        context.placement_label(),
                        context.symbol_key,
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
                        context.symbol_key,
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
                        context.symbol_key,
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
                        context.symbol_key,
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
                        context.symbol_key,
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
                        context.symbol_key,
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
