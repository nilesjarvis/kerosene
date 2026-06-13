use super::super::fills::fill_summary_for_order;
use super::super::model::{TwapChildStatus, TwapOrder, TwapStatus};
use crate::account::UserFill;
use crate::helpers::positive_finite_value;

// ---------------------------------------------------------------------------
// TWAP Fill Reconciliation
// ---------------------------------------------------------------------------

impl TwapOrder {
    pub(crate) fn has_status_unknown_child(&self) -> bool {
        self.child_orders.iter().any(|child| {
            matches!(
                child.status,
                TwapChildStatus::StatusUnknown | TwapChildStatus::AwaitingReconciliation
            )
        })
    }

    pub(crate) fn mark_filled(&mut self, filled_size: f64) {
        let Some(filled_size) = positive_finite_value(filled_size) else {
            return;
        };
        self.filled_size = (self.filled_size + filled_size).min(self.target_size);
        self.remaining_size = (self.target_size - self.filled_size).max(0.0);
        if self.remaining_size <= f64::EPSILON {
            self.remaining_size = 0.0;
            self.clear_pause();
            self.status = TwapStatus::Completed;
        }
    }

    pub(crate) fn reconcile_fills(&mut self, fills: &[UserFill]) {
        let had_status_unknown = self.has_status_unknown_child();
        let expected_coin = self.coin.as_str();
        let expected_is_buy = self.is_buy;

        for child in &mut self.child_orders {
            let Some(oid) = child.oid else {
                continue;
            };
            let summary = fill_summary_for_order(fills, oid, expected_coin, expected_is_buy);
            if let Some(summary) = summary {
                child.filled_size = child.filled_size.max(summary.filled_size);
                child.avg_price = summary.avg_price.or(child.avg_price);
                child.fee = child.fee.max(summary.fee.abs());
                if child.filled_size > 0.0 && child.status != TwapChildStatus::Rejected {
                    child.status = TwapChildStatus::Filled;
                }
            }
        }

        let reconciled: f64 = self
            .child_orders
            .iter()
            .map(|child| child.filled_size)
            .sum();
        if reconciled.is_finite() && reconciled > self.filled_size {
            self.filled_size = reconciled.min(self.target_size);
            self.remaining_size = (self.target_size - self.filled_size).max(0.0);
        }
        if self.remaining_size <= f64::EPSILON
            && (matches!(
                self.status,
                TwapStatus::Running
                    | TwapStatus::WaitingForMarket
                    | TwapStatus::Paused
                    | TwapStatus::CompletedPartial
            ) || (had_status_unknown && self.status == TwapStatus::Error))
        {
            self.remaining_size = 0.0;
            self.clear_pause();
            self.status = TwapStatus::Completed;
        } else if had_status_unknown && self.status == TwapStatus::Error && self.filled_size > 0.0 {
            self.status = TwapStatus::CompletedPartial;
        } else if self.status == TwapStatus::Paused && !self.has_status_unknown_child() {
            self.clear_pause();
        }
    }
}
