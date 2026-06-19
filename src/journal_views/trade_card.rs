mod components;
mod editor;
mod snapshot;

pub(in crate::journal_views) use components::{
    journal_chip, journal_note_block, journal_tag_chips, push_opt,
};

use crate::app_state::TradingTerminal;
use crate::journal::AggregatedTrade;

// ---------------------------------------------------------------------------
// Trade card helpers
//
// The master-detail inspector reuses the chart snapshot (`snapshot`), the
// reflection editor (`editor`), and the accent note block (`components`). This
// module retains the position-label helper shared across those views.
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(in crate::journal_views) fn journal_max_position_label(
        &self,
        trade: &AggregatedTrade,
    ) -> String {
        let side_label = if trade.coin.starts_with('@') {
            "Spot"
        } else if trade.coin.starts_with('#') {
            "Outcome"
        } else if trade.is_long {
            "Long"
        } else {
            "Short"
        };
        let max_position = trade.max_position.abs();
        if self.is_outcome_coin(&trade.coin) {
            format!("{} {:.0}", side_label, max_position)
        } else {
            format!("{} {:.2}", side_label, max_position)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aggregated_trade(coin: &str, max_position: f64, is_long: bool) -> AggregatedTrade {
        AggregatedTrade {
            id: "trade-1".to_string(),
            legacy_note_ids: Vec::new(),
            coin: coin.to_string(),
            start_time: 0,
            end_time: None,
            max_position,
            volume: 0.0,
            fee: 0.0,
            pnl: 0.0,
            status: "OPEN".to_string(),
            fill_count: 1,
            avg_entry_price: 0.0,
            total_entry_notional: 0.0,
            total_entry_size: 0.0,
            is_long,
            basis_complete: true,
        }
    }

    #[test]
    fn journal_max_position_label_uses_whole_units_for_outcome_contracts() {
        let terminal = TradingTerminal::boot().0;

        assert_eq!(
            terminal.journal_max_position_label(&aggregated_trade("#950", 30.0, true)),
            "Outcome 30"
        );
    }

    #[test]
    fn journal_max_position_label_keeps_two_decimals_for_other_markets() {
        let terminal = TradingTerminal::boot().0;

        assert_eq!(
            terminal.journal_max_position_label(&aggregated_trade("BTC", 0.5, false)),
            "Short 0.50"
        );
        assert_eq!(
            terminal.journal_max_position_label(&aggregated_trade("@107", 1.25, true)),
            "Spot 1.25"
        );
    }
}
