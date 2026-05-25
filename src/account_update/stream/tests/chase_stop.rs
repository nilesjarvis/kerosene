use super::fixtures::{account_data_with_timestamp, chase_order, chase_order_by_id, open_order};
use crate::app_state::TradingTerminal;
use crate::signing::{ChaseLifecycle, ChaseStopPhase};
use crate::ws::WsUserData;

#[test]
fn websocket_open_order_update_does_not_override_stop_verification() {
    let mut terminal = TradingTerminal::boot().0;
    let address = "0xabc0000000000000000000000000000000000000".to_string();
    terminal.connected_address = Some(address.clone());
    terminal.account_data = Some(account_data_with_timestamp(1_000));

    let mut chase = chase_order();
    chase.lifecycle = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::VerifyingCancel { oid: 42 },
    };
    chase.stop_reason = Some(("Chase stopped".to_string(), false));
    terminal.chase_orders.insert(1, chase);

    let mut oversized_order = open_order(42, Some(false));
    oversized_order.sz = "2.0".to_string();

    let _task = terminal.apply_ws_user_data_update(
        Some(address),
        WsUserData::OpenOrders {
            dex: String::new(),
            orders: vec![oversized_order],
        },
    );

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::VerifyingCancel { oid: 42 }
        }
    );
}

#[test]
fn stopped_chase_clears_only_after_no_known_open_orders_remain() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    let mut chase = chase_order();
    chase.lifecycle = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::VerifyingCancel { oid: 42 },
    };
    chase.stop_reason = Some(("Chase stopped".to_string(), false));
    terminal.chase_orders.insert(1, chase);
    terminal.account_data = Some(account_data_with_timestamp(1_000));

    let _task = terminal.reconcile_chase_after_account_refresh();

    assert!(terminal.chase_orders.is_empty());
    assert_eq!(
        terminal.order_status,
        Some(("Chase stopped".to_string(), false))
    );
}

#[test]
fn stopped_chase_cancels_next_known_open_order_before_clearing() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    let mut chase = chase_order();
    chase.known_oids.push(43);
    chase.lifecycle = ChaseLifecycle::Stopping {
        phase: ChaseStopPhase::VerifyingCancel { oid: 42 },
    };
    chase.stop_reason = Some(("Chase stopped".to_string(), false));
    terminal.chase_orders.insert(1, chase);
    let mut data = account_data_with_timestamp(1_000);
    data.open_orders = vec![open_order(43, Some(false))];
    terminal.account_data = Some(data);

    let _task = terminal.reconcile_chase_after_account_refresh();

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.current_oid, Some(43));
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Stopping {
            phase: ChaseStopPhase::Canceling { oid: 43 }
        }
    );
}
