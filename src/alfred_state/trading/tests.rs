use super::*;
use crate::api::MarketType;

mod drafts;
mod intents;

fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 5,
        max_leverage: 50,
        only_isolated: false,
        market_type,
        outcome: None,
    }
}

fn spot_symbol(key: &str, ticker: &str, display_name: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: "spot".to_string(),
        display_name: Some(display_name.to_string()),
        keywords: vec!["spot".to_string()],
        asset_index: 10_107,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 1,
        only_isolated: false,
        market_type: MarketType::Spot,
        outcome: None,
    }
}

fn terminal_with_hype() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols = vec![symbol("HYPE", MarketType::Perp)];
    terminal
}

fn terminal_with_hype_perp_and_spot() -> TradingTerminal {
    let mut terminal = TradingTerminal::boot().0;
    // Spot pair listed first to prove perp preference is a rule, not list order.
    terminal.exchange_symbols = vec![
        spot_symbol("@107", "HYPE", "HYPE/USDC"),
        symbol("HYPE", MarketType::Perp),
    ];
    terminal
}

fn trade_intent_or_panic(query: &str) -> ParsedTradeIntent {
    match parse_trade_intent(query) {
        Some(intent) => intent,
        None => panic!("missing trade intent for {query}"),
    }
}

fn trade_draft_or_panic(terminal: &TradingTerminal, query: &str) -> AlfredTradeDraft {
    match terminal.alfred_trade_draft(query) {
        Some(draft) => draft,
        None => panic!("missing trade draft for {query}"),
    }
}

#[test]
fn trade_draft_and_intent_debug_redact_order_values_without_changing_them() {
    let intent = ParsedTradeIntent {
        side: Some(AlfredTradeSide::Buy),
        amount: Some(12_345.678_9),
        amount_is_usd: true,
        symbol: Some("private-trade-symbol-sentinel".to_string()),
        explicit_spot: false,
        explicit_chase: false,
        explicit_limit: true,
        limit_price: Some(98_765.432_1),
        error: Some("private-trade-error-sentinel".to_string()),
    };
    let draft = AlfredTradeDraft {
        side: Some(AlfredTradeSide::Buy),
        symbol_key: Some("private-draft-symbol-sentinel".to_string()),
        icon_symbol: Some("private-draft-icon-sentinel".to_string()),
        icon_title_anchor: Some("private-draft-anchor-sentinel".to_string()),
        quantity: Some(12_345.678_9),
        quantity_is_usd: true,
        order_kind: OrderKind::Limit,
        limit_price: Some(98_765.432_1),
        title: "private-draft-title-sentinel".to_string(),
        detail: "private-draft-detail-sentinel".to_string(),
        tag: "private-draft-tag-sentinel".to_string(),
        error: Some("private-draft-error-sentinel".to_string()),
    };

    let rendered = format!("{intent:?} {draft:?}");

    assert!(rendered.contains("<redacted>"), "{rendered}");
    for sensitive in [
        "private-trade-symbol-sentinel",
        "private-trade-error-sentinel",
        "private-draft-symbol-sentinel",
        "private-draft-icon-sentinel",
        "private-draft-anchor-sentinel",
        "private-draft-title-sentinel",
        "private-draft-detail-sentinel",
        "private-draft-tag-sentinel",
        "private-draft-error-sentinel",
        "12345.6789",
        "98765.4321",
    ] {
        assert!(!rendered.contains(sensitive), "{rendered}");
    }
    assert_eq!(
        intent.symbol.as_deref(),
        Some("private-trade-symbol-sentinel")
    );
    assert_eq!(
        intent.amount.map(f64::to_bits),
        Some(12_345.678_9_f64.to_bits())
    );
    assert_eq!(
        draft.symbol_key.as_deref(),
        Some("private-draft-symbol-sentinel")
    );
    assert_eq!(
        draft.limit_price.map(f64::to_bits),
        Some(98_765.432_1_f64.to_bits())
    );
}
