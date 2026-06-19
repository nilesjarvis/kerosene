use crate::journal_views::style::journal_control_style;
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
            .style(journal_control_style(loading)),
        text(label).size(10),
        tooltip::Position::Bottom,
    )
    .into()
}

pub(super) fn journal_clear_cache_button(enabled: bool) -> Element<'static, Message> {
    let mut clear_button = button(text("\u{232B}").size(14).center())
        .padding([3, 9])
        .style(journal_control_style(false));

    if enabled {
        clear_button = clear_button.on_press(Message::JournalClearCache);
    }

    tooltip(
        clear_button,
        text("Clear cache and reload full history").size(10),
        tooltip::Position::Bottom,
    )
    .into()
}
