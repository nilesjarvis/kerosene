use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{container, scrollable};
use iced::{Element, Fill, Length, Theme};

mod body;
mod components;

// ---------------------------------------------------------------------------
// Add-widget menu view
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_add_widget_menu<'a>(
        &'a self,
        theme: &Theme,
        can_add_income: bool,
    ) -> Option<Element<'a, Message>> {
        if !self.add_widget_menu_open {
            return None;
        }

        let menu_card = container(
            scrollable(self.view_add_widget_menu_body(theme, can_add_income))
                .height(Length::Shrink),
        )
        .padding(6)
        .width(280)
        .max_height(520.0)
        .style(|theme: &Theme| container_style::Style {
            background: Some(theme.extended_palette().background.base.color.into()),
            border: iced::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: theme.extended_palette().background.strong.color,
            },
            ..Default::default()
        });

        Some(
            container(menu_card)
                .width(Fill)
                .padding(iced::Padding {
                    top: 42.0,
                    right: 16.0,
                    bottom: 0.0,
                    left: 0.0,
                })
                .align_x(iced::Alignment::End)
                .into(),
        )
    }
}
