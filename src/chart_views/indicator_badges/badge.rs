use crate::chart_state::ChartId;
use crate::message::Message;

use super::active::ActiveIndicator;

use iced::widget::container as container_style;
use iced::widget::{Space, button, container, row, text, tooltip};
use iced::{Alignment, Color, Element, Theme};

const REMOVE_ICON: &str = "X";

// ---------------------------------------------------------------------------
// Badge
// ---------------------------------------------------------------------------

pub(super) fn indicator_badge(
    chart_id: ChartId,
    indicator: ActiveIndicator,
) -> Element<'static, Message> {
    let swatch = container(Space::new().width(6.0).height(6.0))
        .width(6.0)
        .height(6.0)
        .style(move |_theme: &Theme| container_style::Style {
            background: Some(indicator.color.into()),
            border: iced::Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let badge = button(
        row![
            swatch,
            text(indicator.label)
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(indicator.color),
            text(REMOVE_ICON)
                .size(10)
                .font(crate::app_fonts::monospace_font()),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .on_press(Message::ToggleMacroIndicator(
        chart_id,
        indicator.key.to_string(),
    ))
    .padding([2, 6])
    .style(move |theme: &Theme, status| {
        let bg = match status {
            button::Status::Hovered => theme.extended_palette().background.strong.color,
            _ => Color {
                a: 0.86,
                ..theme.extended_palette().background.base.color
            },
        };

        button::Style {
            background: Some(bg.into()),
            text_color: theme.palette().text,
            border: iced::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: Color {
                    a: 0.5,
                    ..indicator.color
                },
            },
            ..Default::default()
        }
    });

    tooltip(
        badge,
        text(format!("Remove {}", indicator.label))
            .size(10)
            .font(crate::app_fonts::monospace_font()),
        tooltip::Position::Bottom,
    )
    .into()
}
