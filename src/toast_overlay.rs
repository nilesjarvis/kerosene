use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{Column, button, container, row, text};
use iced::{Color, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Toast overlay view
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_toast_overlay(&self, theme: &Theme) -> Option<Element<'_, Message>> {
        if self.toasts.is_empty() {
            return None;
        }

        let success_color = theme.palette().success;
        let danger_color = theme.palette().danger;
        let text_color = theme.palette().text;
        let weak_text_color = theme.extended_palette().background.weak.text;
        let strong_background = theme.extended_palette().background.strong.color;

        let toast_col = self.toasts.iter().rev().take(5).fold(
            Column::new().spacing(4).width(280),
            |col, toast| {
                let border_color = if toast.is_error {
                    danger_color
                } else {
                    success_color
                };
                let message_color = if toast.is_error {
                    danger_color
                } else {
                    text_color
                };
                let tid = toast.id;
                let dismiss = button(text("x").size(9))
                    .on_press(Message::DismissToast(tid))
                    .padding([1, 4])
                    .style(move |_theme: &Theme, _status| button::Style {
                        background: None,
                        text_color: weak_text_color,
                        ..Default::default()
                    });
                let card = container(
                    row![
                        text(&toast.message)
                            .size(11)
                            .color(message_color)
                            .width(Fill),
                        dismiss,
                    ]
                    .spacing(4)
                    .align_y(iced::Alignment::Center),
                )
                .padding([6, 8])
                .style(move |_theme: &Theme| container_style::Style {
                    background: Some(
                        Color {
                            a: 0.95,
                            ..strong_background
                        }
                        .into(),
                    ),
                    border: iced::Border {
                        radius: 4.0.into(),
                        width: 1.0,
                        color: border_color,
                    },
                    ..Default::default()
                });
                col.push(card)
            },
        );

        Some(
            container(toast_col)
                .width(Fill)
                .padding(iced::Padding {
                    top: 8.0,
                    right: 8.0,
                    bottom: 0.0,
                    left: 0.0,
                })
                .align_x(iced::Alignment::End)
                .into(),
        )
    }
}
