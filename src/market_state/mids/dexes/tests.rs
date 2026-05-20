use super::*;

fn symbol(key: &str, market_type: MarketType) -> ExchangeSymbol {
    ExchangeSymbol {
        key: key.to_string(),
        ticker: key.rsplit(':').next().unwrap_or(key).to_string(),
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

#[test]
fn mids_dexes_include_main_known_and_discovered_hip3_dexes() {
    let symbols = vec![
        symbol("newdex:ABC", MarketType::Perp),
        symbol("xyz:NVDA", MarketType::Perp),
        symbol("@1", MarketType::Spot),
        symbol("#0", MarketType::Outcome),
        symbol("BTC", MarketType::Perp),
    ];

    let dexes = known_mids_dexes(&symbols, &["xyz", "flx"]);

    assert_eq!(
        dexes,
        vec![
            String::new(),
            "flx".to_string(),
            "newdex".to_string(),
            "xyz".to_string(),
        ]
    );
}

#[test]
fn mids_dexes_keep_main_dex_first_after_sorting() {
    let dexes = known_mids_dexes(&[], &["z", "a"]);

    assert_eq!(dexes, vec![String::new(), "a".to_string(), "z".to_string()]);
}

#[test]
fn normalized_mids_dexes_keep_main_first_and_dedup() {
    let dexes = normalize_mids_dexes(vec![
        "flx".to_string(),
        "xyz".to_string(),
        "FLX".to_string(),
        String::new(),
    ]);

    assert_eq!(
        dexes,
        vec![String::new(), "flx".to_string(), "xyz".to_string()]
    );
}
