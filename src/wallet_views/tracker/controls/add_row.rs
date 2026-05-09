use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use iced::Element;
use iced::widget::{button, row, text, text_input};

impl TradingTerminal {
    pub(in crate::wallet_views::tracker) fn view_wallet_tracker_add_row(
        &self,
    ) -> Element<'_, Message> {
        row![
            text_input("0x wallet address", &self.wallet_tracker.add_input)
                .style(helpers::text_input_style)
                .on_input(Message::WalletTrackerInputChanged)
                .on_submit(Message::WalletTrackerAdd)
                .size(12)
                .padding([6, 8]),
            text_input("Label (optional)", &self.wallet_tracker.add_label_input)
                .style(helpers::text_input_style)
                .on_input(Message::WalletTrackerLabelInputChanged)
                .on_submit(Message::WalletTrackerAdd)
                .size(12)
                .padding([6, 8])
                .width(220),
            button(text("Add").size(11))
                .on_press(Message::WalletTrackerAdd)
                .padding([6, 10]),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
    }
}
