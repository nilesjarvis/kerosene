mod components;
mod formatting;
#[cfg(test)]
mod tests;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use components::{ticker_tape_bar_style, ticker_tape_item, ticker_tape_separator};
use formatting::{TickerTapeItem, percent_change, ticker_tape_item_width};
use iced::widget::{Space, container, float, responsive, row, stack, text};
use iced::{Element, Fill, Length, Theme, Vector};

// ---------------------------------------------------------------------------
// Ticker Tape
// ---------------------------------------------------------------------------

const TICKER_TAPE_HEIGHT: f32 = 34.0;
const TICKER_TAPE_ICON_SIZE: f32 = 16.0;
const TICKER_TAPE_ITEM_MIN_WIDTH: f32 = 148.0;
const TICKER_TAPE_ITEM_MAX_WIDTH: f32 = 240.0;
const TICKER_TAPE_ITEM_HORIZONTAL_PADDING: u16 = 12;
const TICKER_TAPE_ITEM_SPACING: u32 = 5;
const TICKER_TAPE_SEPARATOR_WIDTH: f32 = 1.0;
const TICKER_TAPE_TEXT_CHAR_WIDTH: f32 = 7.0;

impl TradingTerminal {
    pub(crate) fn ticker_tape_bar_height(&self) -> f32 {
        if self.ticker_tape_enabled {
            TICKER_TAPE_HEIGHT
        } else {
            0.0
        }
    }

    pub(crate) fn view_ticker_tape_bar(&self) -> Element<'_, Message> {
        container(
            responsive(|size| self.view_ticker_tape_bar_sized(size.width))
                .height(Length::Fixed(TICKER_TAPE_HEIGHT)),
        )
        .width(Fill)
        .height(Length::Fixed(TICKER_TAPE_HEIGHT))
        .padding([0.0, self.outer_widget_border_padding()])
        .into()
    }

    fn view_ticker_tape_bar_sized(&self, available_width: f32) -> Element<'_, Message> {
        let theme = self.theme();
        let items = self.ticker_tape_items();
        let denomination = self.display_denomination_context();
        let pane_corner_radius = self.pane_corner_radius;

        if items.is_empty() {
            return container(
                text("No favourites")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
            )
            .width(Fill)
            .height(Length::Fixed(TICKER_TAPE_HEIGHT))
            .padding([0, 12])
            .center_y(Length::Fixed(TICKER_TAPE_HEIGHT))
            .style(move |theme: &Theme| ticker_tape_bar_style(theme, pane_corner_radius))
            .into();
        }

        let item_widths: Vec<f32> = items
            .iter()
            .map(|item| ticker_tape_item_width(item, &denomination))
            .collect();
        let sequence_width: f32 = item_widths
            .iter()
            .map(|width| width + TICKER_TAPE_SEPARATOR_WIDTH)
            .sum();
        let should_scroll = sequence_width > available_width.max(0.0);
        let offset = if should_scroll {
            self.ticker_tape_scroll_px.rem_euclid(sequence_width)
        } else {
            0.0
        };
        let repetitions = if should_scroll { 2 } else { 1 };

        let mut tape_row = row![]
            .spacing(0)
            .height(Length::Fixed(TICKER_TAPE_HEIGHT))
            .align_y(iced::Alignment::Center);
        for _ in 0..repetitions {
            for (item, item_width) in items.iter().zip(item_widths.iter().copied()) {
                tape_row = tape_row
                    .push(ticker_tape_item(item, &denomination, &theme, item_width))
                    .push(ticker_tape_separator());
            }
        }

        let tape_width = sequence_width * repetitions as f32;
        let tape = container(tape_row)
            .width(Length::Fixed(tape_width))
            .center_y(Length::Fixed(TICKER_TAPE_HEIGHT));

        let tape_layer: Element<'_, Message> = if should_scroll {
            float(tape)
                .translate(move |_bounds, _viewport| Vector::new(-offset, 0.0))
                .into()
        } else {
            tape.into()
        };

        let layers: Vec<Element<'_, Message>> = vec![
            Space::new()
                .width(Fill)
                .height(Length::Fixed(TICKER_TAPE_HEIGHT))
                .into(),
            tape_layer,
        ];

        container(
            stack(layers)
                .width(Fill)
                .height(Length::Fixed(TICKER_TAPE_HEIGHT))
                .clip(true),
        )
        .width(Fill)
        .height(Length::Fixed(TICKER_TAPE_HEIGHT))
        .clip(true)
        .style(move |theme: &Theme| ticker_tape_bar_style(theme, pane_corner_radius))
        .into()
    }

    fn ticker_tape_items(&self) -> Vec<TickerTapeItem> {
        self.favourite_symbols
            .iter()
            .filter(|symbol| !self.symbol_key_is_hidden(symbol))
            .filter_map(|symbol| self.ticker_tape_item(symbol))
            .collect()
    }

    fn ticker_tape_item(&self, symbol: &str) -> Option<TickerTapeItem> {
        let symbol_meta = self.resolve_exchange_symbol_by_key_or_ticker(symbol);
        if symbol_meta.is_some_and(|symbol| !symbol.is_user_selectable_market()) {
            return None;
        }

        let ticker = match symbol_meta {
            Some(sym) => sym
                .outcome
                .as_ref()
                .map(|info| info.side_condition_short_label())
                .unwrap_or_else(|| Self::exchange_symbol_display_name(sym)),
            None => self.display_name_for_symbol(symbol),
        };
        let price = self.resolve_mid_for_symbol(symbol);
        let ctx = self
            .ticker_tape_ctxs
            .get(symbol)
            .or_else(|| symbol_meta.and_then(|symbol| self.ticker_tape_ctxs.get(&symbol.ticker)));
        let pct_24h = ctx.and_then(|ctx| percent_change(price, ctx.prev_day_px));

        Some(TickerTapeItem {
            symbol: symbol.to_string(),
            ticker,
            price,
            pct_24h,
        })
    }
}
