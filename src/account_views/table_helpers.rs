use crate::message::Message;

use iced::widget::{column, container, rule, scrollable, text};
use iced::{Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Shared Account Tables
// ---------------------------------------------------------------------------

pub(in crate::account_views) fn empty_account_table<'a>(
    header: impl Into<Element<'a, Message>>,
    message: String,
    theme: &Theme,
) -> Element<'a, Message> {
    let weak_text = theme.extended_palette().background.weak.text;
    let content = column![
        header.into(),
        rule::horizontal(1),
        container(text(message).size(12).color(weak_text)).padding([8, 0]),
    ]
    .spacing(4);

    account_table_scroll(content)
}

#[inline]
pub(in crate::account_views) fn account_table_scroll<'a>(
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    scrollable(content)
        .direction(iced::widget::scrollable::Direction::Vertical(
            iced::widget::scrollable::Scrollbar::new()
                .width(4)
                .margin(0)
                .scroller_width(4),
        ))
        .width(Fill)
        .height(Fill)
        .into()
}
