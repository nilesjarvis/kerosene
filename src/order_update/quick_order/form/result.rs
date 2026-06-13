use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::{OneShotPlacementContext, QuickOrderRecovery};
use crate::order_update::results::{ExecutionOutcomeKind, classify_execution_result};
use crate::signing::ExchangeResponse;
use iced::Task;

// ---------------------------------------------------------------------------
// Quick Order Result Handling
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn handle_quick_order_result(
        &mut self,
        pending_indicator_id: Option<u64>,
        context: OneShotPlacementContext,
        recovery: Option<QuickOrderRecovery>,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        self.pending_order_action = None;
        self.clear_pending_order_indicator(pending_indicator_id);
        let outcome = classify_execution_result(result);
        if outcome.kind == ExecutionOutcomeKind::Rejected
            && self.connected_order_account_matches(&context.account_address)
            && let Some(recovery) = recovery
        {
            self.restore_quick_order_form_if_current(&context.symbol_key, recovery);
        }
        self.apply_one_shot_placement_outcome(context, outcome)
    }
}
