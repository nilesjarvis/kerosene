use super::{
    ExchangeSymbol, ExchangeSymbolsPayload, MarketType, OutcomeSymbolInfo, info_request_payload,
    mark_payload_loaded_from_cache, payload_from_source_results,
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

fn test_spot_symbol(quote_token: Option<u32>) -> ExchangeSymbol {
    ExchangeSymbol {
        key: "@107".to_string(),
        ticker: "HYPE".to_string(),
        category: "spot".to_string(),
        display_name: Some("HYPE/USDC".to_string()),
        keywords: vec!["spot".to_string()],
        asset_index: 10_107,
        collateral_token: quote_token,
        sz_decimals: 2,
        max_leverage: 1,
        only_isolated: false,
        market_type: MarketType::Spot,
        outcome: None,
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
        loaded_from_cache: false,
        perp_meta_failed: false,
        spot_meta_failed: true,
        outcome_meta_failed: false,
    };

    let rendered = format!("{payload:?}");

    assert!(rendered.contains("ExchangeSymbolsPayload"));
    assert!(rendered.contains("symbols_len: 2"));
    assert!(rendered.contains("perp_count: 1"));
    assert!(rendered.contains("outcome_count: 1"));
    assert!(rendered.contains("loaded_from_cache: false"));
    assert!(rendered.contains("perp_meta_failed: false"));
    assert!(rendered.contains("spot_meta_failed: true"));
    assert!(!rendered.contains("private threshold"));
    assert!(!rendered.contains("Long raw outcome description"));
    assert!(!rendered.contains("75348.12"));
}

#[test]
fn symbol_cache_requires_complete_sources_and_spot_quote_tokens() {
    let complete = ExchangeSymbolsPayload {
        symbols: vec![test_spot_symbol(Some(0))],
        loaded_from_cache: false,
        perp_meta_failed: false,
        spot_meta_failed: false,
        outcome_meta_failed: false,
    };
    assert!(complete.is_cacheable());

    let legacy_missing_quote = ExchangeSymbolsPayload {
        symbols: vec![test_spot_symbol(None)],
        ..complete.clone()
    };
    assert!(!legacy_missing_quote.is_cacheable());

    let cached = ExchangeSymbolsPayload {
        loaded_from_cache: true,
        ..complete.clone()
    };
    assert!(!cached.is_cacheable());

    let partial = ExchangeSymbolsPayload {
        spot_meta_failed: true,
        ..complete
    };
    assert!(!partial.is_cacheable());
}

#[test]
fn cache_provenance_is_runtime_only() {
    let payload = ExchangeSymbolsPayload {
        symbols: vec![test_spot_symbol(Some(0))],
        loaded_from_cache: true,
        perp_meta_failed: false,
        spot_meta_failed: false,
        outcome_meta_failed: false,
    };

    let encoded = serde_json::to_value(&payload).expect("payload serializes");
    assert!(encoded.get("loaded_from_cache").is_none());
    let decoded: ExchangeSymbolsPayload =
        serde_json::from_value(encoded).expect("payload deserializes");
    assert!(!decoded.loaded_from_cache);
}

#[test]
fn cached_fetch_marks_payload_as_unverified_for_runtime() {
    let payload = ExchangeSymbolsPayload {
        symbols: vec![test_spot_symbol(Some(0))],
        loaded_from_cache: false,
        perp_meta_failed: false,
        spot_meta_failed: false,
        outcome_meta_failed: false,
    };

    let cached = mark_payload_loaded_from_cache(payload);

    assert!(cached.loaded_from_cache);
    assert!(!cached.is_cacheable());
    assert_eq!(cached.symbols[0].key, "@107");
}

#[test]
fn legacy_cached_payload_defaults_perp_failure_flag_to_false() {
    let payload: ExchangeSymbolsPayload = serde_json::from_value(serde_json::json!({
        "symbols": [],
        "spot_meta_failed": true,
        "outcome_meta_failed": false
    }))
    .expect("legacy payload still deserializes");

    assert!(!payload.perp_meta_failed);
}

#[test]
fn perp_source_failure_preserves_successful_spot_result() {
    let payload = payload_from_source_results(
        Err("perp unavailable".to_string()),
        Ok(vec![test_spot_symbol(Some(0))]),
        Ok(Vec::new()),
    );

    assert!(payload.perp_meta_failed);
    assert!(!payload.spot_meta_failed);
    assert_eq!(payload.symbols.len(), 1);
    assert_eq!(payload.symbols[0].market_type, MarketType::Spot);
}
