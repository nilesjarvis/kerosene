use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::message::Message;
use crate::signing::ExchangeResponse;
use iced::Task;

use super::super::results::result_requires_account_refresh;

impl TradingTerminal {
    pub(crate) fn handle_quick_order_qty_changed(&mut self, id: ChartId, qty: String) {
        let qty = if self
            .charts
            .get(&id)
            .is_some_and(|inst| self.is_outcome_coin(&inst.symbol))
        {
            Self::sanitize_outcome_quantity_input(&qty)
        } else {
            qty
        };
        if let Some(instance) = self.charts.get_mut(&id)
            && let Some(form) = &mut instance.quick_order
        {
            form.quantity = qty;
        }
    }

    pub(crate) fn handle_quick_order_toggle_type(&mut self, id: ChartId) {
        if let Some(instance) = self.charts.get_mut(&id)
            && let Some(form) = &mut instance.quick_order
        {
            form.is_limit = !form.is_limit;
            instance.last_quick_order_is_limit = form.is_limit;
        }
    }

    pub(crate) fn handle_close_quick_order(&mut self, id: ChartId) {
        if let Some(instance) = self.charts.get_mut(&id) {
            instance.clear_quick_order();
        }
    }

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
