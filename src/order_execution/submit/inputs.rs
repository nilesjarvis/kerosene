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

pub(super) fn order_quantity_from_input(
    raw_quantity: f64,
    price: f64,
    quantity_is_usd: bool,
) -> Option<f64> {
    if !raw_quantity.is_finite() || raw_quantity <= 0.0 {
        return None;
    }

    if !quantity_is_usd {
        return Some(raw_quantity);
    }

    if price.is_finite() && price > 0.0 {
        Some(raw_quantity / price)
    } else {
        None
    }
}
