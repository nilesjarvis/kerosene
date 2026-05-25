use iced::Color;

use super::mix_color;

const PNL_CARD_MIN_TEXT_CONTRAST: f32 = 4.5;

// ---------------------------------------------------------------------------
// Contrast
// ---------------------------------------------------------------------------

pub(super) fn readable_card_surfaces(surfaces: [Color; 3]) -> ([Color; 3], Color) {
    let light = Color::WHITE;
    let dark = Color::from_rgb(0.04, 0.04, 0.04);
    let light_surfaces = surfaces.map(|surface| surface_with_min_contrast(surface, light));
    let dark_surfaces = surfaces.map(|surface| surface_with_min_contrast(surface, dark));
    let light_adjustment = contrast_adjustment_score(&surfaces, &light_surfaces);
    let dark_adjustment = contrast_adjustment_score(&surfaces, &dark_surfaces);

    if dark_adjustment < light_adjustment {
        (dark_surfaces, dark)
    } else {
        (light_surfaces, light)
    }
}

fn surface_with_min_contrast(surface: Color, text: Color) -> Color {
    if contrast_ratio(text, surface) >= PNL_CARD_MIN_TEXT_CONTRAST {
        return surface;
    }

    let target = if relative_luminance(text) > 0.5 {
        Color::BLACK
    } else {
        Color::WHITE
    };

    for step in 1..=64 {
        let candidate = mix_color(surface, target, step as f32 / 64.0);
        if contrast_ratio(text, candidate) >= PNL_CARD_MIN_TEXT_CONTRAST {
            return candidate;
        }
    }

    target
}

fn contrast_adjustment_score(original: &[Color; 3], adjusted: &[Color; 3]) -> f32 {
    original
        .iter()
        .zip(adjusted.iter())
        .map(|(left, right)| {
            (left.r - right.r).abs() + (left.g - right.g).abs() + (left.b - right.b).abs()
        })
        .sum()
}

#[cfg(test)]
pub(in crate::pnl_card) fn minimum_contrast_ratio(text: Color, surfaces: &[Color]) -> f32 {
    surfaces
        .iter()
        .map(|surface| contrast_ratio(text, *surface))
        .fold(f32::INFINITY, f32::min)
}

fn contrast_ratio(left: Color, right: Color) -> f32 {
    let left = relative_luminance(left);
    let right = relative_luminance(right);
    let bright = left.max(right);
    let dark = left.min(right);
    (bright + 0.05) / (dark + 0.05)
}

pub(super) fn relative_luminance(color: Color) -> f32 {
    fn channel(value: f32) -> f32 {
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
}
