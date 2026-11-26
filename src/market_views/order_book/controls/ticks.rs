use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::market_state::{OrderBookId, OrderBookInstance};
use crate::message::Message;
use iced::Element;
use iced::widget::{pick_list, text, tooltip};
use std::fmt;

#[derive(Clone, Copy, PartialEq)]
struct TickSize(f64);

impl fmt::Display for TickSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&helpers::format_tick(self.0))
    }
}

impl TradingTerminal {
    pub(in crate::market_views::order_book) fn resolved_order_book_tick(
        inst: &OrderBookInstance,
        tick_options: &[f64],
    ) -> f64 {
        if tick_options.contains(&inst.tick_size) {
            inst.tick_size
        } else {
            helpers::nearest_tick_option(tick_options, inst.tick_size)
        }
    }

    pub(in crate::market_views::order_book) fn view_order_book_tick_picker(
        id: OrderBookId,
        tick_options: [f64; 5],
        selected: f64,
    ) -> Element<'static, Message> {
        let picker = pick_list(
            tick_options.map(TickSize),
            Some(TickSize(selected)),
            move |tick| Message::SetBookTickSize(id, tick.0),
        )
        .text_size(10)
        .padding([2, 6]);

        tooltip(
            picker,
            text("Price denomination").size(10),
            tooltip::Position::Top,
        )
        .into()
    }
}
