use iced::widget::container as container_style;
use iced::{Color, Theme};

mod contrast;

#[cfg(test)]
pub(super) use contrast::minimum_contrast_ratio;
use contrast::{readable_card_surfaces, relative_luminance};

// ---------------------------------------------------------------------------
// PnL Card Styles
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub(super) struct PnlCardPalette {
    pub(super) start: Color,
    pub(super) mid: Color,
    pub(super) end: Color,
    pub(super) accent: Color,
    pub(super) border: Color,
    pub(super) brand: Color,
    pub(super) brand_ink: Color,
    pub(super) panel: Color,
    pub(super) panel_border: Color,
    pub(super) text: Color,
    pub(super) weak_text: Color,
}

pub(super) fn pnl_card_palette(theme: &Theme, pnl_color: Color) -> PnlCardPalette {
    let palette = theme.palette();
    let extended = theme.extended_palette();
    let raw_start = mix_color(extended.background.strong.color, pnl_color, 0.18);
    let raw_mid = mix_color(
        extended.background.base.color,
        mix_color(pnl_color, palette.primary, 0.38),
        0.10,
    );
    let raw_end = mix_color(extended.background.base.color, palette.background, 0.38);
    let ([start, mid, end], text) = readable_card_surfaces([raw_start, raw_mid, raw_end]);
    let accent = mix_color(pnl_color, text, 0.08);
    let brand = palette.primary;
    let brand_ink = if relative_luminance(brand) > 0.42 {
        Color::from_rgb(0.035, 0.04, 0.05)
    } else {
        Color::WHITE
    };
    let panel_tint = if relative_luminance(text) > 0.5 {
        Color::BLACK
    } else {
        Color::WHITE
    };

    PnlCardPalette {
        start,
        mid,
        end,
        accent,
        border: Color { a: 0.30, ..text },
        brand,
        brand_ink,
        panel: Color {
            a: 0.24,
            ..panel_tint
        },
        panel_border: Color { a: 0.14, ..text },
        text,
        weak_text: Color { a: 0.68, ..text },
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

pub(super) fn pnl_card_frame_style(theme: &Theme, pnl_color: Color) -> container_style::Style {
    let palette = pnl_card_palette(theme, pnl_color);
    container_style::Style {
        border: iced::Border {
            radius: 12.0.into(),
            width: 1.0,
            color: Color {
                a: 0.30,
                ..palette.accent
            },
        },
        shadow: iced::Shadow {
            color: Color {
                a: 0.24,
                ..Color::BLACK
            },
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 20.0,
        },
        ..Default::default()
    }
}
