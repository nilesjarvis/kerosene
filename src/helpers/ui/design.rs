//! Shared compact controls and resolved color roles for the design-language rollout.
//! New consumers opt in; existing feature styles are not changed implicitly.

use iced::widget::{button, container, text_input};
use iced::{Border, Color, Theme};

pub(crate) const CONTROL_HEIGHT: f32 = 24.0;
pub(crate) const CONTROL_TEXT_SIZE: u32 = 12;
pub(crate) const META_TEXT_SIZE: u32 = 11;

#[derive(Clone, Copy)]
pub(crate) struct UiColors {
    pub(crate) panel: Color,
    pub(crate) inset: Color,
    pub(crate) text: Color,
    pub(crate) muted: Color,
    pub(crate) accent: Color,
    pub(crate) positive: Color,
    pub(crate) negative: Color,
    pub(crate) warning: Color,
    pub(crate) divider: Color,
}

impl UiColors {
    pub(crate) fn new(theme: &Theme) -> Self {
        let palette = theme.palette();
        let extended = theme.extended_palette();
        let panel = extended.background.strong.color;
        let text = readable_color(palette.text, panel, 4.5);
        Self {
            panel,
            inset: extended.background.weak.color,
            text,
            muted: readable_color(
                mix_color(panel, extended.background.weak.text, 0.72),
                panel,
                4.5,
            ),
            accent: readable_color(palette.primary, panel, 4.5),
            positive: readable_color(palette.success, panel, 4.5),
            negative: readable_color(palette.danger, panel, 4.5),
            warning: readable_color(palette.warning, panel, 4.5),
            divider: mix_color(panel, text, 0.12),
        }
    }

    pub(crate) fn overlay(self, corner_radius: f32) -> container::Style {
        container::Style {
            background: Some(
                Color {
                    a: 1.0,
                    ..self.panel
                }
                .into(),
            ),
            text_color: Some(self.text),
            border: Border {
                width: 1.0,
                color: self.divider,
                radius: crate::config::effective_radius(corner_radius, 6.0).into(),
            },
            ..Default::default()
        }
    }
}

/// Mix surface RGB while preserving the surface's configured window opacity.
pub(crate) fn mix_color(surface: Color, tint: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    Color {
        r: surface.r + (tint.r - surface.r) * amount,
        g: surface.g + (tint.g - surface.g) * amount,
        b: surface.b + (tint.b - surface.b) * amount,
        a: surface.a,
    }
}

/// Contrast of resolved RGB pairs; desktop content behind transparent windows
/// is unknown, so this does not certify contrast against an arbitrary wallpaper.
pub(crate) fn contrast_ratio(a: Color, b: Color) -> f32 {
    let luminance = |color: Color| {
        let [r, g, b, _] = color.into_linear();
        r * 0.2126 + g * 0.7152 + b * 0.0722
    };
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

pub(crate) fn readable_color(preferred: Color, background: Color, minimum: f32) -> Color {
    let preferred = Color {
        a: 1.0,
        ..preferred
    };
    let fallback =
        if contrast_ratio(Color::BLACK, background) > contrast_ratio(Color::WHITE, background) {
            Color::BLACK
        } else {
            Color::WHITE
        };
    (0..=100)
        .map(|step| mix_color(preferred, fallback, step as f32 / 100.0))
        .find(|color| contrast_ratio(*color, background) >= minimum)
        .unwrap_or(fallback)
}

pub(crate) fn compact_button_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
    corner_radius: f32,
) -> button::Style {
    let colors = UiColors::new(theme);
    let disabled = status == button::Status::Disabled;
    let amount = match status {
        button::Status::Hovered => 0.07,
        button::Status::Pressed => 0.12,
        _ => 0.0,
    };
    let background = if disabled {
        colors.inset
    } else if selected {
        mix_color(colors.panel, theme.palette().primary, 0.12 + amount)
    } else {
        mix_color(colors.panel, colors.text, amount)
    };
    let foreground = if selected && !disabled {
        colors.accent
    } else {
        colors.muted
    };
    button::Style {
        background: (selected || disabled || amount > 0.0).then_some(background.into()),
        text_color: readable_color(foreground, background, 4.5),
        border: Border {
            width: 1.0,
            color: if selected {
                readable_color(foreground, background, 3.0)
            } else {
                Color::TRANSPARENT
            },
            radius: crate::config::effective_radius(corner_radius, 4.0).into(),
        },
        ..Default::default()
    }
}

pub(crate) fn compact_input_style(
    theme: &Theme,
    status: text_input::Status,
    invalid: bool,
    corner_radius: f32,
) -> text_input::Style {
    let colors = UiColors::new(theme);
    let focused = matches!(status, text_input::Status::Focused { .. });
    let disabled = status == text_input::Status::Disabled;
    let background = match status {
        text_input::Status::Hovered => mix_color(colors.inset, colors.text, 0.03),
        text_input::Status::Disabled => mix_color(colors.panel, colors.text, 0.03),
        _ => colors.inset,
    };
    let border = if invalid {
        colors.negative
    } else if focused {
        colors.accent
    } else {
        colors.muted
    };
    let value = readable_color(
        if disabled { colors.muted } else { colors.text },
        background,
        4.5,
    );
    // Iced uses the same foreground for selected and unselected input text.
    let selection = (0..=20)
        .rev()
        .map(|step| mix_color(background, theme.palette().primary, step as f32 / 100.0))
        .find(|selection| contrast_ratio(value, *selection) >= 4.5)
        .unwrap_or(background);
    text_input::Style {
        background: background.into(),
        border: Border {
            width: if focused { 2.0 } else { 1.0 },
            color: readable_color(border, background, 3.0),
            radius: crate::config::effective_radius(corner_radius, 4.0).into(),
        },
        icon: readable_color(colors.muted, background, 4.5),
        placeholder: readable_color(colors.muted, background, 4.5),
        value,
        selection,
    }
}

#[cfg(test)]
mod tests;
