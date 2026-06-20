use iced::font::Weight;
use iced::{Color, Font, Theme};

use crate::app_fonts::monospace_font;

// ---------------------------------------------------------------------------
// Portfolio Design Tokens
// ---------------------------------------------------------------------------
//
// The widget is mono-forward and fully theme-skinnable: every color is derived
// from the active iced theme so the pane reskins (accent -> mint, up -> mint,
// ...) with no code changes. Spec token names map to the helpers below.

// ---- Fonts (the widget uses mono exclusively) ----

pub(super) fn mono() -> Font {
    monospace_font()
}

pub(super) fn mono_semibold() -> Font {
    Font {
        weight: Weight::Semibold,
        ..monospace_font()
    }
}

// ---- Color tokens ----

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

/// `--text` — primary cream text.
pub(super) fn text(theme: &Theme) -> Color {
    theme.palette().text
}

/// `--muted` — secondary text. Derived as faded primary text so it mutes under
/// every theme (the background palette pairs all reuse full-strength text).
pub(super) fn muted(theme: &Theme) -> Color {
    with_alpha(theme.palette().text, 0.60)
}

/// `--dim` — tertiary text / uppercase labels.
pub(super) fn dim(theme: &Theme) -> Color {
    with_alpha(theme.palette().text, 0.40)
}

/// `--border` — hairline border.
pub(super) fn border(theme: &Theme) -> Color {
    with_alpha(theme.palette().text, 0.12)
}

/// `--orange` — flame accent (active toggles, Available value).
pub(super) fn accent(theme: &Theme) -> Color {
    theme.palette().primary
}

/// `--orange-soft` — tinted accent text on active segments.
pub(super) fn accent_soft(theme: &Theme) -> Color {
    theme.extended_palette().primary.strong.color
}

/// `--border-orange` — active / focus edge.
pub(super) fn accent_border(theme: &Theme) -> Color {
    with_alpha(theme.palette().primary, 0.34)
}

/// Active segment background wash.
pub(super) fn accent_wash(theme: &Theme) -> Color {
    with_alpha(theme.palette().primary, 0.10)
}

/// `--up` — gain.
pub(super) fn up(theme: &Theme) -> Color {
    theme.palette().success
}

/// `--down` — loss.
pub(super) fn down(theme: &Theme) -> Color {
    theme.palette().danger
}

/// `--up-wash` — gain fill / badge background.
pub(super) fn up_wash(theme: &Theme) -> Color {
    with_alpha(theme.palette().success, 0.14)
}

/// `--down-wash` — loss fill background.
pub(super) fn down_wash(theme: &Theme) -> Color {
    with_alpha(theme.palette().danger, 0.14)
}

/// `--panel-sunken` — recessed well behind segmented control tracks. The pane
/// body sits on `background.strong`; `background.base` is darker and reads as a
/// sunken inset under every theme.
pub(super) fn track(theme: &Theme) -> Color {
    theme.extended_palette().background.base.color
}

/// Sign coloring rule: `>= 0` is up, `< 0` is down, unknown is dim.
pub(super) fn pnl_color(theme: &Theme, value: Option<f64>) -> Color {
    match value {
        Some(value) if value >= 0.0 => up(theme),
        Some(_) => down(theme),
        None => dim(theme),
    }
}
