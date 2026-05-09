use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::{Space, row, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_liquidations_header(&self) -> Element<'_, Message> {
        let theme = self.theme();
        row![
            text("Time")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(60),
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
            text(if self.liquidation_feed_aggregation_enabled {
                "Avg Px"
            } else {
                "Price"
            })
            .size(11)
            .color(theme.extended_palette().background.weak.text)
            .width(80),
            text("Notional")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(80),
            text("User")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .width(90),
            Space::new().width(Fill),
            text("Method")
                .size(11)
                .color(theme.extended_palette().background.weak.text),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
    }
}
