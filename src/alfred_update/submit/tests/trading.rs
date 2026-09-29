use super::*;

#[test]
fn alfred_trade_rejects_usd_sizing_for_outcome_markets() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![outcome_symbol("#660")];
    terminal.active_symbol = "#660".to_string();
    terminal.order_quantity = "old".to_string();
    terminal.order_quantity_is_usd = false;
    terminal.alfred.open = true;
    terminal.alfred.query = "buy $10 #660".to_string();

    let _task = terminal.submit_alfred_command(AlfredCommandId::NaturalLanguageTrading);

    assert_eq!(
        terminal.order_status,
        Some((
            "USD sizing is not supported for outcome markets; use contracts".to_string(),
            true
        ))
    );
    assert_eq!(terminal.order_quantity, "old");
    assert!(!terminal.order_quantity_is_usd);
}

#[test]
fn alfred_chase_preflight_preserves_metadata_and_quantity_error_priority() {
    for (key, quantity, is_usd, hidden, degraded, mid, expected_error) in [
        (
            "MISSING",
            "bad",
            false,
            false,
            false,
            None,
            Some("Symbol 'MISSING' not found"),
        ),
        (
            "ETH",
            "bad",
            false,
            true,
            false,
            None,
            Some("Active ticker is hidden in Settings > Risk"),
        ),
        (
            "@107",
            "bad",
            false,
            false,
            true,
            None,
            Some(
                "HYPE/USDC spot metadata is temporarily unverified; spot trading is disabled until it refreshes",
            ),
        ),
        (
            "ETH",
            "NaN",
            true,
            false,
            false,
            None,
            Some("Invalid quantity"),
        ),
        (
            "ETH",
            "0.0000001",
            false,
            false,
            false,
            None,
            Some("Invalid quantity for asset precision"),
        ),
        (
            "ETH",
            "1",
            true,
            false,
            false,
            None,
            Some(
                "Cannot start USD Chase: no fresh mid price for ETH. Wait for market data or enter size in coin units.",
            ),
        ),
        ("ETH", "1", true, false, false, Some(2_500.0), None),
        ("ETH", "1", false, false, false, None, None),
    ] {
        let mut terminal = alfred_trade_terminal();
        connect_test_account(&mut terminal);
        terminal.exchange_symbols.push(spot_symbol("@107", "HYPE"));
        terminal.spot_metadata_degraded = degraded;
        if hidden {
            terminal.muted_tickers.insert(key.into());
        }
        if let Some(mid) = mid {
            add_mid(&mut terminal, key, mid);
        }
        let before = ticket_snapshot(&terminal);

        assert_eq!(
            terminal.alfred_trade_preflight_ready(
                key,
                OrderKind::Chase,
                is_usd,
                quantity.into(),
                None,
                Some(true)
            ),
            expected_error.is_none(),
            "{key}, {quantity}",
        );
        assert_eq!(
            terminal
                .order_status
                .as_ref()
                .map(|(message, is_error)| (message.as_str(), *is_error)),
            expected_error.map(|error| (error, true))
        );
        assert_eq!(ticket_snapshot(&terminal), before);
        assert!(terminal.pending_order_action.is_none());
    }
}

#[test]
fn alfred_trade_outcome_usd_rejection_does_not_switch_or_mutate_ticket() {
    let mut terminal = alfred_trade_terminal();
    terminal.exchange_symbols.push(outcome_symbol("#660"));
    terminal.alfred.query = "buy $10 #660".to_string();
    let before = ticket_snapshot(&terminal);

    let _task = terminal.submit_alfred_command(AlfredCommandId::NaturalLanguageTrading);

    assert_eq!(ticket_snapshot(&terminal), before);
    assert_eq!(
        terminal.order_status,
        Some((
            "USD sizing is not supported for outcome markets; use contracts".to_string(),
            true
        ))
    );
}

#[test]
fn alfred_trade_missing_signing_context_does_not_switch_or_mutate_ticket() {
    let mut terminal = alfred_trade_terminal();
    add_mid(&mut terminal, "ETH", 2_500.0);
    terminal.alfred.query = "buy 1 ETH".to_string();
    let before = ticket_snapshot(&terminal);

    let _task = terminal.submit_alfred_command(AlfredCommandId::NaturalLanguageTrading);

    assert_eq!(ticket_snapshot(&terminal), before);
    assert_eq!(
        terminal.order_status,
        Some(("Connect wallet and enter agent key first".to_string(), true))
    );
    assert!(terminal.pending_order_action.is_none());
}

#[test]
fn alfred_trade_missing_signing_context_wins_over_missing_mid() {
    let mut terminal = alfred_trade_terminal();
    terminal.alfred.query = "buy 1 ETH".to_string();
    let before = ticket_snapshot(&terminal);

    let _task = terminal.submit_alfred_command(AlfredCommandId::NaturalLanguageTrading);

    assert_eq!(ticket_snapshot(&terminal), before);
    assert_eq!(
        terminal.order_status,
        Some(("Connect wallet and enter agent key first".to_string(), true))
    );
    assert!(terminal.pending_order_action.is_none());
}

#[test]
fn alfred_trade_pending_request_does_not_switch_or_mutate_ticket() {
    let mut terminal = alfred_trade_terminal();
    connect_test_account(&mut terminal);
    add_mid(&mut terminal, "ETH", 2_500.0);
    terminal.pending_order_action = Some(PendingOrderAction::Sell);
    terminal.alfred.query = "buy 1 ETH".to_string();
    let before = ticket_snapshot(&terminal);

    let _task = terminal.submit_alfred_command(AlfredCommandId::NaturalLanguageTrading);

    assert_eq!(ticket_snapshot(&terminal), before);
    assert_eq!(
        terminal.order_status,
        Some((
            "Wait for pending trading requests to finish before placing an order".to_string(),
            true
        ))
    );
    assert_eq!(
        terminal.pending_order_action,
        Some(PendingOrderAction::Sell)
    );
}

#[test]
fn alfred_trade_reconciliation_required_does_not_switch_or_mutate_ticket() {
    let mut terminal = alfred_trade_terminal();
    connect_test_account(&mut terminal);
    add_mid(&mut terminal, "ETH", 2_500.0);
    terminal.account_reconciliation_required = true;
    terminal.alfred.query = "buy 1 ETH".to_string();
    let before = ticket_snapshot(&terminal);

    let _task = terminal.submit_alfred_command(AlfredCommandId::NaturalLanguageTrading);

    assert_eq!(ticket_snapshot(&terminal), before);
    assert_eq!(
        terminal.order_status,
        Some((
            "Account refresh pending; wait for fresh account data before placing an order"
                .to_string(),
            true
        ))
    );
    assert!(terminal.pending_order_action.is_none());
}

#[test]
fn alfred_market_trade_missing_mid_does_not_switch_or_mutate_ticket() {
    let mut terminal = alfred_trade_terminal();
    connect_test_account(&mut terminal);
    terminal.alfred.query = "buy 1 ETH".to_string();
    let before = ticket_snapshot(&terminal);

    let _task = terminal.submit_alfred_command(AlfredCommandId::NaturalLanguageTrading);

    assert_eq!(ticket_snapshot(&terminal), before);
    let (status, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert!(status.starts_with("No mid price for ETH"));
    assert!(terminal.pending_order_action.is_none());
}

#[test]
fn alfred_market_trade_with_ready_context_still_updates_ticket_and_submits() {
    let mut terminal = alfred_trade_terminal();
    connect_test_account(&mut terminal);
    add_mid(&mut terminal, "ETH", 2_500.0);
    terminal.alfred.query = "buy 1 ETH".to_string();

    let _task = terminal.submit_alfred_command(AlfredCommandId::NaturalLanguageTrading);

    assert_eq!(terminal.active_symbol, "ETH");
    assert_eq!(terminal.order_kind, OrderKind::Market);
    assert_eq!(terminal.order_quantity, "1");
    assert!(!terminal.order_quantity_is_usd);
    assert_eq!(terminal.order_price, "");
    assert!(!terminal.presets_menu_expanded);
    assert!(!terminal.alfred.open);
    assert_eq!(terminal.pending_order_action, Some(PendingOrderAction::Buy));
}

#[test]
fn alfred_spot_pair_trade_switches_to_and_submits_on_the_spot_market() {
    let mut terminal = alfred_trade_terminal();
    connect_test_account(&mut terminal);
    // A same-ticker perp exists: the explicit pair spelling must still
    // target the spot market, never the perp.
    terminal
        .exchange_symbols
        .push(symbol("HYPE", MarketType::Perp));
    terminal.exchange_symbols.push(spot_symbol("@107", "HYPE"));
    add_mid(&mut terminal, "@107", 25.0);
    terminal.alfred.query = "sell 1 hype/usdc".to_string();

    let _task = terminal.submit_alfred_command(AlfredCommandId::NaturalLanguageTrading);

    assert_eq!(terminal.active_symbol, "@107");
    assert_eq!(terminal.active_symbol_display, "HYPE/USDC");
    assert_eq!(terminal.order_kind, OrderKind::Market);
    assert_eq!(terminal.order_quantity, "1");
    assert!(!terminal.alfred.open);
    assert_eq!(
        terminal.pending_order_action,
        Some(PendingOrderAction::Sell)
    );
}

#[test]
fn alfred_twap_preflight_preserves_existing_start_path() {
    let mut terminal = alfred_trade_terminal();
    connect_test_account(&mut terminal);

    assert!(terminal.alfred_trade_preflight_ready(
        "ETH",
        OrderKind::Twap,
        false,
        "1".to_string(),
        None,
        Some(true),
    ));
    assert_eq!(terminal.order_status, None);
}
