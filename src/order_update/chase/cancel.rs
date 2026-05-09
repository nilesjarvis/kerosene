use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::{self, ExchangeResponse};

use iced::Task;

impl TradingTerminal {
    pub(crate) fn handle_chase_cancel_result(
        &mut self,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        match result {
            Ok(resp) => {
                if resp.is_error() {
                    self.handle_chase_cancel_error(resp.summary(), false);
                } else if self
                    .active_chase
                    .as_ref()
                    .is_some_and(|chase| chase.stop_requested)
                {
                    self.pending_order_action = None;
                    self.order_status = Some(("Chase stopped".to_string(), false));
                    self.active_chase = None;
                    return self.refresh_account_data();
                } else {
                    if let Some(chase) = &mut self.active_chase {
                        chase.current_oid = None;
                        chase.cancel_in_flight = false;
                        chase.cancel_retries = 0;
                        chase.oid_confirmed = false;
                    }
                    let refresh_task = self.refresh_account_data();
                    let place_task = self.chase_place_at_best();
                    return Task::batch([refresh_task, place_task]);
                }
            }
            Err(e) => {
                self.handle_chase_cancel_error(e, true);
            }
        }

        Task::none()
    }

    fn handle_chase_cancel_error(&mut self, message: String, include_last_on_stop: bool) {
        let still_open = self.chase_order_still_open();
        let oid_confirmed = self
            .active_chase
            .as_ref()
            .is_some_and(|chase| chase.oid_confirmed);
        if !still_open && oid_confirmed {
            self.order_status = Some(("Chase filled".to_string(), false));
            self.active_chase = None;
        } else if let Some(chase) = &mut self.active_chase {
            chase.cancel_retries += 1;
            chase.cancel_in_flight = false;
            if chase.cancel_retries >= signing::MAX_CHASE_CANCEL_RETRIES {
                let suffix = if include_last_on_stop {
                    format!(" (last: {message})")
                } else {
                    String::new()
                };
                self.order_status = Some((
                    format!(
                        "Chase stopped: cancel failed {} times{}",
                        chase.cancel_retries, suffix
                    ),
                    true,
                ));
                self.active_chase = None;
            } else {
                self.order_status = Some((
                    format!(
                        "Chase cancel error (retry {}/{}): {}",
                        chase.cancel_retries,
                        signing::MAX_CHASE_CANCEL_RETRIES,
                        message
                    ),
                    true,
                ));
            }
        }
    }
}
