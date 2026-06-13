mod liquidations;
mod telegram;
mod tracked_trades;

use crate::message::Message;

use iced::widget::{container, text};
use iced::{Element, Fill, Theme};

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
