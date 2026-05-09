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
    let whole = v.trunc() as u64;
    let frac = ((v.fract() * 100.0).round() as u64) % 100;
    let whole_str = whole.to_string();
    let mut result = String::with_capacity(whole_str.len() + whole_str.len() / 3 + 3);
    for (i, ch) in whole_str.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    let reversed: String = result.chars().rev().collect();
    format!("{reversed}.{frac:02}")
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
        format!("{price:.1}")
    } else if price.abs() >= 1.0 {
        format!("{price:.2}")
    } else {
        format!("{price:.4}")
    }
}
