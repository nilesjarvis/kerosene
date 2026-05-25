use super::*;

#[test]
fn missing_rate_falls_back_to_usd_with_status() {
    let ctx = DisplayDenominationContext::from_mids(
        DisplayDenominationConfig::eur(),
        &HashMap::new(),
        &HashMap::new(),
        1_000,
    );

    assert!(ctx.is_fallback_usd());
    assert_eq!(ctx.format_value(125.0, 2), "$125.00");
    assert_eq!(ctx.format_chart_price(125.0), "125.00");
    assert_eq!(
        ctx.unavailable_status().as_deref(),
        Some("EUR rate unavailable; showing USD")
    );
}

#[test]
fn stale_rate_falls_back_to_usd_with_status() {
    let ctx = DisplayDenominationContext::from_mids(
        DisplayDenominationConfig::eur(),
        &HashMap::from([("xyz:EUR".to_string(), 1.25)]),
        &HashMap::from([("xyz:EUR".to_string(), 1_000)]),
        1_000 + DISPLAY_DENOMINATION_RATE_STALE_MS + 1,
    );

    assert!(ctx.is_fallback_usd());
    assert_eq!(ctx.format_value(125.0, 2), "$125.00");
    assert_eq!(ctx.format_chart_price(125.0), "125.00");
    assert_eq!(
        ctx.unavailable_status().as_deref(),
        Some("EUR rate stale; showing USD")
    );
}
