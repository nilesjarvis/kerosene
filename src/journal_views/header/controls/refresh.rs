use crate::journal_views::style::journal_pill_style;
use crate::message::Message;
use iced::Element;
use iced::widget::{button, text};

pub(super) fn journal_refresh_button(loading: bool) -> Element<'static, Message> {
    button(text(if loading { "Refreshing..." } else { "Refresh" }).size(11))
        .on_press(Message::JournalRefresh)
        .padding([3, 9])
        .style(journal_pill_style(loading))
        .into()
}
