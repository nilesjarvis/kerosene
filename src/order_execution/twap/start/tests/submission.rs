use super::*;

#[test]
fn start_twap_keeps_base_size_when_quantity_is_not_usd() {
    let mut terminal = twap_ready_terminal();

    let _task = terminal.start_twap(true);

    let twap = started_twap_or_panic(&terminal);
    assert_eq!(twap.target_size, 2.5);
    assert_eq!(twap.slice_count, 2);
    assert_eq!(twap.min_price, 90.0);
    assert_eq!(twap.max_price, 110.0);
    assert_eq!(twap.status, TwapStatus::WaitingForMarket);
}

#[test]
fn spot_percentage_twap_sell_uses_exact_base_balance_not_rounded_usd_display() {
    let mut terminal = twap_ready_terminal();
    configure_spot_percentage_twap(&mut terminal, "30014.285", "0", 0.00035);
    terminal.twap_form.slices = "1".to_string();
    terminal.twap_form.min_price = "0.00035".to_string();
    terminal.twap_form.max_price = "0.00036".to_string();
    assert_ne!(terminal.order_quantity, "30014.285");

    let _task = terminal.start_twap(false);

    let twap = started_twap_or_panic(&terminal);
    assert_eq!(twap.target_size, 30_014.28);
    assert!(twap.target_size <= 30_014.285);
}

#[test]
fn spot_percentage_twap_buy_cannot_exceed_fractional_quote_balance_at_max_price() {
    let mut terminal = twap_ready_terminal();
    configure_spot_percentage_twap(&mut terminal, "0", "10.505", 0.00035);
    terminal.twap_form.slices = "1".to_string();
    terminal.twap_form.min_price = "0.00035".to_string();
    terminal.twap_form.max_price = "0.00036".to_string();
    assert_ne!(terminal.order_quantity, "10.505");

    let _task = terminal.start_twap(true);

    let twap = started_twap_or_panic(&terminal);
    assert!(twap.target_size * twap.max_price <= 10.505 + 1e-12);
}

#[test]
fn spot_twap_stops_before_dispatch_when_live_metadata_replaces_pair_identity() {
    let mut terminal = twap_ready_terminal();
    configure_spot_percentage_twap(&mut terminal, "100", "0", 1.0);
    terminal.twap_form.slices = "1".to_string();
    terminal.twap_form.min_price = "0.9".to_string();
    terminal.twap_form.max_price = "1.1".to_string();
    let _task = terminal.start_twap(false);
    let twap_id = terminal.selected_twap_id.expect("started TWAP");
    terminal
        .twap_orders
        .get_mut(&twap_id)
        .expect("TWAP")
        .latest_book = Some(TwapBookSnapshot {
        book: OrderBook {
            bids: vec![BookLevel {
                px: 0.99,
                sz: 100.0,
            }],
            asks: vec![BookLevel {
                px: 1.01,
                sz: 100.0,
            }],
        },
        updated_at: Instant::now(),
    });
    terminal.exchange_symbols[0].ticker = "OTHER".to_string();
    terminal.exchange_symbols[0].display_name = Some("OTHER/USDC".to_string());

    let _task = terminal.execute_due_twap_slice(twap_id, Instant::now());

    let twap = terminal.twap_orders.get(&twap_id).expect("TWAP retained");
    assert_eq!(twap.status, TwapStatus::Stopped);
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| {
                *is_error && message.contains("spot market identity changed")
            })
    );
}

#[test]
fn start_twap_from_snapshot_accepts_matching_click_context() {
    let mut terminal = twap_ready_terminal();
    let snapshot = terminal.twap_order_start_snapshot();

    let _task = terminal.start_twap_from_snapshot(true, snapshot);

    let twap = started_twap_or_panic(&terminal);
    assert_eq!(twap.target_size, 2.5);
    assert_eq!(twap.slice_count, 2);
    assert!(twap.is_buy);
}

#[test]
fn start_twap_from_snapshot_rejects_changed_click_context() {
    let mut terminal = twap_ready_terminal();
    let snapshot = terminal.twap_order_start_snapshot();
    terminal.twap_form.slices = "3".to_string();

    let _task = terminal.start_twap_from_snapshot(true, snapshot);

    assert!(terminal.twap_orders.is_empty());
    assert_eq!(terminal.pending_order_action, None);
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| {
                *is_error && message.contains("TWAP settings changed")
            }),
        "status should explain the stale click context: {:?}",
        terminal.order_status
    );
}

#[test]
fn start_twap_rejects_duplicate_start_within_window() {
    let mut terminal = twap_ready_terminal();

    let _task = terminal.start_twap(true);
    assert_eq!(terminal.twap_orders.len(), 1);

    // start_twap is synchronous, so a queued double click replays it
    // immediately; the duplicate-start window must absorb the second press.
    let _task = terminal.start_twap(true);

    assert_eq!(terminal.twap_orders.len(), 1);
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| *is_error && message.contains("just started"))
    );
}

#[test]
fn start_twap_allows_opposite_side_despite_recent_start() {
    let mut terminal = twap_ready_terminal();

    let _task = terminal.start_twap(true);
    let _task = terminal.start_twap(false);

    assert_eq!(terminal.twap_orders.len(), 2);
}

#[test]
fn start_twap_rejects_usd_notional_without_fresh_mid() {
    let mut terminal = twap_ready_terminal();
    terminal.order_quantity = "1000".to_string();
    terminal.order_quantity_is_usd = true;

    let _task = terminal.start_twap(true);

    assert!(terminal.twap_orders.is_empty());
    assert_eq!(terminal.pending_order_action, None);
    assert!(
        terminal
            .order_status
            .as_ref()
            .is_some_and(|(message, is_error)| {
                *is_error && message.contains("Cannot start USD TWAP: no fresh mid price")
            })
    );
}

#[test]
fn start_twap_rejects_usd_notional_for_crypto_quoted_spot() {
    let mut terminal = twap_ready_terminal();
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

    let _task = terminal.start_twap(true);

    assert!(terminal.twap_orders.is_empty());
    assert_eq!(terminal.pending_order_action, None);
    assert!(order_status_error_contains(
        &terminal,
        "quote-token USD valuation and accounting are not verified"
    ));

    terminal.order_status = None;
    terminal.order_quantity_is_usd = false;
    let _task = terminal.start_twap(true);
    assert!(terminal.twap_orders.is_empty());
    assert!(order_status_error_contains(
        &terminal,
        "Spot trading is unavailable for HYPE/UETH"
    ));
}

#[test]
fn start_twap_rejects_non_orderable_fallback_outcome() {
    let mut terminal = twap_ready_terminal();
    terminal.active_symbol = "#66".to_string();
    terminal.active_symbol_display = "Recurring Fallback".to_string();
    terminal.exchange_symbols = vec![fallback_outcome_symbol("#66")];

    let _task = terminal.start_twap(true);

    assert!(terminal.twap_orders.is_empty());
    assert_eq!(terminal.pending_order_action, None);
    assert!(
        order_status_error_contains(&terminal, "not a tradable market"),
        "status should explain the non-orderable symbol: {:?}",
        terminal.order_status
    );
}
