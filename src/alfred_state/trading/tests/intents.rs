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
