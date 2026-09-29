use super::*;

#[test]
fn cancel_followup_preserves_reason_and_optional_order_identity() {
    for (result, prefix) in [
        (
            Err("transport failed".to_string()),
            Some("Cancel status unknown"),
        ),
        (
            Ok(exchange_response(vec![
                serde_json::json!({"error": "Order was never placed, already canceled, or filled."}),
            ])),
            Some("Cancel may have already resolved"),
        ),
        (
            Ok(exchange_response(vec![
                serde_json::json!({"error": "Rejected fixture"}),
            ])),
            None,
        ),
    ] {
        for known_order in [false, true] {
            let (mut terminal, pending_id) = terminal_with_pending_cancel();
            if !known_order {
                terminal.pending_order_indicators.clear();
                terminal.pending_cancel_status_request = None;
            }
            let task =
                terminal.handle_cancel_result(TEST_ACCOUNT.to_string(), pending_id, result.clone());
            assert!(terminal.pending_order_indicators.is_empty());
            assert_eq!(
                terminal
                    .account_data
                    .as_ref()
                    .expect("account data")
                    .open_orders
                    .len(),
                1
            );
            assert_eq!(
                terminal.pending_cancel_status_request.is_some(),
                known_order && prefix.is_some()
            );
            assert_eq!(terminal.account_reconciliation_required, prefix.is_some());
            assert_eq!(task.units() > 0, prefix.is_some());
            let (status, is_error) = terminal.order_status.as_ref().expect("cancel status");
            assert!(*is_error);
            if let Some(prefix) = prefix {
                let label = if known_order { " for order 42" } else { "" };
                assert!(status.starts_with(&format!("{prefix}{label}: ")));
                assert!(status.ends_with("; checking orderStatus and refreshing account data"));
            } else {
                assert!(status.contains("Rejected fixture"));
                assert!(!status.contains("checking orderStatus"));
            }
        }
    }
}

#[test]
fn cancel_result_success_clears_indicator_and_removes_order_locally() {
    let (mut terminal, pending_id) = terminal_with_pending_cancel();

    let _task = terminal.handle_cancel_result(
        TEST_ACCOUNT.to_string(),
        pending_id,
        Ok(cancel_exchange_response(vec![serde_json::json!("success")])),
    );

    assert!(terminal.pending_order_indicators.is_empty());
    assert!(terminal.pending_cancel_status_request.is_none());
    let data = terminal.account_data.as_ref().expect("account data");
    assert!(data.open_orders.is_empty());
    assert!(
        terminal
            .charts
            .get(&1)
            .expect("chart")
            .chart
            .active_orders
            .is_empty()
    );
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert_eq!(message, "Cancelled");
    assert!(!is_error);
}

#[test]
fn cancel_result_terminal_error_checks_status_and_keeps_local_order_until_refresh() {
    let (mut terminal, pending_id) = terminal_with_pending_cancel();

    let _task = terminal.handle_cancel_result(
        TEST_ACCOUNT.to_string(),
        pending_id,
        Ok(exchange_response(vec![serde_json::json!({
            "error": "Order was never placed, already canceled, or filled."
        })])),
    );

    assert!(terminal.pending_order_indicators.is_empty());
    assert!(terminal.pending_cancel_status_request.is_some());
    assert!(terminal.has_pending_trading_request());
    let data = terminal.account_data.as_ref().expect("account data");
    assert_eq!(data.open_orders.len(), 1);
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("Cancel may have already resolved"));
    assert!(message.contains("checking orderStatus"));
    assert!(message.contains("refreshing account data"));

    finish_current_account_refresh(&mut terminal);

    assert!(terminal.pending_cancel_status_request.is_none());
    assert!(!terminal.has_pending_trading_request());
}

#[test]
fn cancel_result_ambiguous_ack_is_uncertain_and_keeps_local_order() {
    let (mut terminal, pending_id) = terminal_with_pending_cancel();

    let _task = terminal.handle_cancel_result(
        TEST_ACCOUNT.to_string(),
        pending_id,
        Ok(malformed_ok_response()),
    );

    assert!(terminal.pending_order_indicators.is_empty());
    assert!(terminal.pending_cancel_status_request.is_some());
    assert!(terminal.has_pending_trading_request());
    let data = terminal.account_data.as_ref().expect("account data");
    assert_eq!(data.open_orders.len(), 1);
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("Cancel status unknown"));
    assert!(message.contains("refreshing account data"));

    finish_current_account_refresh(&mut terminal);

    assert!(terminal.pending_cancel_status_request.is_none());
    assert!(!terminal.has_pending_trading_request());
}

#[test]
fn cancel_result_ambiguous_ack_uses_pending_request_after_indicator_expires() {
    let (mut terminal, pending_id) = terminal_with_pending_cancel();
    terminal.pending_order_indicators.clear();

    let _task = terminal.handle_cancel_result(
        TEST_ACCOUNT.to_string(),
        pending_id,
        Ok(malformed_ok_response()),
    );

    assert!(terminal.pending_cancel_status_request.is_some());
    assert!(terminal.has_pending_trading_request());
    assert!(terminal.account_loading);
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("Cancel status unknown for order 42"));
    assert!(message.contains("checking orderStatus"));

    finish_current_account_refresh(&mut terminal);

    assert!(terminal.pending_cancel_status_request.is_none());
    assert!(!terminal.has_pending_trading_request());
}

#[test]
fn cancel_order_status_open_keeps_cancel_uncertain_and_local_order() {
    let (mut terminal, _pending_id) = terminal_with_pending_cancel();
    // A status check starts after the original cancellation indicator has
    // expired or been consumed by its exchange result.
    terminal.pending_order_indicators.clear();

    let _task = terminal.handle_cancel_order_status_result(
        TEST_ACCOUNT.to_string(),
        42,
        "BTC".to_string(),
        Ok(order_status("open")),
    );

    let data = terminal.account_data.as_ref().expect("account data");
    assert_eq!(data.open_orders.len(), 1);
    assert!(terminal.pending_cancel_status_request.is_some());
    assert!(terminal.has_pending_trading_request());
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("still uncertain"));
    assert!(message.contains("reports open"));

    finish_current_account_refresh(&mut terminal);

    assert!(terminal.pending_cancel_status_request.is_none());
    assert!(!terminal.has_pending_trading_request());
}

#[test]
fn cancel_order_status_error_redacts_sensitive_text() {
    let mut terminal = terminal_with_connected_account();
    arm_pending_cancel_status_request(&mut terminal, TEST_ACCOUNT, 42, "BTC");

    let _task = terminal.handle_cancel_order_status_result(
        TEST_ACCOUNT.to_string(),
        42,
        "BTC".to_string(),
        Err("orderStatus request failed: api_key=super-secret".to_string()),
    );

    assert!(terminal.pending_cancel_status_request.is_some());
    assert!(terminal.has_pending_trading_request());
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("Cancel status still uncertain for order 42"));
    assert!(message.contains("api_key=<redacted>"));
    assert!(!message.contains("super-secret"));

    finish_current_account_refresh(&mut terminal);

    assert!(terminal.pending_cancel_status_request.is_none());
    assert!(!terminal.has_pending_trading_request());
}

#[test]
fn cancel_order_status_without_matching_pending_request_is_ignored() {
    let (mut terminal, _pending_id) = terminal_with_pending_cancel();

    let _task = terminal.handle_cancel_order_status_result(
        TEST_ACCOUNT.to_string(),
        42,
        "ETH".to_string(),
        Ok(order_status("canceled")),
    );

    let data = terminal.account_data.as_ref().expect("account data");
    assert_eq!(data.open_orders.len(), 1);
    assert!(terminal.pending_cancel_status_request.is_some());
    assert!(terminal.order_status.is_none());
    assert!(!terminal.account_loading);
}

#[test]
fn cancel_order_status_terminal_removes_local_order() {
    let (mut terminal, _pending_id) = terminal_with_pending_cancel();

    let _task = terminal.handle_cancel_order_status_result(
        TEST_ACCOUNT.to_string(),
        42,
        "BTC".to_string(),
        Ok(order_status("canceled")),
    );

    let data = terminal.account_data.as_ref().expect("account data");
    assert!(data.open_orders.is_empty());
    assert!(terminal.pending_cancel_status_request.is_none());
    assert!(
        terminal
            .charts
            .get(&1)
            .expect("chart")
            .chart
            .active_orders
            .is_empty()
    );
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(!is_error);
    assert!(message.contains("Cancel resolved"));
}

#[test]
fn cancel_result_after_account_switch_clears_indicator_without_status() {
    let (mut terminal, pending_id) = terminal_with_pending_cancel();
    terminal.connected_address = Some(OTHER_ACCOUNT.to_string());
    terminal.order_status = None;

    let _task = terminal.handle_cancel_result(
        TEST_ACCOUNT.to_string(),
        pending_id,
        Ok(cancel_exchange_response(vec![serde_json::json!("success")])),
    );

    assert!(terminal.pending_order_indicators.is_empty());
    assert!(terminal.pending_cancel_status_request.is_none());
    assert!(terminal.order_status.is_none());
    let data = terminal.account_data.as_ref().expect("account data");
    assert_eq!(data.open_orders.len(), 1);
}

#[test]
fn cancel_result_success_removes_only_matching_symbol_for_same_oid() {
    let mut terminal = terminal_with_connected_account();
    let target_order = open_order_for(42, "flx:BTC");
    let other_order = open_order_for(42, "xyz:BTC");
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        account_data_with_open_orders(vec![target_order.clone(), other_order.clone()]),
    );
    let pending_id =
        terminal.add_pending_order_cancellation_indicator(TEST_ACCOUNT.to_string(), &target_order);
    assert!(pending_id.is_some());

    let _task = terminal.handle_cancel_result(
        TEST_ACCOUNT.to_string(),
        pending_id,
        Ok(cancel_exchange_response(vec![serde_json::json!("success")])),
    );

    let data = terminal.account_data.as_ref().expect("account data");
    assert_eq!(data.open_orders.len(), 1);
    assert_eq!(data.open_orders[0].coin, other_order.coin);
    assert_eq!(data.open_orders[0].oid, 42);
    assert!(terminal.pending_cancel_status_request.is_none());
}

#[test]
fn cancel_result_success_ignores_open_orders_from_stale_account_snapshot() {
    let (mut terminal, pending_id) = terminal_with_pending_cancel();
    terminal.account_data_address = Some(OTHER_ACCOUNT.to_string());

    let _task = terminal.handle_cancel_result(
        TEST_ACCOUNT.to_string(),
        pending_id,
        Ok(cancel_exchange_response(vec![serde_json::json!("success")])),
    );

    assert!(terminal.pending_order_indicators.is_empty());
    let data = terminal.account_data.as_ref().expect("account data");
    assert_eq!(data.open_orders.len(), 1);
    assert_eq!(data.open_orders[0].oid, 42);
    assert!(terminal.pending_cancel_status_request.is_none());
}

#[test]
fn cancel_status_terminal_removes_only_matching_symbol_for_same_oid() {
    let mut terminal = terminal_with_connected_account();
    let target_order = open_order_for(42, "flx:BTC");
    let other_order = open_order_for(42, "xyz:BTC");
    terminal.set_account_data_for_address_for_test(
        TEST_ACCOUNT,
        account_data_with_open_orders(vec![target_order.clone(), other_order.clone()]),
    );
    arm_pending_cancel_status_request(&mut terminal, TEST_ACCOUNT, 42, &target_order.coin);

    let _task = terminal.handle_cancel_order_status_result(
        TEST_ACCOUNT.to_string(),
        42,
        target_order.coin,
        Ok(order_status("canceled")),
    );

    let data = terminal.account_data.as_ref().expect("account data");
    assert_eq!(data.open_orders.len(), 1);
    assert_eq!(data.open_orders[0].coin, other_order.coin);
    assert_eq!(data.open_orders[0].oid, 42);
    assert!(terminal.pending_cancel_status_request.is_none());
}

#[test]
fn cancel_status_terminal_ignores_open_orders_from_stale_account_snapshot() {
    let (mut terminal, _pending_id) = terminal_with_pending_cancel();
    terminal.account_data_address = Some(OTHER_ACCOUNT.to_string());

    let _task = terminal.handle_cancel_order_status_result(
        TEST_ACCOUNT.to_string(),
        42,
        "BTC".to_string(),
        Ok(order_status("canceled")),
    );

    let data = terminal.account_data.as_ref().expect("account data");
    assert_eq!(data.open_orders.len(), 1);
    assert_eq!(data.open_orders[0].oid, 42);
    assert!(terminal.pending_cancel_status_request.is_none());
}
