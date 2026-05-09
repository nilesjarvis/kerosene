#[path = "unlock/controls.rs"]
mod controls;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, container, text};
use iced::{Color, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Unlock Credentials Overlay
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_unlock_credentials_popup(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let status = self.unlock_credentials_status(&theme);
        let password_row = self.unlock_credentials_password_row();

        let mut content = Column::new()
            .spacing(10)
            .push(
                text("Unlock Credentials")
                    .size(16)
                    .color(theme.palette().text),
            )
            .push(
                text("Encrypted API keys and trading keys are locked.")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            )
            .push(password_row);

        if let Some(status) = status {
            content = content.push(status);
        }

        content = content.push(controls::unlock_credentials_action_row());

        let card = container(content)
            .padding(16)
            .width(iced::Length::Fixed(460.0))
            .style(|theme: &Theme| container_style::Style {
                background: Some(theme.extended_palette().background.base.color.into()),
                border: iced::Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: theme.palette().primary,
                },
                ..Default::default()
            });

        container(card)
            .width(Fill)
            .height(Fill)
            .center(Fill)
            .style(|theme: &Theme| container_style::Style {
                background: Some(
                    Color {
                        a: 0.72,
                        ..theme.extended_palette().background.strong.color
                    }
                    .into(),
                ),
                ..Default::default()
            })
            .into()
    }
}
