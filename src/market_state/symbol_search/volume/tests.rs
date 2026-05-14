use crate::api::{ExchangeSymbol, MarketType, WatchlistContext};

use super::*;
use std::collections::HashMap;

fn symbol(key: &str, ticker: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: ticker.to_string(),
        category: String::new(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        collateral_token: None,
        sz_decimals: 0,
        max_leverage: 0,
        only_isolated: false,
        market_type: MarketType::Perp,
        outcome: None,
    }
}

fn context(day_vlm: Option<f64>) -> WatchlistContext {
    WatchlistContext {
        funding: None,
        prev_day_px: None,
        day_vlm,
    }
}

#[test]
fn volume_prefers_symbol_key_then_ticker_fallback() {
    let contexts = HashMap::from([
        ("xyz:NVDA".to_string(), context(Some(10.0))),
        ("NVDA".to_string(), context(Some(20.0))),
    ]);

    assert_eq!(
        symbol_search_volume(&contexts, &symbol("xyz:NVDA", "NVDA")),
        Some(10.0)
    );

    let contexts = HashMap::from([("NVDA".to_string(), context(Some(20.0)))]);
    assert_eq!(
        symbol_search_volume(&contexts, &symbol("xyz:NVDA", "NVDA")),
        Some(20.0)
    );
}

#[test]
fn volume_rejects_missing_nonfinite_or_absent_values() {
    assert_eq!(
        symbol_search_volume(&HashMap::new(), &symbol("BTC", "BTC")),
        None
    );

    let contexts = HashMap::from([
        ("BTC".to_string(), context(Some(f64::NAN))),
        ("ETH".to_string(), context(None)),
    ]);

    assert_eq!(symbol_search_volume(&contexts, &symbol("BTC", "BTC")), None);
    assert_eq!(symbol_search_volume(&contexts, &symbol("ETH", "ETH")), None);
}

#[test]
fn volume_formatter_uses_compact_suffixes() {
    assert_eq!(format_symbol_search_volume(12.0), "$12");
    assert_eq!(format_symbol_search_volume(1_234.0), "$1.2K");
    assert_eq!(format_symbol_search_volume(2_500_000.0), "$2.5M");
    assert_eq!(format_symbol_search_volume(3_400_000_000.0), "$3.4B");
}

#[test]
fn volume_formatter_preserves_negative_sign() {
    assert_eq!(format_symbol_search_volume(-1_234.0), "$-1.2K");
}
