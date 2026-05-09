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
