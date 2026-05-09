mod filters;
mod refresh;
mod sorting;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Element;
use iced::widget::{Space, row};

impl TradingTerminal {
    pub(super) fn view_journal_controls(&self) -> Element<'static, Message> {
        row![
            refresh::journal_refresh_button(self.journal.loading),
            Space::new().width(12.0),
            sorting::journal_sort_controls(self.journal.sort),
            Space::new().width(12.0),
            filters::journal_filter_controls(self.journal.filter)
        ]
        .align_y(iced::Alignment::Center)
        .into()
    }
}
