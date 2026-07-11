use crate::ws::TrackedTradeEvent;

use super::TrackedTradeAggregationKey;

fn event() -> TrackedTradeEvent {
    TrackedTradeEvent {
        address: "0xabc".to_string(),
        coin: "HYPE".to_string(),
        price: 10.0,
        size: 1.0,
        is_buy: true,
        time_ms: 1_000,
        dir: "Open Long".to_string(),
        start_position: Some(0.0),
        closed_pnl: 0.0,
        fee: 0.01,
        fee_token: "USDC".to_string(),
        tid: Some(7),
        hash: "0xhash".to_string(),
        oid: Some(9),
        tx_index: 3,
    }
}

#[test]
fn aggregation_key_prefers_order_id_over_hash() {
    let trade = event();

    assert_eq!(
        TrackedTradeAggregationKey::from_event(&trade),
        TrackedTradeAggregationKey::Order {
            address: "0xabc",
            coin: "HYPE",
            is_buy: true,
            oid: 9
        }
    );
}

#[test]
fn aggregation_key_uses_hash_without_order_id() {
    let mut trade = event();
    trade.oid = None;

    assert_eq!(
        TrackedTradeAggregationKey::from_event(&trade),
        TrackedTradeAggregationKey::Hash {
            address: "0xabc",
            coin: "HYPE",
            is_buy: true,
            hash: "0xhash"
        }
    );
}

#[test]
fn aggregation_key_falls_back_to_time_window_when_hash_is_blank() {
    let mut trade = event();
    trade.oid = None;
    trade.hash = "  ".to_string();

    assert_eq!(
        TrackedTradeAggregationKey::from_event(&trade),
        TrackedTradeAggregationKey::TimeWindow {
            address: "0xabc",
            coin: "HYPE",
            is_buy: true,
            dir: "Open Long"
        }
    );
}

#[test]
fn aggregation_key_debug_redacts_account_and_order_identity() {
    let trade = event();
    let key = TrackedTradeAggregationKey::from_event(&trade);

    let rendered = format!("{key:?}");

    assert!(rendered.contains("<redacted>"), "{rendered}");
    assert!(rendered.contains("is_buy: true"), "{rendered}");
    for sensitive in ["0xabc", "HYPE", "oid: 9", "0xhash"] {
        assert!(!rendered.contains(sensitive), "{rendered}");
    }
    assert_eq!(
        key,
        TrackedTradeAggregationKey::Order {
            address: "0xabc",
            coin: "HYPE",
            is_buy: true,
            oid: 9,
        }
    );
}
