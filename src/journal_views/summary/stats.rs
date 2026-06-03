use crate::journal::AggregatedTrade;
use std::collections::HashMap;

pub(super) type JournalAssetStats = Vec<(String, (usize, f64, f64))>;

pub(super) struct JournalSummaryStats {
    pub(super) total_pnl: f64,
    pub(super) total_fees: f64,
    pub(super) total_closed: usize,
    pub(super) winning_closed: usize,
    pub(super) sorted_assets: JournalAssetStats,
}

impl JournalSummaryStats {
    pub(super) fn win_rate(&self) -> f64 {
        if self.total_closed > 0 {
            (self.winning_closed as f64 / self.total_closed as f64) * 100.0
        } else {
            0.0
        }
    }
}

pub(super) fn journal_summary_stats(filtered_trades: &[&AggregatedTrade]) -> JournalSummaryStats {
    let mut total_pnl = 0.0;
    let mut total_fees = 0.0;
    let mut total_closed = 0;
    let mut winning_closed = 0;
    let mut asset_stats = HashMap::new();

    for trade in filtered_trades {
        total_pnl += trade.pnl;
        total_fees += trade.fee;

        let stats = asset_stats
            .entry(trade.coin.clone())
            .or_insert((0usize, 0.0f64, 0.0f64));
        stats.0 += 1;
        stats.1 += trade.pnl;
        stats.2 += trade.fee;

        if trade.status == "CLOSED"
            && !trade.coin.starts_with('@')
            && !trade.coin.starts_with('#')
            && trade.basis_complete
        {
            total_closed += 1;
            if trade.pnl > 0.0 {
                winning_closed += 1;
            }
        }
    }

    let mut sorted_assets: JournalAssetStats = asset_stats.into_iter().collect();
    sorted_assets.sort_unstable_by(|a, b| b.1.0.cmp(&a.1.0).then_with(|| a.0.cmp(&b.0)));

    JournalSummaryStats {
        total_pnl,
        total_fees,
        total_closed,
        winning_closed,
        sorted_assets,
    }
}
