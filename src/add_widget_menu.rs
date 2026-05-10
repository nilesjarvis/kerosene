use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{container, scrollable};
use iced::{Element, Length, Theme};

mod body;
mod components;

// ---------------------------------------------------------------------------
// Add-widget menu view
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_add_widget_menu_card<'a>(
        &'a self,
        theme: &Theme,
        can_add_income: bool,
    ) -> Element<'a, Message> {
        container(
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
        })
        .into()
    }
}
