use crate::app_state::TradingTerminal;
use crate::assistant::AssistantRole;
use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{Column, Space, button, column, container, row, text};
use iced::{Color, Fill, Theme, color};

impl TradingTerminal {
    pub(super) fn view_assistant_transcript<'a>(&'a self, theme: &Theme) -> Column<'a, Message> {
        if self.assistant.history.is_empty() {
            return column![
                text("Ask a question and include symbols like ${HYPE}")
                    .size(11)
                    .color(color!(0x8a93b3)),
                text("The assistant plans first, then runs deterministic calculations.")
                    .size(10)
                    .color(color!(0x707a9e)),
            ]
            .spacing(4);
        }

        self.assistant
            .history
            .iter()
            .fold(Column::new().spacing(6), |col, msg| {
                let (label, border) = match msg.role {
                    AssistantRole::User => ("You", color!(0x4fa3ff)),
                    AssistantRole::Assistant => ("Assistant", theme.palette().success),
                    AssistantRole::System => ("Plan", theme.palette().primary),
                };
                let copy_text = msg.content.clone();
                let copy_btn = button(text("\u{2398}").size(10))
                    .on_press(Message::AssistantCopyText(copy_text))
                    .padding([1, 5])
                    .style(|theme: &Theme, status| {
                        let bg = match status {
                            button::Status::Hovered => Color {
                                a: 0.45,
                                ..theme.extended_palette().primary.weak.color
                            },
                            _ => Color {
                                a: 0.25,
                                ..theme.extended_palette().primary.weak.color
                            },
                        };
                        button::Style {
                            background: Some(bg.into()),
                            text_color: theme.palette().text,
                            border: iced::Border {
                                radius: 3.0.into(),
                                width: 1.0,
                                color: Color {
                                    a: 0.35,
                                    ..theme.palette().primary
                                },
                            },
                            ..Default::default()
                        }
                    });

                let theme_clone = theme.clone();
                let bubble_style = move |_theme: &Theme| container_style::Style {
                    background: Some(
                        Color {
                            a: 0.95,
                            ..theme_clone.extended_palette().background.strong.color
                        }
                        .into(),
                    ),
                    border: iced::Border {
                        radius: 4.0.into(),
                        width: 1.0,
                        color: border,
                    },
                    ..Default::default()
                };
                let bubble = container(
                    column![
                        row![
                            text(label).size(10).color(color!(0x9ba6c8)),
                            container(Space::new()).width(Fill),
                            copy_btn,
                        ]
                        .align_y(iced::Alignment::Center),
                        text(&msg.content).size(11)
                    ]
                    .spacing(3),
                )
                .padding([6, 8])
                .style(bubble_style);
                col.push(bubble)
            })
    }
}
