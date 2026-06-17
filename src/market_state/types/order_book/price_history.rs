use super::OrderBookInstance;
use crate::helpers::positive_finite_value;

use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Order Book Price History
// ---------------------------------------------------------------------------

const SHORT_TERM_PRICE_MOVE_WINDOW: Duration = Duration::from_secs(3);
const SHORT_TERM_PRICE_HISTORY_WINDOW: Duration = Duration::from_secs(10);
const SHORT_TERM_PRICE_HISTORY_LIMIT: usize = 2_048;

/// How much spread-chart history is retained. Matches the spread chart's
/// visible window so the line always spans the full chart rather than a tail.
const SPREAD_HISTORY_WINDOW: Duration = Duration::from_secs(300);
/// Cap on retained spread samples. With sampling throttled to one sample per
/// second this is a safety backstop, not the primary bound.
const SPREAD_HISTORY_LIMIT: usize = 4_096;
/// Minimum spacing between spread samples. Bounds the per-frame point count
/// rendered by the spread chart under high-frequency book updates.
const SPREAD_SAMPLE_MIN_INTERVAL: Duration = Duration::from_secs(1);

impl OrderBookInstance {
    pub fn best_bid_ask(&self) -> (Option<f64>, Option<f64>) {
        let mut true_best_bid = self.book.bids.first().map(|level| level.px);
        let mut true_best_ask = self.book.asks.first().map(|level| level.px);

        if let Some(ctx) = &self.asset_ctx
            && let Some(impact) = &ctx.impact_pxs
            && impact.len() >= 2
            && let (Ok(best_bid), Ok(best_ask)) =
                (impact[0].parse::<f64>(), impact[1].parse::<f64>())
        {
            true_best_bid = Some(best_bid);
            true_best_ask = Some(best_ask);
        }

        (
            true_best_bid.and_then(positive_finite_value),
            true_best_ask.and_then(positive_finite_value),
        )
    }

    /// Best bid/ask as displayed by the book rows: the raw top of the
    /// in-memory book, without the impact-price override. The spread row uses
    /// this so its numbers always agree with the adjacent rows; impact prices
    /// from the asset context are only a fallback while the book is empty.
    pub fn visible_best_bid_ask(&self) -> (Option<f64>, Option<f64>) {
        let bid = self
            .book
            .bids
            .first()
            .map(|level| level.px)
            .and_then(positive_finite_value);
        let ask = self
            .book
            .asks
            .first()
            .map(|level| level.px)
            .and_then(positive_finite_value);

        if bid.is_some() || ask.is_some() {
            (bid, ask)
        } else {
            self.best_bid_ask()
        }
    }

    pub fn current_mid_price(&self) -> Option<f64> {
        let (best_bid, best_ask) = self.best_bid_ask();
        let mid = match (best_bid, best_ask) {
            (Some(best_bid), Some(best_ask)) => (best_bid + best_ask) / 2.0,
            (Some(best_bid), None) => best_bid,
            (None, Some(best_ask)) => best_ask,
            (None, None) => return None,
        };

        positive_finite_value(mid)
    }

    pub fn record_mid_price_sample(&mut self, now: Instant) {
        let Some(mid) = self.current_mid_price() else {
            return;
        };

        if let Some((latest_time, latest_mid)) = self.mid_price_history.front_mut()
            && mid_prices_match(*latest_mid, mid)
        {
            *latest_time = now;
            self.trim_mid_price_history(now);
            return;
        }

        self.mid_price_history.push_front((now, mid));
        self.trim_mid_price_history(now);
    }

    pub fn clear_mid_price_history(&mut self) {
        self.mid_price_history.clear();
    }

    /// Record a spread sample derived from the visible top of book — the same
    /// source the spread readout row above the chart displays. The live L2
    /// book is the primary source; impact prices from the asset context fill
    /// in only while the book is empty, so the chart populates as soon as any
    /// book snapshot lands, independent of the `activeAssetCtx` stream.
    ///
    /// Sampling is throttled to one sample per second so a burst of book
    /// updates cannot flood the chart's render path; the 300-second window
    /// then bounds the retained history.
    pub fn record_spread_sample(&mut self, now: Instant) {
        if let Some((last_time, _)) = self.spread_history.front()
            && now
                .checked_duration_since(*last_time)
                .is_some_and(|elapsed| elapsed < SPREAD_SAMPLE_MIN_INTERVAL)
        {
            return;
        }

        let (best_bid, best_ask) = self.visible_best_bid_ask();
        let Some((best_bid, best_ask)) = best_bid.zip(best_ask) else {
            return;
        };
        let spread = best_ask - best_bid;
        if !spread.is_finite() || spread < 0.0 {
            return;
        }

        self.spread_history.push_front((now, spread));
        self.trim_spread_history(now);
    }

    pub fn clear_spread_history(&mut self) {
        self.spread_history.clear();
    }

    pub fn short_term_price_move(&self) -> Option<f64> {
        let (latest_time, latest_price) = self.mid_price_history.front().copied()?;
        let cutoff = latest_time
            .checked_sub(SHORT_TERM_PRICE_MOVE_WINDOW)
            .unwrap_or(latest_time);

        let mut reference = None;
        for (time, price) in &self.mid_price_history {
            if *time < cutoff {
                break;
            }
            reference = Some((*time, *price));
        }

        let (reference_time, reference_price) = reference?;
        if reference_time == latest_time {
            return None;
        }

        Some(latest_price - reference_price)
    }

    fn trim_mid_price_history(&mut self, now: Instant) {
        let cutoff = now
            .checked_sub(SHORT_TERM_PRICE_HISTORY_WINDOW)
            .unwrap_or(now);

        while self.mid_price_history.len() > SHORT_TERM_PRICE_HISTORY_LIMIT {
            self.mid_price_history.pop_back();
        }
        while self
            .mid_price_history
            .back()
            .is_some_and(|(time, _)| *time < cutoff)
        {
            self.mid_price_history.pop_back();
        }
    }

    fn trim_spread_history(&mut self, now: Instant) {
        let cutoff = now.checked_sub(SPREAD_HISTORY_WINDOW).unwrap_or(now);

        while self.spread_history.len() > SPREAD_HISTORY_LIMIT {
            self.spread_history.pop_back();
        }
        while self
            .spread_history
            .back()
            .is_some_and(|(time, _)| *time < cutoff)
        {
            self.spread_history.pop_back();
        }
    }
}

fn mid_prices_match(left: f64, right: f64) -> bool {
    let tolerance = f64::EPSILON * 8.0 * left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= tolerance
}
