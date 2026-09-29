use super::*;
use crate::api::MarketType;

#[test]
fn one_shot_place_cloid_is_stable_128_bit_hex_for_same_inputs() {
    let order = PreparedExchangeOrder {
        surface: OrderSurface::Ticket,
        symbol_key: "BTC".to_string(),
        asset: 0,
        is_buy: true,
        price: "100".to_string(),
        size: "1".to_string(),
        order_kind: ExchangeOrderKind::Limit,
        reduce_only: false,
        market_type: MarketType::Perp,
    };

    let first = one_shot_place_cloid("0xabc", 1_000, &order);
    let same = one_shot_place_cloid("0xabc", 1_000, &order);
    let next = one_shot_place_cloid("0xabc", 1_001, &order);

    assert_eq!(first, same);
    assert_ne!(first, next);
    assert_eq!(first.len(), 34);
    assert!(first.starts_with("0x"));
    assert!(first[2..].chars().all(|ch| ch.is_ascii_hexdigit()));
}

#[test]
fn one_shot_cloid_nonce_allocator_is_monotonic() {
    let nonce = AtomicU64::new(0);

    let first = allocate_one_shot_cloid_nonce_from(&nonce, 10);
    let second = allocate_one_shot_cloid_nonce_from(&nonce, 10);
    let third = allocate_one_shot_cloid_nonce_from(&nonce, 9);

    assert_eq!(first, 10);
    assert_eq!(second, 11);
    assert_eq!(third, 12);
}
