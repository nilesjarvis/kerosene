use iced::widget::container as container_style;
use iced::widget::{button, text_input};
use iced::{Color, Theme};

// ---------------------------------------------------------------------------
// Shared journal styling
//
// Centralizes journal chrome so the standalone window follows the same flat,
// square-edged theme language as the rest of the terminal.
// ---------------------------------------------------------------------------

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

/// Shared square compact-control style.
pub(super) fn journal_control_style(
    active: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme: &Theme, status| {
        let palette = theme.palette();
        let extended = theme.extended_palette();
        let muted = theme.extended_palette().background.weak.text;
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let background = if active {
            Color {
                a: 0.14,
                ..palette.primary
            }
        } else if hovered {
            Color {
                a: 0.55,
                ..extended.background.strong.color
            }
        } else {
            Color::TRANSPARENT
        };
        let border_color = if active || hovered {
            Color {
                a: if active { 0.45 } else { 0.18 },
                ..palette.primary
            }
        } else {
            Color {
                a: 0.12,
                ..palette.text
            }
        };

        button::Style {
            background: Some(background.into()),
            text_color: if active { palette.primary } else { muted },
            border: iced::Border {
                radius: 0.0.into(),
                width: 1.0,
                color: border_color,
            },
            ..Default::default()
        }
    }
}

pub(super) fn journal_text_input_style(
    theme: &Theme,
    status: text_input::Status,
) -> text_input::Style {
    let mut style = crate::helpers::text_input_style(theme, status);
    style.border.radius = 0.0.into();
    style
}
