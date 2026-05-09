use crate::app_state::TradingTerminal;
use crate::config;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Live Watchlist Columns
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn ordered_live_watchlist_columns(
        columns: &[config::LiveWatchlistColumn],
    ) -> Vec<config::LiveWatchlistColumn> {
        config::LiveWatchlistColumn::ALL
            .iter()
            .copied()
            .filter(|column| columns.contains(column))
            .collect()
    }

    pub(crate) fn live_watchlist_columns_for_width(
        columns: &[config::LiveWatchlistColumn],
        available_width: f32,
    ) -> Vec<config::LiveWatchlistColumn> {
        const OUTER_PADDING: f32 = 20.0;
        const ROW_PADDING: f32 = 16.0;
        const SYMBOL_MIN_WIDTH: f32 = 92.0;
        const REMOVE_WIDTH: f32 = 24.0;
        const SPACING: f32 = 8.0;

        let mut visible = Self::ordered_live_watchlist_columns(columns);
        let available_width = if available_width.is_finite() && available_width > 0.0 {
            available_width
        } else {
            800.0
        };

        let required_width = |visible: &[config::LiveWatchlistColumn]| {
            let item_count = visible.len() + 2; // symbol + remove button
            let spacing = item_count.saturating_sub(1) as f32 * SPACING;
            OUTER_PADDING
                + ROW_PADDING
                + SYMBOL_MIN_WIDTH
                + REMOVE_WIDTH
                + spacing
                + visible.iter().map(|column| column.width()).sum::<f32>()
        };

        while !visible.is_empty() && required_width(&visible) > available_width {
            visible.pop();
        }

        visible
    }
}
