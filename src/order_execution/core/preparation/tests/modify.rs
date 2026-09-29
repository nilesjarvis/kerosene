use super::*;

#[test]
fn prepare_modify_order_accepts_legacy_indexed_key_for_api_named_spot_pair() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols = vec![purr_spot_symbol()];
    terminal.all_mids.insert("PURR/USDC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("PURR/USDC".to_string(), TradingTerminal::now_ms());

    let prepared = terminal
        .prepare_modify_order(move_modify_intent("@0"))
        .expect("modify by legacy indexed key");

    match prepared {
        PreparedModifyOrderResult::Prepared(prepared) => {
            assert_eq!(prepared.symbol_key, "PURR/USDC");
            assert_eq!(prepared.asset, 10_000);
        }
        PreparedModifyOrderResult::NoPriceChange => panic!("expected prepared order"),
    }
}

#[test]
fn prepare_move_modify_order_rounds_price_and_preserves_known_reduce_only() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.all_mids.insert("BTC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    let prepared = terminal
        .prepare_modify_order(move_modify_intent("BTC"))
        .expect("valid modify order");

    assert_eq!(
        prepared,
        PreparedModifyOrderResult::Prepared(PreparedModifyOrder {
            surface: OrderSurface::Move,
            symbol_key: "BTC".to_string(),
            oid: 42,
            asset: 7,
            is_buy: true,
            price: "101".to_string(),
            size: "0.25".to_string(),
            reduce_only: false,
            market_type: MarketType::Perp,
        })
    );
}

#[test]
fn prepare_move_modify_order_returns_noop_when_rounded_price_is_unchanged() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.all_mids.insert("BTC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());
    let mut intent = move_modify_intent("BTC");
    intent.new_price = 100.001;

    let prepared = terminal
        .prepare_modify_order(intent)
        .expect("valid no-op modify input");

    assert_eq!(prepared, PreparedModifyOrderResult::NoPriceChange);
}

#[test]
fn prepare_move_modify_order_validates_size_price_and_reduce_only_metadata() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.all_mids.insert("BTC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    let mut invalid_size = move_modify_intent("BTC");
    invalid_size.size = "0";
    assert_eq!(
        terminal.prepare_modify_order(invalid_size).unwrap_err(),
        "Move failed: open order has invalid size"
    );

    let mut invalid_price = move_modify_intent("BTC");
    invalid_price.original_price = "0";
    assert_eq!(
        terminal.prepare_modify_order(invalid_price).unwrap_err(),
        "Move failed: open order has invalid price"
    );

    let mut missing_reduce_only = move_modify_intent("BTC");
    missing_reduce_only.reduce_only = None;
    assert!(
        terminal
            .prepare_modify_order(missing_reduce_only)
            .unwrap_err()
            .contains("reduce-only metadata is unavailable")
    );
}

#[test]
fn prepare_move_modify_order_clears_missing_reduce_only_for_spot() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols = vec![purr_spot_symbol()];
    terminal.all_mids.insert("PURR/USDC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("PURR/USDC".to_string(), TradingTerminal::now_ms());
    let mut intent = move_modify_intent("PURR/USDC");
    intent.reduce_only = None;

    let prepared = terminal
        .prepare_modify_order(intent)
        .expect("spot modify order should not need reduce-only metadata");

    match prepared {
        PreparedModifyOrderResult::Prepared(prepared) => assert!(!prepared.reduce_only),
        PreparedModifyOrderResult::NoPriceChange => panic!("expected prepared order"),
    }
}

#[test]
fn prepare_move_modify_order_keeps_outcome_contract_validation() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols = vec![outcome_symbol("#650")];
    terminal.all_mids.insert("#650".to_string(), 0.42);
    terminal
        .all_mids_updated_at_ms
        .insert("#650".to_string(), TradingTerminal::now_ms());
    let mut intent = move_modify_intent("#650");
    intent.original_price = "0.42";
    intent.new_price = 0.43;
    intent.size = "0.25";
    intent.reduce_only = None;

    let error = terminal.prepare_modify_order(intent).unwrap_err();

    assert!(error.contains("whole-contract sizes"));
}
