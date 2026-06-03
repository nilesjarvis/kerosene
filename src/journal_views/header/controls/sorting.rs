use crate::journal;
use crate::journal_views::style::journal_pill_style;
use crate::message::Message;
use iced::widget::{Space, button, row, text};
use iced::{Element, Theme};

pub(super) fn journal_sort_controls(
    active_sort: journal::JournalSort,
) -> Element<'static, Message> {
    let mut sort_row = row![
        text("Sort:").size(11).style(|theme: &Theme| text::Style {
            color: Some(theme.extended_palette().background.weak.text),
        }),
        Space::new().width(4.0)
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center);

    for sort_opt in [
        (journal::JournalSort::TimeDesc, "Newest"),
        (journal::JournalSort::TimeAsc, "Oldest"),
        (journal::JournalSort::PnlDesc, "PnL (High)"),
        (journal::JournalSort::PnlAsc, "PnL (Low)"),
    ] {
        let is_active = active_sort == sort_opt.0;
        sort_row = sort_row.push(
            button(text(sort_opt.1).size(11))
                .on_press(Message::JournalSortChanged(sort_opt.0))
                .padding([3, 9])
                .style(journal_pill_style(is_active)),
        );
    }

    sort_row.into()
}
