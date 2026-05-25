use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::optimistic_updates::OrderSubmissionResult;
use crate::order_update::results::{
    order_submission_result_resolved, result_requires_account_refresh,
};
use iced::Task;

// ---------------------------------------------------------------------------
// Quick Order Result Handling
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn handle_quick_order_result(
        &mut self,
        submission: OrderSubmissionResult,
    ) -> Task<Message> {
        let OrderSubmissionResult { context, result } = submission;
        if order_submission_result_resolved(&result) {
            self.clear_pending_order_change(context.pending_id);
        }
        if !self.optimistic_order_context_matches_current_account(&context) {
            return Task::none();
        }
        let should_refresh = result_requires_account_refresh(&result);
        if let Ok(response) = &result {
            self.apply_optimistic_order_result(context, response);
        }
        match result {
            Ok(resp) => {
                let is_err = resp.is_error();
                self.set_order_status(resp.summary(), is_err);
            }
            Err(e) => {
                self.set_order_status(e, true);
            }
        }
        if should_refresh {
            self.refresh_account_data()
        } else {
            Task::none()
        }
    }
}
