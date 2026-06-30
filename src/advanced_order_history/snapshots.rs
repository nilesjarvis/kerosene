use crate::account::UserFill;
use crate::helpers::positive_finite_value;
use crate::signing::ChaseOrder;
use crate::twap_state::{TwapOrder, TwapStatus, twap_weighted_average_fill_price};

use super::{
    AdvancedOrderHistoryChild, AdvancedOrderHistoryEntry, AdvancedOrderHistoryKind,
    AdvancedOrderHistoryLog, ChaseHistoryFillMetrics,
};
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Advanced Order History Snapshots
// ---------------------------------------------------------------------------

const ADVANCED_ORDER_HISTORY_LOG_LIMIT: usize = 200;
const ADVANCED_ORDER_HISTORY_CHILD_LIMIT: usize = 200;

impl AdvancedOrderHistoryEntry {
    pub(crate) fn chase_fill_metrics(
        fills: &[UserFill],
        chase: &ChaseOrder,
    ) -> Option<ChaseHistoryFillMetrics> {
        let oids = chase.known_oids_with_current();
        if oids.is_empty() {
            return None;
        }

        let mut metrics = ChaseHistoryFillMetrics::default();
        let mut seen = HashSet::new();
        let mut matched = false;
        for fill in fills {
            let Some(oid) = fill.oid else {
                continue;
            };
            if !oids.contains(&oid) {
                continue;
            }
            if !seen.insert(fill.dedup_key()) {
                continue;
            }
            matched = true;

            if let (Some(size), Some(price)) = (
                parse_positive_fill_number(&fill.sz),
                parse_positive_fill_number(&fill.px),
            ) {
                metrics.filled_size += size;
                metrics.gross_notional += size * price;
            }
            if let Some(fee) = parse_finite_fill_number(&fill.fee) {
                metrics.total_fee += fee;
            }
            if let Some(closed_pnl) = parse_finite_fill_number(&fill.closed_pnl) {
                metrics.closed_pnl += closed_pnl;
            }
        }

        matched.then_some(metrics)
    }

    pub(crate) fn from_twap(twap: &TwapOrder, completed_at_ms: u64) -> Self {
        let logs = twap
            .events
            .iter()
            .rev()
            .take(ADVANCED_ORDER_HISTORY_LOG_LIMIT)
            .rev()
            .map(|event| AdvancedOrderHistoryLog {
                elapsed_ms: event
                    .at
                    .saturating_duration_since(twap.started_at)
                    .as_millis() as u64,
                kind: format!("{:?}", event.kind),
                message: event.message.clone(),
                is_error: event.is_error,
            })
            .collect();
        let children = twap
            .child_orders
            .iter()
            .rev()
            .take(ADVANCED_ORDER_HISTORY_CHILD_LIMIT)
            .rev()
            .map(|child| AdvancedOrderHistoryChild {
                index: child.index,
                elapsed_ms: child
                    .requested_at
                    .saturating_duration_since(twap.started_at)
                    .as_millis() as u64,
                planned_size: finite_or_zero(child.planned_size),
                limit_price: finite_or_zero(child.limit_price),
                filled_size: finite_or_zero(child.filled_size),
                avg_price: child.avg_price.and_then(positive_finite_value),
                fee: finite_or_zero(child.fee),
                oid: child.oid,
                cloid: child.cloid.clone(),
                status: child.status.label().to_string(),
                exchange_summary: child.exchange_summary.clone(),
            })
            .collect();
        let summary = twap
            .events
            .last()
            .map(|event| event.message.clone())
            .unwrap_or_else(|| twap.status.label().to_string());

        Self {
            id: format!(
                "twap:{}:{}:{}",
                twap.account_address, twap.started_at_ms, twap.id
            ),
            kind: AdvancedOrderHistoryKind::Twap,
            source_id: twap.id,
            account_address: twap.account_address.clone(),
            coin: twap.coin.clone(),
            display_coin: twap.display_coin.clone(),
            is_buy: twap.is_buy,
            target_size: finite_or_zero(twap.target_size),
            filled_size: finite_or_zero(twap.filled_size),
            remaining_size: finite_or_zero(twap.remaining_size),
            average_price: twap_weighted_average_fill_price(twap),
            last_working_price: None,
            gross_notional: 0.0,
            total_fee: twap
                .child_orders
                .iter()
                .map(|child| finite_or_zero(child.fee))
                .sum(),
            closed_pnl: 0.0,
            min_price: positive_finite_value(twap.min_price),
            max_price: positive_finite_value(twap.max_price),
            reduce_only: twap.reduce_only,
            randomize: twap.randomize,
            slice_count: twap.slice_count,
            slices_sent: twap.slices_sent,
            reprice_count: 0,
            status: twap_history_status(twap.status).to_string(),
            summary,
            started_at_ms: twap.started_at_ms,
            completed_at_ms,
            logs,
            children,
        }
    }

    pub(crate) fn from_chase_with_fill_metrics(
        chase: &ChaseOrder,
        display_coin: String,
        completed_at_ms: u64,
        summary: String,
        fill_metrics: Option<ChaseHistoryFillMetrics>,
    ) -> Self {
        let status = chase
            .stop_reason
            .as_ref()
            .map(|(_, is_error)| if *is_error { "Error" } else { "Stopped" })
            .unwrap_or("Completed");
        let summary = if summary.trim().is_empty() {
            status.to_string()
        } else {
            summary
        };
        let target_size = finite_or_zero(chase.target_size);
        let filled_size = if let Some(metrics) = fill_metrics {
            finite_non_negative_or_zero(metrics.filled_size)
        } else if chase.filled_size.is_finite() && chase.filled_size >= 0.0 {
            chase.filled_size
        } else if target_size > 0.0
            && let Some(remaining_size) = positive_finite_value(chase.remaining_size)
        {
            (target_size - remaining_size).clamp(0.0, target_size)
        } else {
            0.0
        };
        let remaining_size = if target_size > 0.0 {
            (target_size - filled_size.min(target_size)).max(0.0)
        } else {
            finite_or_zero(chase.remaining_size)
        };
        let average_price = fill_metrics.and_then(ChaseHistoryFillMetrics::average_price);
        let gross_notional = fill_metrics
            .map(|metrics| finite_or_zero(metrics.gross_notional))
            .unwrap_or_default();
        let total_fee = fill_metrics
            .map(|metrics| finite_or_zero(metrics.total_fee))
            .unwrap_or_default();
        let closed_pnl = fill_metrics
            .map(|metrics| finite_or_zero(metrics.closed_pnl))
            .unwrap_or_default();

        Self {
            id: format!(
                "chase:{}:{}:{}",
                chase.account_address, chase.started_at_ms, chase.id
            ),
            kind: AdvancedOrderHistoryKind::Chase,
            source_id: chase.id,
            account_address: chase.account_address.clone(),
            coin: chase.coin.clone(),
            display_coin,
            is_buy: chase.is_buy,
            target_size,
            filled_size,
            remaining_size,
            average_price,
            last_working_price: positive_finite_value(chase.current_price),
            gross_notional,
            total_fee,
            closed_pnl,
            min_price: None,
            max_price: None,
            reduce_only: chase.reduce_only,
            randomize: false,
            slice_count: 0,
            slices_sent: 0,
            reprice_count: chase.reprice_count,
            status: status.to_string(),
            summary: summary.clone(),
            started_at_ms: chase.started_at_ms,
            completed_at_ms,
            logs: vec![
                AdvancedOrderHistoryLog {
                    elapsed_ms: 0,
                    kind: "Started".to_string(),
                    message: "Chase started".to_string(),
                    is_error: false,
                },
                AdvancedOrderHistoryLog {
                    elapsed_ms: chase
                        .started_at
                        .elapsed()
                        .as_millis()
                        .try_into()
                        .unwrap_or(u64::MAX),
                    kind: status.to_string(),
                    message: summary,
                    is_error: status == "Error",
                },
            ],
            children: Vec::new(),
        }
    }
}

fn parse_positive_fill_number(value: &str) -> Option<f64> {
    parse_finite_fill_number(value).filter(|value| *value > 0.0)
}

fn parse_finite_fill_number(value: &str) -> Option<f64> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn twap_history_status(status: TwapStatus) -> &'static str {
    match status {
        TwapStatus::Running
        | TwapStatus::WaitingForMarket
        | TwapStatus::Paused
        | TwapStatus::Stopping => "Interrupted",
        TwapStatus::Stopped => "Stopped",
        TwapStatus::Completed => "Completed",
        TwapStatus::CompletedPartial => "Partial",
        TwapStatus::Error => "Error",
    }
}

fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

fn finite_non_negative_or_zero(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}
