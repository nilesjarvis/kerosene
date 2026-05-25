use super::{scaled_px, scaled_text};

#[test]
fn alfred_scaling_clamps_text_and_spacing_to_expected_bounds() {
    assert_eq!(scaled_text(14.0, 0.1), 13);
    assert_eq!(scaled_text(14.0, 2.0), 19);
    assert_eq!(scaled_text(200.0, 2.0), 48);

    assert_eq!(scaled_px(10.0, 0.1), 9);
    assert_eq!(scaled_px(40.0, 2.0), 64);
    assert_eq!(scaled_px(0.1, 1.0), 1);
}
