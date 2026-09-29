use super::*;

#[test]
fn place_intent_debug_redacts_symbol_price_and_quantity() {
    let intent = PlaceIntent {
        surface: OrderSurface::Ticket,
        symbol_key: "SECRETCOIN".to_string(),
        is_buy: true,
        order_kind: ExchangeOrderKind::Limit,
        price_source: PriceSource::LimitInput {
            value: "price-secret".to_string(),
            invalid_message: "Invalid price",
        },
        quantity_source: QuantitySource::UserInput {
            value: "quantity-secret".to_string(),
            denomination: QuantityDenomination::UsdNotional,
            invalid_message: "Invalid quantity",
            precision_invalid_message: "Invalid quantity for asset precision",
        },
        reduce_only_source: ReduceOnlySource::Form(true),
    };

    let rendered = format!("{intent:?}");

    assert!(rendered.contains("symbol_key: <redacted>"));
    assert!(rendered.contains("value: <redacted>"));
    assert!(rendered.contains("denomination: UsdNotional"));
    for secret in ["SECRETCOIN", "price-secret", "quantity-secret"] {
        assert!(!rendered.contains(secret), "{secret} leaked in {rendered}");
    }
}

#[test]
fn prepared_order_debug_redacts_symbols_prices_sizes_and_ids() {
    let cancel = PreparedCancelOrder {
        surface: OrderSurface::Cancel,
        symbol_key: "CANCELSECRET".to_string(),
        asset: 7,
        oid: 123456789,
        market_type: MarketType::Perp,
    };
    let modify = PreparedModifyOrder {
        surface: OrderSurface::Move,
        symbol_key: "MODIFYSECRET".to_string(),
        oid: 987654321,
        asset: 8,
        is_buy: false,
        price: "modify-price-secret".to_string(),
        size: "modify-size-secret".to_string(),
        reduce_only: true,
        market_type: MarketType::Perp,
    };
    let place = PreparedExchangeOrder {
        surface: OrderSurface::Ticket,
        symbol_key: "PLACESECRET".to_string(),
        asset: 9,
        is_buy: true,
        price: "place-price-secret".to_string(),
        size: "place-size-secret".to_string(),
        order_kind: ExchangeOrderKind::Limit,
        reduce_only: false,
        market_type: MarketType::Perp,
    };

    let rendered = format!("{cancel:?} {modify:?} {place:?}");

    assert!(rendered.contains("symbol_key: <redacted>"));
    assert!(rendered.contains("oid: <redacted>"));
    assert!(rendered.contains("price: <redacted>"));
    assert!(rendered.contains("size: <redacted>"));
    for secret in [
        "CANCELSECRET",
        "MODIFYSECRET",
        "PLACESECRET",
        "123456789",
        "987654321",
        "modify-price-secret",
        "modify-size-secret",
        "place-price-secret",
        "place-size-secret",
    ] {
        assert!(!rendered.contains(secret), "{secret} leaked in {rendered}");
    }
}

#[test]
fn one_shot_placement_context_debug_redacts_account_address() {
    const ACCOUNT: &str = "0xabc0000000000000000000000000000000000000";
    const CLOID: &str = "0xdeadbeef";
    const SYMBOL: &str = "SECRETCOIN";
    let context = OneShotPlacementContext {
        account_address: ACCOUNT.to_string(),
        cloid: CLOID.to_string(),
        surface: OrderSurface::Ticket,
        symbol_key: SYMBOL.to_string(),
        order_kind: ExchangeOrderKind::Limit,
    };

    let rendered = format!("{context:?}");

    assert!(rendered.contains("<redacted>"));
    assert!(!rendered.contains(ACCOUNT));
    assert!(!rendered.contains(CLOID));
    assert!(!rendered.contains(SYMBOL));
}

#[test]
fn prepared_order_can_build_request_with_strategy_supplied_cloid() {
    let order = PreparedExchangeOrder {
        surface: OrderSurface::Chase,
        symbol_key: "BTC".to_string(),
        asset: 0,
        is_buy: true,
        price: "100".to_string(),
        size: "1".to_string(),
        order_kind: ExchangeOrderKind::Limit,
        reduce_only: false,
        market_type: MarketType::Perp,
    };

    let request =
        order.place_request_with_existing_cloid("0x11111111111111111111111111111111".to_string());

    assert_eq!(request.asset, 0);
    assert_eq!(request.price, "100");
    assert_eq!(request.size, "1");
    assert_eq!(
        request.cloid,
        Some("0x11111111111111111111111111111111".to_string())
    );
}

#[test]
fn prepared_requests_preserve_all_fields_context_and_independent_ownership() {
    const ADDRESS: &str = " 0x1111111111111111111111111111111111111111 ";
    let mut cloids = std::collections::HashSet::new();
    for surface in [
        OrderSurface::Ticket,
        OrderSurface::Preset,
        OrderSurface::QuickOrder,
        OrderSurface::QuickTrade,
        OrderSurface::Hud,
        OrderSurface::ClosePosition,
        OrderSurface::Cluster,
        OrderSurface::ClusterClose,
        OrderSurface::Nuke,
        OrderSurface::Chase,
        OrderSurface::Twap,
        OrderSurface::Move,
        OrderSurface::Cancel,
    ] {
        for order_kind in [
            ExchangeOrderKind::Market,
            ExchangeOrderKind::Limit,
            ExchangeOrderKind::LimitIoc,
        ] {
            for (is_buy, reduce_only) in
                [(false, false), (false, true), (true, false), (true, true)]
            {
                let order = PreparedExchangeOrder {
                    surface,
                    symbol_key: "xyz:TEST".into(),
                    asset: 17,
                    is_buy,
                    price: "12.30".into(),
                    size: "0.450".into(),
                    order_kind,
                    reduce_only,
                    market_type: MarketType::Perp,
                };
                let (request, context) = order.place_request_with_context(ADDRESS);
                let supplied = order.place_request_with_existing_cloid(context.cloid.clone());
                drop(order);
                for request in [request, supplied] {
                    assert_eq!(request.asset, 17);
                    assert_eq!(request.is_buy, is_buy);
                    assert_eq!(request.price, "12.30");
                    assert_eq!(request.size, "0.450");
                    assert_eq!(request.order_kind, order_kind);
                    assert_eq!(request.reduce_only, reduce_only);
                    assert_eq!(request.cloid.as_deref(), Some(context.cloid.as_str()));
                }
                assert_eq!(context.account_address, ADDRESS);
                assert_eq!(context.symbol_key, "xyz:TEST");
                assert_eq!(context.surface, surface);
                assert_eq!(context.order_kind, order_kind);
                assert_eq!(context.placement_label(), surface.label());
                assert_eq!(context.cloid.len(), 34);
                assert!(context.cloid.starts_with("0x"));
                assert!(
                    context.cloid[2..]
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit())
                );
                assert!(cloids.insert(context.cloid));
            }
        }
    }
    assert_eq!(cloids.len(), 156);
}
