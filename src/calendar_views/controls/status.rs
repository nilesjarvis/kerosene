use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Space, row, text};
use iced::{Color, Element};

impl TradingTerminal {
    pub(crate) fn view_calendar_status_row(
        &self,
        status_text: String,
        status_color: Color,
    ) -> Element<'_, Message> {
        row![
            if self.calendar_loading {
                self.view_spinner(12)
            } else {
                Space::new().width(12).height(12).into()
            },
            text(status_text).size(10).color(status_color),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .into()
    }
}
