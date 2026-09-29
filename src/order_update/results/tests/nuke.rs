use super::*;

#[test]
fn nuke_results_aggregate_until_all_children_settle() {
    let mut terminal = terminal_with_connected_account();
    terminal.pending_nuke_execution = Some(PendingNukeExecution::new(7, 2, 1));

    let _task = terminal.handle_nuke_result(
        7,
        nuke_context("BTC"),
        Ok(exchange_response(vec![serde_json::json!({
            "filled": {
                "totalSz": "1",
                "avgPx": "100",
                "oid": 42_u64
            }
        })])),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(!is_error);
    assert_eq!(message, "NUKE progress: 1/2 confirmed; 1 skipped");
    assert!(terminal.pending_nuke_execution.is_some());

    let _task = terminal.handle_nuke_result(
        7,
        nuke_context("ETH"),
        Ok(exchange_response(vec![serde_json::json!({
            "error": "Order rejected"
        })])),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert_eq!(
        message,
        "NUKE completed: 1/2 confirmed; 1 failed; 1 skipped"
    );
    assert!(terminal.pending_nuke_execution.is_none());
}

#[test]
fn nuke_uncertain_child_waits_for_order_status_before_aggregating() {
    let mut terminal = terminal_with_connected_account();
    terminal.pending_nuke_execution = Some(PendingNukeExecution::new(9, 1, 0));

    let _task = terminal.handle_nuke_result(
        9,
        nuke_context("BTC"),
        Err("exchange request failed".to_string()),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("NUKE placement status unknown for BTC"));
    assert!(terminal.pending_nuke_execution.is_some());

    let _task = terminal.handle_nuke_placement_status_result(
        9,
        nuke_context("BTC"),
        Ok(order_status("filled")),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(!is_error);
    assert_eq!(message, "NUKE completed: 1/1 confirmed");
    assert!(terminal.pending_nuke_execution.is_none());
}

#[test]
fn nuke_direct_resting_market_child_is_uncertain_not_confirmed() {
    let mut terminal = terminal_with_connected_account();
    terminal.pending_nuke_execution = Some(PendingNukeExecution::new(13, 1, 0));

    let _task = terminal.handle_nuke_result(
        13,
        nuke_context("BTC"),
        Ok(exchange_response(vec![serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        })])),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("NUKE market order unexpectedly rested for BTC"));
    assert!(terminal.pending_nuke_execution.is_none());
}

#[test]
fn nuke_status_open_market_child_is_uncertain_not_confirmed() {
    let mut terminal = terminal_with_connected_account();
    terminal.pending_nuke_execution = Some(PendingNukeExecution::new(14, 1, 0));

    let _task = terminal.handle_nuke_placement_status_result(
        14,
        nuke_context("BTC"),
        Ok(order_status("open")),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("NUKE market order unexpectedly rested for BTC"));
    assert!(terminal.pending_nuke_execution.is_none());
}

#[test]
fn nuke_results_after_account_switch_clear_stale_execution_without_status() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some(OTHER_ACCOUNT.to_string());
    terminal.pending_nuke_execution = Some(PendingNukeExecution::new(11, 1, 0));

    let _task = terminal.handle_nuke_result(
        11,
        nuke_context("BTC"),
        Ok(exchange_response(vec![serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        })])),
    );

    assert!(terminal.pending_nuke_execution.is_none());
    assert!(terminal.order_status.is_none());

    terminal.pending_nuke_execution = Some(PendingNukeExecution::new(12, 1, 0));
    let _task = terminal.handle_nuke_placement_status_result(
        12,
        nuke_context("BTC"),
        Ok(order_status("open")),
    );

    assert!(terminal.pending_nuke_execution.is_none());
    assert!(terminal.order_status.is_none());
}
