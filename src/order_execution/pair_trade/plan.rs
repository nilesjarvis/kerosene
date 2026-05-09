// ---------------------------------------------------------------------------
// Pair Trade Planning Helpers
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;

pub(super) fn parse_pair_notional(raw: &str) -> Option<f64> {
    let value = raw.trim().parse::<f64>().ok()?;
    (value > 0.0 && value.is_finite()).then_some(value)
}

pub(super) fn pair_leg_sides(long_a_short_b: bool) -> (bool, bool) {
    (long_a_short_b, !long_a_short_b)
}

pub(super) fn pair_direction_label(symbol_a: &str, symbol_b: &str, long_a_short_b: bool) -> String {
    if long_a_short_b {
        format!("Long {symbol_a} / Short {symbol_b}")
    } else {
        format!("Short {symbol_a} / Long {symbol_b}")
    }
}

pub(super) fn missing_pair_mid_status(
    symbol_a: &str,
    symbol_b: &str,
    mid_a: f64,
    mid_b: f64,
    candidates_a: &[String],
    candidates_b: &[String],
) -> Option<String> {
    let mut missing = Vec::new();
    if !mid_a.is_finite() || mid_a <= 0.0 {
        missing.push(format!("A={symbol_a} (tried {})", candidates_a.join(", ")));
    }
    if !mid_b.is_finite() || mid_b <= 0.0 {
        missing.push(format!("B={symbol_b} (tried {})", candidates_b.join(", ")));
    }

    (!missing.is_empty())
        .then(|| format!("Missing mid prices for pair legs: {}", missing.join("; ")))
}
