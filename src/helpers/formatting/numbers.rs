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

    parsed.is_finite().then_some(parsed)
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
        format!("{price:.4}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn price_formatter_groups_large_prices() {
        assert_eq!(format_price(1_234.56), "1,234.6");
        assert_eq!(format_price(100_000.0), "100,000.0");
        assert_eq!(format_price(-12_345.67), "-12,345.7");
    }

    #[test]
    fn price_formatter_keeps_existing_precision_bands_below_thousand() {
        assert_eq!(format_price(999.99), "999.99");
        assert_eq!(format_price(0.123456), "0.1235");
    }

    #[test]
    fn grouped_decimal_formatter_keeps_requested_precision() {
        assert_eq!(format_decimal_with_commas(12_345.6789, 3), "12,345.679");
        assert_eq!(format_decimal_with_commas(12_345.0, 0), "12,345");
    }

    #[test]
    fn number_parser_accepts_grouped_values() {
        assert_eq!(parse_number("12,345.67"), Some(12_345.67));
        assert_eq!(parse_number("1,234,567"), Some(1_234_567.0));
        assert_eq!(parse_number("-1,234.50"), Some(-1_234.5));
        assert_eq!(parse_number("+1,234.50"), Some(1_234.5));
        assert_eq!(parse_number(""), None);
    }

    #[test]
    fn number_parser_rejects_malformed_grouped_values() {
        assert_eq!(parse_number("1,2"), None);
        assert_eq!(parse_number("1,,000"), None);
        assert_eq!(parse_number("12,34.56"), None);
        assert_eq!(parse_number("1234,567"), None);
        assert_eq!(parse_number(",123"), None);
        assert_eq!(parse_number("1,234.5,6"), None);
        assert_eq!(parse_number("1,234e2"), None);
    }

    #[test]
    fn number_parser_rejects_nonfinite_values() {
        assert_eq!(parse_number("NaN"), None);
        assert_eq!(parse_number("inf"), None);
        assert_eq!(parse_number("1e309"), None);
    }
}
