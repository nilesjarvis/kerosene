mod clock;
mod connectivity;

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{column, container};
use iced::{Color, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Status bar view
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_status_bar(&self) -> Element<'_, Message> {
        container(column![self.status_clock_row(), self.status_connectivity_row()].spacing(2))
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
                border: iced::Border {
                    width: 1.0,
                    color: Color {
                        a: 0.35,
                        ..theme.palette().primary
                    },
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }
}
