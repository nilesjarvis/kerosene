use super::*;

#[test]
fn alfred_close_preflight_preserves_fraction_and_size_error_priority() {
    for size in ["2", "-2", " 2 ", "NaN", "inf", "bad", "0", "1e-13"] {
        for (fraction, invalid_fraction) in
            [(0.5, false), (0.0, true), (1.1, true), (f64::NAN, true)]
        {
            let mut terminal = alfred_close_terminal(TradingTerminal::now_ms());
            let mut data = account_data_with_position("BTC", TradingTerminal::now_ms());
            data.clearinghouse.asset_positions[0].position.szi = size.into();
            terminal.set_account_data_for_address_for_test(TEST_ACCOUNT, data);
            add_mid(&mut terminal, "BTC", 50_000.0);
            let expected_error = if invalid_fraction {
                Some("Close fraction is invalid")
            } else if matches!(size, "2" | "-2" | " 2 ") {
                None
            } else {
                Some("Position size is invalid")
            };

            let task = terminal.alfred_close_position_preflight_task("BTC", fraction);

            assert_eq!(
                task.is_some(),
                expected_error.is_some(),
                "{size}, {fraction}"
            );
            assert_eq!(
                terminal
                    .order_status
                    .as_ref()
                    .map(|(message, is_error)| (message.as_str(), *is_error)),
                expected_error.map(|error| (error, true))
            );
            assert!(terminal.alfred.open);
            assert_eq!(terminal.alfred.query, "close BTC");
            assert_eq!(terminal.close_menu_coin.as_deref(), Some("BTC"));
            assert!(terminal.pending_order_action.is_none());

            if expected_error.is_none() {
                terminal.alfred.query = "close BTC 50%".into();
                let _task = terminal.submit_alfred_command(AlfredCommandId::ClosePosition);
                assert!(!terminal.alfred.open);
                assert!(terminal.alfred.query.is_empty());
                assert!(terminal.close_menu_coin.is_none());
                assert_eq!(
                    terminal.pending_order_action,
                    Some(PendingOrderAction::ClosePosition)
                );
            }
        }
    }
}

#[test]
fn alfred_nuke_keeps_palette_open_until_second_confirmation() {
    for query in ["nuke", "close all"] {
        let mut terminal = alfred_close_terminal(TradingTerminal::now_ms());
        add_mid(&mut terminal, "BTC", 50_000.0);
        terminal.alfred.query = query.into();

        let _first_task = terminal.submit_alfred_command(AlfredCommandId::NukePositions);

        assert!(terminal.alfred.open);
        assert_eq!(terminal.alfred.query, query);
        assert!(terminal.nuke_confirmation.is_some());
        assert!(terminal.pending_nuke_execution.is_none());

        let _second_task = terminal.submit_alfred_command(AlfredCommandId::NukePositions);

        assert!(!terminal.alfred.open);
        assert!(terminal.alfred.query.is_empty());
        assert!(terminal.nuke_confirmation.is_none());
        assert!(terminal.pending_nuke_execution.is_some());
    }
}

#[test]
fn alfred_close_pending_request_does_not_close_or_clear_close_menu() {
    let mut terminal = alfred_close_terminal(TradingTerminal::now_ms());
    add_mid(&mut terminal, "BTC", 50_000.0);
    terminal.pending_order_action = Some(PendingOrderAction::Buy);

    let _task = terminal.submit_alfred_command(AlfredCommandId::ClosePosition);

    assert!(terminal.alfred.open);
    assert_eq!(terminal.close_menu_coin.as_deref(), Some("BTC"));
    assert_eq!(
        terminal.order_status,
        Some((
            "Wait for pending trading requests to finish before closing positions".to_string(),
            true
        ))
    );
    assert_eq!(terminal.pending_order_action, Some(PendingOrderAction::Buy));
}

#[test]
fn alfred_close_missing_signing_context_does_not_close_or_clear_close_menu() {
    let mut terminal = alfred_close_terminal(TradingTerminal::now_ms());
    terminal.accounts[0].agent_key = sensitive_string("").into_zeroizing();
    add_mid(&mut terminal, "BTC", 50_000.0);

    let _task = terminal.submit_alfred_command(AlfredCommandId::ClosePosition);

    assert!(terminal.alfred.open);
    assert_eq!(terminal.close_menu_coin.as_deref(), Some("BTC"));
    assert_eq!(
        terminal.order_status,
        Some(("Connect wallet and enter agent key first".to_string(), true))
    );
    assert!(terminal.pending_order_action.is_none());
}

#[test]
fn alfred_close_missing_signing_context_wins_over_account_loading() {
    let mut terminal = alfred_close_terminal(TradingTerminal::now_ms());
    terminal.accounts[0].agent_key = sensitive_string("").into_zeroizing();
    terminal.account_loading = true;
    add_mid(&mut terminal, "BTC", 50_000.0);

    let _task = terminal.alfred_close_position_preflight_task("BTC", 1.0);

    assert!(terminal.alfred.open);
    assert_eq!(terminal.close_menu_coin.as_deref(), Some("BTC"));
    assert_eq!(
        terminal.order_status,
        Some(("Connect wallet and enter agent key first".to_string(), true))
    );
    assert!(terminal.pending_order_action.is_none());
}

#[test]
fn alfred_close_stale_account_does_not_close_or_clear_close_menu() {
    let mut terminal = alfred_close_terminal(1);
    add_mid(&mut terminal, "BTC", 50_000.0);

    let _task = terminal.submit_alfred_command(AlfredCommandId::ClosePosition);

    assert!(terminal.alfred.open);
    assert_eq!(terminal.close_menu_coin.as_deref(), Some("BTC"));
    let (status, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(status.contains("Account data is stale"));
    assert!(status.contains("refresh before closing positions"));
    assert!(terminal.account_loading);
    assert!(terminal.pending_order_action.is_none());
}

#[test]
fn alfred_close_missing_mid_does_not_close_or_clear_close_menu() {
    let mut terminal = alfred_close_terminal(TradingTerminal::now_ms());

    let _task = terminal.submit_alfred_command(AlfredCommandId::ClosePosition);

    assert!(terminal.alfred.open);
    assert_eq!(terminal.close_menu_coin.as_deref(), Some("BTC"));
    let (status, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(status.starts_with("No mid price for BTC"));
    assert!(terminal.pending_order_action.is_none());
}
