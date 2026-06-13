use super::super::nuke_prepared_order;
use super::{
    DEFAULT_MARKET_SLIPPAGE, build_nuke_position_order, order_or_panic, parse_nuke_position_size,
};
use crate::api::MarketType;
use crate::order_execution::OrderSurface;
use crate::signing::ExchangeOrderKind;

#[test]
fn nuke_order_closes_long_with_sell_market_price() {
    let order = order_or_panic(
        build_nuke_position_order(7, 4, 100.0, 2.5, DEFAULT_MARKET_SLIPPAGE),
        "valid order",
    );

    assert_eq!(order.asset, 7);
    assert!(!order.is_buy);
    assert_eq!(order.price, "99");
    assert_eq!(order.size, "2.5");
}

#[test]
fn nuke_order_closes_short_with_buy_market_price() {
    let order = order_or_panic(
        build_nuke_position_order(8, 4, 100.0, -2.5, DEFAULT_MARKET_SLIPPAGE),
        "valid order",
    );

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

#[test]
fn nuke_order_converts_to_cloid_backed_reduce_only_market_request() {
    let order = order_or_panic(
        build_nuke_position_order(7, 4, 100.0, 2.5, DEFAULT_MARKET_SLIPPAGE),
        "valid order",
    );

    let prepared = nuke_prepared_order("BTC".to_string(), order);
    assert_eq!(prepared.surface, OrderSurface::Nuke);
    assert_eq!(prepared.symbol_key, "BTC");
    assert_eq!(prepared.order_kind, ExchangeOrderKind::Market);
    assert!(prepared.reduce_only);
    assert_eq!(prepared.market_type, MarketType::Perp);

    let (request, context) = prepared.place_request_with_context("0xabc");
    assert_eq!(request.asset, 7);
    assert!(!request.is_buy);
    assert_eq!(request.price, "99");
    assert_eq!(request.size, "2.5");
    assert_eq!(request.order_kind, ExchangeOrderKind::Market);
    assert!(request.reduce_only);
    let cloid = request.cloid.expect("NUKE request should have cloid");
    assert_eq!(context.cloid, cloid);
    assert_eq!(context.account_address, "0xabc");
    assert_eq!(context.surface, OrderSurface::Nuke);
    assert_eq!(context.symbol_key, "BTC");
    assert_eq!(cloid.len(), 34);
    assert!(cloid.starts_with("0x"));
    assert!(cloid[2..].chars().all(|ch| ch.is_ascii_hexdigit()));
}
