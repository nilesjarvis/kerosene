use iced::widget::container as container_style;
use iced::widget::{button, rule, text_input};
use iced::{Border, Color, Theme};

/// Hairline rule styling (used for section dividers and KPI/cell separators).
pub(super) fn journal_rule_style(theme: &Theme) -> rule::Style {
    rule::Style {
        color: journal_hairline(theme),
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

// ---------------------------------------------------------------------------
// Shared journal styling
//
// Centralizes journal chrome so the redesigned master-detail window follows a
// single visual system: a near-black workspace lit by the theme accent, drawn
// with hairline rules and small radii rather than heavy containers.
// ---------------------------------------------------------------------------

// Geometry (3px chips, 4px buttons, 5px primary buttons, 6px cards/panels).
pub(super) const JOURNAL_CHIP_RADIUS: f32 = 3.0;
pub(super) const JOURNAL_BUTTON_RADIUS: f32 = 4.0;
pub(super) const JOURNAL_PRIMARY_RADIUS: f32 = 5.0;
pub(super) const JOURNAL_CARD_RADIUS: f32 = 6.0;

// ---- Palette helpers ----

/// Hairline rule / border tint (light at ~10% alpha).
pub(super) fn journal_hairline(theme: &Theme) -> Color {
    Color {
        a: 0.10,
        ..theme.palette().text
    }
}

/// Orange focus tint for selected-row bars and segment outlines.
pub(super) fn journal_accent_focus(theme: &Theme) -> Color {
    Color {
        a: 0.34,
        ..theme.palette().primary
    }
}

/// Soft, lifted variant of the accent for active segment text.
pub(super) fn journal_accent_soft(theme: &Theme) -> Color {
    lighten(theme.palette().primary, 0.34)
}

/// Primary panel surface that sits above the page background.
pub(super) fn journal_surface_panel(theme: &Theme) -> Color {
    theme.extended_palette().background.weak.color
}

/// Raised surface used for hover and monograms.
pub(super) fn journal_surface_raised(theme: &Theme) -> Color {
    theme.extended_palette().background.strong.color
}

/// Sunken well, slightly darker than the page background.
pub(super) fn journal_surface_sunken(theme: &Theme) -> Color {
    darken(theme.palette().background, 0.35)
}

/// Muted label text.
pub(super) fn journal_muted(theme: &Theme) -> Color {
    theme.extended_palette().background.weak.text
}

/// Dimmer text for sub-lines.
pub(super) fn journal_dim(theme: &Theme) -> Color {
    Color {
        a: 0.68,
        ..journal_muted(theme)
    }
}

fn lighten(color: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color {
        r: color.r + (1.0 - color.r) * t,
        g: color.g + (1.0 - color.g) * t,
        b: color.b + (1.0 - color.b) * t,
        a: color.a,
    }
}

fn darken(color: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color {
        r: color.r * (1.0 - t),
        g: color.g * (1.0 - t),
        b: color.b * (1.0 - t),
        a: color.a,
    }
}

/// Shared flat panel chrome for summary blocks.
pub(super) fn journal_panel_style(theme: &Theme) -> container_style::Style {
    let mut border_color = theme.extended_palette().background.strong.text;
    border_color.a = 0.10;

    container_style::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: iced::Border {
            color: border_color,
            width: 1.0,
            radius: JOURNAL_PANEL_RADIUS.into(),
        },
        ..Default::default()
    }
}

pub(super) const JOURNAL_PANEL_RADIUS: f32 = 0.0;
pub(super) const JOURNAL_PANEL_PADDING: [u16; 2] = [12, 12];

pub(super) fn journal_text_input_style(
    theme: &Theme,
    status: text_input::Status,
) -> text_input::Style {
    let mut style = crate::helpers::text_input_style(theme, status);
    style.border.radius = JOURNAL_BUTTON_RADIUS.into();
    style
}

// ---- Container surfaces ----

/// Full-bleed window background for the journal root.
pub(super) fn journal_window_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(theme.palette().background.into()),
        ..Default::default()
    }
}

/// Rounded, hairline-bordered card used for cockpit and detail panels.
pub(super) fn journal_card_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(journal_surface_panel(theme).into()),
        border: Border {
            color: journal_hairline(theme),
            width: 1.0,
            radius: JOURNAL_CARD_RADIUS.into(),
        },
        ..Default::default()
    }
}

/// Status/side chip: tinted fill + tinted border, small radius.
pub(super) fn journal_chip_style(tint: Color) -> impl Fn(&Theme) -> container_style::Style {
    move |_theme: &Theme| container_style::Style {
        background: Some(Color { a: 0.14, ..tint }.into()),
        text_color: Some(tint),
        border: Border {
            color: Color { a: 0.40, ..tint },
            width: 1.0,
            radius: JOURNAL_CHIP_RADIUS.into(),
        },
        ..Default::default()
    }
}

/// 30px monogram badge surface (neutral raised fill).
pub(super) fn journal_monogram_style(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(journal_surface_raised(theme).into()),
        text_color: Some(journal_muted(theme)),
        border: Border {
            color: journal_hairline(theme),
            width: 1.0,
            radius: JOURNAL_CHIP_RADIUS.into(),
        },
        ..Default::default()
    }
}

// ---- Buttons ----

/// Segmented toolbar/timeframe control: orange outline + faint fill when active.
pub(super) fn journal_segment_style(
    active: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme: &Theme, status| {
        let palette = theme.palette();
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let background = if active {
            Color {
                a: 0.14,
                ..palette.primary
            }
        } else if hovered {
            Color {
                a: 0.06,
                ..palette.text
            }
        } else {
            Color::TRANSPARENT
        };
        let border_color = if active {
            journal_accent_focus(theme)
        } else if hovered {
            Color {
                a: 0.18,
                ..palette.text
            }
        } else {
            Color {
                a: 0.12,
                ..palette.text
            }
        };

        button::Style {
            background: Some(background.into()),
            text_color: if active {
                journal_accent_soft(theme)
            } else {
                journal_muted(theme)
            },
            border: Border {
                radius: JOURNAL_BUTTON_RADIUS.into(),
                width: 1.0,
                color: border_color,
            },
            ..Default::default()
        }
    }
}

/// Filled accent button (Save reflection): orange fill, ink-on-orange text.
pub(super) fn journal_primary_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let background = if hovered {
        lighten(palette.primary, 0.10)
    } else {
        palette.primary
    };

    button::Style {
        background: Some(background.into()),
        text_color: palette.background,
        border: Border {
            radius: JOURNAL_PRIMARY_RADIUS.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        ..Default::default()
    }
}

/// Quiet ghost button for the detail-pane "← Overview" control.
pub(super) fn journal_ghost_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(
            if hovered {
                Color {
                    a: 0.08,
                    ..palette.text
                }
            } else {
                Color::TRANSPARENT
            }
            .into(),
        ),
        text_color: journal_muted(theme),
        border: Border {
            radius: JOURNAL_BUTTON_RADIUS.into(),
            width: 1.0,
            color: journal_hairline(theme),
        },
        ..Default::default()
    }
}
