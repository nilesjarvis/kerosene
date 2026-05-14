// ---------------------------------------------------------------------------
// Order Input Helpers
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;

pub(super) fn parse_positive_amount(input: &str) -> Option<f64> {
    let amount = input.trim().parse::<f64>().ok()?;
    if amount.is_finite() && amount > 0.0 {
        Some(amount)
    } else {
        None
    }
}
