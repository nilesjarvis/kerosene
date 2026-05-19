use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Column, button, row, text};
use iced::{Fill, Theme, color};

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
                .style(|_theme: &Theme, _status| button::Style {
                    background: Some(color!(0x3a3a3a).into()),
                    text_color: color!(0xaaaaaa),
                    border: iced::Border {
                        radius: 2.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
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
            form.push(
                text("Outcome orders use USDH, probability prices, and whole-contract sizes")
                    .size(10)
                    .color(color!(0x666666)),
            )
        } else if !can_trade {
            form.push(
                text("Connect wallet + agent key to trade")
                    .size(10)
                    .color(color!(0x666666)),
            )
        } else {
            form
        }
    }
}
