use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{button, text};
use iced::{Element, Theme};

// ---------------------------------------------------------------------------
// Account Summary Window Buttons
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn summary_widgets_button(&self) -> Element<'_, Message> {
        button(
            text(if self.add_widget_menu_open {
                "Widgets ^"
            } else {
                "Widgets v"
            })
            .size(10)
            .center(),
        )
        .on_press(Message::ToggleAddWidgetMenu)
        .padding([2, 6])
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().success,
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
    }

    pub(crate) fn summary_settings_button(&self) -> Element<'_, Message> {
        button(text("\u{2699}").size(12).center())
            .on_press(Message::OpenSettingsWindow)
            .padding([2, 6])
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.strong.color,
                    _ => theme.extended_palette().background.weak.color,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: theme.palette().text,
                    border: iced::Border {
                        radius: 3.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    pub(crate) fn summary_disconnect_button(&self) -> Element<'_, Message> {
        button(text("X").size(11).center())
            .on_press(Message::DisconnectWallet)
            .padding([2, 6])
            .style(|theme: &Theme, _status| button::Style {
                background: Some(theme.extended_palette().background.strong.color.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }
}
