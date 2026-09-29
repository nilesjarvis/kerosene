use crate::app_state::TradingTerminal;
use crate::market_state::{OrderBookId, OrderBookInstance, OrderBookSymbolMode};
use crate::message::Message;
use iced::widget::{Column, button, column, container, text, text_input};
use iced::{Fill, Theme};

const SYMBOL_RESULT_ROW_HEIGHT: f32 = 24.0;
const SYMBOL_RESULT_ROWS: usize = 5;
// Reserve the full five-row area regardless of how many results match, so
// the panel (and the live book below it) never resizes while typing.
const SYMBOL_RESULTS_AREA_HEIGHT: f32 =
    SYMBOL_RESULT_ROWS as f32 * SYMBOL_RESULT_ROW_HEIGHT + (SYMBOL_RESULT_ROWS as f32 - 1.0) * 4.0;

impl TradingTerminal {
    pub(super) fn view_order_book_symbol_mode_controls<'a>(
        &'a self,
        id: OrderBookId,
        inst: &'a OrderBookInstance,
    ) -> Column<'a, Message> {
        let active_btn = button(text("Track Active Symbol").size(12).center().width(Fill))
            .on_press(Message::OrderBookSetMode(id, OrderBookSymbolMode::Active))
            .style(symbol_mode_button_style(matches!(
                inst.mode,
                OrderBookSymbolMode::Active
            )));

        let search_col = column![
            active_btn,
            text_input("Search fixed asset...", &inst.search_query)
                .on_input(move |q| Message::OrderBookSearchChanged(id, q))
                .padding(5)
                .size(12)
        ]
        .spacing(4);

        let mut results = Column::new().spacing(4);
        let query_lower = inst.search_query.to_lowercase();
        let mut matches = 0;
        for sym in &self.exchange_symbols {
            if !sym.is_user_selectable_market() || self.exchange_symbol_is_hidden(sym) {
                continue;
            }
            if matches >= SYMBOL_RESULT_ROWS {
                break;
            }
            let display = Self::exchange_symbol_display_name(sym);
            if display.to_lowercase().contains(&query_lower)
                || sym.key.to_lowercase().contains(&query_lower)
            {
                let is_active =
                    matches!(&inst.mode, OrderBookSymbolMode::Fixed(key) if key == &sym.key);
                let btn = button(text(display).size(12).width(Fill))
                    .height(SYMBOL_RESULT_ROW_HEIGHT)
                    .on_press(Message::OrderBookSetMode(
                        id,
                        OrderBookSymbolMode::Fixed(sym.key.clone()),
                    ))
                    .style(symbol_mode_button_style(is_active));
                results = results.push(btn);
                matches += 1;
            }
        }

        search_col.push(
            container(results)
                .width(Fill)
                .height(SYMBOL_RESULTS_AREA_HEIGHT),
        )
    }
}

fn symbol_mode_button_style(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let bg = if active || matches!(status, button::Status::Hovered) {
            theme.extended_palette().background.strong.color
        } else {
            iced::Color::TRANSPARENT
        };
        button::Style {
            background: Some(bg.into()),
            text_color: theme.palette().text,
            ..Default::default()
        }
    }
}
