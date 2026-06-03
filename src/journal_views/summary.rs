mod assets;
mod chart;
mod stats;

use crate::app_state::TradingTerminal;
use crate::journal::AggregatedTrade;
use crate::message::Message;
use iced::widget::Column;

use self::stats::journal_summary_stats;

// ---------------------------------------------------------------------------
// Journal Summary
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn push_journal_summary<'a>(
        &'a self,
        content: Column<'a, Message>,
        filtered_trades: &[&AggregatedTrade],
    ) -> Column<'a, Message> {
        let stats = journal_summary_stats(filtered_trades, self.journal.include_fees_in_pnl);
        let total_pnl = stats.total_pnl;
        let total_fees = stats.total_fees;
        let total_closed = stats.total_closed;
        let win_rate = stats.win_rate();

        let top_assets_box = self.view_journal_top_assets_box(stats.sorted_assets);
        let performance = self.view_journal_summary_chart(
            filtered_trades,
            total_pnl,
            total_fees,
            win_rate,
            total_closed,
        );

        content.push(performance).push(top_assets_box)
    }
}
