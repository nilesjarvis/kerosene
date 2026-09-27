use crate::helpers;
use crate::message::Message;
use crate::wallet_state::address_book::WalletDisplay;
use crate::wallet_views::{WalletAddressActionCell, wallet_address_action_cell};

use iced::widget::{column, text, text_input};
use iced::{Element, Theme};

pub(super) fn wallet_identity_cell(
    address: String,
    label_value: String,
    display: WalletDisplay,
    is_remote: bool,
    hovered_wallet_action_key: Option<&str>,
    theme: &Theme,
) -> Element<'static, Message> {
    let address_text = if display.has_label {
        display.secondary.clone()
    } else {
        display.primary.clone()
    };
    let tooltip_label = if display.has_label {
        format!("{} ({address})", display.primary)
    } else {
        format!("Copy {address}")
    };
    let secondary_text = theme.extended_palette().background.weak.text;

    let label: Element<'static, Message> = if is_remote {
        column![
            text(if label_value.is_empty() {
                "Unlabeled".into()
            } else {
                label_value
            })
            .size(11),
            text("Remote").size(10).color(secondary_text),
        ]
        .width(185)
        .into()
    } else {
        text_input("Label", &label_value)
            .style(helpers::text_input_style)
            .on_input({
                let address = address.clone();
                move |value| Message::WalletTrackerLabelChanged(address.clone().into(), value)
            })
            .size(11)
            .padding([3, 6])
            .width(185)
            .into()
    };

    column![
        label,
        wallet_address_action_cell(WalletAddressActionCell {
            address: address.clone(),
            label: address_text,
            tooltip_label,
            hover_key: format!("wallet-tracker:{address}"),
            hovered_key: hovered_wallet_action_key,
            width: 185.0,
            text_size: 10,
            text_color: secondary_text,
        }),
    ]
    .spacing(3)
    .width(205)
    .into()
}
