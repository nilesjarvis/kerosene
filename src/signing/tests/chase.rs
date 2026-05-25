use crate::signing::{ChaseLifecycle, ChaseOrder};

fn chase_order(agent_key: &str) -> ChaseOrder {
    ChaseOrder {
        id: 1,
        coin: "BTC".to_string(),
        account_address: "0xabc0000000000000000000000000000000000000".to_string(),
        agent_key: agent_key.to_string().into(),
        is_buy: true,
        target_size: 1.0,
        filled_size: 0.0,
        remaining_size: 1.0,
        known_oids: vec![42],
        current_cloid: None,
        place_attempt_count: 0,
        asset: 0,
        sz_decimals: 5,
        is_spot: false,
        reduce_only: false,
        current_oid: Some(42),
        current_price: 100.0,
        current_price_wire: "100".to_string(),
        initial_price: 100.0,
        started_at: std::time::Instant::now(),
        started_at_ms: 1_000,
        reprice_count: 0,
        lifecycle: ChaseLifecycle::Resting,
        last_reprice_at: None,
        desired_price: None,
        stop_reason: None,
        cancel_retries: 0,
    }
}

#[test]
fn chase_place_cloid_is_stable_unique_128_bit_hex() {
    let first = super::super::chase_place_cloid("0xabc", 7, 1_000, 1);
    let same = super::super::chase_place_cloid("0xabc", 7, 1_000, 1);
    let next_attempt = super::super::chase_place_cloid("0xabc", 7, 1_000, 2);

    assert_eq!(first, same);
    assert_ne!(first, next_attempt);
    assert_eq!(first.len(), 34);
    assert!(first.starts_with("0x"));
    assert!(first[2..].chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn chase_order_debug_redacts_agent_key() {
    let chase = chase_order("super-secret-agent-key");
    let rendered = format!("{chase:?}");

    assert!(!rendered.contains("super-secret-agent-key"));
    assert!(rendered.contains("<redacted>"));
}

#[test]
fn chase_price_moves_only_toward_fill() {
    let mut chase = chase_order("agent-key");

    assert!(chase.price_moves_toward_fill(100.1));
    assert!(!chase.price_moves_toward_fill(99.9));

    chase.is_buy = false;
    assert!(chase.price_moves_toward_fill(99.9));
    assert!(!chase.price_moves_toward_fill(100.1));
}
