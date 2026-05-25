#[cfg(test)]
mod tests;

use super::formatting::{finite_value, positive_finite_value};

// ---------------------------------------------------------------------------
// Optional Aggregation
// ---------------------------------------------------------------------------

pub(crate) fn add_optional_f64(total: &mut Option<f64>, value: Option<f64>) {
    *total = total.and_then(|total| value.map(|value| total + value));
}

pub(crate) fn sum_optional_f64(values: impl IntoIterator<Item = Option<f64>>) -> Option<f64> {
    let mut total = Some(0.0);
    for value in values {
        add_optional_f64(&mut total, value);
    }
    total
}

pub(crate) fn positive_percent_change(current: Option<f64>, previous: Option<f64>) -> Option<f64> {
    let current = positive_finite_value(current?)?;
    let previous = positive_finite_value(previous?)?;
    let change = (current - previous) / previous * 100.0;
    finite_value(change)
}
