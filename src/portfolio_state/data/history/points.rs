use crate::helpers::{finite_value, positive_finite_value};

// ---------------------------------------------------------------------------
// Portfolio History Points
// ---------------------------------------------------------------------------

const MIN_RELIABLE_ACCOUNT_VALUE: f64 = 10.0;

pub(in crate::portfolio_state::data) fn apply_cutoff_with_baseline(
    points: &[(u64, f64)],
    cutoff: u64,
) -> Vec<(u64, f64)> {
    if points.is_empty() {
        return Vec::new();
    }

    let mut baseline: Option<f64> = None;
    let mut filtered: Vec<(u64, f64)> = Vec::new();
    for (ts, value) in points {
        if *ts <= cutoff {
            baseline = Some(*value);
        }
        if *ts >= cutoff {
            filtered.push((*ts, *value));
        }
    }

    if let Some(base) = baseline {
        if filtered.is_empty() {
            return vec![(cutoff, base)];
        }
        if filtered.first().is_some_and(|(ts, _)| *ts > cutoff) {
            filtered.insert(0, (cutoff, base));
        }
    }

    filtered
}

pub(in crate::portfolio_state::data::history) fn latest_account_value_at_or_before(
    points: &[(u64, f64)],
    timestamp_ms: u64,
) -> Option<f64> {
    points
        .iter()
        .take_while(|(ts, _)| *ts <= timestamp_ms)
        .last()
        .map(|(_, value)| *value)
}

pub(in crate::portfolio_state::data::history) fn sorted_finite_points(
    points: &[(u64, f64)],
) -> Vec<(u64, f64)> {
    let mut sorted = points
        .iter()
        .copied()
        .filter_map(|(ts, value)| finite_value(value).map(|value| (ts, value)))
        .collect::<Vec<_>>();
    sorted.sort_by_key(|(ts, _)| *ts);
    sorted
}

pub(in crate::portfolio_state::data::history) fn sorted_reliable_account_values(
    points: &[(u64, f64)],
) -> Vec<(u64, f64)> {
    let mut sorted = points
        .iter()
        .copied()
        .filter(|(_, value)| is_reliable_account_value(*value))
        .collect::<Vec<_>>();
    sorted.sort_by_key(|(ts, _)| *ts);
    sorted
}

fn is_reliable_account_value(value: f64) -> bool {
    positive_finite_value(value).is_some_and(|value| value > MIN_RELIABLE_ACCOUNT_VALUE)
}
