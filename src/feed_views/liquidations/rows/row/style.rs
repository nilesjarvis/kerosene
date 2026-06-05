use iced::Color;
use iced::widget::container as container_style;

// Neutral magnitude colors — row background conveys attention level, not side.
// The Side cell carries the buy/sell semantic color separately.
pub(super) fn liquidation_row_color(
    _theme: &iced::Theme,
    _is_buy: bool,
    notional: f64,
) -> (Color, f32) {
    let color = Color {
        r: 0.55,
        g: 0.45,
        b: 0.35,
        a: 1.0,
    };

    let opacity = if notional < 1_000.0 {
        0.02
    } else if notional < 10_000.0 {
        0.05
    } else if notional < 50_000.0 {
        0.1
    } else if notional < 100_000.0 {
        0.2
    } else if notional < 500_000.0 {
        0.35
    } else {
        0.6
    };

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
