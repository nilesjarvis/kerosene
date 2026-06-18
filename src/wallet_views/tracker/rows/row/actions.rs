use crate::message::Message;

use iced::Element;
use iced::widget::{button, row, text};

pub(super) fn wallet_tracker_actions(address: String, is_muted: bool) -> Element<'static, Message> {
    let mute_button = if is_muted {
        button(text("Unmute").size(10))
            .on_press(Message::WalletTrackerUnmute(address.clone().into()))
            .padding([2, 6])
    } else {
        button(text("Mute").size(10))
            .on_press(Message::WalletTrackerMute(address.clone().into()))
            .padding([2, 6])
    };

    row![
        button(text("Refresh").size(10))
            .on_press(Message::WalletTrackerRefreshOne(address.clone().into()))
            .padding([2, 6]),
        button(text("Orders").size(10))
            .on_press(Message::WalletTrackerRefreshOrders(address.clone().into()))
            .padding([2, 6]),
        mute_button,
        button(text("Delete").size(10))
            .on_press(Message::WalletTrackerRemove(address.clone().into()))
            .padding([2, 6]),
    ]
    .spacing(4)
    .width(220)
    .into()
}
