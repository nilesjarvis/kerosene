use super::{
    ChaseLifecycle, ChaseQueuedAction, ChaseVerificationReason, Duration, Instant, chase,
    chase_by_id, exchange_ready_terminal,
};
use crate::api::{ExchangeSymbol, MarketType, USDC_TOKEN_INDEX};
use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::signing::ChaseStopPhase;
use iced::Task;

const IDENTITY_ERROR: &str = "Chase stopped: spot market identity changed";
const METADATA_ERROR: &str = "Chase stopped: spot metadata has not been verified";

fn terminal_with_market_faults(
    faults: u8,
    is_spot: bool,
    current_oid: Option<u64>,
) -> TradingTerminal {
    let mut terminal = exchange_ready_terminal();
    let symbol = ExchangeSymbol {
        key: "@7".into(),
        ticker: "SYNTH".into(),
        category: "spot".into(),
        display_name: Some(
            if faults & 2 == 0 {
                "SYNTH/USDC"
            } else {
                "SYNTH/OTHER"
            }
            .into(),
        ),
        keywords: vec!["spot".into()],
        asset_index: 10_007,
        collateral_token: Some(USDC_TOKEN_INDEX),
        sz_decimals: 2,
        max_leverage: 1,
        only_isolated: false,
        growth_mode: false,
        market_type: MarketType::Spot,
        outcome: None,
    };
    terminal.record_chase_spot_symbol_identity(1, &symbol);
    terminal.exchange_symbols = vec![symbol];
    if faults & 1 != 0 {
        terminal.exchange_symbols[0].asset_index += 1;
    }
    terminal.spot_metadata_degraded = faults & 4 != 0;
    terminal.chase_orders.clear();
    terminal.advanced_order_history.clear();
    let mut order = chase();
    order.coin = "@7".into();
    order.asset = 10_007;
    order.sz_decimals = 2;
    order.is_spot = is_spot;
    order.current_oid = current_oid;
    order.desired_price = Some(103.0);
    terminal.chase_orders.insert(1, order);
    let mut unrelated = chase();
    unrelated.id = 2;
    unrelated.coin = "UNRELATED".into();
    terminal.chase_orders.insert(2, unrelated);
    terminal.selected_chase_id = Some(1);
    terminal.order_status = Some(("unchanged".into(), false));
    terminal
}

fn reprice(terminal: &mut TradingTerminal, reconcile: bool, best: f64) -> Task<Message> {
    if reconcile {
        terminal.chase_modify_for_current_price_reconciliation(1)
    } else {
        terminal.chase_reprice_to_best_price(1, best)
    }
}

#[test]
fn spot_reprice_checks_preserve_error_priority_recovery_and_perp_bypass() {
    for reconcile in [false, true] {
        for is_spot in [false, true] {
            for current_oid in [None, Some(42)] {
                for faults in 0..8 {
                    let mut terminal = terminal_with_market_faults(faults, is_spot, current_oid);
                    let expected_error = if is_spot {
                        match faults {
                            1 | 3 | 5 | 7 => Some(IDENTITY_ERROR.to_string()),
                            2 | 6 => Some(
                                terminal
                                    .validate_spot_quantity_denomination("@7", false)
                                    .expect_err("unsupported quote"),
                            ),
                            4 => Some(METADATA_ERROR.to_string()),
                            _ => None,
                        }
                    } else {
                        None
                    };

                    let task = reprice(&mut terminal, reconcile, 101.0);

                    if let Some(reason) = expected_error {
                        assert_eq!(task.units(), usize::from(current_oid.is_some()));
                        let expected_status = if let Some(oid) = current_oid {
                            let order = chase_by_id(&terminal, 1);
                            assert_eq!(
                                order.lifecycle,
                                ChaseLifecycle::Stopping {
                                    phase: ChaseStopPhase::Canceling { oid }
                                }
                            );
                            assert_eq!(order.stop_reason, Some((reason.clone(), true)));
                            assert_eq!(order.desired_price, Some(103.0));
                            assert_eq!(order.reprice_count, 0);
                            assert!(order.last_reprice_at.is_none());
                            format!("{reason}: cancelling order {oid}")
                        } else {
                            assert!(!terminal.chase_orders.contains_key(&1));
                            assert_eq!(terminal.selected_chase_id, Some(2));
                            assert_eq!(terminal.advanced_order_history.len(), 1);
                            assert_eq!(terminal.advanced_order_history[0].summary, reason);
                            reason
                        };
                        assert_eq!(terminal.order_status, Some((expected_status, true)));
                        assert!(!terminal.account_loading);
                        assert!(!terminal.account_reconciliation_required);
                        assert!(terminal.last_advanced_exchange_request_at.is_none());
                    } else {
                        let order = chase_by_id(&terminal, 1);
                        assert!(order.stop_reason.is_none());
                        if current_oid.is_none() {
                            assert_eq!(task.units(), 0);
                            assert_eq!(order.lifecycle, ChaseLifecycle::Resting);
                            assert_eq!(order.desired_price, Some(103.0));
                            assert_eq!(terminal.order_status, Some(("unchanged".into(), false)));
                        } else if reconcile {
                            assert_eq!(task.units(), 1);
                            assert_eq!(order.lifecycle, ChaseLifecycle::Modifying { oid: 42 });
                            assert_eq!(order.desired_price, Some(103.0));
                            assert_eq!(order.reprice_count, 1);
                            assert!(terminal.last_advanced_exchange_request_at.is_some());
                        } else {
                            assert!(task.units() > 0);
                            assert_eq!(
                                order.lifecycle,
                                ChaseLifecycle::Verifying {
                                    reason: ChaseVerificationReason::Reprice
                                }
                            );
                            assert_eq!(order.desired_price, Some(101.0));
                            assert_eq!(order.reprice_count, 0);
                            assert!(terminal.account_loading);
                            assert!(terminal.account_reconciliation_required);
                        }
                    }
                    let unrelated = chase_by_id(&terminal, 2);
                    assert_eq!(unrelated.lifecycle, ChaseLifecycle::Resting);
                    assert_eq!(unrelated.desired_price, None);
                    assert!(unrelated.stop_reason.is_none());
                }
            }
        }
    }
}

#[test]
fn spot_reprice_checks_keep_their_position_among_existing_gates() {
    for reconcile in [false, true] {
        for gate in 0..4 {
            let mut terminal = terminal_with_market_faults(7, true, Some(42));
            let mut best = 101.0;
            match gate {
                0 => {
                    terminal.chase_orders.get_mut(&1).expect("chase").lifecycle =
                        ChaseLifecycle::Modifying { oid: 42 }
                }
                1 => terminal.connected_address = None,
                2 => {
                    terminal.last_advanced_exchange_request_at =
                        Some(Instant::now() + Duration::from_secs(60))
                }
                _ => {
                    terminal
                        .chase_orders
                        .get_mut(&1)
                        .expect("chase")
                        .desired_price = Some(f64::NAN);
                    best = f64::NAN;
                }
            }

            let task = reprice(&mut terminal, reconcile, best);

            let order = chase_by_id(&terminal, 1);
            if reconcile && matches!(gate, 0 | 2) {
                assert_eq!(task.units(), 0);
                assert_eq!(
                    order.lifecycle,
                    if gate == 0 {
                        ChaseLifecycle::Modifying { oid: 42 }
                    } else {
                        ChaseLifecycle::Queued {
                            action: ChaseQueuedAction::Reprice,
                        }
                    }
                );
                assert!(order.stop_reason.is_none());
                assert_eq!(terminal.order_status, Some(("unchanged".into(), false)));
            } else {
                let reason = match gate {
                    1 => "Chase stopped: account changed before reprice",
                    3 if reconcile => "Chase stopped: invalid chase price",
                    _ => IDENTITY_ERROR,
                };
                assert_eq!(task.units(), 1);
                assert_eq!(
                    order.lifecycle,
                    ChaseLifecycle::Stopping {
                        phase: ChaseStopPhase::Canceling { oid: 42 }
                    }
                );
                assert_eq!(order.stop_reason, Some((reason.into(), true)));
                assert_eq!(
                    terminal.order_status,
                    Some((format!("{reason}: cancelling order 42"), true))
                );
            }
            assert_eq!(order.reprice_count, 0);
        }

        let mut terminal = terminal_with_market_faults(7, true, Some(42));
        terminal.chase_orders.remove(&1);
        assert_eq!(reprice(&mut terminal, reconcile, 101.0).units(), 0);
        assert_eq!(terminal.order_status, Some(("unchanged".into(), false)));
        assert_eq!(chase_by_id(&terminal, 2).lifecycle, ChaseLifecycle::Resting);
    }
}
