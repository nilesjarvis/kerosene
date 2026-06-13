use super::{alfred_visible_detail, scaled_px, scaled_text};

#[test]
fn alfred_scaling_clamps_text_and_spacing_to_expected_bounds() {
    assert_eq!(scaled_text(14.0, 0.1), 13);
    assert_eq!(scaled_text(14.0, 2.0), 19);
    assert_eq!(scaled_text(200.0, 2.0), 48);

    assert_eq!(scaled_px(10.0, 0.1), 9);
    assert_eq!(scaled_px(40.0, 2.0), 64);
    assert_eq!(scaled_px(0.1, 1.0), 1);
}

#[test]
fn disabled_alfred_rows_show_disabled_reason_as_detail() {
    assert_eq!(
        alfred_visible_detail(
            false,
            "Close all open perp positions at market",
            Some("Account data is stale; refresh before NUKE"),
        ),
        "Account data is stale; refresh before NUKE"
    );
    assert_eq!(
        alfred_visible_detail(false, "Fallback detail", None),
        "Fallback detail"
    );
    assert_eq!(
        alfred_visible_detail(true, "Enabled detail", Some("Disabled reason")),
        "Enabled detail"
    );
}
