mod button;
mod menu;

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::row;
use iced::{Element, Theme};

impl TradingTerminal {
    pub(super) fn view_position_close_cell(
        &self,
        coin: String,
        can_close: bool,
        is_hidden: bool,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let hide_button = button::view_position_hide_button(coin.clone(), is_hidden, theme);
        let can_close_position = can_close && self.is_perp_coin(&coin);
        if !can_close_position {
            return hide_button;
        }

        let coin_for_close = coin.clone();
        if self.close_menu_coin.as_deref() == Some(coin.as_str()) {
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
