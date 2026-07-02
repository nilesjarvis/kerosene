use super::{ExchangeSymbol, append_spot_symbols};
use serde_json::json;

fn spot_symbols() -> Vec<ExchangeSymbol> {
    let spot_meta = json!({
        "tokens": [
            { "name": "USDC", "szDecimals": 8, "index": 0 },
            { "name": "PURR", "szDecimals": 0, "index": 1, "fullName": "Purr" },
            { "name": "HYPE", "szDecimals": 2, "index": 150, "fullName": "Hyperliquid" },
            { "name": "UETH", "szDecimals": 4, "index": 221 },
            { "name": "UBTC", "szDecimals": 5, "index": 197 },
        ],
        "universe": [
            { "name": "PURR/USDC", "tokens": [1, 0], "index": 0, "isCanonical": true },
            { "name": "@107", "tokens": [150, 0], "index": 107, "isCanonical": true },
            { "name": "@151", "tokens": [221, 197], "index": 151, "isCanonical": false },
        ],
    });

    let mut symbols = Vec::new();
    append_spot_symbols(&mut symbols, &spot_meta);
    symbols
}

fn symbol_for_key<'a>(symbols: &'a [ExchangeSymbol], key: &str) -> &'a ExchangeSymbol {
    match symbols.iter().find(|symbol| symbol.key == key) {
        Some(symbol) => symbol,
        None => panic!("expected a spot symbol keyed {key:?}"),
    }
}

#[test]
fn api_named_pair_is_keyed_by_its_api_coin_name() {
    let symbols = spot_symbols();

    // The API reports PURR/USDC under that name (not "@0") in allMids,
    // candles, l2Book, open orders, and fills, so the key must match it.
    let purr = symbol_for_key(&symbols, "PURR/USDC");
    assert_eq!(purr.ticker, "PURR");
    assert_eq!(purr.display_name.as_deref(), Some("PURR/USDC"));
    assert_eq!(purr.asset_index, 10_000);
    assert_eq!(purr.sz_decimals, 0);
    assert!(
        !symbols.iter().any(|symbol| symbol.key == "@0"),
        "the API-named pair must not also be keyed by its index"
    );
}

#[test]
fn indexed_pairs_keep_their_indexed_key() {
    let symbols = spot_symbols();

    let hype = symbol_for_key(&symbols, "@107");
    assert_eq!(hype.ticker, "HYPE");
    assert_eq!(hype.display_name.as_deref(), Some("HYPE/USDC"));
    assert_eq!(hype.asset_index, 10_107);
}

#[test]
fn non_usdc_quoted_pairs_are_labeled_with_their_quote_token() {
    let symbols = spot_symbols();

    let ueth_ubtc = symbol_for_key(&symbols, "@151");
    assert_eq!(ueth_ubtc.ticker, "UETH");
    assert_eq!(ueth_ubtc.display_name.as_deref(), Some("UETH/UBTC"));
    assert!(!ueth_ubtc.spot_quote_is_usd_stable());

    let hype = symbol_for_key(&symbols, "@107");
    assert!(hype.spot_quote_is_usd_stable());
}
