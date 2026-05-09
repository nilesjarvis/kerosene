use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{button, container, text, tooltip};
use iced::{Color, Theme};

pub(super) fn format_bytes_human(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    let b = bytes as f64;
    if b < KB {
        format!("{bytes} B")
    } else if b < MB {
        format!("{:.1} KB", b / KB)
    } else if b < GB {
        format!("{:.2} MB", b / MB)
    } else {
        format!("{:.2} GB", b / GB)
    }
}

pub(super) fn status_tooltip<'a>(
    label: String,
    body: &'static str,
) -> tooltip::Tooltip<'a, Message> {
    tooltip(
        text(label).size(10).style(|theme: &Theme| text::Style {
            color: Some(theme.palette().primary),
        }),
        container(text(body).size(10).style(|theme: &Theme| text::Style {
            color: Some(theme.palette().primary),
        }))
        .padding([4, 6])
        .style(tooltip_style),
        tooltip::Position::Top,
    )
}

pub(super) fn unlock_credentials_button<'a>() -> button::Button<'a, Message> {
    button(text("Unlock Credentials").size(10))
        .on_press(Message::OpenUnlockCredentialsPopup)
        .padding([1, 6])
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.weak.color,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().warning,
                border: iced::Border {
                    radius: 3.0.into(),
                    width: 1.0,
                    color: Color {
                        a: 0.55,
                        ..theme.palette().warning
                    },
                },
                ..Default::default()
            }
        })
}

fn tooltip_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(
            Color {
                a: 0.98,
                ..theme.extended_palette().background.strong.color
            }
            .into(),
        ),
        border: iced::Border {
            width: 1.0,
            color: Color {
                a: 0.45,
                ..theme.palette().primary
            },
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}
