use super::clamp_to_viewport;

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
