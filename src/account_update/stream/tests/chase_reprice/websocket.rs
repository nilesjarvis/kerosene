use super::{
    ChaseLifecycle, ChaseVerificationReason, WsUserData, account_data_with_timestamp,
    chase_order_by_id, connected_terminal, open_order, reprice_verification_chase,
};

fn terminal_waiting_for_chase_refresh() -> super::TradingTerminal {
    let mut terminal = connected_terminal();
    terminal.account_data = Some(account_data_with_timestamp(1_000));
    terminal.order_status = Some((
        "Chasing (oid 42); refreshing account data...".to_string(),
        false,
    ));
    terminal
        .chase_orders
        .insert(1, reprice_verification_chase());
    terminal
}

#[test]
fn websocket_open_order_update_does_not_bypass_account_verification() {
    let mut terminal = terminal_waiting_for_chase_refresh();
    let mut order = open_order(42, Some(false));
    order.limit_px = "101".to_string();
    order.sz = "1.0".to_string();

    let _task = terminal.apply_ws_user_data_update(
        Some(super::CONNECTED_ADDRESS.to_string()),
        WsUserData::OpenOrders {
            dex: String::new(),
            orders: vec![order],
        },
    );

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Reprice
        }
    );
    assert_eq!(
        terminal.order_status,
        Some((
            "Chasing (oid 42); refreshing account data...".to_string(),
            false
        ))
    );
}

#[test]
fn stale_websocket_open_order_keeps_chase_refresh_pending() {
    let mut terminal = terminal_waiting_for_chase_refresh();
    let mut stale_order = open_order(42, Some(false));
    stale_order.limit_px = "100".to_string();
    stale_order.sz = "1.0".to_string();

    let _task = terminal.apply_ws_user_data_update(
        Some(super::CONNECTED_ADDRESS.to_string()),
        WsUserData::OpenOrders {
            dex: String::new(),
            orders: vec![stale_order],
        },
    );

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::Reprice
        }
    );
    assert_eq!(
        terminal.order_status,
        Some((
            "Chasing (oid 42); refreshing account data...".to_string(),
            false
        ))
    );
}
