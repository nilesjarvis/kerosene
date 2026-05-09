use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::{column, container, rule, scrollable};
use iced::{Element, Fill};

mod controls;
mod input;
mod transcript;

// ---------------------------------------------------------------------------
// Assistant pane view
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_assistant(&self) -> Element<'_, Message> {
        let theme = self.theme();

        let content = column![
            self.view_assistant_controls(),
            self.view_assistant_toggles(),
            rule::horizontal(1),
            container(scrollable(self.view_assistant_transcript(&theme)).height(Fill))
                .width(Fill)
                .height(Fill),
            self.view_assistant_bottom_block(&theme),
        ]
        .height(Fill)
        .spacing(8);

        container(content).width(Fill).height(Fill).into()
    }
}
