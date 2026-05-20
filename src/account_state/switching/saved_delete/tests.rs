use crate::app_state::{TradingTerminal, sensitive_string};
use crate::config::AccountProfile;
use crate::signing::ChaseOrder;

use std::time::Instant;

fn account(secret_id: &str, name: &str, wallet_address: &str) -> AccountProfile {
    AccountProfile {
        secret_id: secret_id.to_string(),
        name: name.to_string(),
        wallet_address: wallet_address.to_string(),
        agent_key: sensitive_string(format!("{secret_id}-agent-key")),
        hydromancer_api_key: sensitive_string(""),
    }
}

fn chase_order(account_address: &str) -> ChaseOrder {
    ChaseOrder {
        id: 42,
        coin: "BTC".to_string(),
        account_address: account_address.to_string(),
        agent_key: sensitive_string("old-account-agent-key"),
        is_buy: true,
        target_size: 1.0,
        filled_size: 0.0,
        remaining_size: 1.0,
        known_oids: vec![1001],
        current_cloid: None,
        place_attempt_count: 0,
        asset: 0,
        sz_decimals: 5,
        is_spot: false,
        reduce_only: false,
        current_oid: Some(1001),
        current_price: 50_000.0,
        current_price_wire: "50000".to_string(),
        initial_price: 50_000.0,
        started_at: Instant::now(),
        started_at_ms: 1,
        reprice_count: 0,
        pending_op: None,
        last_reprice_at: None,
        pending_best_price: None,
        pending_size_correction: false,
        stop_requested: false,
        stop_reason: None,
        cancel_retries: 0,
        oid_confirmed: true,
        missing_open_order_refresh_requested: false,
    }
}

#[test]
fn adjust_active_index_shifts_down_when_earlier_account_removed() {
    assert_eq!(TradingTerminal::adjust_active_index_after_removal(3, 1), 2);
}

#[test]
fn adjust_active_index_keeps_value_when_later_account_removed() {
    assert_eq!(TradingTerminal::adjust_active_index_after_removal(2, 5), 2);
}

#[test]
fn adjust_active_index_keeps_value_when_active_itself_is_removed() {
    // The active account being removed is handled separately by the
    // fallback-switch path; the index adjustment alone should not shift.
    assert_eq!(TradingTerminal::adjust_active_index_after_removal(4, 4), 4);
}

#[test]
fn adjust_active_index_handles_zero_indexes() {
    assert_eq!(TradingTerminal::adjust_active_index_after_removal(0, 0), 0);
    assert_eq!(TradingTerminal::adjust_active_index_after_removal(1, 0), 0);
}

#[test]
fn active_account_delete_is_blocked_while_chase_order_is_active() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.desktop_notifications = false;
    terminal.accounts = vec![
        account(
            "account-a",
            "Account A",
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        account(
            "account-b",
            "Account B",
            "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ),
    ];
    terminal.active_account_index = 0;
    terminal.wallet_address_input = terminal.accounts[0].wallet_address.clone();
    terminal.wallet_key_input = terminal.accounts[0].agent_key.clone();
    terminal.connected_address = Some(terminal.accounts[0].wallet_address.clone());
    terminal.chase_orders.insert(
        42,
        chase_order("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    );

    let _task = terminal.delete_saved_account_task(0);

    assert_eq!(terminal.active_account_index, 0);
    assert_eq!(terminal.accounts.len(), 2);
    assert_eq!(terminal.accounts[0].secret_id, "account-a");
    assert_eq!(
        terminal.wallet_address_input,
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    assert!(terminal.chase_orders.contains_key(&42));
    let toast = terminal.toasts.last().expect("blocked delete should toast");
    assert!(toast.is_error);
    assert!(toast.message.contains("Stop active chase orders"));
}
