use crate::app_state::TradingTerminal;
use std::collections::HashMap;

use super::super::LIQUIDATION_FEED_RENDER_LIMIT;
use super::model::{TrackedTradeAggregationKey, TrackedTradeFeedRow};

// ---------------------------------------------------------------------------
// Tracked Trade Row Aggregation
// ---------------------------------------------------------------------------

impl TradingTerminal {
    fn aggregate_tracked_trade_rows(&self) -> Vec<TrackedTradeFeedRow> {
        let mut rows: Vec<TrackedTradeFeedRow> = Vec::new();
        let mut latest_by_key: HashMap<TrackedTradeAggregationKey, usize> = HashMap::new();

        for trade in &self.tracked_trades {
            if self.is_ticker_muted(&trade.coin) {
                continue;
            }
            let key = TrackedTradeAggregationKey::from_event(trade);
            if let Some(index) = latest_by_key.get(&key).copied()
                && rows.get(index).is_some_and(|row| row.can_merge(trade))
            {
                if let Some(row) = rows.get_mut(index) {
                    row.add_event(trade);
                }
                continue;
            }

            let index = rows.len();
            rows.push(TrackedTradeFeedRow::from_event(trade));
            latest_by_key.insert(key, index);
            if rows.len() >= LIQUIDATION_FEED_RENDER_LIMIT + 100 {
                break;
            }
        }

        rows.truncate(LIQUIDATION_FEED_RENDER_LIMIT);
        rows
    }

    pub(crate) fn visible_tracked_trade_rows(&self) -> Vec<TrackedTradeFeedRow> {
        if self.tracked_trade_aggregation_enabled {
            self.aggregate_tracked_trade_rows()
        } else {
            self.tracked_trades
                .iter()
                .filter(|trade| !self.is_ticker_muted(&trade.coin))
                .take(LIQUIDATION_FEED_RENDER_LIMIT)
                .map(TrackedTradeFeedRow::from_event)
                .collect()
        }
    }
}
