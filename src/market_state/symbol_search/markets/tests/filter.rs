use super::*;

#[test]
fn market_filter_matches_native_spot_outcome_and_hip3_variants() {
    let native = symbol("BTC", MarketType::Perp);
    let hip3 = symbol("xyz:NVDA", MarketType::Perp);
    let spot = symbol("@1", MarketType::Spot);
    let outcome = outcome_symbol("#0", false);

    assert!(symbol_search_matches_market_filter(
        &native,
        SymbolSearchMarketFilter::NativePerps,
        None
    ));
    assert!(!symbol_search_matches_market_filter(
        &hip3,
        SymbolSearchMarketFilter::NativePerps,
        None
    ));
    assert!(symbol_search_matches_market_filter(
        &spot,
        SymbolSearchMarketFilter::Spot,
        None
    ));
    assert!(symbol_search_matches_market_filter(
        &outcome,
        SymbolSearchMarketFilter::Outcomes,
        None
    ));
    assert!(symbol_search_matches_market_filter(
        &hip3,
        SymbolSearchMarketFilter::Hip3,
        None
    ));
    assert!(symbol_search_matches_market_filter(
        &hip3,
        SymbolSearchMarketFilter::Hip3,
        Some("xyz")
    ));
    assert!(!symbol_search_matches_market_filter(
        &hip3,
        SymbolSearchMarketFilter::Hip3,
        Some("abc")
    ));
}

#[test]
fn market_filter_hides_question_fallback_outcomes() {
    let fallback = outcome_symbol("#660", true);
    let named = outcome_symbol("#670", false);

    assert!(!symbol_search_matches_market_filter(
        &fallback,
        SymbolSearchMarketFilter::All,
        None
    ));
    assert!(!symbol_search_matches_market_filter(
        &fallback,
        SymbolSearchMarketFilter::Outcomes,
        None
    ));
    assert!(symbol_search_matches_market_filter(
        &named,
        SymbolSearchMarketFilter::All,
        None
    ));
    assert!(symbol_search_matches_market_filter(
        &named,
        SymbolSearchMarketFilter::Outcomes,
        None
    ));
}
