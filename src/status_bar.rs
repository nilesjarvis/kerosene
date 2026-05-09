mod clock;
mod connectivity;

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{column, container, rule};
use iced::{Color, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Status bar view
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_status_bar(&self) -> Element<'_, Message> {
        let separator = rule::horizontal(1).style(|theme: &Theme| rule::Style {
            color: Color {
                a: 0.35,
                ..theme.palette().primary
            },
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        });

        let content =
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
                    ..Default::default()
                });

        container(column![separator, content]).width(Fill).into()
    }
}
