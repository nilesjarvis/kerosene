use super::{compare_symbol_keys_for_same_ticker, hip3_dex, svg_aspect_ratio, symbol_svg_logo};
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

#[test]
fn svg_aspect_ratio_reads_view_box_dimensions() {
    // Uses the third/fourth viewBox values (width/height), ignoring the origin.
    let svg = br#"<svg viewBox="14 14 36 18" xmlns="http://www.w3.org/2000/svg"></svg>"#;
    assert_eq!(svg_aspect_ratio(svg), Some(2.0));
}

#[test]
fn svg_aspect_ratio_falls_back_to_width_height_attrs() {
    let svg = br#"<svg width="800" height="200" xmlns="http://www.w3.org/2000/svg"></svg>"#;
    assert_eq!(svg_aspect_ratio(svg), Some(4.0));
}

#[test]
fn svg_aspect_ratio_ignores_stroke_width_attr() {
    // `width` must match the svg dimension, not `stroke-width`.
    let svg = br#"<svg stroke-width="3" width="40" height="20"></svg>"#;
    assert_eq!(svg_aspect_ratio(svg), Some(2.0));
}

#[test]
fn svg_aspect_ratio_strips_length_units() {
    let svg = br#"<svg width="256px" height="128px"></svg>"#;
    assert_eq!(svg_aspect_ratio(svg), Some(2.0));
}

#[test]
fn symbol_svg_logo_resolves_embedded_asset_with_aspect() {
    // BTC's logo has a square viewBox; resolution also strips pair suffixes.
    let (_, aspect) = symbol_svg_logo("BTC/USDC").expect("btc logo resolves");
    assert!((aspect - 1.0).abs() < 0.05, "aspect was {aspect}");
}
