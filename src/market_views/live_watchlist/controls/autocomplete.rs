use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::market_state::LiveWatchlistId;
use crate::message::Message;
use iced::widget::{Column, Space, button, row, text};
use iced::{Fill, Theme};

impl TradingTerminal {
    pub(in crate::market_views::live_watchlist) fn view_live_watchlist_autocomplete<'a>(
        &'a self,
        id: LiveWatchlistId,
        search_query: &str,
    ) -> Column<'a, Message> {
        let theme = self.theme();
        let query = search_query.to_lowercase();
        let mut autocomplete = Column::new();

        if query.is_empty() {
            return autocomplete;
        }

        let mut matches: Vec<&ExchangeSymbol> = self
            .exchange_symbols
            .iter()
            .filter(|s| !self.exchange_symbol_is_hidden(s))
            .filter(|s| {
                s.ticker.to_lowercase().contains(&query) || s.key.to_lowercase().contains(&query)
            })
            .collect();
        matches.sort_by(|a, b| a.ticker.cmp(&b.ticker));
        matches.truncate(5);

        for m in matches {
            let sym_key = m.key.clone();
            let display = m.display_name.as_deref().unwrap_or(&m.ticker);
            let mut coin_content = row![];
            if let Some(icon) = helpers::symbol_icon(&sym_key, 14, theme.palette().text) {
                coin_content = coin_content.push(icon).push(Space::new().width(4.0));
            }
            coin_content = coin_content.push(
                text(display)
                    .size(12)
                    .color(theme.palette().text)
                    .width(Fill),
            );

            let btn = button(
                coin_content
                    .push(text("Add +").size(10).color(theme.palette().primary))
                    .align_y(iced::Alignment::Center),
            )
            .on_press(Message::LiveWatchlistAddSymbol(id, sym_key))
            .padding([4, 8])
            .style(move |t: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => t.extended_palette().background.strong.color,
                    _ => t.extended_palette().background.weak.color,
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
            .width(Fill);
            autocomplete = autocomplete.push(btn);
        }

        autocomplete
    }
}
