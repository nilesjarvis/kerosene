use serde_json::Value;

/// Format a USD string from the API.
pub fn format_usd(s: &str) -> String {
    match s.parse::<f64>() {
        Ok(v) => {
            let sign = if v < 0.0 { "-" } else { "" };
            let abs = v.abs();
            if abs >= 1_000_000.0 {
                format!("{}${:.2}M", sign, abs / 1_000_000.0)
            } else {
                format!("{}${}", sign, format_with_commas(abs))
            }
        }
        Err(_) => s.to_string(),
    }
}

/// Format a non-negative float with 2 decimal places and comma thousands separators.
pub fn format_with_commas(v: f64) -> String {
    format_decimal_with_commas(v, 2)
}

pub fn format_decimal_with_commas(value: f64, decimals: usize) -> String {
    let sign = if value < 0.0 { "-" } else { "" };
    let formatted = format!("{:.*}", decimals, value.abs());
    let (whole, fraction) = formatted
        .split_once('.')
        .map_or((formatted.as_str(), ""), |(whole, fraction)| {
            (whole, fraction)
        });

    let mut grouped = String::with_capacity(whole.len() + whole.len() / 3);
    for (i, ch) in whole.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let whole_grouped: String = grouped.chars().rev().collect();
    if decimals == 0 {
        format!("{sign}{whole_grouped}")
    } else {
        format!("{sign}{whole_grouped}.{fraction}")
    }
}

pub fn trim_decimal_zeros(value: String) -> String {
    let Some((whole, fraction)) = value.rsplit_once('.') else {
        return value;
    };
    let fraction = fraction.trim_end_matches('0');
    if fraction.is_empty() {
        whole.to_string()
    } else {
        format!("{whole}.{fraction}")
    }
}

pub fn normalize_two_decimal_display_value(value: f64) -> f64 {
    if value.abs() < 0.005 { 0.0 } else { value }
}

pub fn format_signed_percent_value(value: f64) -> String {
    let display_value = normalize_two_decimal_display_value(value);
    if display_value > 0.0 {
        format!("+{display_value:.2}%")
    } else {
        format!("{display_value:.2}%")
    }
}

pub fn parse_number(input: &str) -> Option<f64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let parsed = if trimmed.contains(',') {
        if !has_valid_thousands_grouping(trimmed) {
            return None;
        }

        trimmed.replace(',', "").parse::<f64>().ok()?
    } else {
        trimmed.parse::<f64>().ok()?
    };

    finite_value(parsed)
}

pub fn parse_positive_number(input: &str) -> Option<f64> {
    parse_number(input).filter(|value| *value > 0.0)
}

pub fn finite_value(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

pub fn positive_finite_value(value: f64) -> Option<f64> {
    finite_value(value).filter(|value| *value > 0.0)
}

pub fn parse_finite_number(input: &str) -> Option<f64> {
    finite_value(input.trim().parse::<f64>().ok()?)
}

pub fn parse_finite_json_number(value: &Value) -> Option<f64> {
    if let Some(text) = value.as_str() {
        parse_finite_number(text)
    } else {
        value.as_f64().and_then(finite_value)
    }
}

pub fn parse_positive_finite_number(input: &str) -> Option<f64> {
    parse_finite_number(input).filter(|value| *value > 0.0)
}

/// Wire strings for the same price/size can format differently (e.g. "100"
/// vs "100.0"), so compare parsed values with a small relative tolerance.
pub fn values_match_approx(a: f64, b: f64) -> bool {
    (a - b).abs() <= a.abs().max(b.abs()) * 1e-9
}

pub fn invalid_data_placeholder() -> String {
    "Invalid data".to_string()
}

pub fn not_available_placeholder() -> String {
    "n/a".to_string()
}

fn has_valid_thousands_grouping(input: &str) -> bool {
    if input.contains('e') || input.contains('E') {
        return false;
    }

    let unsigned = input
        .strip_prefix('+')
        .or_else(|| input.strip_prefix('-'))
        .unwrap_or(input);
    let (whole, fraction) = match unsigned.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (unsigned, ""),
    };

    if fraction.contains(',') || !fraction.chars().all(|ch| ch.is_ascii_digit()) {
        return false;
    }

    let mut groups = whole.split(',');
    let Some(first) = groups.next() else {
        return false;
    };
    if first.is_empty() || first.len() > 3 || !first.chars().all(|ch| ch.is_ascii_digit()) {
        return false;
    }

    let mut saw_separator = false;
    for group in groups {
        saw_separator = true;
        if group.len() != 3 || !group.chars().all(|ch| ch.is_ascii_digit()) {
            return false;
        }
    }

    saw_separator
}

pub fn format_size(size: f64) -> String {
    if size >= 1_000_000.0 {
        format!("{:.1}M", size / 1_000_000.0)
    } else if size >= 10_000.0 {
        format!("{:.1}K", size / 1_000.0)
    } else if size >= 100.0 {
        format!("{size:.1}")
    } else if size >= 1.0 {
        format!("{size:.2}")
    } else {
        format!("{size:.4}")
    }
}

pub fn format_price(price: f64) -> String {
    if price.abs() >= 1000.0 {
        format_decimal_with_commas(price, 1)
    } else if price.abs() >= 1.0 {
        format!("{price:.2}")
    } else {
        format!("{price:.*}", small_price_decimals(price))
    }
}

/// Format a price as a plain decimal string suitable for order-input
/// prefills: no thousands grouping, at least four decimals, and enough
/// significant figures for low-priced spot markets.
pub fn format_price_input(price: f64) -> String {
    format!("{price:.*}", small_price_decimals(price))
}

/// Decimal places for sub-1.0 prices: keep at least four significant figures
/// so low-priced spot tokens (which legally carry up to 8 decimal places on
/// Hyperliquid) do not collapse to "0.0000", while prices at or above 0.1
/// keep the historical fixed four decimals.
fn small_price_decimals(price: f64) -> usize {
    const MIN_DECIMALS: usize = 4;
    // Hyperliquid prices never carry more than 8 decimal places.
    const MAX_DECIMALS: usize = 8;
    const SIGNIFICANT_FIGURES: i32 = 4;

    let abs = price.abs();
    if !abs.is_finite() || abs <= 0.0 || abs >= 1.0 {
        return MIN_DECIMALS;
    }

    // -1 for 0.1234, -3 for 0.001234, -5 for 0.00001234.
    let magnitude = abs.log10().floor() as i32;
    let decimals = SIGNIFICANT_FIGURES - 1 - magnitude;
    usize::try_from(decimals)
        .unwrap_or(MIN_DECIMALS)
        .clamp(MIN_DECIMALS, MAX_DECIMALS)
}

#[cfg(test)]
mod tests;
