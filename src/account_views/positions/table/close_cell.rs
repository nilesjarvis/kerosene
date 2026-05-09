mod button;
mod menu;

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::text;
use iced::{Element, Theme};

impl TradingTerminal {
    pub(super) fn view_position_close_cell<'a>(
        &'a self,
        coin: &'a str,
        can_close: bool,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let can_close_position = can_close && !self.is_outcome_coin(coin);
        if !can_close_position {
            return text("").size(12).into();
        }

        let coin_for_close = coin.to_string();
        if self.close_menu_coin.as_deref() == Some(coin) {
            return menu::view_position_close_menu(coin_for_close, theme);
        }

        button::view_position_close_button(coin_for_close, theme)
    }
}
