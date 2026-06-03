use crate::journal_views::style::journal_pill_style;
use crate::message::Message;
use iced::Element;
use iced::widget::{button, text, tooltip};

pub(super) fn journal_refresh_button(loading: bool) -> Element<'static, Message> {
    let icon = if loading { "\u{23F3}" } else { "\u{21BB}" };
    let label = if loading { "Refreshing..." } else { "Refresh" };

    tooltip(
        button(text(icon).size(14).center())
            .on_press(Message::JournalRefresh)
            .padding([3, 9])
            .style(journal_pill_style(loading)),
        text(label).size(10),
        tooltip::Position::Bottom,
    )
    .into()
}
