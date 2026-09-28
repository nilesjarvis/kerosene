use super::TelegramColors;
use crate::helpers;
use iced::widget::container as container_style;
use iced::widget::{button, rule, text_input};
use iced::{Background, Border, Color, Theme};

/// Left-only padding to indent card body content under the 32px avatar gutter
/// (iced `Padding` has no `[_; 4]` array conversion).
pub(super) fn left_pad(left: f32) -> iced::Padding {
    iced::Padding {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left,
    }
}

pub(super) fn theme_on_orange(colors: TelegramColors) -> Color {
    // Dark ink that reads on the flame-orange fill.
    blend_color(colors.primary, Color::BLACK, 0.82)
}

pub(super) fn blend_color(base: Color, accent: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    Color {
        r: base.r + (accent.r - base.r) * amount,
        g: base.g + (accent.g - base.g) * amount,
        b: base.b + (accent.b - base.b) * amount,
        a: base.a + (accent.a - base.a) * amount,
    }
}

// ----------------------------------------------------------------------------
// Styles
// ----------------------------------------------------------------------------

pub(super) fn telegram_primary_button(
    colors: TelegramColors,
    status: button::Status,
) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let background = if hovered {
        blend_color(colors.primary, Color::WHITE, 0.08)
    } else {
        colors.primary
    };
    button::Style {
        background: Some(background.into()),
        text_color: theme_on_orange(colors),
        border: Border {
            radius: 5.0.into(),
            width: 1.0,
            color: colors.border_orange,
        },
        ..Default::default()
    }
}

pub(super) fn telegram_quiet_button(
    colors: TelegramColors,
    status: button::Status,
) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(
            if hovered {
                Color {
                    a: 0.06,
                    ..colors.text
                }
            } else {
                Color::TRANSPARENT
            }
            .into(),
        ),
        text_color: colors.muted,
        border: Border {
            radius: 5.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(super) fn telegram_accent_button(
    colors: TelegramColors,
    status: button::Status,
) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(
            Color {
                a: if hovered { 0.16 } else { 0.09 },
                ..colors.primary
            }
            .into(),
        ),
        text_color: colors.orange_soft,
        border: Border {
            radius: 5.0.into(),
            width: 1.0,
            color: colors.border_orange,
        },
        ..Default::default()
    }
}

pub(super) fn telegram_icon_button(
    colors: TelegramColors,
    status: button::Status,
) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(
            Color {
                a: if hovered { 0.06 } else { 0.035 },
                ..colors.text
            }
            .into(),
        ),
        text_color: colors.muted,
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: colors.border,
        },
        ..Default::default()
    }
}

pub(super) fn telegram_chip_toggle_button(
    colors: TelegramColors,
    status: button::Status,
    active: bool,
) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let background = match (active, hovered) {
        (true, true) => Color {
            a: 0.18,
            ..colors.primary
        },
        (true, false) => Color {
            a: 0.1,
            ..colors.primary
        },
        (false, true) => Color {
            a: 0.06,
            ..colors.text
        },
        (false, false) => Color {
            a: 0.035,
            ..colors.text
        },
    };
    button::Style {
        background: Some(background.into()),
        text_color: if active {
            colors.orange_soft
        } else {
            colors.muted
        },
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: if active {
                colors.border_orange
            } else {
                colors.border
            },
        },
        ..Default::default()
    }
}

pub(super) fn telegram_link_button(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(Color::TRANSPARENT.into()),
        border: Border {
            radius: 3.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(super) fn telegram_status_chip_button(
    border_color: Color,
    status: button::Status,
) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(
            Color {
                a: if hovered { 0.08 } else { 0.03 },
                ..Color::WHITE
            }
            .into(),
        ),
        border: Border {
            radius: 3.0.into(),
            width: 1.0,
            color: border_color,
        },
        ..Default::default()
    }
}

pub(super) fn telegram_sunken_button(
    colors: TelegramColors,
    status: button::Status,
) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(
            if hovered {
                Color {
                    a: 0.04,
                    ..colors.text
                }
            } else {
                Color::TRANSPARENT
            }
            .into(),
        ),
        text_color: colors.text,
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(super) fn telegram_impact_chip_button(
    colors: TelegramColors,
    status: button::Status,
) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(colors.sunken.into()),
        text_color: colors.text,
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: if hovered {
                colors.border_orange
            } else {
                colors.border
            },
        },
        ..Default::default()
    }
}

pub(super) fn telegram_media_button(_theme: &Theme, status: button::Status) -> button::Style {
    let overlay = match status {
        button::Status::Hovered | button::Status::Pressed => Color {
            a: 0.06,
            ..Color::BLACK
        },
        _ => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(overlay.into()),
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(super) fn telegram_focus_input_style(
    theme: &Theme,
    status: text_input::Status,
) -> text_input::Style {
    let base = helpers::text_input_style(theme, status);
    match status {
        text_input::Status::Focused { .. } => text_input::Style {
            border: Border {
                color: theme.palette().primary,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..base
        },
        _ => base,
    }
}

pub(super) fn telegram_transparent_field_style(
    theme: &Theme,
    _status: text_input::Status,
) -> text_input::Style {
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        icon: theme.extended_palette().background.weak.text,
        placeholder: theme.extended_palette().background.weak.text,
        value: theme.palette().text,
        selection: theme.extended_palette().primary.weak.color,
    }
}

pub(super) fn telegram_transparent_input_style(
    _theme: &Theme,
    _status: text_input::Status,
) -> text_input::Style {
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        icon: Color::TRANSPARENT,
        placeholder: Color::TRANSPARENT,
        value: Color::TRANSPARENT,
        selection: Color::TRANSPARENT,
    }
}

pub(super) fn telegram_dot_style(color: Color) -> container_style::Style {
    container_style::Style {
        background: Some(color.into()),
        border: Border {
            radius: 999.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(super) fn telegram_pill_style(colors: TelegramColors) -> container_style::Style {
    container_style::Style {
        background: Some(
            Color {
                a: 0.025,
                ..colors.text
            }
            .into(),
        ),
        border: Border {
            radius: 999.0.into(),
            width: 1.0,
            color: colors.border,
        },
        ..Default::default()
    }
}

pub(super) fn telegram_add_pill_style(colors: TelegramColors) -> container_style::Style {
    container_style::Style {
        background: Some(Color::TRANSPARENT.into()),
        border: Border {
            radius: 999.0.into(),
            width: 1.0,
            color: Color {
                a: 0.2,
                ..colors.muted
            },
        },
        ..Default::default()
    }
}

pub(super) fn telegram_well_style(colors: TelegramColors) -> container_style::Style {
    container_style::Style {
        background: Some(colors.sunken.into()),
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: colors.border,
        },
        ..Default::default()
    }
}

pub(super) fn telegram_code_cell_style(
    colors: TelegramColors,
    active: bool,
) -> container_style::Style {
    container_style::Style {
        background: Some(colors.sunken.into()),
        border: Border {
            radius: 5.0.into(),
            width: 1.0,
            color: if active {
                colors.border_orange
            } else {
                colors.border
            },
        },
        ..Default::default()
    }
}

pub(super) fn telegram_icon_tile_style(colors: TelegramColors) -> container_style::Style {
    container_style::Style {
        background: Some(
            Color {
                a: 0.1,
                ..colors.primary
            }
            .into(),
        ),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: colors.border_orange,
        },
        ..Default::default()
    }
}

pub(super) fn telegram_info_card_style(
    colors: TelegramColors,
    accent: bool,
) -> container_style::Style {
    container_style::Style {
        background: Some(
            if accent {
                Color {
                    a: 0.05,
                    ..colors.primary
                }
            } else {
                colors.panel
            }
            .into(),
        ),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: if accent {
                colors.border_orange
            } else {
                colors.border
            },
        },
        ..Default::default()
    }
}

pub(super) fn telegram_sunken_outline_style(colors: TelegramColors) -> container_style::Style {
    container_style::Style {
        background: Some(colors.sunken.into()),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: colors.border,
        },
        ..Default::default()
    }
}

pub(super) fn telegram_rule_style(colors: TelegramColors) -> rule::Style {
    rule::Style {
        color: colors.border,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

pub(super) fn telegram_section_divider_style(colors: TelegramColors) -> container_style::Style {
    container_style::Style {
        background: Some(
            Color {
                a: 0.008,
                ..colors.text
            }
            .into(),
        ),
        border: Border {
            color: colors.border,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn telegram_media_placeholder_style(colors: TelegramColors) -> container_style::Style {
    container_style::Style {
        background: Some(colors.sunken.into()),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: Color {
                a: 0.18,
                ..colors.muted
            },
        },
        ..Default::default()
    }
}

pub(super) fn telegram_avatar_placeholder_style(colors: TelegramColors) -> container_style::Style {
    container_style::Style {
        background: Some(
            Color {
                a: 0.1,
                ..colors.muted
            }
            .into(),
        ),
        border: Border {
            radius: 999.0.into(),
            width: 1.0,
            color: Color {
                a: 0.22,
                ..colors.muted
            },
        },
        ..Default::default()
    }
}

pub(super) fn telegram_post_row_style(colors: TelegramColors, heat: f32) -> container_style::Style {
    let clamped = heat.clamp(0.0, 1.0);
    let background = blend_color(Color::TRANSPARENT, colors.primary, 0.06 * clamped);
    container_style::Style {
        background: Some(background.into()),
        border: Border {
            // Hairline divider between cards (bottom edge only is not expressible,
            // so a faint full border reads as a separator on the flat surface).
            color: colors.border,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}
