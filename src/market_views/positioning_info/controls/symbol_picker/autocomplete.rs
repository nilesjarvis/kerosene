use super::super::super::metrics::positioning_symbol_matches;
use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::positioning_state::PositioningInfoId;

use iced::widget::{Column, Row, button, text};
use iced::{Alignment, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Autocomplete
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(in crate::market_views::positioning_info) fn view_positioning_info_autocomplete<'a>(
        &'a self,
        id: PositioningInfoId,
        search_query: &str,
        theme: &Theme,
    ) -> Column<'a, Message> {
        let query = search_query.trim().to_lowercase();
        let autocomplete = Column::new().spacing(3);
        if query.is_empty() {
            return autocomplete;
        }

        let mut matches: Vec<&ExchangeSymbol> = self
            .exchange_symbols
            .iter()
            .filter(|symbol| symbol.market_type == MarketType::Perp)
            .filter(|symbol| !self.exchange_symbol_is_hidden(symbol))
            .filter(|symbol| positioning_symbol_matches(symbol, &query))
            .collect();
        matches.sort_by(|a, b| {
            a.ticker
                .cmp(&b.ticker)
                .then_with(|| helpers::compare_symbol_keys_for_same_ticker(&a.key, &b.key))
        });
        matches.truncate(5);

        matches
            .into_iter()
            .fold(autocomplete, |autocomplete, symbol| {
                autocomplete.push(self.view_positioning_info_autocomplete_row(id, symbol, theme))
            })
    }

    fn view_positioning_info_autocomplete_row<'a>(
        &'a self,
        id: PositioningInfoId,
        symbol: &'a ExchangeSymbol,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let sym_key = symbol.key.clone();
        let display = symbol
            .display_name
            .as_deref()
            .unwrap_or(&symbol.ticker)
            .to_string();
        let mut coin_content = Row::new().spacing(6).align_y(Alignment::Center);
        if let Some(icon) = helpers::symbol_icon(&sym_key, 14, theme.palette().text) {
            coin_content = coin_content.push(icon);
        }
        coin_content = coin_content.push(
            text(display)
                .size(12)
                .color(theme.palette().text)
                .width(Fill),
        );
        if let Some(dex) = helpers::hip3_dex(&sym_key) {
            coin_content = coin_content.push(
                text(dex.to_string())
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            );
        }
        coin_content = coin_content.push(
            text("Select")
                .size(10)
                .color(theme.extended_palette().primary.base.color),
        );

        button(coin_content)
            .on_press(Message::PositioningInfoSymbolSelected(id, sym_key))
            .padding([4, 8])
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.strong.color,
                    _ => theme.extended_palette().background.weak.color,
                };
                button::Style {
                    background: Some(bg.into()),
                    border: iced::Border {
                        radius: 3.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .width(Fill)
            .into()
    }
}
