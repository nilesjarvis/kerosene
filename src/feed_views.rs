mod controls;
mod liquidations;
mod summary;
mod telegram;
mod tracked_trades;
mod x;

use crate::message::Message;

use iced::widget::{container, text};
use iced::{Element, Fill, Theme};

pub(crate) use summary::{FEED_SUMMARY_TIMEFRAMES, view_feed_summary};

// ---------------------------------------------------------------------------
// Shared Feed Components
// ---------------------------------------------------------------------------

fn feed_empty_state(theme: &Theme, message: &'static str) -> Element<'static, Message> {
    container(text(message).color(theme.palette().text))
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill)
        .into()
}
