use super::*;

#[test]
fn serialized_placement_results_preserve_cleanup_account_gates_and_context() {
    let handlers = [
        TradingTerminal::handle_order_result,
        TradingTerminal::handle_close_position_result,
        TradingTerminal::handle_quick_trade_order_result,
    ];
    for (handle, surface) in handlers.into_iter().zip([
        OrderSurface::Ticket,
        OrderSurface::ClosePosition,
        OrderSurface::QuickTrade,
    ]) {
        for (result, unknown, refresh, is_error) in [
            (
                Ok(exchange_response(vec![
                    serde_json::json!({"resting": {"oid": 42}}),
                ])),
                false,
                true,
                false,
            ),
            (
                Ok(exchange_response(vec![
                    serde_json::json!({"error": "Rejected fixture"}),
                ])),
                false,
                false,
                true,
            ),
            (
                Ok(ExchangeResponse::local_rejection(
                    "Builder fee is not approved",
                )),
                false,
                true,
                true,
            ),
            (Ok(malformed_ok_response()), true, true, true),
            (Err("transport failed".to_string()), true, true, true),
        ] {
            for current_account in [false, true] {
                let mut terminal = terminal_with_connected_account();
                terminal.order_status = Some(("previous status".to_string(), false));
                terminal.pending_order_action =
                    Some(crate::order_execution::PendingOrderAction::Sell);
                let pending_id = terminal.add_pending_order_placement_indicator(
                    TEST_ACCOUNT.to_string(),
                    "BTC".to_string(),
                    true,
                    "1".to_string(),
                    "100".to_string(),
                );
                let other_id = terminal
                    .add_pending_order_placement_indicator(
                        TEST_ACCOUNT.to_string(),
                        "ETH".to_string(),
                        false,
                        "2".to_string(),
                        "200".to_string(),
                    )
                    .expect("unrelated indicator");
                if !current_account {
                    terminal.connected_address = Some(OTHER_ACCOUNT.to_string());
                }
                let context = OneShotPlacementContext {
                    surface,
                    ..one_shot_context()
                };
                let label = context.placement_label();
                let task = handle(&mut terminal, pending_id, context, result.clone());

                assert!(terminal.pending_order_action.is_none());
                assert_eq!(terminal.pending_order_indicators.len(), 1);
                assert!(terminal.pending_order_indicators.contains_key(&other_id));
                assert_eq!(
                    terminal.account_reconciliation_required,
                    current_account && refresh
                );
                assert_eq!(
                    terminal.pending_one_shot_status_requests.len(),
                    usize::from(current_account && unknown)
                );
                if !current_account {
                    assert_eq!(
                        terminal.order_status.as_ref(),
                        Some(&("previous status".to_string(), false))
                    );
                    assert_eq!(task.units(), 0);
                } else {
                    let (status, error) =
                        terminal.order_status.as_ref().expect("completion status");
                    assert_eq!(*error, is_error);
                    if unknown {
                        assert!(
                            status
                                .starts_with(&format!("{label} placement status unknown for BTC:"))
                        );
                        assert_eq!(
                            terminal
                                .pending_one_shot_status_requests
                                .values()
                                .next()
                                .expect("status request")
                                .surface(),
                            surface
                        );
                    }
                    assert_eq!(task.units() > 0, refresh);
                }
            }
        }
    }
}

#[test]
fn local_builder_rejection_refreshes_spot_balances_without_unknown_order_reconciliation() {
    for surface in [
        OrderSurface::Ticket,
        OrderSurface::QuickOrder,
        OrderSurface::QuickTrade,
        OrderSurface::Hud,
    ] {
        let mut terminal = terminal_with_connected_account();
        let mut data = account_data_with_open_orders(Vec::new());
        data.mark_spot_balances_fetched_at(TradingTerminal::now_ms());
        terminal.account_data = Some(data);
        terminal.invalidate_spot_balances_after_exchange_dispatch(TEST_ACCOUNT, MarketType::Spot);
        let context = OneShotPlacementContext {
            symbol_key: "@1".into(),
            surface,
            ..one_shot_context()
        };
        let result = Ok(ExchangeResponse::local_rejection(
            "Builder fee is not approved",
        ));

        let task = match surface {
            OrderSurface::QuickOrder => {
                terminal.handle_quick_order_result(None, context, None, result)
            }
            OrderSurface::QuickTrade => {
                terminal.handle_quick_trade_order_result(None, context, result)
            }
            OrderSurface::Hud => terminal.handle_hud_order_result(None, None, context, result),
            _ => terminal.handle_order_result(None, context, result),
        };

        assert!(task.units() > 0);
        assert!(terminal.account_loading);
        assert!(terminal.account_reconciliation_required);
        assert!(terminal.pending_one_shot_status_requests.is_empty());
        assert!(
            !terminal
                .account_data
                .as_ref()
                .expect("account snapshot")
                .completeness
                .spot_balances_complete
        );

        // Read completion, rather than the rejection itself, restores freshness.
        let mut fresh = account_data_with_open_orders(Vec::new());
        fresh.mark_spot_balances_fetched_at(TradingTerminal::now_ms());
        let refresh_context = terminal.current_account_data_request_context();
        let _ = terminal.apply_account_data_loaded(TEST_ACCOUNT.into(), refresh_context, Ok(fresh));
        assert!(
            terminal
                .account_data
                .as_ref()
                .expect("fresh snapshot")
                .completeness
                .spot_balances_complete
        );
    }
}

#[test]
fn order_result_clears_pending_indicator() {
    let mut terminal = terminal_with_connected_account();
    terminal.charts.clear();
    terminal
        .charts
        .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));
    let pending_id = terminal.add_pending_order_placement_indicator(
        TEST_ACCOUNT.to_string(),
        "BTC".to_string(),
        true,
        "1".to_string(),
        "100".to_string(),
    );
    assert!(pending_id.is_some());

    let _task = terminal.handle_order_result(
        pending_id,
        one_shot_context(),
        Ok(exchange_response(vec![serde_json::json!({
            "resting": {
                "oid": 42_u64
            }
        })])),
    );

    assert!(terminal.pending_order_indicators.is_empty());
}

#[test]
fn close_position_result_clears_pending_indicator() {
    let mut terminal = terminal_with_connected_account();
    terminal.charts.clear();
    terminal
        .charts
        .insert(1, ChartInstance::new(1, "BTC".to_string(), Timeframe::H1));
    let pending_id = terminal.add_pending_market_order_placement_indicator(
        TEST_ACCOUNT.to_string(),
        "BTC".to_string(),
        false,
        "1".to_string(),
        "100".to_string(),
    );
    assert!(pending_id.is_some());
    assert!(
        terminal
            .charts
            .get(&1)
            .expect("chart")
            .chart
            .hud_order_animation_active()
    );

    let _task = terminal.handle_close_position_result(
        pending_id,
        one_shot_context(),
        Ok(exchange_response(vec![serde_json::json!({
            "filled": {
                "totalSz": "1",
                "avgPx": "100",
                "oid": 43_u64
            }
        })])),
    );

    assert!(terminal.pending_order_indicators.is_empty());
    assert!(
        !terminal
            .charts
            .get(&1)
            .expect("chart")
            .chart
            .hud_order_animation_active()
    );
}
