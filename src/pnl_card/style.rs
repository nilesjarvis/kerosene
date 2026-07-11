use iced::gradient;
use iced::widget::container as container_style;
use iced::{Color, Degrees, Theme};
use std::fmt;

mod contrast;

#[cfg(test)]
pub(super) use contrast::minimum_contrast_ratio;
use contrast::{readable_card_surfaces, relative_luminance};

// ---------------------------------------------------------------------------
// PnL Card Styles
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub(super) struct PnlCardPalette {
    pub(super) start: Color,
    pub(super) mid: Color,
    pub(super) end: Color,
    pub(super) border_start: Color,
    pub(super) border_mid: Color,
    pub(super) border_end: Color,
    pub(super) text: Color,
    pub(super) weak_text: Color,
}

impl fmt::Debug for PnlCardPalette {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PnlCardPalette(<redacted>)")
    }
}

pub(super) fn pnl_card_palette(theme: &Theme, pnl_color: Color) -> PnlCardPalette {
    let palette = theme.palette();
    let extended = theme.extended_palette();
    let raw_start = mix_color(pnl_color, palette.primary, 0.26);
    let raw_mid = mix_color(
        extended.background.base.color,
        mix_color(pnl_color, palette.primary, 0.34),
        0.42,
    );
    let raw_end = mix_color(extended.background.weak.color, palette.background, 0.52);
    let ([start, mid, end], text) = readable_card_surfaces([raw_start, raw_mid, raw_end]);
    let border_start = mix_color(palette.primary, Color::WHITE, 0.08);
    let border_mid = mix_color(pnl_color, palette.primary, 0.20);
    let border_end = mix_color(extended.background.strong.color, pnl_color, 0.24);
    let weak_text = Color { a: 0.84, ..text };

    PnlCardPalette {
        start,
        mid,
        end,
        border_start,
        border_mid,
        border_end,
        text,
        weak_text,
    }
}

pub(super) fn mix_color(left: Color, right: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color {
        r: left.r + (right.r - left.r) * t,
        g: left.g + (right.g - left.g) * t,
        b: left.b + (right.b - left.b) * t,
        a: left.a + (right.a - left.a) * t,
    }
}

pub(super) fn pnl_card_detail_band_style(
    theme: &Theme,
    pnl_color: Color,
) -> container_style::Style {
    let palette = pnl_card_palette(theme, pnl_color);
    container_style::Style {
        background: Some(detail_band_color(palette.text, 0.16).into()),
        border: iced::Border {
            radius: 5.0.into(),
            width: 1.0,
            color: Color {
                a: 0.18,
                ..palette.text
            },
        },
        ..Default::default()
    }
}

fn detail_band_color(text_color: Color, alpha: f32) -> Color {
    if relative_luminance(text_color) > 0.5 {
        Color {
            a: alpha,
            ..Color::BLACK
        }
    } else {
        Color {
            a: alpha,
            ..Color::WHITE
        }
    }
}

pub(super) fn detail_band_rgba(text_color: Color, alpha: u8) -> [u8; 4] {
    if relative_luminance(text_color) > 0.5 {
        [0, 0, 0, alpha]
    } else {
        [255, 255, 255, alpha]
    }
}

pub(super) fn pnl_card_border_style(theme: &Theme, pnl_color: Color) -> container_style::Style {
    let palette = pnl_card_palette(theme, pnl_color);

    container_style::Style {
        background: Some(
            gradient::Linear::new(Degrees(135.0))
                .add_stop(0.0, palette.border_start)
                .add_stop(0.45, palette.border_mid)
                .add_stop(1.0, palette.border_end)
                .into(),
        ),
        border: iced::Border {
            radius: 10.0.into(),
            width: 1.0,
            color: Color {
                a: 0.42,
                ..palette.border_mid
            },
        },
        ..Default::default()
    }
}

pub(super) fn pnl_card_inner_style(theme: &Theme, pnl_color: Color) -> container_style::Style {
    let palette = pnl_card_palette(theme, pnl_color);

    container_style::Style {
        background: Some(
            gradient::Linear::new(Degrees(135.0))
                .add_stop(0.0, palette.start)
                .add_stop(0.56, palette.mid)
                .add_stop(1.0, palette.end)
                .into(),
        ),
        border: iced::Border {
            radius: 7.0.into(),
            width: 1.0,
            color: Color {
                a: 0.20,
                ..palette.text
            },
        },
        ..Default::default()
    }
}
