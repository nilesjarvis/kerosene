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
            let dex_options: Vec<_> = std::iter::once(SYMBOL_SEARCH_ALL_HIP3_DEXES)
                .chain(self.symbol_search_hip3_dexes())
                .collect();
            let selected_dex = self
                .symbol_search_hip3_dex_filter
                .as_deref()
                .unwrap_or(SYMBOL_SEARCH_ALL_HIP3_DEXES);
            header_content = header_content.push(
                pick_list(dex_options, Some(selected_dex), |dex: &str| {
                    Message::SymbolSearchHip3DexFilterChanged(dex.to_string())
                })
                .width(Fill)
                .padding([2, 8])
                .text_size(11),
            );
        }

        header_content
    }
}
