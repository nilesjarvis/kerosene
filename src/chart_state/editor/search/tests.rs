use crate::api::{ExchangeSymbol, MarketType};

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
