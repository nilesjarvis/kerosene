use super::*;
use crate::api::MarketType;

fn symbol(key: &str, ticker: &str, market_type: MarketType) -> ExchangeSymbol {
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
        market_type,
        outcome: None,
    }
}

fn context(day_vlm: f64) -> WatchlistContext {
    WatchlistContext {
        funding: None,
        prev_day_px: None,
        day_vlm: Some(day_vlm),
    }
}

#[test]
fn filtered_indices_cache_applies_query_favourites_and_mute_filter() {
    let symbols = vec![
        symbol("BTC", "BTC", MarketType::Perp),
        symbol("ETH", "ETH", MarketType::Perp),
        symbol("@1", "UBTC", MarketType::Spot),
    ];
    let favourites = vec!["ETH".to_string()];
    let contexts = HashMap::new();

    let (indices, favourite_count) = filtered_symbol_search_indices(SymbolSearchResultsInput {
        symbols: &symbols,
        query: "t",
        sort_mode: SymbolSearchSortMode::Alphabetical,
        market_filter: SymbolSearchMarketFilter::All,
        hip3_dex_filter: None,
        favourite_symbols: &favourites,
        contexts: &contexts,
        is_muted: |symbol| symbol.key == "BTC",
    });

    assert_eq!(indices, vec![1, 2]);
    assert_eq!(favourite_count, 1);
}

#[test]
fn filtered_indices_cache_sorts_by_volume_when_contexts_are_available() {
    let symbols = vec![
        symbol("BTC", "BTC", MarketType::Perp),
        symbol("ETH", "ETH", MarketType::Perp),
    ];
    let favourites = Vec::new();
    let contexts = HashMap::from([
        ("BTC".to_string(), context(10.0)),
        ("ETH".to_string(), context(20.0)),
    ]);

    let (indices, favourite_count) = filtered_symbol_search_indices(SymbolSearchResultsInput {
        symbols: &symbols,
        query: "",
        sort_mode: SymbolSearchSortMode::Volume24h,
        market_filter: SymbolSearchMarketFilter::All,
        hip3_dex_filter: None,
        favourite_symbols: &favourites,
        contexts: &contexts,
        is_muted: |_| false,
    });

    assert_eq!(indices, vec![1, 0]);
    assert_eq!(favourite_count, 0);
}

#[test]
fn filtered_indices_prefer_primary_known_hip3_dex_for_duplicate_tickers() {
    let symbols = vec![
        symbol("flx:CRCL", "CRCL", MarketType::Perp),
        symbol("xyz:CRCL", "CRCL", MarketType::Perp),
    ];
    let favourites = Vec::new();
    let contexts = HashMap::new();

    let (indices, favourite_count) = filtered_symbol_search_indices(SymbolSearchResultsInput {
        symbols: &symbols,
        query: "crcl",
        sort_mode: SymbolSearchSortMode::Relevance,
        market_filter: SymbolSearchMarketFilter::Hip3,
        hip3_dex_filter: None,
        favourite_symbols: &favourites,
        contexts: &contexts,
        is_muted: |_| false,
    });

    assert_eq!(indices, vec![1, 0]);
    assert_eq!(favourite_count, 0);
}
