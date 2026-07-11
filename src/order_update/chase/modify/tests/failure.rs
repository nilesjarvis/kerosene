use super::fixtures::{chase, chase_by_id, exchange_response, order_status_or_panic};
use crate::app_state::TradingTerminal;
use crate::signing::{ChaseLifecycle, ChaseStopPhase};

// A non-retryable modify rejection must cancel the still-resting order, not
// park the chase in Stopping::AwaitingModify waiting on a modify result that
// has already been consumed (which left the order live at a stale price).

#[test]
fn chase_modify_non_retryable_error_cancels_resting_order() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.chase_orders.insert(1, chase());

    let _task = terminal.handle_chase_modify_result(
        1,
        42,
        1,
        Ok(exchange_response(serde_json::json!({
            "error": "Order must have minimum value of $10"
        }))),
    );

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid: 42 }
        }
    );
    assert_eq!(chase.current_oid, Some(42));

    let (status, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(status.contains("modify failed"));
    assert!(status.contains("cancelling order 42"));
}

#[test]
fn chase_modify_non_retryable_error_while_stopping_cancels_resting_order() {
    let mut terminal = TradingTerminal::boot().0;
    let mut stopping = chase();
    stopping.lifecycle = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::AwaitingModify { oid: 42 },
    };
    stopping.stop_reason = Some(("Chase stopped".to_string(), false));
    terminal.chase_orders.insert(1, stopping);

    let _task = terminal.handle_chase_modify_result(
        1,
        42,
        1,
        Ok(exchange_response(serde_json::json!({
            "error": "Insufficient margin to place order"
        }))),
    );

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid: 42 }
        }
    );
}
