use crate::api::{ExchangeSymbol, MarketType, OutcomeSymbolInfo};

use super::*;
use std::cmp::Ordering;

fn symbol(
    key: &str,
    ticker: &str,
    category: &str,
    display_name: Option<&str>,
    keywords: &[&str],
) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: category.to_string(),
        display_name: display_name.map(str::to_string),
        keywords: keywords.iter().map(|keyword| keyword.to_string()).collect(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 2,
        max_leverage: 50,
        only_isolated: false,
        market_type: MarketType::Perp,
        outcome: None,
    }
}

fn fallback_outcome_symbol() -> ExchangeSymbol {
    let mut symbol = symbol(
        "#660",
        "OUT66-YES",
        "outcome",
        Some("YES: fallback / other settlement"),
        &["outcome", "prediction", "fallback"],
    );
    symbol.market_type = MarketType::Outcome;
    symbol.outcome = Some(OutcomeSymbolInfo {
        outcome_id: 66,
        question_id: Some(12),
        question_name: Some("Recurring".to_string()),
        question_description: Some(
            "class:priceBucket|underlying:BTC|expiry:20260520-0600|priceThresholds:75348,78423|period:1d"
                .to_string(),
        ),
        question_class: Some("priceBucket".to_string()),
        question_underlying: Some("BTC".to_string()),
        question_expiry: Some("20260520-0600".to_string()),
        question_price_thresholds: vec!["75348".to_string(), "78423".to_string()],
        question_period: Some("1d".to_string()),
        question_named_outcomes: vec![67, 68, 69],
        question_settled_named_outcomes: Vec::new(),
        question_fallback_outcome: Some(66),
        bucket_index: None,
        is_question_fallback: true,
        side_index: 0,
        side_name: "Yes".to_string(),
        outcome_name: "Recurring Fallback".to_string(),
        description: "other".to_string(),
        class: None,
        underlying: None,
        expiry: None,
        target_price: None,
        period: None,
        quote_symbol: "USDH".to_string(),
        quote_token_index: Some(150),
        encoding: 660,
    });
    symbol
}

#[test]
fn symbol_match_checks_ticker_category_display_keywords_and_key() {
    let btc = symbol(
        "xyz:NVDA",
        "NVDA",
        "stocks",
        Some("Nvidia"),
        &["AI", "semiconductors"],
    );

    assert!(chart_editor_symbol_matches(&btc, ""));
    assert!(chart_editor_symbol_matches(&btc, "nvd"));
    assert!(chart_editor_symbol_matches(&btc, "stock"));
    assert!(chart_editor_symbol_matches(&btc, "nvidia"));
    assert!(chart_editor_symbol_matches(&btc, "semi"));
    assert!(chart_editor_symbol_matches(&btc, "xyz"));
    assert!(!chart_editor_symbol_matches(&btc, "btc"));
}

#[test]
fn symbol_match_hides_question_fallback_outcomes() {
    let fallback = fallback_outcome_symbol();

    assert!(!chart_editor_symbol_matches(&fallback, ""));
    assert!(!chart_editor_symbol_matches(&fallback, "fallback"));
    assert!(!chart_editor_symbol_matches(&fallback, "#660"));
}

#[test]
fn symbol_score_prioritizes_exact_and_prefix_matches() {
    let btc = symbol("BTC", "BTC", "crypto", Some("Bitcoin"), &["store of value"]);

    assert_eq!(chart_editor_symbol_score(&btc, ""), 0);
    assert_eq!(chart_editor_symbol_score(&btc, "btc"), 0);
    assert_eq!(chart_editor_symbol_score(&btc, "bit"), 1);
    assert_eq!(chart_editor_symbol_score(&btc, "coin"), 2);
}

#[test]
fn compare_prefers_score_then_favourites_then_symbol_order() {
    let btc = symbol("BTC", "BTC", "crypto", Some("Bitcoin"), &[]);
    let eth = symbol("ETH", "ETH", "crypto", Some("Ethereum"), &[]);
    let hype = symbol("HYPE", "HYPE", "crypto", None, &[]);
    let favourites = vec!["HYPE".to_string(), "ETH".to_string()];

    assert_eq!(
        compare_chart_editor_symbols(&btc, &eth, "eth", &favourites),
        Ordering::Greater
    );
    assert_eq!(
        compare_chart_editor_symbols(&hype, &eth, "", &favourites),
        Ordering::Less
    );
    assert_eq!(
        compare_chart_editor_symbols(&btc, &eth, "", &[]),
        Ordering::Less
    );
}

#[test]
fn compare_prefers_primary_known_hip3_dex_for_duplicate_tickers() {
    let flx_crcl = symbol("flx:CRCL", "CRCL", "stocks", None, &[]);
    let xyz_crcl = symbol("xyz:CRCL", "CRCL", "stocks", Some("CRCL"), &[]);

    assert_eq!(
        compare_chart_editor_symbols(&xyz_crcl, &flx_crcl, "crcl", &[]),
        Ordering::Less
    );
    assert_eq!(
        compare_chart_editor_symbols(&flx_crcl, &xyz_crcl, "crcl", &[]),
        Ordering::Greater
    );
}
