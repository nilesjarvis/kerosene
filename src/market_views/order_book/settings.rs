#[path = "settings/spread_chart.rs"]
mod spread_chart;
#[path = "settings/symbol_mode.rs"]
mod symbol_mode;

use crate::app_state::TradingTerminal;
use crate::market_state::{OrderBookId, OrderBookInstance};
use crate::message::Message;
use iced::widget::{Space, column, container};
use iced::{Element, Theme};

impl TradingTerminal {
    pub(super) fn view_order_book_settings<'a>(
        &'a self,
        id: OrderBookId,
        inst: &'a OrderBookInstance,
    ) -> Element<'a, Message> {
        let search_col = self.view_order_book_symbol_mode_controls(id, inst);
        let show_chart_btn = spread_chart::view_order_book_spread_toggle(id, inst);

        container(column![
            search_col,
            Space::new().height(10.0),
            show_chart_btn
        ])
        .padding(8)
        .style(move |theme: &Theme| container::Style {
            background: Some(theme.extended_palette().background.weak.color.into()),
            border: iced::border::rounded(4),
            ..Default::default()
        })
        .into()
    }
}
