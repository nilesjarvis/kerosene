use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::ExchangeResponse;
use iced::Task;

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(crate) fn handle_order_result(
        &mut self,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        self.pending_order_action = None;
        let should_refresh = result_requires_account_refresh(&result);
        self.set_result_status(result);
        self.refresh_account_after_success(should_refresh)
    }

    pub(crate) fn handle_cancel_result(
        &mut self,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        let should_refresh = result_requires_account_refresh(&result);
        self.set_result_status(result);
        self.refresh_account_after_success(should_refresh)
    }

    pub(crate) fn handle_close_position_result(
        &mut self,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        let should_refresh = result_requires_account_refresh(&result);
        self.set_result_status(result);
        self.refresh_account_after_success(should_refresh)
    }

    pub(crate) fn handle_nuke_result(
        &mut self,
        result: Result<ExchangeResponse, String>,
    ) -> Task<Message> {
        let should_refresh = result_requires_account_refresh(&result);
        self.set_result_status(result);
        self.refresh_account_after_success(should_refresh)
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

    fn set_result_status(&mut self, result: Result<ExchangeResponse, String>) {
        match result {
            Ok(resp) => {
                let is_err = resp.is_error();
                self.set_order_status(resp.summary(), is_err);
            }
            Err(e) => {
                self.set_order_status(e, true);
            }
        }
    }

    fn refresh_account_after_success(&mut self, should_refresh: bool) -> Task<Message> {
        if should_refresh {
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
