use crate::account_views::table_helpers::account_table_scroll;
use crate::message::Message;

use iced::widget::{Column, column, container, row, rule, text};
use iced::{Element, Fill, Theme};

pub(super) fn empty_balances_table(msg: &'static str, theme: &Theme) -> Element<'static, Message> {
    let content = column![
        balances_header(),
        rule::horizontal(1),
        container(
            text(msg)
                .size(12)
                .color(theme.extended_palette().background.weak.text)
        )
        .padding([8, 0]),
    ]
    .spacing(4);

    account_table_scroll(content)
}

pub(super) fn balances_rows_table(rows: Column<'static, Message>) -> Element<'static, Message> {
    account_table_scroll(column![balances_header(), rule::horizontal(1), rows].spacing(4))
}

fn balances_header() -> Element<'static, Message> {
    row![
        text("Asset").size(12).width(Fill),
        text("Total").size(12).width(Fill),
        text("Hold").size(12).width(Fill),
        text("Available").size(12).width(Fill),
        text("Entry Ntl").size(12).width(Fill),
        text("").size(12).width(60),
    ]
    .spacing(4)
    .into()
}
