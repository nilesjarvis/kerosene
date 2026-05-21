use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Space, container, row, text};
use iced::{Color, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Trade Card Components
// ---------------------------------------------------------------------------

pub(super) fn journal_note_block<'a>(
    label: &'static str,
    body: &'a str,
    accent_color: Color,
    label_color: Color,
    body_color: Color,
) -> Element<'a, Message> {
    container(
        row![
            container(Space::new().width(4.0).height(Fill)).style(move |_theme: &Theme| {
                container_style::Style {
                    background: Some(accent_color.into()),
                    border: iced::Border {
                        radius: 2.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }),
            Space::new().width(8.0),
            text(label)
                .size(12)
                .color(label_color)
                .font(crate::app_fonts::monospace_font()),
            Space::new().width(8.0),
            text(body).size(12).color(body_color)
        ]
        .align_y(iced::Alignment::Center),
    )
    .width(Fill)
    .padding([8, 12])
    .style(move |_theme: &Theme| container_style::Style {
        background: Some(
            Color {
                a: 0.1,
                ..accent_color
            }
            .into(),
        ),
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}
