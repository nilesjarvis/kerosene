use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;

impl TradingTerminal {
    pub(in crate::market_views::watchlist) fn filtered_symbol_search_results(
        &self,
    ) -> Vec<&ExchangeSymbol> {
        self.symbol_search_result_indices
            .iter()
            .filter_map(|index| self.exchange_symbols.get(*index))
            .collect()
    }
}
