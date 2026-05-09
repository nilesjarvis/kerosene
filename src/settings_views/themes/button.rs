use crate::app_state::TradingTerminal;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Space, button, container, row, text};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_theme_option_button(
        &self,
        theme_name: String,
        message: Message,
        is_active: bool,
        preview_palette: iced::theme::Palette,
    ) -> Element<'static, Message> {
        let active_check_color = self.theme().palette().primary;
        let preview_row = row![
            color_square(preview_palette.background),
            color_square(preview_palette.text),
            color_square(preview_palette.primary),
            color_square(preview_palette.success),
            color_square(preview_palette.danger),
        ]
        .spacing(2);

        let mut btn_content = row![preview_row, text(theme_name).size(12),]
            .spacing(12)
            .align_y(iced::Alignment::Center)
            .width(Fill);

        if is_active {
            btn_content = btn_content.push(
                text("\u{2713}")
                    .size(14)
                    .color(active_check_color)
                    .align_x(iced::alignment::Horizontal::Right)
                    .width(Fill),
            );
        }

        button(btn_content)
            .padding([6, 10])
            .width(Fill)
            .on_press(message)
            .style(move |t: &Theme, status| {
                let extended = t.extended_palette();
                let bg = match status {
                    button::Status::Hovered => extended.background.strong.color.into(),
                    _ => extended.background.weak.color.into(),
                };

                button::Style {
                    background: Some(bg),
                    text_color: t.palette().text,
                    border: iced::Border {
                        radius: 4.0.into(),
                        width: if is_active { 1.0 } else { 0.0 },
                        color: if is_active {
                            t.palette().primary
                        } else {
                            iced::Color::TRANSPARENT
                        },
                    },
                    ..Default::default()
                }
            })
            .into()
    }
}

fn color_square(color: iced::Color) -> Element<'static, Message> {
    container(Space::new().width(12).height(12))
        .style(move |_theme: &Theme| container_style::Style {
            background: Some(color.into()),
            border: iced::Border {
                radius: 2.0.into(),
                width: 1.0,
                color: iced::Color {
                    a: 0.2,
                    ..iced::Color::BLACK
                },
            },
            ..Default::default()
        })
        .into()
}
