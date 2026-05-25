use crate::helpers::{finite_value, positive_finite_value};

// ---------------------------------------------------------------------------
// HYPE ETF Number Parsing
// ---------------------------------------------------------------------------

pub(super) fn finite(value: Option<f64>) -> Option<f64> {
    value.and_then(finite_value)
}

pub(super) fn finite_positive(value: Option<f64>) -> Option<f64> {
    value.and_then(positive_finite_value)
}

pub(super) fn positive_ratio(numerator: f64, denominator: f64) -> Option<f64> {
    let numerator = finite(Some(numerator))?;
    let denominator = positive_finite_value(denominator)?;
    Some(numerator / denominator)
}

#[cfg(test)]
mod tests;
