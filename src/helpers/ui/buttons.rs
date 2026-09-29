use super::text_color_for_bg;
use crate::message::Message;
use iced::widget::{button, text};
use iced::{Color, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Shared Buttons
// ---------------------------------------------------------------------------

pub fn selected_row_button_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    let bg = match status {
        button::Status::Hovered => theme.extended_palette().background.strong.color,
        _ if selected => theme.extended_palette().background.weak.color,
        _ => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(bg.into()),
        text_color: theme.palette().text,
        border: iced::Border {
            radius: 4.0.into(),
            width: if selected { 1.0 } else { 0.0 },
            color: if selected {
                theme.palette().primary
            } else {
                Color::TRANSPARENT
            },
        },
        ..Default::default()
    }
}

pub fn order_type_button(label: &str, active: bool, msg: Message) -> Element<'_, Message> {
    let btn = button(
        text(label)
            .size(if active { 13 } else { 12 })
            .center()
            .width(Fill),
    )
    .on_press(msg)
    .padding([6, 8])
    .width(Fill);

    btn.style(move |theme: &Theme, status| {
        let palette = theme.palette();
        let extended = theme.extended_palette();

        let bg = if active {
            Color {
                a: 0.15,
                ..palette.primary
            }
        } else {
            match status {
                button::Status::Hovered => extended.background.strong.color,
                _ => extended.background.weak.color,
            }
        };

        button::Style {
            background: Some(bg.into()),
            text_color: if active {
                palette.primary
            } else {
                extended.background.weak.text
            },
            border: iced::Border {
                radius: 4.0.into(),
                width: if active { 1.0 } else { 0.0 },
                color: if active {
                    Color {
                        a: 0.3,
                        ..palette.primary
                    }
                } else {
                    Color::TRANSPARENT
                },
            },
            ..Default::default()
        }
    })
    .into()
}

pub fn buy_button(label: String, msg: Message) -> Element<'static, Message> {
    trade_action_button(label, msg, |theme| theme.palette().success)
}

pub fn sell_button(label: String, msg: Message) -> Element<'static, Message> {
    trade_action_button(label, msg, |theme| theme.palette().danger)
}

fn trade_action_button(
    label: String,
    msg: Message,
    base_color: fn(&Theme) -> Color,
) -> Element<'static, Message> {
    button(text(label).size(14).center().width(Fill))
        .on_press(msg)
        .padding([8, 16])
        .width(Fill)
        .style(move |theme: &Theme, status| trade_action_button_style(base_color(theme), status))
        .into()
}

fn trade_action_button_style(base_color: Color, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color {
            a: 0.9,
            ..base_color
        },
        _ => base_color,
    };

    button::Style {
        background: Some(bg.into()),
        text_color: text_color_for_bg(base_color),
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
