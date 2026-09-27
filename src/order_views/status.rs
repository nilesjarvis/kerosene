use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Column, button, row, text};
use iced::{Fill, Theme};

impl TradingTerminal {
    pub(super) fn push_order_status_feedback<'a>(
        &'a self,
        form: Column<'a, Message>,
        theme: &Theme,
    ) -> Column<'a, Message> {
        let Some((msg, is_err)) = &self.order_status else {
            return form;
        };

        let status_color = if *is_err {
            theme.palette().danger
        } else {
            theme.palette().success
        };
        let status_row = row![
            text(msg).size(11).color(status_color).width(Fill),
            button(text("X").size(10))
                .on_press(Message::DismissOrderStatus)
                .padding([1, 4])
                .style(button::text),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center);

        form.push(status_row)
    }

    pub(super) fn push_order_entry_hint<'a>(
        &self,
        form: Column<'a, Message>,
        active_is_outcome: bool,
        can_trade: bool,
    ) -> Column<'a, Message> {
        if active_is_outcome {
            let quote_symbol = self.outcome_quote_symbol_for_coin(&self.active_symbol);
            form.push(
                text(format!(
                    "Outcome orders use {quote_symbol} prices and whole-contract sizes"
                ))
                .size(10)
                .color(self.theme().extended_palette().background.weak.text),
            )
        } else if !can_trade {
            form.push(
                text(if self.connected_address.is_none() {
                    "Connect wallet to trade"
                } else if !self.has_active_committed_agent_key() {
                    "Add agent key to trade"
                } else {
                    "Market unavailable for trading"
                })
                .size(10)
                .color(self.theme().extended_palette().background.weak.text),
            )
        } else {
            form
        }
    }
}
