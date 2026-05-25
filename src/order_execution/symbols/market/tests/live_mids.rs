use super::*;

#[test]
fn valid_mid_price_accepts_only_positive_finite_values() {
    assert!(valid_mid_price(1.0));
    assert!(!valid_mid_price(0.0));
    assert!(!valid_mid_price(-1.0));
    assert!(!valid_mid_price(f64::NAN));
    assert!(!valid_mid_price(f64::INFINITY));
}

#[test]
fn live_mid_resolution_accepts_fresh_positive_finite_candidates() {
    let candidates = vec!["BTC".to_string()];
    let all_mids = HashMap::from([("BTC".to_string(), 100.0)]);
    let updated_at = HashMap::from([("BTC".to_string(), 10_000)]);

    assert_eq!(
        resolve_live_mid_from_candidates(&candidates, &all_mids, &updated_at, 10_001),
        Some(100.0)
    );
}

#[test]
fn live_mid_resolution_rejects_stale_missing_or_future_timestamps() {
    let now_ms = LIVE_MID_MAX_AGE_MS + 10_000;
    let candidates = vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string()];
    let all_mids = HashMap::from([
        ("BTC".to_string(), 100.0),
        ("ETH".to_string(), 200.0),
        ("SOL".to_string(), 300.0),
    ]);
    let updated_at = HashMap::from([
        ("BTC".to_string(), now_ms - LIVE_MID_MAX_AGE_MS - 1),
        ("SOL".to_string(), now_ms + 1),
    ]);

    assert_eq!(
        resolve_live_mid_from_candidates(&candidates, &all_mids, &updated_at, now_ms),
        None
    );
}

#[test]
fn live_mid_resolution_uses_later_candidate_when_first_is_stale() {
    let now_ms = LIVE_MID_MAX_AGE_MS + 10_000;
    let candidates = vec!["BTC".to_string(), "UBTC".to_string()];
    let all_mids = HashMap::from([("BTC".to_string(), 100.0), ("UBTC".to_string(), 101.0)]);
    let updated_at = HashMap::from([
        ("BTC".to_string(), now_ms - LIVE_MID_MAX_AGE_MS - 1),
        ("UBTC".to_string(), now_ms),
    ]);

    assert_eq!(
        resolve_live_mid_from_candidates(&candidates, &all_mids, &updated_at, now_ms),
        Some(101.0)
    );
}

#[test]
fn refresh_order_price_for_symbol_seeds_limit_ioc_price_from_mid() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.order_kind = OrderKind::LimitIoc;
    terminal.order_price = "1".to_string();
    terminal.all_mids.insert("BTC".to_string(), 101.25);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    terminal.refresh_order_price_for_symbol("BTC");

    assert_eq!(terminal.order_price, format_price(101.25));
}
