use crate::app_state::TradingTerminal;
use crate::twap_state::{
    TwapChildOrder, TwapChildStatus, TwapOrder, TwapOrderInit, TwapPauseReason, TwapPendingOp,
    TwapPendingSlice, TwapStatus,
};

use std::time::{Duration, Instant};

pub(in crate::order_execution::twap::tests) fn test_twap(
    id: u64,
    cloid: &str,
    now: Instant,
) -> TwapOrder {
    let mut twap = TwapOrder::new(TwapOrderInit {
        id,
        coin: "BTC".to_string(),
        display_coin: "BTC".to_string(),
        account_address: "0xabc".to_string(),
        agent_key: "test-agent-key".to_string().into(),
        is_buy: true,
        target_size: 1.0,
        asset: 0,
        sz_decimals: 3,
        is_spot: false,
        reduce_only: false,
        min_price: 90.0,
        max_price: 110.0,
        randomize: false,
        duration: Duration::from_secs(300),
        slice_count: 2,
        now,
        started_at_ms: 1_000,
    });
    twap.status = TwapStatus::Paused;
    twap.pause_reason = Some(TwapPauseReason::StatusUnknown);
    twap.status_check_cloid = Some(cloid.to_string());
    twap.child_orders.push(TwapChildOrder {
        index: 1,
        requested_at: now,
        planned_size: 0.5,
        limit_price: 100.0,
        oid: None,
        cloid: Some(cloid.to_string()),
        status: TwapChildStatus::StatusUnknown,
        exchange_summary: "status unknown".to_string(),
        filled_size: 0.0,
        avg_price: None,
        fee: 0.0,
        retry_count: 0,
    });
    twap
}

pub(in crate::order_execution::twap::tests) fn pending_twap(
    id: u64,
    cloid: &str,
    now: Instant,
) -> TwapOrder {
    let mut twap = test_twap(id, cloid, now);
    twap.status = TwapStatus::Running;
    twap.pause_reason = None;
    twap.paused_until = None;
    twap.status_check_cloid = None;
    twap.pending_op = Some(TwapPendingOp::Place(TwapPendingSlice {
        index: 1,
        planned_size: 0.5,
        limit_price: 100.0,
        cloid: cloid.to_string(),
        retry_count: 0,
    }));
    if let Some(child) = twap.child_orders.first_mut() {
        child.status = TwapChildStatus::Pending;
    }
    twap
}

pub(in crate::order_execution::twap::tests) fn twap_by_id(
    terminal: &TradingTerminal,
    id: u64,
) -> &TwapOrder {
    match terminal.twap_orders.get(&id) {
        Some(twap) => twap,
        None => panic!("twap should remain active"),
    }
}

pub(in crate::order_execution::twap::tests) fn reconciliation_deadline(
    twap: &TwapOrder,
) -> Instant {
    match twap.reconciliation_deadline {
        Some(deadline) => deadline,
        None => panic!("exchange-filled child must arm reconciliation watchdog"),
    }
}
