mod clock;
mod connectivity;

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::container;
use iced::{Color, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Status bar view
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_status_bar(&self) -> Element<'_, Message> {
        let content = container(self.status_connectivity_row())
            .width(Fill)
            .padding([4, 8])
            .style(|theme: &Theme| container_style::Style {
                background: Some(
                    Color {
                        a: 0.96,
                        ..theme.extended_palette().background.strong.color
                    }
                    .into(),
                ),
                ..Default::default()
            });

        container(content).width(Fill).into()
    }
}
