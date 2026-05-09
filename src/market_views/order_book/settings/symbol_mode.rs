use crate::app_state::TradingTerminal;
use crate::market_state::{OrderBookId, OrderBookInstance, OrderBookSymbolMode};
use crate::message::Message;
use iced::widget::{Column, button, column, text, text_input};
use iced::{Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_order_book_symbol_mode_controls<'a>(
        &'a self,
        id: OrderBookId,
        inst: &'a OrderBookInstance,
    ) -> Column<'a, Message> {
        let active_btn = button(text("Track Active Symbol").size(12).center().width(Fill))
            .on_press(Message::OrderBookSetMode(id, OrderBookSymbolMode::Active))
            .style(move |theme: &Theme, status| {
                let is_active = matches!(inst.mode, OrderBookSymbolMode::Active);
                let bg = if is_active {
                    theme.extended_palette().background.strong.color
                } else {
                    match status {
                        button::Status::Hovered => theme.extended_palette().background.strong.color,
                        _ => iced::Color::TRANSPARENT,
                    }
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: theme.palette().text,
                    ..Default::default()
                }
            });

        let mut search_col = column![
            active_btn,
            text_input("Search fixed asset...", &inst.search_query)
                .on_input(move |q| Message::OrderBookSearchChanged(id, q))
                .padding(5)
                .size(12)
        ]
        .spacing(4);

        let query_lower = inst.search_query.to_lowercase();
        let mut matches = 0;
        for sym in &self.exchange_symbols {
            if self.exchange_symbol_is_muted(sym) {
                continue;
            }
            if matches >= 5 {
                break;
            }
            if sym
                .display_name
                .as_deref()
                .unwrap_or(&sym.key)
                .to_lowercase()
                .contains(&query_lower)
                || sym.key.to_lowercase().contains(&query_lower)
            {
                let s_key = sym.key.clone();
                let mode = OrderBookSymbolMode::Fixed(s_key.clone());
                let btn = button(
                    text(sym.display_name.clone().unwrap_or(sym.key.clone()))
                        .size(12)
                        .width(Fill),
                )
                .on_press(Message::OrderBookSetMode(id, mode.clone()))
                .style(move |theme: &Theme, status| {
                    let is_active =
                        matches!(&inst.mode, OrderBookSymbolMode::Fixed(s) if s == &s_key);
                    let bg = if is_active {
                        theme.extended_palette().background.strong.color
                    } else {
                        match status {
                            button::Status::Hovered => {
                                theme.extended_palette().background.strong.color
                            }
                            _ => iced::Color::TRANSPARENT,
                        }
                    };
                    button::Style {
                        background: Some(bg.into()),
                        text_color: theme.palette().text,
                        ..Default::default()
                    }
                });
                search_col = search_col.push(btn);
                matches += 1;
            }
        }

        search_col
    }
}
