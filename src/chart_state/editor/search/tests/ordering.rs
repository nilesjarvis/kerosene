use super::*;
use std::cmp::Ordering;

#[test]
fn compare_prefers_score_then_favourites_then_symbol_order() {
    let btc = symbol("BTC", "BTC", "crypto", Some("Bitcoin"), &[]);
    let eth = symbol("ETH", "ETH", "crypto", Some("Ethereum"), &[]);
    let hype = symbol("HYPE", "HYPE", "crypto", None, &[]);
    let favourites = vec!["HYPE".to_string(), "ETH".to_string()];

    assert_eq!(
        compare_chart_editor_symbols(&btc, &eth, "eth", &favourites),
        Ordering::Greater
    );
    assert_eq!(
        compare_chart_editor_symbols(&hype, &eth, "", &favourites),
        Ordering::Less
    );
    assert_eq!(
        compare_chart_editor_symbols(&btc, &eth, "", &[]),
        Ordering::Less
    );
}

#[test]
fn compare_prefers_perp_over_spot_for_bare_ticker_collisions() {
    // Enter with no selection takes the first result, so this ordering decides
    // which market a bare ticker opens. It must match the perp-first rule the
    // symbol resolver and Alfred use, or the two keyboard flows drift apart.
    let perp = symbol("HYPE", "HYPE", "crypto", None, &[]);
    let mut spot = symbol("@107", "HYPE", "spot", Some("HYPE/USDC"), &["spot"]);
    spot.market_type = MarketType::Spot;

    assert_eq!(
        compare_chart_editor_symbols(&perp, &spot, "hype", &[]),
        Ordering::Less
    );
    assert_eq!(
        compare_chart_editor_symbols(&spot, &perp, "hype", &[]),
        Ordering::Greater
    );
}

#[test]
fn compare_prefers_primary_known_hip3_dex_for_duplicate_tickers() {
    let flx_crcl = symbol("flx:CRCL", "CRCL", "stocks", None, &[]);
    let xyz_crcl = symbol("xyz:CRCL", "CRCL", "stocks", Some("CRCL"), &[]);

    assert_eq!(
        compare_chart_editor_symbols(&xyz_crcl, &flx_crcl, "crcl", &[]),
        Ordering::Less
    );
    assert_eq!(
        compare_chart_editor_symbols(&flx_crcl, &xyz_crcl, "crcl", &[]),
        Ordering::Greater
    );
}
