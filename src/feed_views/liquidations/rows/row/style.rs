use iced::widget::container as container_style;
use iced::{Color, Theme};

pub(super) fn liquidation_row_color(theme: &Theme, is_buy: bool, notional: f64) -> (Color, f32) {
    let mut color = if is_buy {
        theme.palette().success
    } else {
        theme.palette().danger
    };

    let (opacity, brightness) = if notional < 1_000.0 {
        (0.02, 0.4)
    } else if notional < 10_000.0 {
        (0.05, 0.55)
    } else if notional < 50_000.0 {
        (0.1, 0.7)
    } else if notional < 100_000.0 {
        (0.2, 0.85)
    } else if notional < 500_000.0 {
        (0.35, 1.0)
    } else {
        (0.6, 1.2)
    };

    color.r = (color.r * brightness).min(1.0);
    color.g = (color.g * brightness).min(1.0);
    color.b = (color.b * brightness).min(1.0);

    (color, opacity)
}

pub(super) fn liquidation_row_style(
    color: Color,
    opacity: f32,
    corner_radius: f32,
) -> container_style::Style {
    let effective_radius = crate::config::effective_radius(corner_radius, 4.0);

    container_style::Style {
        background: Some(
            Color {
                a: opacity,
                ..color
            }
            .into(),
        ),
        border: iced::Border {
            radius: effective_radius.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
