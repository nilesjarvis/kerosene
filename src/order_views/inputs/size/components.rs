use crate::helpers::design::compact_button_style;
use crate::message::Message;

use iced::Theme;
use iced::widget::{button, text};

// ---------------------------------------------------------------------------
// Size Input Components
// ---------------------------------------------------------------------------

/// Denomination switch: compact ghost control sharing the size header row.
pub(super) fn denomination_button<'a>(
    label: impl ToString,
    pane_corner_radius: f32,
) -> button::Button<'a, Message> {
    button(text(label.to_string()).size(12).center())
        .on_press(Message::ToggleOrderDenomination)
        .height(24)
        .padding([0, 8])
        .style(move |theme: &Theme, status| {
            compact_button_style(theme, status, false, pane_corner_radius)
        })
}
