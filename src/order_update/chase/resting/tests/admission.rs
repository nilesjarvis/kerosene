use super::*;
use crate::signing::ChaseLifecycle;

fn spot_terminal(mut order: OpenOrder) -> TradingTerminal {
    order.coin = "@7".into();
    let mut terminal = terminal_with_open_order(order);
    terminal.exchange_symbols = vec![ExchangeSymbol {
        key: "@7".into(),
        ticker: "SYNTH".into(),
        display_name: Some("SYNTH/USDC".into()),
        asset_index: 10_007,
        collateral_token: Some(crate::api::USDC_TOKEN_INDEX),
        market_type: MarketType::Spot,
        ..btc_symbol()
    }];
    terminal.spot_metadata_degraded = false;
    terminal
}

#[test]
fn resting_admission_keeps_wire_numeric_and_market_error_precedence() {
    for (faults, expected) in [
        (
            1 | 2 | 4 | 8 | 16 | 32,
            "Cannot chase order: trigger orders cannot be chased safely yet",
        ),
        (
            2 | 4 | 8 | 16 | 32,
            "Cannot chase order: order type cannot be chased safely yet",
        ),
        (
            4 | 8 | 16 | 32,
            "Cannot chase order: non-GTC orders cannot be chased safely yet",
        ),
        (
            8 | 16 | 32,
            "Cannot chase order: open order has invalid side",
        ),
        (16 | 32, "Cannot chase order with invalid size"),
        (32, "Cannot chase order with invalid price"),
        (0, "Symbol 'BTC' not found"),
    ] {
        let mut order = open_order(42);
        order.is_trigger = Some(faults & 1 != 0);
        order.order_type = Some(if faults & 2 != 0 { "Market" } else { "Limit" }.into());
        order.tif = Some(if faults & 4 != 0 { "Ioc" } else { "Gtc" }.into());
        if faults & 8 != 0 {
            order.side = "bad".into();
        }
        if faults & 16 != 0 {
            order.sz = "NaN".into();
        }
        if faults & 32 != 0 {
            order.limit_px = "0".into();
        }
        let mut terminal = terminal_with_open_order(order);
        terminal.exchange_symbols.clear();
        let next_id = terminal.next_chase_id;

        let task = terminal.handle_chase_resting_order("BTC".into(), 42);

        assert_eq!(task.units(), 0);
        assert!(terminal.chase_orders.is_empty());
        assert!(terminal.chase_spot_symbol_identities.is_empty());
        assert_eq!(terminal.next_chase_id, next_id);
        assert_eq!(terminal.order_status, Some((expected.into(), true)));
        assert_eq!(
            terminal.toasts.last().expect("error toast").message,
            expected
        );
        assert!(!terminal.account_loading);
    }

    for degraded in [true, false] {
        let mut order = open_order(42);
        order.reduce_only = None;
        let mut terminal = spot_terminal(order);
        terminal.spot_metadata_degraded = degraded;
        terminal.exchange_symbols[0].display_name = Some("SYNTH/OTHER".into());
        let expected = if degraded {
            "Cannot chase spot order until metadata is verified".to_string()
        } else {
            "Spot trading is unavailable for SYNTH/OTHER because quote-token USD valuation and accounting are not verified".to_string()
        };
        assert_eq!(
            terminal.handle_chase_resting_order("@7".into(), 42).units(),
            0
        );
        assert_eq!(terminal.order_status, Some((expected, true)));
        assert!(terminal.chase_orders.is_empty());
        assert!(terminal.chase_spot_symbol_identities.is_empty());
    }
}

#[test]
fn resting_admission_keeps_owned_state_rounding_cutoff_and_id_wrap() {
    for (is_spot, side, reduce_only, expected_price, expected_wire) in [
        (false, "B", Some(true), 0.12, "0.12"),
        (false, "A", Some(false), 0.12, "0.12"),
        (true, "B", None, 0.1235, "0.1235"),
        (true, "A", Some(true), 0.1235, "0.1235"),
    ] {
        let mut order = open_order(42);
        order.side = side.into();
        order.sz = "0.12567".into();
        order.limit_px = "0.123456".into();
        order.reduce_only = reduce_only;
        let mut terminal = if is_spot {
            spot_terminal(order)
        } else {
            terminal_with_open_order(order)
        };
        let coin = if is_spot { "@7" } else { "BTC" };
        terminal.next_chase_id = u64::MAX;
        let before_ms = TradingTerminal::now_ms();
        let before = std::time::Instant::now();

        assert_eq!(
            terminal.handle_chase_resting_order(coin.into(), 42).units(),
            0
        );

        assert_eq!(terminal.next_chase_id, 1);
        assert_eq!(terminal.selected_chase_id, Some(u64::MAX));
        assert_eq!(terminal.chase_orders.len(), 1);
        let chase = &terminal.chase_orders[&u64::MAX];
        assert_eq!(chase.id, u64::MAX);
        assert_eq!(chase.coin, coin);
        assert_eq!(chase.account_address, TEST_ACCOUNT);
        assert_eq!(chase.is_buy, side == "B");
        assert_eq!(chase.is_spot, is_spot);
        assert_eq!(chase.reduce_only, !is_spot && reduce_only == Some(true));
        assert_eq!(chase.asset, if is_spot { 10_007 } else { 0 });
        assert_eq!(chase.sz_decimals, 4);
        assert_eq!(
            (chase.target_size, chase.remaining_size, chase.filled_size),
            (0.12567, 0.12567, 0.0)
        );
        assert_eq!(
            (chase.current_price, chase.initial_price),
            (expected_price, expected_price)
        );
        assert_eq!(chase.current_price_wire, expected_wire);
        assert_eq!(chase.current_oid, Some(42));
        assert_eq!(chase.known_oids, [42]);
        assert!(chase.current_cloid.is_none());
        assert_eq!(chase.lifecycle, ChaseLifecycle::Resting);
        assert_eq!(
            (
                chase.place_attempt_count,
                chase.reprice_count,
                chase.cancel_retries
            ),
            (0, 0, 0)
        );
        assert!(chase.last_reprice_at.is_none());
        assert!(chase.desired_price.is_none());
        assert!(chase.stop_reason.is_none());
        assert!(chase.started_at >= before && chase.started_at <= std::time::Instant::now());
        assert!(
            chase.started_at_ms >= before_ms && chase.started_at_ms <= TradingTerminal::now_ms()
        );
        assert_eq!(
            chase.fill_cutoff_ms_by_oid,
            [(42, chase.started_at_ms.saturating_sub(60_000))]
        );
        assert_eq!(
            terminal.chase_spot_symbol_identities.len(),
            usize::from(is_spot)
        );
        assert_eq!(
            terminal.order_status,
            Some(("Chasing resting order 42...".into(), false))
        );
        let source = &terminal
            .account_data
            .as_ref()
            .expect("source snapshot")
            .open_orders[0];
        assert_eq!(source.limit_px, "0.123456");
        assert_eq!(source.sz, "0.12567");
        assert_eq!(source.reduce_only, reduce_only);
        assert!(terminal.last_advanced_exchange_request_at.is_none());
    }
}

#[test]
fn adopted_spot_identity_retains_only_execution_relevant_metadata() {
    let mut terminal = spot_terminal(open_order(42));
    assert_eq!(
        terminal.handle_chase_resting_order("@7".into(), 42).units(),
        0
    );
    let id = terminal.selected_chase_id.expect("adopted Chase");
    let symbol = terminal.exchange_symbols[0].clone();
    assert!(terminal.chase_spot_symbol_identity_is_current(id, "@7"));
    for changed_field in 0..6 {
        terminal.exchange_symbols[0] = symbol.clone();
        let current = &mut terminal.exchange_symbols[0];
        match changed_field {
            0 => current.key = "@8".into(),
            1 => current.ticker = "OTHER".into(),
            2 => current.display_name = Some("OTHER/USDC".into()),
            3 => current.asset_index += 1,
            4 => current.collateral_token = None,
            _ => current.sz_decimals += 1,
        }
        assert!(!terminal.chase_spot_symbol_identity_is_current(id, "@7"));
    }
    terminal.exchange_symbols[0] = symbol;
    let current = &mut terminal.exchange_symbols[0];
    current.category = "other".into();
    current.keywords.push("new-search-label".into());
    current.max_leverage += 1;
    assert!(terminal.chase_spot_symbol_identity_is_current(id, "@7"));
}
