mod controls;
mod rows;

use crate::app_state::TradingTerminal;
use crate::market_state::SymbolSearchSortMode;
use crate::message::Message;
use iced::widget::{container, row, rule, scrollable, text};
use iced::{Element, Fill, color};

impl TradingTerminal {
    pub(crate) fn view_watchlist(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let mut header_content = self.view_symbol_search_controls();

        if self.exchange_symbols.is_empty() {
            let loading_row: Element<'_, Message> = if self.symbols_loading {
                row![
                    self.view_spinner(18),
                    text("Loading symbols...")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .into()
            } else {
                text("No symbols available")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text)
                    .into()
            };
            if let Some((status, is_error)) = &self.symbol_search_status {
                let status_color = if *is_error {
                    color!(0xff5555)
                } else {
                    theme.extended_palette().background.weak.text
                };
                header_content = header_content.push(text(status).size(11).color(status_color));
            }
            let content = header_content
                .push(container(loading_row).padding([8, 0]))
                .spacing(4);

            return container(content)
                .width(Fill)
                .height(Fill)
                .padding(10)
                .into();
        }

        let filtered = self.filtered_symbol_search_results();
        let fav_count = self.symbol_search_favourite_count;
        let mut count_text = format!("{} favourites, {} symbols", fav_count, filtered.len());
        if self.symbol_search_sort_mode == SymbolSearchSortMode::Volume24h {
            if self.symbol_search_contexts_loading {
                count_text.push_str(" - loading 24h volume");
            } else {
                count_text.push_str(" - 24h volume sort");
            }
        }
        let count_label = text(count_text)
            .size(11)
            .color(theme.extended_palette().background.weak.text);

        let rows = self.view_symbol_search_rows(&filtered, &theme);

        header_content = header_content.push(count_label);
        if let Some((status, is_error)) = &self.symbol_search_status {
            let status_color = if *is_error {
                color!(0xff5555)
            } else {
                theme.extended_palette().background.weak.text
            };
            header_content = header_content.push(text(status).size(11).color(status_color));
        }

        let content = header_content
            .push(rule::horizontal(1))
            .push(scrollable(rows))
            .spacing(4);

        container(content).width(Fill).height(Fill).into()
    }
}
