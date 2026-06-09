use super::*;
use crate::account::UserFill;

fn fill(coin: &str, time: u64, px: &str, sz: &str, side: &str) -> UserFill {
    UserFill {
        coin: coin.to_string(),
        px: px.to_string(),
        sz: sz.to_string(),
        side: side.to_string(),
        time,
        hash: None,
        tid: None,
        oid: None,
        dir: "Open Long".to_string(),
        closed_pnl: "0".to_string(),
        fee: "0".to_string(),
    }
}

#[test]
fn trade_markers_for_symbol_maps_valid_fills() {
    let fills = vec![
        fill("BTC", 2, "100", "0.2", "B"),
        fill("BTC", 1, "101", "0.1", "A"),
    ];

    let markers = trade_markers_for_symbol(&fills, "BTC");

    assert_eq!(markers.len(), 2);
    assert_eq!(markers[0].time_ms, 2);
    assert!(markers[0].is_buy);
    assert_eq!(markers[1].time_ms, 1);
    assert!(!markers[1].is_buy);
}

#[test]
fn trade_markers_for_symbol_skips_invalid_values_and_sides() {
    let fills = vec![
        fill("BTC", 1, "nan", "0.1", "B"),
        fill("BTC", 2, "100", "0", "B"),
        fill("BTC", 3, "100", "0.1", "X"),
        fill("BTC", 4, "100", "0.1", "A"),
    ];

    let markers = trade_markers_for_symbol(&fills, "BTC");

    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0].time_ms, 4);
}

#[test]
fn trade_markers_for_symbol_requires_exact_symbol_match() {
    let fills = vec![
        fill("BTC", 1, "100", "0.1", "B"),
        fill("xyz:BTC", 2, "101", "0.2", "B"),
    ];

    let main_markers = trade_markers_for_symbol(&fills, "BTC");
    let dex_markers = trade_markers_for_symbol(&fills, "xyz:BTC");

    assert_eq!(main_markers.len(), 1);
    assert_eq!(main_markers[0].time_ms, 1);
    assert_eq!(dex_markers.len(), 1);
    assert_eq!(dex_markers[0].time_ms, 2);
}
