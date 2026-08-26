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

pub(crate) fn mono() -> Font {
    monospace_font()
}

pub(crate) fn mono_semibold() -> Font {
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
pub(crate) fn text(theme: &Theme) -> Color {
    theme.palette().text
}

/// `--muted` — secondary text. Derived as faded primary text so it mutes under
/// every theme (the background palette pairs all reuse full-strength text).
pub(crate) fn muted(theme: &Theme) -> Color {
    with_alpha(theme.palette().text, 0.60)
}

/// `--dim` — tertiary text / uppercase labels.
pub(crate) fn dim(theme: &Theme) -> Color {
    with_alpha(theme.palette().text, 0.40)
}

/// `--border` — hairline border.
pub(crate) fn border(theme: &Theme) -> Color {
    with_alpha(theme.palette().text, 0.12)
}

/// `--orange` — flame accent (active toggles, Available value).
pub(crate) fn accent(theme: &Theme) -> Color {
    theme.palette().primary
}

/// `--orange-soft` — tinted accent text on active segments.
pub(crate) fn accent_soft(theme: &Theme) -> Color {
    theme.extended_palette().primary.strong.color
}

/// `--border-orange` — active / focus edge.
pub(crate) fn accent_border(theme: &Theme) -> Color {
    with_alpha(theme.palette().primary, 0.34)
}

/// Active segment background wash.
pub(crate) fn accent_wash(theme: &Theme) -> Color {
    with_alpha(theme.palette().primary, 0.10)
}

/// `--up` — gain.
pub(crate) fn up(theme: &Theme) -> Color {
    theme.palette().success
}

/// `--down` — loss.
pub(crate) fn down(theme: &Theme) -> Color {
    theme.palette().danger
}

/// `--up-wash` — gain fill / badge background.
pub(crate) fn up_wash(theme: &Theme) -> Color {
    with_alpha(theme.palette().success, 0.14)
}

/// `--down-wash` — loss fill background.
pub(crate) fn down_wash(theme: &Theme) -> Color {
    with_alpha(theme.palette().danger, 0.14)
}

/// `--panel-sunken` — recessed well behind segmented control tracks. The pane
/// body sits on `background.strong`; `background.base` is darker and reads as a
/// sunken inset under every theme.
pub(crate) fn track(theme: &Theme) -> Color {
    theme.extended_palette().background.base.color
}

/// Sign coloring rule: `>= 0` is up, `< 0` is down, unknown is dim.
pub(crate) fn pnl_color(theme: &Theme, value: Option<f64>) -> Color {
    match value {
        Some(value) if value >= 0.0 => up(theme),
        Some(_) => down(theme),
        None => dim(theme),
    }
}
