use super::axis_display_label;

#[test]
fn axis_display_label_keeps_labels_that_fit() {
    assert_eq!(axis_display_label("BTC", 60.0), "BTC");
}

#[test]
fn axis_display_label_ellipsizes_long_outcome_labels() {
    // 48px of text width at 6px per char leaves room for 8 chars.
    assert_eq!(
        axis_display_label("YES: Will BTC close green? (Jun 30)", 48.0),
        "YES: ..."
    );
}

#[test]
fn axis_display_label_handles_no_available_width() {
    assert_eq!(axis_display_label("BTC", 0.0), "");
    assert_eq!(axis_display_label("BTC", -10.0), "");
}
