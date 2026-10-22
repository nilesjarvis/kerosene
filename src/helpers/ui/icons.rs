//! Shared monochrome vector action icons.
//!
//! Inline SVG assets drawn with `currentColor` and tinted through resolved
//! [`UiColors`] roles, per the design language's icon direction: one
//! consistent vector family for app actions instead of font-dependent
//! glyphs. Sizes are logical pixels at 100% UI scale.

use crate::helpers::design::UiColors;
use crate::message::Message;

use iced::widget::svg;
use iced::widget::svg::Handle as SvgHandle;
use iced::{Element, Length, Theme};

/// Settings "sliders" action.
pub(crate) const SETTINGS: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round">
  <path d="M4 8h16M4 16h16"/>
  <circle cx="9" cy="8" r="2.4" fill="currentColor" stroke="none"/>
  <circle cx="15" cy="16" r="2.4" fill="currentColor" stroke="none"/>
</svg>
"#;

/// Map pin: a symbol pinned to one market instead of tracking the active one.
pub(crate) const PIN: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round"
     stroke-linejoin="round">
  <path d="M12 21c-4-4.4-6-7.5-6-10a6 6 0 1 1 12 0c0 2.5-2 5.6-6 10z"/>
  <circle cx="12" cy="11" r="2.2" fill="currentColor" stroke="none"/>
</svg>
"#;

/// Chevron pointing up; pickers and open menus.
pub(crate) const CHEVRON_UP: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round"
     stroke-linejoin="round">
  <path d="m6 15 6-6 6 6"/>
</svg>
"#;

/// Chevron pointing down; pickers and closed menus.
pub(crate) const CHEVRON_DOWN: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round"
     stroke-linejoin="round">
  <path d="m6 9 6 6 6-6"/>
</svg>
"#;

/// Circular refresh action for reloading data.
pub(crate) const REFRESH: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round"
     stroke-linejoin="round">
  <path d="M21 12a9 9 0 1 1-2.64-6.36"/>
  <path d="M21 3v6h-6"/>
</svg>
"#;

/// Close / dismiss action.
pub(crate) const CLOSE: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round">
  <path d="M6 6l12 12M18 6L6 18"/>
</svg>
"#;

/// Warning state marker for inline notices.
pub(crate) const WARNING: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round"
     stroke-linejoin="round">
  <path d="M12 3 2.5 19.5h19z"/>
  <path d="M12 10v4"/>
  <path d="M12 17.4v.1"/>
</svg>
"#;
/// Solid status dot: the book is tracking the active (selected) symbol.
pub(crate) const ACTIVE_DOT: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <circle cx="12" cy="12" r="6" fill="currentColor"/>
</svg>
"#;

/// Semantic tint for an icon, resolved against the active theme.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum IconColor {
    /// Secondary text: low-emphasis utilities and ghost actions.
    Muted,
    /// Accent: active tracking, selection, or focus.
    Accent,
    /// Warning: stale, delayed, or attention-needed states.
    Warning,
}

/// Render `icon` at `size` logical pixels, tinted by `role`.
pub(crate) fn icon(icon: &'static [u8], size: f32, role: IconColor) -> Element<'static, Message> {
    svg(SvgHandle::from_memory(icon))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |theme: &Theme, _status| svg::Style {
            color: Some(role.resolve(theme)),
        })
        .into()
}

impl IconColor {
    fn resolve(self, theme: &Theme) -> iced::Color {
        let colors = UiColors::new(theme);
        match self {
            IconColor::Muted => colors.muted,
            IconColor::Accent => colors.accent,
            IconColor::Warning => colors.warning,
        }
    }
}
