/// Convert a float to the wire format string matching the Python SDK's `float_to_wire`.
/// Rounds to 8 decimal places, then strips trailing zeros.
pub fn float_to_wire(x: f64) -> String {
    let rounded = format!("{x:.8}");
    let back: f64 = rounded.parse().unwrap_or(x);
    if (back - x).abs() >= 1e-12 {
        return format!("{x}");
    }
    if rounded.contains('.') {
        let trimmed = rounded.trim_end_matches('0').trim_end_matches('.');
        if trimmed.is_empty() || trimmed == "-0" {
            "0".to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        rounded
    }
}

/// Round a price to 5 significant figures, matching Python's `f"{px:.5g}"`.
/// Then clamp to `max_decimals` decimal places:
///   - Perps: `6 - sz_decimals`
///   - Spot:  `8 - sz_decimals`
pub fn round_price(px: f64, sz_decimals: u32, is_spot: bool) -> f64 {
    if px == 0.0 {
        return 0.0;
    }
    let magnitude = px.abs().log10().floor() as i32;
    let sig_decimals = (4 - magnitude).max(0) as usize;
    let factor = 10f64.powi(sig_decimals as i32);
    let sig5 = (px * factor).round() / factor;

    let base = if is_spot { 8u32 } else { 6u32 };
    let max_decimals = base.saturating_sub(sz_decimals);
    let clamp_factor = 10f64.powi(max_decimals as i32);
    (sig5 * clamp_factor).round() / clamp_factor
}
