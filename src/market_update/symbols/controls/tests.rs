use crate::market_state::{SYMBOL_SEARCH_ALL_HIP3_DEXES, SymbolSearchMarketFilter};

use super::*;

#[test]
fn favourite_toggle_adds_missing_symbol_to_end() {
    let mut favourites = vec!["BTC".to_string()];

    toggle_favourite_symbol(&mut favourites, "ETH".to_string());

    assert_eq!(favourites, vec!["BTC".to_string(), "ETH".to_string()]);
}

#[test]
fn favourite_toggle_removes_existing_symbol_without_reordering_others() {
    let mut favourites = vec!["BTC".to_string(), "ETH".to_string(), "SOL".to_string()];

    toggle_favourite_symbol(&mut favourites, "ETH".to_string());

    assert_eq!(favourites, vec!["BTC".to_string(), "SOL".to_string()]);
}

#[test]
fn market_filter_change_clears_hip3_dex_when_leaving_hip3() {
    let mut filter = SymbolSearchMarketFilter::Hip3;
    let mut hip3_dex_filter = Some("testdex".to_string());

    apply_market_filter(
        &mut filter,
        &mut hip3_dex_filter,
        SymbolSearchMarketFilter::Spot,
    );

    assert_eq!(filter, SymbolSearchMarketFilter::Spot);
    assert_eq!(hip3_dex_filter, None);
}

#[test]
fn market_filter_change_keeps_hip3_dex_when_staying_on_hip3() {
    let mut filter = SymbolSearchMarketFilter::All;
    let mut hip3_dex_filter = Some("testdex".to_string());

    apply_market_filter(
        &mut filter,
        &mut hip3_dex_filter,
        SymbolSearchMarketFilter::Hip3,
    );

    assert_eq!(filter, SymbolSearchMarketFilter::Hip3);
    assert_eq!(hip3_dex_filter, Some("testdex".to_string()));
}

#[test]
fn hip3_dex_filter_all_option_clears_specific_dex() {
    let mut hip3_dex_filter = Some("testdex".to_string());

    apply_hip3_dex_filter(
        &mut hip3_dex_filter,
        SYMBOL_SEARCH_ALL_HIP3_DEXES.to_string(),
    );

    assert_eq!(hip3_dex_filter, None);
}

#[test]
fn hip3_dex_filter_specific_dex_is_stored() {
    let mut hip3_dex_filter = None;

    apply_hip3_dex_filter(&mut hip3_dex_filter, "testdex".to_string());

    assert_eq!(hip3_dex_filter, Some("testdex".to_string()));
}
