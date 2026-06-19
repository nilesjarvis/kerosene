use super::*;
use crate::app_state::TradingTerminal;
use std::time::{Duration, Instant};

#[test]
fn twap_history_snapshot_sanitizes_numbers_and_preserves_recent_activity() {
    let now = Instant::now();
    let mut twap = twap_order(now);
    twap.status = TwapStatus::CompletedPartial;
    twap.filled_size = 4.0;
    twap.remaining_size = f64::INFINITY;
    twap.events.push(TwapEvent {
        at: now + Duration::from_secs(1),
        kind: TwapEventKind::Filled,
        message: "filled slice".to_string(),
        is_error: false,
    });
    twap.child_orders = vec![
        child(now, 1, 1.0, Some(100.0), f64::INFINITY),
        child(now, 2, 3.0, Some(110.0), 0.25),
        child(now, 3, 2.0, Some(f64::INFINITY), 0.0),
        child(now, 4, f64::INFINITY, Some(999.0), 0.0),
    ];

    let entry = AdvancedOrderHistoryEntry::from_twap(&twap, 2_000);

    assert_eq!(entry.kind, AdvancedOrderHistoryKind::Twap);
    assert_eq!(entry.status, "Partial");
    assert_eq!(entry.summary, "filled slice");
    assert_eq!(entry.remaining_size, 0.0);
    assert_eq!(entry.average_price, Some(107.5));
    assert_eq!(entry.children[0].fee, 0.0);
    assert_eq!(entry.children[2].avg_price, None);
    assert_eq!(entry.logs.len(), 2);
}

#[test]
fn archive_twap_if_terminal_scrubs_runtime_agent_key() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    let mut twap = twap_order(now);
    twap.status = TwapStatus::Stopped;
    twap.push_event(TwapEventKind::Stopped, "TWAP stopped".to_string(), false);
    terminal.twap_orders.insert(twap.id, twap);

    terminal.archive_twap_if_terminal(7);

    let twap = terminal
        .twap_orders
        .get(&7)
        .expect("twap remains available");
    assert!(twap.agent_key.as_str().is_empty());
    assert_eq!(terminal.advanced_order_history.len(), 1);
    let entry = terminal
        .advanced_order_history
        .front()
        .expect("history entry");
    assert_eq!(entry.kind, AdvancedOrderHistoryKind::Twap);
    assert_eq!(entry.source_id, 7);
    assert_eq!(entry.summary, "TWAP stopped");
}
