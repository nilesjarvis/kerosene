// ---------------------------------------------------------------------------
// Order Input Helpers
// ---------------------------------------------------------------------------

use crate::helpers::parse_number;

#[cfg(test)]
mod tests;

pub(super) fn parse_positive_amount(input: &str) -> Option<f64> {
    let amount = parse_number(input)?;
    if amount.is_finite() && amount > 0.0 {
        Some(amount)
    } else {
        None
    }
}
