use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::ExchangeResponse;
use iced::Task;

use super::results::{ExecutionOutcomeKind, classify_execution_result};

impl TradingTerminal {
    pub(super) fn handle_move_order_modify_result(
        &mut self,
        oid: u64,
        pending_indicator_id: Option<u64>,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        self.pending_move_order_contexts.remove(&oid);
        self.clear_pending_order_indicator(pending_indicator_id);
        self.sync_all_chart_orders();

        let mut outcome = classify_execution_result(result);
        match outcome.kind {
            ExecutionOutcomeKind::Rejected => {
                outcome.status = format!("Move failed: {}", outcome.status);
            }
            ExecutionOutcomeKind::TransportUnknown => {
                outcome.status = format!("Move modify failed: {}", outcome.status);
            }
            ExecutionOutcomeKind::AcceptedResting
            | ExecutionOutcomeKind::Filled
            | ExecutionOutcomeKind::Cancelled
            | ExecutionOutcomeKind::Ambiguous => {}
        }
        self.apply_execution_outcome(outcome)
    }
}
