use crate::api::{ExchangeSymbol, MarketType};

use super::*;

fn symbol(key: &str, category: &str) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.to_string(),
        category: category.to_string(),
        display_name: None,
        keywords: Vec::new(),
        asset_index: 0,
        sz_decimals: 0,
        max_leverage: 0,
        only_isolated: false,
        market_type: MarketType::Perp,
        outcome: None,
    }
}

#[test]
fn context_symbol_keys_filter_sort_and_deduplicate_symbols() {
    let symbols = vec![
        symbol("ETH", "crypto"),
        symbol("BTC", "crypto"),
        symbol("ETH", "crypto"),
        symbol("NVDA", "stocks"),
        symbol("SOL", "crypto"),
    ];

    let keys = context_symbol_keys(
        &symbols,
        |symbol| symbol.category == "crypto",
        |symbol| symbol.key == "SOL",
    );

    assert_eq!(keys, vec!["BTC".to_string(), "ETH".to_string()]);
}

#[test]
fn context_symbol_keys_returns_empty_when_no_symbol_matches() {
    let symbols = vec![symbol("BTC", "crypto")];

    let keys = context_symbol_keys(&symbols, |_| false, |_| false);

    assert!(keys.is_empty());
}
