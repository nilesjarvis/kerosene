pub(in crate::assistant::planning) fn sanitize_interval(raw: &str) -> String {
    match raw.trim().to_lowercase().as_str() {
        "1m" | "m1" => "1m".to_string(),
        "5m" | "m5" => "5m".to_string(),
        "15m" | "m15" => "15m".to_string(),
        "1h" | "h1" => "1h".to_string(),
        "4h" | "h4" => "4h".to_string(),
        "1d" | "d1" => "1d".to_string(),
        _ => "1h".to_string(),
    }
}
