use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
use crate::market_state::SymbolSearchSortMode;
use crate::message::Message;
use iced::Theme;
use iced::widget::{Column, container, rule, text};

mod filtering;
mod item;

// ---------------------------------------------------------------------------
// Symbol Search Rows
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_symbol_search_rows<'a>(
        &'a self,
        filtered: &[&'a ExchangeSymbol],
        theme: &Theme,
    ) -> Column<'a, Message> {
        let favs = &self.favourite_symbols;
        let active_sym = &self.active_symbol;
        let mut rows = Column::new().spacing(2);
        let mut past_favs = false;
        let mut current_exchange_group: Option<String> = None;

        for (i, sym) in filtered.iter().enumerate() {
            let is_fav = favs.contains(&sym.key);

            if !past_favs && !is_fav && i > 0 {
                past_favs = true;
                current_exchange_group = None;
                rows = rows.push(rule::horizontal(1));
            }

            if self.symbol_search_sort_mode == SymbolSearchSortMode::Exchange {
                let exchange_group = Self::symbol_search_exchange_label(sym);
                if current_exchange_group.as_deref() != Some(exchange_group.as_str()) {
                    rows = rows.push(
                        container(
                            text(exchange_group.clone())
                                .size(10)
                                .color(theme.extended_palette().background.weak.text),
                        )
                        .padding([4, 6]),
                    );
                    current_exchange_group = Some(exchange_group);
                }
            }

            rows = rows.push(self.view_symbol_search_row(sym, is_fav, active_sym, theme));
        }

        if filtered.is_empty() {
            rows = rows.push(
                container(
                    text("No matching symbols")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                )
                .padding([8, 0]),
            );
        }

        rows
    }
}
