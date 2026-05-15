use crate::app_state::TradingTerminal;
use crate::helpers::parse_number;
use crate::message::Message;
use crate::order_execution::order_size_from_quantity_input;
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
            OrderKind::Limit | OrderKind::LimitIoc => parse_number(&self.order_price),
            OrderKind::Market | OrderKind::Chase | OrderKind::Twap => {
                self.resolve_mid_for_symbol(&self.active_symbol)
            }
        };
        let fee_qty = fee_price.and_then(|price| {
            let sz_decimals = self
                .exchange_symbols
                .iter()
                .find(|symbol| symbol.key == self.active_symbol)
                .map(|symbol| symbol.sz_decimals)?;
            order_fee_quantity(
                &self.order_quantity,
                price,
                self.order_quantity_is_usd,
                sz_decimals,
            )
        });

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

fn order_fee_quantity(
    raw_quantity: &str,
    price: f64,
    quantity_is_usd: bool,
    sz_decimals: u32,
) -> Option<f64> {
    let quantity = parse_number(raw_quantity)?;
    order_size_from_quantity_input(quantity, price, quantity_is_usd, sz_decimals)
}

#[cfg(test)]
mod tests {
    use super::order_fee_quantity;

    #[test]
    fn usd_fee_quantity_converts_notional_to_base_size() {
        assert_eq!(order_fee_quantity("250", 100.0, true, 5), Some(2.5));
    }

    #[test]
    fn coin_fee_quantity_keeps_base_size() {
        assert_eq!(order_fee_quantity("2.5", 100.0, false, 5), Some(2.5));
    }

    #[test]
    fn usd_fee_quantity_uses_asset_precision() {
        assert_eq!(order_fee_quantity("10", 30_000.0, true, 5), Some(0.00033));
        assert_eq!(order_fee_quantity("10", 30_000.0, true, 2), None);
    }
}
