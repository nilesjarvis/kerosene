pub(in crate::assistant::planning) fn parse_lookback_days(prompt: &str) -> Option<u32> {
    let p = prompt.to_lowercase();
    if p.contains("last week") || p.contains("past week") {
        return Some(7);
    }
    if p.contains("last month") || p.contains("past month") {
        return Some(30);
    }
    None
}

pub(in crate::assistant::planning) fn parse_usd_amount(prompt: &str) -> Option<f64> {
    for token in prompt.split_whitespace() {
        if let Some(rest) = token.strip_prefix('$') {
            let cleaned = rest
                .trim_matches(|c: char| c == ',' || c == '.' || c == ';' || c == '!' || c == '?');
            if cleaned.is_empty() {
                continue;
            }
            if let Ok(v) = cleaned.replace(',', "").parse::<f64>() {
                return Some(v);
            }
        }
    }
    None
}

pub(in crate::assistant::planning) fn default_liq_range(latest_price: Option<f64>) -> (f64, f64) {
    let px = latest_price.unwrap_or(1.0).max(1e-6);
    (px * 0.5, px * 1.5)
}
