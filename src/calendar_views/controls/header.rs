use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{row, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_calendar_table_header(&self) -> Element<'_, Message> {
        let theme = self.theme();
        row![
            text("Time")
                .size(10)
                .color(theme.extended_palette().background.weak.text)
                .width(92),
            text("CCY")
                .size(10)
                .color(theme.extended_palette().background.weak.text)
                .width(50),
            text("Impact")
                .size(10)
                .color(theme.extended_palette().background.weak.text)
                .width(76),
            text("Event")
                .size(10)
                .color(theme.extended_palette().background.weak.text)
                .width(Fill),
            text("Forecast")
                .size(10)
                .color(theme.extended_palette().background.weak.text)
                .width(72),
            text("Previous")
                .size(10)
                .color(theme.extended_palette().background.weak.text)
                .width(72),
        ]
        .spacing(8)
        .padding([0, 6])
        .into()
    }
}
