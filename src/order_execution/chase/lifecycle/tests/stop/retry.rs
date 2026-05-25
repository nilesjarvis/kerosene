use super::{
    ChaseLifecycle, ChaseStopPhase, Duration, Instant, TradingTerminal, chase, chase_by_id,
};

#[test]
fn retry_stopped_chase_cancels_rearms_retryable_cancel_failure() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.account_loading = false;
    terminal.account_reconciliation_required = false;
    terminal.last_advanced_exchange_request_at = None;
    let mut chase = chase();
    chase.lifecycle = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::VerifyingCancel { oid: 42 },
    };
    chase.stop_reason = Some(("Chase stopped".to_string(), false));
    chase.cancel_retries = 1;
    chase.last_reprice_at = Some(Instant::now() - Duration::from_secs(2));
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.retry_stopped_chase_cancels(Instant::now());

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid: 42 }
        }
    );
}
