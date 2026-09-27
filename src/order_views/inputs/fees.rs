use super::order_detail_row;
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
            let terms = self
                .exchange_symbol_for_key(&self.active_symbol)
                .and_then(|symbol| symbol.outcome.as_ref())
                .map(|info| info.fee_terms_label())
                .unwrap_or_else(|| "Outcome fee terms unavailable".to_string());
            return form.push(
                text(terms)
                    .size(11)
                    .color(theme.extended_palette().background.weak.text)
                    .width(Fill),
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

        let fee_text = |is_maker| {
            fee_price
                .zip(fee_qty)
                .and_then(|(price, quantity)| {
                    self.estimate_fee(price, quantity, is_maker, active_is_spot)
                })
                .map(|(amount, _)| format!("${amount:.2}"))
                .unwrap_or_else(|| "\u{2014}".to_string())
        };
        if matches!(self.order_kind, OrderKind::Market | OrderKind::LimitIoc) {
            form.push(order_detail_row("Est. taker fee", fee_text(false), &theme))
        } else {
            form.push(order_detail_row("Est. maker fee", fee_text(true), &theme))
                .push(order_detail_row("Est. taker fee", fee_text(false), &theme))
        }
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
mod tests;
