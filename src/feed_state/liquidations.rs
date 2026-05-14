mod model;

use crate::app_state::TradingTerminal;
use crate::ws::LiquidationEvent;
use std::collections::HashMap;

use super::LIQUIDATION_FEED_RENDER_LIMIT;
use model::LiquidationAggregationKey;

pub(crate) use model::LiquidationFeedRow;

// ---------------------------------------------------------------------------
// Liquidation Feed State
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn recompute_liquidation_buckets(&mut self) {
        self.liquidation_summary_buckets.clear();
        self.liquidation_chart_buckets.clear();
        for liq in &self.liquidations {
            let event_notional = liq.size * liq.price;
            let bucket_ms = liq.time_ms / 60_000;
            let summary_entry = self
                .liquidation_summary_buckets
                .entry(bucket_ms)
                .or_insert((0.0, 0.0));
            if liq.is_buy {
                summary_entry.1 += event_notional;
            } else {
                summary_entry.0 += event_notional;
            }

            let chart_bucket_sec = liq.time_ms / 1000;
            let chart_entry = self
                .liquidation_chart_buckets
                .entry(chart_bucket_sec)
                .or_insert((0.0, 0.0));
            if liq.is_buy {
                chart_entry.1 += event_notional;
            } else {
                chart_entry.0 += event_notional;
            }
        }
    }

    pub(crate) fn normalize_liquidation_event(mut liq: LiquidationEvent) -> LiquidationEvent {
        liq.liquidated_user = Self::normalize_wallet_address(&liq.liquidated_user)
            .unwrap_or_else(|| liq.liquidated_user.trim().to_string());
        liq
    }

    fn aggregated_liquidation_feed_rows(&self) -> Vec<LiquidationFeedRow> {
        let mut rows: Vec<LiquidationFeedRow> = Vec::new();
        let mut latest_by_key: HashMap<LiquidationAggregationKey, usize> = HashMap::new();
        let mut yielded_count = 0;

        for liq in &self.liquidations {
            if self.symbol_key_is_hidden(&liq.coin) {
                continue;
            }
            let key = LiquidationAggregationKey::from_event(liq);
            let existing_index = latest_by_key.get(&key).copied();

            if let Some(index) = existing_index
                && rows.get(index).is_some_and(|row| row.can_merge(liq))
            {
                if let Some(row) = rows.get_mut(index) {
                    row.add_event(liq);
                }
                continue;
            }

            let index = rows.len();
            rows.push(LiquidationFeedRow::from_event(liq));
            latest_by_key.insert(key, index);

            let notional = liq.size * liq.price;
            if notional >= self.liquidation_alert_threshold {
                yielded_count += 1;
                if yielded_count >= LIQUIDATION_FEED_RENDER_LIMIT + 50 {
                    break;
                }
            }
        }

        rows
    }

    pub(crate) fn visible_liquidation_feed_rows(&self) -> Vec<LiquidationFeedRow> {
        let mut visible = Vec::new();

        if self.liquidation_feed_aggregation_enabled {
            for row in self.aggregated_liquidation_feed_rows() {
                if row.notional < self.liquidation_alert_threshold {
                    continue;
                }
                visible.push(row);
                if visible.len() >= LIQUIDATION_FEED_RENDER_LIMIT {
                    break;
                }
            }

            return visible;
        }

        for liq in &self.liquidations {
            if self.symbol_key_is_hidden(&liq.coin) {
                continue;
            }
            let notional = liq.size * liq.price;
            if notional < self.liquidation_alert_threshold {
                continue;
            }

            visible.push(LiquidationFeedRow::from_event(liq));
            if visible.len() >= LIQUIDATION_FEED_RENDER_LIMIT {
                break;
            }
        }

        visible
    }

    pub(crate) fn calculate_liquidation_summary(&self, minutes: u64, now_ms: u64) -> (f64, f64) {
        let current_minute = now_ms / 60_000;
        let cutoff = current_minute.saturating_sub(minutes);

        self.liquidation_summary_buckets.range(cutoff..).fold(
            (0.0, 0.0),
            |acc, (_, (long_notional, short_notional))| {
                (acc.0 + long_notional, acc.1 + short_notional)
            },
        )
    }
}
