use super::*;

#[test]
fn hip3_dexes_are_sorted_unique_perp_prefixes_only() {
    let symbols = vec![
        symbol("xyz:NVDA", MarketType::Perp),
        symbol("@1", MarketType::Spot),
        symbol("abc:BTC", MarketType::Perp),
        symbol("xyz:TSLA", MarketType::Perp),
        symbol("BTC", MarketType::Perp),
    ];

    assert_eq!(
        symbol_search_hip3_dexes(&symbols),
        vec!["abc".to_string(), "xyz".to_string()]
    );
}
