use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};
use crate::app_state::{TradingTerminal, sensitive_string};
use crate::order_execution::{
    MarketUsdSizeReference, PendingOrderAction, PlaceIntent, PreparedExchangeOrder, PriceSource,
    QuantityDenomination, QuantitySource, ReduceOnlySource,
};
use crate::signing::{ExchangeOrderKind, OrderKind};

mod outcomes;

fn outcome_info(is_question_fallback: bool) -> OutcomeSymbolInfo {
    OutcomeSymbolInfo {
        outcome_id: 65,
        question_id: Some(12),
        question_name: Some("Recurring".to_string()),
        question_description: Some(
            "class:priceBucket|underlying:BTC|expiry:20260520-0600".to_string(),
        ),
        question_class: Some("priceBucket".to_string()),
        question_underlying: Some("BTC".to_string()),
        question_expiry: Some("20260520-0600".to_string()),
        question_price_thresholds: vec!["75348".to_string(), "78423".to_string()],
        question_period: Some("1d".to_string()),
        question_named_outcomes: vec![67, 68, 69],
        question_settled_named_outcomes: Vec::new(),
        question_fallback_outcome: Some(66),
        bucket_index: Some(0),
        is_question_fallback,
        side_index: 0,
        side_name: "Yes".to_string(),
        outcome_name: "Recurring Named Outcome".to_string(),
        description: "index:0".to_string(),
        class: None,
        underlying: None,
        expiry: None,
        target_price: None,
        period: None,
        quote_symbol: "USDH".to_string(),
        quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
        encoding: 650,
    }
}

fn outcome_symbol(key: &str, asset_index: u32, is_question_fallback: bool) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: "OUT65-YES".to_string(),
        category: "outcome".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: true,
        market_type: MarketType::Outcome,
        outcome: Some(outcome_info(is_question_fallback)),
    }
}

fn terminal_for_outcome_order(symbol: ExchangeSymbol) -> TradingTerminal {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.wallet_key_input = sensitive_string("agent-key");
    terminal.active_symbol = symbol.key.clone();
    terminal.exchange_symbols = vec![symbol];
    terminal.order_kind = OrderKind::Limit;
    terminal.order_price = "0.42123456".to_string();
    terminal.order_quantity = "3".to_string();
    terminal.order_quantity_is_usd = true;
    terminal.order_reduce_only = true;
    terminal
        .all_mids
        .insert(terminal.active_symbol.clone(), 0.42);
    terminal
        .all_mids_updated_at_ms
        .insert(terminal.active_symbol.clone(), TradingTerminal::now_ms());
    terminal
}

fn first_symbol_or_panic(terminal: &TradingTerminal) -> &ExchangeSymbol {
    match terminal.exchange_symbols.first() {
        Some(symbol) => symbol,
        None => panic!("missing symbol"),
    }
}

fn prepared_order_or_panic(
    terminal: &TradingTerminal,
    symbol: &ExchangeSymbol,
    is_buy: bool,
) -> PreparedExchangeOrder {
    let order_kind = ExchangeOrderKind::try_from(terminal.order_kind)
        .expect("test order kind should be exchange order kind");
    let intent = PlaceIntent {
        surface: crate::order_execution::OrderSurface::Ticket,
        symbol_key: symbol.key.clone(),
        is_buy,
        order_kind,
        price_source: match order_kind {
            ExchangeOrderKind::Market => PriceSource::MarketWithSlippage {
                invalid_message: None,
                usd_size_reference: MarketUsdSizeReference::ExecutionPrice,
            },
            ExchangeOrderKind::Limit | ExchangeOrderKind::LimitIoc => PriceSource::LimitInput {
                value: terminal.order_price.clone(),
                invalid_message: "Invalid price",
            },
        },
        quantity_source: QuantitySource::UserInput {
            value: terminal.order_quantity.clone(),
            denomination: if terminal.order_quantity_is_usd {
                QuantityDenomination::UsdNotional
            } else {
                QuantityDenomination::Coin
            },
            invalid_message: "Invalid quantity",
            precision_invalid_message: "Invalid quantity for asset precision",
        },
        reduce_only_source: ReduceOnlySource::Form(terminal.order_reduce_only),
    };

    match terminal.prepare_place_order(intent) {
        Ok(prepared) => prepared,
        Err(error) => panic!("valid outcome order: {error}"),
    }
}

fn order_status_or_panic(terminal: &TradingTerminal) -> (&str, bool) {
    match terminal.order_status.as_ref() {
        Some((message, is_error)) => (message.as_str(), *is_error),
        None => panic!("missing order status"),
    }
}

#[test]
fn execute_order_rejects_while_order_action_pending() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.order_kind = OrderKind::Market;
    terminal.pending_order_action = Some(PendingOrderAction::Buy);

    let _task = terminal.execute_order(true);

    let (status, is_error) = order_status_or_panic(&terminal);
    assert!(is_error);
    assert_eq!(status, "Wait for the pending order action to finish");
    assert_eq!(terminal.pending_order_action, Some(PendingOrderAction::Buy));
}

fn perp_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 4,
        max_leverage: 50,
        only_isolated: false,
        market_type: MarketType::Perp,
        outcome: None,
    }
}

#[test]
fn ioc_limit_orders_project_like_market_orders() {
    let (mut terminal, _) = TradingTerminal::boot();
    terminal.connected_address = Some("0xabc0000000000000000000000000000000000000".to_string());
    terminal.wallet_key_input = sensitive_string("agent-key");
    terminal.active_symbol = "BTC".to_string();
    terminal.exchange_symbols = vec![perp_symbol("BTC")];
    terminal.order_kind = OrderKind::LimitIoc;
    terminal.order_price = "100".to_string();
    terminal.order_quantity = "1".to_string();
    terminal.order_quantity_is_usd = false;
    terminal.order_reduce_only = false;
    terminal.all_mids.insert("BTC".to_string(), 100.0);
    terminal
        .all_mids_updated_at_ms
        .insert("BTC".to_string(), TradingTerminal::now_ms());

    let symbol = first_symbol_or_panic(&terminal).clone();
    let prepared = prepared_order_or_panic(&terminal, &symbol, true);
    let _task = terminal.submit_prepared_ticket_order("agent-key".to_string().into(), prepared);

    // IOC orders are taker orders that never rest: they must project a
    // position delta (MarketPlacing), not a provisional resting row.
    let kinds: Vec<_> = terminal
        .pending_order_indicators
        .values()
        .map(|indicator| indicator.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![crate::order_pending_indicators::PendingOrderIndicatorKind::MarketPlacing]
    );
}
