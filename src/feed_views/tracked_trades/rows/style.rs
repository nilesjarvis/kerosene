use iced::widget::container as container_style;
use iced::{Color, Theme};

pub(super) fn tracked_trade_pnl_color(theme: &Theme, closed_pnl: f64) -> Color {
    if closed_pnl > 0.0 {
        theme.palette().success
    } else if closed_pnl < 0.0 {
        theme.palette().danger
    } else {
        theme.extended_palette().background.weak.text
    }
}

pub(super) fn tracked_trade_row_style(
    side_color: Color,
    notional: f64,
) -> impl Fn(&Theme) -> container_style::Style {
    let (opacity, brightness) = notional_intensity(notional);
    let row_color = brightened_color(side_color, brightness);

    move |_theme: &Theme| container_style::Style {
        background: Some(
            Color {
                a: opacity,
                ..row_color
            }
            .into(),
        ),
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn notional_intensity(notional: f64) -> (f32, f32) {
    if notional < 1_000.0 {
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
    }
}

fn brightened_color(mut color: Color, brightness: f32) -> Color {
    color.r = (color.r * brightness).min(1.0);
    color.g = (color.g * brightness).min(1.0);
    color.b = (color.b * brightness).min(1.0);
    color
}
