use crate::message::Message;
use iced::color;
use iced::widget::{Row, row, text};

pub(super) fn income_token_table_header() -> Row<'static, Message> {
    row![
        text("Token").size(10).color(color!(0x7f8ab0)).width(120),
        text("Supply").size(10).color(color!(0x7f8ab0)).width(90),
        text("S APR").size(10).color(color!(0x7f8ab0)).width(56),
        text("Borrow").size(10).color(color!(0x7f8ab0)).width(90),
        text("Net / Y").size(10).color(color!(0x7f8ab0)),
    ]
    .spacing(8)
}

pub(super) fn income_hourly_table_header() -> Row<'static, Message> {
    row![
        text("Time").size(10).color(color!(0x7f8ab0)).width(92),
        text("Token").size(10).color(color!(0x7f8ab0)).width(70),
        text("Supply").size(10).color(color!(0x7f8ab0)).width(84),
        text("Borrow").size(10).color(color!(0x7f8ab0)).width(84),
        text("Net").size(10).color(color!(0x7f8ab0)),
    ]
    .spacing(6)
}
