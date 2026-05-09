use crate::message::Message;

use iced::widget::{container, text};
use iced::{Element, Fill, Theme};

pub(super) fn dropdown_title(theme: &Theme) -> Element<'static, Message> {
    container(text("Accounts").size(12).color(theme.palette().text))
        .padding([6, 8])
        .width(Fill)
        .into()
}

pub(super) fn section_label(label: &'static str, theme: &Theme) -> Element<'static, Message> {
    container(
        text(label)
            .size(10)
            .color(theme.extended_palette().background.weak.text)
            .width(Fill),
    )
    .padding([5, 8])
    .width(Fill)
    .into()
}

pub(super) fn empty_saved_profiles(theme: &Theme) -> Element<'static, Message> {
    container(
        text("No saved profiles")
            .size(11)
            .color(theme.extended_palette().background.weak.text),
    )
    .padding([5, 8])
    .width(Fill)
    .into()
}
