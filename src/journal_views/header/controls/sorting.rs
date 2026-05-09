use crate::helpers;
use crate::journal;
use crate::message::Message;
use iced::widget::{Space, button, row, text};
use iced::{Color, Element, Theme};

pub(super) fn journal_sort_controls(
    active_sort: journal::JournalSort,
) -> Element<'static, Message> {
    let mut sort_row =
        row![text("Sort:").size(12), Space::new().width(4.0)].align_y(iced::Alignment::Center);

    for sort_opt in [
        (journal::JournalSort::TimeDesc, "Newest"),
        (journal::JournalSort::TimeAsc, "Oldest"),
        (journal::JournalSort::PnlDesc, "PnL (High)"),
        (journal::JournalSort::PnlAsc, "PnL (Low)"),
    ] {
        let is_active = active_sort == sort_opt.0;
        sort_row = sort_row.push(
            button(text(sort_opt.1).size(12))
                .on_press(Message::JournalSortChanged(sort_opt.0))
                .padding([4, 8])
                .style(move |theme: &Theme, status| {
                    let mut style = if is_active {
                        button::primary(theme, status)
                    } else {
                        button::secondary(theme, status)
                    };
                    let bg = if is_active {
                        theme.palette().primary
                    } else {
                        Color::TRANSPARENT
                    };
                    style.background = Some(bg.into());
                    style.text_color = if is_active {
                        helpers::text_color_for_bg(bg)
                    } else {
                        theme.palette().text
                    };
                    style
                }),
        );
    }

    sort_row.into()
}
