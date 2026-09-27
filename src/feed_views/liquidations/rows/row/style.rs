use crate::app_state::TradingTerminal;
use iced::widget::container as container_style;
use iced::{Color, Theme};

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(super) fn liquidation_row_color(
        &self,
        theme: &Theme,
        is_buy: bool,
        notional: f64,
    ) -> (Color, f32) {
        let (up, down) = self.direction_colors(theme);
        let color = if is_buy { up } else { down };

        // Magnitude changes the background opacity while preserving the candle hue.
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
