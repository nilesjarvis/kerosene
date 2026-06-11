use super::*;
use crate::api::{ExchangeSymbol, OutcomeSymbolInfo};
use crate::x_feed::XTickerMention;

fn perp_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: "crypto".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 50,
        only_isolated: false,
        market_type: MarketType::Perp,
        outcome: None,
    }
}

fn outcome_symbol(key: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: "OUT95-YES".to_string(),
        category: "outcome".to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: true,
        market_type: MarketType::Outcome,
        outcome: Some(OutcomeSymbolInfo {
            outcome_id: 95,
            question_id: None,
            question_name: Some("Will BTC close green?".to_string()),
            question_description: None,
            question_class: None,
            question_underlying: None,
            question_expiry: None,
            question_price_thresholds: Vec::new(),
            question_period: None,
            question_named_outcomes: Vec::new(),
            question_settled_named_outcomes: Vec::new(),
            question_fallback_outcome: None,
            bucket_index: None,
            is_question_fallback: false,
            side_index: 0,
            side_name: "Yes".to_string(),
            outcome_name: "Recurring".to_string(),
            description: "Will BTC close green?".to_string(),
            class: None,
            underlying: None,
            expiry: None,
            target_price: None,
            period: None,
            quote_symbol: "USDH".to_string(),
            quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
            encoding: 950,
        }),
    }
}

fn post_with_mention(symbol: &str, ticker: &str) -> XFeedPost {
    XFeedPost {
        id: "1".to_string(),
        author_id: "author".to_string(),
        username: "user".to_string(),
        text: "post".to_string(),
        timestamp_ms: 0,
        fetched_at_ms: 0,
        request_started_ms: 0,
        request_duration_ms: 0,
        first_seen_ms: 0,
        url: String::new(),
        ticker_mentions: vec![XTickerMention {
            symbol: symbol.to_string(),
            ticker: ticker.to_string(),
            reference_price: None,
            reference_seen_ms: 0,
        }],
    }
}

#[test]
fn x_ticker_impact_cards_resolve_outcome_display_label() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols.push(outcome_symbol("#950"));

    let cards = terminal.x_ticker_impact_cards(&post_with_mention("#950", "OUT95-YES"));

    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].label, "YES: Will BTC close green?");
}

#[test]
fn x_ticker_impact_cards_keep_perp_tickers() {
    let mut terminal = TradingTerminal::boot().0;
    terminal.exchange_symbols.push(perp_symbol("HYPE"));

    let cards = terminal.x_ticker_impact_cards(&post_with_mention("HYPE", "HYPE"));

    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].label, "HYPE");
}
