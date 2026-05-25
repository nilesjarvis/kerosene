use crate::message::Message;

use super::super::super::details::section_title;
use iced::widget::{column, text};
use iced::{Element, Theme};

// ---------------------------------------------------------------------------
// TWAP Operating Notes
// ---------------------------------------------------------------------------

pub(in crate::order_views::twap_details) fn twap_notes<'a>(theme: &Theme) -> Element<'a, Message> {
    let weak = theme.extended_palette().background.weak.text;
    column![
        section_title("Operating Notes", theme),
        text(concat!(
            "TWAP slices are bounded Limit IOC orders. They do not intentionally leave ",
            "resting child orders behind."
        ))
        .size(11)
        .color(weak),
        text(concat!(
            "A slice is skipped when the current book cannot fill the full planned size ",
            "inside the configured min/max range."
        ))
        .size(11)
        .color(weak),
        text(concat!(
            "Stale market data, rate limits, and unknown child status pause the TWAP ",
            "instead of silently burning future slices."
        ))
        .size(11)
        .color(weak),
        text(concat!(
            "Closing or switching charts does not affect the TWAP. Disconnecting or ",
            "changing wallets stops future slices."
        ))
        .size(11)
        .color(weak),
        text(concat!(
            "Live TWAPs do not resume after app restart. Completed/stopped history is ",
            "saved in Advanced Orders."
        ))
        .size(11)
        .color(weak),
    ]
    .spacing(5)
    .into()
}
