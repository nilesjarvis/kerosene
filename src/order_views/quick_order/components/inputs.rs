use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::helpers;
use crate::message::Message;
use crate::order_execution::QuickOrderForm;
use iced::Element;
use iced::widget::{text, text_input};

impl TradingTerminal {
    pub(in crate::order_views::quick_order) fn quick_order_quantity_input<'a>(
        chart_id: ChartId,
        form: &'a QuickOrderForm,
    ) -> Element<'a, Message> {
        let id = chart_id;
        text_input("Quantity", &form.quantity)
            .id(iced::widget::Id::from(format!(
                "quick_order_qty_{}",
                chart_id
            )))
            .style(helpers::text_input_style)
            .on_input(move |q| Message::QuickOrderQtyChanged(id, q))
            .on_submit(Message::SubmitQuickOrder(id, true))
            .size(12)
            .padding([4, 6])
            .into()
    }

    pub(in crate::order_views::quick_order) fn quick_order_fee_estimate<'a>(
        &'a self,
        form: &QuickOrderForm,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let is_spot = self.is_spot_coin(&self.active_symbol);
        let fee_qty = form.quantity.parse::<f64>().ok().filter(|q| *q > 0.0);
        let fee_price = if form.is_limit {
            Some(form.price)
        } else {
            self.resolve_mid_for_symbol(&self.active_symbol)
        };

        match (fee_price, fee_qty) {
            (Some(px), Some(qty)) => {
                let m_text = if let Some((fee_amt, _)) = self.estimate_fee(px, qty, true, is_spot) {
                    format!("Maker: ${fee_amt:.2}")
                } else {
                    "Maker: \u{2014}".to_string()
                };

                let t_text = if let Some((fee_amt, _)) = self.estimate_fee(px, qty, false, is_spot)
                {
                    format!("Taker: ${fee_amt:.2}")
                } else {
                    "Taker: \u{2014}".to_string()
                };

                text(format!("Est. Fees: {m_text} | {t_text}"))
                    .size(9)
                    .color(theme.extended_palette().background.weak.text)
                    .into()
            }
            _ => text("Est. Fees: Maker: \u{2014} | Taker: \u{2014}")
                .size(9)
                .color(theme.extended_palette().background.weak.text)
                .into(),
        }
    }
}
