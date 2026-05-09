use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::{button, text};
use iced::{Element, Theme};

// ---------------------------------------------------------------------------
// Account Summary Toggles
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn summary_hide_pnl_button(&self) -> Element<'_, Message> {
        let hide = self.hide_pnl;
        button(
            text(if hide { "Show PnL" } else { "Hide PnL" })
                .size(10)
                .center(),
        )
        .on_press(Message::ToggleHidePnl)
        .padding([2, 6])
        .style(move |theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if hide {
                    theme.palette().primary
                } else {
                    theme.extended_palette().background.weak.text
                },
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
    }

    pub(crate) fn summary_sound_button(&self) -> Element<'_, Message> {
        let sound_enabled = self.sound_enabled;
        button(
            text(if sound_enabled {
                "Sound: ON"
            } else {
                "Sound: OFF"
            })
            .size(10)
            .center(),
        )
        .on_press(Message::ToggleSound)
        .padding([2, 6])
        .style(move |theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if sound_enabled {
                    theme.palette().success
                } else {
                    theme.extended_palette().background.weak.text
                },
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
    }

    pub(crate) fn summary_test_sound_button(&self) -> Element<'_, Message> {
        button(text("Test Sound").size(10).center())
            .on_press(Message::TestSound)
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

    pub(crate) fn summary_notifications_button(&self) -> Element<'_, Message> {
        let notifications_enabled = self.desktop_notifications;
        button(
            text(if notifications_enabled {
                "Notif: ON"
            } else {
                "Notif: OFF"
            })
            .size(10)
            .center(),
        )
        .on_press(Message::ToggleDesktopNotifications)
        .padding([2, 6])
        .style(move |theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: if notifications_enabled {
                    theme.palette().success
                } else {
                    theme.extended_palette().background.weak.text
                },
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
    }
}
