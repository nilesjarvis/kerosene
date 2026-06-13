use crate::app_state::TradingTerminal;
use crate::journal::{self, AggregatedTrade};

impl TradingTerminal {
    pub(super) fn filtered_journal_trades(&self) -> Vec<&AggregatedTrade> {
        let mut filtered_trades: Vec<_> = self
            .journal
            .trades
            .iter()
            .filter(|trade| !self.symbol_key_is_hidden(&trade.coin))
            .filter(|trade| self.journal.filter.matches_coin(&trade.coin))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn trade(coin: &str, start_time: u64) -> AggregatedTrade {
        AggregatedTrade {
            id: format!("{coin}-{start_time}"),
            legacy_note_ids: Vec::new(),
            coin: coin.to_string(),
            start_time,
            end_time: Some(start_time),
            max_position: 1.0,
            volume: 100.0,
            fee: 1.0,
            pnl: 1.0,
            status: "CLOSED".to_string(),
            fill_count: 1,
            avg_entry_price: 100.0,
            total_entry_notional: 100.0,
            total_entry_size: 1.0,
            is_long: true,
            basis_complete: true,
        }
    }

    #[test]
    fn journal_filters_partition_perp_spot_and_outcome_trades() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.journal.trades = vec![trade("BTC", 3), trade("@107", 2), trade("#950", 1)];

        terminal.journal.filter = journal::JournalFilter::All;
        assert_eq!(
            terminal
                .filtered_journal_trades()
                .iter()
                .map(|trade| trade.coin.as_str())
                .collect::<Vec<_>>(),
            vec!["BTC", "@107", "#950"]
        );

        terminal.journal.filter = journal::JournalFilter::Perp;
        assert_eq!(
            terminal
                .filtered_journal_trades()
                .iter()
                .map(|trade| trade.coin.as_str())
                .collect::<Vec<_>>(),
            vec!["BTC"]
        );

        terminal.journal.filter = journal::JournalFilter::Spot;
        assert_eq!(
            terminal
                .filtered_journal_trades()
                .iter()
                .map(|trade| trade.coin.as_str())
                .collect::<Vec<_>>(),
            vec!["@107"]
        );

        terminal.journal.filter = journal::JournalFilter::Outcome;
        assert_eq!(
            terminal
                .filtered_journal_trades()
                .iter()
                .map(|trade| trade.coin.as_str())
                .collect::<Vec<_>>(),
            vec!["#950"]
        );
    }
}
