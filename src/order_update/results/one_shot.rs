use super::{
    ExecutionOutcome, ExecutionOutcomeKind, PendingOneShotStatusRequest, PlacementLookup,
    classify_execution_result,
};
use crate::api::{OrderStatusResult, fetch_order_status_by_cloid};
use crate::app_state::TradingTerminal;
use crate::helpers::redact_sensitive_response_text;
use crate::message::Message;
use crate::order_execution::OneShotPlacementContext;
use crate::signing::ExchangeResponse;
use iced::Task;

impl TradingTerminal {
    pub(super) fn set_unexpected_one_shot_resting_status(
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
        if context.surface.is_standard_ticket() {
            self.ticket_receipt_during_refresh = self.order_status.clone();
        }
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

    pub(crate) fn handle_close_position_result(
        &mut self,
        pending_indicator_id: Option<u64>,
        context: OneShotPlacementContext,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        self.handle_order_result(pending_indicator_id, context, result)
    }

    pub(crate) fn apply_execution_outcome(&mut self, outcome: ExecutionOutcome) -> Task<Message> {
        self.set_order_status(outcome.status, outcome.is_error);
        if outcome.refresh_account {
            self.refresh_account_data()
        } else {
            Task::none()
        }
    }

    pub(super) fn one_shot_context_matches_current_account(
        &self,
        context: &OneShotPlacementContext,
    ) -> bool {
        self.connected_order_account_matches(&context.account_address)
    }

    pub(super) fn begin_one_shot_status_request(
        &mut self,
        context: &OneShotPlacementContext,
    ) -> u64 {
        let request_id = self.next_one_shot_status_request_id;
        self.next_one_shot_status_request_id = self.next_one_shot_status_request_id.wrapping_add(1);
        self.insert_pending_one_shot_status_request(PendingOneShotStatusRequest::new(
            request_id, context,
        ));
        request_id
    }

    pub(crate) fn apply_one_shot_placement_outcome(
        &mut self,
        context: OneShotPlacementContext,
        mut outcome: ExecutionOutcome,
    ) -> Task<Message> {
        if !self.one_shot_context_matches_current_account(&context) {
            return Task::none();
        }
        self.pending_one_shot_status_requests
            .retain(|_, pending| !pending.is_for_context(&context));

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
            let request_id = self.begin_one_shot_status_request(&context);
            let status_task = placement_status_task(request_id, context);
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

        if context.surface.is_standard_ticket()
            && matches!(
                outcome.kind,
                ExecutionOutcomeKind::Filled | ExecutionOutcomeKind::AcceptedResting
            )
        {
            let display = self.display_name_for_symbol(&context.symbol_key);
            outcome.status = format!("{display} · {}", outcome.status);
        }
        if context.surface.is_standard_ticket() && outcome.refresh_account {
            self.ticket_receipt_during_refresh = Some((outcome.status.clone(), outcome.is_error));
        }
        self.apply_execution_outcome(outcome)
    }

    pub(crate) fn recheck_ticket_order_status(&mut self, id: u64) -> Task<Message> {
        let Some(check) = self.pending_one_shot_status_requests.get(&id) else {
            return Task::none();
        };
        if !check.needs_ticket_confirmation()
            || check.lookup != PlacementLookup::Unconfirmed
            || !self.one_shot_context_matches_current_account(&check.context)
        {
            return Task::none();
        }
        let Some(mut check) = self.pending_one_shot_status_requests.remove(&id) else {
            return Task::none();
        };
        check.lookup = PlacementLookup::Checking;
        check.request_id = self.next_one_shot_status_request_id;
        self.next_one_shot_status_request_id = self.next_one_shot_status_request_id.wrapping_add(1);
        let task = placement_status_task(check.request_id, check.context.clone());
        self.insert_pending_one_shot_status_request(check);
        task
    }

    pub(crate) fn handle_one_shot_placement_status_result(
        &mut self,
        request_id: u64,
        context: OneShotPlacementContext,
        result: Result<OrderStatusResult, String>,
    ) -> Task<Message> {
        let display = self.display_name_for_symbol(&context.symbol_key);
        let current_account = self.one_shot_context_matches_current_account(&context);
        let Some(check) = self.pending_one_shot_status_requests.get_mut(&request_id) else {
            return Task::none();
        };
        if !check.matches(request_id, &context) || check.lookup != PlacementLookup::Checking {
            return Task::none();
        }
        check.lookup = PlacementLookup::Unconfirmed;
        if !current_account {
            if !check.needs_ticket_confirmation() {
                self.pending_one_shot_status_requests.remove(&request_id);
            }
            return Task::none();
        }

        let ticket = context.surface.is_standard_ticket();
        let result = result.and_then(|status| {
            if ticket && !status.is_missing() && status.cloid.as_deref() != Some(&context.cloid) {
                Err("Exchange did not confirm the requested order ID".to_owned())
            } else {
                Ok(status)
            }
        });
        match result {
            Ok(status) if status.is_open() && context.order_kind.allows_resting_response() => {
                self.pending_one_shot_status_requests.remove(&request_id);
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
                self.pending_one_shot_status_requests.remove(&request_id);
                return self
                    .handle_unexpected_one_shot_resting_order(&context, &status.raw_summary);
            }
            Ok(status) if status.is_filled() => {
                self.pending_one_shot_status_requests.remove(&request_id);
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
                self.pending_one_shot_status_requests.remove(&request_id);
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
            Ok(status) if ticket && status.is_no_fill_terminal() => {
                check.lookup = PlacementLookup::Canceled;
                self.set_order_status(
                    format!(
                        "{display} order canceled. Check trade history for partial fills. {}",
                        status.raw_summary
                    ),
                    true,
                );
            }
            Ok(status) if status.is_no_fill_terminal() => {
                self.set_order_status(
                    format!(
                        "{} placement status still uncertain for {} ({}): {}; refreshing account data",
                        context.placement_label(),
                        display,
                        context.cloid,
                        status.raw_summary
                    ),
                    true,
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
                let error = redact_sensitive_response_text(&error);
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

        if ticket {
            self.ticket_receipt_during_refresh = self.order_status.clone();
        }
        self.refresh_account_data()
    }
}

fn placement_status_task(request_id: u64, context: OneShotPlacementContext) -> Task<Message> {
    Task::perform(
        fetch_order_status_by_cloid(context.account_address.clone(), context.cloid.clone()),
        move |result| Message::OneShotPlacementStatusLoaded {
            request_id,
            context,
            result: Box::new(result),
        },
    )
}
