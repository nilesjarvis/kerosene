use crate::message::Message;
use iced::widget::{button, text};
use iced::{Element, Theme};

pub(super) fn journal_refresh_button(loading: bool) -> Element<'static, Message> {
    button(text(if loading { "Refreshing..." } else { "Refresh" }).size(12))
        .on_press(Message::JournalRefresh)
        .padding([4, 12])
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}
