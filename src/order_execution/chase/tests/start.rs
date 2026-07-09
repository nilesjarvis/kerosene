use super::*;

#[test]
fn start_chase_keeps_base_size_when_quantity_is_not_usd() {
    let mut terminal = chase_ready_terminal();

    let _task = terminal.start_chase(true);

    let chase = selected_chase(&terminal);
    assert_eq!(chase.target_size, 2.5);
    assert_eq!(chase.remaining_size, 2.5);
}

#[test]
fn start_chase_from_snapshot_accepts_matching_click_context() {
    let mut terminal = chase_ready_terminal();
    let snapshot = terminal.advanced_order_start_snapshot();

    let _task = terminal.start_chase_from_snapshot(true, snapshot);

    let chase = selected_chase(&terminal);
    assert_eq!(chase.target_size, 2.5);
    assert!(chase.is_buy);
}

#[test]
fn start_chase_from_snapshot_rejects_changed_click_context() {
    let mut terminal = chase_ready_terminal();
    let snapshot = terminal.advanced_order_start_snapshot();
    terminal.order_quantity = "3.5".to_string();

    let _task = terminal.start_chase_from_snapshot(true, snapshot);

    assert!(terminal.chase_orders.is_empty());
    assert_eq!(terminal.pending_order_action, None);
    assert!(
        order_status_error_contains(&terminal, "Chase settings changed"),
        "status should explain the stale click context: {:?}",
        terminal.order_status
    );
}

#[test]
fn start_chase_quantizes_base_size_to_asset_precision() {
    let mut terminal = chase_ready_terminal();
    terminal.order_quantity = "1.239".to_string();
    if let Some(symbol) = terminal.exchange_symbols.first_mut() {
        symbol.sz_decimals = 2;
    }

    let _task = terminal.start_chase(true);

    let chase = selected_chase(&terminal);
    assert_eq!(chase.target_size, 1.23);
    assert_eq!(chase.remaining_size, 1.23);
}

#[test]
fn spot_percentage_chase_sell_uses_exact_base_balance_not_rounded_usd_display() {
    let mut terminal = chase_ready_terminal();
    configure_spot_percentage_chase(&mut terminal, "100", "0", 0.00035);
    assert_ne!(terminal.order_quantity, "100");

    let _task = terminal.start_chase(false);

    let chase = selected_chase(&terminal);
    assert_eq!(chase.target_size, 100.0);
}

#[test]
fn spot_percentage_chase_rejects_first_book_above_reserved_drift_budget() {
    let mut terminal = chase_ready_terminal();
    configure_spot_percentage_chase(&mut terminal, "0", "100", 100.0);

    let _task = terminal.start_chase(true);
    let chase_id = selected_chase_id(&terminal);
    assert_eq!(selected_chase(&terminal).initial_price, 100.0);

    let _task = terminal.chase_place_at_best(chase_id, 106.0);

    assert!(terminal.chase_orders.is_empty());
    assert!(order_status_error_contains(
        &terminal,
        "price drift limit exceeded"
    ));
}

#[test]
fn spot_chase_stops_before_dispatch_when_live_metadata_replaces_pair_identity() {
    let mut terminal = chase_ready_terminal();
    configure_spot_percentage_chase(&mut terminal, "100", "0", 1.0);
    let _task = terminal.start_chase(false);
    let chase_id = selected_chase_id(&terminal);

    terminal.exchange_symbols[0].ticker = "OTHER".to_string();
    terminal.exchange_symbols[0].display_name = Some("OTHER/USDC".to_string());
    let _task = terminal.chase_place_at_best(chase_id, 1.0);

    assert!(terminal.chase_orders.is_empty());
    assert!(order_status_error_contains(
        &terminal,
        "spot market identity changed"
    ));
}

#[test]
fn start_chase_converts_usd_notional_to_base_size_using_fresh_mid() {
    let mut terminal = chase_ready_terminal();
    terminal.order_quantity = "1000".to_string();
    terminal.order_quantity_is_usd = true;
    terminal.all_mids.insert("BTC".to_string(), 50_000.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    let _task = terminal.start_chase(true);

    let chase = selected_chase(&terminal);
    assert!((chase.target_size - 0.02).abs() < f64::EPSILON);
    assert!((chase.remaining_size - 0.02).abs() < f64::EPSILON);
}

#[test]
fn start_chase_converts_usd_notional_using_mid_candidates() {
    let mut terminal = chase_ready_terminal();
    terminal.active_symbol = "UBTC".to_string();
    terminal.active_symbol_display = "UBTC".to_string();
    terminal.exchange_symbols = vec![symbol("UBTC", MarketType::Perp)];
    terminal.order_quantity = "1000".to_string();
    terminal.order_quantity_is_usd = true;
    terminal.all_mids.insert("BTC".to_string(), 50_000.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    let _task = terminal.start_chase(true);

    let chase = selected_chase(&terminal);
    assert!((chase.target_size - 0.02).abs() < f64::EPSILON);
    assert!((chase.remaining_size - 0.02).abs() < f64::EPSILON);
}

#[test]
fn start_chase_rejects_usd_notional_without_fresh_mid() {
    let mut terminal = chase_ready_terminal();
    terminal.order_quantity = "1000".to_string();
    terminal.order_quantity_is_usd = true;

    let _task = terminal.start_chase(true);

    assert!(terminal.chase_orders.is_empty());
    assert_eq!(terminal.pending_order_action, None);
    assert!(
        order_status_error_contains(&terminal, "no fresh mid price"),
        "status should explain the missing fresh mid: {:?}",
        terminal.order_status
    );
}

#[test]
fn start_chase_rejects_usd_notional_with_stale_mid() {
    let mut terminal = chase_ready_terminal();
    terminal.order_quantity = "1000".to_string();
    terminal.order_quantity_is_usd = true;
    terminal.all_mids.insert("BTC".to_string(), 50_000.0);
    terminal.all_mids_updated_at_ms.insert("BTC".to_string(), 0);

    let _task = terminal.start_chase(true);

    assert!(terminal.chase_orders.is_empty());
    assert_eq!(terminal.pending_order_action, None);
}

#[test]
fn start_chase_rejects_usd_notional_for_crypto_quoted_spot() {
    let mut terminal = chase_ready_terminal();
    terminal.active_symbol = "@151".to_string();
    terminal.active_symbol_display = "HYPE/UETH".to_string();
    terminal.exchange_symbols = vec![ExchangeSymbol {
        ticker: "HYPE".to_string(),
        category: "spot".to_string(),
        display_name: Some("HYPE/UETH".to_string()),
        asset_index: 10_151,
        collateral_token: Some(221),
        sz_decimals: 2,
        max_leverage: 1,
        market_type: MarketType::Spot,
        ..symbol("@151", MarketType::Spot)
    }];
    terminal.order_quantity = "10".to_string();
    terminal.order_quantity_is_usd = true;
    terminal.all_mids.insert("@151".to_string(), 2.0);
    terminal
        .all_mids_updated_at_ms
        .insert("@151".to_string(), TradingTerminal::now_ms());

    let _task = terminal.start_chase(true);

    assert!(terminal.chase_orders.is_empty());
    assert_eq!(terminal.pending_order_action, None);
    assert!(order_status_error_contains(
        &terminal,
        "quote-token USD valuation and accounting are not verified"
    ));

    terminal.order_status = None;
    terminal.order_quantity_is_usd = false;
    let _task = terminal.start_chase(true);
    assert!(terminal.chase_orders.is_empty());
    assert!(order_status_error_contains(
        &terminal,
        "Spot trading is unavailable for HYPE/UETH"
    ));
}

#[test]
fn start_chase_rejects_non_orderable_fallback_outcome() {
    let mut terminal = chase_ready_terminal();
    terminal.active_symbol = "#66".to_string();
    terminal.active_symbol_display = "Recurring Fallback".to_string();
    terminal.exchange_symbols = vec![fallback_outcome_symbol("#66")];

    let _task = terminal.start_chase(true);

    assert!(terminal.chase_orders.is_empty());
    assert_eq!(terminal.pending_order_action, None);
    assert!(
        order_status_error_contains(&terminal, "not a tradable market"),
        "status should explain the non-orderable symbol: {:?}",
        terminal.order_status
    );
}
