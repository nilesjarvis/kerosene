use crate::app_state::TradingTerminal;
use crate::journal::{self, AggregatedTrade};

impl TradingTerminal {
    pub(super) fn filtered_journal_trades(&self) -> Vec<&AggregatedTrade> {
        let mut filtered_trades: Vec<_> = self
            .journal
            .trades
            .iter()
            .filter(|trade| !self.symbol_key_is_hidden(&trade.coin))
            .filter(|trade| match self.journal.filter {
                journal::JournalFilter::All => true,
                journal::JournalFilter::Perp => {
                    !trade.coin.starts_with('@') && !trade.coin.starts_with('#')
                }
                journal::JournalFilter::Spot => trade.coin.starts_with('@'),
            })
            .collect();

        match self.journal.sort {
            journal::JournalSort::TimeDesc => {
                // Already sorted this way by aggregate_trades.
            }
            journal::JournalSort::TimeAsc => {
                filtered_trades.reverse();
            }
            journal::JournalSort::PnlDesc => {
                filtered_trades.sort_by(|a, b| {
                    b.pnl
                        .partial_cmp(&a.pnl)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            journal::JournalSort::PnlAsc => {
                filtered_trades.sort_by(|a, b| {
                    a.pnl
                        .partial_cmp(&b.pnl)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        filtered_trades
    }
}
