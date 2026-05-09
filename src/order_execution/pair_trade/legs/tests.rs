use super::*;

#[test]
fn pair_leg_order_sizes_by_notional_and_prices_buy_with_slippage() {
    let leg = build_pair_leg_order("BTC".to_string(), 0, 5, 100.0, 1_000.0, true, 0.05)
        .expect("valid leg");

    assert_eq!(leg.coin, "BTC");
    assert_eq!(leg.asset, 0);
    assert!(leg.is_buy);
    assert_eq!(leg.price, "105");
    assert_eq!(leg.size, "10");
}

#[test]
fn pair_leg_order_prices_sell_with_slippage() {
    let leg = build_pair_leg_order("ETH".to_string(), 1, 4, 50.0, 1_000.0, false, 0.05)
        .expect("valid leg");

    assert_eq!(leg.price, "47.5");
    assert_eq!(leg.size, "20");
}

#[test]
fn pair_leg_order_rejects_invalid_mid_or_notional() {
    assert!(build_pair_leg_order("BTC".to_string(), 0, 5, 0.0, 1_000.0, true, 0.01).is_none());
    assert!(build_pair_leg_order("BTC".to_string(), 0, 5, 100.0, 0.0, true, 0.01).is_none());
    assert!(
        build_pair_leg_order("BTC".to_string(), 0, 5, 100.0, 1_000.0, true, f64::NAN).is_none()
    );
    assert!(build_pair_leg_order("BTC".to_string(), 0, 5, 100.0, 1_000.0, true, -0.01).is_none());
}
