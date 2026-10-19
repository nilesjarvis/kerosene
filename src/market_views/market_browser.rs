use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::market_state::market_browser::{MarketPicker, MarketVenue};
use crate::message::Message;
use iced::widget::{Column, Row, button, column, container, row, scrollable, text, tooltip};
use iced::{Alignment, Element, Fill, Theme};

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(crate) fn view_market_venue_switch(&self, picker: MarketPicker) -> Element<'_, Message> {
        let selected = self.market_browser.venue(picker);
        MarketVenue::ALL
            .into_iter()
            .fold(Row::new().spacing(2), |controls, venue| {
                controls.push(
                    button(text(venue.label()).size(11))
                        .padding([2, 7])
                        .on_press(Message::MarketPickerVenueChanged(picker, venue))
                        .style(move |theme: &Theme, status| {
                            let active = selected == venue;
                            button::Style {
                                background: (active || status == button::Status::Hovered)
                                    .then(|| theme.extended_palette().background.weak.color.into()),
                                text_color: if active {
                                    theme.palette().text
                                } else {
                                    theme.extended_palette().background.weak.text
                                },
                                border: iced::Border {
                                    radius: 3.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        }),
                )
            })
            .into()
    }

    pub(crate) fn view_papertrade_markets(&self, query: &str) -> Element<'_, Message> {
        let theme = self.theme();
        let muted = theme.extended_palette().background.weak.text;
        let catalog = &self.market_browser.papertrade;
        let query = query.trim().to_ascii_lowercase();
        let markets: Vec<_> = catalog
            .data
            .iter()
            .flat_map(|data| &data.markets)
            .filter(|market| !self.is_ticker_muted(&market.symbol))
            .filter(|market| market.matches_query(&query))
            .collect();
        let caption = if catalog.loading {
            "Loading markets…".to_string()
        } else if catalog.data.is_some() {
            format!("{} markets · Preview", markets.len())
        } else {
            "Market preview".to_string()
        };
        let mut refresh = button(
            text(if catalog.error.is_some() {
                "Retry"
            } else {
                "Refresh"
            })
            .size(11),
        )
        .padding([2, 5])
        .style(button::text);
        if !catalog.loading {
            refresh = refresh.on_press(Message::RefreshPapertradeMarkets);
        }
        let header = row![
            tooltip(
                text(caption).size(11).color(muted),
                text("Browse only. Papertrade chart selection and trading are not connected yet.")
                    .size(11),
                tooltip::Position::Bottom,
            ),
            iced::widget::Space::new().width(Fill),
            refresh,
        ]
        .align_y(Alignment::Center)
        .spacing(4);
        let mut content = column![header].spacing(4);
        if let Some(error) = &catalog.error {
            content = content.push(text(error).size(11).color(theme.palette().danger));
            if catalog.data.is_some() {
                content = content.push(text("Showing cached markets").size(10).color(muted));
            }
        }
        let mut rows = Column::new().spacing(2);
        for market in &markets {
            let mut cells = Row::new().spacing(6).align_y(Alignment::Center);
            if let Some(icon) = helpers::symbol_icon(&market.symbol, 14, theme.palette().text) {
                cells = cells.push(icon);
            }
            cells = cells
                .push(text(market.symbol.as_str()).size(12).width(Fill))
                .push(
                    text(format!("{}×", market.max_leverage))
                        .size(10)
                        .color(muted),
                )
                .push(text(market.status).size(10).color(muted));
            let block = catalog
                .data
                .as_ref()
                .map(|data| data.indexed_through_block)
                .unwrap_or_default();
            rows = rows.push(tooltip(
                container(cells).padding([4, 6]).width(Fill),
                text(format!(
                    "Papertrade instrument {} · block {block} · browse only",
                    market.id.0
                ))
                .size(11),
                tooltip::Position::Bottom,
            ));
        }
        if markets.is_empty() && catalog.data.is_some() && !catalog.loading {
            rows = rows.push(text("No matching markets").size(12).color(muted));
        }
        content = content.push(container(scrollable(rows)).max_height(150).width(Fill));
        content.width(Fill).into()
    }
}
