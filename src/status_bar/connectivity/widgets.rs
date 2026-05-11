use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{Space, button, container, row, rule, text, tooltip};
use iced::{Color, Element, Theme};

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

pub(super) fn status_group_separator<'a>() -> Element<'a, Message> {
    container(rule::vertical(1).style(|theme: &Theme| rule::Style {
        color: Color {
            a: 0.42,
            ..theme.palette().primary
        },
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }))
    .height(18)
    .padding([0, 6])
    .into()
}

pub(super) fn ws_status_badge<'a>(
    label: &'static str,
    color: Color,
    live: bool,
    pulse_phase: f32,
) -> Element<'a, Message> {
    let mut content = row![].align_y(iced::Alignment::Center);
    if live {
        content = content.push(live_dot(color, pulse_phase));
    }

    content = content.push(text(label).size(10).color(color));

    container(content.spacing(5))
        .padding([1, 6])
        .style(move |theme: &Theme| {
            let background = if live {
                Color { a: 0.08, ..color }
            } else {
                Color {
                    a: 0.30,
                    ..theme.extended_palette().background.weak.color
                }
            };
            let border_color = Color {
                a: if live { 0.22 } else { 0.16 },
                ..color
            };
            container_style::Style {
                background: Some(background.into()),
                border: iced::Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: border_color,
                },
                ..Default::default()
            }
        })
        .into()
}

fn live_dot<'a>(color: Color, pulse_phase: f32) -> Element<'a, Message> {
    let pulse = 0.5 + 0.5 * pulse_phase.sin();
    let alpha = 0.48 + 0.24 * pulse;

    container(Space::new().width(7).height(7))
        .style(move |_theme: &Theme| container_style::Style {
            background: Some(Color { a: alpha, ..color }.into()),
            border: iced::Border {
                radius: 10.0.into(),
                width: 1.0,
                color: Color { a: 0.28, ..color },
            },
            ..Default::default()
        })
        .into()
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
