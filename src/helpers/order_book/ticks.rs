#[cfg(test)]
mod tests;

use super::super::formatting::positive_finite_value;

pub fn valid_book_tick_size(tick: f64) -> bool {
    positive_finite_value(tick).is_some()
}

/// Number of decimal places needed to display a given tick size.
pub fn tick_decimals(tick: f64) -> usize {
    if !valid_book_tick_size(tick) {
        return 2;
    }
    let log = -(tick.log10().floor() as isize);
    if log <= 0 { 0 } else { log as usize }
}

/// Compute a sensible default tick size for a given mid price.
/// Aims for ~0.01% of the price, rounded to a clean power of 10.
pub fn default_tick_for_price(mid_price: f64) -> f64 {
    if positive_finite_value(mid_price).is_none() {
        return 0.01;
    }
    let raw = mid_price * 0.0001;
    let log = raw.log10().floor();
    10f64.powf(log)
}

/// Compute dynamic tick size options based on the current mid price.
/// Returns 5 geometrically spaced options centered around the default tick.
pub fn book_tick_options(mid_price: f64) -> Vec<f64> {
    let base = default_tick_for_price(mid_price);
    vec![base, base * 5.0, base * 10.0, base * 50.0, base * 100.0]
}

pub fn compute_sigfigs(tick_size: f64, mid_price: f64) -> (Option<u8>, Option<u8>) {
    if positive_finite_value(mid_price).is_none() || !valid_book_tick_size(tick_size) {
        return (None, None);
    }

    let e = mid_price.log10().floor() as i32;

    let mut best_n = None;
    let mut best_m = None;
    let mut best_server_tick = 0.0;

    for n in [5, 4, 3, 2] {
        let allowed_m = if n == 5 { vec![1, 2, 5] } else { vec![1] };
        for m in allowed_m {
            let server_tick = (m as f64) * 10f64.powi(e - n + 1);
            if server_tick <= tick_size * 1.0001 && server_tick > best_server_tick {
                best_server_tick = server_tick;
                best_n = Some(n as u8);
                best_m = if n == 5 && m != 1 {
                    Some(m as u8)
                } else {
                    None
                };
            }
        }
    }

    (best_n, best_m)
}

pub fn sigfig_server_tick(sigfigs: (Option<u8>, Option<u8>), mid_price: f64) -> Option<f64> {
    let n = sigfigs.0?;
    positive_finite_value(mid_price)?;

    let mantissa = sigfigs.1.unwrap_or(1) as f64;
    let exponent = mid_price.log10().floor() as i32 - n as i32 + 1;
    let tick = mantissa * 10f64.powi(exponent);
    valid_book_tick_size(tick).then_some(tick)
}

/// The option closest to `tick` on a log scale, for snapping a stored tick
/// onto the current option set after the price regime moved. Returns `tick`
/// unchanged if the option list is empty or the tick is invalid.
pub fn nearest_tick_option(options: &[f64], tick: f64) -> f64 {
    if !valid_book_tick_size(tick) {
        return options.first().copied().unwrap_or(tick);
    }
    options
        .iter()
        .copied()
        .filter(|opt| valid_book_tick_size(*opt))
        .min_by(|a, b| {
            let da = (a.ln() - tick.ln()).abs();
            let db = (b.ln() - tick.ln()).abs();
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(tick)
}

pub fn tick_sizes_match(a: f64, b: f64) -> bool {
    valid_book_tick_size(a) && valid_book_tick_size(b) && (a - b).abs() / a.max(b).max(1e-12) < 0.01
}

/// Format a tick size for display in the selector buttons.
pub fn format_tick(tick: f64) -> String {
    if !valid_book_tick_size(tick) {
        return "-".to_string();
    }
    if tick >= 100.0 {
        format!("{tick:.0}")
    } else if tick >= 1.0 {
        // Show as integer if whole, otherwise 1 decimal
        if (tick - tick.round()).abs() < 1e-9 {
            format!("{tick:.0}")
        } else {
            format!("{tick:.1}")
        }
    } else {
        let decimals = tick_decimals(tick);
        format!("{tick:.decimals$}")
    }
}
