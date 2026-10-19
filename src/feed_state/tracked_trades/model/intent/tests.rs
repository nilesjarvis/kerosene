use super::TrackedTradeIntent;

#[test]
fn intent_labels_match_feed_copy() {
    assert_eq!(TrackedTradeIntent::Opening.label(), "Opening");
    assert_eq!(TrackedTradeIntent::Increasing.label(), "Increasing");
    assert_eq!(TrackedTradeIntent::Reducing.label(), "Reducing");
    assert_eq!(TrackedTradeIntent::Closing.label(), "Closing");
    assert_eq!(TrackedTradeIntent::Reversing.label(), "Reversing");
    assert_eq!(TrackedTradeIntent::Unknown.label(), "Unknown");
}

#[test]
fn trade_feed_fill_intent_distinguishes_all_position_changes_without_rounding_away_small_sizes() {
    use TrackedTradeIntent::*;
    for (start, buy, size, expected) in [
        (0.0, true, 1.0, Opening),
        (0.0, false, 1.0, Opening),
        (2.0, true, 1.0, Increasing),
        (-2.0, false, 1.0, Increasing),
        (2.0, false, 1.0, Reducing),
        (-2.0, true, 1.0, Reducing),
        (2.0, false, 2.0, Closing),
        (-2.0, true, 2.0, Closing),
        (2.0, false, 3.0, Reversing),
        (-2.0, true, 3.0, Reversing),
        (1e-10, true, 1e-10, Increasing),
        (1e-10, false, 1e-10, Closing),
        (1.0, false, 0.9999999999, Reducing),
        (1.0, false, 1.0000000001, Reversing),
    ] {
        assert_eq!(
            TrackedTradeIntent::from_fill(Some(start), buy, size),
            expected
        );
    }
    assert_eq!(TrackedTradeIntent::from_fill(None, true, 1.0), Unknown);
    assert_eq!(
        TrackedTradeIntent::from_fill(Some(f64::NAN), true, 1.0),
        Unknown
    );
    for size in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            TrackedTradeIntent::from_fill(Some(1.0), true, size),
            Unknown
        );
    }
}
