use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{row, text};
use iced::{Color, Element};

impl TradingTerminal {
    pub(crate) fn view_calendar_status_row(
        &self,
        status_text: String,
        status_color: Color,
    ) -> Element<'_, Message> {
        let mut status = row![];
        if self.calendar_loading {
            status = status.push(self.view_spinner(10));
        }
        status
            .push(
                text(format!("{} · {status_text}", self.display_timezone))
                    .size(10)
                    .color(status_color),
            )
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .into()
    }
}
