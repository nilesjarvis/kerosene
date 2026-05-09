use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::{Space, row, text};
use iced::{Element, Fill};

const WALLET_COLUMN_WIDTH: u32 = 164;

impl TradingTerminal {
    pub(crate) fn view_tracked_trades_header(&self) -> Element<'_, Message> {
        let theme = self.theme();

        row![
            text("Time")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(60),
            text("Wallet")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(WALLET_COLUMN_WIDTH),
            text("Coin")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(80),
            text("Side")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(50),
            text("Size")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(80),
            text("Price")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(80),
            text("Notional")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(80),
            text("PnL")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(80),
            text("Fee")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(80),
            Space::new().width(Fill),
            text("Intent")
                .size(11)
                .color(theme.extended_palette().background.weak.text),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
    }
}
