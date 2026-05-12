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
    format_grouped_decimal(v, 2)
}

fn format_grouped_decimal(value: f64, decimals: usize) -> String {
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
        format_grouped_decimal(price, 1)
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
}
