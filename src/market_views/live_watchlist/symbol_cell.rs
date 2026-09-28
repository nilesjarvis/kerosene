use crate::helpers;
use crate::message::Message;
use iced::widget::{Row, Space, row, text};
use iced::{Fill, Theme};

pub(super) fn view_live_watchlist_symbol_cell<'a>(
    sym_key: &str,
    display: &'a str,
    growth_mode: bool,
    theme: &Theme,
) -> Row<'a, Message> {
    let mut coin_content = row![];
    if let Some(icon) = helpers::symbol_icon(sym_key, 14, theme.palette().text) {
        coin_content = coin_content.push(icon).push(Space::new().width(4.0));
    }
    coin_content = coin_content.push(
        text(display)
            .size(12)
            .color(theme.palette().text)
            .width(Fill),
    );
    if growth_mode {
        coin_content = coin_content.push(helpers::growth_mode_chip());
    }
    coin_content
}
