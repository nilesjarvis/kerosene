use super::finite_sorted_points;

// ---------------------------------------------------------------------------
// Account Value Series
// ---------------------------------------------------------------------------

pub(in crate::journal_views::summary::chart) fn account_value_points_for_range(
    points: &[(u64, f64)],
    start_ms: u64,
    end_ms: u64,
) -> Vec<(u64, f64)> {
    if end_ms <= start_ms {
        return Vec::new();
    }

    let mut sorted = finite_sorted_points(points);
    if sorted.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    if let Some((_, value)) = sorted
        .iter()
        .rev()
        .find(|(timestamp_ms, _)| *timestamp_ms <= start_ms)
    {
        out.push((start_ms, *value));
    }

    out.extend(
        sorted
            .drain(..)
            .filter(|(timestamp_ms, _)| *timestamp_ms > start_ms && *timestamp_ms <= end_ms),
    );

    out
}
