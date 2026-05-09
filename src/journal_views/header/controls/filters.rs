use crate::helpers;
use crate::journal;
use crate::message::Message;
use iced::widget::{button, row, text};
use iced::{Element, Theme};

pub(super) fn journal_filter_controls(
    active_filter: journal::JournalFilter,
) -> Element<'static, Message> {
    let mut filter_row = row![].spacing(8);
    for filter in [
        (journal::JournalFilter::All, "All"),
        (journal::JournalFilter::Perp, "Perp"),
        (journal::JournalFilter::Spot, "Spot"),
    ] {
        let is_active = active_filter == filter.0;
        filter_row = filter_row.push(
            button(text(filter.1).size(12))
                .on_press(Message::JournalFilterChanged(filter.0))
                .padding([4, 12])
                .style(move |theme: &Theme, status| {
                    let mut style = if is_active {
                        button::primary(theme, status)
                    } else {
                        button::secondary(theme, status)
                    };
                    let bg = if is_active {
                        theme.palette().primary
                    } else {
                        theme.extended_palette().background.weak.color
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
    filter_row.into()
}
