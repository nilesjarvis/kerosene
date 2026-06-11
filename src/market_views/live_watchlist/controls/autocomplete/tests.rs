use super::live_watchlist_autocomplete_matches;
use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};

fn outcome_symbol() -> ExchangeSymbol {
    ExchangeSymbol {
        key: "#950".to_string(),
        ticker: "OUT95-YES".to_string(),
        category: "outcome".to_string(),
        display_name: Some("YES: Will BTC close green?".to_string()),
        keywords: vec!["prediction".to_string()],
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

#[test]
fn autocomplete_matches_outcome_question_text_in_display_name() {
    let symbol = outcome_symbol();

    assert!(live_watchlist_autocomplete_matches(
        &symbol,
        "btc close green"
    ));
}

#[test]
fn autocomplete_matches_keywords_key_and_ticker() {
    let symbol = outcome_symbol();

    assert!(live_watchlist_autocomplete_matches(&symbol, "prediction"));
    assert!(live_watchlist_autocomplete_matches(&symbol, "#950"));
    assert!(live_watchlist_autocomplete_matches(&symbol, "out95"));
    assert!(!live_watchlist_autocomplete_matches(&symbol, "solana"));
}
