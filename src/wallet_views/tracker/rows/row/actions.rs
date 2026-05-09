use crate::message::Message;

use iced::Element;
use iced::widget::{button, row, text};

pub(super) fn wallet_tracker_actions(address: String) -> Element<'static, Message> {
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
        button(text("Delete").size(10))
            .on_press(Message::WalletTrackerRemove(address.clone()))
            .padding([2, 6]),
    ]
    .spacing(4)
    .width(280)
    .into()
}
