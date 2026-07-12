use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::{Space, button, container, row, text, tooltip};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_widget_placement_bar(
        &self,
        theme: &Theme,
    ) -> Option<Element<'static, Message>> {
        let widget = self.placing_widget?;
        let weak_text = theme.extended_palette().background.weak.text;

        let cancel = button(text("x").size(11).center())
            .on_press(Message::CancelWidgetPlacement)
            .padding([2, 6])
            .style(|theme: &Theme, status| button::Style {
                background: Some(
                    match status {
                        button::Status::Hovered => theme.extended_palette().background.strong.color,
                        _ => Color::TRANSPARENT,
                    }
                    .into(),
                ),
                text_color: match status {
                    button::Status::Hovered => theme.palette().danger,
                    _ => theme.extended_palette().background.weak.text,
                },
                border: iced::Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        let cancel = tooltip(
            cancel,
            text("Cancel placement").size(10),
            tooltip::Position::Bottom,
        );

        Some(
            container(
                row![
                    text("Placing").size(11).color(weak_text),
                    text(widget.label()).size(11).color(theme.palette().primary),
                    Space::new().width(Fill),
                    cancel,
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .width(Fill)
            .padding([4, 8])
            .style(|theme: &Theme| {
                let mut border = theme.palette().primary;
                border.a = 0.45;
                iced::widget::container::Style {
                    background: Some(theme.extended_palette().background.weak.color.into()),
                    border: iced::Border {
                        width: 1.0,
                        color: border,
                        radius: 2.0.into(),
                    },
                    ..Default::default()
                }
            })
            .into(),
        )
    }
}
