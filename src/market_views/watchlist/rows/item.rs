use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
use crate::helpers::{self, category_color};
use crate::market_state::SymbolSearchSortMode;
use crate::message::Message;
use iced::widget::{Space, button, row, text};
use iced::{Color, Element, Fill, Theme, color};

impl TradingTerminal {
    pub(super) fn view_symbol_search_row<'a>(
        &'a self,
        sym: &'a ExchangeSymbol,
        is_fav: bool,
        active_sym: &str,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let display = sym.display_name.as_deref().unwrap_or(&sym.ticker);
        let is_selected = sym.key == active_sym;
        let exchange_label = Self::symbol_search_exchange_label(sym);
        let category_label = sym.category.to_uppercase();

        let star_key = sym.key.clone();
        let star_btn = button(text(if is_fav { "\u{2605}" } else { "\u{2606}" }).size(12))
            .on_press(Message::ToggleFavourite(star_key))
            .padding([0, 4])
            .style(move |theme: &Theme, status| {
                let text_color = if is_fav {
                    theme.palette().primary
                } else {
                    match status {
                        button::Status::Hovered => theme.palette().primary,
                        _ => color!(0x666666),
                    }
                };
                button::Style {
                    background: None,
                    text_color,
                    ..Default::default()
                }
            });

        let key = sym.key.clone();
        let mut identity = row![];
        if let Some(icon) = helpers::symbol_icon(&sym.key, 14, theme.palette().text) {
            identity = identity.push(icon).push(Space::new().width(4.0));
        }
        identity = identity
            .push(text(display).size(12).width(Fill))
            .width(Fill)
            .align_y(iced::Alignment::Center);

        let mut coin_content = row![
            identity,
            text(exchange_label)
                .size(9)
                .color(color!(0x666666))
                .width(iced::Length::Fixed(54.0)),
            text(category_label)
                .size(9)
                .color(category_color(&sym.category, theme))
                .width(iced::Length::Fixed(66.0)),
        ];

        if self.symbol_search_sort_mode == SymbolSearchSortMode::Volume24h {
            let volume_label = self
                .symbol_search_volume(sym)
                .map(Self::format_symbol_search_volume)
                .unwrap_or_else(|| "--".to_string());
            coin_content = coin_content.push(
                text(volume_label)
                    .size(10)
                    .color(theme.extended_palette().background.weak.text)
                    .font(crate::app_fonts::monospace_font())
                    .align_x(iced::alignment::Horizontal::Right)
                    .width(iced::Length::Fixed(74.0)),
            );
        }

        coin_content = coin_content.spacing(6).align_y(iced::Alignment::Center);

        let row_btn = button(coin_content)
            .on_press(Message::SymbolSelected(key))
            .padding([3, 6])
            .width(Fill)
            .style(move |theme: &Theme, status| {
                let bg = if is_selected {
                    match status {
                        button::Status::Hovered => theme.extended_palette().background.strong.color,
                        _ => color!(0x2a2a4a),
                    }
                } else {
                    match status {
                        button::Status::Hovered => theme.extended_palette().background.weak.color,
                        _ => theme.extended_palette().background.strong.color,
                    }
                };
                let text_color = theme.palette().text;
                let border_color = if is_selected {
                    Color {
                        a: 0.4,
                        ..theme.palette().primary
                    }
                } else {
                    Color::TRANSPARENT
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color,
                    border: iced::Border {
                        radius: 2.0.into(),
                        width: if is_selected { 1.0 } else { 0.0 },
                        color: border_color,
                    },
                    ..Default::default()
                }
            });

        row![star_btn, row_btn]
            .spacing(2)
            .align_y(iced::Alignment::Center)
            .into()
    }
}
