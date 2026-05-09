use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Space, row, text};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(in crate::wallet_views::tracker) fn view_wallet_tracker_table_header(
        theme: &Theme,
    ) -> Element<'static, Message> {
        row![
            text("Wallet")
                .size(10)
                .color(theme.extended_palette().background.weak.text)
                .width(205),
            text("Equity")
                .size(10)
                .color(theme.extended_palette().background.weak.text)
                .width(85),
            text("Available")
                .size(10)
                .color(theme.extended_palette().background.weak.text)
                .width(85),
            text("uPnL")
                .size(10)
                .color(theme.extended_palette().background.weak.text)
                .width(75),
            text("Margin")
                .size(10)
                .color(theme.extended_palette().background.weak.text)
                .width(60),
            text("Risk")
                .size(10)
                .color(theme.extended_palette().background.weak.text)
                .width(95),
            text("State")
                .size(10)
                .color(theme.extended_palette().background.weak.text)
                .width(90),
            Space::new().width(Fill),
            text("Actions")
                .size(10)
                .color(theme.extended_palette().background.weak.text)
                .width(280),
        ]
        .spacing(8)
        .padding([0, 8])
        .into()
    }
}
