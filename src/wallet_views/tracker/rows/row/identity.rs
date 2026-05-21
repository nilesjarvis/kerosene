use crate::helpers;
use crate::message::Message;
use crate::wallet_state::address_book::WalletDisplay;

use iced::widget::{column, text, text_input};
use iced::{Color, Element};

pub(super) fn wallet_identity_cell(
    address: String,
    label_value: String,
    display: WalletDisplay,
    secondary_text: Color,
) -> Element<'static, Message> {
    let address_text = if display.has_label {
        display.secondary
    } else {
        display.primary
    };

    column![
        text_input("Label", &label_value)
            .style(helpers::text_input_style)
            .on_input({
                let address = address.clone();
                move |value| Message::WalletTrackerLabelChanged(address.clone(), value)
            })
            .size(11)
            .padding([3, 6])
            .width(185),
        text(address_text)
            .size(10)
            .font(crate::app_fonts::monospace_font())
            .color(secondary_text),
    ]
    .spacing(3)
    .width(205)
    .into()
}
