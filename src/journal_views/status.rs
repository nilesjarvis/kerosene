use crate::app_state::TradingTerminal;

impl TradingTerminal {
    /// Visible (fill, trade) counts for the toolbar, honoring hidden symbols.
    pub(super) fn journal_visible_counts(&self) -> (usize, usize) {
        let visible_fill_count = self
            .journal
            .raw_fills
            .iter()
            .filter(|fill| !self.symbol_key_is_hidden(&fill.coin))
            .count();
        let visible_trade_count = self
            .journal
            .trades
            .iter()
            .filter(|trade| !self.symbol_key_is_hidden(&trade.coin))
            .count();

        (visible_fill_count, visible_trade_count)
    }
}
