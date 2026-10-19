use super::{TradeFeedInstance, TradePrint};

const SUMMARY_MINUTES: u64 = 60;

impl TradeFeedInstance {
    pub(super) fn add_to_summary(&mut self, trade: &TradePrint) {
        let totals = self
            .summary_buckets
            .entry(trade.time_ms / 60_000)
            .or_default();
        if trade.is_buy {
            totals.0 += trade.value;
        } else {
            totals.1 += trade.value;
        }
    }

    pub(super) fn prune_summary(&mut self) {
        if let Some(latest) = self.trades.front() {
            let cutoff = (latest.time_ms / 60_000).saturating_sub(SUMMARY_MINUTES);
            self.summary_buckets.retain(|&minute, _| minute >= cutoff);
        }
    }

    /// Match the liquidation footer's minute buckets, including the current minute.
    /// Totals follow live ingestion even when rows are grouped, filtered, or held.
    pub(crate) fn calculate_summary(&self, minutes: u64, now_ms: u64) -> (f64, f64) {
        let current_minute = now_ms / 60_000;
        let cutoff = current_minute.saturating_sub(minutes.min(SUMMARY_MINUTES));
        self.summary_buckets
            .range(cutoff..=current_minute)
            .fold((0.0, 0.0), |(buy, sell), (_, (bucket_buy, bucket_sell))| {
                (buy + bucket_buy, sell + bucket_sell)
            })
    }
}

#[cfg(test)]
mod tests;
