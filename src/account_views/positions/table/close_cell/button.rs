use crate::message::Message;
use iced::widget::{button, text};
use iced::{Color, Element, Theme};

pub(super) fn view_position_close_button(
    coin_for_close: String,
    theme: &Theme,
) -> Element<'static, Message> {
    button(
        text("Close")
            .size(10)
            .center()
            .color(theme.palette().danger),
    )
    .on_press(Message::ToggleCloseMenu(coin_for_close))
    .padding([1, 6])
    .style(|theme: &Theme, _status| button::Style {
        background: Some(
            Color {
                a: 0.15,
                ..theme.palette().danger
            }
            .into(),
        ),
        text_color: theme.palette().danger,
        border: iced::Border {
            radius: 3.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}
