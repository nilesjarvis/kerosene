use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::market_state::{
    SYMBOL_SEARCH_ALL_HIP3_DEXES, SymbolSearchMarketFilter, SymbolSearchSortMode,
};
use crate::message::Message;
use iced::Fill;
use iced::widget::{Column, column, pick_list, row, text_input};

// ---------------------------------------------------------------------------
// Symbol Search Controls
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_symbol_search_controls(&self) -> Column<'_, Message> {
        let search_bar = text_input("Search symbols...", &self.symbol_search_query)
            .style(helpers::text_input_style)
            .on_input(Message::SymbolSearchChanged)
            .size(12)
            .padding([5, 8]);

        let market_picker = pick_list(
            SymbolSearchMarketFilter::ALL.as_ref(),
            Some(self.symbol_search_market_filter),
            Message::SymbolSearchMarketFilterChanged,
        )
        .width(Fill)
        .padding([2, 8])
        .text_size(11);

        let sort_picker = pick_list(
            SymbolSearchSortMode::ALL.as_ref(),
            Some(self.symbol_search_sort_mode),
            Message::SymbolSearchSortChanged,
        )
        .width(Fill)
        .padding([2, 8])
        .text_size(11);

        let controls = row![market_picker, sort_picker]
            .spacing(4)
            .align_y(iced::Alignment::Center);

        let mut header_content = column![search_bar, controls].spacing(4);

        if self.symbol_search_market_filter == SymbolSearchMarketFilter::Hip3 {
            let mut dex_options = Vec::with_capacity(self.symbol_search_hip3_dexes().len() + 1);
            dex_options.push(SYMBOL_SEARCH_ALL_HIP3_DEXES.to_string());
            dex_options.extend(self.symbol_search_hip3_dexes());
            let selected_dex = self
                .symbol_search_hip3_dex_filter
                .clone()
                .unwrap_or_else(|| SYMBOL_SEARCH_ALL_HIP3_DEXES.to_string());
            header_content = header_content.push(
                pick_list(
                    dex_options,
                    Some(selected_dex),
                    Message::SymbolSearchHip3DexFilterChanged,
                )
                .width(Fill)
                .padding([2, 8])
                .text_size(11),
            );
        }

        header_content
    }
}
