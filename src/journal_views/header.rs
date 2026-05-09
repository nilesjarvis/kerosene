mod controls;
mod title;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{Space, row};
use iced::{Element, Fill};

// ---------------------------------------------------------------------------
// Journal Header
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_journal_header(&self) -> Element<'_, Message> {
        row![
            self.view_journal_title(),
            Space::new().width(Fill),
            self.view_journal_controls()
        ]
        .align_y(iced::Alignment::Center)
        .into()
    }
}
