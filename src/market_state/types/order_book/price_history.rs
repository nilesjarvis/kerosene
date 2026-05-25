use super::OrderBookInstance;
use crate::helpers::positive_finite_value;

use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Order Book Price History
// ---------------------------------------------------------------------------

const SHORT_TERM_PRICE_MOVE_WINDOW: Duration = Duration::from_secs(3);
const SHORT_TERM_PRICE_HISTORY_WINDOW: Duration = Duration::from_secs(10);
const SHORT_TERM_PRICE_HISTORY_LIMIT: usize = 2_048;

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
}

fn mid_prices_match(left: f64, right: f64) -> bool {
    let tolerance = f64::EPSILON * 8.0 * left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= tolerance
}
