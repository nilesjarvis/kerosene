use crate::journal::AggregatedTrade;

const DAY_MS: u64 = 24 * 60 * 60 * 1000;
const LEADING_ZERO_POINT_COUNT: usize = 4;
const MAX_LEADING_ZERO_WINDOW_MS: u64 = 7 * DAY_MS;

// ---------------------------------------------------------------------------
// PnL Series
// ---------------------------------------------------------------------------

pub(in crate::journal_views::summary::chart) fn journal_cumulative_pnl_points(
    trades: &[&AggregatedTrade],
) -> Vec<(u64, f64)> {
    let mut trade_pnls = trades
        .iter()
        .filter_map(|trade| {
            let pnl = trade.pnl;
            pnl.is_finite()
                .then_some((trade.end_time.unwrap_or(trade.start_time), pnl))
        })
        .collect::<Vec<_>>();
    trade_pnls.sort_by_key(|(timestamp_ms, _)| *timestamp_ms);

    let Some(first_timestamp) = trade_pnls.first().map(|(timestamp_ms, _)| *timestamp_ms) else {
        return Vec::new();
    };
    let last_timestamp = trade_pnls
        .last()
        .map(|(timestamp_ms, _)| *timestamp_ms)
        .unwrap_or(first_timestamp);

    let mut points = journal_leading_zero_points(first_timestamp, last_timestamp);
    let mut cumulative_pnl = 0.0;
    let mut idx = 0;
    while idx < trade_pnls.len() {
        let timestamp_ms = trade_pnls[idx].0;
        while idx < trade_pnls.len() && trade_pnls[idx].0 == timestamp_ms {
            cumulative_pnl += trade_pnls[idx].1;
            idx += 1;
        }
        if let Some(last) = points.last_mut()
            && last.0 == timestamp_ms
        {
            last.1 = cumulative_pnl;
        } else {
            points.push((timestamp_ms, cumulative_pnl));
        }
    }

    points
}

fn journal_leading_zero_points(first_timestamp: u64, last_timestamp: u64) -> Vec<(u64, f64)> {
    let active_span = last_timestamp.saturating_sub(first_timestamp);
    let requested_span = if first_timestamp > DAY_MS {
        active_span
            .saturating_mul(2)
            .clamp(DAY_MS, MAX_LEADING_ZERO_WINDOW_MS)
    } else {
        active_span
            .saturating_mul(2)
            .max(LEADING_ZERO_POINT_COUNT as u64)
    };
    let baseline_span = requested_span.min(first_timestamp.saturating_sub(1));
    if baseline_span == 0 {
        return vec![(first_timestamp.saturating_sub(1), 0.0)];
    }

    let step = (baseline_span / LEADING_ZERO_POINT_COUNT as u64).max(1);
    let mut points = Vec::with_capacity(LEADING_ZERO_POINT_COUNT + 1);
    for idx in (1..=LEADING_ZERO_POINT_COUNT).rev() {
        let timestamp = first_timestamp.saturating_sub(step.saturating_mul(idx as u64));
        if timestamp < first_timestamp
            && points
                .last()
                .is_none_or(|(last_timestamp, _)| *last_timestamp < timestamp)
        {
            points.push((timestamp, 0.0));
        }
    }

    let anchor_timestamp = first_timestamp.saturating_sub(1);
    if points
        .last()
        .is_none_or(|(last_timestamp, _)| *last_timestamp < anchor_timestamp)
    {
        points.push((anchor_timestamp, 0.0));
    }

    points
}
