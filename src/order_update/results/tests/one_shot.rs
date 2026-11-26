use super::*;
use crate::order_execution::TicketWait;

#[test]
fn one_shot_ambiguous_outcome_sets_cloid_reconciliation_status() {
    let mut terminal = terminal_with_connected_account();

    let _task = terminal.apply_one_shot_placement_outcome(
        one_shot_context(),
        ExecutionOutcome {
            kind: ExecutionOutcomeKind::TransportUnknown,
            status: "exchange request failed".to_string(),
            is_error: true,
            refresh_account: true,
        },
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("Ticket placement status unknown for BTC"));
    assert!(message.contains("exchange request failed"));
    assert!(message.contains("checking 0x00000000000000000000000000000000"));
}

#[test]
fn one_shot_mixed_exchange_error_starts_cloid_reconciliation() {
    let mut terminal = terminal_with_connected_account();

    let _task = terminal.handle_order_result(
        None,
        one_shot_context(),
        Ok(exchange_response(vec![
            serde_json::json!({
                "resting": {
                    "oid": 42_u64
                }
            }),
            serde_json::json!({
                "error": "Second order rejected"
            }),
        ])),
    );

    assert!(terminal.has_pending_one_shot_status_requests_for_test());
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("Ticket placement status unknown for BTC"));
    assert!(message.contains("Resting (oid 42)"));
    assert!(message.contains("Error: Second order rejected"));
}

#[test]
fn one_shot_order_status_result_normalizes_terminal_statuses() {
    let mut terminal = terminal_with_connected_account();
    let context = one_shot_context();
    let request_id = begin_one_shot_status_request(&mut terminal, &context);

    let _task = terminal.handle_one_shot_placement_status_result(
        request_id,
        context,
        Ok(order_status("open")),
    );
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(!is_error);
    assert!(message.contains("Ticket placement confirmed by orderStatus for BTC"));
    assert!(terminal.pending_one_shot_status_requests.is_empty());

    let context = one_shot_context();
    let request_id = begin_one_shot_status_request(&mut terminal, &context);
    let _task = terminal.handle_one_shot_placement_status_result(
        request_id,
        context,
        Ok(order_status("rejected")),
    );
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("Ticket placement rejected according to orderStatus for BTC"));
    assert!(terminal.pending_one_shot_status_requests.is_empty());
}

#[test]
fn ticket_missing_status_stays_unconfirmed_after_account_refresh() {
    let mut terminal = terminal_with_connected_account();
    let context = one_shot_context();
    let request_id = begin_one_shot_status_request(&mut terminal, &context);

    let _task = terminal.handle_one_shot_placement_status_result(
        request_id,
        context,
        Ok(order_status("unknownOid")),
    );

    assert!(terminal.has_pending_one_shot_status_requests_for_test());
    assert!(terminal.has_pending_trading_request());
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("placement status still uncertain"));

    finish_current_account_refresh(&mut terminal);

    assert!(
        matches!(terminal.standard_ticket_wait(), Some(TicketWait::OrderStatus(check)) if check.lookup == PlacementLookup::Unconfirmed)
    );
    assert!(!terminal.has_pending_trading_request());
}

#[test]
fn one_shot_canceled_status_stays_pending_until_account_refresh() {
    let mut terminal = terminal_with_connected_account();
    let context = one_shot_context();
    let request_id = begin_one_shot_status_request(&mut terminal, &context);

    let _task = terminal.handle_one_shot_placement_status_result(
        request_id,
        context,
        Ok(order_status("canceled")),
    );

    assert!(terminal.has_pending_one_shot_status_requests_for_test());
    assert!(terminal.has_pending_trading_request());
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("BTC order canceled"));
    assert!(message.contains("partial fills"));

    finish_current_account_refresh(&mut terminal);

    assert!(terminal.pending_one_shot_status_requests.is_empty());
    assert!(!terminal.has_pending_trading_request());
}

#[test]
fn ticket_status_error_stays_unconfirmed_after_account_refresh() {
    let mut terminal = terminal_with_connected_account();
    let context = one_shot_context();
    let request_id = begin_one_shot_status_request(&mut terminal, &context);

    let _task = terminal.handle_one_shot_placement_status_result(
        request_id,
        context,
        Err("orderStatus request failed: private_key=super-secret".to_string()),
    );

    assert!(terminal.has_pending_one_shot_status_requests_for_test());
    assert!(terminal.has_pending_trading_request());
    assert!(terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("placement status still uncertain"));
    assert!(message.contains("orderStatus request failed"));
    assert!(message.contains("private_key=<redacted>"));
    assert!(!message.contains("super-secret"));

    finish_current_account_refresh(&mut terminal);

    assert!(
        matches!(terminal.standard_ticket_wait(), Some(TicketWait::OrderStatus(check)) if check.lookup == PlacementLookup::Unconfirmed)
    );
    assert!(!terminal.has_pending_trading_request());
}

#[test]
fn one_shot_status_result_with_stale_request_id_is_ignored() {
    let mut terminal = terminal_with_connected_account();
    let context = one_shot_context();
    let request_id = begin_one_shot_status_request(&mut terminal, &context);

    let _task = terminal.handle_one_shot_placement_status_result(
        request_id.wrapping_add(1),
        context,
        Ok(order_status("open")),
    );

    assert!(terminal.order_status.is_none());
    assert_eq!(
        terminal
            .pending_one_shot_status_requests
            .values()
            .next()
            .map(|request| request.request_id),
        Some(request_id)
    );
}

#[test]
fn older_status_request_survives_newer_unrelated_outcome() {
    // Regression: with one shared slot, a second placement outcome used to
    // clobber the first placement's ambiguity check and its orderStatus
    // response was silently dropped. Each cloid must resolve independently.
    let mut terminal = terminal_with_connected_account();
    let old_context = one_shot_context_with_kind(ExchangeOrderKind::Market);
    let _task = terminal.apply_one_shot_placement_outcome(
        old_context.clone(),
        ExecutionOutcome {
            kind: ExecutionOutcomeKind::TransportUnknown,
            status: "exchange request failed".to_string(),
            is_error: true,
            refresh_account: false,
        },
    );
    let old_request_id = terminal
        .pending_one_shot_status_requests
        .values()
        .next()
        .expect("status request should be pending")
        .request_id;

    let newer_context = one_shot_context_with_cloid(
        "0x00000000000000000000000000000001",
        ExchangeOrderKind::Limit,
    );
    let _task = terminal.apply_one_shot_placement_outcome(
        newer_context,
        ExecutionOutcome {
            kind: ExecutionOutcomeKind::Filled,
            status: "Filled avgPx=100 totalSz=1".to_string(),
            is_error: false,
            refresh_account: false,
        },
    );
    assert!(
        terminal.has_pending_one_shot_status_requests_for_test(),
        "unrelated outcome must not clear the older ambiguity check"
    );

    let _task = terminal.handle_one_shot_placement_status_result(
        old_request_id,
        old_context,
        Ok(order_status("open")),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("unexpectedly rested"));
    assert!(terminal.pending_one_shot_status_requests.is_empty());
}

#[test]
fn one_shot_ioc_like_order_status_open_is_unexpected_resting_error() {
    for order_kind in [ExchangeOrderKind::Market, ExchangeOrderKind::LimitIoc] {
        let mut terminal = terminal_with_connected_account();
        let context = one_shot_context_with_kind(order_kind);
        let request_id = begin_one_shot_status_request(&mut terminal, &context);

        let _task = terminal.handle_one_shot_placement_status_result(
            request_id,
            context,
            Ok(order_status("open")),
        );

        let (message, is_error) = terminal.order_status.clone().expect("status should be set");
        assert!(is_error);
        assert!(message.contains("Ticket"));
        assert!(message.contains(order_kind.label()));
        assert!(message.contains("unexpectedly rested"));
        assert!(message.contains("cancel 0x00000000000000000000000000000000"));
    }
}

#[test]
fn one_shot_ioc_like_direct_resting_response_is_unexpected_resting_error() {
    let mut terminal = terminal_with_connected_account();

    let _task = terminal.handle_order_result(
        None,
        one_shot_context_with_kind(ExchangeOrderKind::Market),
        Ok(exchange_response(vec![serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        })])),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(is_error);
    assert!(message.contains("Ticket market order unexpectedly rested for BTC"));
    assert!(message.contains("Resting (oid 42)"));
}

#[test]
fn one_shot_limit_direct_resting_response_remains_successful() {
    let mut terminal = terminal_with_connected_account();

    let _task = terminal.handle_order_result(
        None,
        one_shot_context_with_kind(ExchangeOrderKind::Limit),
        Ok(exchange_response(vec![serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        })])),
    );

    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(!is_error);
    assert_eq!(message, "BTC · Resting (oid 42)");
}

#[test]
fn one_shot_success_during_refresh_backoff_marks_reconciliation_required() {
    let mut terminal = terminal_with_connected_account();
    terminal.account_refresh_backoff_until_ms = Some(TradingTerminal::now_ms() + 60_000);

    let _task = terminal.handle_order_result(
        None,
        one_shot_context_with_kind(ExchangeOrderKind::Limit),
        Ok(exchange_response(vec![serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        })])),
    );

    assert!(!terminal.account_loading);
    assert!(terminal.account_reconciliation_required);
    assert!(
        terminal
            .account_error
            .as_deref()
            .is_some_and(|error| error.contains("rate limited"))
    );
    let (message, is_error) = terminal.order_status.clone().expect("status should be set");
    assert!(!is_error);
    assert_eq!(message, "BTC · Resting (oid 42)");
}

#[test]
fn one_shot_statuses_use_outcome_display_label_not_raw_key() {
    let mut terminal = terminal_with_connected_account();
    let sym = outcome_exchange_symbol("#660");
    let label = TradingTerminal::exchange_symbol_display_name(&sym);
    terminal.exchange_symbols = vec![sym];

    let _task = terminal.apply_one_shot_placement_outcome(
        one_shot_outcome_context("#660"),
        ExecutionOutcome {
            kind: ExecutionOutcomeKind::TransportUnknown,
            status: "exchange request failed".to_string(),
            is_error: true,
            refresh_account: true,
        },
    );
    let (message, _) = terminal.order_status.clone().expect("status should be set");
    assert!(message.contains(&format!("placement status unknown for {label}")));
    assert!(!message.contains("#660"));

    let context = one_shot_outcome_context("#660");
    let request_id = begin_one_shot_status_request(&mut terminal, &context);
    let _task = terminal.handle_one_shot_placement_status_result(
        request_id,
        context,
        Ok(order_status("open")),
    );
    let (message, _) = terminal.order_status.clone().expect("status should be set");
    assert!(message.contains(&format!("confirmed by orderStatus for {label}")));
    assert!(!message.contains("#660"));
}

#[test]
fn one_shot_results_are_ignored_after_account_switch() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some(OTHER_ACCOUNT.to_string());

    let _task = terminal.apply_one_shot_placement_outcome(
        one_shot_context(),
        ExecutionOutcome {
            kind: ExecutionOutcomeKind::AcceptedResting,
            status: "Resting (oid 42)".to_string(),
            is_error: false,
            refresh_account: true,
        },
    );

    assert!(terminal.order_status.is_none());

    let context = one_shot_context();
    let request_id = begin_one_shot_status_request(&mut terminal, &context);
    let _task = terminal.handle_one_shot_placement_status_result(
        request_id,
        context,
        Ok(order_status("open")),
    );

    assert!(terminal.order_status.is_none());
}

#[test]
fn ticket_status_lookup_survives_refresh_and_rechecks_only_the_original_order() {
    for surface in [OrderSurface::Ticket, OrderSurface::Preset] {
        let mut terminal = terminal_with_connected_account();
        let context = OneShotPlacementContext {
            surface,
            ..one_shot_context()
        };
        let _ =
            terminal.handle_order_result(None, context.clone(), Err("request timed out".into()));
        let initial = terminal.pending_one_shot_status_requests[&0].request_id;
        finish_current_account_refresh(&mut terminal);
        assert!(
            matches!(terminal.standard_ticket_wait(), Some(TicketWait::OrderStatus(check)) if check.lookup == PlacementLookup::Checking)
        );
        assert_eq!(terminal.recheck_ticket_order_status(initial).units(), 0);

        let mut wrong_order = order_status("filled");
        wrong_order.cloid = Some("another-order".into());
        let _ = terminal.handle_one_shot_placement_status_result(
            initial,
            context.clone(),
            Ok(wrong_order),
        );
        finish_current_account_refresh(&mut terminal);
        terminal.handle_dismiss_order_status();
        assert!(terminal.reject_if_standard_ticket_waiting());
        assert!(terminal.pending_order_action.is_none());
        assert!(terminal.pending_order_indicators.is_empty());

        terminal.connected_address = Some(OTHER_ACCOUNT.into());
        terminal.retain_unconfirmed_ticket_checks();
        assert!(terminal.standard_ticket_wait().is_none());
        assert_eq!(terminal.recheck_ticket_order_status(initial).units(), 0);
        terminal.connected_address = Some(TEST_ACCOUNT.into());
        let task = terminal.recheck_ticket_order_status(initial);
        assert_eq!(task.units(), 1);
        assert_eq!(terminal.recheck_ticket_order_status(initial).units(), 0);
        let (&id, check) = terminal
            .pending_one_shot_status_requests
            .first_key_value()
            .expect("pending lookup");
        assert_eq!(check.context, context);
        assert_ne!(id, initial);
        assert!(terminal.pending_order_action.is_none());
        assert!(terminal.pending_order_indicators.is_empty());

        let _ = terminal.handle_one_shot_placement_status_result(
            initial,
            context.clone(),
            Ok(order_status("open")),
        );
        assert!(matches!(
            terminal.standard_ticket_wait(),
            Some(TicketWait::OrderStatus(_))
        ));
        let _ = terminal.handle_one_shot_placement_status_result(
            id,
            context,
            Ok(order_status("filled")),
        );
        assert_eq!(
            terminal.standard_ticket_wait(),
            Some(TicketWait::Refreshing)
        );
        finish_current_account_refresh(&mut terminal);
        assert!(terminal.standard_ticket_wait().is_none());
        assert!(
            terminal
                .order_status
                .as_ref()
                .is_some_and(|(msg, error)| !error && msg.contains("filled"))
        );
        assert!(terminal.pending_order_action.is_none());
    }
}

#[test]
fn account_refresh_keeps_existing_recovery_policy_for_other_order_surfaces() {
    let mut terminal = terminal_with_connected_account();
    let context = OneShotPlacementContext {
        surface: OrderSurface::Hud,
        ..one_shot_context()
    };
    let id = terminal.begin_one_shot_status_request(&context);
    assert_eq!(
        terminal.standard_ticket_wait(),
        Some(TicketWait::TradingRequest)
    );
    finish_current_account_refresh(&mut terminal);
    assert!(terminal.pending_one_shot_status_requests.is_empty());
    assert!(terminal.standard_ticket_wait().is_none());
    assert_eq!(terminal.recheck_ticket_order_status(id).units(), 0);
}
