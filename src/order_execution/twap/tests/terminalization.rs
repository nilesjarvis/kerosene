use super::fixtures::{test_twap, twap_by_id};
use crate::api::{BookLevel, OrderBook};
use crate::app_state::TradingTerminal;
use crate::twap_state::{TwapBookSnapshot, TwapChildStatus, TwapPendingSlice, TwapStatus};

use std::time::Instant;

fn outside_allowed_range_book() -> OrderBook {
    OrderBook {
        bids: vec![BookLevel { px: 199.0, sz: 1.0 }],
        asks: vec![BookLevel { px: 200.0, sz: 1.0 }],
    }
}

fn final_slice_twap(now: Instant) -> crate::twap_state::TwapOrder {
    let mut twap = test_twap(1, "0xchild", now);
    twap.status = TwapStatus::Running;
    twap.pause_reason = None;
    twap.status_check_cloid = None;
    twap.status_check_pending_attempt = None;
    twap.child_orders.clear();
    twap.slice_count = 1;
    twap.next_slice_due = now;
    twap.latest_book = Some(TwapBookSnapshot {
        book: outside_allowed_range_book(),
        updated_at: now,
    });
    twap
}

#[test]
fn nonterminal_slice_skip_retains_key_for_the_next_slice() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc".to_string());
    let mut twap = final_slice_twap(now);
    twap.slice_count = 2;
    terminal.twap_orders.insert(1, twap);

    let task = terminal.execute_due_twap_slice(1, now);

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(task.units(), 0);
    assert_eq!(twap.status, TwapStatus::WaitingForMarket);
    assert_eq!(twap.slices_attempted, 1);
    assert!(!twap.agent_key.as_str().is_empty());
    assert!(terminal.advanced_order_history.is_empty());
}

#[test]
fn final_initial_slice_skip_scrubs_key_without_changing_history_visibility() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc".to_string());
    terminal.twap_orders.insert(1, final_slice_twap(now));

    let task = terminal.execute_due_twap_slice(1, now);

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(task.units(), 0);
    assert_eq!(twap.status, TwapStatus::Stopped);
    assert_eq!(twap.slices_attempted, 1);
    assert!(twap.agent_key.as_str().is_empty());
    assert!(terminal.advanced_order_history.is_empty());
}

#[test]
fn final_retry_slice_skip_scrubs_key_without_changing_history_visibility() {
    let now = Instant::now();
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc".to_string());
    let mut twap = final_slice_twap(now);
    twap.slices_attempted = 1;
    twap.filled_size = 0.25;
    twap.remaining_size = 0.75;
    twap.retry_slice = Some(TwapPendingSlice {
        index: 1,
        planned_size: 1.0,
        limit_price: 100.0,
        cloid: "0xchild".to_string(),
        retry_count: 1,
    });
    twap.child_orders.push(crate::twap_state::TwapChildOrder {
        index: 1,
        requested_at: now,
        planned_size: 1.0,
        limit_price: 100.0,
        oid: None,
        cloid: Some("0xchild".to_string()),
        status: TwapChildStatus::Retrying,
        exchange_summary: "retrying".to_string(),
        filled_size: 0.0,
        avg_price: None,
        fee: 0.0,
        retry_count: 1,
    });
    terminal.twap_orders.insert(1, twap);

    let task = terminal.execute_due_twap_slice(1, now);

    let twap = twap_by_id(&terminal, 1);
    assert_eq!(task.units(), 0);
    assert_eq!(twap.status, TwapStatus::CompletedPartial);
    assert_eq!(twap.child_orders[0].status, TwapChildStatus::NoFill);
    assert!(twap.retry_slice.is_none());
    assert!(twap.agent_key.as_str().is_empty());
    assert!(terminal.advanced_order_history.is_empty());
}
