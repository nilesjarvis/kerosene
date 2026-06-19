use super::{
    ChaseLifecycle, ChaseVerificationReason, WsUserData, account_data_with_timestamp, chase_order,
    chase_order_by_id, connected_terminal, open_order, reprice_verification_chase,
    set_account_data_for_connected_account,
};

fn terminal_waiting_for_chase_refresh() -> super::TradingTerminal {
    let mut terminal = connected_terminal();
    set_account_data_for_connected_account(&mut terminal, account_data_with_timestamp(1_000));
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

#[test]
fn websocket_open_order_disappearance_ignores_chase_from_other_account() {
    let mut terminal = connected_terminal();
    set_account_data_for_connected_account(&mut terminal, account_data_with_timestamp(1_000));
    let mut chase = chase_order();
    chase.account_address = "0xdef0000000000000000000000000000000000000".to_string();
    chase.lifecycle = ChaseLifecycle::Resting;
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.apply_ws_user_data_update(
        Some(super::CONNECTED_ADDRESS.to_string()),
        WsUserData::OpenOrders {
            dex: String::new(),
            orders: Vec::new(),
        },
    );

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Resting);
    assert_eq!(terminal.order_status, None);
}

#[test]
fn main_open_order_update_does_not_verify_hip3_chase_disappearance() {
    let mut terminal = connected_terminal();
    set_account_data_for_connected_account(&mut terminal, account_data_with_timestamp(1_000));
    let mut chase = chase_order();
    chase.coin = "flx:BTC".to_string();
    chase.lifecycle = ChaseLifecycle::Resting;
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.apply_ws_user_data_update(
        Some(super::CONNECTED_ADDRESS.to_string()),
        WsUserData::OpenOrders {
            dex: String::new(),
            orders: Vec::new(),
        },
    );

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Resting);
    assert_eq!(terminal.order_status, None);
}

#[test]
fn hip3_open_order_update_does_not_verify_main_chase_disappearance() {
    let mut terminal = connected_terminal();
    set_account_data_for_connected_account(&mut terminal, account_data_with_timestamp(1_000));
    let mut chase = chase_order();
    chase.lifecycle = ChaseLifecycle::Resting;
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.apply_ws_user_data_update(
        Some(super::CONNECTED_ADDRESS.to_string()),
        WsUserData::OpenOrders {
            dex: "flx".to_string(),
            orders: Vec::new(),
        },
    );

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Resting);
    assert_eq!(terminal.order_status, None);
}

#[test]
fn websocket_open_order_disappearance_verifies_matching_account_chase() {
    let mut terminal = connected_terminal();
    set_account_data_for_connected_account(&mut terminal, account_data_with_timestamp(1_000));
    let mut chase = chase_order();
    chase.lifecycle = ChaseLifecycle::Resting;
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.apply_ws_user_data_update(
        Some(super::CONNECTED_ADDRESS.to_string()),
        WsUserData::OpenOrders {
            dex: String::new(),
            orders: Vec::new(),
        },
    );

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(
        chase.lifecycle,
        ChaseLifecycle::Verifying {
            reason: ChaseVerificationReason::MissingOrder
        }
    );
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| !*is_error
                && message.contains("open-orders stream no longer shows the order"))
    );
}

#[test]
fn websocket_open_order_disappearance_ignores_stale_account_snapshot() {
    let mut terminal = connected_terminal();
    terminal.account_data_address = Some("0xdef0000000000000000000000000000000000000".to_string());
    terminal.account_data = Some(account_data_with_timestamp(1_000));
    terminal.account_loading = false;
    let mut chase = chase_order();
    chase.lifecycle = ChaseLifecycle::Resting;
    terminal.chase_orders.insert(1, chase);

    let _task = terminal.apply_ws_user_data_update(
        Some(super::CONNECTED_ADDRESS.to_string()),
        WsUserData::OpenOrders {
            dex: String::new(),
            orders: Vec::new(),
        },
    );

    let chase = chase_order_by_id(&terminal, 1);
    assert_eq!(chase.lifecycle, ChaseLifecycle::Resting);
    assert_eq!(terminal.order_status, None);
    assert!(terminal.account_data.is_none());
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
}
