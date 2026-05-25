use super::{compare_symbol_keys_for_same_ticker, hip3_dex};
use std::cmp::Ordering;

#[test]
fn hip3_dex_extracts_only_prefixed_perp_symbols() {
    assert_eq!(hip3_dex("xyz:CRCL"), Some("xyz"));
    assert_eq!(hip3_dex("BTC"), None);
    assert_eq!(hip3_dex("@107"), None);
    assert_eq!(hip3_dex(":CRCL"), None);
    assert_eq!(hip3_dex("xyz:"), None);
}

#[test]
fn duplicate_hip3_tickers_use_known_dex_order() {
    assert_eq!(
        compare_symbol_keys_for_same_ticker("xyz:CRCL", "flx:CRCL"),
        Ordering::Less
    );
    assert_eq!(
        compare_symbol_keys_for_same_ticker("unknown:CRCL", "xyz:CRCL"),
        Ordering::Greater
    );
    assert_eq!(
        compare_symbol_keys_for_same_ticker("BTC", "ETH"),
        Ordering::Less
    );
}
