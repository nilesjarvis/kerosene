use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_execution::OneShotPlacementContext;
use crate::order_update::results::classify_execution_result;
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
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        self.clear_pending_order_indicator(pending_indicator_id);
        let outcome = classify_execution_result(result);
        self.apply_one_shot_placement_outcome(context, outcome)
    }
}
