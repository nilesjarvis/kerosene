use super::chase_terminal_cancel_error;
use crate::app_state::TradingTerminal;
use crate::signing::{ChaseLifecycle, ChaseOrder, ChaseStopPhase, MAX_CHASE_CANCEL_RETRIES};
use std::time::Instant;

#[test]
fn terminal_cancel_error_detects_already_gone_orders() {
    assert!(chase_terminal_cancel_error(
        "Error: Order was never placed, already canceled, or filled"
    ));
    assert!(chase_terminal_cancel_error("cannot cancel cancelled order"));
    assert!(chase_terminal_cancel_error("cannot cancel cancled order"));
    assert!(chase_terminal_cancel_error("order no longer open"));
    assert!(chase_terminal_cancel_error("order not found"));
}

#[test]
fn terminal_cancel_error_rejects_unrelated_cancel_failures() {
    assert!(!chase_terminal_cancel_error("Error: rate limited"));
    assert!(!chase_terminal_cancel_error("Exchange request failed"));
    assert!(!chase_terminal_cancel_error("invalid signature"));
}

fn chase() -> ChaseOrder {
    let started_at = Instant::now();
    ChaseOrder {
        id: 1,
        coin: "BTC".to_string(),
        account_address: "0xabc0000000000000000000000000000000000000".to_string(),
        agent_key: "agent-key".to_string().into(),
        is_buy: true,
        target_size: 1.0,
        filled_size: 0.0,
        remaining_size: 1.0,
        known_oids: vec![42],
        current_cloid: None,
        place_attempt_count: 0,
        asset: 0,
        sz_decimals: 3,
        is_spot: false,
        reduce_only: false,
        current_oid: Some(42),
        current_price: 100.0,
        current_price_wire: "100".to_string(),
        initial_price: 100.0,
        started_at,
        started_at_ms: 1_000,
        reprice_count: 0,
        lifecycle: ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid: 42 },
        },
        last_reprice_at: None,
        desired_price: None,
        stop_reason: Some(("Chase stopped".to_string(), false)),
        cancel_retries: MAX_CHASE_CANCEL_RETRIES - 1,
    }
}

#[test]
fn max_cancel_retry_keeps_chase_for_manual_check() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.chase_orders.insert(1, chase());

    let _task = terminal.handle_chase_cancel_result(1, 42, Err("network timeout".to_string()));

    let chase = terminal.chase_orders.get(&1).expect("chase should remain");
    assert_eq!(chase.cancel_retries, MAX_CHASE_CANCEL_RETRIES);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::VerifyingCancel { oid: 42 }
        }
    );
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| {
                *is_error && message.contains("requires manual check")
            })
    );
}
