use super::{book, twap_limit_price_for_slice};

#[test]
fn twap_price_gate_walks_buy_depth_inside_range() {
    let book = book(&[(99.0, 1.0)], &[(100.0, 0.5), (101.0, 0.75)]);
    assert_eq!(
        twap_limit_price_for_slice(&book, true, 1.0, 99.0, 101.0),
        Some(101.0)
    );
    assert_eq!(
        twap_limit_price_for_slice(&book, true, 1.0, 99.0, 100.5),
        None
    );
}

#[test]
fn twap_price_gate_walks_sell_depth_inside_range() {
    let book = book(&[(100.0, 0.25), (99.0, 1.0)], &[(101.0, 1.0)]);
    assert_eq!(
        twap_limit_price_for_slice(&book, false, 1.0, 99.0, 101.0),
        Some(99.0)
    );
    assert_eq!(
        twap_limit_price_for_slice(&book, false, 1.0, 99.5, 101.0),
        None
    );
}

#[test]
fn twap_price_gate_rejects_best_price_outside_hard_range() {
    let book = book(&[(105.0, 1.0)], &[(95.0, 1.0)]);
    assert_eq!(
        twap_limit_price_for_slice(&book, true, 0.5, 99.0, 101.0),
        None
    );
    assert_eq!(
        twap_limit_price_for_slice(&book, false, 0.5, 99.0, 101.0),
        None
    );
}
