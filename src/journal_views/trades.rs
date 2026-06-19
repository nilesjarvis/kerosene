use crate::app_state::TradingTerminal;
use crate::journal::{self, AggregatedTrade};
use crate::message::Message;
use iced::widget::{container, text};
use iced::{Color, Element, Fill, Theme};
use std::cmp::Ordering;

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
                // Already sorted newest-first by aggregate_trades.
            }
            journal::JournalSort::PnlDesc => {
                filtered_trades.sort_by(|a, b| compare_f64_desc(a.pnl, b.pnl));
            }
            journal::JournalSort::PnlAsc => {
                filtered_trades.sort_by(|a, b| compare_f64_asc(a.pnl, b.pnl));
            }
        }

        filtered_trades
    }

    pub(super) fn view_journal_fetching_history_row<'a>(
        &self,
        theme: &Theme,
    ) -> Element<'a, Message> {
        container(
            text("Fetching historical trades...")
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(theme.palette().success),
        )
        .width(Fill)
        .padding(12)
        .center_x(Fill)
        .into()
    }
}

pub(super) fn trade_duration_ms(trade: &AggregatedTrade, current_time_ms: u64) -> u64 {
    trade
        .end_time
        .unwrap_or(current_time_ms)
        .saturating_sub(trade.start_time)
}

fn compare_f64_asc(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

fn compare_f64_desc(a: f64, b: f64) -> Ordering {
    b.partial_cmp(&a).unwrap_or(Ordering::Equal)
}

pub(super) fn journal_pnl_color(pnl: f64, theme: &Theme) -> Color {
    if pnl > 0.0 {
        theme.palette().success
    } else if pnl < 0.0 {
        theme.palette().danger
    } else {
        theme.palette().text
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

    fn sortable_trade(coin: &str, start_time: u64, pnl: f64) -> AggregatedTrade {
        AggregatedTrade {
            pnl,
            ..trade(coin, start_time)
        }
    }

    fn sorted_coins(terminal: &mut TradingTerminal, sort: journal::JournalSort) -> Vec<String> {
        terminal.journal.sort = sort;
        terminal
            .filtered_journal_trades()
            .iter()
            .map(|trade| trade.coin.clone())
            .collect()
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

    #[test]
    fn journal_sort_orders_visible_trades() {
        let mut terminal = TradingTerminal::boot().0;
        terminal.journal.trades = vec![
            sortable_trade("BTC", 300, -4.0),
            sortable_trade("ETH", 200, 7.0),
            sortable_trade("SOL", 100, 0.0),
        ];

        assert_eq!(
            sorted_coins(&mut terminal, journal::JournalSort::TimeDesc),
            vec!["BTC", "ETH", "SOL"]
        );
        assert_eq!(
            sorted_coins(&mut terminal, journal::JournalSort::PnlDesc),
            vec!["ETH", "SOL", "BTC"]
        );
        assert_eq!(
            sorted_coins(&mut terminal, journal::JournalSort::PnlAsc),
            vec!["BTC", "SOL", "ETH"]
        );
    }
}
