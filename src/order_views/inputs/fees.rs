use crate::app_state::TradingTerminal;
use crate::helpers::parse_number;
use crate::message::Message;
use crate::signing::OrderKind;
use iced::Fill;
use iced::widget::{Column, text};

// ---------------------------------------------------------------------------
// Fee Estimate
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_fee_estimate<'a>(
        &'a self,
        form: Column<'a, Message>,
        active_is_spot: bool,
        active_is_outcome: bool,
    ) -> Column<'a, Message> {
        let theme = self.theme();
        if active_is_outcome {
            return form.push(
                text("Est. Fees: unavailable for read-only outcomes")
                    .size(10)
                    .color(theme.extended_palette().background.weak.text)
                    .width(Fill)
                    .align_x(iced::alignment::Horizontal::Center),
            );
        }

        let fee_price = match self.order_kind {
            OrderKind::Limit | OrderKind::Chase | OrderKind::LimitIoc => {
                parse_number(&self.order_price)
            }
            OrderKind::Market => self.resolve_mid_for_symbol(&self.active_symbol),
        };
        let fee_qty = parse_number(&self.order_quantity).filter(|qty| *qty > 0.0);

        let combined_fees = match (fee_price, fee_qty) {
            (Some(price), Some(quantity)) => {
                let maker_text = if let Some((fee_amt, _)) =
                    self.estimate_fee(price, quantity, true, active_is_spot)
                {
                    format!("Maker: ${fee_amt:.2}")
                } else {
                    "Maker: \u{2014}".to_string()
                };

                let taker_text = if let Some((fee_amt, _)) =
                    self.estimate_fee(price, quantity, false, active_is_spot)
                {
                    format!("Taker: ${fee_amt:.2}")
                } else {
                    "Taker: \u{2014}".to_string()
                };

                format!("Est. Fees: {maker_text} | {taker_text}")
            }
            _ => "Est. Fees: Maker: \u{2014} | Taker: \u{2014}".to_string(),
        };

        form.push(
            text(combined_fees)
                .size(10)
                .color(theme.extended_palette().background.weak.text)
                .width(Fill)
                .align_x(iced::alignment::Horizontal::Center),
        )
    }
}
