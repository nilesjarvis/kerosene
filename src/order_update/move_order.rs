use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::ExchangeResponse;
use iced::Task;

use super::results::result_requires_account_refresh;

impl TradingTerminal {
    pub(super) fn handle_move_order_modify_result(
        &mut self,
        oid: u64,
        pending_indicator_id: Option<u64>,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        let should_refresh = result_requires_account_refresh(&result);
        self.pending_move_order_contexts.remove(&oid);
        self.clear_pending_order_indicator(pending_indicator_id);
        self.sync_all_chart_orders();
        match result {
            Ok(resp) => {
                let is_error = resp.is_error();
                let summary = if is_error {
                    format!("Move failed: {}", resp.summary())
                } else {
                    resp.summary()
                };
                self.order_status = Some((summary, is_error));
            }
            Err(e) => {
                self.order_status = Some((format!("Move modify failed: {e}"), true));
            }
        }
        if should_refresh {
            self.refresh_account_data()
        } else {
            Task::none()
        }
    }
}
