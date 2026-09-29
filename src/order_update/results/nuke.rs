use super::{ExecutionOutcomeKind, classify_execution_result};
use crate::api::{OrderStatusResult, fetch_order_status_by_cloid};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::OneShotPlacementContext;
use crate::signing::ExchangeResponse;
use iced::Task;

impl TradingTerminal {
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
            return Task::perform(
                fetch_order_status_by_cloid(context.account_address.clone(), context.cloid.clone()),
                move |result| Message::NukePlacementStatusLoaded {
                    execution_id,
                    context,
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

    fn clear_nuke_execution_if_current(&mut self, execution_id: u64) {
        if self
            .pending_nuke_execution
            .as_ref()
            .is_some_and(|execution| execution.id == execution_id)
        {
            self.pending_nuke_execution = None;
        }
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
