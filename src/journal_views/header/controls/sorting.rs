use crate::journal;
use crate::journal_views::style::journal_pill_style;
use crate::message::Message;
use iced::widget::{Space, button, row, text, tooltip};
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
        (
            journal::JournalSort::TimeDesc,
            "\u{23F2} \u{2193}",
            "Newest",
        ),
        (journal::JournalSort::TimeAsc, "\u{23F2} \u{2191}", "Oldest"),
        (
            journal::JournalSort::PnlDesc,
            "\u{0024} \u{2193}",
            "PnL (High)",
        ),
        (
            journal::JournalSort::PnlAsc,
            "\u{0024} \u{2191}",
            "PnL (Low)",
        ),
    ] {
        let is_active = active_sort == sort_opt.0;
        let btn = button(text(sort_opt.1).size(11))
            .on_press(Message::JournalSortChanged(sort_opt.0))
            .padding([3, 9])
            .style(journal_pill_style(is_active));

        sort_row = sort_row.push(tooltip(
            btn,
            text(sort_opt.2).size(10),
            tooltip::Position::Bottom,
        ));
    }

    sort_row.into()
}
