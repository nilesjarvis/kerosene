use iced::widget::button;
use iced::widget::container as container_style;
use iced::{Color, Theme};

// ---------------------------------------------------------------------------
// Shared journal styling
//
// Centralizes the accent color, panel chrome, and the segmented "pill" button
// idiom so the header controls and the summary panels share one visual
// language.
// ---------------------------------------------------------------------------

/// Mint accent used across journal charts, panels, and controls.
pub(super) fn journal_accent_mint() -> Color {
    Color {
        r: 0.16,
        g: 0.94,
        b: 0.78,
        a: 1.0,
    }
}

/// Shared elevated panel chrome for the summary cards so stacked panels read as
/// siblings (matching border radius, padding via [`JOURNAL_PANEL_PADDING`], and
/// elevation).
pub(super) fn journal_panel_style(theme: &Theme) -> container_style::Style {
    let mut shadow_color = Color::BLACK;
    shadow_color.a = 0.24;

    container_style::Style {
        background: Some(
            Color {
                a: 0.94,
                ..theme.extended_palette().background.strong.color
            }
            .into(),
        ),
        border: iced::Border {
            color: Color {
                a: 0.14,
                ..journal_accent_mint()
            },
            width: 1.0,
            radius: JOURNAL_PANEL_RADIUS.into(),
        },
        shadow: iced::Shadow {
            color: shadow_color,
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 22.0,
        },
        ..Default::default()
    }
}

pub(super) const JOURNAL_PANEL_RADIUS: f32 = 14.0;
pub(super) const JOURNAL_PANEL_PADDING: [u16; 2] = [14, 16];

/// Shared segmented-control pill style. Used by the header sort/filter/refresh
/// controls and the in-panel timeframe selector so every mutually-exclusive
/// control in the journal reads the same way.
pub(super) fn journal_pill_style(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme: &Theme, status| {
        let mint = journal_accent_mint();
        let muted = theme.extended_palette().background.weak.text;
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let background = if active {
            Color { a: 0.16, ..mint }
        } else if hovered {
            Color { a: 0.08, ..mint }
        } else {
            Color {
                a: 0.025,
                ..theme.palette().text
            }
        };
        let border_color = if active || hovered {
            Color { a: 0.38, ..mint }
        } else {
            Color {
                a: 0.10,
                ..theme.palette().text
            }
        };

        button::Style {
            background: Some(background.into()),
            text_color: if active { mint } else { muted },
            border: iced::Border {
                radius: 999.0.into(),
                width: 1.0,
                color: border_color,
            },
            ..Default::default()
        }
    }
}
