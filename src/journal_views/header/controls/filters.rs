use crate::journal;
use crate::journal_views::style::journal_pill_style;
use crate::message::Message;
use iced::Element;
use iced::widget::{button, row, text};

pub(super) fn journal_filter_controls(
    active_filter: journal::JournalFilter,
) -> Element<'static, Message> {
    let mut filter_row = row![].spacing(4);
    for filter in [
        (journal::JournalFilter::All, "All"),
        (journal::JournalFilter::Perp, "Perp"),
        (journal::JournalFilter::Spot, "Spot"),
    ] {
        let is_active = active_filter == filter.0;
        filter_row = filter_row.push(
            button(text(filter.1).size(11))
                .on_press(Message::JournalFilterChanged(filter.0))
                .padding([3, 9])
                .style(journal_pill_style(is_active)),
        );
    }
    filter_row.into()
}
