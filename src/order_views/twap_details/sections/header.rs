use crate::message::Message;
use crate::twap_state::TwapOrder;

use iced::widget::{row, text};
use iced::{Alignment, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// TWAP Header
// ---------------------------------------------------------------------------

pub(in crate::order_views::twap_details) fn twap_header<'a>(
    twap: &TwapOrder,
    theme: &Theme,
) -> Element<'a, Message> {
    let side_color = if twap.is_buy {
        theme.palette().success
    } else {
        theme.palette().danger
    };
    row![
        text(format!("TWAP #{}", twap.id)).size(16).width(Fill),
        text(twap.side_label()).size(12).color(side_color),
        text(twap.display_coin.clone()).size(13),
        text(twap.status.label())
            .size(12)
            .color(theme.palette().primary),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}
