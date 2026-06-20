use super::{
    ExchangeSymbol, ExchangeSymbolsPayload, MarketType, OutcomeSymbolInfo, info_request_payload,
};

#[test]
fn info_request_payload_uses_requested_type() {
    assert_eq!(
        info_request_payload("spotMeta"),
        serde_json::json!({ "type": "spotMeta" })
    );
}

fn test_outcome_symbol() -> ExchangeSymbol {
    ExchangeSymbol {
        key: "#660".to_string(),
        ticker: "#660".to_string(),
        category: "outcome".to_string(),
        display_name: Some("BTC above private threshold".to_string()),
        keywords: vec!["btc".to_string(), "private-threshold".to_string()],
        asset_index: 100_000_000,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 1,
        only_isolated: true,
        market_type: MarketType::Outcome,
        outcome: Some(OutcomeSymbolInfo {
            outcome_id: 66,
            question_id: Some(12),
            question_name: Some("Will BTC close above the private threshold?".to_string()),
            question_description: Some("Long raw outcome description".to_string()),
            question_class: Some("priceBucket".to_string()),
            question_underlying: Some("BTC".to_string()),
            question_expiry: Some("20260520-0600".to_string()),
            question_price_thresholds: vec!["75348.12".to_string(), "78423.45".to_string()],
            question_period: Some("1d".to_string()),
            question_named_outcomes: vec![67, 68, 69],
            question_settled_named_outcomes: Vec::new(),
            question_fallback_outcome: Some(66),
            bucket_index: Some(2),
            is_question_fallback: false,
            side_index: 0,
            side_name: "Yes".to_string(),
            outcome_name: "BTC above private threshold".to_string(),
            description: "Outcome contract description".to_string(),
            class: None,
            underlying: None,
            expiry: None,
            target_price: Some("75348.12".to_string()),
            period: None,
            quote_symbol: "USDH".to_string(),
            quote_token_index: Some(crate::api::USDH_TOKEN_INDEX),
            encoding: 660,
        }),
    }
}

#[test]
fn exchange_symbols_payload_debug_summarizes_symbol_list() {
    let payload = ExchangeSymbolsPayload {
        symbols: vec![
            ExchangeSymbol {
                key: "BTC".to_string(),
                ticker: "BTC".to_string(),
                category: "crypto".to_string(),
                display_name: None,
                keywords: Vec::new(),
                asset_index: 0,
                collateral_token: None,
                sz_decimals: 5,
                max_leverage: 50,
                only_isolated: false,
                market_type: MarketType::Perp,
                outcome: None,
            },
            test_outcome_symbol(),
        ],
        spot_meta_failed: true,
        outcome_meta_failed: false,
    };

    let rendered = format!("{payload:?}");

    assert!(rendered.contains("ExchangeSymbolsPayload"));
    assert!(rendered.contains("symbols_len: 2"));
    assert!(rendered.contains("perp_count: 1"));
    assert!(rendered.contains("outcome_count: 1"));
    assert!(rendered.contains("spot_meta_failed: true"));
    assert!(!rendered.contains("private threshold"));
    assert!(!rendered.contains("Long raw outcome description"));
    assert!(!rendered.contains("75348.12"));
}
