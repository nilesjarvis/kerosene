use super::{clamp_to_viewport, ease_out_cubic};

// ---------------------------------------------------------------------------
// Anchored Menu Geometry Tests
// ---------------------------------------------------------------------------

#[test]
fn clamp_to_viewport_keeps_values_inside_viewport_margins() {
    assert_eq!(clamp_to_viewport(50.0, 20.0, 0.0, 100.0), 50.0);
    assert_eq!(clamp_to_viewport(-10.0, 20.0, 0.0, 100.0), 4.0);
    assert_eq!(clamp_to_viewport(90.0, 20.0, 0.0, 100.0), 76.0);
}

#[test]
fn clamp_to_viewport_prefers_minimum_when_menu_is_larger_than_viewport() {
    assert_eq!(clamp_to_viewport(20.0, 120.0, 0.0, 100.0), 4.0);
}

#[test]
fn ease_out_cubic_is_clamped_and_monotonic() {
    assert_eq!(ease_out_cubic(0.0), 0.0);
    assert_eq!(ease_out_cubic(1.0), 1.0);
    assert_eq!(ease_out_cubic(-1.0), 0.0);
    assert_eq!(ease_out_cubic(2.0), 1.0);

    let quarter = ease_out_cubic(0.25);
    let half = ease_out_cubic(0.5);
    assert!(quarter > 0.25, "ease-out should lead a linear ramp early");
    assert!(half > quarter);
    assert!(half < 1.0);
}
