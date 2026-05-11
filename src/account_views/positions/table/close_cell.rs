mod button;
mod menu;

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::row;
use iced::{Element, Theme};

impl TradingTerminal {
    pub(super) fn view_position_close_cell<'a>(
        &'a self,
        coin: &'a str,
        can_close: bool,
        is_hidden: bool,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let hide_button = button::view_position_hide_button(coin.to_string(), is_hidden, theme);
        let can_close_position = can_close && !self.is_outcome_coin(coin);
        if !can_close_position {
            return hide_button;
        }

        let coin_for_close = coin.to_string();
        if self.close_menu_coin.as_deref() == Some(coin) {
            return row![
                hide_button,
                menu::view_position_close_menu(coin_for_close, theme)
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center)
            .into();
        }

        row![
            hide_button,
            button::view_position_close_button(coin_for_close, theme)
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center)
        .into()
    }
}
