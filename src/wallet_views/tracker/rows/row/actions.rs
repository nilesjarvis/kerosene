use crate::message::Message;

use iced::Element;
use iced::widget::{button, row, text};

pub(super) fn wallet_tracker_actions(address: String, is_muted: bool) -> Element<'static, Message> {
    let mute_button = if is_muted {
        button(text("Unmute").size(10))
            .on_press(Message::WalletTrackerUnmute(address.clone()))
            .padding([2, 6])
    } else {
        button(text("Mute").size(10))
            .on_press(Message::WalletTrackerMute(address.clone()))
            .padding([2, 6])
    };

    row![
        button(text("Copy").size(10))
            .on_press(Message::CopyToClipboard(address.clone()))
            .padding([2, 6]),
        button(text("Details").size(10))
            .on_press(Message::OpenWalletDetailsWindow(address.clone()))
            .padding([2, 6]),
        button(text("Ghost").size(10))
            .on_press(Message::GhostWallet(address.clone()))
            .padding([2, 6]),
        button(text("Refresh").size(10))
            .on_press(Message::WalletTrackerRefreshOne(address.clone()))
            .padding([2, 6]),
        button(text("Orders").size(10))
            .on_press(Message::WalletTrackerRefreshOrders(address.clone()))
            .padding([2, 6]),
        mute_button,
        button(text("Delete").size(10))
            .on_press(Message::WalletTrackerRemove(address.clone()))
            .padding([2, 6]),
    ]
    .spacing(4)
    .width(330)
    .into()
}
