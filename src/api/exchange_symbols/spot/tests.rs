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
    append_spot_symbols(&mut symbols, &spot_meta).expect("valid spot metadata");
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
    assert_eq!(hype.collateral_token, Some(0));
}

#[test]
fn non_usdc_quoted_pairs_are_labeled_with_their_quote_token() {
    let symbols = spot_symbols();

    let ueth_ubtc = symbol_for_key(&symbols, "@151");
    assert_eq!(ueth_ubtc.ticker, "UETH");
    assert_eq!(ueth_ubtc.display_name.as_deref(), Some("UETH/UBTC"));
    assert_eq!(ueth_ubtc.collateral_token, Some(197));
    assert!(!ueth_ubtc.spot_quote_is_usd_stable());

    let hype = symbol_for_key(&symbols, "@107");
    assert!(hype.spot_quote_is_usd_stable());
}

#[test]
fn rejects_error_shaped_or_empty_metadata() {
    for invalid in [
        json!({ "error": "temporarily unavailable" }),
        json!({ "tokens": [], "universe": [] }),
        json!({ "tokens": [{ "name": "USDC", "szDecimals": 8, "index": 0 }], "universe": [] }),
    ] {
        let mut symbols = Vec::new();
        assert!(
            append_spot_symbols(&mut symbols, &invalid).is_err(),
            "invalid metadata must fail closed: {invalid}"
        );
        assert!(symbols.is_empty());
    }
}

#[test]
fn rejects_unknown_token_references_without_appending_partial_symbols() {
    let invalid = json!({
        "tokens": [
            { "name": "USDC", "szDecimals": 8, "index": 0 },
            { "name": "HYPE", "szDecimals": 2, "index": 150 }
        ],
        "universe": [
            { "name": "@107", "tokens": [150, 0], "index": 107 },
            { "name": "@108", "tokens": [999, 0], "index": 108 }
        ]
    });
    let mut symbols = Vec::new();

    let error = append_spot_symbols(&mut symbols, &invalid).expect_err("unknown base must fail");

    assert!(error.contains("unknown base token 999"));
    assert!(symbols.is_empty(), "validation must be atomic");
}

#[test]
fn rejects_duplicate_indices_and_non_pair_token_arrays() {
    let duplicate = json!({
        "tokens": [
            { "name": "USDC", "szDecimals": 8, "index": 0 },
            { "name": "HYPE", "szDecimals": 2, "index": 0 }
        ],
        "universe": [{ "name": "@107", "tokens": [0, 1], "index": 107 }]
    });
    let wrong_pair_shape = json!({
        "tokens": [
            { "name": "USDC", "szDecimals": 8, "index": 0 },
            { "name": "HYPE", "szDecimals": 2, "index": 150 }
        ],
        "universe": [{ "name": "@107", "tokens": [150], "index": 107 }]
    });

    assert!(append_spot_symbols(&mut Vec::new(), &duplicate).is_err());
    assert!(append_spot_symbols(&mut Vec::new(), &wrong_pair_shape).is_err());
}

#[test]
fn rejects_duplicate_token_names_even_when_case_differs() {
    let invalid = json!({
        "tokens": [
            { "name": "USDC", "szDecimals": 8, "index": 0 },
            { "name": "HYPE", "szDecimals": 2, "index": 150 },
            { "name": "hype", "szDecimals": 2, "index": 151 }
        ],
        "universe": [
            { "name": "@107", "tokens": [150, 0], "index": 107 },
            { "name": "@108", "tokens": [151, 0], "index": 108 }
        ]
    });
    let mut symbols = Vec::new();

    let error = append_spot_symbols(&mut symbols, &invalid)
        .expect_err("duplicate token identity must fail closed");

    assert!(error.contains("duplicate token name HYPE"));
    assert!(symbols.is_empty(), "validation must remain atomic");
}

#[test]
fn rejects_size_precision_outside_exchange_bounds() {
    let invalid = json!({
        "tokens": [
            { "name": "USDC", "szDecimals": 8, "index": 0 },
            { "name": "HYPE", "szDecimals": 9, "index": 150 }
        ],
        "universe": [{ "name": "@107", "tokens": [150, 0], "index": 107 }]
    });
    let mut symbols = Vec::new();

    let error = append_spot_symbols(&mut symbols, &invalid)
        .expect_err("out-of-range size precision must fail closed");

    assert!(error.contains("unsafe szDecimals 9"));
    assert!(symbols.is_empty());
}

#[test]
fn rejects_api_pair_names_that_spoof_the_referenced_quote_token() {
    let invalid_name = json!({
        "tokens": [
            { "name": "USDC", "szDecimals": 8, "index": 0 },
            { "name": "UBTC", "szDecimals": 5, "index": 197 },
            { "name": "UETH", "szDecimals": 4, "index": 221 }
        ],
        "universe": [
            { "name": "UETH/USDC", "tokens": [221, 197], "index": 151 }
        ]
    });
    let slash_in_token = json!({
        "tokens": [
            { "name": "USDC", "szDecimals": 8, "index": 0 },
            { "name": "UETH/USDC", "szDecimals": 4, "index": 221 }
        ],
        "universe": [
            { "name": "@151", "tokens": [221, 0], "index": 151 }
        ]
    });

    for invalid in [invalid_name, slash_in_token] {
        let mut symbols = Vec::new();
        assert!(append_spot_symbols(&mut symbols, &invalid).is_err());
        assert!(symbols.is_empty());
    }
}
