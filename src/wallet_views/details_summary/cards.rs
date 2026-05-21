use crate::message::Message;

use iced::widget::container as container_style;
use iced::widget::{column, container, text};
use iced::{Color, Element, Fill, Theme};

pub(super) fn summary_metric_card(
    label: &'static str,
    value: String,
    value_color: Color,
    theme: &Theme,
) -> Element<'static, Message> {
    container(
        column![
            text(label)
                .size(10)
                .color(theme.extended_palette().background.weak.text),
            text(value)
                .size(13)
                .font(crate::app_fonts::monospace_font())
                .color(value_color),
        ]
        .spacing(2),
    )
    .padding([6, 8])
    .width(Fill)
    .style(|theme: &Theme| container_style::Style {
        background: Some(
            Color {
                a: 0.35,
                ..theme.extended_palette().background.weak.color
            }
            .into(),
        ),
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: theme.extended_palette().background.strong.color,
        },
        ..Default::default()
    })
    .into()
}

pub(super) fn summary_pm_status_line(
    pm_ratio: Option<f64>,
    pm_available: String,
    theme: &Theme,
) -> iced::widget::Text<'static> {
    text(format!(
        "Portfolio margin ratio: {} | PM available USDC: {}",
        pm_ratio
            .map(|ratio| format!("{:.2}%", ratio * 100.0))
            .unwrap_or_else(|| "-".to_string()),
        pm_available
    ))
    .size(10)
    .font(crate::app_fonts::monospace_font())
    .color(theme.extended_palette().background.weak.text)
}
