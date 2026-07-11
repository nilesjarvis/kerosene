use super::fixtures::{chase, chase_by_id, exchange_response, order_status_or_panic};
use crate::app_state::TradingTerminal;
use crate::signing::{ChaseLifecycle, ChaseQueuedAction};

use std::time::{Duration, Instant};

#[test]
fn chase_modify_rate_limit_keeps_target_queued_for_retry() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.chase_orders.insert(1, chase());

    let _task = terminal.handle_chase_modify_result(
        1,
        42,
        1,
        Ok(exchange_response(serde_json::json!({
            "error": "rate limit"
        }))),
    );

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Queued {
            action: ChaseQueuedAction::Reprice
        }
    );
    assert_eq!(chase.desired_price, Some(101.0));
    assert_eq!(chase.current_price, 100.0);
    assert!(!chase.can_reprice_now(Instant::now() + Duration::from_secs(4)));
    assert!(
        terminal
            .last_advanced_exchange_request_at
            .is_some_and(|last| last > Instant::now())
    );

    let (status, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(status.contains("rate limit"));
}

#[test]
fn chase_modify_rate_limit_keeps_size_correction_queued_for_retry() {
    let mut terminal = TradingTerminal::boot().0;
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Modifying { oid: 42 };
    chase.desired_price = Some(101.0);
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.handle_chase_modify_result(
        1,
        42,
        1,
        Ok(exchange_response(serde_json::json!({
            "error": "too many requests"
        }))),
    );

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Queued {
            action: ChaseQueuedAction::Reprice
        }
    );
    assert_eq!(chase.desired_price, Some(101.0));
}
