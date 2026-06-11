use super::*;
use crate::account::UserFill;
use crate::signing::{ChaseLifecycle, ChaseOrder};

#[test]
fn chase_history_uses_fill_vwap_not_last_working_price() {
    let mut chase = chase_order();
    chase.target_size = 1.0;
    chase.filled_size = 0.75;
    chase.remaining_size = 0.25;
    chase.current_price = 101.0;
    chase.known_oids = vec![42];
    chase.current_oid = Some(43);

    let fills = vec![
        fill(42, 1_000, "100", "0.25", "0.01", "0"),
        fill(43, 2_000, "110", "0.5", "-0.002", "1.2"),
        fill(43, 2_000, "110", "0.5", "-0.002", "1.2"),
        fill(99, 3_000, "999", "9", "9", "9"),
    ];
    let metrics = AdvancedOrderHistoryEntry::chase_fill_metrics(&fills, &chase)
        .expect("matching chase fills");

    let entry = AdvancedOrderHistoryEntry::from_chase_with_fill_metrics(
        &chase,
        "BTC".to_string(),
        10_000,
        "Chase filled".to_string(),
        Some(metrics),
    );

    assert_eq!(entry.kind, AdvancedOrderHistoryKind::Chase);
    assert_eq!(entry.average_price, Some(80.0 / 0.75));
    assert_eq!(entry.last_working_price, Some(101.0));
    assert_eq!(entry.filled_size, 0.75);
    assert_eq!(entry.remaining_size, 0.25);
    assert_eq!(entry.gross_notional, 80.0);
    assert!((entry.total_fee - 0.008).abs() < f64::EPSILON);
    assert_eq!(entry.closed_pnl, 1.2);
}

#[test]
fn chase_history_does_not_guess_average_without_authoritative_fills() {
    let mut chase = chase_order();
    chase.filled_size = 1.0;
    chase.remaining_size = 0.0;
    chase.current_price = 123.0;

    let entry = AdvancedOrderHistoryEntry::from_chase_with_fill_metrics(
        &chase,
        "BTC".to_string(),
        10_000,
        "Done".to_string(),
        None,
    );

    assert_eq!(entry.average_price, None);
    assert_eq!(entry.last_working_price, Some(123.0));
    assert_eq!(entry.gross_notional, 0.0);
}

#[test]
fn chase_history_persists_resolved_display_coin_not_raw_key() {
    let mut chase = chase_order();
    chase.coin = "#660".to_string();

    let entry = AdvancedOrderHistoryEntry::from_chase_with_fill_metrics(
        &chase,
        "YES: BTC above 75348".to_string(),
        10_000,
        "Done".to_string(),
        None,
    );

    assert_eq!(entry.coin, "#660");
    assert_eq!(entry.display_coin, "YES: BTC above 75348");
}

fn chase_order() -> ChaseOrder {
    let now = std::time::Instant::now();
    ChaseOrder {
        id: 1,
        coin: "BTC".to_string(),
        account_address: "0xabc".to_string(),
        agent_key: "key".to_string().into(),
        is_buy: true,
        target_size: 1.0,
        filled_size: 0.0,
        remaining_size: 1.0,
        known_oids: Vec::new(),
        current_cloid: None,
        place_attempt_count: 0,
        asset: 0,
        sz_decimals: 3,
        is_spot: false,
        reduce_only: false,
        current_oid: None,
        current_price: 100.0,
        current_price_wire: "100".to_string(),
        initial_price: 100.0,
        started_at: now,
        started_at_ms: 1_000,
        fill_cutoff_ms_by_oid: Vec::new(),
        reprice_count: 0,
        lifecycle: ChaseLifecycle::Resting,
        last_reprice_at: None,
        desired_price: None,
        stop_reason: None,
        cancel_retries: 0,
    }
}

fn fill(oid: u64, time: u64, px: &str, sz: &str, fee: &str, closed_pnl: &str) -> UserFill {
    UserFill {
        coin: "BTC".to_string(),
        px: px.to_string(),
        sz: sz.to_string(),
        side: "B".to_string(),
        time,
        hash: None,
        tid: None,
        oid: Some(oid),
        dir: "Open Long".to_string(),
        closed_pnl: closed_pnl.to_string(),
        fee: fee.to_string(),
    }
}
