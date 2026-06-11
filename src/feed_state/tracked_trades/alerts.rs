use crate::app_state::TradingTerminal;
use crate::ws::TrackedTradeEvent;
use std::collections::VecDeque;

use super::model::{TrackedTradeAggregationKey, TrackedTradeFeedRow, TrackedTradeIntent};

// ---------------------------------------------------------------------------
// Tracked Trade Alerts
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn tracked_trade_alert_row_for_event_from(
        existing_trades: &VecDeque<TrackedTradeEvent>,
        aggregation_enabled: bool,
        trade: &TrackedTradeEvent,
    ) -> Option<TrackedTradeFeedRow> {
        let row = TrackedTradeFeedRow::from_event(trade);
        if !aggregation_enabled {
            return Some(row);
        }

        let key = TrackedTradeAggregationKey::from_event(trade);
        for existing in existing_trades {
            if TrackedTradeAggregationKey::from_event(existing) == key && row.can_merge(existing) {
                return None;
            }
        }

        Some(row)
    }

    pub(crate) fn tracked_trade_alert_row_for_event(
        &self,
        trade: &TrackedTradeEvent,
    ) -> Option<TrackedTradeFeedRow> {
        Self::tracked_trade_alert_row_for_event_from(
            &self.tracked_trades,
            self.tracked_trade_aggregation_enabled,
            trade,
        )
    }

    pub(crate) fn tracked_trade_alert_message_for_row(&self, row: &TrackedTradeFeedRow) -> String {
        let wallet = self.wallet_display(&row.address).primary;
        let side = if row.is_buy { "BUY" } else { "SELL" };
        let intent = if row.intent == TrackedTradeIntent::Unknown && !row.dir.is_empty() {
            row.dir.as_str()
        } else {
            row.intent.label()
        };
        let unit = if self.tracked_trade_aggregation_enabled {
            "order"
        } else {
            "fill"
        };
        let coin = if row.coin.starts_with('@') || row.coin.starts_with('#') {
            self.display_coin_for_journal(&row.coin)
        } else {
            row.coin.to_uppercase()
        };
        format!(
            "{} {} {} {} {} {} at {}",
            wallet,
            intent,
            side,
            coin,
            unit,
            self.format_display_usd_value(row.notional, 0),
            self.format_display_price(row.avg_price)
        )
    }
}

#[cfg(test)]
mod tests;
