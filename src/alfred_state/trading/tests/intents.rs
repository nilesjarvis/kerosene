use super::*;

#[test]
fn parses_coin_market_order() {
    let intent = trade_intent_or_panic("buy 1k HYPE");

    assert_eq!(intent.side, Some(AlfredTradeSide::Buy));
    assert_eq!(intent.amount, Some(1_000.0));
    assert!(!intent.amount_is_usd);
    assert_eq!(intent.symbol.as_deref(), Some("HYPE"));
    assert_eq!(intent.order_kind(), OrderKind::Market);
}

#[test]
fn parses_usd_market_order_without_side_as_draft() {
    let intent = trade_intent_or_panic("$1k hype");

    assert_eq!(intent.side, None);
    assert_eq!(intent.amount, Some(1_000.0));
    assert!(intent.amount_is_usd);
    assert_eq!(intent.symbol.as_deref(), Some("hype"));
    assert_eq!(intent.order_kind(), OrderKind::Market);
}

#[test]
fn parses_usd_limit_order() {
    let intent = trade_intent_or_panic("buy $1k hype at 43");

    assert_eq!(intent.side, Some(AlfredTradeSide::Buy));
    assert_eq!(intent.amount, Some(1_000.0));
    assert!(intent.amount_is_usd);
    assert_eq!(intent.symbol.as_deref(), Some("hype"));
    assert_eq!(intent.limit_price, Some(43.0));
    assert_eq!(intent.order_kind(), OrderKind::Limit);
}

#[test]
fn parses_coin_chase_order_without_side() {
    let intent = trade_intent_or_panic("chase 1k HYPE");

    assert_eq!(intent.side, None);
    assert_eq!(intent.amount, Some(1_000.0));
    assert!(!intent.amount_is_usd);
    assert_eq!(intent.symbol.as_deref(), Some("HYPE"));
    assert_eq!(intent.order_kind(), OrderKind::Chase);
}

#[test]
fn parses_usd_chase_order_without_side() {
    let intent = trade_intent_or_panic("chase $1k hype");

    assert_eq!(intent.side, None);
    assert_eq!(intent.amount, Some(1_000.0));
    assert!(intent.amount_is_usd);
    assert_eq!(intent.symbol.as_deref(), Some("hype"));
    assert_eq!(intent.order_kind(), OrderKind::Chase);
}

#[test]
fn parses_chase_order_with_side_before_or_after_keyword() {
    let buy = trade_intent_or_panic("buy chase $1k HYPE");
    let sell = trade_intent_or_panic("chase sell 250 HYPE");

    assert_eq!(buy.side, Some(AlfredTradeSide::Buy));
    assert_eq!(buy.order_kind(), OrderKind::Chase);
    assert_eq!(sell.side, Some(AlfredTradeSide::Sell));
    assert_eq!(sell.order_kind(), OrderKind::Chase);
}

#[test]
fn rejects_chase_price_modifiers() {
    let intent = trade_intent_or_panic("chase $1k HYPE at 43");

    assert_eq!(intent.order_kind(), OrderKind::Chase);
    assert_eq!(
        intent.error.as_deref(),
        Some("Chase orders do not take a market, limit, or price modifier")
    );
}

#[test]
fn parses_spot_qualifier_token() {
    let intent = trade_intent_or_panic("sell 10 HYPE spot");

    assert_eq!(intent.side, Some(AlfredTradeSide::Sell));
    assert_eq!(intent.amount, Some(10.0));
    assert_eq!(intent.symbol.as_deref(), Some("HYPE"));
    assert!(intent.explicit_spot);
    assert_eq!(intent.order_kind(), OrderKind::Market);
}

#[test]
fn spot_qualifier_is_not_mistaken_for_the_symbol() {
    let intent = trade_intent_or_panic("sell 10 spot");

    assert!(intent.explicit_spot);
    assert_eq!(intent.symbol, None);
}

#[test]
fn ignores_non_trade_queries() {
    assert_eq!(parse_trade_intent("portfolio pane"), None);
    assert_eq!(parse_trade_intent("hype"), None);
    assert_eq!(parse_trade_intent("chase"), None);
}

#[test]
fn trade_tokens_preserve_dollar_joining_and_punctuation_boundaries() {
    for (query, amount, is_usd, symbol, price) in [
        (
            "[buy] ($) () [1k], {HYPE}; at '$' (43)",
            Some(1_000.0),
            true,
            Some("HYPE"),
            Some(43.0),
        ),
        (
            "buy\u{2003}$\t1,000\nHYPE",
            Some(1_000.0),
            true,
            Some("HYPE"),
            None,
        ),
        ("buy $", None, false, Some("$"), None),
        ("buy $ HYPE", None, false, Some("$HYPE"), None),
        ("buy $ $ 5 HYPE", Some(5.0), false, Some("$$"), None),
        ("buy $ $ $ 5 HYPE", Some(5.0), true, Some("$$"), None),
        ("buy $ $5 HYPE", None, false, Some("$$5"), None),
        ("buy 1 HYPE $", Some(1.0), false, Some("HYPE"), None),
        ("buy the 1 of [HYPE]", Some(1.0), false, Some("HYPE"), None),
        ("buy 1 ÉTH/USDC", Some(1.0), false, Some("ÉTH/USDC"), None),
        ("buy 1 @107", Some(1.0), false, Some("@107"), None),
        ("buy 1 #12", Some(1.0), false, Some("#12"), None),
        ("buy 1k;HYPE", None, false, Some("1k;HYPE"), None),
        ("buy [] , ; {}", None, false, None, None),
    ] {
        assert_eq!(
            parse_trade_intent(query),
            Some(ParsedTradeIntent {
                side: Some(AlfredTradeSide::Buy),
                amount,
                amount_is_usd: is_usd,
                symbol: symbol.map(str::to_owned),
                explicit_spot: false,
                explicit_chase: false,
                explicit_limit: price.is_some(),
                limit_price: price,
                error: None,
            }),
            "{query}",
        );
    }
}

#[test]
fn trade_parser_keeps_modifier_precedence_and_first_unconsumed_symbol() {
    let intent = trade_intent_or_panic("sell BUY 2 HYPE at 3 at 4 spot");
    assert_eq!(intent.side, Some(AlfredTradeSide::Buy));
    assert_eq!(intent.amount, Some(2.0));
    assert_eq!(intent.symbol.as_deref(), Some("HYPE"));
    assert_eq!(intent.limit_price, Some(4.0));
    assert!(intent.explicit_spot);
    assert!(intent.explicit_limit);
    assert_eq!(intent.error, None);

    let intent = trade_intent_or_panic("buy 1 2 HYPE");
    assert_eq!(intent.amount, Some(1.0));
    assert_eq!(intent.symbol.as_deref(), Some("2"));

    let intent = trade_intent_or_panic("buy at nope 1 HYPE");
    assert_eq!(intent.amount, Some(1.0));
    assert_eq!(intent.symbol.as_deref(), Some("nope"));
    assert_eq!(intent.limit_price, None);
    assert_eq!(intent.order_kind(), OrderKind::Limit);

    for query in ["", " [] , ; {} ", "$", "$ $", "(chase)", "spot HYPE"] {
        assert_eq!(parse_trade_intent(query), None, "{query}");
    }
}
