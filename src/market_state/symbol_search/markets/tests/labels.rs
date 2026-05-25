use super::*;

#[test]
fn exchange_labels_match_market_kind() {
    assert_eq!(
        symbol_search_exchange_label(&symbol("BTC", MarketType::Perp)),
        "Native Perps"
    );
    assert_eq!(
        symbol_search_exchange_label(&symbol("xyz:NVDA", MarketType::Perp)),
        "HIP-3: xyz"
    );
    assert_eq!(
        symbol_search_exchange_label(&symbol("@1", MarketType::Spot)),
        "Spot"
    );
    assert_eq!(
        symbol_search_exchange_label(&symbol("#0", MarketType::Outcome)),
        "Outcomes"
    );
}

#[test]
fn exchange_rank_groups_native_spot_hip3_and_outcomes() {
    assert_eq!(
        symbol_search_exchange_rank(&symbol("BTC", MarketType::Perp)),
        (0, String::new())
    );
    assert_eq!(
        symbol_search_exchange_rank(&symbol("@1", MarketType::Spot)),
        (1, String::new())
    );
    assert_eq!(
        symbol_search_exchange_rank(&symbol("xyz:NVDA", MarketType::Perp)),
        (2, "xyz".to_string())
    );
    assert_eq!(
        symbol_search_exchange_rank(&symbol("#0", MarketType::Outcome)),
        (3, String::new())
    );
}
