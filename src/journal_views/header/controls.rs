mod filters;
mod refresh;
mod sorting;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::Element;
use iced::widget::{Space, row};

impl TradingTerminal {
    pub(super) fn view_journal_controls(&self) -> Element<'static, Message> {
        let cache_clear_enabled = self.connected_address.is_some() && !self.journal.loading;

        row![
            refresh::journal_refresh_button(self.journal.loading),
            Space::new().width(4.0),
            refresh::journal_clear_cache_button(cache_clear_enabled),
            Space::new().width(12.0),
            sorting::journal_sort_controls(self.journal.sort),
            Space::new().width(12.0),
            filters::journal_filter_controls(self.journal.filter)
        ]
        .align_y(iced::Alignment::Center)
        .into()
    }
}
