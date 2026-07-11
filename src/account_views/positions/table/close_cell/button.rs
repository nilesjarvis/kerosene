use crate::account_views::style::compact_action_button;
use crate::message::Message;
use iced::widget::{button, text, tooltip};
use iced::{Color, Element, Theme};

pub(super) fn view_position_hide_button(
    coin: String,
    is_hidden: bool,
    theme: &Theme,
) -> Element<'static, Message> {
    let icon = if is_hidden { "\u{25C9}" } else { "\u{2298}" };
    let tooltip_label = if is_hidden {
        "Unhide position"
    } else {
        "Hide position"
    };
    tooltip(
        button(
            text(icon)
                .size(11)
                .center()
                .color(hidden_button_color(theme)),
        )
        .on_press(Message::ToggleHiddenPosition(coin.into()))
        .padding([1, 5])
        .style(|theme: &Theme, status| {
            let color = hidden_button_color(theme);
            let background = match status {
                button::Status::Hovered => Color { a: 0.16, ..color },
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(background.into()),
                text_color: color,
                border: iced::Border {
                    radius: 3.0.into(),
                    width: 1.0,
                    color: Color { a: 0.35, ..color },
                },
                ..Default::default()
            }
        }),
        text(tooltip_label).size(10),
        tooltip::Position::Top,
    )
    .into()
}

pub(super) fn view_position_close_button(
    coin_for_close: String,
    theme: &Theme,
) -> Element<'static, Message> {
    compact_action_button(
        "Close",
        theme.palette().danger,
        Message::ToggleCloseMenu(coin_for_close.into()),
    )
}

fn hidden_button_color(theme: &Theme) -> Color {
    theme.extended_palette().background.weak.text
}
