use super::{
    active_position, connect_test_account, exchange_symbol, order_status_or_panic,
    stale_account_data, terminal_with_degraded_fresh_account,
    terminal_with_incomplete_fresh_account, terminal_with_stale_account,
};
use crate::order_execution::{OneShotPlacementContext, OrderSurface, PendingOrderAction};
use crate::order_update::PendingOneShotStatusRequest;
use crate::signing::ExchangeOrderKind;

fn make_nuke_ready(terminal: &mut crate::app_state::TradingTerminal, positions: Vec<(&str, &str)>) {
    let now_ms = crate::app_time::now_ms();
    terminal.exchange_symbols = positions
        .iter()
        .map(|(coin, _)| exchange_symbol(coin))
        .collect();
    let mut data = stale_account_data();
    data.fetched_at_ms = now_ms;
    data.clearinghouse.asset_positions = positions
        .into_iter()
        .map(|(coin, size)| active_position(coin, size))
        .collect();
    let address = terminal
        .connected_address
        .clone()
        .expect("test connected account");
    terminal.set_account_data_for_address_for_test(address, data);
    terminal.account_loading = false;
}

fn set_mid(terminal: &mut crate::app_state::TradingTerminal, coin: &str, mid: f64) {
    let now_ms = crate::app_time::now_ms();
    terminal.all_mids.insert(coin.to_string(), mid);
    terminal
        .all_mids_updated_at_ms
        .insert(coin.to_string(), now_ms);
}

#[test]
fn execute_nuke_refuses_stale_account_snapshot_and_requests_refresh() {
    let mut terminal = terminal_with_stale_account();

    let _task = terminal.execute_nuke_positions();

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("Account data is stale"));
    assert!(message.contains("refresh before NUKE"));
    assert!(terminal.account_loading);
}

#[test]
fn handle_nuke_refuses_stale_account_snapshot_before_arming() {
    let mut terminal = terminal_with_stale_account();
    terminal.close_menu_coin = Some("BTC".to_string());

    let _task = terminal.handle_nuke_positions();

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("Account data is stale"));
    assert!(message.contains("refresh before NUKE"));
    assert!(terminal.account_loading);
    assert!(terminal.nuke_confirmation.is_none());
    assert!(terminal.pending_nuke_execution.is_none());
    assert_eq!(terminal.close_menu_coin.as_deref(), Some("BTC"));
}

#[test]
fn execute_nuke_refuses_missing_account_snapshot_without_pending_work() {
    let mut terminal = crate::app_state::TradingTerminal::boot().0;
    connect_test_account(&mut terminal);
    terminal.set_committed_agent_key_for_test("agent-key");
    terminal.account_data = None;
    terminal.account_loading = false;

    let _task = terminal.execute_nuke_positions();

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(message, "No account data available; refresh before NUKE");
    assert!(terminal.pending_nuke_execution.is_none());
}

#[test]
fn handle_nuke_refuses_pending_trading_request_before_arming() {
    let mut terminal = terminal_with_stale_account();
    make_nuke_ready(&mut terminal, vec![("BTC", "1")]);
    set_mid(&mut terminal, "BTC", 100.0);
    terminal.pending_order_action = Some(PendingOrderAction::Buy);
    terminal.close_menu_coin = Some("BTC".to_string());

    let _task = terminal.handle_nuke_positions();

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(
        message,
        "Wait for pending trading requests to finish before NUKE"
    );
    assert!(terminal.nuke_confirmation.is_none());
    assert!(terminal.pending_nuke_execution.is_none());
    assert_eq!(terminal.close_menu_coin.as_deref(), Some("BTC"));
}

#[test]
fn handle_nuke_clears_armed_confirmation_when_trading_request_becomes_pending() {
    let mut terminal = terminal_with_stale_account();
    make_nuke_ready(&mut terminal, vec![("BTC", "1")]);
    set_mid(&mut terminal, "BTC", 100.0);

    let _task = terminal.handle_nuke_positions();
    assert!(terminal.nuke_confirmation.is_some());

    terminal.pending_order_action = Some(PendingOrderAction::Buy);
    let _task = terminal.handle_nuke_positions();

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(
        message,
        "Wait for pending trading requests to finish before NUKE"
    );
    assert!(terminal.nuke_confirmation.is_none());
    assert!(terminal.pending_nuke_execution.is_none());
}

#[test]
fn execute_nuke_refuses_pending_status_reconciliation_before_signing() {
    let mut terminal = terminal_with_stale_account();
    make_nuke_ready(&mut terminal, vec![("BTC", "1")]);
    set_mid(&mut terminal, "BTC", 100.0);
    terminal.insert_pending_one_shot_status_request(PendingOneShotStatusRequest::new(
        7,
        &OneShotPlacementContext {
            account_address: super::TEST_ACCOUNT.to_string(),
            cloid: "0x00000000000000000000000000000000".to_string(),
            surface: OrderSurface::Ticket,
            symbol_key: "BTC".to_string(),
            order_kind: ExchangeOrderKind::Market,
        },
    ));

    let _task = terminal.execute_nuke_positions();

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(
        message,
        "Wait for pending trading requests to finish before NUKE"
    );
    assert!(terminal.pending_nuke_execution.is_none());
}

#[test]
fn handle_nuke_refuses_incomplete_position_snapshot_before_arming() {
    let mut terminal = terminal_with_incomplete_fresh_account();

    let _task = terminal.handle_nuke_positions();

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("Positions may be incomplete"));
    assert!(message.contains("HIP-3 positions unavailable"));
    assert!(message.contains("refresh before NUKE"));
    assert!(terminal.account_loading);
    assert!(terminal.nuke_confirmation.is_none());
    assert!(terminal.pending_nuke_execution.is_none());
}

#[test]
fn execute_nuke_refuses_incomplete_position_snapshot_and_requests_refresh() {
    let mut terminal = terminal_with_incomplete_fresh_account();

    let _task = terminal.execute_nuke_positions();

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(message.contains("Positions may be incomplete"));
    assert!(message.contains("HIP-3 positions unavailable"));
    assert!(message.contains("refresh before NUKE"));
    assert!(terminal.account_loading);
    assert!(terminal.pending_nuke_execution.is_none());
}

#[test]
fn execute_nuke_arms_on_degraded_fallback_snapshot() {
    // Regression: a complete Hyperliquid fallback snapshot (degraded, but with
    // usable positions) must not be blocked by the positions-incomplete gate —
    // otherwise a Hydromancer-without-key config could never close risk.
    let mut terminal = terminal_with_degraded_fresh_account();

    let _task = terminal.execute_nuke_positions();

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(!is_error, "expected a non-error status, got: {message}");
    assert!(message.contains("Nuking"), "unexpected status: {message}");
    assert!(!message.contains("Positions may be incomplete"));
    assert!(terminal.pending_nuke_execution.is_some());
    assert!(!terminal.account_loading);
}

#[test]
fn execute_nuke_aborts_loudly_when_hidden_exposure_cannot_route() {
    let mut terminal = terminal_with_stale_account();
    terminal.exchange_symbols = vec![exchange_symbol("HIDDEN")];
    terminal.muted_tickers.insert("HIDDEN".to_string());
    let mut data = stale_account_data();
    data.fetched_at_ms = crate::app_time::now_ms();
    data.clearinghouse.asset_positions = vec![active_position("HIDDEN", "1")];
    let address = terminal
        .connected_address
        .clone()
        .expect("test connected account");
    terminal.set_account_data_for_address_for_test(address, data);

    let _task = terminal.execute_nuke_positions();

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(
        message,
        "NUKE aborted: hidden exposure could not be routed. Hidden skipped: HIDDEN (no mid price)"
    );
}

#[test]
fn handle_nuke_rearms_when_confirmation_plan_changes() {
    let mut terminal = terminal_with_stale_account();
    make_nuke_ready(&mut terminal, vec![("BTC", "1"), ("ETH", "-2")]);
    set_mid(&mut terminal, "BTC", 100.0);

    let _task = terminal.handle_nuke_positions();

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(
        message,
        concat!(
            "NUKE armed: will close 1 (BTC); SKIPPING ETH (no mid price). ",
            "Press NUKE again within 5 seconds to fire partial nuke."
        )
    );
    assert!(terminal.nuke_confirmation.is_some());
    assert!(terminal.pending_nuke_execution.is_none());

    set_mid(&mut terminal, "ETH", 100.0);
    let _task = terminal.handle_nuke_positions();

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(
        message,
        "NUKE armed: will close 2 positions (BTC, ETH). Press NUKE again within 5 seconds."
    );
    assert!(terminal.nuke_confirmation.is_some());
    assert!(terminal.pending_nuke_execution.is_none());
}

#[test]
fn handle_nuke_executes_when_confirmation_plan_is_unchanged() {
    let mut terminal = terminal_with_stale_account();
    make_nuke_ready(&mut terminal, vec![("BTC", "1")]);
    set_mid(&mut terminal, "BTC", 100.0);

    let _task = terminal.handle_nuke_positions();
    assert!(terminal.nuke_confirmation.is_some());

    let _task = terminal.handle_nuke_positions();

    let (message, is_error) = order_status_or_panic(&terminal);
    assert!(!is_error);
    assert_eq!(message, "Nuking 1 position...");
    assert!(terminal.nuke_confirmation.is_none());
    assert!(terminal.pending_nuke_execution.is_some());
}
