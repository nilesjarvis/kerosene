use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::order_update::results::result_requires_account_refresh;
use crate::signing::ExchangeResponse;
use iced::Task;

// ---------------------------------------------------------------------------
// Quick Order Result Handling
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn handle_quick_order_result(
        &mut self,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        let should_refresh = result_requires_account_refresh(&result);
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
