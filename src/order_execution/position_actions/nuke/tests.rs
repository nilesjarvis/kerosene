use super::{build_nuke_position_order, parse_nuke_position_size};
use crate::order_execution::pricing::DEFAULT_MARKET_SLIPPAGE_PCT;

const DEFAULT_MARKET_SLIPPAGE: f64 = DEFAULT_MARKET_SLIPPAGE_PCT / 100.0;

#[test]
fn nuke_order_closes_long_with_sell_market_price() {
    let order =
        build_nuke_position_order(7, 4, 100.0, 2.5, DEFAULT_MARKET_SLIPPAGE).expect("valid order");

    assert_eq!(order.asset, 7);
    assert!(!order.is_buy);
    assert_eq!(order.price, "99");
    assert_eq!(order.size, "2.5");
}

#[test]
fn nuke_order_closes_short_with_buy_market_price() {
    let order =
        build_nuke_position_order(8, 4, 100.0, -2.5, DEFAULT_MARKET_SLIPPAGE).expect("valid order");

    assert_eq!(order.asset, 8);
    assert!(order.is_buy);
    assert_eq!(order.price, "101");
    assert_eq!(order.size, "2.5");
}

#[test]
fn nuke_order_rejects_zero_or_nonfinite_inputs() {
    assert!(build_nuke_position_order(7, 4, 100.0, 0.0, DEFAULT_MARKET_SLIPPAGE).is_none());
    assert!(build_nuke_position_order(7, 4, 0.0, 2.5, DEFAULT_MARKET_SLIPPAGE).is_none());
    assert!(build_nuke_position_order(7, 4, f64::NAN, 2.5, DEFAULT_MARKET_SLIPPAGE).is_none());
    assert!(
        build_nuke_position_order(7, 4, 100.0, f64::INFINITY, DEFAULT_MARKET_SLIPPAGE).is_none()
    );
    assert!(build_nuke_position_order(7, 4, 100.0, 2.5, f64::NAN).is_none());
    assert!(build_nuke_position_order(7, 4, 100.0, 2.5, -0.01).is_none());
}

#[test]
fn nuke_position_size_parser_rejects_malformed_sizes_instead_of_zeroing_them() {
    assert_eq!(parse_nuke_position_size("BTC", "2.5"), Ok(Some(2.5)));
    assert_eq!(parse_nuke_position_size("BTC", "0"), Ok(None));

    assert!(parse_nuke_position_size("BTC", "not-a-number").is_err());
    assert!(parse_nuke_position_size("BTC", "NaN").is_err());
    assert!(parse_nuke_position_size("BTC", "inf").is_err());
}
