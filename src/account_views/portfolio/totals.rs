use crate::helpers::finite_value;

pub(super) use crate::helpers::format_signed_percent_value;

// ---------------------------------------------------------------------------
// Totals
// ---------------------------------------------------------------------------

pub(super) fn portfolio_total_performance(points: &[(u64, f64)]) -> Option<f64> {
    points.last().and_then(|(_, value)| finite_value(*value))
}

pub(super) fn portfolio_total_pnl(points: &[(u64, f64)]) -> Option<f64> {
    match points {
        [] => None,
        [(_, only)] => finite_value(*only),
        points => {
            let first = points.first().map(|(_, value)| *value)?;
            let last = points.last().map(|(_, value)| *value)?;
            let total = last - first;
            finite_value(total)
        }
    }
}
