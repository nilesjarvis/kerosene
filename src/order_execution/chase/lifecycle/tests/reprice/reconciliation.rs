use super::{
    ChaseLifecycle, ChaseQueuedAction, chase, chase_by_id, exchange_busy_terminal,
    exchange_ready_terminal,
};
use crate::api::{ExchangeSymbol, MarketType, USDC_TOKEN_INDEX};
use crate::signing::ChaseStopPhase;

#[test]
fn chase_reconciliation_uses_pending_target_price() {
    let mut terminal = exchange_ready_terminal();
    let mut chase = chase();
    chase.current_price = f64::NAN;
    chase.current_price_wire.clear();
    chase.desired_price = Some(101.0);
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.chase_modify_for_current_price_reconciliation(1);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Modifying { oid: 42 });
    assert_eq!(chase.desired_price, Some(101.0));
}

#[test]
fn chase_reconciliation_queues_size_correction_when_exchange_gate_is_busy() {
    let mut terminal = exchange_busy_terminal();
    terminal.chase_orders.insert(1, chase());

    let _task = terminal.chase_modify_for_current_price_reconciliation(1);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Queued {
            action: ChaseQueuedAction::SizeCorrection
        }
    );
}

#[test]
fn spot_chase_reconciliation_rechecks_market_identity_at_modify_dispatch() {
    let mut terminal = exchange_ready_terminal();
    let spot_symbol = ExchangeSymbol {
        key: "@7".to_string(),
        ticker: "LOW".to_string(),
        category: "spot".to_string(),
        display_name: Some("LOW/USDC".to_string()),
        keywords: vec!["spot".to_string()],
        asset_index: 10_007,
        collateral_token: Some(USDC_TOKEN_INDEX),
        sz_decimals: 2,
        max_leverage: 1,
        only_isolated: false,
        market_type: MarketType::Spot,
        outcome: None,
    };
    terminal.exchange_symbols = vec![spot_symbol.clone()];
    terminal.record_chase_spot_symbol_identity(1, &spot_symbol);

    let mut chase = chase();
    chase.coin = "@7".to_string();
    chase.asset = 10_007;
    chase.sz_decimals = 2;
    chase.is_spot = true;
    chase.desired_price = Some(101.0);
    terminal.chase_orders.insert(1, chase);

    // Simulate a successful metadata refresh after the account-verification
    // task started but before its result reaches the final modify path.
    terminal.exchange_symbols[0].asset_index = 10_008;

    let _task = terminal.chase_modify_for_current_price_reconciliation(1);

    let chase = chase_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid: 42 }
        }
    );
    assert_eq!(chase.reprice_count, 0);
    assert!(chase.stop_reason.as_ref().is_some_and(
        |(message, is_error)| *is_error && message.contains("spot market identity changed")
    ));
}
