use super::*;

#[test]
fn failed_perp_bootstrap_blocks_perp_placement_but_not_spot_placement_state() {
    const ADDRESS: &str = "0xabc0000000000000000000000000000000000000";
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some(ADDRESS.to_string());
    terminal.set_account_data_for_address_for_test(ADDRESS, incomplete_perp_account_data());

    assert_eq!(
        terminal.validate_place_account_market_state(OrderSurface::Ticket, MarketType::Perp),
        Err(
            "Perpetual account state is incomplete; refresh account data before placing an order"
                .to_string()
        )
    );
    assert_eq!(
        terminal.validate_place_account_market_state(OrderSurface::Ticket, MarketType::Spot),
        Ok(())
    );
    assert_eq!(
        terminal.validate_place_account_market_state(OrderSurface::Cluster, MarketType::Perp),
        Ok(()),
        "cluster legs validate their own member snapshots"
    );
}

#[test]
fn prepare_ticket_limit_order_converts_usd_size_and_reduce_only_for_perps() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.all_mids.insert("BTC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    let prepared = terminal
        .prepare_place_order(ticket_limit_intent("BTC"))
        .expect("valid prepared order");

    assert_eq!(
        prepared,
        PreparedExchangeOrder {
            surface: OrderSurface::Ticket,
            symbol_key: "BTC".to_string(),
            asset: 7,
            is_buy: true,
            price: "100.12".to_string(),
            size: "2.5019".to_string(),
            order_kind: ExchangeOrderKind::Limit,
            reduce_only: true,
            market_type: MarketType::Perp,
        }
    );
}

#[test]
fn prepare_quick_market_usd_order_sizes_from_mid_not_slipped_execution_price() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.market_slippage_pct = 5.0;
    terminal.all_mids.insert("BTC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    for surface in [OrderSurface::QuickOrder, OrderSurface::QuickTrade] {
        let buy = terminal
            .prepare_place_order(market_usd_intent(
                surface,
                MarketUsdSizeReference::Mid,
                true,
            ))
            .expect("valid quick market buy");
        let sell = terminal
            .prepare_place_order(market_usd_intent(
                surface,
                MarketUsdSizeReference::Mid,
                false,
            ))
            .expect("valid quick market sell");

        assert_eq!(buy.price, "105");
        assert_eq!(sell.price, "95");
        assert_eq!(buy.size, "2.5");
        assert_eq!(sell.size, "2.5");
    }
}

#[test]
fn prepare_ticket_market_usd_order_can_size_from_slipped_execution_price() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.market_slippage_pct = 5.0;
    terminal.all_mids.insert("BTC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    let prepared = terminal
        .prepare_place_order(market_usd_intent(
            OrderSurface::Ticket,
            MarketUsdSizeReference::ExecutionPrice,
            true,
        ))
        .expect("valid ticket market buy");

    assert_eq!(prepared.price, "105");
    assert_eq!(prepared.size, "2.3809");
}

#[test]
fn spot_percentage_sell_uses_exact_base_balance_at_sub_cent_prices() {
    let (mut terminal, _) = TradingTerminal::boot();
    let mut spot = symbol("@7", MarketType::Spot);
    spot.ticker = "LOW".to_string();
    spot.display_name = Some("LOW/USDC".to_string());
    spot.collateral_token = Some(crate::api::USDC_TOKEN_INDEX);
    terminal.exchange_symbols = vec![spot];
    terminal.all_mids.insert("@7".to_string(), 0.00035);
    terminal
        .all_mids_updated_at_ms
        .insert("@7".to_string(), TradingTerminal::now_ms());

    let prepared = terminal
        .prepare_place_order(PlaceIntent {
            surface: OrderSurface::Ticket,
            symbol_key: "@7".to_string(),
            is_buy: false,
            order_kind: ExchangeOrderKind::Limit,
            price_source: PriceSource::LimitInput {
                value: "0.00035".to_string(),
                invalid_message: "Invalid price",
            },
            quantity_source: QuantitySource::SpotPercentageBalance {
                available_balance: 100.0,
                percentage: 100.0,
                invalid_message: "Invalid spot percentage balance",
                precision_invalid_message: "Invalid spot percentage size",
            },
            reduce_only_source: ReduceOnlySource::Form(false),
        })
        .expect("exact spot percentage sell should prepare");

    let size: f64 = prepared.size.parse().expect("wire size");
    assert!(size <= 100.0);
    assert_eq!(size, 100.0);
}

#[test]
fn spot_percentage_buy_cannot_spend_more_than_fractional_quote_balance() {
    let (mut terminal, _) = TradingTerminal::boot();
    let mut spot = symbol("@7", MarketType::Spot);
    spot.ticker = "LOW".to_string();
    spot.display_name = Some("LOW/USDC".to_string());
    spot.collateral_token = Some(crate::api::USDC_TOKEN_INDEX);
    terminal.exchange_symbols = vec![spot];
    terminal.market_slippage_pct = 5.0;
    terminal.all_mids.insert("@7".to_string(), 0.35);
    terminal
        .all_mids_updated_at_ms
        .insert("@7".to_string(), TradingTerminal::now_ms());
    let quote_balance = 1.000_001;

    let prepared = terminal
        .prepare_place_order(PlaceIntent {
            surface: OrderSurface::QuickOrder,
            symbol_key: "@7".to_string(),
            is_buy: true,
            order_kind: ExchangeOrderKind::Market,
            price_source: PriceSource::MarketWithSlippage {
                invalid_message: Some("Invalid market price"),
                usd_size_reference: MarketUsdSizeReference::Mid,
            },
            quantity_source: QuantitySource::SpotPercentageBalance {
                available_balance: quote_balance,
                percentage: 100.0,
                invalid_message: "Invalid spot percentage balance",
                precision_invalid_message: "Invalid spot percentage size",
            },
            reduce_only_source: ReduceOnlySource::Form(false),
        })
        .expect("exact spot percentage buy should prepare");

    let size: f64 = prepared.size.parse().expect("wire size");
    let price: f64 = prepared.price.parse().expect("wire price");
    assert!(size * price <= quote_balance + 1e-12);
}

#[test]
fn prepare_limit_order_rejects_prices_that_round_to_zero() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.all_mids.insert("BTC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());
    let mut intent = ticket_limit_intent("BTC");
    intent.price_source = PriceSource::LimitInput {
        value: "0.0000001".to_string(),
        invalid_message: "Invalid price",
    };

    let error = terminal.prepare_place_order(intent).unwrap_err();

    assert_eq!(error, "Invalid price");
}

#[test]
fn prepare_market_order_rejects_prices_that_round_to_zero() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.all_mids.insert("BTC".to_string(), 0.0000001);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    let error = terminal
        .prepare_place_order(market_usd_intent(
            OrderSurface::QuickOrder,
            MarketUsdSizeReference::Mid,
            true,
        ))
        .unwrap_err();

    assert_eq!(error, "Invalid market price");
}

#[test]
fn prepare_market_order_missing_mid_error_leads_with_outcome_display_label() {
    let (mut terminal, _) = TradingTerminal::boot();
    let sym = outcome_symbol("#650");
    let label = TradingTerminal::exchange_symbol_display_name(&sym);
    terminal.exchange_symbols = vec![sym];
    terminal.all_mids.clear();
    terminal.all_mids_updated_at_ms.clear();
    let mut intent = ticket_limit_intent("#650");
    intent.order_kind = ExchangeOrderKind::Market;
    intent.price_source = PriceSource::MarketWithSlippage {
        invalid_message: Some("Invalid market price"),
        usd_size_reference: MarketUsdSizeReference::Mid,
    };
    intent.quantity_source = QuantitySource::UserInput {
        value: "3".to_string(),
        denomination: QuantityDenomination::Coin,
        invalid_message: "Invalid quantity",
        precision_invalid_message: "Invalid quantity for asset precision",
    };

    let error = terminal.prepare_place_order(intent).unwrap_err();

    assert!(error.starts_with(&format!("No mid price for {label}")));
    assert!(error.contains("(tried #650"));
}

#[test]
fn prepare_ticket_outcome_order_forces_coin_quantity_and_clears_reduce_only() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols = vec![outcome_symbol("#650")];
    terminal.all_mids.insert("#650".to_string(), 0.42);
    terminal
        .all_mids_updated_at_ms
        .insert("#650".to_string(), TradingTerminal::now_ms());
    let mut intent = ticket_limit_intent("#650");
    intent.price_source = PriceSource::LimitInput {
        value: "0.421234".to_string(),
        invalid_message: "Invalid price",
    };
    intent.quantity_source = QuantitySource::UserInput {
        value: "3".to_string(),
        denomination: QuantityDenomination::UsdNotional,
        invalid_message: "Invalid quantity",
        precision_invalid_message: "Invalid quantity for asset precision",
    };

    let prepared = terminal
        .prepare_place_order(intent)
        .expect("valid outcome order");

    assert_eq!(prepared.price, "0.42123");
    assert_eq!(prepared.size, "3");
    assert!(!prepared.reduce_only);
    assert_eq!(prepared.market_type, MarketType::Outcome);
}

#[test]
fn prepare_close_position_limit_uses_reference_mid_and_fixed_reduce_only() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols = vec![symbol("BTC", MarketType::Perp)];
    terminal.all_mids.insert("BTC".to_string(), 100.123456);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    let prepared = terminal
        .prepare_place_order(PlaceIntent {
            surface: OrderSurface::ClosePosition,
            symbol_key: "BTC".to_string(),
            is_buy: false,
            order_kind: ExchangeOrderKind::Limit,
            price_source: PriceSource::ReferenceMid,
            quantity_source: QuantitySource::CoinSize {
                size: 1.239,
                invalid_message: "Position size is invalid",
                precision_invalid_message: "Position size is invalid",
            },
            reduce_only_source: ReduceOnlySource::Fixed(true),
        })
        .expect("valid close-position order");

    assert_eq!(prepared.price, "100.12");
    assert_eq!(prepared.size, "1.239");
    assert!(prepared.reduce_only);
}

#[test]
fn prepare_close_position_rejects_outcomes_through_capability_policy() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols = vec![outcome_symbol("#650")];
    terminal.all_mids.insert("#650".to_string(), 0.42);
    terminal
        .all_mids_updated_at_ms
        .insert("#650".to_string(), TradingTerminal::now_ms());

    let error = terminal
        .prepare_place_order(PlaceIntent {
            surface: OrderSurface::ClosePosition,
            symbol_key: "#650".to_string(),
            is_buy: false,
            order_kind: ExchangeOrderKind::Limit,
            price_source: PriceSource::ReferenceMid,
            quantity_source: QuantitySource::CoinSize {
                size: 1.0,
                invalid_message: "Position size is invalid",
                precision_invalid_message: "Position size is invalid",
            },
            reduce_only_source: ReduceOnlySource::Fixed(true),
        })
        .unwrap_err();

    assert_eq!(
        error,
        "Outcome position closing is not available from this control; use the main order ticket"
    );
}

#[test]
fn metadata_free_indexed_spot_fallback_is_not_used_for_placement() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.exchange_symbols.clear();

    let error = terminal
        .prepare_place_order(PlaceIntent {
            surface: OrderSurface::Ticket,
            symbol_key: "@107".to_string(),
            is_buy: true,
            order_kind: ExchangeOrderKind::Market,
            price_source: PriceSource::ReferenceMid,
            quantity_source: QuantitySource::CoinSize {
                size: 1.0,
                invalid_message: "Invalid quantity",
                precision_invalid_message: "Invalid quantity for asset precision",
            },
            reduce_only_source: ReduceOnlySource::Form(false),
        })
        .expect_err("placement must still require exchange metadata");

    assert_eq!(error, "Symbol '@107' not found in exchange metadata");
}

#[test]
fn spot_market_order_rejects_fresh_same_ticker_perp_mid() {
    let (mut terminal, _) = TradingTerminal::boot();
    let mut spot = symbol("@107", MarketType::Spot);
    spot.ticker = "HYPE".to_string();
    spot.display_name = Some("HYPE/USDC".to_string());
    spot.asset_index = 10_107;
    terminal.exchange_symbols = vec![symbol("HYPE", MarketType::Perp), spot];
    terminal.all_mids.insert("HYPE".to_string(), 40.0);
    terminal
        .all_mids_updated_at_ms
        .insert("HYPE".to_string(), TradingTerminal::now_ms());

    let error = terminal
        .prepare_place_order(PlaceIntent {
            surface: OrderSurface::Ticket,
            symbol_key: "@107".to_string(),
            is_buy: true,
            order_kind: ExchangeOrderKind::Market,
            price_source: PriceSource::MarketWithSlippage {
                invalid_message: Some("Invalid market price"),
                usd_size_reference: MarketUsdSizeReference::Mid,
            },
            quantity_source: QuantitySource::UserInput {
                value: "100".to_string(),
                denomination: QuantityDenomination::UsdNotional,
                invalid_message: "Invalid quantity",
                precision_invalid_message: "Invalid quantity for asset precision",
            },
            reduce_only_source: ReduceOnlySource::Form(false),
        })
        .expect_err("spot orders must not use a perpetual mid");

    assert_eq!(error, "No mid price for HYPE/USDC (tried @107)");
}

#[test]
fn non_usd_quoted_spot_rejects_all_placement_denominations() {
    let (mut terminal, _) = TradingTerminal::boot();
    let mut spot = symbol("@55", MarketType::Spot);
    spot.ticker = "UETH".to_string();
    spot.display_name = Some("UETH/UBTC".to_string());
    spot.asset_index = 10_055;
    terminal.exchange_symbols = vec![spot];
    terminal.all_mids.insert("@55".to_string(), 0.05);
    terminal
        .all_mids_updated_at_ms
        .insert("@55".to_string(), TradingTerminal::now_ms());

    let usd_error = terminal
        .prepare_place_order(PlaceIntent {
            surface: OrderSurface::Ticket,
            symbol_key: "@55".to_string(),
            is_buy: true,
            order_kind: ExchangeOrderKind::Market,
            price_source: PriceSource::MarketWithSlippage {
                invalid_message: Some("Invalid market price"),
                usd_size_reference: MarketUsdSizeReference::Mid,
            },
            quantity_source: QuantitySource::UserInput {
                value: "100".to_string(),
                denomination: QuantityDenomination::UsdNotional,
                invalid_message: "Invalid quantity",
                precision_invalid_message: "Invalid quantity for asset precision",
            },
            reduce_only_source: ReduceOnlySource::Form(false),
        })
        .expect_err("a crypto-quoted pair has no safe USD conversion");

    assert_eq!(
        usd_error,
        "Spot trading is unavailable for UETH/UBTC because quote-token USD valuation and accounting are not verified"
    );

    let coin_error = terminal
        .prepare_place_order(PlaceIntent {
            surface: OrderSurface::Ticket,
            symbol_key: "@55".to_string(),
            is_buy: true,
            order_kind: ExchangeOrderKind::Market,
            price_source: PriceSource::MarketWithSlippage {
                invalid_message: Some("Invalid market price"),
                usd_size_reference: MarketUsdSizeReference::Mid,
            },
            quantity_source: QuantitySource::CoinSize {
                size: 1.2345,
                invalid_message: "Invalid quantity",
                precision_invalid_message: "Invalid quantity for asset precision",
            },
            reduce_only_source: ReduceOnlySource::Form(false),
        })
        .expect_err("coin size must not bypass unverified quote accounting");

    assert_eq!(coin_error, usd_error);
}
