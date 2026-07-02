use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::OrderKind;

use iced::widget::Column;

mod fees;
mod price;
mod size;
mod warnings;

// ---------------------------------------------------------------------------
// Order Inputs
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_order_input_controls<'a>(
        &'a self,
        form: Column<'a, Message>,
        active_is_spot: bool,
        active_is_outcome: bool,
    ) -> Column<'a, Message> {
        if self.order_kind == OrderKind::Twap {
            let (form, notional_val) =
                self.push_size_input_controls(form, active_is_spot, active_is_outcome);
            return self.push_leverage_warning(
                form,
                active_is_spot,
                active_is_outcome,
                notional_val,
            );
        }

        let form = self.push_price_input_controls(form);
        let (form, notional_val) =
            self.push_size_input_controls(form, active_is_spot, active_is_outcome);
        let form = self.push_fee_estimate(form, active_is_spot, active_is_outcome);
        self.push_leverage_warning(form, active_is_spot, active_is_outcome, notional_val)
    }
}
