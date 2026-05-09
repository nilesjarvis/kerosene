pub(in crate::account_analytics::income) fn parse_f64_str(value: &str) -> Option<f64> {
    let parsed = value.trim().parse::<f64>().ok()?;
    parsed.is_finite().then_some(parsed)
}
